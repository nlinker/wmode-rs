"""
Dump a memory region of a running process to a file.

    python dump_region.py <pid> <hex_addr> <hex_size> <out_file> [more pairs...]

Each pair is one region: the bytes land in out_file as they sit in memory
(file offsets equal virtual offsets, raw = virtual layout, the layout
Ghidra accepts for a manually mapped image). Read-only on the process.
"""

import ctypes
import ctypes.wintypes as wt
import sys

k32 = ctypes.WinDLL("kernel32", use_last_error=True)
adv = ctypes.WinDLL("advapi32", use_last_error=True)

PROCESS_QUERY_INFORMATION = 0x0400
PROCESS_VM_READ = 0x0010
TOKEN_ADJUST_PRIVILEGES = 0x0020
SE_PRIVILEGE_ENABLED = 0x0002


class LUID(ctypes.Structure):
    _fields_ = [("LowPart", wt.DWORD), ("HighPart", wt.LONG)]


class TOKEN_PRIVILEGES(ctypes.Structure):
    _fields_ = [("PrivilegeCount", wt.DWORD), ("Luid", LUID), ("Attributes", wt.DWORD)]


k32.GetCurrentProcess.restype = wt.HANDLE
k32.OpenProcess.restype = wt.HANDLE
k32.OpenProcess.argtypes = [wt.DWORD, wt.BOOL, wt.DWORD]
k32.ReadProcessMemory.restype = wt.BOOL
k32.ReadProcessMemory.argtypes = [wt.HANDLE, ctypes.c_void_p, ctypes.c_void_p,
                                  ctypes.c_size_t, ctypes.POINTER(ctypes.c_size_t)]
k32.CloseHandle.argtypes = [wt.HANDLE]
adv.OpenProcessToken.restype = wt.BOOL
adv.OpenProcessToken.argtypes = [wt.HANDLE, wt.DWORD, ctypes.POINTER(wt.HANDLE)]
adv.LookupPrivilegeValueW.restype = wt.BOOL
adv.LookupPrivilegeValueW.argtypes = [wt.LPCWSTR, wt.LPCWSTR, ctypes.POINTER(LUID)]
adv.AdjustTokenPrivileges.restype = wt.BOOL
adv.AdjustTokenPrivileges.argtypes = [wt.HANDLE, wt.BOOL, ctypes.POINTER(TOKEN_PRIVILEGES),
                                      wt.DWORD, ctypes.c_void_p, ctypes.c_void_p]


def enable_debug_privilege():
    tok = wt.HANDLE()
    if not adv.OpenProcessToken(k32.GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES, ctypes.byref(tok)):
        return False
    luid = LUID()
    if not adv.LookupPrivilegeValueW(None, "SeDebugPrivilege", ctypes.byref(luid)):
        k32.CloseHandle(tok)
        return False
    tp = TOKEN_PRIVILEGES(1, luid, SE_PRIVILEGE_ENABLED)
    ok = adv.AdjustTokenPrivileges(tok, False, ctypes.byref(tp), 0, None, None)
    k32.CloseHandle(tok)
    return bool(ok)


def dump(h, addr, size, out):
    buf = ctypes.create_string_buffer(size)
    got = ctypes.c_size_t()
    if not k32.ReadProcessMemory(h, addr, buf, size, ctypes.byref(got)):
        print(f"  {addr:08x}: ReadProcessMemory failed, error {ctypes.get_last_error()}")
        return
    with open(out, "wb") as f:
        f.write(buf.raw[:got.value])
    print(f"  {addr:08x} {got.value:#x} bytes -> {out}")


def main():
    args = sys.argv[1:]
    if len(args) < 4 or (len(args) - 1) % 3:
        sys.exit(__doc__)
    pid = int(args[0])
    if not enable_debug_privilege():
        print("warning: could not enable SeDebugPrivilege")
    h = k32.OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, False, pid)
    if not h:
        sys.exit(f"OpenProcess failed: error {ctypes.get_last_error()} (run as Administrator)")
    rest = args[1:]
    for i in range(0, len(rest), 3):
        dump(h, int(rest[i], 16), int(rest[i + 1], 16), rest[i + 2])
    k32.CloseHandle(h)


if __name__ == "__main__":
    main()
