//! Cursor translation between the game's coordinate space (a 640x480
//! screen rooted at the desktop origin) and the real window client area
//! elsewhere on screen. The game clips and positions the cursor as if it
//! still owned the whole screen, so the three user32 imports it calls get
//! patched in the game executable and remapped onto the window.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};

use windows_sys::core::BOOL;
use windows_sys::Win32::Foundation::{FALSE, HWND, LPARAM, POINT, RECT, TRUE};
use windows_sys::Win32::Graphics::Gdi::{ClientToScreen, ScreenToClient};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleA;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    ClipCursor, GetClientRect, GetCursorPos, GetForegroundWindow, SetCursorPos,
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
/// User switch for cursor clipping, the ALT+F1 state. Default off, per
/// the sample ini.
static CLIP_ENABLED: AtomicBool = AtomicBool::new(false);
/// Diagnostic mouse log: how many samples are left before it goes quiet.
static MOUSE_LOG_BUDGET: AtomicU32 = AtomicU32::new(400);
/// Total GetCursorPos calls seen, for sampling the log.
static MOUSE_CALLS: AtomicU32 = AtomicU32::new(0);
/// Last logged screen position, to skip samples where nothing moved.
static MOUSE_X: AtomicI32 = AtomicI32::new(i32::MIN);
static MOUSE_Y: AtomicI32 = AtomicI32::new(i32::MIN);

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

unsafe extern "system" fn get_cursor_pos(point: *mut POINT) -> BOOL {
    let res = real_get(point);
    if res == FALSE || point.is_null() {
        return res;
    }
    let Some(m) = mapping() else {
        return res;
    };
    let (sx, sy) = ((*point).x, (*point).y);
    let focused = GetForegroundWindow() == m.hwnd;
    // a focused game sees the real cursor, clamped to the game rect so
    // edge scrolling never stalls. An unfocused game always sees the
    // center: it keeps polling this position while deactivated, and a
    // stray mouse over or away from the window must not scroll its map.
    // Clicks still land: the click that raises the window focuses it
    // before the button message is delivered.
    if focused {
        ScreenToClient(m.hwnd, point);
        (*point).x = (*point).x * m.gw / m.cw;
        (*point).y = (*point).y * m.gh / m.ch;
        // the game scrolls when the cursor reaches the edge of its rect:
        // clamp anything outside so edge scroll never stalls
        (*point).x = (*point).x.clamp(0, m.gw - 1);
        (*point).y = (*point).y.clamp(0, m.gh - 1);
    } else {
        (*point).x = m.gw / 2;
        (*point).y = m.gh / 2;
    }
    debug_mouse(&m, sx, sy, (*point).x, (*point).y, focused);
    TRUE
}

/// Diagnostic sample: screen position (absolute), position relative to
/// the StarCraft client area, and the game coordinates the hook handed
/// out. The game polls every frame, so only every 8th changed position
/// gets logged, until the budget runs out.
fn debug_mouse(m: &Map, sx: i32, sy: i32, gx: i32, gy: i32, focused: bool) {
    let calls = MOUSE_CALLS.fetch_add(1, Ordering::Relaxed);
    if MOUSE_X.load(Ordering::Relaxed) == sx && MOUSE_Y.load(Ordering::Relaxed) == sy {
        return;
    }
    MOUSE_X.store(sx, Ordering::Relaxed);
    MOUSE_Y.store(sy, Ordering::Relaxed);
    if calls % 8 != 0 {
        return;
    }
    let budget = MOUSE_LOG_BUDGET.fetch_sub(1, Ordering::Relaxed);
    if budget == 0 {
        MOUSE_LOG_BUDGET.store(0, Ordering::Relaxed);
        return;
    }
    log(&format!(
        "mouse: screen ({sx},{sy}) client ({},{}) game ({gx},{gy}) fg={} clip={}",
        sx - m.ox,
        sy - m.oy,
        focused as u8,
        clip_enabled()
    ));
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
    if !render::windowed() {
        return real_clip(rect);
    }
    // while windowed the clip is ours: game requests are swallowed and
    // only the switch state decides, so the visible state always equals
    // the flag (plus the ALT inversion), whatever the game asks for
    TRUE
}

/// Whether the user wants the cursor confined, the ALT+F1 state.
pub fn clip_enabled() -> bool {
    CLIP_ENABLED.load(Ordering::Relaxed)
}

pub fn set_clip_enabled(on: bool) {
    CLIP_ENABLED.store(on, Ordering::Relaxed);
}

/// Brings the cursor clip in line with the current switch state:
/// confines the cursor to the game client area, or frees it.
pub fn refresh_clip() {
    apply_state(CLIP_ENABLED.load(Ordering::Relaxed));
}

fn apply_state(confine: bool) {
    let m = unsafe { mapping() };
    unsafe {
        match (confine, m) {
            (true, Some(m)) => {
                let rect = RECT {
                    left: m.ox,
                    top: m.oy,
                    right: m.ox + m.cw,
                    bottom: m.oy + m.ch,
                };
                log(&format!(
                    "clip: confined to ({},{})-({},{})",
                    rect.left, rect.top, rect.right, rect.bottom
                ));
                real_clip(&rect);
            }
            _ => {
                log("clip: released");
                real_clip(core::ptr::null());
            }
        }
    }
}

/// Rescales mouse message coordinates from client pixels to game units
/// while the client is not the game size. lParam packs two signed shorts.
pub fn fix_points(lparam: LPARAM) -> LPARAM {
    let Some(m) = (unsafe { mapping() }) else {
        return lparam;
    };
    if m.cw == m.gw && m.ch == m.gh {
        return lparam;
    }
    let x = (lparam as i32) as u16 as i32;
    let y = (((lparam as i32) >> 16) & 0xFFFF) as u16 as i32;
    let gx = (x * m.gw / m.cw).clamp(0, 0xFFFF as i32);
    let gy = (y * m.gh / m.ch).clamp(0, 0xFFFF as i32);
    ((gy << 16) | gx) as LPARAM
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
