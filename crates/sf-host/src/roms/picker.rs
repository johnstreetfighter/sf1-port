//! Native file and folder dialogs for the missing-ROM screen.

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use windows::core::{w, HRESULT};
use windows::Win32::Foundation::{ERROR_CANCELLED, HWND};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FileOpenDialog, IFileOpenDialog, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FOS_NOCHANGEDIR,
    FOS_PATHMUSTEXIST, FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
};

use super::BrowseKind;
use crate::gpu_window::WindowState;

struct Apartment;

impl Apartment {
    fn enter() -> windows::core::Result<Self> {
        // SAFETY: The window runs the picker synchronously on its UI thread.
        // Drop balances both S_OK and S_FALSE on that same thread.
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };
        Ok(Self)
    }
}

impl Drop for Apartment {
    fn drop(&mut self) {
        // SAFETY: enter successfully initialized COM on the current thread.
        unsafe { CoUninitialize() };
    }
}

pub(super) fn browse(
    window: &WindowState,
    kind: BrowseKind,
) -> windows::core::Result<Option<PathBuf>> {
    let _apartment = Apartment::enter()?;
    let owner = window
        .win32_hwnd()
        .map(|handle| HWND(handle.get() as *mut _));

    // SAFETY: COM is initialized and the optional parent is the current
    // winit window, which stays alive for the duration of this modal dialog.
    unsafe {
        let dialog: IFileOpenDialog =
            CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
        let mut options =
            dialog.GetOptions()? | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST | FOS_NOCHANGEDIR;
        match kind {
            BrowseKind::Folder => {
                dialog.SetTitle(w!("Select the Street Fighter ROM folder"))?;
                options |= FOS_PICKFOLDERS;
            }
            BrowseKind::Archive => {
                dialog.SetTitle(w!("Select the Street Fighter ROM archive"))?;
                dialog.SetFileTypes(&[COMDLG_FILTERSPEC {
                    pszName: w!("ROM archives (*.zip, *.7z)"),
                    pszSpec: w!("*.zip;*.7z"),
                }])?;
                options |= FOS_FILEMUSTEXIST;
            }
        }
        dialog.SetOptions(options)?;
        if let Err(error) = dialog.Show(owner) {
            return if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) {
                Ok(None)
            } else {
                Err(error)
            };
        }

        let item = dialog.GetResult()?;
        let path = item.GetDisplayName(SIGDN_FILESYSPATH)?;
        let selected = PathBuf::from(OsString::from_wide(path.as_wide()));
        CoTaskMemFree(Some(path.0.cast()));
        Ok(Some(selected))
    }
}
