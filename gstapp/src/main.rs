use std::path::Path;

use gst::prelude::*;
use log::{error, info, warn};
use structopt::StructOpt;

mod klvelm;
use klvelm::{uasdls_print_sink, uasdls_test_src, KLV_CAPS};

/// play ts file by path
fn decode_mpegtsklv(path: String) {
    gst::init().unwrap();

    let pipeline = gst::Pipeline::new(None);
    let src = gst::ElementFactory::make("filesrc", None).unwrap();
    let tsdemux = gst::ElementFactory::make("tsdemux", None).unwrap();
    let h264parse = gst::ElementFactory::make("h264parse", None).unwrap();
    let avdec_h264 = gst::ElementFactory::make("avdec_h264", None).unwrap();
    let videoconvert = gst::ElementFactory::make("videoconvert", None).unwrap();
    let ximagesink = gst::ElementFactory::make("ximagesink", None).unwrap();
    let queue = gst::ElementFactory::make("queue", None).unwrap();
    let uasdas_sink = uasdls_print_sink().unwrap();

    pipeline
        .add_many(&[
            &src,
            &tsdemux,
            &h264parse,
            &avdec_h264,
            &videoconvert,
            &ximagesink,
            &queue,
            &uasdas_sink,
        ])
        .unwrap();
    let filepath = Path::new(&path);
    if !filepath.exists() {
        error!("not found file {}", filepath.to_str().unwrap());
        return;
    }
    info!("using videofile {}", &path);
    src.set_property("location", &path);
    src.link(&tsdemux).unwrap();
    let h264_sink_pad = h264parse
        .static_pad("sink")
        .expect("h264 could not be linked.");
    let queue_sink_pad = queue
        .static_pad("sink")
        .expect("private sink could not be linked.");

    // demuxはPad Capability: Sometimes
    // つまりソースによってできたり出来なかったりするのでDynamic Connectionが必要
    tsdemux.connect_pad_added(move |src, src_pad| {
        info!("Received new pad {} from {}", src_pad.name(), src.name());
        if src_pad.name().contains("video") {
            src_pad.link(&h264_sink_pad).unwrap();
        } else if src_pad.name().contains("private") {
            src_pad.link(&queue_sink_pad).unwrap();
        }
    });

    gst::Element::link_many(&[&h264parse, &avdec_h264, &videoconvert, &ximagesink])
        .expect("Elements could not be linked.");

    queue.link(&uasdas_sink).unwrap();

    // Actually start the pipeline.
    pipeline
        .set_state(gst::State::Playing)
        .expect("Unable to set the pipeline to the `Playing` state");
    let pipeline = pipeline.dynamic_cast::<gst::Pipeline>().unwrap();

    let bus = pipeline
        .bus()
        .expect("Pipeline without bus. Shouldn't happen!");

    // And run until EOS or an error happened.
    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                println!(
                    "Error from {:?}: {} ({:?})",
                    err.src().map(|s| s.path_string()),
                    err.error(),
                    err.debug()
                );
                break;
            }
            _ => (),
        }
    }

    // Finally shut down everything.
    pipeline
        .set_state(gst::State::Null)
        .expect("Unable to set the pipeline to the `Null` state");
}

/// play videotestsrcwith custom klv data and encode to mpeg2ts
fn video_with_klv<P: AsRef<str>>(savefilename: Option<P>) {
    gst::init().unwrap();
    let pipeline = gst::Pipeline::new(None);
    let videosrc = gst::ElementFactory::make("videotestsrc", None).unwrap();
    let x264enc = gst::ElementFactory::make("x264enc", None).unwrap();
    let h264parse = gst::ElementFactory::make("h264parse", None).unwrap();
    let mpegtsmux = gst::ElementFactory::make("mpegtsmux", None).unwrap();
    let tsdemux = gst::ElementFactory::make("tsdemux", None).unwrap();
    let _queue = gst::ElementFactory::make("queue", None).unwrap();
    let tee = gst::ElementFactory::make("tee", None).unwrap();

    let h264parse_dest = gst::ElementFactory::make("h264parse", None).unwrap();
    let avdec_h264 = gst::ElementFactory::make("avdec_h264", None).unwrap();
    let videoconvert = gst::ElementFactory::make("videoconvert", None).unwrap();
    let ximagesink = gst::ElementFactory::make("ximagesink", None).unwrap();

    let appsrc = uasdls_test_src().unwrap();

    let videosrc_caps = gst::Caps::builder("video/x-raw")
        .field("width", 320)
        .field("height", 240)
        .field("format", "I420")
        .build();

    let appsink = uasdls_print_sink().unwrap();

    pipeline
        .add_many(&[
            &appsrc,
            &videosrc,
            &h264parse,
            &x264enc,
            &mpegtsmux,
            &tee,
            &tsdemux,
            &h264parse_dest,
            &avdec_h264,
            &videoconvert,
            &ximagesink,
        ])
        .unwrap();

    // link video source to mpegtsmux
    videosrc.link_filtered(&x264enc, &videosrc_caps).unwrap();
    x264enc.link(&h264parse).unwrap();
    h264parse.link(&mpegtsmux).unwrap();
    appsrc.link_filtered(&mpegtsmux, &KLV_CAPS).unwrap();
    mpegtsmux.link(&tee).unwrap();
    mpegtsmux.set_property("alignment", 7);

    // if attach save filename
    if let Some(filepath) = savefilename {
        // add tee
        let queue = gst::ElementFactory::make("queue", None).unwrap();
        let filesink = gst::ElementFactory::make("filesink", None).unwrap();
        pipeline.add_many(&[&queue, &filesink]).unwrap();
        filesink.set_property("location", filepath.as_ref());
        gst::Element::link_many(&[&tee, &queue, &filesink]).unwrap();
    }

    // link display pipe
    gst::Element::link_many(&[&tee, &tsdemux]).unwrap();
    gst::Element::link_many(&[&h264parse_dest, &avdec_h264, &videoconvert, &ximagesink]).unwrap();
    let h264_sink_pad = h264parse_dest
        .static_pad("sink")
        .expect("h264 could not be linked.");
    let pipeline_weak = pipeline.downgrade();
    let appsink = appsink.upcast::<gst::Element>();

    // Demuxer need connect after playing (detect source)
    tsdemux.connect_pad_added(move |src, src_pad| {
        if src_pad.name().contains("video") {
            info!(
                "connect new video pad {} from {}",
                src_pad.name(),
                src.name()
            );
            src_pad.link(&h264_sink_pad).unwrap();
        } else if src_pad.name().contains("private") {
            info!(
                "connect new metadata pad {} from {}",
                src_pad.name(),
                src.name()
            );
            let pipeline = match pipeline_weak.upgrade() {
                Some(pipeline) => pipeline,
                None => return,
            };
            let queue = gst::ElementFactory::make("queue", None).unwrap();
            let elements = &[&queue, &appsink];
            pipeline
                .add_many(elements)
                .expect("failed to add audio elements to pipeline");
            gst::Element::link_many(elements).unwrap();

            let appsink_pad = queue
                .static_pad("sink")
                .expect("failed to get queue and appsink pad.");
            src_pad.link(&appsink_pad).unwrap();
            for e in elements {
                e.sync_state_with_parent().unwrap();
            }
        } else {
            warn!(
                "Received unsupported new pad {} from {}",
                src_pad.name(),
                src.name()
            );
        }
    });

    // Actually start the pipeline.
    pipeline
        .set_state(gst::State::Playing)
        .expect("Unable to set the pipeline to the `Playing` state");
    let pipeline = pipeline.dynamic_cast::<gst::Pipeline>().unwrap();

    let bus = pipeline
        .bus()
        .expect("Pipeline without bus. Shouldn't happen!");

    // And run until EOS or an error happened.
    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                println!(
                    "Error from {:?}: {} ({:?})",
                    err.src().map(|s| s.path_string()),
                    err.error(),
                    err.debug()
                );
                break;
            }

            MessageView::StateChanged(s) => {
                info!(
                    "State changed from {:?}: {:?} -> {:?} ({:?})",
                    s.src().map(|s| s.path_string()),
                    s.old(),
                    s.current(),
                    s.pending()
                );
            }

            _ => (),
        }
    }

    // Finally shut down everything.
    pipeline
        .set_state(gst::State::Null)
        .expect("Unable to set the pipeline to the `Null` state");
}

#[derive(Debug, StructOpt)]
#[structopt(about = "the stupid content tracker")]
enum Cmd {
    Decode {
        #[structopt(default_value = "../testdata/DayFlight.mpg")]
        path: String,
    },
    Klv {
        #[structopt(short, long)]
        save: Option<String>,
    },
}
fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cmd = Cmd::from_args();
    log::debug!("cmd {:?}", &cmd);
    match cmd {
        Cmd::Klv { save } => video_with_klv(save),
        Cmd::Decode { path } => decode_mpegtsklv(path),
    }
}
