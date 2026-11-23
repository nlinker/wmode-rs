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
; Window scale, one of 1.0 1.25 1.5 1.75 2.0.\r\n\
; Change it with ALT+- and ALT+=, reset with ALT+F9.\r\n\
; (wmode-rs extension)\r\n\
; Default value: 1\r\n\
Scale=1\r\n\
; Limit the maximum frame/sec to reach better performance.\r\n\
; Default and recommended value: 100\r\n\
; Minimum value: 30\r\n\
MaxFps=100\r\n";

/// Full path of wmode.ini in the StarCraft directory.
pub fn ini_path() -> Option<String> {
    starcraft_dir().map(|dir| format!("{dir}\\wmode.ini"))
}

/// Values loaded from wmode.ini. Missing keys fall back to the same
/// defaults the template documents.
pub struct Config {
    pub save_window_client_x: bool,
    pub save_window_client_y: bool,
    pub save_clip_cursor: bool,
    pub save_enable_window_move: bool,
    pub save_always_on_top: bool,
    pub save_disable_controls: bool,
    pub save_scale: bool,
    /// Saved client position, negative means "not set, center me".
    pub window_client_x: i32,
    pub window_client_y: i32,
    pub clip_cursor: bool,
    pub enable_window_move: bool,
    pub always_on_top: bool,
    pub disable_controls: bool,
    pub max_fps: i32,
    pub scale: f32,
}

fn default_config() -> Config {
    Config {
        save_window_client_x: true,
        save_window_client_y: true,
        save_clip_cursor: false,
        save_enable_window_move: true,
        save_always_on_top: true,
        save_disable_controls: true,
        save_scale: true,
        window_client_x: -1,
        window_client_y: -1,
        clip_cursor: false,
        enable_window_move: true,
        always_on_top: false,
        disable_controls: false,
        max_fps: 100,
        scale: 1.0,
    }
}

/// Reads wmode.ini, or None when the file does not exist: startup then
/// runs on the built-in defaults (centered window, scale 1x, everything
/// off except window moving).
pub fn read_config() -> Option<Config> {
    let bytes = std::fs::read(ini_path()?).ok()?;
    let mut cfg = default_config();
    for line in bytes.split(|b| *b == b'\n') {
        let line = strip_cr(line);
        if line.is_empty() || line[0] == b';' || line[0] == b'[' {
            continue;
        }
        let Some(eq) = line.iter().position(|b| *b == b'=') else {
            continue;
        };
        let key: String = line[..eq].iter().map(|b| b.to_ascii_lowercase() as char).collect();
        let val = String::from_utf8_lossy(&line[eq + 1..]).into_owned();
        let num: i32 = val.parse().unwrap_or(0);
        let on = val.trim() == "1";
        match key.as_str() {
            "savewindowclientx" => cfg.save_window_client_x = on,
            "savewindowclienty" => cfg.save_window_client_y = on,
            "saveclipcursor" => cfg.save_clip_cursor = on,
            "saveenablewindowmove" => cfg.save_enable_window_move = on,
            "savealwaysontop" => cfg.save_always_on_top = on,
            "savedisablecontrols" => cfg.save_disable_controls = on,
            "savescale" => cfg.save_scale = on,
            "windowclientx" => cfg.window_client_x = num,
            "windowclienty" => cfg.window_client_y = num,
            "clipcursor" => cfg.clip_cursor = on,
            "enablewindowmove" => cfg.enable_window_move = on,
            "alwaysontop" => cfg.always_on_top = on,
            "disablecontrols" => cfg.disable_controls = on,
            "maxfps" => cfg.max_fps = num,
            "scale" => cfg.scale = val.trim().parse().unwrap_or(1.0),
            _ => {}
        }
    }
    Some(cfg)
}

fn strip_cr(line: &[u8]) -> &[u8] {
    match line.last() {
        Some(b'\r') => &line[..line.len() - 1],
        _ => line,
    }
}

/// State to persist, gathered by the caller at exit time.
pub struct SaveState {
    pub client_x: i32,
    pub client_y: i32,
    pub clip_cursor: bool,
    pub enable_window_move: bool,
    pub always_on_top: bool,
    pub disable_controls: bool,
    pub scale: f32,
}

/// Writes the state back into wmode.ini, honoring the Save* switches.
/// Comment lines and unknown keys survive untouched, line endings stay
/// CRLF. A missing ini is created from the template first. Returns true
/// when the file was written.
pub fn save_state(state: &SaveState) -> bool {
    let Some(path) = ini_path() else {
        return false;
    };
    if !std::path::Path::new(&path).exists() && std::fs::write(&path, TEMPLATE).is_err() {
        return false;
    }
    let Some(cfg) = read_config() else {
        return false;
    };
    let mut wanted: Vec<(&[u8], String)> = Vec::new();
    if cfg.save_window_client_x {
        wanted.push((b"WindowClientX", state.client_x.to_string()));
    }
    if cfg.save_window_client_y {
        wanted.push((b"WindowClientY", state.client_y.to_string()));
    }
    if cfg.save_clip_cursor {
        wanted.push((b"ClipCursor", bool_num(state.clip_cursor)));
    }
    if cfg.save_enable_window_move {
        wanted.push((b"EnableWindowMove", bool_num(state.enable_window_move)));
    }
    if cfg.save_always_on_top {
        wanted.push((b"AlwaysOnTop", bool_num(state.always_on_top)));
    }
    if cfg.save_disable_controls {
        wanted.push((b"DisableControls", bool_num(state.disable_controls)));
    }
    if cfg.save_scale {
        wanted.push((b"Scale", format_scale(state.scale)));
    }
    if wanted.is_empty() {
        return true;
    }

    let bytes = std::fs::read(&path).unwrap_or_default();
    let mut lines: Vec<Vec<u8>> = Vec::new();
    for line in bytes.split(|b| *b == b'\n') {
        lines.push(strip_cr(line).to_vec());
    }
    let mut written: Vec<Vec<u8>> = Vec::new();
    for line in lines.iter_mut() {
        let Some(eq) = line.iter().position(|b| *b == b'=') else {
            continue;
        };
        let key: Vec<u8> = line[..eq].iter().map(|b| b.to_ascii_lowercase()).collect();
        if let Some((want, val)) = wanted.iter().find(|(k, _)| k.eq_ignore_ascii_case(&key)) {
            *line = format!("{}={val}", String::from_utf8_lossy(want)).into_bytes();
            written.push(want.to_vec());
        }
    }
    for (want, val) in &wanted {
        if !written.iter().any(|k| k.eq_ignore_ascii_case(want)) {
            lines.push(format!("{}={val}", String::from_utf8_lossy(want)).into_bytes());
        }
    }
    let mut out = lines.join(&b"\r\n"[..]);
    out.extend_from_slice(b"\r\n");
    std::fs::write(&path, &out).is_ok()
}

fn bool_num(on: bool) -> String {
    if on { "1".into() } else { "0".into() }
}

/// Scale without trailing zeros: 1, 1.25, 1.5.
fn format_scale(s: f32) -> String {
    let mut text = format!("{s:.2}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}
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
