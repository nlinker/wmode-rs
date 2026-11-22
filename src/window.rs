//! Game window subclassing: installs our window procedure and forwards
//! everything to the original. Behavior changes land with the hotkey and
//! render handling.

use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallWindowProcA, DefWindowProcA, SetWindowLongA, GWL_WNDPROC, WM_DISPLAYCHANGE, WM_PAINT,
};

use crate::log::log;
use crate::render;

/// Original window procedure. SetWindowLongA flavor (i32, x86-safe).
static ORIGINAL_PROC: AtomicI32 = AtomicI32::new(0);
/// How many messages get logged before the diagnostic goes quiet.
static LOG_BUDGET: AtomicU32 = AtomicU32::new(30);

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
    let old = ORIGINAL_PROC.load(Ordering::Acquire);
    if old != 0 {
        type Proc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;
        let prev: Proc = core::mem::transmute::<usize, Proc>(old as usize);
        CallWindowProcA(Some(prev), hwnd, msg, wparam, lparam)
    } else {
        DefWindowProcA(hwnd, msg, wparam, lparam)
    }
}
