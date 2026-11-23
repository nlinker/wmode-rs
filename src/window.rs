//! Game window subclassing: installs our window procedure and forwards
//! everything to the original. Behavior changes land with the hotkey and
//! render handling.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    VK_F1, VK_F10, VK_F11, VK_F12, VK_F9, VK_OEM_MINUS, VK_OEM_PLUS,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallWindowProcA, DefWindowProcA, EnableMenuItem, GetSystemMenu, SetWindowLongA, SetWindowPos,
    GWL_WNDPROC, HTCAPTION, HTCLOSE, HTMAXBUTTON, HTMINBUTTON, HWND_NOTOPMOST, HWND_TOPMOST,
    MF_BYCOMMAND, MF_ENABLED, MF_GRAYED, SC_CLOSE, SC_MAXIMIZE, SC_MINIMIZE, SWP_NOMOVE,
    ShowCursor, SWP_NOSIZE, WM_ACTIVATE, WM_ACTIVATEAPP, WA_INACTIVE, WM_DISPLAYCHANGE,
    WM_MOUSEFIRST,
    WM_MOUSELAST, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_MOVE, WM_NCLBUTTONDOWN, WM_NCMOUSEMOVE,
    WM_PAINT, WM_SIZE, WM_SYSCOMMAND, WM_SYSKEYDOWN,
};

use crate::cursor;
use crate::log::log;
use crate::render;
use crate::scale;

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
/// Whether the arrow cursor is currently hidden by us. The game hides
/// the system cursor while it renders its own, over the caption nothing
/// renders it, so we show the arrow there.
static CURSOR_HIDDEN: AtomicBool = AtomicBool::new(true);
/// Last known client position, cached on every move: the exit save runs
/// in DLL_PROCESS_DETACH where window queries are off limits.
static WINDOW_CLIENT_X: AtomicI32 = AtomicI32::new(-1);
static WINDOW_CLIENT_Y: AtomicI32 = AtomicI32::new(-1);

/// Shows or hides the arrow cursor, one ShowCursor call per state change.
fn set_cursor_show(show: bool) {
    if show == CURSOR_HIDDEN.load(Ordering::Relaxed) {
        unsafe {
            ShowCursor(show as i32);
        }
        CURSOR_HIDDEN.store(!show, Ordering::Relaxed);
    }
}

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

/// Applies a loaded ini config to the toggle states and the window.
pub fn apply_config(hwnd: HWND, cfg: &crate::ini::Config) {
    MOVE_ENABLED.store(cfg.enable_window_move, Ordering::Relaxed);
    TOPMOST.store(cfg.always_on_top, Ordering::Relaxed);
    CAPTION_CONTROLS.store(!cfg.disable_controls, Ordering::Relaxed);
    unsafe {
        apply_topmost(hwnd);
        apply_caption_controls(hwnd);
    }
}

/// Current toggle states, for the exit save.
pub fn move_enabled() -> bool {
    MOVE_ENABLED.load(Ordering::Relaxed)
}

pub fn topmost() -> bool {
    TOPMOST.load(Ordering::Relaxed)
}

pub fn caption_controls() -> bool {
    CAPTION_CONTROLS.load(Ordering::Relaxed)
}

/// Remembers the client position for the exit save. Minimized windows
/// report the placeholder position -32000 and must not poison the cache.
unsafe fn remember_client_pos(hwnd: HWND) {
    let (x, y) = render::client_origin(hwnd);
    if x <= -10000 || y <= -10000 {
        return;
    }
    WINDOW_CLIENT_X.store(x, Ordering::Relaxed);
    WINDOW_CLIENT_Y.store(y, Ordering::Relaxed);
}

/// Exit save for DLL_PROCESS_DETACH: uses the cached client position,
/// no window queries, the loader lock is held at that point.
pub fn save_cached() {
    let ok = crate::ini::save_state(&crate::ini::SaveState {
        client_x: WINDOW_CLIENT_X.load(Ordering::Relaxed),
        client_y: WINDOW_CLIENT_Y.load(Ordering::Relaxed),
        clip_cursor: cursor::clip_enabled(),
        enable_window_move: move_enabled(),
        always_on_top: topmost(),
        disable_controls: !caption_controls(),
        scale: scale::current(),
    });
    log(&format!(
        "window: exit save {} ({},{})",
        if ok { "written" } else { "failed" },
        WINDOW_CLIENT_X.load(Ordering::Relaxed),
        WINDOW_CLIENT_Y.load(Ordering::Relaxed)
    ));
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
    mut lparam: LPARAM,
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
    if msg == WM_MOVE {
        remember_client_pos(hwnd);
    }
    if msg == WM_ACTIVATE && wparam as u32 & 0xFFFF != WA_INACTIVE {
        // the system drops ClipCursor confinement when the window
        // deactivates: bring it back on every activation while the
        // switch is on, or Alt+Tab leaves the cursor free forever
        cursor::refresh_clip();
    }
    if msg == WM_PAINT && render::windowed() {
        render::paint(hwnd);
    }
    if msg == WM_SIZE && render::windowed() {
        if render::correct_client_size(hwnd) {
            // the correction moved or resized the client box the cursor
            // clip confines to, bring the applied clip back in line
            cursor::refresh_clip();
        }
    }
    if msg == WM_SYSKEYDOWN {
        // the VK_* constants are u16, so the key is matched in that width
        match wparam as u16 {
            VK_F9 => scale::reset(hwnd),
            VK_OEM_MINUS => scale::smaller(hwnd),
            VK_OEM_PLUS => scale::bigger(hwnd),
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
    if msg == WM_ACTIVATEAPP {
        // the game halts its loop when told it deactivated: it always
        // hears "active" here, so it keeps running unfocused like the
        // original wmode does (its readme: switching windows does not
        // pause the game)
        let old = ORIGINAL_PROC.load(Ordering::Acquire);
        if old != 0 {
            type Proc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;
            let prev: Proc = core::mem::transmute::<usize, Proc>(old as usize);
            return CallWindowProcA(Some(prev), hwnd, msg, 1, 0);
        }
        return DefWindowProcA(hwnd, msg, 1, 0);
    }
    if msg == WM_NCMOUSEMOVE {
        set_cursor_show(true);
    }
    if msg == WM_MOUSEMOVE {
        set_cursor_show(false);
    }
    // mouse messages carry client pixels: rescale them to game units
    // while the client is not the game size (the scale cycle). Wheel
    // messages carry screen coordinates and stay untouched.
    if render::windowed()
        && msg >= WM_MOUSEFIRST
        && msg <= WM_MOUSELAST
        && msg != WM_MOUSEWHEEL
    {
        lparam = cursor::fix_points(lparam);
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
