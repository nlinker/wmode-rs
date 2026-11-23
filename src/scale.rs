//! Window scaling: ALT+- and ALT+= walk the scale ladder down and up,
//! ALT+F9 resets to 1x. The game keeps rendering into its own surface,
//! the client area is resized and the paint path stretches.

use std::sync::atomic::{AtomicUsize, Ordering};

use windows_sys::Win32::Foundation::HWND;

use crate::cursor;
use crate::log::log;
use crate::render;

/// The scale ladder, small to large.
pub const SCALES: [f32; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];
/// Index of 1.0, the startup and reset scale.
const IDX_ONE: usize = 0;
static IDX: AtomicUsize = AtomicUsize::new(IDX_ONE);

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

/// Restores the scale from an ini value: the nearest ladder entry wins.
pub fn set_from_value(value: f32) {
    let mut best = 0;
    let mut best_err = f32::MAX;
    for (i, s) in SCALES.iter().enumerate() {
        let err = (s - value).abs();
        if err < best_err {
            best_err = err;
            best = i;
        }
    }
    IDX.store(best, Ordering::Relaxed);
}

/// Resizes the window to the current ladder scale. A no-op at 1x when
/// the client already matches.
pub fn apply_current(hwnd: HWND) {
    let (cw, ch) = client_wanted();
    unsafe {
        render::set_client_size(hwnd, cw, ch);
    }
}

fn apply(hwnd: HWND) {
    let (cw, ch) = client_wanted();
    unsafe {
        render::set_client_size(hwnd, cw, ch);
    }
    cursor::refresh_clip();
    log(&format!("hotkey: scale {} client {cw}x{ch}", current()));
}

/// ALT+F9: back to 1x.
pub fn reset(hwnd: HWND) {
    IDX.store(IDX_ONE, Ordering::Relaxed);
    apply(hwnd);
}

/// ALT+-: one ladder step down, clamped at the smallest scale.
pub fn smaller(hwnd: HWND) {
    let idx = IDX.load(Ordering::Relaxed);
    if idx == 0 {
        log(&format!("hotkey: ALT+- already at minimum {}", SCALES[0]));
        return;
    }
    IDX.store(idx - 1, Ordering::Relaxed);
    apply(hwnd);
}

/// ALT+=: one ladder step up, clamped at the largest scale.
pub fn bigger(hwnd: HWND) {
    let idx = IDX.load(Ordering::Relaxed);
    if idx == SCALES.len() - 1 {
        log(&format!("hotkey: ALT+= already at maximum {}", SCALES[SCALES.len() - 1]));
        return;
    }
    IDX.store(idx + 1, Ordering::Relaxed);
    apply(hwnd);
}
