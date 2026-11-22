//! Fake DirectDraw handed to storm.dll. Storm resolves ddraw.dll at runtime
//! through GetProcAddress, so the GetProcAddress hook in storm's import
//! table substitutes our DirectDrawCreate. The fake object accepts mode
//! switches and surface creation without ever touching the display mode:
//! the desktop resolution stays intact. Real rendering goes through the
//! SDraw hooks instead.

use std::sync::atomic::{AtomicU32, Ordering};

use windows_sys::core::PCSTR;
use windows_sys::Win32::Foundation::HMODULE;
use windows_sys::Win32::System::LibraryLoader::{
    GetModuleFileNameA, GetModuleHandleA, GetModuleHandleExA,
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
};
use windows_sys::Win32::System::Memory::VirtualProtect;

use crate::iat;
use crate::log::log;
use crate::render;

const E_NOINTERFACE: i32 = 0x8000_4002u32 as i32;
const DDERR_INVALIDPARAMS: i32 = 0x8000_4003u32 as i32;

/// Our stand-in for the DirectDrawCreate export.
unsafe extern "system" fn direct_draw_create(
    _guid: *const core::ffi::c_void,
    out_dd: *mut *mut core::ffi::c_void,
    _outer: *mut core::ffi::c_void,
) -> i32 {
    log("ddraw: DirectDrawCreate intercepted");
    if out_dd.is_null() {
        return DDERR_INVALIDPARAMS;
    }
    *out_dd = fake_dd_ptr();
    0
}

unsafe extern "system" fn dd_query_interface(
    _s: usize,
    _iid: *const core::ffi::c_void,
    out: *mut *mut core::ffi::c_void,
) -> i32 {
    // stay on the base IDirectDraw interface, refuse everything newer
    if !out.is_null() {
        *out = core::ptr::null_mut();
    }
    E_NOINTERFACE
}

unsafe extern "system" fn dd_add_ref(_s: usize) -> i32 {
    1
}

unsafe extern "system" fn dd_release(_s: usize) -> i32 {
    1
}

unsafe extern "system" fn dd_create_surface(
    _s: usize,
    _desc: *mut core::ffi::c_void,
    out_surface: *mut *mut core::ffi::c_void,
    _unk: *mut core::ffi::c_void,
) -> i32 {
    if out_surface.is_null() {
        return DDERR_INVALIDPARAMS;
    }
    *out_surface = fake_surface_ptr();
    0
}

/// Stub generators, one per argument count: stdcall lets the callee pop
/// its arguments, so every vtable slot needs the matching arity.
macro_rules! stubs {
    ($($name:ident ($($arg:ident)*);)*) => {
        $(unsafe extern "system" fn $name(_s: usize $(, $arg: usize)*) -> i32 {
            $(let _ = $arg;)*
            0
        })*
    };
}
stubs! {
    dd0 ();
    dd2 (_a _b);
    dd3 (_a _b _c);
    dd4 (_a _b _c _d);
    dd5 (_a _b _c _d _e);
}

#[repr(C)]
struct FakeDd {
    vtbl: *const usize,
}

unsafe impl Send for FakeDd {}
unsafe impl Sync for FakeDd {}

#[repr(C)]
struct FakeSurface {
    vtbl: *const usize,
}

unsafe impl Send for FakeSurface {}
unsafe impl Sync for FakeSurface {}

/// IDirectDrawSurface::Lock: hands out our DIB bits through the surface
/// descriptor, matching what the SDraw lock hook does.
unsafe extern "system" fn surface_lock(
    _s: usize,
    _rect: *mut core::ffi::c_void,
    desc: *mut core::ffi::c_void,
    _flags: usize,
    _event: usize,
) -> i32 {
    if desc.is_null() {
        return DDERR_INVALIDPARAMS;
    }
    render::lock_surface_desc(desc);
    0
}

stubs! {
    surf0 ();
    surf1 (_a);
    surf2 (_a _b);
    surf3 (_a _b _c);
    surf4 (_a _b _c _d);
}

/// IDirectDraw vtable, base interface, slot order per ddraw.h.
/// Built lazily: fn-to-int casts are not const-evaluable.
fn build_dd_vtable() -> [usize; 23] {
    [
    dd_query_interface as *const () as usize,
    dd_add_ref as *const () as usize,
    dd_release as *const () as usize,
    dd0 as *const () as usize, // Compact
    dd3 as *const () as usize, // CreateClipper(self, flags, out)
    dd4 as *const () as usize, // CreatePalette(self, flags, entries, out, unk)
    dd_create_surface as *const () as usize,
    dd3 as *const () as usize, // DuplicateSurface(self, desc, out)
    dd4 as *const () as usize, // EnumDisplayModes(self, flags, desc, ctx)
    dd4 as *const () as usize, // EnumSurfaces
    dd0 as *const () as usize, // FlipToGDISurface
    dd3 as *const () as usize, // GetCaps(self, driverCaps, helCaps)
    dd3 as *const () as usize, // GetDisplayMode(self, flags, desc)
    dd3 as *const () as usize, // GetFourCCCodes(self, count, codes)
    dd2 as *const () as usize, // GetGDISurface(self, out)
    dd2 as *const () as usize, // GetMonitorFrequency(self, out)
    dd2 as *const () as usize, // GetScanLine(self, out)
    dd2 as *const () as usize, // GetVerticalBlankStatus(self, out)
    dd2 as *const () as usize, // Initialize(self, guid)
    dd0 as *const () as usize, // RestoreDisplayMode
    dd3 as *const () as usize, // SetCooperativeLevel(self, hwnd, flags)
    dd5 as *const () as usize, // SetDisplayMode(self, w, h, bpp, refresh, extra)
    dd2 as *const () as usize, // WaitForVerticalBlank(self, flags, event)
    ]
}

/// IDirectDrawSurface vtable, slot order per ddraw.h.
fn build_surface_vtable() -> [usize; 35] {
    [
    dd_query_interface as *const () as usize,
    dd_add_ref as *const () as usize,
    dd_release as *const () as usize,
    surf1 as *const () as usize, // AddAttachedSurface(self, surface)
    surf4 as *const () as usize, // Blt(self, dstRect, src, srcRect, flags, fx)
    surf4 as *const () as usize, // BltBatch(self, batch, count, flags)
    surf4 as *const () as usize, // BltFast(self, x, y, src, srcRect, flags)
    surf2 as *const () as usize, // DeleteAttachedSurface(self, flags, surface)
    surf2 as *const () as usize, // EnumAttachedSurfaces(self, ctx, callback)
    surf3 as *const () as usize, // EnumOverlayZOrders(self, front, ctx, callback)
    surf2 as *const () as usize, // Flip(self, surface, flags)
    surf2 as *const () as usize, // GetAttachedSurface(self, caps, out)
    surf1 as *const () as usize, // GetBltStatus(self, flags)
    surf1 as *const () as usize, // GetCaps(self, caps)
    surf1 as *const () as usize, // GetClipper(self, out)
    surf2 as *const () as usize, // GetColorKey(self, flags, out)
    surf1 as *const () as usize, // GetDC(self, out)
    surf1 as *const () as usize, // GetFlipStatus(self, flags)
    surf2 as *const () as usize, // GetOverlayPosition(self, x, y)
    surf1 as *const () as usize, // GetPalette(self, out)
    surf1 as *const () as usize, // GetPixelFormat(self, out)
    surf1 as *const () as usize, // GetSurfaceDesc(self, out)
    surf2 as *const () as usize, // Initialize(self, dd, desc)
    surf0 as *const () as usize, // IsLost
    surface_lock as *const () as usize,
    surf1 as *const () as usize, // ReleaseDC(self, hdc)
    surf0 as *const () as usize, // Restore
    surf1 as *const () as usize, // SetClipper(self, clipper)
    surf2 as *const () as usize, // SetColorKey(self, flags, key)
    surf2 as *const () as usize, // SetOverlayPosition(self, x, y)
    surf1 as *const () as usize, // SetPalette(self, palette)
    surf1 as *const () as usize, // Unlock(self, rect)
    surf3 as *const () as usize, // UpdateOverlay(self, src, dst, flags)
    surf1 as *const () as usize, // UpdateOverlayDisplay(self, flags)
    surf2 as *const () as usize, // UpdateOverlayZOrder(self, front, relative)
    ]
}

static DD_VTABLE: std::sync::OnceLock<[usize; 23]> = std::sync::OnceLock::new();
static SURFACE_VTABLE: std::sync::OnceLock<[usize; 35]> = std::sync::OnceLock::new();
static FAKE_DD: std::sync::OnceLock<FakeDd> = std::sync::OnceLock::new();
static FAKE_SURFACE: std::sync::OnceLock<FakeSurface> = std::sync::OnceLock::new();

pub fn fake_dd_ptr() -> *mut core::ffi::c_void {
    let dd = FAKE_DD.get_or_init(|| FakeDd {
        vtbl: DD_VTABLE.get_or_init(build_dd_vtable).as_ptr(),
    });
    dd as *const FakeDd as *mut core::ffi::c_void
}

fn fake_surface_ptr() -> *mut core::ffi::c_void {
    let surface = FAKE_SURFACE.get_or_init(|| FakeSurface {
        vtbl: SURFACE_VTABLE.get_or_init(build_surface_vtable).as_ptr(),
    });
    surface as *const FakeSurface as *mut core::ffi::c_void
}

static GETPROC_OLD: AtomicU32 = AtomicU32::new(0);

type GetProcFn = unsafe extern "system" fn(HMODULE, PCSTR) -> *mut core::ffi::c_void;

/// Diagnostic budget for the GetProcAddress hook.
static GETPROC_TRACE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(40);

/// Reads a NUL-terminated C string as Rust text.
unsafe fn cstr(name: PCSTR) -> String {
    let bytes = name as *const u8;
    let mut n = 0usize;
    while *bytes.add(n) != 0 {
        n += 1;
    }
    String::from_utf8_lossy(core::slice::from_raw_parts(bytes, n)).into_owned()
}

unsafe fn call_real_get_proc(
    real: u32,
    module: HMODULE,
    name: PCSTR,
) -> *mut core::ffi::c_void {
    let f: GetProcFn = core::mem::transmute(real as usize);
    f(module, name)
}

/// The GetProcAddress replacement in storm.dll's import table.
unsafe extern "system" fn get_proc_address(module: HMODULE, name: PCSTR) -> *mut core::ffi::c_void {
    let real = GETPROC_OLD.load(Ordering::Acquire);
    if real == 0 {
        return core::ptr::null_mut();
    }
    if name.is_null() || name as usize <= 0xFFFF {
        return call_real_get_proc(real, module, name);
    }
    {
        let asked = cstr(name);
        let trace = GETPROC_TRACE.fetch_sub(1, Ordering::Relaxed);
        if trace > 0 {
            log(&format!("ddraw: storm asks GetProcAddress({asked})"));
        }
    }
    let proc = call_real_get_proc(real, module, name);
    let asked = cstr(name);
    if asked != "DirectDrawCreate" {
        return proc;
    }
    let mut buf = [0u8; 260];
    let len = GetModuleFileNameA(module, buf.as_mut_ptr(), buf.len() as u32);
    let path = String::from_utf8_lossy(&buf[..len as usize]).to_ascii_lowercase();
    if !path.ends_with("ddraw.dll") {
        return proc;
    }
    log("ddraw: GetProcAddress(DirectDrawCreate) intercepted");
    {
        let create: unsafe extern "system" fn(
            *const core::ffi::c_void,
            *mut *mut core::ffi::c_void,
            *mut core::ffi::c_void,
        ) -> i32 = direct_draw_create;
        create as usize as *mut core::ffi::c_void
    }
}

/// Replaces the storm-cached DirectDraw object pointer with the fake one.
/// storm resolves ddraw in its own DllMain, before any hook can run, so the
/// only reliable interception point is the global it stores the object in.
pub fn replace_storm_ddraw_object(storm_fn_addr: u32) -> bool {
    let mut storm = core::ptr::null_mut();
    let from_addr = unsafe {
        GetModuleHandleExA(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            storm_fn_addr as usize as _,
            &mut storm,
        )
    };
    if from_addr == 0 || storm.is_null() {
        log("ddraw: storm module not resolvable from hook address");
        return false;
    }
    let slot = (storm as usize + 0x5E664) as *mut *mut core::ffi::c_void;
    unsafe {
        let previous = slot.read_unaligned();
        let mut old = 0u32;
        VirtualProtect(slot.cast(), 4, 0x04, &mut old); // PAGE_READWRITE
        slot.write_unaligned(fake_dd_ptr());
        VirtualProtect(slot.cast(), 4, old, &mut old);
        log(&format!(
            "ddraw: storm DirectDraw object slot {:#010x} replaced (was {:#010x})",
            slot as usize,
            previous as usize
        ));
    }
    true
}

/// The LoadLibraryA replacement in storm.dll's import table. Every library
/// storm loads gets its own GetProcAddress import redirected to the same
/// wrapper, so runtime DirectDraw resolutions inside those libraries
/// (smackw32.dll plays the intro movies through DirectDraw, the source of
/// the fullscreen mode flips) receive the fake object too.
unsafe extern "system" fn load_library_a(file_name: PCSTR) -> HMODULE {
    type LoadFn = unsafe extern "system" fn(PCSTR) -> HMODULE;
    let real = LOADLIB_OLD.load(Ordering::Acquire);
    if real == 0 {
        return core::ptr::null_mut();
    }
    let asked = if file_name.is_null() {
        String::new()
    } else {
        cstr(file_name)
    };
    let f: LoadFn = core::mem::transmute(real as usize);
    let handle = f(file_name);
    if !handle.is_null() && !asked.is_empty() {
        log(&format!("ddraw: storm LoadLibraryA({asked})"));
        hook_child_resolution(handle, &asked);
    }
    handle
}

/// Patches GetProcAddress in a freshly loaded library so its own runtime
/// DirectDraw resolution goes through the fake.
unsafe fn hook_child_resolution(module: HMODULE, name: &str) {
    let g: GetProcFn = get_proc_address;
    let g_addr = g as usize;
    match iat::patch_import(module, "kernel32.dll", "GetProcAddress", g_addr as _) {
        Some(_) => log(&format!("ddraw: child {name} GetProcAddress hooked")),
        None => log(&format!("ddraw: child {name} has no GetProcAddress import")),
    }
}

static LOADLIB_OLD: AtomicU32 = AtomicU32::new(0);

/// Patches GetProcAddress in storm.dll's import table so the fake
/// DirectDraw is handed to the storm renderer.
pub fn install() -> bool {
    let storm = unsafe { GetModuleHandleA(b"storm.dll\0".as_ptr()) };
    if storm.is_null() {
        log("ddraw: storm.dll not loaded yet");
        return false;
    }
    let f: GetProcFn = get_proc_address;
    let f_addr = f as usize;
    let saved =
        unsafe { iat::patch_import(storm, "kernel32.dll", "GetProcAddress", f_addr as _) };
    match saved {
        Some(old) => {
            GETPROC_OLD.store(old, Ordering::Release);
            log(&format!("ddraw: storm GetProcAddress hooked, old {old:#010x}"));
        }
        None => {
            log("ddraw: storm GetProcAddress import NOT FOUND");
        }
    }
    // the game executable itself also resolves ddraw at runtime through
    // its own kernel32 imports, cover it with the same wrappers
    let exe = unsafe { GetModuleHandleA(core::ptr::null()) };
    if !exe.is_null() {
        let g_addr0: GetProcFn = get_proc_address;
        unsafe {
            iat::patch_import(exe, "kernel32.dll", "GetProcAddress", g_addr0 as usize as _);
        }
        let l_addr0: unsafe extern "system" fn(PCSTR) -> HMODULE = load_library_a;
        unsafe {
            iat::patch_import(exe, "kernel32.dll", "LoadLibraryA", l_addr0 as usize as _);
        }
        log("ddraw: game executable kernel32 imports hooked");
    }

    let g: unsafe extern "system" fn(PCSTR) -> HMODULE = load_library_a;
    let g_addr = g as usize;
    match unsafe { iat::patch_import(storm, "kernel32.dll", "LoadLibraryA", g_addr as _) } {
        Some(old) => {
            LOADLIB_OLD.store(old, Ordering::Release);
            log(&format!("ddraw: storm LoadLibraryA hooked, old {old:#010x}"));
            true
        }
        None => {
            log("ddraw: storm LoadLibraryA import NOT FOUND");
            false
        }
    }
}

// --- display-mode guard -----------------------------------------------------
// Whatever path resolves DirectDraw (storm may cache it in its own module
// init, before any of our hooks), the actual mode flip ends in ddraw.dll
// calling ChangeDisplaySettingsEx. Once draw.dll is loaded we patch that
// import: the request is logged and answered with success, so the desktop
// mode never changes.

static MODE_GUARD_DONE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static CHANGEEX_OLD: AtomicU32 = AtomicU32::new(0);

/// Reads the interesting fields of a DEVMODE request. DEVMODEW is larger
/// than DEVMODEA and shifts the graphics fields by 32 bytes, dmSize tells
/// them apart (124 vs 188).
unsafe fn devmode_summary(devmode: *const core::ffi::c_void) -> String {
    if devmode.is_null() {
        return "null".to_owned();
    }
    let base = devmode as *const u8;
    let size = (base.add(36) as *const u16).read_unaligned();
    let (bpp_off, w_off, h_off) = if size >= 160 { (134, 136, 140) } else { (102, 104, 108) };
    let bpp = (base.add(bpp_off) as *const u32).read_unaligned();
    let w = (base.add(w_off) as *const u32).read_unaligned();
    let h = (base.add(h_off) as *const u32).read_unaligned();
    format!("{w}x{h}x{bpp} (dmSize {size})")
}

unsafe extern "system" fn change_display_settings_ex(
    device: PCSTR,
    devmode: *const core::ffi::c_void,
    hwnd: windows_sys::Win32::Foundation::HWND,
    flags: u32,
    param: isize,
) -> i32 {
    let requested = unsafe { devmode_summary(devmode) };
    log(&format!(
        "ddraw: ChangeDisplaySettingsEx {requested} blocked (flags {flags:#x}, device null={})",
        device.is_null()
    ));
    let _ = (hwnd, param);
    0 // DISP_CHANGE_SUCCESSFUL, without actually switching
}

unsafe extern "system" fn change_display_settings(
    devmode: *const core::ffi::c_void,
    flags: u32,
) -> i32 {
    let requested = unsafe { devmode_summary(devmode) };
    log(&format!("ddraw: ChangeDisplaySettings {requested} blocked (flags {flags:#x})"));
    0
}

/// Patches ChangeDisplaySettingsExA inside ddraw.dll once it is loaded.
/// Returns true when the guard is in place.
pub fn install_mode_guard() -> bool {
    if MODE_GUARD_DONE.load(Ordering::Acquire) {
        return true;
    }
    let ddraw = unsafe { GetModuleHandleA(b"ddraw.dll\0".as_ptr()) };
    if ddraw.is_null() {
        return false;
    }
    let ex_fn: unsafe extern "system" fn(
        PCSTR,
        *const core::ffi::c_void,
        windows_sys::Win32::Foundation::HWND,
        u32,
        isize,
    ) -> i32 = change_display_settings_ex;
    let plain_fn: unsafe extern "system" fn(*const core::ffi::c_void, u32) -> i32 =
        change_display_settings;
    let mut guarded = 0;
    for (name, addr) in [
        ("ChangeDisplaySettingsExW", ex_fn as *const () as usize),
        ("ChangeDisplaySettingsExA", ex_fn as *const () as usize),
        ("ChangeDisplaySettingsW", plain_fn as *const () as usize),
        ("ChangeDisplaySettingsA", plain_fn as *const () as usize),
    ] {
        let saved = unsafe { iat::patch_import(ddraw, "user32.dll", name, addr as _) };
        match saved {
            Some(old) => {
                CHANGEEX_OLD.store(old, Ordering::Release);
                guarded += 1;
            }
            None => log(&format!("ddraw: {name} import not present in ddraw.dll")),
        }
    }
    MODE_GUARD_DONE.store(true, Ordering::Release);
    log(&format!("ddraw: mode guard active, {guarded} display-change imports blocked"));
    true
}
