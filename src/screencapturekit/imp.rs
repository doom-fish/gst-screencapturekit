use gst::glib;
use gst::subclass::prelude::*;
use gst_base::prelude::*;
use gst_base::subclass::prelude::*;
use screencapturekit::output::sc_stream_frame_info::SCFrameStatus;
use screencapturekit::output::sc_stream_frame_info::SCStreamFrameInfo;
use screencapturekit::output::CMSampleBuffer;
use screencapturekit::output::LockTrait;
use screencapturekit::shareable_content::SCShareableContent;
use screencapturekit::stream::configuration::pixel_format::PixelFormat;
use screencapturekit::stream::configuration::SCStreamConfiguration;
use screencapturekit::stream::content_filter::SCContentFilter;
use screencapturekit::stream::output_trait::SCStreamOutputTrait;
use screencapturekit::stream::output_type::SCStreamOutputType;
use screencapturekit::stream::SCStream;

use std::io::Read;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::Mutex;

#[derive(Default)]
enum State {
    #[default]
    Stopped,
    Started {
        stream: SCStream,
    },
}

#[derive(Default)]
pub struct ScreenCaptureSrc {
    receiver: Mutex<Option<Receiver<CMSampleBuffer>>>,
    sender: Mutex<Option<Sender<CMSampleBuffer>>>,
    state: Mutex<State>,
}

use std::sync::LazyLock;

static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "screencapture",
        gst::DebugColorFlags::all(),
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
            let caps = gst_video::VideoCapsBuilder::new()
                .format(gst_video::VideoFormat::Bgra)
                .width(1440)
                .height(2560)
                .build();
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
impl SCStreamOutputTrait for &ScreenCaptureSrc {
    fn did_output_sample_buffer(
        &self,
        sample_buffer: CMSampleBuffer,
        _of_type: SCStreamOutputType,
    ) {
        let info = SCStreamFrameInfo::from_sample_buffer(&sample_buffer).expect("should work");

        if info.status() == SCFrameStatus::Complete {
            let b = self.sender.lock().expect("should work");
            if let Some(ref sender) = *b {
                gst::info!(CAT, imp = self, "Sent");
                sender.send(sample_buffer).ok();
            }
        }
    }
}
impl BaseSrcImpl for ScreenCaptureSrc {
    fn is_seekable(&self) -> bool {
        false
    }

    fn start(&self) -> Result<(), gst::ErrorMessage> {
        let mut state = self.state.lock().unwrap();
        if let Ok(config) = SCStreamConfiguration::new()
            .set_width(1440)
            .and_then(|x| x.set_height(2560))
            .and_then(|x| x.set_pixel_format(PixelFormat::BGRA))
        {
            let display = SCShareableContent::get().unwrap().displays().remove(1);
            let filter = SCContentFilter::new().with_display_excluding_windows(&display, &[]);
            let mut stream = SCStream::new(&filter, &config);
            stream.add_output_handler(self, SCStreamOutputType::Screen);

            let (sender, receiver) = std::sync::mpsc::channel();

            let mut send = self.sender.lock().expect("should work");
            let mut recv = self.receiver.lock().expect("should work");

            *send = Some(sender);
            *recv = Some(receiver);

            stream.start_capture().expect("should work");
            *state = State::Started { stream };
        }
        gst::info!(CAT, imp = self, "Started");
        Ok(())
    }

    fn stop(&self) -> Result<(), gst::ErrorMessage> {
        let mut state = self.state.lock().unwrap();
        if let State::Stopped = *state {
            return Err(gst::error_msg!(
                gst::ResourceError::Settings,
                ["Fil Internal data stream erroreSrc not started"]
            ));
        }

        if let State::Started { ref stream, .. } = *state {
            stream.stop_capture().ok();
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
        let b = self.receiver.lock().expect("should work");
        if let Some(ref receiver) = *b {
            let b = receiver.recv().expect("should get");
            let pb = b.get_pixel_buffer().expect("should work");

            let internal_base_address = pb.lock().expect("TEST");
            let mut curs = internal_base_address.as_cursor();
            let size = {
                let mut map = buffer.map_writable().map_err(|_| {
                    gst::element_imp_error!(
                        self,
                        gst::LibraryError::Failed,
                        ["Failed to map buffer"]
                    );
                    gst::FlowError::Error
                })?;

                curs.read(map.as_mut()).map_err(|err| {
                    gst::element_imp_error!(
                        self,
                        gst::LibraryError::Failed,
                        ["Failed to read at {}: {}", offset, err.to_string()]
                    );
                    gst::FlowError::Error
                })?
            };

            gst_video::VideoMeta::add_full(
                buffer,
                gst_video::VideoFrameFlags::empty(),
                gst_video::VideoFormat::Bgra,
                pb.get_width(),
                pb.get_height(),
                &[0],
                &[pb.get_bytes_per_row() as i32],
            )
            .ok();
            buffer.set_size(size);
        }

        Ok(gst::FlowSuccess::Ok)
    }
}
