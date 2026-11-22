//! Sacrificial host that loads the packed wmode.bwl and captures the
//! payload. The stub chain is: layer 1 unpacks the XMOD loader, the loader
//! unpacks the plugin and drops xmod2bwl.dll, then everything may crash
//! the host. The watcher thread therefore grabs the payload the moment it
//! appears, as a file copy or as a memory dump, and exits immediately.

use std::env;
use std::path::PathBuf;

use windows_sys::Win32::Foundation::HMODULE;
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, LoadLibraryA};
use windows_sys::Win32::System::SystemInformation::GetTickCount;
use windows_sys::Win32::System::Threading::Sleep;

const POLL_MS: u32 = 2;
/// Give the unpack chain this long before giving up.
const CHAIN_TIMEOUT_MS: u32 = 15_000;
const LAYER3_MODULE: &[u8] = b"xmod2bwl.dll\0";

fn pcstr(bytes: &[u8]) -> *const u8 {
    bytes.as_ptr()
}

unsafe fn read_u32(base: *const u8, off: usize) -> u32 {
    (base.add(off) as *const u32).read_unaligned()
}

unsafe fn write_u32(base: *mut u8, off: usize, value: u32) {
    (base.add(off) as *mut u32).write_unaligned(value)
}

/// Dump of a loaded module rebuilt so file layout equals virtual layout.
unsafe fn dump_module(module: HMODULE, dst: &std::path::Path) -> Option<usize> {
    let base = module as *const u8;
    let pe = read_u32(base, 0x3C) as usize;
    if read_u16(base, pe) != 0x4550 {
        return None;
    }
    let image_size = read_u32(base, pe + 0x18 + 56) as usize;
    let mut out = std::slice::from_raw_parts(base, image_size).to_vec();
    // FileAlignment = SectionAlignment, raw pointers = virtual addresses,
    // raw sizes = virtual sizes: the disk image equals the memory image
    let opt = pe + 0x18;
    let n_sections = read_u16(base, pe + 6) as usize;
    let opt_size = read_u16(base, pe + 0x14) as usize;
    let section_align = read_u32(base, opt + 32);
    write_u32(out.as_mut_ptr(), opt + 36, section_align);
    for s in 0..n_sections {
        let sh = opt + opt_size + s * 40;
        let vsize = read_u32(base, sh + 8) as usize;
        let va = read_u32(base, sh + 12) as usize;
        if out.len() < va + vsize {
            out.resize(va + vsize, 0);
        }
        write_u32(out.as_mut_ptr(), sh + 16, vsize as u32);
        write_u32(out.as_mut_ptr(), sh + 20, va as u32);
    }
    std::fs::write(dst, &out).ok()?;
    Some(out.len())
}

unsafe fn read_u16(base: *const u8, off: usize) -> u16 {
    (base.add(off) as *const u16).read_unaligned()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--dummy") {
        loop {
            unsafe { Sleep(1000); }
        }
    }
    let src: PathBuf = PathBuf::from(
        args.get(1)
            .cloned()
            .unwrap_or_else(|| "original\\wmode.bwl".to_owned()),
    );
    let out_dir: PathBuf = PathBuf::from(
        args.get(2)
            .cloned()
            .unwrap_or_else(|| "gproj".to_owned()),
    );
    std::fs::create_dir_all(&out_dir).expect("create out dir");

    let file = std::fs::read(&src).unwrap_or_else(|e| panic!("read {src:?}: {e}"));
    let load_copy = std::env::temp_dir().join("wmode-unpack-host.dll");
    std::fs::write(&load_copy, &file).unwrap_or_else(|e| panic!("write temp copy: {e}"));
    let load_path = load_copy.to_str().expect("ascii temp path").to_owned();

    let watcher = std::thread::spawn(move || {
        let started = unsafe { GetTickCount() };
        loop {
            unsafe {
                // payload as a loaded module
                let layer3: HMODULE = GetModuleHandleA(pcstr(LAYER3_MODULE));
                if !layer3.is_null() {
                    let dst = out_dir.join("wmode-layer3.dll");
                    if let Some(n) = dump_module(layer3, &dst) {
                        eprintln!("layer3 module dumped: {n} bytes -> {}", dst.display());
                        std::process::exit(0);
                    }
                }
                // payload as a dropped file
                let dropped = std::env::temp_dir().join("xmod2bwl.dll");
                if dropped.exists() {
                    if let Ok(bytes) = std::fs::read(&dropped) {
                        let dst = out_dir.join("xmod2bwl-file.dll");
                        let _ = std::fs::write(&dst, &bytes);
                        eprintln!(
                            "dropped file copied: {} bytes -> {}",
                            bytes.len(),
                            dst.display()
                        );
                        // keep watching: the file may appear before the
                        // module, the module dump is the complete picture
                    }
                }
                // timeout: dump the stub module itself as a fallback
                if GetTickCount().wrapping_sub(started) > CHAIN_TIMEOUT_MS {
                    let stub: HMODULE = GetModuleHandleA(pcstr(b"wmode-unpack-host.dll\0"));
                    if !stub.is_null() {
                        let dst = out_dir.join("wmode-layer2.dll");
                        if let Some(n) = dump_module(stub, &dst) {
                            eprintln!("timeout, stub dumped: {n} bytes -> {}", dst.display());
                        }
                    }
                    eprintln!("timeout, xmod2bwl.dll never appeared");
                    std::process::exit(2);
                }
                Sleep(POLL_MS);
            }
        }
    });

    let mut path_b = load_path.into_bytes();
    path_b.push(0);
    let stub: HMODULE = unsafe { LoadLibraryA(pcstr(&path_b)) };
    eprintln!("LoadLibraryA returned {stub:p}");
    if stub.is_null() {
        std::process::exit(3);
    }

    // act as the BWL4 launcher: suspended target, then the export call
    // that makes the stub unpack and inject xmod2bwl.dll
    let apply_suspended: unsafe extern "C" fn(*mut core::ffi::c_void, u32) -> i32 = unsafe {
        let proc = find_export_memory(stub, b"ApplyPatchSuspended");
        if proc.is_null() {
            eprintln!("ApplyPatchSuspended export not found in memory");
            std::process::exit(4);
        }
        core::mem::transmute(proc)
    };
    let dummy = unsafe { spawn_suspended_dummy() };
    if dummy.is_null() {
        eprintln!("failed to spawn the dummy process");
        std::process::exit(5);
    }
    eprintln!("calling ApplyPatchSuspended");
    let rc = unsafe { apply_suspended(dummy, 0) };
    eprintln!("ApplyPatchSuspended returned {rc}");

    let _ = watcher.join();
    eprintln!("host survived the whole chain");
}

/// Resolves an export from a loaded module's in-memory export table,
/// bypassing the loader catalog (which was built from the packed file).
unsafe fn find_export_memory(module: HMODULE, name: &[u8]) -> *mut core::ffi::c_void {
    let base = module as *const u8;
    let pe = read_u32(base, 0x3C) as usize;
    let opt = pe + 0x18;
    let exp_rva = read_u32(base, opt + 96) as usize;
    if exp_rva == 0 {
        return core::ptr::null_mut();
    }
    let exports = base.add(exp_rva);
    let ordinal_base = read_u32(exports, 16);
    let n_names = read_u32(exports, 24) as usize;
    let names = base.add(read_u32(exports, 32) as usize);
    let ords = base.add(read_u32(exports, 36) as usize);
    let funcs = base.add(read_u32(exports, 28) as usize);
    for i in 0..n_names {
        let name_rva = read_u32(names, i * 4) as usize;
        let candidate = base.add(name_rva);
        let mut len = 0usize;
        while *candidate.add(len) != 0 {
            len += 1;
        }
        if core::slice::from_raw_parts(candidate, len) == name {
            let ordinal = (ords.add(i * 2) as *const u16).read_unaligned();
            let index = ordinal as isize - ordinal_base as isize;
            if index < 0 {
                return core::ptr::null_mut();
            }
            let func_rva = read_u32(funcs, index as usize * 4) as usize;
            return base.add(func_rva) as *mut core::ffi::c_void;
        }
    }
    core::ptr::null_mut()
}

/// Full path of a fresh suspended copy of this executable.
unsafe fn spawn_suspended_dummy() -> *mut core::ffi::c_void {
    use windows_sys::Win32::System::Threading::{
        CreateProcessA, PROCESS_INFORMATION, STARTUPINFOA, CREATE_SUSPENDED,
    };
    let exe = env::current_exe().expect("current exe path");
    let mut cmd = format!("\"{}\" --dummy\0", exe.display());
    let mut startup = STARTUPINFOA {
        cb: size_of::<STARTUPINFOA>() as u32,
        ..Default::default()
    };
    let mut info = PROCESS_INFORMATION::default();
    let ok = CreateProcessA(
        core::ptr::null(),
        cmd.as_mut_ptr(),
        core::ptr::null(),
        core::ptr::null(),
        0,
        CREATE_SUSPENDED,
        core::ptr::null(),
        core::ptr::null(),
        &mut startup,
        &mut info,
    );
    if ok == 0 {
        return core::ptr::null_mut();
    }
    info.hProcess
}
