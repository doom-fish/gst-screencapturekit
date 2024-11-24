use gst::glib;
use gst::prelude::*;

mod imp;

glib::wrapper! {
    pub struct ScreenCaptureSrc(ObjectSubclass<imp::ScreenCaptureSrc>) @extends gst_base::BaseSrc, gst::Element, gst::Object;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "screencapturesrc",
        gst::Rank::NONE,
        ScreenCaptureSrc::static_type(),
    )
}
