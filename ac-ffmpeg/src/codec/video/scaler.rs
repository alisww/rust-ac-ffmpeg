//! Video frame scaler.

use std::os::raw::{c_int, c_void};

use crate::{
    codec::video::{PixelFormat, VideoFrame},
    Error,
};

extern "C" {
    fn ffw_frame_scaler_new(
        sformat: c_int,
        swidth: c_int,
        sheight: c_int,
        tformat: c_int,
        twidth: c_int,
        theight: c_int,
        flags: c_int,
    ) -> *mut c_void;

    fn ffw_frame_scaler_scale(scaler: *mut c_void, src: *const c_void) -> *mut c_void;

    fn ffw_frame_scaler_free(scaler: *mut c_void);
}

/// Scaling algorithm.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(C)]
pub enum Algorithm {
    FastBilinear = 1,
    Bilinear = 2,
    Bicubic = 4,
    Experimental = 8,
    Point = 0x10,
    Area = 0x20,
    BicubicLinear = 0x40,
    Gauss = 0x80,
    Sinc = 0x100,
    Lanczos = 0x200,
    Spline = 0x400,
}

/// Builder for a video frame scaler.
pub struct VideoFrameScalerBuilder {
    sformat: c_int,
    swidth: c_int,
    sheight: c_int,

    tformat: Option<c_int>,
    twidth: c_int,
    theight: c_int,

    flags: c_int,
}

impl VideoFrameScalerBuilder {
    /// Create a new video frame scaler builder.
    fn new() -> Self {
        let default_algorithm = Algorithm::Bicubic;

        let flags = default_algorithm as c_int;

        Self {
            sformat: -1,
            swidth: 0,
            sheight: 0,

            tformat: None,
            twidth: 0,
            theight: 0,

            flags,
        }
    }

    /// Set source pixel format.
    pub fn source_pixel_format(mut self, format: PixelFormat) -> Self {
        self.sformat = format.into_raw();
        self
    }

    /// Set source frame width.
    pub fn source_width(mut self, width: usize) -> Self {
        self.swidth = width as _;
        self
    }

    /// Set source frame height.
    pub fn source_height(mut self, height: usize) -> Self {
        self.sheight = height as _;
        self
    }

    /// Set target pixel format. The default is equal to the source format.
    pub fn target_pixel_format(mut self, format: PixelFormat) -> Self {
        self.tformat = Some(format.into_raw());
        self
    }

    /// Set target frame width.
    pub fn target_width(mut self, width: usize) -> Self {
        self.twidth = width as _;
        self
    }

    /// Set target frame height.
    pub fn target_height(mut self, height: usize) -> Self {
        self.theight = height as _;
        self
    }

    /// Set scaling algorithm. The default is bicubic.
    pub fn algorithm(mut self, algorithm: Algorithm) -> Self {
        self.flags = algorithm as c_int;

        self
    }

    /// Build the video frame scaler.
    pub fn build(self) -> Result<VideoFrameScaler, Error> {
        let tformat = self.tformat.unwrap_or(self.sformat);

        if self.sformat < 0 {
            return Err(Error::new("invalid source format"));
        } else if tformat < 0 {
            return Err(Error::new("invalid target format"));
        } else if self.swidth < 1 {
            return Err(Error::new("invalid source width"));
        } else if self.sheight < 1 {
            return Err(Error::new("invalid source height"));
        } else if self.twidth < 1 {
            return Err(Error::new("invalid target width"));
        } else if self.theight < 1 {
            return Err(Error::new("invalid target height"));
        }

        let ptr = unsafe {
            ffw_frame_scaler_new(
                self.sformat,
                self.swidth,
                self.sheight,
                tformat,
                self.twidth,
                self.theight,
                self.flags,
            )
        };

        if ptr.is_null() {
            return Err(Error::new("unable to create a frame scaler"));
        }

        let res = VideoFrameScaler {
            ptr,

            sformat: PixelFormat::from_raw(self.sformat),
            swidth: self.swidth as _,
            sheight: self.sheight as _,
        };

        Ok(res)
    }
}

/// Video frame scaler.
pub struct VideoFrameScaler {
    ptr: *mut c_void,

    sformat: PixelFormat,
    swidth: usize,
    sheight: usize,
}

impl VideoFrameScaler {
    /// Get a frame scaler builder.
    pub fn builder() -> VideoFrameScalerBuilder {
        VideoFrameScalerBuilder::new()
    }

    /// Scale a given frame.
    pub fn scale(&mut self, frame: &VideoFrame) -> Result<VideoFrame, Error> {
        if self.swidth != frame.width() {
            return Err(Error::new("frame width does not match"));
        } else if self.sheight != frame.height() {
            return Err(Error::new("frame height does not match"));
        } else if self.sformat != frame.pixel_format() {
            return Err(Error::new("frame pixel format does not match"));
        }

        let res = unsafe { ffw_frame_scaler_scale(self.ptr, frame.as_ptr()) };

        if res.is_null() {
            panic!("unable to scale a frame");
        }

        let frame = unsafe { VideoFrame::from_raw_ptr(res, frame.time_base()) };

        Ok(frame)
    }
}

impl Drop for VideoFrameScaler {
    fn drop(&mut self) {
        unsafe { ffw_frame_scaler_free(self.ptr) }
    }
}

unsafe impl Send for VideoFrameScaler {}
unsafe impl Sync for VideoFrameScaler {}
