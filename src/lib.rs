use gst::glib;

gst::plugin_define!(
    screencapturekit,
    env!("CARGO_PKG_DESCRIPTION"),
    init,
    concat!(env!("CARGO_PKG_VERSION"), "-", env!("COMMIT_ID")),
    "MIT/X11",
    env!("CARGO_PKG_NAME"),
    env!("CARGO_PKG_NAME"),
    env!("CARGO_PKG_REPOSITORY"),
    env!("BUILD_REL_DATE")
);

pub fn init(_plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    Ok(())
}
