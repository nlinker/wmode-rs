//! Game window subclassing: installs our window procedure and forwards
//! everything to the original. Behavior changes land with the hotkey and
//! render handling.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    ClientToScreen, CreatePen, CreateSolidBrush, DeleteObject, FillRect, GetStockObject,
    GetWindowDC, HDC, LineTo, MoveToEx, RedrawWindow, ReleaseDC, SelectObject, SetBkMode,
    SetTextColor, TextOutA, DEFAULT_GUI_FONT, PS_SOLID, RDW_FRAME, RDW_INVALIDATE, RDW_UPDATENOW,
    TRANSPARENT,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    VK_F1, VK_F10, VK_F11, VK_F12, VK_F9, VK_OEM_MINUS, VK_OEM_PLUS,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallWindowProcA, DefWindowProcA, EnableMenuItem, GetSystemMenu, GetWindowRect, LoadCursorW,
    PostMessageA, SetCursor, SetWindowLongA, SetWindowPos, ShowWindow, SystemParametersInfoA,
    GWL_WNDPROC, IDC_ARROW, WM_SETCURSOR,
    HTCAPTION, HTCLOSE, HTMAXBUTTON, HTMINBUTTON, HWND_NOTOPMOST, HWND_TOPMOST, MF_BYCOMMAND,
    MF_ENABLED, MF_GRAYED, SC_CLOSE, SC_MAXIMIZE,
    SC_MINIMIZE, SPI_GETSCREENSAVEACTIVE, SPI_SETSCREENSAVEACTIVE, SW_MINIMIZE, SWP_NOMOVE,
    ShowCursor, SWP_NOSIZE, GetClientRect, GetCursorPos, WA_INACTIVE, WM_ACTIVATE, WM_ACTIVATEAPP,
    WM_DISPLAYCHANGE,
    WM_MOUSEFIRST, WM_NCCALCSIZE, WM_NCHITTEST, WM_NCLBUTTONDOWN, WM_NCPAINT,
    WM_MOUSELAST, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_MOVE, WM_NCMOUSEMOVE,
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
/// Last known client position, cached on every move: the exit save runs
/// in DLL_PROCESS_DETACH where window queries are off limits.
static WINDOW_CLIENT_X: AtomicI32 = AtomicI32::new(-1);
static WINDOW_CLIENT_Y: AtomicI32 = AtomicI32::new(-1);
/// True while we turned the screensaver off and owe a restore.
static SCREENSAVER_OFF: AtomicBool = AtomicBool::new(false);

/// Shows or hides the arrow cursor. The call saturates the ShowCursor
/// counter instead of stepping it once: the game keeps hiding the OS
/// cursor every frame while it draws its own, so a single step loses.
/// Over the caption (nonclient) the arrow must win, over the client the
/// game cursor does.
fn set_cursor_show(show: bool) {
    let mut count = unsafe { ShowCursor(show as i32) };
    while (show && count < 0) || (!show && count > 0) {
        count = unsafe { ShowCursor(show as i32) };
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

/// Turns the screensaver off, remembering to restore it later. Only used
/// while the caption controls are disabled, per the readme.
unsafe fn screensaver_disable() {
    if SCREENSAVER_OFF.load(Ordering::Relaxed) {
        return;
    }
    let mut active = 0;
    SystemParametersInfoA(
        SPI_GETSCREENSAVEACTIVE,
        0,
        &mut active as *mut i32 as _,
        0,
    );
    if active != 0 {
        SystemParametersInfoA(SPI_SETSCREENSAVEACTIVE, 0, core::ptr::null_mut(), 0);
        SCREENSAVER_OFF.store(true, Ordering::Relaxed);
        log("window: screensaver disabled");
    }
}

/// Restores the screensaver if we are the ones who turned it off.
unsafe fn screensaver_restore() {
    if SCREENSAVER_OFF.swap(false, Ordering::Relaxed) {
        SystemParametersInfoA(SPI_SETSCREENSAVEACTIVE, 1, core::ptr::null_mut(), 0);
        log("window: screensaver restored");
    }
}

// Custom caption: a narrow black bar with a red frame and a row of state
// buttons, painted instead of the standard frame (original W-MODE look).

/// Caption button size and spacing.
const BTN: i32 = 12;
const BTN_GAP: i32 = 2;
const BTN_COUNT: usize = 6;
/// Hit-test codes for the caption buttons, kept clear of the HT_* range.
const HT_BTN_FIRST: usize = 0x100;
const HT_BTN_CLIP: usize = HT_BTN_FIRST;
const HT_BTN_MOVE: usize = HT_BTN_FIRST + 1;
const HT_BTN_TOPMOST: usize = HT_BTN_FIRST + 2;
const HT_BTN_CONTROLS: usize = HT_BTN_FIRST + 3;
const HT_BTN_MIN: usize = HT_BTN_FIRST + 4;
// HT_BTN_FIRST + 5 is the close button, handled by the wildcard arm

fn rgb(r: u32, g: u32, b: u32) -> u32 {
    r | (g << 8) | (b << 16)
}

/// Button boxes in window coordinates, right to left on the bar: close,
/// minimize, controls, topmost, move, clip.
fn button_rects(w: i32) -> [RECT; BTN_COUNT] {
    let y = render::FRAME + (render::BAR_H - BTN) / 2;
    let mut x = w - render::FRAME - BTN;
    let mut out = [RECT::default(); BTN_COUNT];
    let mut i = BTN_COUNT;
    while i > 0 {
        i -= 1;
        out[i] = RECT {
            left: x,
            top: y,
            right: x + BTN,
            bottom: y + BTN,
        };
        x -= BTN + BTN_GAP;
    }
    out
}

/// Repaints the caption bar and border after a state change.
unsafe fn redraw_frame(hwnd: HWND) {
    RedrawWindow(
        hwnd,
        core::ptr::null(),
        core::ptr::null_mut(),
        RDW_FRAME | RDW_INVALIDATE | RDW_UPDATENOW,
    );
}

/// One 1-pixel line with the currently selected pen.
unsafe fn line(hdc: HDC, x1: i32, y1: i32, x2: i32, y2: i32) {
    MoveToEx(hdc, x1, y1, core::ptr::null_mut());
    LineTo(hdc, x2, y2);
}

/// Paints the whole custom nonclient: the red border, the black caption
/// bar with the title, and the button row with its state glyphs.
unsafe fn paint_frame(hwnd: HWND) {
    let hdc = GetWindowDC(hwnd);
    let mut wr = RECT::default();
    GetWindowRect(hwnd, &mut wr);
    let w = wr.right - wr.left;
    let h = wr.bottom - wr.top;
    if w > 0 && h > 0 {
        let red = CreateSolidBrush(rgb(150, 0, 0));
        let black = CreateSolidBrush(rgb(0, 0, 0));
        FillRect(hdc, &RECT { left: 0, top: 0, right: w, bottom: render::FRAME }, red);
        FillRect(hdc, &RECT { left: 0, top: h - render::FRAME, right: w, bottom: h }, red);
        FillRect(hdc, &RECT { left: 0, top: 0, right: render::FRAME, bottom: h }, red);
        FillRect(hdc, &RECT { left: w - render::FRAME, top: 0, right: w, bottom: h }, red);
        FillRect(
            hdc,
            &RECT {
                left: render::FRAME,
                top: render::FRAME,
                right: w - render::FRAME,
                bottom: render::FRAME + render::BAR_H,
            },
            black,
        );
        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextColor(hdc, rgb(200, 200, 200));
        let old_font = SelectObject(hdc, GetStockObject(DEFAULT_GUI_FONT));
        let title = b"StarCraft";
        TextOutA(
            hdc,
            render::FRAME + 4,
            render::FRAME + (render::BAR_H - 9) / 2,
            title.as_ptr(),
            title.len() as i32,
        );
        SelectObject(hdc, old_font);

        let dim = !CAPTION_CONTROLS.load(Ordering::Relaxed);
        let grey = rgb(110, 110, 110);
        let off = rgb(90, 90, 90);
        let green = rgb(0, 210, 0);
        let boxbrush = CreateSolidBrush(rgb(35, 35, 35));
        // every stateful button carries its own on/off color: bright while
        // its switch is on, dim while off, grey all over while the caption
        // controls are disabled
        let pen_on = CreatePen(PS_SOLID, 1, green);
        let pen_off = CreatePen(PS_SOLID, 1, off);
        let pen_grey = CreatePen(PS_SOLID, 1, grey);
        let red_pen = CreatePen(PS_SOLID, 1, if dim { grey } else { rgb(200, 40, 40) });
        let old_pen = SelectObject(hdc, pen_grey);
        for (i, b) in button_rects(w).iter().enumerate() {
            let on = match HT_BTN_FIRST + i {
                HT_BTN_CLIP => cursor::clip_enabled(),
                HT_BTN_MOVE => move_enabled(),
                HT_BTN_TOPMOST => topmost(),
                _ => true,
            };
            let pen = if dim { pen_grey } else if on { pen_on } else { pen_off };
            SelectObject(hdc, pen);
            FillRect(hdc, b, boxbrush);
            let cx = (b.left + b.right) / 2;
            let cy = (b.top + b.bottom) / 2;
            match HT_BTN_FIRST + i {
                HT_BTN_CLIP => {
                    // filled center square mirrors the switch state
                    let fill = if dim { grey } else if on { green } else { off };
                    let brush = CreateSolidBrush(fill);
                    FillRect(hdc, &RECT { left: cx - 3, top: cy - 3, right: cx + 3, bottom: cy + 3 }, brush);
                    DeleteObject(brush);
                }
                HT_BTN_MOVE => {
                    line(hdc, cx - 4, cy, cx + 4, cy);
                    line(hdc, cx - 4, cy, cx - 2, cy - 2);
                    line(hdc, cx - 4, cy, cx - 2, cy + 2);
                    line(hdc, cx + 4, cy, cx + 2, cy - 2);
                    line(hdc, cx + 4, cy, cx + 2, cy + 2);
                }
                HT_BTN_TOPMOST => {
                    line(hdc, cx, cy + 4, cx, cy - 4);
                    line(hdc, cx, cy - 4, cx - 2, cy - 2);
                    line(hdc, cx, cy - 4, cx + 2, cy - 2);
                }
                HT_BTN_CONTROLS => {
                    for dy in [-3, 0, 3] {
                        line(hdc, cx - 4, cy + dy, cx + 4, cy + dy);
                    }
                }
                HT_BTN_MIN => {
                    line(hdc, cx - 4, cy + 3, cx + 4, cy + 3);
                }
                _ => {
                    let old = SelectObject(hdc, red_pen);
                    line(hdc, cx - 3, cy - 3, cx + 3, cy + 3);
                    line(hdc, cx - 3, cy + 3, cx + 3, cy - 3);
                    SelectObject(hdc, old);
                }
            }
        }
        SelectObject(hdc, old_pen);
        DeleteObject(boxbrush);
        DeleteObject(pen_on);
        DeleteObject(pen_off);
        DeleteObject(pen_grey);
        DeleteObject(red_pen);
        DeleteObject(red);
        DeleteObject(black);
    }
    ReleaseDC(hwnd, hdc);
}

/// Whether the physical cursor sits over the window frame (caption bar
/// or border) rather than the client area.
unsafe fn cursor_over_frame(hwnd: HWND) -> bool {
    let mut pt = POINT { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut pt) } == 0 {
        return false;
    }
    let mut wr = RECT::default();
    GetWindowRect(hwnd, &mut wr);
    let mut cr = RECT::default();
    GetClientRect(hwnd, &mut cr);
    let mut tl = POINT { x: cr.left, y: cr.top };
    let mut br = POINT { x: cr.right, y: cr.bottom };
    ClientToScreen(hwnd, &mut tl);
    ClientToScreen(hwnd, &mut br);
    pt.x >= wr.left
        && pt.x < wr.right
        && pt.y >= wr.top
        && pt.y < wr.bottom
        && !(pt.x >= tl.x && pt.x < br.x && pt.y >= tl.y && pt.y < br.y)
}

/// Per-frame arrow decision by cursor position. The game re-hides the OS
/// cursor when it regains focus or enters its screens, so whoever speaks
/// last wins - this runs at the end of every frame and makes the arrow
/// over the frame win within one frame.
pub fn arrow_frame_check() {
    let hwnd = render::game_hwnd();
    if hwnd.is_null() {
        return;
    }
    let over_frame = unsafe { cursor_over_frame(hwnd) };
    if over_frame {
        // the game nulls the cursor handle even over our caption, and
        // ShowCursor cannot fix a nulled handle - set the arrow explicitly
        unsafe {
            SetCursor(LoadCursorW(core::ptr::null_mut(), IDC_ARROW));
        }
    }
    set_cursor_show(over_frame);
}

/// Toggles the cursor clip switch, from the hotkey or the caption button.
unsafe fn toggle_clip(hwnd: HWND) {
    let on = !cursor::clip_enabled();
    cursor::set_clip_enabled(on);
    cursor::refresh_clip();
    log(&format!("hotkey: ALT+F1 cursor clip {}", if on { "ON" } else { "OFF" }));
    redraw_frame(hwnd);
}

/// Toggles window dragging.
unsafe fn toggle_move(hwnd: HWND) {
    let on = !move_enabled();
    MOVE_ENABLED.store(on, Ordering::Relaxed);
    log(&format!("hotkey: ALT+F10 window move {}", if on { "ON" } else { "OFF" }));
    redraw_frame(hwnd);
}

/// Toggles always-on-top.
unsafe fn toggle_topmost(hwnd: HWND) {
    let on = !topmost();
    TOPMOST.store(on, Ordering::Relaxed);
    apply_topmost(hwnd);
    log(&format!("hotkey: ALT+F11 always on top {}", if on { "ON" } else { "OFF" }));
    redraw_frame(hwnd);
}

/// Toggles the caption controls and the screensaver guard with them.
unsafe fn toggle_controls(hwnd: HWND) {
    let on = !caption_controls();
    CAPTION_CONTROLS.store(on, Ordering::Relaxed);
    apply_caption_controls(hwnd);
    if on {
        screensaver_restore();
    } else {
        screensaver_disable();
    }
    log(&format!("hotkey: ALT+F12 caption buttons {}", if on { "ON" } else { "OFF" }));
    redraw_frame(hwnd);
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
/// no window queries, the loader lock is held at that point. Also
/// restores the screensaver if we disabled it, so a game exit cannot
/// leave it off system-wide.
pub fn save_cached() {
    unsafe {
        screensaver_restore();
    }
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
    if msg == WM_NCCALCSIZE && wparam != 0 && render::windowed() {
        // carve the custom frame out of the window box: the bar on top,
        // the border on the other three sides
        let rct = lparam as *mut RECT;
        (*rct).left += render::FRAME;
        (*rct).top += render::FRAME + render::BAR_H;
        (*rct).right -= render::FRAME;
        (*rct).bottom -= render::FRAME;
        return 0;
    }
    if msg == WM_NCPAINT && render::windowed() {
        paint_frame(hwnd);
        return 0;
    }
    if msg == WM_NCHITTEST && render::windowed() {
        // lParam packs the cursor position in screen coordinates
        let mut wr = RECT::default();
        GetWindowRect(hwnd, &mut wr);
        let x = ((lparam as i32) & 0xFFFF) as i16 as i32 - wr.left;
        let y = (((lparam as i32) >> 16) & 0xFFFF) as i16 as i32 - wr.top;
        if y >= render::FRAME && y < render::FRAME + render::BAR_H {
            for (i, b) in button_rects(wr.right - wr.left).iter().enumerate() {
                if x >= b.left && x < b.right && y >= b.top && y < b.bottom {
                    return (HT_BTN_FIRST + i) as LRESULT;
                }
            }
            return HTCAPTION as LRESULT;
        }
    }
    if msg == WM_NCLBUTTONDOWN {
        let hit = wparam as usize;
        if (HT_BTN_FIRST..HT_BTN_FIRST + BTN_COUNT).contains(&hit) {
            // like the original: the icons go grey and inert while the
            // caption controls are off, only the hotkeys keep working
            if CAPTION_CONTROLS.load(Ordering::Relaxed) {
                match hit {
                    HT_BTN_CLIP => toggle_clip(hwnd),
                    HT_BTN_MOVE => toggle_move(hwnd),
                    HT_BTN_TOPMOST => toggle_topmost(hwnd),
                    HT_BTN_CONTROLS => toggle_controls(hwnd),
                    HT_BTN_MIN => {
                        ShowWindow(hwnd, SW_MINIMIZE);
                    }
                    _ => {
                        PostMessageA(hwnd, WM_SYSCOMMAND, SC_CLOSE as usize, 0);
                    }
                }
            }
            return 0;
        }
    }
    if msg == WM_ACTIVATE {
        if wparam as u32 & 0xFFFF != WA_INACTIVE {
            // the system drops ClipCursor confinement when the window
            // deactivates: bring it back on every activation while the
            // switch is on, or Alt+Tab leaves the cursor free forever
            cursor::refresh_clip();
            // the game left the arrow hidden before deactivation and no
            // NC mouse move will come until the mouse moves: decide the
            // arrow state from where the cursor sits right now
            set_cursor_show(unsafe { cursor_over_frame(hwnd) });
            // the screensaver half of DisableControls: off while the
            // game window is the foreground one
            if !CAPTION_CONTROLS.load(Ordering::Relaxed) {
                screensaver_disable();
            }
        } else {
            screensaver_restore();
        }
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
            VK_F1 => toggle_clip(hwnd),
            VK_F10 => toggle_move(hwnd),
            VK_F11 => toggle_topmost(hwnd),
            VK_F12 => toggle_controls(hwnd),
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
    if msg == WM_SETCURSOR && render::windowed() && unsafe { cursor_over_frame(hwnd) } {
        // the game's WM_SETCURSOR handling nulls the cursor over the
        // whole window including our caption: answer for the frame zone
        // ourselves before it runs
        unsafe {
            SetCursor(LoadCursorW(core::ptr::null_mut(), IDC_ARROW));
        }
        return 1;
    }
    if msg == WM_MOUSEMOVE || msg == WM_NCMOUSEMOVE {
        // decide by geometry, not message flavor: the game captures the
        // mouse in its menus, and captured moves over the caption arrive
        // as plain WM_MOUSEMOVE, never as a nonclient message
        set_cursor_show(unsafe { cursor_over_frame(hwnd) });
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
