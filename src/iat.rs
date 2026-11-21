//! Import Address Table walking and patching for loaded PE modules.
//! Used to redirect the game's imports (storm.dll SDraw functions and the
//! user32 cursor trio) into this DLL.

use core::ffi::c_void;

use windows_sys::Win32::Foundation::HMODULE;
use windows_sys::Win32::System::Memory::VirtualProtect;

const DOS_MAGIC: u16 = 0x5A4D;
const PE_MAGIC: u32 = 0x4550;
/// Index of the import table in the optional header data directories.
const DIR_IMPORT: usize = 1;
/// Size of one IMAGE_IMPORT_DESCRIPTOR.
const DESCRIPTOR_SIZE: usize = 20;
/// Thunk bit set when the entry imports by ordinal, not by name.
const ORDINAL_FLAG: u32 = 0x8000_0000;

/// One import of a module: owning DLL, function name and the IAT slot
/// currently holding its resolved address.
pub struct ImportEntry {
    pub dll: String,
    pub func: String,
    pub address: *mut c_void,
}

/// Reads unaligned little-endian values at `base + off`.
unsafe fn read_at<T: Copy>(base: *const u8, off: usize) -> T {
    (base.add(off) as *const T).read_unaligned()
}

/// Reads a NUL-terminated ASCII string at `base + off`.
unsafe fn read_str(base: *const u8, off: usize) -> String {
    let mut end = off;
    while *base.add(end) != 0 {
        end += 1;
    }
    let bytes = core::slice::from_raw_parts(base.add(off), end - off);
    String::from_utf8_lossy(bytes).into_owned()
}

fn eq_ignore_case(a: &str, b: &str) -> bool {
    a.bytes().map(|x| x.to_ascii_lowercase()).eq(b.bytes().map(|x| x.to_ascii_lowercase()))
}

/// Walks the import table of a loaded module. Returns None when the module
/// has no readable PE headers. The names come from the original
/// name table when present, so entries stay meaningful after the loader
/// has resolved the addresses.
pub unsafe fn imports(module: HMODULE) -> Option<Vec<ImportEntry>> {
    let base = module as *const u8;
    if read_at::<u16>(base, 0) != DOS_MAGIC {
        return None;
    }
    let pe = read_at::<i32>(base, 0x3C) as usize;
    if read_at::<u32>(base, pe) != PE_MAGIC {
        return None;
    }
    let optional = pe + 4 + 20;
    let import_rva = read_at::<u32>(base, optional + 96 + DIR_IMPORT * 8) as usize;
    let import_size = read_at::<u32>(base, optional + 96 + DIR_IMPORT * 8 + 4) as usize;
    if import_rva == 0 || import_size == 0 {
        return None;
    }
    let mut entries = Vec::new();
    let mut desc = import_rva;
    loop {
        let original = read_at::<u32>(base, desc);
        let name_rva = read_at::<u32>(base, desc + 12);
        let first_thunk = read_at::<u32>(base, desc + 16);
        if original == 0 && name_rva == 0 && first_thunk == 0 {
            break;
        }
        let dll = read_str(base, name_rva as usize);
        let names_thunk = if original != 0 { original } else { first_thunk };
        let mut thunk = first_thunk as usize;
        let mut name_thunk = names_thunk as usize;
        loop {
            let iat_value = read_at::<u32>(base, thunk);
            if iat_value == 0 {
                break;
            }
            let name_rva2 = read_at::<u32>(base, name_thunk);
            if name_rva2 & ORDINAL_FLAG == 0 {
                // skip the 2-byte import hint before the name
                let func = read_str(base, name_rva2 as usize + 2);
                entries.push(ImportEntry {
                    dll: dll.clone(),
                    func,
                    address: base.add(thunk) as *mut c_void,
                });
            }
            thunk += 4;
            name_thunk += 4;
        }
        desc += DESCRIPTOR_SIZE;
    }
    Some(entries)
}

/// Finds the IAT slot of one import. `dll` matches case-insensitively.
pub unsafe fn find_import(module: HMODULE, dll: &str, func: &str) -> Option<*mut c_void> {
    imports(module)?.into_iter().find_map(|e| {
        if eq_ignore_case(&e.dll, dll) && e.func == func {
            Some(e.address)
        } else {
            None
        }
    })
}

/// Replaces the IAT slot of one import with `replacement`, making the page
/// writable for the write and restoring its protection afterwards. Returns
/// the number of slots patched (0 or 1).
/// Not wired into the runtime path yet, the hook installation uses it when
/// the render and cursor hooks land. Unit-tested until then.
#[allow(dead_code)]
pub unsafe fn patch_import(
    module: HMODULE,
    dll: &str,
    func: &str,
    replacement: *const c_void,
) -> usize {
    let Some(slot) = find_import(module, dll, func) else {
        return 0;
    };
    let mut old = 0u32;
    VirtualProtect(slot.cast(), 4, 0x04, &mut old); // PAGE_READWRITE
    (slot as *mut u32).write_unaligned(replacement as u32);
    VirtualProtect(slot.cast(), 4, old, &mut old);
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::System::LibraryLoader::{
        GetModuleHandleExA, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
        GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    };

    fn own_module() -> HMODULE {
        let mut h = core::ptr::null_mut();
        let ok = unsafe {
            GetModuleHandleExA(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
                    | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                (own_module as fn() -> HMODULE) as usize as *const u8,
                &mut h,
            )
        };
        assert_eq!(ok, 1, "GetModuleHandleExA failed");
        h
    }

    #[test]
    fn own_module_has_imports_with_resolved_addresses() {
        let entries = unsafe { imports(own_module()) }.expect("own module imports");
        assert!(!entries.is_empty(), "import table should not be empty");
        for e in &entries {
            assert!(!e.dll.is_empty());
            assert!(!e.func.is_empty());
        }
    }

    #[test]
    fn find_import_agrees_with_table_walk() {
        let h = own_module();
        let entries = unsafe { imports(h) }.expect("own module imports");
        let sample = entries
            .iter()
            .find(|e| e.dll.to_ascii_lowercase().contains("kernel32"))
            .expect("test binary imports from kernel32");
        assert_eq!(unsafe { find_import(h, &sample.dll, &sample.func) }, Some(sample.address));
    }

    #[test]
    fn find_import_missing_returns_none() {
        assert_eq!(
            unsafe { find_import(own_module(), "does_not_exist.dll", "Nope") },
            None
        );
    }

    #[test]
    fn patch_import_rewrites_and_restores_slot() {
        extern "C" fn fake() {}
        let h = own_module();
        let entries = unsafe { imports(h) }.expect("own module imports");
        let sample = entries
            .iter()
            .find(|e| e.dll.to_ascii_lowercase().contains("kernel32"))
            .expect("test binary imports from kernel32");
        let original = unsafe { (sample.address as *const u32).read_unaligned() };
        let patched = unsafe { patch_import(h, &sample.dll, &sample.func, fake as usize as _) };
        assert_eq!(patched, 1, "exactly one slot should be patched");
        let now = unsafe { (sample.address as *const u32).read_unaligned() };
        assert_eq!(now, fake as usize as u32);
        unsafe { patch_import(h, &sample.dll, &sample.func, original as _) };
        let restored = unsafe { (sample.address as *const u32).read_unaligned() };
        assert_eq!(restored, original);
    }
}
