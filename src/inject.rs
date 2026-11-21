//! Injection of this DLL into the suspended StarCraft process, using the
//! CreateRemoteThread(LoadLibraryA) pattern standard for BWL4 plugins.

use crate::log::log;
use windows_sys::core::PCSTR;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows_sys::Win32::System::LibraryLoader::{
    GetModuleHandleA, GetModuleHandleExA, GetModuleFileNameA, GetProcAddress,
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
};
use windows_sys::Win32::System::Memory::{
    VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
};
use windows_sys::Win32::System::Threading::{
    CreateRemoteThread, GetExitCodeThread, WaitForSingleObject, INFINITE, LPTHREAD_START_ROUTINE,
};

/// NUL-terminated byte string as a PCSTR.
fn pcstr(bytes: &[u8]) -> PCSTR {
    bytes.as_ptr()
}

/// Full path of this module as NUL-terminated bytes.
fn module_path() -> Option<Vec<u8>> {
    let mut hmodule = core::ptr::null_mut();
    let found = unsafe {
        GetModuleHandleExA(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            (module_path as fn() -> Option<Vec<u8>>) as usize as PCSTR,
            &mut hmodule,
        )
    };
    if found == 0 {
        return None;
    }
    let mut buf = [0u8; 260];
    let len = unsafe { GetModuleFileNameA(hmodule, buf.as_mut_ptr(), buf.len() as u32) };
    if len == 0 || len as usize >= buf.len() {
        return None;
    }
    let mut path = buf[..len as usize].to_vec();
    path.push(0);
    Some(path)
}

/// Thread-start shape LoadLibraryA is called through: one pointer param,
/// u32 return (a 32-bit HMODULE, nonzero on success).
type ThreadStartFn = unsafe extern "system" fn(param: *mut core::ffi::c_void) -> u32;

/// Resolves the local LoadLibraryA address. kernel32 is mapped at the
/// same address in every process of a boot session, so the local address
/// is valid inside the target process too.
fn local_load_library_a() -> Option<ThreadStartFn> {
    let k32 = unsafe { GetModuleHandleA(pcstr(b"kernel32.dll\0")) };
    if k32.is_null() {
        return None;
    }
    let proc = unsafe { GetProcAddress(k32, pcstr(b"LoadLibraryA\0")) }?;
    type FarProc = unsafe extern "system" fn() -> isize;
    Some(unsafe { core::mem::transmute::<FarProc, ThreadStartFn>(proc) })
}

/// Injects this DLL into `hprocess` (the suspended StarCraft.exe the
/// launcher hands over): allocates a page, writes the DLL path, runs
/// LoadLibraryA there via CreateRemoteThread and checks its return value.
/// Returns 1 on success, 0 on failure.
pub fn apply_patch_suspended(hprocess: HANDLE) -> i32 {
    let Some(path) = module_path() else {
        log("ApplyPatchSuspended: own module path not found");
        return 0;
    };
    let path_text = String::from_utf8_lossy(&path[..path.len() - 1]).into_owned();
    let Some(load_library_a) = local_load_library_a() else {
        log("ApplyPatchSuspended: LoadLibraryA not resolved");
        return 0;
    };
    log(&format!("ApplyPatchSuspended: injecting {path_text}"));
    unsafe {
        let remote = VirtualAllocEx(
            hprocess,
            core::ptr::null(),
            path.len(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );
        if remote.is_null() {
            return 0;
        }
        let cleanup = |ret: i32| {
            VirtualFreeEx(hprocess, remote, 0, MEM_RELEASE);
            ret
        };
        if WriteProcessMemory(
            hprocess,
            remote,
            path.as_ptr().cast(),
            path.len(),
            core::ptr::null_mut(),
        ) == 0
        {
            return cleanup(0);
        }
        let start: LPTHREAD_START_ROUTINE = Some(load_library_a);
        let thread = CreateRemoteThread(
            hprocess,
            core::ptr::null(),
            0,
            start,
            remote.cast(),
            0,
            core::ptr::null_mut(),
        );
        if thread.is_null() {
            return cleanup(0);
        }
        WaitForSingleObject(thread, INFINITE);
        let mut exit_code = 0u32;
        GetExitCodeThread(thread, &mut exit_code);
        CloseHandle(thread);
        log(&format!(
            "ApplyPatchSuspended: LoadLibraryA exit code {exit_code} ({})",
            if exit_code == 0 { "FAILED" } else { "loaded" }
        ));
        cleanup(i32::from(exit_code != 0))
    }
}
