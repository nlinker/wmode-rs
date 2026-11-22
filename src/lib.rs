//! wmode-rs, a Rust rewrite of W-MODE v1.02 (XeNotRoN, 2008).
//!
//! Windowed-mode plugin for StarCraft: Brood War 1.16.1, loaded as a
//! ChaosLauncher (BWL4) plugin. Alt+F9 cycles window scale 1x -> 1.5x -> 2x.
//! Everything else mirrors the original per `original\WModeReadme.txt`.

#![allow(non_snake_case)]

mod bwl;
mod iat;
mod ini;
mod inject;
mod log;
mod window;

use log::log;

use windows_sys::core::BOOL;
use windows_sys::Win32::Foundation::{CloseHandle, HINSTANCE, TRUE};
use windows_sys::Win32::System::LibraryLoader::{
    DisableThreadLibraryCalls, GetModuleFileNameA, GetModuleHandleA,
};
use windows_sys::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows_sys::Win32::System::Threading::{CreateThread, Sleep};

/// StarCraft 1.16.1 main window class.
const GAME_WINDOW_CLASS: &[u8] = b"SWarClass\0";
const WINDOW_POLL_MS: u32 = 50;
const WINDOW_POLL_MAX_TRIES: u32 = 1200; // 60 s

/// storm.dll exports by ordinal only (no name table), and StarCraft.exe
/// imports it by ordinal. Numbers per BWAPI CodePatch.cpp.
const GAME_STORM_ORDINALS: &[(u16, &str)] = &[
    (350, "SDrawLockSurface"),
    (356, "SDrawUnlockSurface"),
    (357, "SDrawUpdatePalette"),
];
const GAME_USER32_IMPORTS: &[&str] = &["GetCursorPos", "SetCursorPos", "ClipCursor"];

/// DLL entry point. Only inside the game process: disables thread
/// notifications and spawns the init thread. The launcher loads this DLL
/// into itself too, where it must stay dormant. Loader-lock rule: nothing
/// else runs here, the spawned thread starts once DllMain has returned.
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

/// Whether the host EXE is StarCraft.exe.
fn host_exe_is_starcraft() -> bool {
    let mut buf = [0u8; 260];
    let len =
        unsafe { GetModuleFileNameA(core::ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32) };
    if len == 0 || len as usize >= buf.len() {
        return false;
    }
    let exe = String::from_utf8_lossy(&buf[..len as usize]).to_ascii_lowercase();
    exe.ends_with("\\starcraft.exe")
}

/// Starts the init thread and drops its handle.
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

/// Logs which of the hook-target imports exist in the game executable.
fn log_game_imports() {
    let exe = unsafe { GetModuleHandleA(core::ptr::null()) };
    for (ordinal, name) in GAME_STORM_ORDINALS {
        match unsafe { iat::find_import_ordinal(exe, "storm.dll", *ordinal) } {
            Some(slot) => log(&format!("init: import storm.dll!{name} (#{ordinal}) at {slot:p}")),
            None => log(&format!("init: import storm.dll!{name} (#{ordinal}) NOT FOUND")),
        }
    }
    for func in GAME_USER32_IMPORTS {
        match unsafe { iat::find_import(exe, "user32.dll", func) } {
            Some(slot) => log(&format!("init: import user32.dll!{func} at {slot:p}")),
            None => log(&format!("init: import user32.dll!{func} NOT FOUND")),
        }
    }
}

/// Init thread: waits for the game window, then (once window management
/// lands) subclasses it and installs the hooks.
unsafe extern "system" fn init_thread(_param: *mut core::ffi::c_void) -> u32 {
    log("init: thread started, waiting for the game window");
    for _ in 0..WINDOW_POLL_MAX_TRIES {
        let hwnd = windows_sys::Win32::UI::WindowsAndMessaging::FindWindowA(
            GAME_WINDOW_CLASS.as_ptr(),
            core::ptr::null(),
        );
        if !hwnd.is_null() {
            log(&format!("init: game window found hwnd={hwnd:p}"));
            log_game_imports();
            if !window::install(hwnd) {
                log("init: window subclass FAILED");
            }
            return 0;
        }
        Sleep(WINDOW_POLL_MS);
    }
    log("init: game window not found after 60 s, giving up");
    0
}
