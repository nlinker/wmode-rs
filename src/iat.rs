//! Import Address Table walking and patching for loaded PE modules.

use core::ffi::c_void;

use windows_sys::Win32::Foundation::HMODULE;
use windows_sys::Win32::System::Memory::VirtualProtect;

const DOS_MAGIC: u16 = 0x5A4D;
const PE_MAGIC: u32 = 0x4550;
/// Data directory index of the import table.
const DIR_IMPORT: usize = 1;
/// Size of one IMAGE_IMPORT_DESCRIPTOR.
const DESCRIPTOR_SIZE: usize = 20;
/// Thunk bit set for imports by ordinal.
const ORDINAL_FLAG: u32 = 0x8000_0000;

/// One import: owning DLL, function name or import ordinal, IAT slot.
/// storm.dll exports by ordinal only, so ordinal imports are the norm
/// there and `func` stays empty for them.
pub struct ImportEntry {
    pub dll: String,
    pub func: String,
    pub ordinal: Option<u16>,
    pub address: *mut c_void,
}

unsafe fn read_at<T: Copy>(base: *const u8, off: usize) -> T {
    (base.add(off) as *const T).read_unaligned()
}

unsafe fn read_str(base: *const u8, off: usize) -> String {
    let mut end = off;
    while *base.add(end) != 0 {
        end += 1;
    }
    String::from_utf8_lossy(core::slice::from_raw_parts(base.add(off), end - off)).into_owned()
}

fn eq_ignore_case(a: &str, b: &str) -> bool {
    a.bytes().map(|x| x.to_ascii_lowercase()).eq(b.bytes().map(|x| x.to_ascii_lowercase()))
}

/// Walks the import table. Names come from the original name table when
/// present, so entries stay meaningful after loader resolution.
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
    // PE32+ optional headers are 16 bytes longer before the data
    // directories than PE32 ones
    let data_dir = match read_at::<u16>(base, optional) {
        0x20B => optional + 112,
        _ => optional + 96,
    };
    let import_rva = read_at::<u32>(base, data_dir + DIR_IMPORT * 8) as usize;
    let import_size = read_at::<u32>(base, data_dir + DIR_IMPORT * 8 + 4) as usize;
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
            if name_rva2 & ORDINAL_FLAG != 0 {
                entries.push(ImportEntry {
                    dll: dll.clone(),
                    func: String::new(),
                    ordinal: Some((name_rva2 & 0xFFFF) as u16),
                    address: base.add(thunk) as *mut c_void,
                });
            } else {
                // name follows a 2-byte hint
                entries.push(ImportEntry {
                    dll: dll.clone(),
                    func: read_str(base, name_rva2 as usize + 2),
                    ordinal: None,
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

/// Finds the IAT slot of one named import. `dll` matches case-insensitively.
pub unsafe fn find_import(module: HMODULE, dll: &str, func: &str) -> Option<*mut c_void> {
    imports(module)?.into_iter().find_map(|e| {
        if eq_ignore_case(&e.dll, dll) && e.func == func {
            Some(e.address)
        } else {
            None
        }
    })
}

/// Finds the IAT slot of one ordinal import.
pub unsafe fn find_import_ordinal(
    module: HMODULE,
    dll: &str,
    ordinal: u16,
) -> Option<*mut c_void> {
    imports(module)?.into_iter().find_map(|e| {
        if eq_ignore_case(&e.dll, dll) && e.ordinal == Some(ordinal) {
            Some(e.address)
        } else {
            None
        }
    })
}

/// Writes `replacement` into a slot, returns the previous slot value.
unsafe fn rewrite_slot(slot: *mut c_void, replacement: *const c_void) -> u32 {
    let previous = (slot as *const u32).read_unaligned();
    let mut old = 0u32;
    VirtualProtect(slot.cast(), 4, 0x04, &mut old); // PAGE_READWRITE
    (slot as *mut u32).write_unaligned(replacement as u32);
    VirtualProtect(slot.cast(), 4, old, &mut old);
    previous
}

/// Replaces one named IAT slot with `replacement`. Returns the previous
/// slot value, the saved original to forward to, or None when the import
/// does not exist. Runtime wiring happens with the render and cursor hooks.
#[allow(dead_code)]
pub unsafe fn patch_import(
    module: HMODULE,
    dll: &str,
    func: &str,
    replacement: *const c_void,
) -> Option<u32> {
    find_import(module, dll, func).map(|slot| rewrite_slot(slot, replacement))
}

/// Replaces one ordinal IAT slot with `replacement`. Returns the previous
/// slot value, or None when the import does not exist.
#[allow(dead_code)]
pub unsafe fn patch_import_ordinal(
    module: HMODULE,
    dll: &str,
    ordinal: u16,
    replacement: *const c_void,
) -> Option<u32> {
    find_import_ordinal(module, dll, ordinal).map(|slot| rewrite_slot(slot, replacement))
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
        assert!(!entries.is_empty());
        for e in &entries {
            assert!(!e.dll.is_empty());
            // name or ordinal, exactly one
            assert!(e.func.is_empty() != e.ordinal.is_none());
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
        assert_eq!(unsafe { find_import(own_module(), "does_not_exist.dll", "Nope") }, None);
    }

    /// Minimal synthetic PE image with one named import, laid out so every
    /// RVA equals its buffer offset. Patching it cannot disturb the test
    /// process, unlike patching our own live IAT.
    fn synthetic_module() -> Vec<u8> {
        const PE_OFF: usize = 0x40;
        const OPTIONAL_OFF: usize = PE_OFF + 4 + 20;
        const IMPORT_OFF: usize = 0x100;
        const INT_OFF: usize = 0x140;
        const IAT_OFF: usize = 0x148;
        const BY_NAME_OFF: usize = 0x160;
        const DLLNAME_OFF: usize = 0x180;

        let mut buf = vec![0u8; 0x200];
        let put16 = |b: &mut Vec<u8>, off: usize, v: u16| {
            b[off..off + 2].copy_from_slice(&v.to_le_bytes())
        };
        let put32 = |b: &mut Vec<u8>, off: usize, v: u32| {
            b[off..off + 4].copy_from_slice(&v.to_le_bytes())
        };

        put16(&mut buf, 0, DOS_MAGIC);
        put32(&mut buf, 0x3C, PE_OFF as u32);
        put32(&mut buf, PE_OFF, PE_MAGIC);
        put16(&mut buf, PE_OFF + 4, 0x14C); // machine i386
        put16(&mut buf, OPTIONAL_OFF, 0x10B); // PE32 optional header
        put32(&mut buf, OPTIONAL_OFF + 96 + DIR_IMPORT * 8, IMPORT_OFF as u32);
        put32(&mut buf, OPTIONAL_OFF + 96 + DIR_IMPORT * 8 + 4, DESCRIPTOR_SIZE as u32 + 20);

        // descriptor: OriginalFirstThunk, TimeDateStamp, ForwarderChain, Name, FirstThunk
        put32(&mut buf, IMPORT_OFF, INT_OFF as u32);
        put32(&mut buf, IMPORT_OFF + 12, DLLNAME_OFF as u32);
        put32(&mut buf, IMPORT_OFF + 16, IAT_OFF as u32);
        // the all-zero terminator descriptor follows implicitly

        put32(&mut buf, INT_OFF, BY_NAME_OFF as u32);
        put32(&mut buf, INT_OFF + 4, 0x8000_015E); // ordinal import 350
        put32(&mut buf, IAT_OFF, 0x1234_5678); // fake resolved address
        put32(&mut buf, IAT_OFF + 4, 0xDEAD_BEEF);

        let by_name = b"\x00\x00GetModuleHandleA\0";
        buf[BY_NAME_OFF..BY_NAME_OFF + by_name.len()].copy_from_slice(by_name);
        let dll = b"kernel32.dll\0";
        buf[DLLNAME_OFF..DLLNAME_OFF + dll.len()].copy_from_slice(dll);
        buf
    }

    #[test]
    fn patch_import_rewrites_and_restores_slot() {
        extern "C" fn fake() {}
        let buf = synthetic_module();
        let base = buf.as_ptr();
        let slot = unsafe { find_import(base as HMODULE, "kernel32.dll", "GetModuleHandleA") }
            .expect("synthetic import present");
        assert_eq!(slot as usize, base as usize + 0x148);
        assert_eq!(unsafe { (slot as *const u32).read_unaligned() }, 0x1234_5678);

        assert_eq!(
            unsafe { patch_import(base as HMODULE, "kernel32.dll", "GetModuleHandleA", fake as *const () as usize as _) },
            Some(0x1234_5678)
        );
        assert_eq!(unsafe { (slot as *const u32).read_unaligned() }, fake as *const () as usize as u32);

        assert_eq!(
            unsafe {
                patch_import(
                    base as HMODULE,
                    "kernel32.dll",
                    "GetModuleHandleA",
                    0x1234_5678usize as _,
                )
            },
            Some(fake as *const () as usize as u32)
        );
        assert_eq!(unsafe { (slot as *const u32).read_unaligned() }, 0x1234_5678);
    }

    #[test]
    fn patch_import_ordinal_rewrites_slot() {
        extern "C" fn fake() {}
        let buf = synthetic_module();
        let base = buf.as_ptr();
        let slot = unsafe { find_import_ordinal(base as HMODULE, "kernel32.dll", 350) }
            .expect("synthetic ordinal import present");
        assert_eq!(unsafe { (slot as *const u32).read_unaligned() }, 0xDEAD_BEEF);
        assert_eq!(
            unsafe { patch_import_ordinal(base as HMODULE, "kernel32.dll", 350, fake as *const () as usize as _) },
            Some(0xDEAD_BEEF)
        );
        assert_eq!(unsafe { (slot as *const u32).read_unaligned() }, fake as *const () as usize as u32);
    }
}
