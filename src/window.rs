//! Game window subclassing: installs our window procedure and forwards
//! everything to the original. Behavior changes land with the hotkey and
//! render handling.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_F1, VK_F10, VK_F11, VK_F12, VK_MENU};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallWindowProcA, DefWindowProcA, EnableMenuItem, GetSystemMenu, SetWindowLongA, SetWindowPos,
    GWL_WNDPROC, HTCAPTION, HTCLOSE, HTMAXBUTTON, HTMINBUTTON, HWND_NOTOPMOST, HWND_TOPMOST,
    MF_BYCOMMAND, MF_ENABLED, MF_GRAYED, SC_CLOSE, SC_MAXIMIZE, SC_MINIMIZE, SWP_NOMOVE,
    SWP_NOSIZE, WM_DISPLAYCHANGE, WM_NCLBUTTONDOWN, WM_PAINT, WM_SIZE, WM_SYSCOMMAND,
    WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use crate::cursor;
use crate::log::log;
use crate::render;

/// Original window procedure. SetWindowLongA flavor (i32, x86-safe).
static ORIGINAL_PROC: AtomicI32 = AtomicI32::new(0);
/// How many messages get logged before the diagnostic goes quiet.
static LOG_BUDGET: AtomicU32 = AtomicU32::new(30);
/// Window dragging by the caption allowed, the ALT+F10 state.
static MOVE_ENABLED: AtomicBool = AtomicBool::new(true);
/// Always-on-top mode, the ALT+F11 state.
static TOPMOST: AtomicBool = AtomicBool::new(false);
/// Caption buttons (close, maximize, minimize) active, the ALT+F12 state.
static CAPTION_CONTROLS: AtomicBool = AtomicBool::new(true);

/// Applies the always-on-top state to the window.
unsafe fn apply_topmost(hwnd: HWND) {
    let after = if TOPMOST.load(Ordering::Relaxed) {
        HWND_TOPMOST
    } else {
        HWND_NOTOPMOST
    };
    SetWindowPos(hwnd, after, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
}

/// Greys or restores the close, maximize and minimize caption buttons.
unsafe fn apply_caption_controls(hwnd: HWND) {
    let menu = GetSystemMenu(hwnd, 0);
    if menu.is_null() {
        return;
    }
    let flags = if CAPTION_CONTROLS.load(Ordering::Relaxed) {
        MF_BYCOMMAND | MF_ENABLED
    } else {
        MF_BYCOMMAND | MF_GRAYED
    };
    EnableMenuItem(menu, SC_CLOSE, flags);
    EnableMenuItem(menu, SC_MAXIMIZE, flags);
    EnableMenuItem(menu, SC_MINIMIZE, flags);
}

/// Subclasses the game window, keeping the original procedure for
/// forwarding. Returns false when the subclass did not take.
pub fn install(hwnd: HWND) -> bool {
    type Proc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;
    let our: Proc = our_wnd_proc;
    let previous = unsafe { SetWindowLongA(hwnd, GWL_WNDPROC, our as usize as i32) };
    if previous == 0 {
        return false;
    }
    ORIGINAL_PROC.store(previous, Ordering::Release);
    log(&format!("window: subclass installed, old proc {previous:#010x}"));
    true
}

/// Pass-through procedure: logs the first messages, then stays silent and
/// forwards everything.
unsafe extern "system" fn our_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let budget = LOG_BUDGET.load(Ordering::Relaxed);
    if budget > 0 {
        LOG_BUDGET.store(budget - 1, Ordering::Relaxed);
        log(&format!("window: msg {msg:#06x}"));
    }
    if msg == WM_DISPLAYCHANGE {
        // lParam packs the new size: low word width, high word height
        let w = (lparam & 0xFFFF) as u16;
        let h = ((lparam >> 16) & 0xFFFF) as u16;
        log(&format!("window: WM_DISPLAYCHANGE {w}x{h} depth={wparam}"));
    }
    if msg == WM_PAINT && render::windowed() {
        render::paint(hwnd);
    }
    if msg == WM_SIZE && render::windowed() {
        if render::correct_client_size(hwnd) {
            // the resize just changed the translation base, the applied
            // clip rect is stale until the game asks again, so re-apply
            // it right here
            cursor::refresh_clip();
        }
    }
    if msg == WM_SYSKEYDOWN {
        // the VK_* constants are u16, so the key is matched in that width
        match wparam as u16 {
            VK_MENU => cursor::set_alt_held(true),
            VK_F1 => {
                let on = !cursor::clip_enabled();
                cursor::set_clip_enabled(on);
                cursor::refresh_clip();
                log(&format!("hotkey: ALT+F1 cursor clip {}", if on { "ON" } else { "OFF" }));
            }
            VK_F10 => {
                let on = !MOVE_ENABLED.load(Ordering::Relaxed);
                MOVE_ENABLED.store(on, Ordering::Relaxed);
                log(&format!("hotkey: ALT+F10 window move {}", if on { "ON" } else { "OFF" }));
            }
            VK_F11 => {
                let on = !TOPMOST.load(Ordering::Relaxed);
                TOPMOST.store(on, Ordering::Relaxed);
                apply_topmost(hwnd);
                log(&format!("hotkey: ALT+F11 always on top {}", if on { "ON" } else { "OFF" }));
            }
            VK_F12 => {
                let on = !CAPTION_CONTROLS.load(Ordering::Relaxed);
                CAPTION_CONTROLS.store(on, Ordering::Relaxed);
                apply_caption_controls(hwnd);
                log(&format!(
                    "hotkey: ALT+F12 caption buttons {}",
                    if on { "ON" } else { "OFF" }
                ));
            }
            _ => {}
        }
    }
    if msg == WM_SYSKEYUP && wparam == VK_MENU as usize {
        cursor::set_alt_held(false);
    }
    if msg == WM_NCLBUTTONDOWN {
        // the HT_* constants are u32, so the hit test is compared in that width
        let hit = wparam as u32;
        let move_blocked = !MOVE_ENABLED.load(Ordering::Relaxed) && hit == HTCAPTION;
        let buttons_blocked = !CAPTION_CONTROLS.load(Ordering::Relaxed)
            && (hit == HTCLOSE || hit == HTMAXBUTTON || hit == HTMINBUTTON);
        if move_blocked || buttons_blocked {
            return 0;
        }
    }
    if msg == WM_SYSCOMMAND
        && !CAPTION_CONTROLS.load(Ordering::Relaxed)
        && (wparam as u32 & 0xFFF0) == SC_CLOSE
    {
        // ALT+F4 and the system menu Close item stay inert while the
        // caption controls are off
        return 0;
    }
    let old = ORIGINAL_PROC.load(Ordering::Acquire);
    if old != 0 {
        type Proc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;
        let prev: Proc = core::mem::transmute::<usize, Proc>(old as usize);
        CallWindowProcA(Some(prev), hwnd, msg, wparam, lparam)
    } else {
        DefWindowProcA(hwnd, msg, wparam, lparam)
    }
}
