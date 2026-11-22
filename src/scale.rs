//! Window scale cycling: ALT+F9 walks 1x -> 1.5x -> 2x -> 1x. The game
//! keeps rendering into its own surface, the client area is resized and
//! the paint path stretches.

use std::sync::atomic::{AtomicUsize, Ordering};

use windows_sys::Win32::Foundation::HWND;

use crate::cursor;
use crate::log::log;
use crate::render;

/// The scale cycle, in ALT+F9 order.
pub const SCALES: [f32; 3] = [1.0, 1.5, 2.0];
static IDX: AtomicUsize = AtomicUsize::new(0);

/// The active scale factor.
pub fn current() -> f32 {
    SCALES[IDX.load(Ordering::Relaxed)]
}

/// Client size the window should carry: the game size scaled by the
/// current factor, rounded to whole pixels.
pub fn client_wanted() -> (i32, i32) {
    let (gw, gh) = render::game_size();
    let s = current();
    (
        (gw as f32 * s).round() as i32,
        (gh as f32 * s).round() as i32,
    )
}

/// ALT+F9: advances the cycle, resizes the window and re-applies the
/// cursor clip for the new translation base.
pub fn cycle(hwnd: HWND) {
    let next = (IDX.load(Ordering::Relaxed) + 1) % SCALES.len();
    IDX.store(next, Ordering::Relaxed);
    let (cw, ch) = client_wanted();
    unsafe {
        render::set_client_size(hwnd, cw, ch);
    }
    cursor::apply_switch();
    log(&format!("hotkey: ALT+F9 scale {} client {cw}x{ch}", SCALES[next]));
}
