//! SDraw hooks installed into the game's import table. Each hook handles
//! the call while windowed mode is active and forwards to the saved
//! original otherwise.

use std::sync::atomic::{AtomicU32, Ordering};

use windows_sys::core::BOOL;
use windows_sys::Win32::Foundation::{FALSE, RECT, TRUE};
use windows_sys::Win32::Graphics::Gdi::PALETTEENTRY;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleA;

use crate::iat;
use crate::log::log;
use crate::render;

/// storm.dll export ordinals (nameless exports, base 101).
const ORD_SDRAW_LOCK: u16 = 350;
const ORD_SDRAW_UNLOCK: u16 = 356;
const ORD_SDRAW_UPDATE_PALETTE: u16 = 357;

type LockFn =
    unsafe extern "system" fn(i32, *mut RECT, *mut *mut core::ffi::c_void, *mut i32, i32) -> BOOL;
type UnlockFn = unsafe extern "system" fn(i32, *mut core::ffi::c_void, i32, *mut RECT) -> BOOL;
type PaletteFn = unsafe extern "system" fn(u32, u32, *mut PALETTEENTRY, i32) -> BOOL;

static LOCK_OLD: AtomicU32 = AtomicU32::new(0);
static UNLOCK_OLD: AtomicU32 = AtomicU32::new(0);
static PALETTE_OLD: AtomicU32 = AtomicU32::new(0);

unsafe extern "system" fn sdraw_lock(
    surfacenumber: i32,
    dest: *mut RECT,
    surface: *mut *mut core::ffi::c_void,
    pitch: *mut i32,
    unused: i32,
) -> BOOL {
    // always served from our DIB: forwarding to storm would lazily
    // initialize real DirectDraw in fullscreen exclusive and flip the
    // desktop mode
    if render::lock(dest, surface, pitch) {
        return TRUE;
    }
    let old = LOCK_OLD.load(Ordering::Acquire);
    if old != 0 {
        let f: LockFn = core::mem::transmute(old as usize);
        f(surfacenumber, dest, surface, pitch, unused)
    } else {
        FALSE
    }
}

unsafe extern "system" fn sdraw_unlock(
    surfacenumber: i32,
    surface: *mut core::ffi::c_void,
    a3: i32,
    rect: *mut RECT,
) -> BOOL {
    if render::unlock() {
        return TRUE;
    }
    let old = UNLOCK_OLD.load(Ordering::Acquire);
    if old != 0 {
        let f: UnlockFn = core::mem::transmute(old as usize);
        f(surfacenumber, surface, a3, rect)
    } else {
        FALSE
    }
}

unsafe extern "system" fn sdraw_update_palette(
    firstentry: u32,
    numentries: u32,
    entries: *mut PALETTEENTRY,
    a4: i32,
) -> BOOL {
    if render::update_palette(firstentry, numentries, entries) {
        return TRUE;
    }
    let old = PALETTE_OLD.load(Ordering::Acquire);
    if old != 0 {
        let f: PaletteFn = core::mem::transmute(old as usize);
        f(firstentry, numentries, entries, a4)
    } else {
        TRUE
    }
}

/// Patches the three SDraw imports in the game executable plus the storm
/// GetProcAddress entry that resolves DirectDraw. Window conversion is not
/// part of this: it happens once the game window exists.
/// Returns false when the critical hooks (lock and unlock) did not
/// install, leaving the game running untouched.
pub fn install() -> bool {
    let exe = unsafe { GetModuleHandleA(core::ptr::null()) };
    let lock_fn: LockFn = sdraw_lock;
    let unlock_fn: UnlockFn = sdraw_unlock;
    let palette_fn: PaletteFn = sdraw_update_palette;

    let lock_saved = unsafe {
        iat::patch_import_ordinal(exe, "storm.dll", ORD_SDRAW_LOCK, lock_fn as usize as _)
    };
    let unlock_saved = unsafe {
        iat::patch_import_ordinal(exe, "storm.dll", ORD_SDRAW_UNLOCK, unlock_fn as usize as _)
    };
    let palette_saved = unsafe {
        iat::patch_import_ordinal(
            exe,
            "storm.dll",
            ORD_SDRAW_UPDATE_PALETTE,
            palette_fn as usize as _,
        )
    };

    match lock_saved {
        Some(old) => {
            LOCK_OLD.store(old, Ordering::Release);
            log(&format!("hooks: SDrawLockSurface patched, old {old:#010x}"));
        }
        None => log("hooks: SDrawLockSurface NOT PATCHED"),
    }
    match unlock_saved {
        Some(old) => {
            UNLOCK_OLD.store(old, Ordering::Release);
            log(&format!("hooks: SDrawUnlockSurface patched, old {old:#010x}"));
        }
        None => log("hooks: SDrawUnlockSurface NOT PATCHED"),
    }
    match palette_saved {
        Some(old) => {
            PALETTE_OLD.store(old, Ordering::Release);
            log(&format!("hooks: SDrawUpdatePalette patched, old {old:#010x}"));
        }
        None => log("hooks: SDrawUpdatePalette NOT PATCHED"),
    }

    if lock_saved.is_none() || unlock_saved.is_none() {
        return false;
    }
    crate::ddraw::install();
    crate::ddraw::replace_storm_ddraw_object(LOCK_OLD.load(Ordering::Acquire))
}
