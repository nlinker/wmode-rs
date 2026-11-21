//! wmode.ini location, creation and opening.
//! Upstream rule: the ini always lives in the StarCraft directory, never
//! next to the plugin.

use windows_sys::core::{BOOL, PCSTR};
use windows_sys::Win32::System::LibraryLoader::{
    GetModuleFileNameA, GetModuleHandleExA, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
    GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
};
use windows_sys::Win32::System::Registry::{RegGetValueA, HKEY_CURRENT_USER, RRF_RT_REG_SZ};
use windows_sys::Win32::UI::Shell::ShellExecuteA;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// Template written when the ini does not exist yet. Mirrors the upstream
/// sample from WModeReadme.txt, plus the SaveScale key added by wmode-rs.
const TEMPLATE: &str = "[W-MODE]\r\n\
; What settings do you want to save when you exit StarCraft?\r\n\
; Save WindowClientX?\r\n\
; Default value: 1\r\n\
SaveWindowClientX=1\r\n\
; Save WindowClientY?\r\n\
; Default value: 1\r\n\
SaveWindowClientY=1\r\n\
; Save ClipCursor?\r\n\
; Default value: 0\r\n\
SaveClipCursor=0\r\n\
; Save EnableWindowMove?\r\n\
; Default value: 1\r\n\
SaveEnableWindowMove=1\r\n\
; Save AlwaysOnTop?\r\n\
; Default value: 1\r\n\
SaveAlwaysOnTop=1\r\n\
; Save DisableControls?\r\n\
; Default value: 1\r\n\
SaveDisableControls=1\r\n\
; Save the ALT+F9 window scale? (wmode-rs extension)\r\n\
; Default value: 1\r\n\
SaveScale=1\r\n\
; X and Y coordinates of the StarCraft game screen.\r\n\
; (Upper left corner of client area.)\r\n\
; Default values: center the game screen on the desktop.\r\n\
WindowClientX=30\r\n\
WindowClientY=30\r\n\
; Cursor clip (Toggle hotkey: ALT+F1)\r\n\
; Default value: 0\r\n\
ClipCursor=0\r\n\
; Enable window move (Toggle hotkey: ALT+F10)\r\n\
; Default value: 1\r\n\
EnableWindowMove=1\r\n\
; Enable always-on-top mode (Toggle hotkey: ALT+F11)\r\n\
; Default value: 0\r\n\
AlwaysOnTop=0\r\n\
; Disable all controls in the caption of the StarCraft\r\n\
; window. Disable the screensaver when the sc window is\r\n\
; active. Window can not be closed with ALT+F4.\r\n\
; (Toggle hotkey: ALT+F12)\r\n\
; Default value: 0\r\n\
DisableControls=0\r\n\
; Limit the maximum frame/sec to reach better performance.\r\n\
; Default and recommended value: 100\r\n\
; Minimum value: 30\r\n\
MaxFps=100\r\n\
; Mute all sound when the main window loses focus.\r\n\
; Default value: 0\r\n\
MuteNotFocused=0\r\n";

/// NUL-terminated byte string as a PCSTR.
fn pcstr(bytes: &[u8]) -> PCSTR {
    bytes.as_ptr()
}

/// Reads a NUL-terminated byte buffer as a String.
fn string_from_buf(buf: &[u8]) -> String {
    let end = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

/// Strips the last path component, returns None at a drive root.
fn parent_of(path: &str) -> Option<String> {
    let cut = path.rfind(['\\', '/'])?;
    if cut == 0 || path[..cut].ends_with(':') {
        return None;
    }
    Some(path[..cut].to_owned())
}

/// StarCraft directory from the registry Game Path the installer writes.
/// The value can point at StarCraft.exe or at the directory itself.
fn starcraft_dir_from_registry() -> Option<String> {
    let mut buf = [0u8; 260];
    let mut size = buf.len() as u32;
    let ret = unsafe {
        RegGetValueA(
            HKEY_CURRENT_USER,
            pcstr(b"Software\\Battle.net\\Configuration\0"),
            pcstr(b"Game Path\0"),
            RRF_RT_REG_SZ,
            core::ptr::null_mut(),
            buf.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if ret != 0 {
        return None;
    }
    let value = string_from_buf(&buf);
    if value.is_empty() {
        return None;
    }
    if value.to_ascii_lowercase().ends_with(".exe") {
        return parent_of(&value);
    }
    Some(value)
}

/// StarCraft directory derived from this DLL's own location when installed
/// as <sc>\Chaoslauncher\Plugins\<name>.bwl. The module handle is resolved
/// from a function address, no stored state involved.
fn starcraft_dir_from_module_path() -> Option<String> {
    let mut hmodule = core::ptr::null_mut();
    let found: BOOL = unsafe {
        GetModuleHandleExA(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
                | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            (starcraft_dir_from_module_path as fn() -> Option<String>) as usize as PCSTR,
            &mut hmodule,
        )
    };
    if found == 0 {
        return None;
    }
    let mut buf = [0u8; 260];
    let len = unsafe { GetModuleFileNameA(hmodule, buf.as_mut_ptr(), buf.len() as u32) };
    if len == 0 || len as usize >= buf.len() {
        return None;
    }
    let mut dir = parent_of(&string_from_buf(&buf[..len as usize]))?;
    for _ in 0..2 {
        dir = parent_of(&dir)?;
    }
    Some(dir)
}

/// Finds the StarCraft directory. Registry first, module path as fallback.
pub fn starcraft_dir() -> Option<String> {
    starcraft_dir_from_registry()
        .or_else(starcraft_dir_from_module_path)
}

/// Creates wmode.ini from the template when missing and opens it in the
/// default text editor. Returns 1 on success, 0 on failure.
pub fn open_config() -> i32 {
    let Some(dir) = starcraft_dir() else {
        return 0;
    };
    let path = format!("{dir}\\wmode.ini");
    if !std::path::Path::new(&path).exists()
        && std::fs::write(&path, TEMPLATE).is_err()
    {
        return 0;
    }
    let mut file = path.into_bytes();
    file.push(0);
    let opened = unsafe {
        ShellExecuteA(
            core::ptr::null_mut(),
            pcstr(b"open\0"),
            pcstr(&file),
            core::ptr::null(),
            core::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    i32::from(opened as isize > 32)
}
