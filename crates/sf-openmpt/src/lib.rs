//! libopenmpt (BSD-3-Clause; version 0.8.10, vendored and built by
//! `build.rs`) through the handful of C calls the Jukebox needs (ADR-0009,
//! design section 6a.5, ticket #201 `ST9`): load a tracker module from
//! memory, set how often it repeats, and render it as stereo `f32`.
//!
//! libopenmpt reads, by content and not by name, every format its project
//! lists: MOD, XM, S3M, IT, MPTM, 669, MTM, and some forty more (see
//! [`supported_extensions`]).
//!
//! Only the C API's few calls used are declared; no `bindgen`, no libclang.

use std::ffi::{c_char, c_double, c_int, c_void, CStr, CString};
use std::ptr::NonNull;

#[allow(non_camel_case_types)]
type openmpt_module = c_void;

extern "C" {
    fn openmpt_get_library_version() -> u32;
    fn openmpt_get_supported_extensions() -> *const c_char;
    fn openmpt_free_string(string: *const c_char);
    fn openmpt_log_func_silent(message: *const c_char, user: *mut c_void);
    fn openmpt_module_create_from_memory2(
        filedata: *const c_void,
        filesize: usize,
        logfunc: Option<unsafe extern "C" fn(*const c_char, *mut c_void)>,
        loguser: *mut c_void,
        errfunc: Option<unsafe extern "C" fn(c_int, *mut c_void) -> c_int>,
        erruser: *mut c_void,
        error: *mut c_int,
        error_message: *mut *const c_char,
        ctls: *const c_void,
    ) -> *mut openmpt_module;
    fn openmpt_module_ctl_set_text(module: *mut openmpt_module, ctl: *const c_char, value: *const c_char) -> c_int;
    fn openmpt_module_destroy(module: *mut openmpt_module);
    fn openmpt_module_set_repeat_count(module: *mut openmpt_module, repeat_count: i32) -> c_int;
    fn openmpt_module_read_interleaved_float_stereo(module: *mut openmpt_module, samplerate: i32, count: usize, interleaved_stereo: *mut f32) -> usize;
    fn openmpt_module_get_metadata(module: *mut openmpt_module, key: *const c_char) -> *const c_char;
    fn openmpt_module_get_restart_order(module: *mut openmpt_module, subsong: i32) -> i32;
    fn openmpt_module_get_restart_row(module: *mut openmpt_module, subsong: i32) -> i32;
    fn openmpt_module_error_get_last(module: *mut openmpt_module) -> c_int;
    fn openmpt_module_error_get_last_message(module: *mut openmpt_module) -> *const c_char;
    fn openmpt_module_error_clear(module: *mut openmpt_module);
    fn openmpt_module_get_num_orders(module: *mut openmpt_module) -> i32;
    fn openmpt_module_get_current_order(module: *mut openmpt_module) -> i32;
    fn openmpt_module_get_current_row(module: *mut openmpt_module) -> i32;
    fn openmpt_module_get_position_seconds(module: *mut openmpt_module) -> c_double;
    fn openmpt_module_set_position_order_row(module: *mut openmpt_module, order: i32, row: i32) -> c_double;
}

/// libopenmpt's version number: `major << 24 | minor << 16 | patch` (0.8.10 is `0x0008000a`).
pub fn library_version() -> u32 {
    unsafe { openmpt_get_library_version() }
}

/// The file extensions libopenmpt knows, lower case, without the dot.
pub fn supported_extensions() -> Vec<String> {
    unsafe {
        let list = openmpt_get_supported_extensions();
        if list.is_null() {
            return Vec::new();
        }
        let text = CStr::from_ptr(list).to_string_lossy().into_owned();
        openmpt_free_string(list);
        text.split(';').filter(|e| !e.is_empty()).map(str::to_owned).collect()
    }
}

/// A loaded module.
pub struct Module {
    raw: NonNull<openmpt_module>,
}

// A module has no thread affinity; it is only ever used by one thread at a time (`&mut self`).
unsafe impl Send for Module {}

impl Drop for Module {
    fn drop(&mut self) {
        unsafe { openmpt_module_destroy(self.raw.as_ptr()) }
    }
}

impl Module {
    /// Load the module in `data`, whatever its format. libopenmpt reads this
    /// by content, so a wrong or missing file name does not matter. Errors
    /// carry libopenmpt's own message. Nothing is printed.
    pub fn from_memory(data: &[u8]) -> Result<Module, String> {
        let (mut code, mut message): (c_int, *const c_char) = (0, std::ptr::null());
        let raw = unsafe {
            openmpt_module_create_from_memory2(
                data.as_ptr().cast(),
                data.len(),
                Some(openmpt_log_func_silent),
                std::ptr::null_mut(),
                None,
                std::ptr::null_mut(),
                &mut code,
                &mut message,
                std::ptr::null(),
            )
        };
        let text = if message.is_null() {
            None
        } else {
            let text = unsafe { CStr::from_ptr(message) }.to_string_lossy().into_owned();
            unsafe { openmpt_free_string(message) };
            Some(text)
        };
        match NonNull::new(raw) {
            Some(raw) => Ok(Module { raw }),
            None => Err(text.unwrap_or_else(|| format!("libopenmpt error {code}"))),
        }
    }

    /// How many more times the song plays after its first play-through: 0
    /// plays it once, `-1` for ever. Each repeat starts at the module's own
    /// restart position, which libopenmpt handles.
    pub fn set_repeat_count(&mut self, count: i32) {
        unsafe { openmpt_module_set_repeat_count(self.raw.as_ptr(), count) };
    }

    /// Set a libopenmpt control by name, such as `play.at_end` to `stop` (the
    /// default, `fadeout`, adds a tail after the song's end). Returns whether
    /// libopenmpt took it.
    pub fn set_control(&mut self, name: &str, value: &str) -> bool {
        let (Ok(name), Ok(value)) = (CString::new(name), CString::new(value)) else { return false };
        unsafe { openmpt_module_ctl_set_text(self.raw.as_ptr(), name.as_ptr(), value.as_ptr()) != 0 }
    }

    /// Render up to `out.len() / 2` stereo frames at `rate` Hz into `out`
    /// (interleaved), returning the number of frames written. `Ok(0)` means
    /// the song has ended (at its repeat count); an `Err` is libopenmpt
    /// stopping on an internal error, which also returns no frames.
    pub fn read_stereo(&mut self, rate: u32, out: &mut [f32]) -> Result<usize, String> {
        unsafe {
            openmpt_module_error_clear(self.raw.as_ptr());
            let n = openmpt_module_read_interleaved_float_stereo(self.raw.as_ptr(), rate as i32, out.len() / 2, out.as_mut_ptr());
            if n == 0 && openmpt_module_error_get_last(self.raw.as_ptr()) != 0 {
                let message = openmpt_module_error_get_last_message(self.raw.as_ptr());
                let text = if message.is_null() {
                    "libopenmpt stopped on an error".to_owned()
                } else {
                    let text = CStr::from_ptr(message).to_string_lossy().into_owned();
                    openmpt_free_string(message);
                    text
                };
                return Err(text);
            }
            Ok(n)
        }
    }

    /// Where the song goes back to once it has played to its end, in the
    /// first sub-song: `(order, row)`, as libopenmpt works it out from the
    /// module's restart position and from jumps back that make it loop.
    /// `None` if it reports none.
    pub fn restart_position(&self) -> Option<(i32, i32)> {
        let (order, row) = unsafe { (openmpt_module_get_restart_order(self.raw.as_ptr(), 0), openmpt_module_get_restart_row(self.raw.as_ptr(), 0)) };
        (order >= 0 && row >= 0).then_some((order, row))
    }

    /// A metadata field (`type`, `type_long`, `title`, ...), if the module has it.
    pub fn metadata(&self, key: &str) -> Option<String> {
        let key = CString::new(key).ok()?;
        unsafe {
            let value = openmpt_module_get_metadata(self.raw.as_ptr(), key.as_ptr());
            if value.is_null() {
                return None;
            }
            let text = CStr::from_ptr(value).to_string_lossy().into_owned();
            openmpt_free_string(value);
            Some(text).filter(|t| !t.is_empty())
        }
    }

    pub fn num_orders(&self) -> i32 {
        unsafe { openmpt_module_get_num_orders(self.raw.as_ptr()) }
    }

    /// The order and row being played.
    pub fn position(&self) -> (i32, i32) {
        unsafe { (openmpt_module_get_current_order(self.raw.as_ptr()), openmpt_module_get_current_row(self.raw.as_ptr())) }
    }

    pub fn seconds(&self) -> f64 {
        unsafe { openmpt_module_get_position_seconds(self.raw.as_ptr()) }
    }

    /// Move to `order`, `row`; returns the time there, in seconds.
    pub fn seek_order_row(&mut self, order: i32, row: i32) -> f64 {
        unsafe { openmpt_module_set_position_order_row(self.raw.as_ptr(), order, row) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_library_is_built_in_and_lists_the_tracker_formats() {
        assert_eq!(library_version() >> 16, 0x0008, "the vendored release is libopenmpt 0.8");
        let all = supported_extensions();
        for ext in ["mod", "xm", "s3m", "it", "mptm", "669", "mtm"] {
            assert!(all.iter().any(|e| e == ext), "{ext} in {all:?}");
        }
    }

    #[test]
    fn garbage_is_an_error_not_a_crash() {
        assert!(Module::from_memory(b"this is not a module").is_err());
        assert!(Module::from_memory(&[]).is_err());
    }
}
