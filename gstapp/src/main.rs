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

    let appsrc = gst::ElementFactory::make("appsrc", None)
        .unwrap()
        .downcast::<gst_app::AppSrc>()
        .unwrap();
    let klv_caps = gst::Caps::builder("meta/x-klv")
        .field("parsed", "true")
        .build();

    appsrc.set_caps(Some(&klv_caps));
    let mut i = 0;
    appsrc.set_callbacks(
        gst_app::AppSrcCallbacks::builder()
            .need_data(move |appsrc, _| {
                // Add a custom meta with a label to this buffer.
                let mut buffer = gst::Buffer::with_size(6).unwrap();
                {
                    let buffer = buffer.get_mut().unwrap();
                    println!("Producing buffer {}", buffer.size());
                    buffer.copy_from_slice(0, &[0, 1, 2, 3, 4, 5]).unwrap();
                    buffer.set_pts(i * 500 * gst::ClockTime::MSECOND);
                }
                i += 1;

                // appsrc already handles the error here for us.
                let _ = appsrc.push_buffer(buffer);
            })
            .build(),
    );

    let videosrc_caps = gst::Caps::builder("video/x-raw")
        .field("width", 320)
        .field("height", 240)
        // .field("framerate", "30/1")
        .field("format", "I420")
        .build();

    // pipeline.add(&appsrc).unwrap();
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
    gst::Element::link_many(&[&x264enc, &h264parse_src]).unwrap();
    gst::Element::link_many(&[&mpegtsmux, &tsdemux]).unwrap();
    gst::Element::link_many(&[&h264parse_dest, &avdec_h264, &videoconvert, &ximagesink]).unwrap();

    let h264parse_src_pad = h264parse_src.static_pad("src").unwrap();
    let mpegtsmux_sink = mpegtsmux.request_pad_simple("sink_%d").unwrap();
    h264parse_src_pad.link(&mpegtsmux_sink).unwrap();

    let mpegtsmux_sink_klv = mpegtsmux.request_pad_simple("sink_%d").unwrap();
    let appsrc_pad = appsrc.static_pad("src").unwrap();
    appsrc_pad.link(&mpegtsmux_sink_klv).unwrap();
    // appsrc.link_filtered(&mpegtsmux, &klv_caps).unwrap();

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

fn only_klv() {
    gst::init().unwrap();
    let pipeline = gst::Pipeline::new(None);
    let mpegtsmux = gst::ElementFactory::make("mpegtsmux", None).unwrap();
    let tsdemux = gst::ElementFactory::make("tsdemux", None).unwrap();
    let fakesink = gst::ElementFactory::make("fakesink", None).unwrap();

    let appsrc = gst::ElementFactory::make("appsrc", None)
        .unwrap()
        .downcast::<gst_app::AppSrc>()
        .unwrap();
    let klv_caps = gst::Caps::builder("meta/x-klv")
        .field("parsed", "true")
        .build();

    appsrc.set_caps(Some(&klv_caps));
    let mut i = 0;
    appsrc.set_callbacks(
        gst_app::AppSrcCallbacks::builder()
            .need_data(move |appsrc, _| {
                // Add a custom meta with a label to this buffer.
                let mut buffer = gst::Buffer::with_size(6).unwrap();
                {
                    let buffer = buffer.get_mut().unwrap();
                    println!("Producing buffer {}", buffer.size());
                    buffer.copy_from_slice(0, &[0, 1, 2, 3, 4, 5]).unwrap();
                    // buffer.set_pts(i * 500 * gst::ClockTime::MSECOND);
                }
                i += 1;

                // appsrc already handles the error here for us.
                let _ = appsrc.push_buffer(buffer);
            })
            .build(),
    );

    pipeline.add(&appsrc).unwrap();
    pipeline
        .add_many(&[&mpegtsmux, &tsdemux, &fakesink])
        .unwrap();

    gst::Element::link_many(&[&mpegtsmux, &tsdemux]).unwrap();
    let mpegtsmux_sink_klv = mpegtsmux.request_pad_simple("sink_%d").unwrap();
    let appsrc_pad = appsrc.static_pad("src").unwrap();
    appsrc_pad.link(&mpegtsmux_sink_klv).unwrap();
    // appsrc.link_filtered(&mpegtsmux, &klv_caps).unwrap();

    let fakesink_pad = fakesink
        .static_pad("sink")
        .expect("failed to get fakesink pad.");

    tsdemux.connect_pad_added(move |src, src_pad| {
        info!("Received new pad {} from {}", src_pad.name(), src.name());
        src_pad.link(&fakesink_pad).unwrap();
        // if src_pad.name().contains("video") {
        //     src_pad.link(&h264_sink_pad).unwrap();
        // }
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


fn only_fake(){
    // TODO Appsrcがcaps通じてnego出来るのを確認する
    // app src sinkの対から
    // appsrc 固定sink
    // appsrc mux
    gst::init().unwrap();

    let pipeline = gst::Pipeline::new(None);
    let klv_caps = gst::Caps::builder("meta/x-klv")
        .field("parsed", "true")
        .build();

    let appsrc = gst::ElementFactory::make("appsrc", None)
        .unwrap()
        .downcast::<gst_app::AppSrc>()
        .unwrap();

    let appsink = gst::ElementFactory::make("appsink", None)
        .unwrap()
        .downcast::<gst_app::AppSink>()
        .unwrap();
    
    let mut i = 0;
    appsrc.set_callbacks(
        gst_app::AppSrcCallbacks::builder()
            .need_data(move |appsrc, _| {
                if i > 5 {
                    let _ = appsrc.end_of_stream();
                    return
                }
                // Add a custom meta with a label to this buffer.
                let mut buffer = gst::Buffer::with_size(6).unwrap();
                {
                    let buffer = buffer.get_mut().unwrap();
                    println!("Producing buffer {}", buffer.size());
                    buffer.copy_from_slice(0, &[0, 1, 2, 3, 4, 5]).unwrap();
                    // buffer.set_pts(i * 500 * gst::ClockTime::MSECOND);
                }
                i += 1;

                // appsrc already handles the error here for us.
                let _ = appsrc.push_buffer(buffer);
            })
            .build(),
    );

    // build appsink
    appsink.set_caps(Some(&klv_caps));
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

    pipeline.add(&appsrc).unwrap();
    pipeline.add(&appsink).unwrap();
    appsrc.link(&appsink).unwrap();


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
    Klv,
    Fake,
}
fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cmd = Cmd::from_args();
    log::debug!("cmd {:?}", &cmd);
    match cmd {
        Cmd::App => include_klv(),
        Cmd::Klv => only_klv(),
        Cmd::Fake => only_fake(),
        Cmd::Decode => decode_mpegtsklv(),
    }
}
