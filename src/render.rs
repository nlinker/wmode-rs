//! Windowed render path: an 8-bpp DIB section the game draws into through
//! the hooked SDraw functions, blitted to the client area on WM_PAINT.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::Mutex;

use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, ClientToScreen, CreateCompatibleDC, CreateDIBSection, DeleteDC,
    DeleteObject, EndPaint, GetDC, InvalidateRect, ReleaseDC, SelectObject, SetDIBColorTable,
    SetStretchBltMode, StretchDIBits, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
    HALFTONE, HDC, HBITMAP, PAINTSTRUCT, PALETTEENTRY, RGBQUAD, SRCCOPY,
};
use windows_sys::Win32::System::SystemInformation::GetTickCount;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetSystemMetrics, GetWindowRect, IsIconic, SetWindowLongA, SetWindowPos,
    GWL_STYLE, HWND_NOTOPMOST, SM_CXSCREEN, SM_CYSCREEN, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    SWP_SHOWWINDOW, WS_POPUP, WS_SYSMENU,
};

use crate::log::log;

#[repr(C)]
struct BitmapInfo256 {
    header: BITMAPINFOHEADER,
    colors: [RGBQUAD; 256],
}

struct Render {
    hdc_mem: HDC,
    hbmp: HBITMAP,
    old_bmp: HBITMAP,
    bits: *mut core::ffi::c_void,
    bmi: BitmapInfo256,
    width: i32,
    height: i32,
}

unsafe impl Send for Render {}

static RENDER: Mutex<Option<Render>> = Mutex::new(None);
/// The game window handle, set once the window exists.
static GAME_HWND: std::sync::atomic::AtomicPtr<core::ffi::c_void> =
    std::sync::atomic::AtomicPtr::new(core::ptr::null_mut());
static WINDOWED: AtomicBool = AtomicBool::new(false);
/// Game screen size of the current DIB, for coordinate translation.
static GAME_W: AtomicI32 = AtomicI32::new(DEFAULT_WIDTH);
static GAME_H: AtomicI32 = AtomicI32::new(DEFAULT_HEIGHT);
static DIRTY: AtomicBool = AtomicBool::new(false);
static LAST_PAINT_TICK: AtomicU32 = AtomicU32::new(0);
/// How many client-size corrections get logged before going quiet.
static CORRECT_LOG_BUDGET: AtomicU32 = AtomicU32::new(10);
/// Minimum milliseconds between repaints, from the MaxFps ini key.
static PAINT_INTERVAL_MS: AtomicU32 = AtomicU32::new(10);

/// Sets the repaint throttle from a MaxFps value, clamped to at least
/// 30 fps per the template note.
pub fn set_max_fps(fps: i32) {
    let fps = fps.clamp(30, 1000);
    PAINT_INTERVAL_MS.store(1000 / fps as u32, Ordering::Relaxed);
}

pub fn windowed() -> bool {
    WINDOWED.load(Ordering::Acquire)
}

/// The game window handle, null until the window exists.
pub fn game_hwnd() -> HWND {
    GAME_HWND.load(Ordering::Acquire)
}

/// The game screen size the DIB currently carries.
pub fn game_size() -> (i32, i32) {
    (
        GAME_W.load(Ordering::Acquire),
        GAME_H.load(Ordering::Acquire),
    )
}

/// Custom nonclient metrics: the narrow caption bar height and the frame
/// border width, painted by the window subclass.
pub const BAR_H: i32 = 18;
pub const FRAME: i32 = 3;

/// Outer window size that yields a client of cw x ch under the custom
/// popup frame: the bar on top, the border around. Constant math, no
/// live style measuring (a just-restyled window reports stale metrics).
unsafe fn outer_size_for(_hwnd: HWND, cw: i32, ch: i32) -> (i32, i32) {
    (cw + FRAME * 2, ch + BAR_H + FRAME)
}

/// Creates the DIB surface for the given game size, replacing any previous
/// surface. Palette is preserved on recreate. The screen DC is enough, so
/// this can run before the game window exists.
unsafe fn create_surface(width: i32, height: i32) -> bool {
    let mut guard = RENDER.lock().unwrap();
    let mut bmi = BitmapInfo256 {
        header: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // top-down
            biPlanes: 1,
            biBitCount: 8,
            biCompression: BI_RGB,
            biSizeImage: (width * height) as u32,
            ..BITMAPINFOHEADER::default()
        },
        colors: [RGBQUAD::default(); 256],
    };
    if let Some(old) = guard.as_ref() {
        bmi.colors = old.bmi.colors;
    }

    let screen: HWND = core::ptr::null_mut();
    let hdc_window = GetDC(screen);
    let mut bits = core::ptr::null_mut();
    let hbmp = CreateDIBSection(
        hdc_window,
        &bmi as *const BitmapInfo256 as *const BITMAPINFO,
        DIB_RGB_COLORS,
        &mut bits,
        core::ptr::null_mut(),
        0,
    );
    let hdc_mem = CreateCompatibleDC(hdc_window);
    ReleaseDC(screen, hdc_window);
    if hbmp.is_null() || hdc_mem.is_null() || bits.is_null() {
        if !hdc_mem.is_null() {
            DeleteDC(hdc_mem);
        }
        return false;
    }
    let old_bmp = SelectObject(hdc_mem, hbmp);

    if let Some(previous) = guard.take() {
        SelectObject(previous.hdc_mem, previous.old_bmp);
        DeleteDC(previous.hdc_mem);
        DeleteObject(previous.hbmp);
    }
    *guard = Some(Render {
        hdc_mem,
        hbmp,
        old_bmp,
        bits,
        bmi,
        width,
        height,
    });
    GAME_W.store(width, Ordering::Release);
    GAME_H.store(height, Ordering::Release);
    true
}

/// Creates the initial DIB surface before the game window exists, so the
/// fake DirectDraw surface Lock has real bits to hand out from the very
/// first frame.
pub fn prepare() {
    if !unsafe { create_surface(DEFAULT_WIDTH, DEFAULT_HEIGHT) } {
        log("render: initial DIB surface creation FAILED");
    }
}

/// The game screen size until the first SDraw lock says otherwise.
const DEFAULT_WIDTH: i32 = 640;
const DEFAULT_HEIGHT: i32 = 480;

/// Switches the game window to a decorated, centered, 1x-sized window and
/// marks the render path active.
pub fn activate_window(hwnd: HWND) {
    let width = DEFAULT_WIDTH;
    let height = DEFAULT_HEIGHT;
    GAME_HWND.store(hwnd, Ordering::Release);
    // flag the render path active before the resize so the very first
    // WM_NCCALCSIZE already carves out the custom bar and border
    WINDOWED.store(true, Ordering::Release);
    unsafe {
        SetWindowLongA(hwnd, GWL_STYLE, (WS_POPUP | WS_SYSMENU) as i32);
        let (ow, oh) = outer_size_for(hwnd, width, height);
        let cx = GetSystemMetrics(SM_CXSCREEN);
        let cy = GetSystemMetrics(SM_CYSCREEN);
        SetWindowPos(
            hwnd,
            HWND_NOTOPMOST,
            (cx - ow) / 2,
            (cy - oh) / 2,
            ow,
            oh,
            SWP_SHOWWINDOW,
        );
    }
    DIRTY.store(true, Ordering::Release);
    let mut client = RECT::default();
    let mut outer = RECT::default();
    unsafe {
        GetClientRect(hwnd, &mut client);
        GetWindowRect(hwnd, &mut outer);
    }
    log(&format!(
        "render: windowed mode active {width}x{height}, client {}x{}, window {} {} {} {}",
        client.right, client.bottom, outer.left, outer.top, outer.right, outer.bottom
    ));
}

/// Client top left in screen coordinates.
pub unsafe fn client_origin(hwnd: HWND) -> (i32, i32) {
    let mut pt = POINT { x: 0, y: 0 };
    ClientToScreen(hwnd, &mut pt);
    (pt.x, pt.y)
}

/// Moves the window so the client top left lands at x y, clamped to the
/// visible desktop. Negative coordinates mean "leave it where it is".
pub unsafe fn set_client_origin(hwnd: HWND, x: i32, y: i32) {
    if x < 0 || y < 0 {
        return;
    }
    let (ox, oy) = client_origin(hwnd);
    let mut outer = RECT::default();
    GetWindowRect(hwnd, &mut outer);
    let cx = GetSystemMetrics(SM_CXSCREEN);
    let cy = GetSystemMetrics(SM_CYSCREEN);
    let w = outer.right - outer.left;
    let h = outer.bottom - outer.top;
    let wx = (outer.left + (x - ox)).clamp(0, (cx - w).max(0));
    let wy = (outer.top + (y - oy)).clamp(0, (cy - h).max(0));
    SetWindowPos(
        hwnd,
        core::ptr::null_mut(),
        wx,
        wy,
        0,
        0,
        SWP_NOSIZE | SWP_NOZORDER,
    );
}

/// Resizes the window so the client becomes cw x ch, keeping the top
/// left corner inside the visible desktop. A no-op when the client
/// already carries that size.
pub unsafe fn set_client_size(hwnd: HWND, cw: i32, ch: i32) {
    let mut client = RECT::default();
    GetClientRect(hwnd, &mut client);
    if client.right == cw && client.bottom == ch {
        return;
    }
    let (ow, oh) = outer_size_for(hwnd, cw, ch);
    let mut outer = RECT::default();
    GetWindowRect(hwnd, &mut outer);
    let cx = GetSystemMetrics(SM_CXSCREEN);
    let cy = GetSystemMetrics(SM_CYSCREEN);
    let x = outer.left.clamp(0, (cx - ow).max(0));
    let y = outer.top.clamp(0, (cy - oh).max(0));
    SetWindowPos(hwnd, core::ptr::null_mut(), x, y, ow, oh, SWP_NOZORDER);
}

/// Restores the client area to the scaled target size: storm keeps
/// sizing the window to an outer 640x480, so every WM_SIZE snaps the
/// outer size back to wanted client plus frame. Skips minimized windows
/// (stub geometry). Returns true when a correction ran.
pub unsafe fn correct_client_size(hwnd: HWND) -> bool {
    if IsIconic(hwnd) != 0 {
        // correcting the parked placeholder would resize it
        return false;
    }
    let (want_w, want_h) = crate::scale::client_wanted();
    let mut client = RECT::default();
    GetClientRect(hwnd, &mut client);
    if client.right == want_w && client.bottom == want_h {
        return false;
    }
    if client.right <= 0 || client.bottom <= 0 {
        return false;
    }
    let budget = CORRECT_LOG_BUDGET.load(Ordering::Relaxed);
    if budget > 0 {
        CORRECT_LOG_BUDGET.store(budget - 1, Ordering::Relaxed);
        let mut outer = RECT::default();
        GetWindowRect(hwnd, &mut outer);
        log(&format!(
            "render: correcting client {}x{} back to {want_w}x{want_h}, window {} {} {} {}",
            client.right, client.bottom, outer.left, outer.top, outer.right, outer.bottom
        ));
    }
    let (ow, oh) = outer_size_for(hwnd, want_w, want_h);
    SetWindowPos(
        hwnd,
        core::ptr::null_mut(),
        0,
        0,
        ow,
        oh,
        SWP_NOMOVE | SWP_NOZORDER,
    );
    true
}

/// SDrawLockSurface: hands the game our DIB bits, recreating the surface
/// when the requested size changes.
pub fn lock(
    dest: *mut RECT,
    surface: *mut *mut core::ffi::c_void,
    pitch: *mut i32,
) -> bool {
    let mut guard = RENDER.lock().unwrap();
    let Some(render) = guard.as_mut() else {
        return false;
    };
    let mut width = render.width;
    let mut height = render.height;
    if !dest.is_null() {
        let rect = unsafe { &*dest };
        if rect.right - rect.left > 0 && rect.bottom - rect.top > 0 {
            width = rect.right - rect.left;
            height = rect.bottom - rect.top;
        }
    }
    if width != render.width || height != render.height {
        drop(guard);
        if !unsafe { create_surface(width, height) } {
            return false;
        }
        let mut guard = RENDER.lock().unwrap();
        let Some(render) = guard.as_mut() else {
            return false;
        };
        unsafe {
            *surface = render.bits;
            *pitch = render.width;
        }
        return true;
    }
    unsafe {
        *surface = render.bits;
        *pitch = render.width;
    }
    true
}

/// SDrawUnlockSurface: marks the frame dirty and asks for a repaint.
pub fn unlock() -> bool {
    let guard = RENDER.lock().unwrap();
    if guard.is_none() {
        return false;
    }
    DIRTY.store(true, Ordering::Release);
    let hwnd = GAME_HWND.load(Ordering::Acquire);
    if !hwnd.is_null() {
        unsafe {
            InvalidateRect(hwnd, core::ptr::null(), 0);
        }
    }
    true
}

/// SDrawUpdatePalette: copies palette entries into the DIB color table.
pub fn update_palette(first: u32, count: u32, entries: *const PALETTEENTRY) -> bool {
    let mut guard = RENDER.lock().unwrap();
    let Some(render) = guard.as_mut() else {
        return false;
    };
    if entries.is_null() {
        return true;
    }
    let entries = unsafe { core::slice::from_raw_parts(entries, count as usize) };
    for (offset, entry) in entries.iter().enumerate() {
        let index = first as usize + offset;
        if index < 256 {
            render.bmi.colors[index] = RGBQUAD {
                rgbBlue: entry.peBlue,
                rgbGreen: entry.peGreen,
                rgbRed: entry.peRed,
                rgbReserved: 0,
            };
        }
    }
    unsafe {
        SetDIBColorTable(
            render.hdc_mem,
            first,
            count,
            render.bmi.colors.as_ptr(),
        );
    }
    true
}

/// Fills a DDSURFACEDESC for the fake DirectDraw surface Lock: our DIB
/// bits, pitch and size. Layout offsets per ddraw.h (dwSize 0, dwFlags 4,
/// dwHeight 8, dwWidth 12, lPitch 16, dwReserved 20, lpSurface 24).
pub fn lock_surface_desc(desc: *mut core::ffi::c_void) {
    let guard = RENDER.lock().unwrap();
    let Some(render) = guard.as_ref() else {
        return;
    };
    unsafe {
        let base = desc as *mut u8;
        let size = (base as *const u32).read_unaligned();
        if size < 28 {
            return;
        }
        (base.add(4) as *mut u32).write_unaligned(
            0x0000_0002 | 0x0000_0004 | 0x0000_0008 | 0x0000_0800, // HEIGHT|WIDTH|PITCH|LPSURFACE
        );
        (base.add(8) as *mut u32).write_unaligned(render.height as u32);
        (base.add(12) as *mut u32).write_unaligned(render.width as u32);
        (base.add(16) as *mut i32).write_unaligned(render.width);
        (base.add(24) as *mut *mut core::ffi::c_void).write_unaligned(render.bits);
    }
}

/// WM_PAINT body: blits the DIB to the client area when a frame is due.
pub fn paint(hwnd: HWND) {
    if !windowed() {
        return;
    }
    let now = unsafe { GetTickCount() };
    let last = LAST_PAINT_TICK.load(Ordering::Relaxed);
    if now.wrapping_sub(last) < PAINT_INTERVAL_MS.load(Ordering::Relaxed) {
        return; // keep dirty, the next unlock re-arms the repaint
    }
    if !DIRTY.swap(false, Ordering::AcqRel) {
        return;
    }
    LAST_PAINT_TICK.store(now, Ordering::Relaxed);

    let guard = RENDER.lock().unwrap();
    let Some(render) = guard.as_ref() else {
        return;
    };
    unsafe {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        let mut client = RECT::default();
        GetClientRect(hwnd, &mut client);
        let cw = client.right - client.left;
        let ch = client.bottom - client.top;
        if cw == render.width && ch == render.height {
            BitBlt(hdc, 0, 0, render.width, render.height, render.hdc_mem, 0, 0, SRCCOPY);
        } else {
            SetStretchBltMode(hdc, HALFTONE);
            StretchDIBits(
                hdc,
                0,
                0,
                cw,
                ch,
                0,
                0,
                render.width,
                render.height,
                render.bits,
                &render.bmi as *const BitmapInfo256 as *const BITMAPINFO,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
        }
        EndPaint(hwnd, &ps);
    }
}
