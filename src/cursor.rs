//! Cursor translation between the game's coordinate space (a 640x480
//! screen rooted at the desktop origin) and the real window client area
//! elsewhere on screen. The game clips and positions the cursor as if it
//! still owned the whole screen, so the three user32 imports it calls get
//! patched in the game executable and remapped onto the window.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

use windows_sys::core::BOOL;
use windows_sys::Win32::Foundation::{FALSE, HWND, POINT, RECT, TRUE};
use windows_sys::Win32::Graphics::Gdi::{ClientToScreen, ScreenToClient};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleA;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    ClipCursor, GetClientRect, GetCursorPos, GetWindowRect, SetCursorPos,
};

use crate::iat;
use crate::log::log;
use crate::render;

type GetCursorPosFn = unsafe extern "system" fn(*mut POINT) -> BOOL;
type SetCursorPosFn = unsafe extern "system" fn(i32, i32) -> BOOL;
type ClipCursorFn = unsafe extern "system" fn(*const RECT) -> BOOL;

static GET_OLD: AtomicU32 = AtomicU32::new(0);
static SET_OLD: AtomicU32 = AtomicU32::new(0);
static CLIP_OLD: AtomicU32 = AtomicU32::new(0);
/// Last clip rect the game asked for, in game coordinates.
static CLIP_REQUEST: Mutex<Option<RECT>> = Mutex::new(None);
/// User switch for cursor clipping, the ALT+F1 state. Default off, per
/// the sample ini.
static CLIP_ENABLED: AtomicBool = AtomicBool::new(false);
/// Holding ALT temporarily inverts the clip switch.
static ALT_HELD: AtomicBool = AtomicBool::new(false);
/// How many clip calls get logged before the diagnostic goes quiet.
static LOG_BUDGET: AtomicU32 = AtomicU32::new(20);

/// Window geometry needed for translation, all in one snapshot.
struct Map {
    hwnd: HWND,
    /// Game screen size (the DIB the game draws into).
    gw: i32,
    gh: i32,
    /// Client area size in pixels.
    cw: i32,
    ch: i32,
    /// Client top left in screen coordinates.
    ox: i32,
    oy: i32,
}

/// Collects the translation snapshot, or None while the window does not
/// exist yet or windowed mode is off (calls then pass through unchanged).
unsafe fn mapping() -> Option<Map> {
    if !render::windowed() {
        return None;
    }
    let hwnd = render::game_hwnd();
    if hwnd.is_null() {
        return None;
    }
    let mut client = RECT::default();
    GetClientRect(hwnd, &mut client);
    let cw = client.right - client.left;
    let ch = client.bottom - client.top;
    if cw <= 0 || ch <= 0 {
        return None;
    }
    let (gw, gh) = render::game_size();
    if gw <= 0 || gh <= 0 {
        return None;
    }
    let mut origin = POINT { x: 0, y: 0 };
    ClientToScreen(hwnd, &mut origin);
    Some(Map {
        hwnd,
        gw,
        gh,
        cw,
        ch,
        ox: origin.x,
        oy: origin.y,
    })
}

unsafe fn real_get(point: *mut POINT) -> BOOL {
    let old = GET_OLD.load(Ordering::Acquire);
    if old != 0 {
        let f: GetCursorPosFn = core::mem::transmute(old as usize);
        f(point)
    } else {
        GetCursorPos(point)
    }
}

unsafe fn real_set(x: i32, y: i32) -> BOOL {
    let old = SET_OLD.load(Ordering::Acquire);
    if old != 0 {
        let f: SetCursorPosFn = core::mem::transmute(old as usize);
        f(x, y)
    } else {
        SetCursorPos(x, y)
    }
}

unsafe fn real_clip(rect: *const RECT) -> BOOL {
    let old = CLIP_OLD.load(Ordering::Acquire);
    if old != 0 {
        let f: ClipCursorFn = core::mem::transmute(old as usize);
        f(rect)
    } else {
        ClipCursor(rect)
    }
}

/// Translates one clip rect from game coordinates to the screen, clamped
/// to the client area, and applies it.
unsafe fn apply_clip(rect: RECT) -> BOOL {
    if let Some(m) = mapping() {
        // a pre-translated caller already anchors the rect at the client
        // origin, strip that anchor or the origin lands in twice
        let mut r = rect;
        if r.left == m.ox && r.top == m.oy {
            r.left = 0;
            r.top = 0;
            r.right -= m.ox;
            r.bottom -= m.oy;
        }
        let mut out = RECT {
            left: m.ox + r.left * m.cw / m.gw,
            top: m.oy + r.top * m.ch / m.gh,
            right: m.ox + (r.right * m.cw + m.gw - 1) / m.gw,
            bottom: m.oy + (r.bottom * m.ch + m.gh - 1) / m.gh,
        };
        out.left = out.left.max(m.ox);
        out.top = out.top.max(m.oy);
        out.right = out.right.min(m.ox + m.cw);
        out.bottom = out.bottom.min(m.oy + m.ch);
        log_clip(&rect, &out, &m);
        if out.right > out.left && out.bottom > out.top {
            return real_clip(&out);
        }
    }
    real_clip(&rect)
}

/// One-shot diagnostic of the translation math.
unsafe fn log_clip(req: &RECT, out: &RECT, m: &Map) {
    let budget = LOG_BUDGET.load(Ordering::Relaxed);
    if budget == 0 {
        return;
    }
    LOG_BUDGET.store(budget - 1, Ordering::Relaxed);
    let mut outer = RECT::default();
    GetWindowRect(m.hwnd, &mut outer);
    log(&format!(
        "cursor: clip req {} {} {} {} -> {} {} {} {} client {}x{} game {}x{} origin {} {} window {} {} {} {}",
        req.left, req.top, req.right, req.bottom,
        out.left, out.top, out.right, out.bottom,
        m.cw, m.ch, m.gw, m.gh, m.ox, m.oy,
        outer.left, outer.top, outer.right, outer.bottom
    ));
}

unsafe extern "system" fn get_cursor_pos(point: *mut POINT) -> BOOL {
    let res = real_get(point);
    if res == FALSE || point.is_null() {
        return res;
    }
    if let Some(m) = mapping() {
        ScreenToClient(m.hwnd, point);
        (*point).x = (*point).x * m.gw / m.cw;
        (*point).y = (*point).y * m.gh / m.ch;
    }
    TRUE
}

unsafe extern "system" fn set_cursor_pos(x: i32, y: i32) -> BOOL {
    if let Some(m) = mapping() {
        let mut pt = POINT {
            x: x * m.cw / m.gw,
            y: y * m.ch / m.gh,
        };
        ClientToScreen(m.hwnd, &mut pt);
        return real_set(pt.x, pt.y);
    }
    real_set(x, y)
}

unsafe extern "system" fn clip_cursor(rect: *const RECT) -> BOOL {
    if rect.is_null() {
        *CLIP_REQUEST.lock().unwrap() = None;
        return real_clip(core::ptr::null());
    }
    let rect = *rect;
    *CLIP_REQUEST.lock().unwrap() = Some(rect);
    if !clip_active() {
        return real_clip(core::ptr::null());
    }
    apply_clip(rect)
}

/// Whether the user wants the cursor confined, the ALT+F1 state.
pub fn clip_enabled() -> bool {
    CLIP_ENABLED.load(Ordering::Relaxed)
}

pub fn set_clip_enabled(on: bool) {
    CLIP_ENABLED.store(on, Ordering::Relaxed);
}

/// The clip applies only with the switch on and ALT not held: holding
/// ALT inverts the switch for as long as it is down.
fn clip_active() -> bool {
    CLIP_ENABLED.load(Ordering::Relaxed) != ALT_HELD.load(Ordering::Relaxed)
}

/// ALT press or release: refreshes the clip when the effective state
/// actually changed.
pub fn set_alt_held(held: bool) {
    if ALT_HELD.swap(held, Ordering::Relaxed) != held {
        refresh_clip();
    }
}

/// Brings the applied cursor clip in line with the current switch
/// state: re-applies the remembered game rect, or frees the cursor.
pub fn refresh_clip() {
    let request = *CLIP_REQUEST.lock().unwrap();
    unsafe {
        if clip_active() {
            if let Some(rect) = request {
                apply_clip(rect);
            }
        } else {
            real_clip(core::ptr::null());
        }
    }
}

/// Patches the three cursor imports in the game executable. Returns true
/// when every one of them was found, false otherwise (the game keeps
/// running either way, only cursor behavior degrades).
pub fn install() -> bool {
    let exe = unsafe { GetModuleHandleA(core::ptr::null()) };
    let get_fn: GetCursorPosFn = get_cursor_pos;
    let set_fn: SetCursorPosFn = set_cursor_pos;
    let clip_fn: ClipCursorFn = clip_cursor;

    let mut ok = true;
    match unsafe {
        iat::patch_import(exe, "user32.dll", "GetCursorPos", get_fn as usize as _)
    } {
        Some(old) => {
            GET_OLD.store(old, Ordering::Release);
            log(&format!("cursor: GetCursorPos patched, old {old:#010x}"));
        }
        None => {
            ok = false;
            log("cursor: GetCursorPos NOT PATCHED");
        }
    }
    match unsafe { iat::patch_import(exe, "user32.dll", "SetCursorPos", set_fn as usize as _) } {
        Some(old) => {
            SET_OLD.store(old, Ordering::Release);
            log(&format!("cursor: SetCursorPos patched, old {old:#010x}"));
        }
        None => {
            ok = false;
            log("cursor: SetCursorPos NOT PATCHED");
        }
    }
    match unsafe { iat::patch_import(exe, "user32.dll", "ClipCursor", clip_fn as usize as _) } {
        Some(old) => {
            CLIP_OLD.store(old, Ordering::Release);
            log(&format!("cursor: ClipCursor patched, old {old:#010x}"));
        }
        None => {
            ok = false;
            log("cursor: ClipCursor NOT PATCHED");
        }
    }
    ok
}
