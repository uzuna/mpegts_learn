use std::time::SystemTime;

use glib::BoolError;
use gst::{prelude::*, Caps};
use gst_app::gst::element_error;

use klv::{from_bytes, to_bytes, uasdls::UASDatalinkLS};
use log::info;

use once_cell::sync::Lazy;

/// Caps meta/x-klv for mpegtsmux and tsdemux
pub static KLV_CAPS: Lazy<Caps> = Lazy::new(|| {
    gst::Caps::builder("meta/x-klv")
        .field("parsed", true)
        .build()
});

/// UADDLSを見つけたらパースするSink
pub fn uasdls_print_sink() -> Result<gst::Element, BoolError> {
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
                    let mr = buffer.map_readable().unwrap();
                    if let Ok(res) = from_bytes::<UASDatalinkLS>(mr.as_slice()) {
                        log::info!("uasdls {:?}", res);
                    }
                }
                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );
    Ok(appsink.upcast::<gst::Element>())
}

/// UADDLSに基づいてタイムスタンプだけを埋め込んだメタデータを生成するSrc
pub fn uasdls_test_src() -> Result<gst::Element, BoolError> {
    let appsrc = gst::ElementFactory::make("appsrc", None)?
        .downcast::<gst_app::AppSrc>()
        .unwrap();
    appsrc.set_caps(Some(&KLV_CAPS));

    appsrc.set_format(gst::Format::Time);
    // TODO 決まったタイミングでデータを送る方法
    // instantからPTS自体は作れそう
    // データ生成周期の作り方を確認する。今は500msのPTSを入れるとイベント発火が制限されて結果的に2Hz周期になっている
    let mut i = 0;
    appsrc.set_callbacks(
        gst_app::AppSrcCallbacks::builder()
            .need_data(move |appsrc, _| {
                let records = UASDatalinkLS {
                    timestamp: SystemTime::now(),
                    ..Default::default()
                };
                let data = to_bytes(&records).unwrap();

                let mut buffer = gst::Buffer::with_size(data.len()).unwrap();
                {
                    let bufref = buffer.make_mut();
                    bufref.set_pts(i * 500 * gst::ClockTime::MSECOND);
                    let mut mw = bufref.map_writable().unwrap();
                    mw.as_mut_slice().copy_from_slice(&data)
                }

                info!("sending buffer: {}", buffer.size());
                i += 1;

                // appsrc already handles the error here for us.
                let _ = appsrc.push_buffer(buffer);
            })
            .build(),
    );

    Ok(appsrc.upcast::<gst::Element>())
}
