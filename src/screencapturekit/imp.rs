use gst::glib;
use gst::subclass::prelude::*;
use gst_base::prelude::*;
use gst_base::subclass::prelude::*;

use std::sync::Mutex;

#[derive(Debug)]
struct Settings {}

impl Default for Settings {
    fn default() -> Self {
        Settings {}
    }
}

#[derive(Default)]
enum State {
    #[default]
    Stopped,
    Started {},
}

#[derive(Default)]
pub struct ScreenCaptureSrc {
    settings: Mutex<Settings>,
    state: Mutex<State>,
}

use std::sync::LazyLock;
static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "screencapture",
        gst::DebugColorFlags::empty(),
        Some("ScreenCapture Source"),
    )
});

impl ScreenCaptureSrc {}

#[glib::object_subclass]
impl ObjectSubclass for ScreenCaptureSrc {
    const NAME: &'static str = "GstScreenCaptureSrc";
    type Type = super::ScreenCaptureSrc;
    type ParentType = gst_base::BaseSrc;
}

impl ObjectImpl for ScreenCaptureSrc {
    fn constructed(&self) {
        self.parent_constructed();
        self.obj().set_format(gst::Format::Bytes);
    }
}

impl GstObjectImpl for ScreenCaptureSrc {}

impl ElementImpl for ScreenCaptureSrc {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static ELEMENT_METADATA: LazyLock<gst::subclass::ElementMetadata> = LazyLock::new(|| {
            gst::subclass::ElementMetadata::new(
                "ScreenCaptureSrc",
                "Source/ScreenCapture",
                "Capture screen, audio and or microphone",
                "Per Johansson <per@doom.fish>",
            )
        });

        Some(&*ELEMENT_METADATA)
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: LazyLock<Vec<gst::PadTemplate>> = LazyLock::new(|| {
            let caps = gst::Caps::new_any();
            let src_pad_template = gst::PadTemplate::new(
                "src",
                gst::PadDirection::Src,
                gst::PadPresence::Always,
                &caps,
            )
            .unwrap();

            vec![src_pad_template]
        });

        PAD_TEMPLATES.as_ref()
    }
}

impl BaseSrcImpl for ScreenCaptureSrc {
    fn is_seekable(&self) -> bool {
        false
    }

    fn start(&self) -> Result<(), gst::ErrorMessage> {
        let mut state = self.state.lock().unwrap();
        if let State::Started { .. } = *state {
            unreachable!("FileSrc already started");
        }

        let settings = self.settings.lock().unwrap();

        gst::info!(CAT, imp = self, "Started");

        Ok(())
    }

    fn stop(&self) -> Result<(), gst::ErrorMessage> {
        let mut state = self.state.lock().unwrap();
        if let State::Stopped = *state {
            return Err(gst::error_msg!(
                gst::ResourceError::Settings,
                ["FileSrc not started"]
            ));
        }

        *state = State::Stopped;

        gst::info!(CAT, imp = self, "Stopped");

        Ok(())
    }

    fn fill(
        &self,
        offset: u64,
        _length: u32,
        buffer: &mut gst::BufferRef,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        let mut state = self.state.lock().unwrap();

        Ok(gst::FlowSuccess::Ok)
    }
}
