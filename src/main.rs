#[macro_use]
extern crate mpeg2ts_reader;
extern crate hex_slice;

mod klv;

use hex_slice::AsHex;

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
            // The 'Program Association Table' is is always on PID 0.  We just use the standard
            // handling here, but an application could insert its own logic if required,
            demultiplex::FilterRequest::ByPid(psi::pat::PAT_PID) => {
                DumpFilterSwitch::Pat(demultiplex::PatPacketFilter::default())
            }
            // 'Stuffing' data on PID 0x1fff may be used to pad-out parts of the transport stream
            // so that it has constant overall bitrate.  This causes it to be ignored if present.
            demultiplex::FilterRequest::ByPid(mpeg2ts_reader::STUFFING_PID) => {
                DumpFilterSwitch::Null(demultiplex::NullPacketFilter::default())
            }
            // Some Transport Streams will contain data on 'well known' PIDs, which are not
            // announced in PAT / PMT metadata.  This application does not process any of these
            // well known PIDs, so we register NullPacketFiltet such that they will be ignored
            demultiplex::FilterRequest::ByPid(_) => {
                DumpFilterSwitch::Null(demultiplex::NullPacketFilter::default())
            }
            // This match-arm installs our application-specific handling for each H264 stream
            // discovered within the transport stream,
            demultiplex::FilterRequest::ByStream {
                stream_type: _,
                pmt,
                stream_info,
                ..
            } => PtsDumpElementaryStreamConsumer::construct(pmt, stream_info),
            // We need to have a match-arm to specify how to handle any other StreamType values
            // that might be present; we answer with NullPacketFilter so that anything other than
            // // H264 (handled above) is ignored,
            // demultiplex::FilterRequest::ByStream {
            //     stream_type: StreamType::Adts,
            //     pmt,
            //     stream_info,
            //     ..
            // } => {
            //     println!("adts {:?} {:?}", pmt, stream_info);
            //     DumpFilterSwitch::Null(demultiplex::NullPacketFilter::default())
            // }
            // demultiplex::FilterRequest::ByStream { stream_type, stream_info, .. } => {
            //     println!("stream_type {:?} {:?}", stream_type, stream_info);
            //     DumpFilterSwitch::Null(demultiplex::NullPacketFilter::default())
            // }
            // The 'Program Map Table' defines the sub-streams for a particular program within the
            // Transport Stream (it is common for Transport Streams to contain only one program).
            // We just use the standard handling here, but an application could insert its own
            // logic if required,
            demultiplex::FilterRequest::Pmt {
                pid,
                program_number,
            } => {
                println!("Pmt {:?} {}", pid, program_number);
                DumpFilterSwitch::Pmt(demultiplex::PmtPacketFilter::new(pid, program_number))
            }
            // Ignore 'Network Information Table', if present,
            demultiplex::FilterRequest::Nit { pid } => {
                println!("pid {:?}", pid);
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
                        print!("{:?}: pts {:#08x}                ", self.pid, pts.value())
                    }
                    Ok(pes::PtsDts::Both {
                        pts: Ok(pts),
                        dts: Ok(dts),
                    }) => print!(
                        "{:?}: pts {:?} sec dts {:?} ",
                        self.pid,
                        Duration::from_secs_f64(pts.value() as f64 / Timestamp::TIMEBASE as f64),
                        Duration::from_secs_f64(dts.value() as f64 / Timestamp::TIMEBASE as f64),
                    ),
                    _ => (),
                }
                let payload = parsed.payload();
                self.len = Some(payload.len());
                // println!(
                //     "{:02x}",
                //     payload[..cmp::min(payload.len(), 16)].plain_hex(false)
                // )
            }
            pes::PesContents::Parsed(None) => (println!("parsed")),
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
        // println!(
        //     "{:?}:                     continues {:02x}",
        //     self.pid,
        //     data[..cmp::min(data.len(), 16)].plain_hex(false)
        // );
        self.len = self.len.map(|l| l + data.len());
    }
    fn end_packet(&mut self, _ctx: &mut DumpDemuxContext) {
        println!(
            "{:?}: {:?} end of packet length={:?}",
            self.pid, self.format, self.len
        );
    }
    fn continuity_error(&mut self, _ctx: &mut DumpDemuxContext) {}
}

use structopt::StructOpt;

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
    println!("opt {:?}", &opt);

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
                    let itr = (&buf[0..n])
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
                                    let len = buf.len();
                                    if len >= 16 {
                                        let key = &buf[..16];
                                        println!("pat {:?}, {:02x?}", pk.pid(), key);
                                        let mut pos = 18;
                                        while pos < len {
                                            let tag = buf[pos];
                                            let len = buf[pos + 1] as usize;
                                            let val = &buf[pos + 2..pos + 2 + len];
                                            pos += len + 2;
                                            println!("  TLV {} {} {:02x?}", tag, len, val);
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
