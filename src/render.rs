//! Windowed render path: an 8-bpp DIB section the game draws into through
//! the hooked SDraw functions, blitted to the client area on WM_PAINT.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::Mutex;

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, EndPaint,
    GetDC, InvalidateRect, ReleaseDC, SelectObject, SetDIBColorTable, SetStretchBltMode,
    StretchDIBits, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HALFTONE, HDC, HBITMAP,
    PAINTSTRUCT, PALETTEENTRY, RGBQUAD, SRCCOPY,
};
use windows_sys::Win32::System::SystemInformation::GetTickCount;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, GetClientRect, GetSystemMetrics, GetWindowLongA, GetWindowRect,
    SetWindowLongA, SetWindowPos, GWL_EXSTYLE, GWL_STYLE, HWND_NOTOPMOST, SM_CXSCREEN,
    SM_CYSCREEN, SWP_NOMOVE, SWP_NOZORDER, SWP_SHOWWINDOW, WS_OVERLAPPEDWINDOW,
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
/// Minimum milliseconds between repaints.
const PAINT_INTERVAL_MS: u32 = 20;

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

/// Outer window size that yields a client of cw x ch under the window
/// styles currently set. Computed from the style flags alone, the frame
/// does not have to be applied to the window yet.
unsafe fn outer_size_for(hwnd: HWND, cw: i32, ch: i32) -> (i32, i32) {
    let style = GetWindowLongA(hwnd, GWL_STYLE) as u32;
    let ex = GetWindowLongA(hwnd, GWL_EXSTYLE) as u32;
    let mut r = RECT {
        left: 0,
        top: 0,
        right: cw,
        bottom: ch,
    };
    AdjustWindowRectEx(&mut r, style, 0, ex);
    (r.right - r.left, r.bottom - r.top)
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
    unsafe {
        SetWindowLongA(hwnd, GWL_STYLE, WS_OVERLAPPEDWINDOW as i32);
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
    WINDOWED.store(true, Ordering::Release);
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

/// Resizes the window so the client becomes cw x ch, keeping the top
/// left corner inside the visible desktop.
pub unsafe fn set_client_size(hwnd: HWND, cw: i32, ch: i32) {
    let (ow, oh) = outer_size_for(hwnd, cw, ch);
    let mut outer = RECT::default();
    GetWindowRect(hwnd, &mut outer);
    let cx = GetSystemMetrics(SM_CXSCREEN);
    let cy = GetSystemMetrics(SM_CYSCREEN);
    let x = outer.left.clamp(0, (cx - ow).max(0));
    let y = outer.top.clamp(0, (cy - oh).max(0));
    SetWindowPos(hwnd, core::ptr::null_mut(), x, y, ow, oh, SWP_NOZORDER);
}

/// Restores the client area to the scaled target size. The game (or
/// storm) keeps sizing the window to an outer 640x480, which the window
/// frame eats into, so every WM_SIZE snaps the outer size back to the
/// wanted client plus frame. Returns true when a correction ran, false
/// when the client already matched or the window is minimized or gone.
pub unsafe fn correct_client_size(hwnd: HWND) -> bool {
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
    if now.wrapping_sub(last) < PAINT_INTERVAL_MS {
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
