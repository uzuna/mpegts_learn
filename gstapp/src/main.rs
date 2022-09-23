use gst::prelude::*;
use gst_app::gst::element_error;
use klv::{
    uasdms::{UASDataset, LS_UNIVERSAL_KEY0601_8_10},
    KLVGlobal, KLVReader,
};
use log::{info, warn};
use structopt::StructOpt;

fn decode_mpegtsklv() {
    gst::init().unwrap();

    let pipeline = gst::Pipeline::new(None);
    let src = gst::ElementFactory::make("filesrc", None).unwrap();
    let tsdemux = gst::ElementFactory::make("tsdemux", None).unwrap();
    let h264parse = gst::ElementFactory::make("h264parse", None).unwrap();
    let avdec_h264 = gst::ElementFactory::make("avdec_h264", None).unwrap();
    let videoconvert = gst::ElementFactory::make("videoconvert", None).unwrap();
    let ximagesink = gst::ElementFactory::make("ximagesink", None).unwrap();

    let queue = gst::ElementFactory::make("queue", None).unwrap();

    let appsink = gst::ElementFactory::make("appsink", None)
        .unwrap()
        .downcast::<gst_app::AppSink>()
        .unwrap();

    pipeline
        .add_many(&[
            &src,
            &tsdemux,
            &h264parse,
            &avdec_h264,
            &videoconvert,
            &ximagesink,
            &queue,
        ])
        .unwrap();
    pipeline.add(&appsink).unwrap();
    src.set_property("location", "/home/fmy/Downloads/gstrec/DayFlight.mpg");
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

    queue.link(&appsink).unwrap();

    // build appsink
    appsink.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            // Add a handler to the "new-sample" signal.
            .new_sample(|appsink| {
                // Pull the sample in question out of the appsink's buffer.
                let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                let buffer = sample.buffer().ok_or_else(|| {
                    element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to get buffer from appsink")
                    );

                    gst::FlowError::Error
                })?;

                if buffer.size() > 0 {
                    let mut slice = vec![0; buffer.size()];
                    buffer.copy_to_slice(0, &mut slice).unwrap();
                    // Check UniversalKey
                    if let Ok(klvg) = KLVGlobal::try_from_bytes(&slice) {
                        if klvg.key_is(&LS_UNIVERSAL_KEY0601_8_10) {
                            let r = KLVReader::<UASDataset>::from_bytes(klvg.content());

                            for x in r {
                                println!("uas ds {:?} {:?}", x.key(), x.parse());
                            }
                        } else {
                            warn!("unknown key {:?}", &slice[..16]);
                        }
                    }
                }
                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );

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

fn include_klv() {
    // testvideosrc with klv
    gst::init().unwrap();
    let pipeline = gst::Pipeline::new(None);
    let videosrc = gst::ElementFactory::make("videotestsrc", None).unwrap();
    let x264enc = gst::ElementFactory::make("x264enc", None).unwrap();
    let h264parse_src = gst::ElementFactory::make("h264parse", None).unwrap();
    let mpegtsmux = gst::ElementFactory::make("mpegtsmux", None).unwrap();
    let tsdemux = gst::ElementFactory::make("tsdemux", None).unwrap();
    let h264parse_dest = gst::ElementFactory::make("h264parse", None).unwrap();
    let avdec_h264 = gst::ElementFactory::make("avdec_h264", None).unwrap();
    let videoconvert = gst::ElementFactory::make("videoconvert", None).unwrap();
    let ximagesink = gst::ElementFactory::make("ximagesink", None).unwrap();

    let videosrc_caps = gst::Caps::builder("video/x-raw")
        .field("width", 320)
        .field("height", 240)
        // .field("framerate", "30/1")
        .field("format", "I420")
        .build();

    pipeline
        .add_many(&[
            &videosrc,
            &x264enc,
            &h264parse_src,
            &mpegtsmux,
            &tsdemux,
            &h264parse_dest,
            &avdec_h264,
            &videoconvert,
            &ximagesink,
        ])
        .unwrap();

    videosrc.link_filtered(&x264enc, &videosrc_caps).unwrap();
    gst::Element::link_many(&[&x264enc, &h264parse_src, &mpegtsmux, &tsdemux]).unwrap();
    gst::Element::link_many(&[&h264parse_dest, &avdec_h264, &videoconvert, &ximagesink]).unwrap();

    let h264_sink_pad = h264parse_dest
        .static_pad("sink")
        .expect("h264 could not be linked.");

    // demuxはPad Capability: Sometimes
    // つまりソースによってできたり出来なかったりするのでDynamic Connectionが必要
    tsdemux.connect_pad_added(move |src, src_pad| {
        info!("Received new pad {} from {}", src_pad.name(), src.name());
        if src_pad.name().contains("video") {
            src_pad.link(&h264_sink_pad).unwrap();
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
    Decode,
    App,
}
fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cmd = Cmd::from_args();
    log::debug!("cmd {:?}", &cmd);
    match cmd {
        Cmd::App => include_klv(),
        Cmd::Decode => decode_mpegtsklv(),
    }
}
