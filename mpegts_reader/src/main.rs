#[macro_use]
extern crate mpeg2ts_reader;
extern crate hex_slice;

use hex_slice::AsHex;
use log::{debug, info};

use klv::uasdls::LS_UNIVERSAL_KEY0601_8_10;
use mpeg2ts_reader::demultiplex;
use mpeg2ts_reader::packet;
use mpeg2ts_reader::packet::Pid;
use mpeg2ts_reader::pes;
use mpeg2ts_reader::pes::PesHeader;

use mpeg2ts_reader::pes::Timestamp;
use mpeg2ts_reader::psi;
use mpeg2ts_reader::StreamType;
use std::cmp;

use std::fs::File;
use std::io::Read;
use std::time::Duration;

use klv::uasdls::UASDataset;
use klv::{KLVGlobal, KLVReader};
use structopt::StructOpt;

// This macro invocation creates an enum called DumpFilterSwitch, encapsulating all possible ways
// that this application may handle transport stream packets.  Each enum variant is just a wrapper
// around an implementation of the PacketFilter trait
packet_filter_switch! {
    DumpFilterSwitch<DumpDemuxContext> {
        // the DumpFilterSwitch::Pes variant will perform the logic actually specific to this
        // application,
        Pes: pes::PesPacketFilter<DumpDemuxContext,PtsDumpElementaryStreamConsumer>,

        // these definitions are boilerplate required by the framework,
        Pat: demultiplex::PatPacketFilter<DumpDemuxContext>,
        Pmt: demultiplex::PmtPacketFilter<DumpDemuxContext>,

        // this variant will be used when we want to ignore data in the transport stream that this
        // application does not care about
        Null: demultiplex::NullPacketFilter<DumpDemuxContext>,
    }
}

// This macro invocation creates a type called DumpDemuxContext, which is our application-specific
// implementation of the DemuxContext trait.
demux_context!(DumpDemuxContext, DumpFilterSwitch);

// When the de-multiplexing process needs to create a PacketFilter instance to handle a particular
// kind of data discovered within the Transport Stream being processed, it will send a
// FilterRequest to our application-specific implementation of the do_construct() method
impl DumpDemuxContext {
    fn do_construct(&mut self, req: demultiplex::FilterRequest<'_, '_>) -> DumpFilterSwitch {
        match req {
            demultiplex::FilterRequest::ByPid(psi::pat::PAT_PID) => {
                DumpFilterSwitch::Pat(demultiplex::PatPacketFilter::default())
            }
            demultiplex::FilterRequest::ByPid(mpeg2ts_reader::STUFFING_PID) => {
                DumpFilterSwitch::Null(demultiplex::NullPacketFilter::default())
            }
            demultiplex::FilterRequest::ByPid(_) => {
                DumpFilterSwitch::Null(demultiplex::NullPacketFilter::default())
            }
            demultiplex::FilterRequest::ByStream {
                stream_type: StreamType::H2220PesPrivateData,
                pmt,
                stream_info,
                ..
            } => PtsDumpElementaryStreamConsumer::construct(pmt, stream_info),
            demultiplex::FilterRequest::ByStream { .. } => {
                DumpFilterSwitch::Null(demultiplex::NullPacketFilter::default())
            }
            demultiplex::FilterRequest::Pmt {
                pid,
                program_number,
            } => DumpFilterSwitch::Pmt(demultiplex::PmtPacketFilter::new(pid, program_number)),
            demultiplex::FilterRequest::Nit { .. } => {
                DumpFilterSwitch::Null(demultiplex::NullPacketFilter::default())
            }
        }
    }
}

// Implement the ElementaryStreamConsumer to just dump and PTS/DTS timestamps to stdout
pub struct PtsDumpElementaryStreamConsumer {
    pid: packet::Pid,
    format: StreamType,
    len: Option<usize>,
    buf: Vec<u8>,
}
impl PtsDumpElementaryStreamConsumer {
    fn construct(
        _pmt_sect: &psi::pmt::PmtSection,
        stream_info: &psi::pmt::StreamInfo,
    ) -> DumpFilterSwitch {
        let filter = pes::PesPacketFilter::new(PtsDumpElementaryStreamConsumer {
            pid: stream_info.elementary_pid(),
            format: stream_info.stream_type(),
            len: None,
            buf: vec![],
        });
        DumpFilterSwitch::Pes(filter)
    }
}
impl pes::ElementaryStreamConsumer<DumpDemuxContext> for PtsDumpElementaryStreamConsumer {
    fn start_stream(&mut self, _ctx: &mut DumpDemuxContext) {
        println!("start stream: {:?}", self.format);
    }
    fn begin_packet(&mut self, _ctx: &mut DumpDemuxContext, header: pes::PesHeader) {
        match header.contents() {
            pes::PesContents::Parsed(Some(parsed)) => {
                match parsed.pts_dts() {
                    Ok(pes::PtsDts::PtsOnly(Ok(pts))) => {
                        println!("{:?}: pts {:#08x}                ", self.pid, pts.value())
                    }
                    Ok(pes::PtsDts::Both {
                        pts: Ok(pts),
                        dts: Ok(dts),
                    }) => println!(
                        "{:?}: pts {:?} sec dts {:?} ",
                        self.pid,
                        Duration::from_secs_f64(pts.value() as f64 / Timestamp::TIMEBASE as f64),
                        Duration::from_secs_f64(dts.value() as f64 / Timestamp::TIMEBASE as f64),
                    ),
                    _ => (),
                }
                let payload = parsed.payload();
                self.len = Some(payload.len());
                self.buf.extend_from_slice(payload);
            }
            pes::PesContents::Parsed(None) => println!("parsed"),
            pes::PesContents::Payload(payload) => {
                self.len = Some(payload.len());
                println!(
                    "{:?}:                               {:02x}",
                    self.pid,
                    payload[..cmp::min(payload.len(), 16)].plain_hex(false)
                )
            }
        }
    }
    fn continue_packet(&mut self, _ctx: &mut DumpDemuxContext, data: &[u8]) {
        self.buf.extend_from_slice(data);
        self.len = self.len.map(|l| l + data.len());
    }
    fn end_packet(&mut self, _ctx: &mut DumpDemuxContext) {
        if let Ok(klvg) = KLVGlobal::try_from_bytes(&self.buf) {
            if klvg.key_is(&LS_UNIVERSAL_KEY0601_8_10) {
                info!("Found UASDLS");
                let r = KLVReader::<UASDataset>::from_bytes(klvg.content());
                for x in r {
                    info!("  {:?} {:?}", x.key(), x.parse());
                }
            }
            self.buf.clear();
            self.len = None;
        }
    }
    fn continuity_error(&mut self, _ctx: &mut DumpDemuxContext) {}
}

#[derive(Debug, StructOpt)]
#[structopt(name = "mpegts-parse")]
struct Opt {
    #[structopt(short, long)]
    raw: bool,
    #[structopt(name = "FILE")]
    file_name: String,
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let opt = Opt::from_args();
    debug!("opt {:?}", &opt);

    // open input file named on command line,
    let mut f =
        File::open(&opt.file_name).unwrap_or_else(|_| panic!("file not found: {}", &opt.file_name));

    // create the context object that stores the state of the transport stream demultiplexing
    // process
    let mut ctx = DumpDemuxContext::new();

    // create the demultiplexer, which will use the ctx to create a filter for pid 0 (PAT)
    let mut demux = demultiplex::Demultiplex::new(&mut ctx);

    // consume the input file,
    let mut buf = [0u8; 188 * 1024];
    loop {
        match f.read(&mut buf[..]).expect("read failed") {
            0 => break,
            n => {
                if opt.raw {
                    println!("read buf {}", n);
                    let itr = buf[0..n]
                        .chunks_exact(packet::Packet::SIZE)
                        .map(packet::Packet::try_new);

                    for pk in itr.into_iter().flatten() {
                        if pk.pid() == Pid::PAT {
                            // println!("pat {:?}", pk.payload());
                        } else if pk.pid() == Pid::new(0x1f1) {
                            let payload = pk.payload().unwrap();
                            match PesHeader::from_bytes(payload).unwrap().contents() {
                                pes::PesContents::Parsed(Some(ppc)) => {
                                    let buf = ppc.payload();
                                    if let Ok(klvg) = KLVGlobal::try_from_bytes(buf) {
                                        if klvg.key_is(&LS_UNIVERSAL_KEY0601_8_10) {
                                            let r =
                                                KLVReader::<UASDataset>::from_bytes(klvg.content());

                                            for x in r {
                                                println!("uas ds {:?} {:?}", x.key(), x.parse());
                                            }
                                        }
                                    }
                                }
                                pes::PesContents::Parsed(None) => {
                                    println!("pat {:?}, None", pk.pid());
                                }
                                pes::PesContents::Payload(buf) => {
                                    println!("pat {:?}, {:02x?}", pk.pid(), buf);
                                }
                            }
                        } else {
                            // println!("pid {:?} {:?}", pk.pid(), pk.adaptation_field());
                        }
                    }
                } else {
                    demux.push(&mut ctx, &buf[0..n])
                }
            }
        }
    }
}
