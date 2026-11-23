//! wmode-rs, a Rust rewrite of W-MODE v1.02 (XeNotRoN, 2008).
//!
//! Windowed-mode plugin for StarCraft: Brood War 1.16.1, loaded as a
//! ChaosLauncher (BWL4) plugin. ALT+- and ALT+= walk the window scale
//! ladder [1.0 .. 2.0], ALT+F9 resets to 1x. Everything else mirrors
//! the original per `original\WModeReadme.txt`.

#![allow(non_snake_case)]

mod bwl;
mod cursor;
mod ddraw;
mod hooks;
mod iat;
mod ini;
mod inject;
mod log;
mod render;
mod scale;
mod window;

use log::log;


use windows_sys::core::BOOL;
use windows_sys::Win32::Foundation::{CloseHandle, HINSTANCE, TRUE};
use windows_sys::Win32::System::LibraryLoader::{
    DisableThreadLibraryCalls, GetModuleFileNameA, GetModuleHandleA,
};
use windows_sys::Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};
use windows_sys::Win32::UI::WindowsAndMessaging::{SM_CXSCREEN, SM_CYSCREEN};
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
/// One deliberate exception: on process detach the window state is saved
/// into wmode.ini - the game exits via ExitProcess and delivers neither
/// WM_CLOSE nor WM_DESTROY, so DLL_PROCESS_DETACH is the only exit
/// notification an injected DLL ever sees. The save touches the file
/// system and cached statics only, no window queries.
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
    if reason == DLL_PROCESS_DETACH && host_exe_is_starcraft() {
        window::save_cached();
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

/// storm.dll reads this registry value on startup: nonzero makes it create
/// a plain cooperative (DDSCL_NORMAL) window of the requested size instead
/// of a fullscreen exclusive one, so the desktop mode never changes.
const STORM_INTERNAL_KEY: &[u8] = b"Software\\Blizzard Entertainment\\Internal\0";
const STORM_WINDOW_MODE: &[u8] = b"Window Mode\0";

fn storm_window_mode() -> Option<u32> {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExA, RegQueryValueExA, HKEY_CURRENT_USER, KEY_QUERY_VALUE,
        REG_DWORD,
    };
    unsafe {
        let mut key = core::ptr::null_mut();
        if RegOpenKeyExA(
            HKEY_CURRENT_USER,
            STORM_INTERNAL_KEY.as_ptr(),
            0,
            KEY_QUERY_VALUE,
            &mut key,
        ) != 0
        {
            return None;
        }
        let mut value = 0u32;
        let mut size = 4u32;
        let mut kind = 0u32;
        let ok = RegQueryValueExA(
            key,
            STORM_WINDOW_MODE.as_ptr(),
            core::ptr::null_mut(),
            &mut kind,
            &mut value as *mut u32 as _,
            &mut size,
        );
        RegCloseKey(key);
        if ok == 0 && kind == REG_DWORD {
            Some(value)
        } else {
            None
        }
    }
}

fn set_storm_window_mode(value: Option<u32>) {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExA, RegDeleteValueA, RegSetValueExA, HKEY_CURRENT_USER,
        KEY_SET_VALUE, REG_DWORD, REG_OPTION_NON_VOLATILE,
    };
    unsafe {
        let mut key = core::ptr::null_mut();
        if RegCreateKeyExA(
            HKEY_CURRENT_USER,
            STORM_INTERNAL_KEY.as_ptr(),
            0,
            core::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            core::ptr::null(),
            &mut key,
            core::ptr::null_mut(),
        ) != 0
        {
            return;
        }
        match value {
            Some(v) => {
                RegSetValueExA(
                    key,
                    STORM_WINDOW_MODE.as_ptr(),
                    0,
                    REG_DWORD,
                    &v as *const u32 as _,
                    4,
                );
            }
            None => {
                RegDeleteValueA(key, STORM_WINDOW_MODE.as_ptr());
            }
        }
        RegCloseKey(key);
    }
}

/// Current screen size for the diagnostic log.
fn screen_size() -> (i32, i32) {
    unsafe {
        (
            windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(SM_CXSCREEN),
            windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(SM_CYSCREEN),
        )
    }
}

fn log_res(context: &str) {
    let (cx, cy) = screen_size();
    log(&format!("res: {context} {cx}x{cy}"));
}

/// Watches the screen size for the first minute and logs every change, to
/// pinpoint when and from what the display mode flips.
unsafe extern "system" fn res_watcher(_param: *mut core::ffi::c_void) -> u32 {
    let (mut cx, mut cy) = screen_size();
    for _ in 0..600 {
        Sleep(100);
        ddraw::install_mode_guard();
        let (nx, ny) = screen_size();
        if nx != cx || ny != cy {
            log(&format!("res: CHANGED {cx}x{cy} -> {nx}x{ny}"));
            cx = nx;
            cy = ny;
        }
    }
    0
}

/// Init thread. Runs while the game main thread is still suspended (the
/// launcher injects this DLL before resume), so the import patches land
/// before the game can initialize video. Only the window-dependent part
/// waits for the game window.
unsafe extern "system" fn init_thread(_param: *mut core::ffi::c_void) -> u32 {
    log("init: thread started, installing hooks");
    log_res("init start");
    log_game_imports();
    render::prepare();
    if !hooks::install() {
        log("init: render hooks FAILED, game left untouched");
        return 0;
    }
    if !cursor::install() {
        log("init: cursor hooks incomplete");
    }
    log_res("hooks installed");
    let watcher = unsafe {
        CreateThread(
            core::ptr::null(),
            0,
            Some(res_watcher),
            core::ptr::null(),
            0,
            core::ptr::null_mut(),
        )
    };
    if !watcher.is_null() {
        unsafe {
            CloseHandle(watcher);
        }
    }
    for _ in 0..WINDOW_POLL_MAX_TRIES {
        let hwnd = windows_sys::Win32::UI::WindowsAndMessaging::FindWindowA(
            GAME_WINDOW_CLASS.as_ptr(),
            core::ptr::null(),
        );
        if !hwnd.is_null() {
            log(&format!("init: game window found hwnd={hwnd:p}"));
            log_res("window found");
            if !window::install(hwnd) {
                log("init: window subclass FAILED");
            }
            render::activate_window(hwnd);
            if let Some(cfg) = ini::read_config() {
                log("init: wmode.ini loaded");
                window::apply_config(hwnd, &cfg);
                cursor::set_clip_enabled(cfg.clip_cursor);
                scale::set_from_value(cfg.scale);
                render::set_max_fps(cfg.max_fps);
                unsafe {
                    render::set_client_origin(hwnd, cfg.window_client_x, cfg.window_client_y);
                }
            }
            scale::apply_current(hwnd);
            cursor::apply_switch();
            return 0;
        }
        Sleep(WINDOW_POLL_MS);
    }
    log("init: game window not found after 60 s, giving up");
    0
}
