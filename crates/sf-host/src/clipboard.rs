//! Win32 Unicode clipboard access on the game's window thread.

use std::mem::{size_of, size_of_val};
use std::num::NonZeroIsize;
use std::{ptr, slice};

use windows_sys::Win32::Foundation::{GlobalFree, HWND};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
};
use windows_sys::Win32::System::Ole::CF_UNICODETEXT;

// Room codes are short. Allow whitespace from a copied line without reading
// a large clipboard allocation into the game.
const MAX_TEXT_UNITS: usize = 1024;

struct Open;

impl Open {
    fn for_window(owner: NonZeroIsize) -> Option<Self> {
        // SAFETY: the owner is the Win32 window handle supplied by the game.
        (unsafe { OpenClipboard(owner.get() as HWND) } != 0).then(|| Self)
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        // SAFETY: Open is created only after successfully opening on this thread.
        unsafe { CloseClipboard() };
    }
}

pub fn read_text(owner: NonZeroIsize) -> Option<String> {
    let _clipboard = Open::for_window(owner)?;
    // SAFETY: the clipboard remains open while we lock its Unicode text
    // handle and copy at most the allocated number of UTF-16 units.
    unsafe {
        let handle = GetClipboardData(u32::from(CF_UNICODETEXT));
        if handle.is_null() {
            return None;
        }
        let count = (GlobalSize(handle) / size_of::<u16>()).min(MAX_TEXT_UNITS + 1);
        if count == 0 {
            return None;
        }
        let data = GlobalLock(handle).cast::<u16>();
        if data.is_null() {
            return None;
        }
        let end = (0..count).find(|&index| data.add(index).read() == 0);
        let text = end.and_then(|len| String::from_utf16(slice::from_raw_parts(data, len)).ok());
        GlobalUnlock(handle);
        text
    }
}

pub fn write_text(owner: NonZeroIsize, text: &str) -> bool {
    let Some(_clipboard) = Open::for_window(owner) else {
        return false;
    };
    let units: Vec<_> = text.encode_utf16().chain([0]).collect();
    // SAFETY: GlobalAlloc makes a block sized for the copied units. We
    // unlock it before publishing, then either transfer ownership to the
    // clipboard or free it ourselves.
    unsafe {
        let handle = GlobalAlloc(GMEM_MOVEABLE, size_of_val(units.as_slice()));
        if handle.is_null() {
            return false;
        }
        let data = GlobalLock(handle).cast::<u16>();
        if data.is_null() {
            GlobalFree(handle);
            return false;
        }
        ptr::copy_nonoverlapping(units.as_ptr(), data, units.len());
        GlobalUnlock(handle);
        if EmptyClipboard() == 0 || SetClipboardData(u32::from(CF_UNICODETEXT), handle).is_null() {
            GlobalFree(handle);
            return false;
        }
        true
    }
}
