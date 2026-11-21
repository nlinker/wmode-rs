//! BWL4 plugin contract for ChaosLauncher and compatible launchers.
//! Layout and semantics follow BWLauncher 4 (struct from BWAPI's
//! BWAPI_PluginInjector chaoslauncher.h). The launcher loads this DLL in
//! its own process and asks these exports what the plugin is.
//! All plugin exports use the C (cdecl) convention on x86, matching the
//! `extern "C" __declspec(dllexport)` declarations of existing plugins.

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

/// Display name shown in the launcher plugin list.
const NAME: &[u8] = b"wmode-rs";
/// Description shown in the launcher plugin list.
const DESCRIPTION: &[u8] = b"Windowed mode plugin for StarCraft: Brood War 1.16.1.\r\n\
     ALT+F9 cycles window scale 1x -> 1.5x -> 2x.\r\n\
     Rust rewrite of W-MODE v1.02 by XeNotRoN (2008)";

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

/// Called when StarCraft is already running. Not used: injection happens
/// while the process is still suspended.
#[no_mangle]
pub extern "C" fn ApplyPatch(_hprocess: HANDLE, _reserved: u32) -> i32 {
    1
}

/// Called while StarCraft.exe is suspended at startup: injects this DLL
/// into the game process so it can install its hooks at startup.
#[no_mangle]
pub extern "C" fn ApplyPatchSuspended(hprocess: HANDLE, _reserved: u32) -> i32 {
    crate::inject::apply_patch_suspended(hprocess)
}
