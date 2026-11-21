//! wmode-rs, a Rust rewrite of W-MODE v1.02 (XeNotRoN, 2008).
//!
//! Windowed-mode plugin for StarCraft: Brood War 1.16.1, loaded as a
//! ChaosLauncher (BWL4) plugin. Alt+F9 cycles window scale 1x -> 1.5x -> 2x
//! (the feature that started this rewrite). Everything else mirrors the
//! original per `wmode\WModeReadme.txt`.

#![allow(non_snake_case)]

mod bwl;
mod ini;
mod inject;

use windows_sys::core::BOOL;
use windows_sys::Win32::Foundation::{HINSTANCE, TRUE};

/// DLL entry point. Deliberately does nothing because of the loader-lock
/// rule (no library, window or COM calls from DllMain). All initialization
/// (game window discovery, hook installation) runs on a separate thread.
#[no_mangle]
pub extern "system" fn DllMain(
    _hinst: HINSTANCE,
    _reason: u32,
    _reserved: *mut core::ffi::c_void,
) -> BOOL {
    TRUE
}
