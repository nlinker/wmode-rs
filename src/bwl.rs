//! BWL4 plugin contract for ChaosLauncher. Layout follows BWAPI's
//! BWAPI_PluginInjector chaoslauncher.h. Exports use the C (cdecl)
//! convention on x86, like every existing plugin.

use core::ffi::c_char;

use windows_sys::Win32::Foundation::HANDLE;

/// Launcher/plugin exchange structure, filled in by GetPluginAPI.
#[repr(C)]
pub struct ExchangeData {
    pub iPluginAPI: i32,
    pub iStarCraftBuild: i32,
    pub bNotSCBWmodule: i32,
    pub bConfigDialog: i32,
}

const PLUGIN_API_VERSION: i32 = 4;
/// StarCraft: Brood War 1.16.1.
const STARCRAFT_BUILD: i32 = 13;

/// Display name shown in the launcher plugin list, version included.
const NAME: &[u8] = concat!("WMode-rs ", env!("CARGO_PKG_VERSION")).as_bytes();
/// Description shown in the launcher plugin list: the full hotkey set.
const DESCRIPTION: &[u8] = b"Windowed mode plugin for StarCraft: Brood War 1.16.1.\r\n\
     Rust rewrite of W-MODE v1.02 by XeNotRoN (2008).\r\n\
     \r\n\
     Hotkeys:\r\n\
     ALT+F1   cursor clip on/off\r\n\
     ALT+- / ALT+=   window scale down/up [1.0..2.0]\r\n\
     ALT+F9   reset scale to 1x\r\n\
     ALT+F10  window move on/off\r\n\
     ALT+F11  always-on-top on/off\r\n\
     ALT+F12  caption buttons on/off (also blocks ALT+F4 and disables\r\n\
     the screensaver while the game is focused)\r\n\
     \r\n\
     Config: wmode.ini in the StarCraft folder (MaxFps, Scale, Save*).";

/// Copies `src` into a launcher-provided NUL-terminated buffer, truncating
/// to `cap - 1` bytes. Never writes past `cap`.
unsafe fn write_c_str(dst: *mut c_char, cap: usize, src: &[u8]) {
    if dst.is_null() || cap == 0 {
        return;
    }
    let n = src.len().min(cap - 1);
    core::ptr::copy_nonoverlapping(src.as_ptr(), dst as *mut u8, n);
    *dst.add(n) = 0;
}

/// Reports the plugin API version and the StarCraft build this plugin
/// targets. API 4 with build 13 selects Brood War 1.16.1.
#[no_mangle]
pub unsafe extern "C" fn GetPluginAPI(data: *mut ExchangeData) {
    if data.is_null() {
        return;
    }
    (*data).iPluginAPI = PLUGIN_API_VERSION;
    (*data).iStarCraftBuild = STARCRAFT_BUILD;
    (*data).bNotSCBWmodule = 0;
    (*data).bConfigDialog = 1;
}

/// Fills launcher-provided buffers with the plugin name, description and
/// update URL. Buffer capacities 64 and 512 bytes per BWL4 usage, the
/// update URL is left empty.
#[no_mangle]
pub unsafe extern "C" fn GetData(
    name: *mut c_char,
    description: *mut c_char,
    updateurl: *mut c_char,
) {
    write_c_str(name, 64, NAME);
    write_c_str(description, 512, DESCRIPTION);
    write_c_str(updateurl, 256, b"");
}

/// Launcher Config button: creates wmode.ini from the template when the
/// file does not exist yet, then opens it in the default text editor.
#[no_mangle]
pub extern "C" fn OpenConfig() -> i32 {
    crate::ini::open_config()
}

/// Called when StarCraft is already running and its window exists: the
/// game has consumed the storm Window Mode flag by now, the previous
/// registry value can go back.
#[no_mangle]
pub extern "C" fn ApplyPatch(_hprocess: HANDLE, _reserved: u32) -> i32 {
    let saved = SAVED_WINDOW_MODE.lock().unwrap().take();
    if saved.is_some() {
        crate::set_storm_window_mode(saved);
        crate::log::log("bwl: registry Window Mode restored");
    }
    1
}

/// Window Mode value seen before the game started, restored by ApplyPatch.
static SAVED_WINDOW_MODE: std::sync::Mutex<Option<u32>> = std::sync::Mutex::new(None);

/// Called while StarCraft.exe is suspended at startup: sets the storm
/// Window Mode flag (the game reads it while creating its window, a
/// fullscreen-exclusive DirectDraw setup never happens) and injects this
/// DLL into the game process.
#[no_mangle]
pub extern "C" fn ApplyPatchSuspended(hprocess: HANDLE, _reserved: u32) -> i32 {
    let previous = crate::storm_window_mode();
    crate::set_storm_window_mode(Some(1));
    *SAVED_WINDOW_MODE.lock().unwrap() = previous;
    crate::log::log("bwl: registry Window Mode set to 1");
    let rc = crate::inject::apply_patch_suspended(hprocess);
    // wait for the in-game side to report readiness through the shared log
    // (the flag lives in another process, the log file is common ground)
    let marker = "res: hooks installed";
    let log_path = std::env::temp_dir().join("wmode-rs.log");
    for _ in 0..600 {
        if let Ok(text) = std::fs::read_to_string(&log_path) {
            if text.contains(marker) {
                crate::log::log("bwl: in-game hooks confirmed ready");
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    rc
}
