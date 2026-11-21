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
mod log;

use log::log;

use windows_sys::core::BOOL;
use windows_sys::Win32::Foundation::{CloseHandle, HINSTANCE, TRUE};
use windows_sys::Win32::System::LibraryLoader::{DisableThreadLibraryCalls, GetModuleFileNameA};
use windows_sys::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows_sys::Win32::System::Threading::{CreateThread, Sleep};

/// StarCraft 1.16.1 main window class.
const GAME_WINDOW_CLASS: &[u8] = b"SWarClass\0";
/// Window poll interval in milliseconds.
const WINDOW_POLL_MS: u32 = 50;
/// Window poll budget: 1200 x 50 ms = 60 s before giving up.
const WINDOW_POLL_MAX_TRIES: u32 = 1200;

/// DLL entry point. On attach, only inside the game process, it disables
/// thread notifications and spawns the init thread. The launcher also loads
/// this DLL into itself, where everything must stay dormant. Nothing else
/// runs here: the loader-lock rule forbids library, window and COM calls
/// from DllMain, and the spawned thread only starts executing once DllMain
/// has returned.
#[no_mangle]
pub extern "system" fn DllMain(
    hinst: HINSTANCE,
    reason: u32,
    _reserved: *mut core::ffi::c_void,
) -> BOOL {
    if reason == DLL_PROCESS_ATTACH && host_exe_is_starcraft() {
        unsafe {
            DisableThreadLibraryCalls(hinst);
        }
        spawn_init_thread();
    }
    TRUE
}

/// Whether the hosting process executable is StarCraft.exe. Keeps the init
/// thread out of ChaosLauncher, which loads this DLL into itself too.
fn host_exe_is_starcraft() -> bool {
    let mut buf = [0u8; 260];
    let len = unsafe { GetModuleFileNameA(core::ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32) };
    if len == 0 || len as usize >= buf.len() {
        return false;
    }
    let exe = String::from_utf8_lossy(&buf[..len as usize]).to_ascii_lowercase();
    exe.ends_with("\\starcraft.exe")
}

/// Starts the init thread and drops its handle immediately.
fn spawn_init_thread() {
    let thread = unsafe {
        CreateThread(
            core::ptr::null(),
            0,
            Some(init_thread),
            core::ptr::null(),
            0,
            core::ptr::null_mut(),
        )
    };
    if !thread.is_null() {
        unsafe {
            CloseHandle(thread);
        }
    }
}

/// Init thread body: waits for the game window to exist, then (once window
/// management lands) subclasses it and installs the hooks. Returns 0, the
/// thread exit code, in every case.
unsafe extern "system" fn init_thread(_param: *mut core::ffi::c_void) -> u32 {
    log("init: thread started, waiting for the game window");
    for _ in 0..WINDOW_POLL_MAX_TRIES {
        let hwnd = windows_sys::Win32::UI::WindowsAndMessaging::FindWindowA(
            GAME_WINDOW_CLASS.as_ptr(),
            core::ptr::null(),
        );
        if !hwnd.is_null() {
            log(&format!("init: game window found hwnd={hwnd:p}"));
            return 0;
        }
        Sleep(WINDOW_POLL_MS);
    }
    log("init: game window not found after 60 s, giving up");
    0
}
