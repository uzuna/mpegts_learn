use std::time::SystemTime;

use glib::BoolError;
use gst::{prelude::*, Caps};
use gst_app::gst::element_error;

use klv::{
    uasdms::{UASDataset, LS_UNIVERSAL_KEY0601_8_10},
    KLVGlobal, KLVReader,
};
use log::{info, warn};

use once_cell::sync::Lazy;

/// Caps meta/x-klv for mpegtsmux and tsdemux
pub static KLV_CAPS: Lazy<Caps> = Lazy::new(|| {
    gst::Caps::builder("meta/x-klv")
        .field("parsed", true)
        .build()
});

pub fn uasds_print_sink() -> Result<gst::Element, BoolError> {
    let appsink = gst::ElementFactory::make("appsink", None)?
        .downcast::<gst_app::AppSink>()
        .unwrap();
    appsink.set_caps(Some(&KLV_CAPS));
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
                    // info!("buffer {:?}", slice);
                    if let Ok(klvg) = KLVGlobal::try_from_bytes(&slice) {
                        if klvg.key_is(&LS_UNIVERSAL_KEY0601_8_10) {
                            let r = KLVReader::<UASDataset>::from_bytes(klvg.content());
                            for x in r {
                                info!("  uas ds {:?} {:?}", x.key(), x.parse());
                            }
                        } else {
                            warn!("unknown key {:?}", &slice[..16]);
                        }
                    } else {
                        warn!("unknown data {:?}", &slice);
                    }
                }
                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );
    Ok(appsink.upcast::<gst::Element>())
}

pub fn uasds_test_src() -> Result<gst::Element, BoolError> {
    let appsrc = gst::ElementFactory::make("appsrc", None)?
        .downcast::<gst_app::AppSrc>()
        .unwrap();
    appsrc.set_caps(Some(&KLV_CAPS));

    appsrc.set_format(gst::Format::Time);
    let mut i = 0;
    appsrc.set_callbacks(
        gst_app::AppSrcCallbacks::builder()
            .need_data(move |appsrc, _| {
                use klv::uasdms::{encode, encode_len, Value};
                let records = [(UASDataset::Timestamp, Value::Timestamp(SystemTime::now()))];

                let expect_buffer_size = encode_len(&records);

                let mut buffer = gst::Buffer::with_size(expect_buffer_size).unwrap();
                {
                    let mut write_buf = vec![0; expect_buffer_size];
                    let buffer = buffer.get_mut().unwrap();
                    encode(&mut write_buf, &records).unwrap();
                    buffer.copy_from_slice(0, &write_buf).unwrap();
                    buffer.set_pts(i * 500 * gst::ClockTime::MSECOND);
                    info!("sending buffer: {}", buffer.size());
                }
                i += 1;

                // appsrc already handles the error here for us.
                let _ = appsrc.push_buffer(buffer);
            })
            .build(),
    );

    Ok(appsrc.upcast::<gst::Element>())
}
