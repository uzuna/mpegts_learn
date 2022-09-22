use gst::prelude::*;
use gst_app::gst::element_error;
use klv::{
    uasdms::{UASDataset, LS_UNIVERSAL_KEY0601_8_10},
    KLVReader,
};
use log::{info, warn};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
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
                    if slice[..16] == LS_UNIVERSAL_KEY0601_8_10[..] {
                        info!("KLV PTS: {:?}", &buffer.pts());
                        // show data
                        let len = slice[17] as usize;
                        let r = KLVReader::<UASDataset>::from_bytes(&slice[18..18 + len]);
                        for x in r {
                            info!("  UAS {:?} {:?}", x.key(), x.parse());
                        }
                    } else {
                        warn!("unknown key {:?}", &slice[..16]);
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
