//! A/V stream information.

use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    os::raw::{c_char, c_int, c_void},
    ptr,
};

use crate::{
    codec::CodecParameters,
    packet::{SideDataRef, SideDataType},
    time::{TimeBase, Timestamp},
    Error,
};

extern "C" {
    fn ffw_stream_get_time_base(stream: *const c_void, num: *mut u32, den: *mut u32);
    fn ffw_stream_set_time_base(stream: *mut c_void, num: u32, den: u32);
    fn ffw_stream_get_start_time(stream: *const c_void) -> i64;
    fn ffw_stream_get_duration(stream: *const c_void) -> i64;
    fn ffw_stream_get_nb_frames(stream: *const c_void) -> i64;
    fn ffw_stream_get_r_frame_rate(stream: *const c_void) -> f64;
    fn ffw_stream_get_codec_parameters(stream: *const c_void) -> *mut c_void;
    fn ffw_stream_get_id(stream: *const c_void) -> c_int;
    fn ffw_stream_set_metadata(
        stream: *mut c_void,
        key: *const c_char,
        value: *const c_char,
    ) -> c_int;

    fn ffw_stream_get_metadata_entry(
        stream: *const c_void,
        key: *const c_char,
        prev: *const c_void,
        flags: c_int,
    ) -> *const c_void;
    fn ffw_stream_get_metadata_entry_value(entry: *const c_void) -> *const c_char;
    fn ffw_stream_get_metadata_entry_key(entry: *const c_void) -> *const c_char;
    fn ffw_stream_set_id(stream: *mut c_void, id: c_int);
    fn ffw_stream_get_nb_side_data(stream: *const c_void) -> usize;
    fn ffw_stream_get_side_data(stream: *const c_void, index: usize) -> *const c_void;
    fn ffw_stream_add_side_data(
        stream: *mut c_void,
        data_type: c_int,
        data: *const u8,
        size: usize,
    ) -> c_int;
}

/// Stream.
pub struct Stream {
    ptr: *mut c_void,
    time_base: TimeBase,
}

impl Stream {
    /// Create a new stream from its raw representation.
    pub(crate) unsafe fn from_raw_ptr(ptr: *mut c_void) -> Self {
        let mut num = 0_u32;
        let mut den = 0_u32;

        ffw_stream_get_time_base(ptr, &mut num, &mut den);

        Stream {
            ptr,
            time_base: TimeBase::new(num, den),
        }
    }

    /// Get stream time base.
    pub fn time_base(&self) -> TimeBase {
        self.time_base
    }

    /// Provide a hint to the muxer about the desired timebase.
    pub fn set_time_base(&mut self, time_base: TimeBase) {
        self.time_base = time_base;
        unsafe {
            ffw_stream_set_time_base(self.ptr, self.time_base.num(), self.time_base.den());
        }
    }

    /// Get the pts of the first frame of the stream in presentation order.
    pub fn start_time(&self) -> Timestamp {
        let pts = unsafe { ffw_stream_get_start_time(self.ptr) as _ };

        Timestamp::new(pts, self.time_base)
    }

    /// Get the duration of the stream.
    pub fn duration(&self) -> Timestamp {
        let pts = unsafe { ffw_stream_get_duration(self.ptr) as _ };

        Timestamp::new(pts, self.time_base)
    }

    /// Get the number of frames in the stream.
    ///
    /// # Note
    /// The number may not represent the total number of frames, depending on the type of the
    /// stream and the demuxer it may represent only the total number of keyframes.
    pub fn frames(&self) -> Option<u64> {
        let count = unsafe { ffw_stream_get_nb_frames(self.ptr) };

        if count <= 0 {
            None
        } else {
            Some(count as _)
        }
    }

    pub fn real_frame_rate(&self) -> Option<f64> {
        let fps = unsafe { ffw_stream_get_r_frame_rate(self.ptr) };

        if fps <= 0.0 {
            None
        } else {
            Some(fps as _)
        }
    }

    /// Get codec parameters.
    pub fn codec_parameters(&self) -> CodecParameters {
        unsafe {
            let ptr = ffw_stream_get_codec_parameters(self.ptr);

            if ptr.is_null() {
                panic!("unable to allocate codec parameters");
            }

            CodecParameters::from_raw_ptr(ptr)
        }
    }

    /// Get stream id.
    pub fn stream_id(&self) -> i32 {
        unsafe { ffw_stream_get_id(self.ptr) as i32 }
    }

    /// Set stream metadata.
    pub fn set_metadata<V>(&mut self, key: &str, value: V)
    where
        V: ToString,
    {
        let key = CString::new(key).expect("invalid metadata key");
        let value = CString::new(value.to_string()).expect("invalid metadata value");

        let ret = unsafe { ffw_stream_set_metadata(self.ptr, key.as_ptr(), value.as_ptr()) };

        if ret < 0 {
            panic!("unable to allocate metadata");
        }
    }

    pub fn get_metadata(&self, key: &str) -> Option<&'static str> {
        unsafe {
            let key = CString::new(key).expect("invalid metadata key");
            let ptr = ffw_stream_get_metadata_entry(self.ptr, key.as_ptr(), ptr::null(), 0);

            if ptr.is_null() {
                None
            } else {
                let val_ptr = ffw_stream_get_metadata_entry_value(ptr);
                if val_ptr.is_null() {
                    None
                } else {
                    let val = CStr::from_ptr(val_ptr as _);
                    Some(val.to_str().unwrap())
                }
            }
        }
    }

    pub fn metadata_dict(&self) -> HashMap<&'static str, &'static str> {
        let mut res = HashMap::new();
        let nil_str = CString::new("").unwrap();
        unsafe {
            let mut prev = ptr::null();
            loop {
                prev = ffw_stream_get_metadata_entry(self.ptr, nil_str.as_ptr(), prev, 2); // AV_DICT_IGNORE_SUFFIX
                if !prev.is_null() {
                    let key_ptr = ffw_stream_get_metadata_entry_key(prev);
                    let val_ptr = ffw_stream_get_metadata_entry_value(prev);
                    if !key_ptr.is_null() && !val_ptr.is_null() {
                        let key = CStr::from_ptr(key_ptr as _);
                        let val = CStr::from_ptr(val_ptr as _);
                        res.insert(key.to_str().unwrap(), val.to_str().unwrap());
                    }
                } else {
                    break;
                }
            }
        }

        res
    }

    /// Set stream id.
    pub fn set_stream_id(&mut self, id: i32) {
        unsafe { ffw_stream_set_id(self.ptr, id as c_int) };
    }

    /// Get stream side data.
    pub fn side_data(&self) -> SideDataIter<'_> {
        let len = unsafe { ffw_stream_get_nb_side_data(self.ptr) };

        SideDataIter {
            stream: self,
            index: 0,
            len,
        }
    }

    /// Add stream side data.
    pub fn add_side_data(&mut self, data_type: SideDataType, data: &[u8]) -> Result<(), Error> {
        let ret = unsafe {
            ffw_stream_add_side_data(self.ptr, data_type.into_raw(), data.as_ptr(), data.len())
        };

        if ret < 0 {
            return Err(Error::from_raw_error_code(ret));
        }

        Ok(())
    }
}

unsafe impl Send for Stream {}
unsafe impl Sync for Stream {}

/// Iterator over stream side data.
pub struct SideDataIter<'a> {
    stream: &'a Stream,
    index: usize,
    len: usize,
}

impl<'a> Iterator for SideDataIter<'a> {
    type Item = &'a SideDataRef;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.len {
            return None;
        }

        let side_data = unsafe {
            SideDataRef::from_raw_ptr(ffw_stream_get_side_data(self.stream.ptr, self.index))
        };
        self.index += 1;

        Some(side_data)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let hint = self.len - self.index;
        (hint, Some(hint))
    }
}

impl ExactSizeIterator for SideDataIter<'_> {}
