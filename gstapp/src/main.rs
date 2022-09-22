
use gst::prelude::*;

fn main() {
    gst::init().unwrap();

    let pipeline = gst::Pipeline::new(None);
    let src = gst::ElementFactory::make("filesrc", None).unwrap();
    let tsdemux = gst::ElementFactory::make("tsdemux", None).unwrap();
    let h264parse = gst::ElementFactory::make("h264parse", None).unwrap();
    let avdec_h264 = gst::ElementFactory::make("avdec_h264", None).unwrap();
    let videoconvert = gst::ElementFactory::make("videoconvert", None).unwrap();
    let ximagesink = gst::ElementFactory::make("ximagesink", None).unwrap();

    pipeline
        .add_many(&[
            &src,
            &tsdemux,
            &h264parse,
            &avdec_h264,
            &videoconvert,
            &ximagesink,
        ])
        .unwrap();
    src.set_property("location", "/home/fmy/Downloads/gstrec/DayFlight.mpg");
    src.link(&tsdemux).unwrap();
    let h264_sink_pad = h264parse
        .static_pad("sink")
        .expect("filesrc could not be linked.");

    // demuxはPad Capability: Sometimes
    // つまりソースによってできたり出来なかったりするのでDynamic Connectionが必要
    tsdemux.connect_pad_added(move |src, src_pad| {
        println!("Received new pad {} from {}", src_pad.name(), src.name());
        if src_pad.name().contains("video") {
            src_pad.link(&h264_sink_pad).unwrap();
        }
    });

    gst::Element::link_many(&[&h264parse, &avdec_h264, &videoconvert, &ximagesink])
        .expect("Elements could not be linked.");

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
