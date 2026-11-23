"""
Locate the real W-MODE payload (layer 3) inside a running StarCraft.exe.

The XMOD loader injects layer 3 without ever calling LoadLibrary, so it has
no module entry and pe-sieve does not classify it as an implanted PE. This
script reads the process memory directly and reports:

  1. the loaded module list (anything wmode-ish is flagged),
  2. every memory hit for the W-MODE marker strings, with the region it
     belongs to and whether that region lies inside a known module,
  3. executable regions outside every module - the candidates for the
     manually mapped payload.

Run from an Administrator console (the game runs with elevated integrity):

    python scan_wmode.py [pid]

Without a pid the newest StarCraft.exe is picked. Read-only: it opens the
process with PROCESS_QUERY_INFORMATION | PROCESS_VM_READ only.
"""

import ctypes
import ctypes.wintypes as wt
import sys

k32 = ctypes.WinDLL("kernel32", use_last_error=True)
psapi = ctypes.WinDLL("psapi", use_last_error=True)

PROCESS_QUERY_INFORMATION = 0x0400
PROCESS_VM_READ = 0x0010
MEM_COMMIT = 0x1000
PAGE_EXECUTE = 0x10
PAGE_EXECUTE_READ = 0x20
PAGE_EXECUTE_READWRITE = 0x40
PAGE_EXECUTE_WRITECOPY = 0x80
PAGE_GUARD = 0x100
PAGE_NOACCESS = 0x01
EXEC_MASKS = (PAGE_EXECUTE, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_EXECUTE_WRITECOPY)
EXEC_PROTECT = 0xF0  # the four execute flags together


class MEMORY_BASIC_INFORMATION(ctypes.Structure):
    _fields_ = [
        ("BaseAddress", ctypes.c_void_p),
        ("AllocationBase", ctypes.c_void_p),
        ("AllocationProtect", wt.DWORD),
        ("RegionSize", ctypes.c_size_t),
        ("State", wt.DWORD),
        ("Protect", wt.DWORD),
        ("Type", wt.DWORD),
    ]


class MODULEINFO(ctypes.Structure):
    _fields_ = [
        ("lpBaseOfDll", ctypes.c_void_p),
        ("SizeOfImage", wt.DWORD),
        ("EntryPoint", ctypes.c_void_p),
    ]


k32.OpenProcess.restype = wt.HANDLE
k32.OpenProcess.argtypes = [wt.DWORD, wt.BOOL, wt.DWORD]
k32.VirtualQueryEx.restype = ctypes.c_size_t
k32.VirtualQueryEx.argtypes = [wt.HANDLE, ctypes.c_void_p,
                               ctypes.POINTER(MEMORY_BASIC_INFORMATION), ctypes.c_size_t]
k32.ReadProcessMemory.restype = wt.BOOL
k32.ReadProcessMemory.argtypes = [wt.HANDLE, ctypes.c_void_p, ctypes.c_void_p,
                                  ctypes.c_size_t, ctypes.POINTER(ctypes.c_size_t)]
k32.CloseHandle.argtypes = [wt.HANDLE]
psapi.EnumProcessModulesEx.restype = wt.BOOL
psapi.EnumProcessModulesEx.argtypes = [wt.HANDLE, ctypes.POINTER(wt.HMODULE),
                                        wt.DWORD, ctypes.POINTER(wt.DWORD), wt.DWORD]
psapi.GetModuleFileNameExA.restype = wt.DWORD
psapi.GetModuleFileNameExA.argtypes = [wt.HANDLE, wt.HMODULE, ctypes.c_char_p, wt.DWORD]
psapi.GetModuleInformation.restype = wt.BOOL
psapi.GetModuleInformation.argtypes = [wt.HANDLE, wt.HMODULE,
                                       ctypes.POINTER(MODULEINFO), wt.DWORD]

LIST_MODULES_ALL = 0x03

TOKEN_ADJUST_PRIVILEGES = 0x0020
SE_PRIVILEGE_ENABLED = 0x0002


class LUID(ctypes.Structure):
    _fields_ = [("LowPart", wt.DWORD), ("HighPart", wt.LONG)]


class TOKEN_PRIVILEGES(ctypes.Structure):
    _fields_ = [("PrivilegeCount", wt.DWORD), ("Luid", LUID), ("Attributes", wt.DWORD)]


adv = ctypes.WinDLL("advapi32", use_last_error=True)
k32.GetCurrentProcess.restype = wt.HANDLE
adv.OpenProcessToken.restype = wt.BOOL
adv.OpenProcessToken.argtypes = [wt.HANDLE, wt.DWORD, ctypes.POINTER(wt.HANDLE)]
adv.LookupPrivilegeValueW.restype = wt.BOOL
adv.LookupPrivilegeValueW.argtypes = [wt.LPCWSTR, wt.LPCWSTR, ctypes.POINTER(LUID)]
adv.AdjustTokenPrivileges.restype = wt.BOOL
adv.AdjustTokenPrivileges.argtypes = [wt.HANDLE, wt.BOOL, ctypes.POINTER(TOKEN_PRIVILEGES),
                                      wt.DWORD, ctypes.c_void_p, ctypes.c_void_p]


def enable_debug_privilege():
    """SeDebugPrivilege: without it OpenProcess on the game fails with
    error 5 even from an Administrator console, the token only carries
    the privilege disabled by default."""
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

MARKERS = [
    b"W-MODE", b"wmode.ini", b"SaveWindowClientX", b"SaveClipCursor",
    b"DblSized", b"XeNotRoN", b"xmod2bwl", b"XMOD",
]


def find_pid():
    """Newest StarCraft.exe pid, via the toolhelp snapshot."""
    class PROCESSENTRY32W(ctypes.Structure):
        _fields_ = [
            ("dwSize", wt.DWORD), ("cntUsage", wt.DWORD),
            ("th32ProcessID", wt.DWORD), ("th32DefaultHeapID", ctypes.c_void_p),
            ("th32ModuleID", wt.DWORD), ("cntThreads", wt.DWORD),
            ("th32ParentProcessID", wt.DWORD), ("pcPriClassBase", wt.LONG),
            ("dwFlags", wt.DWORD), ("szExeFile", wt.WCHAR * 260),
        ]

    k32.CreateToolhelp32Snapshot.restype = wt.HANDLE
    k32.CreateToolhelp32Snapshot.argtypes = [wt.DWORD, wt.DWORD]
    k32.Process32FirstW.restype = wt.BOOL
    k32.Process32FirstW.argtypes = [wt.HANDLE, ctypes.POINTER(PROCESSENTRY32W)]
    k32.Process32NextW.restype = wt.BOOL
    k32.Process32NextW.argtypes = [wt.HANDLE, ctypes.POINTER(PROCESSENTRY32W)]

    TH32CS_SNAPPROCESS = 0x02
    snap = k32.CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
    pe = PROCESSENTRY32W()
    pe.dwSize = ctypes.sizeof(pe)
    found = []
    if k32.Process32FirstW(snap, ctypes.byref(pe)):
        while True:
            if pe.szExeFile.lower() == "starcraft.exe":
                found.append(pe.th32ProcessID)
            if not k32.Process32NextW(snap, ctypes.byref(pe)):
                break
    k32.CloseHandle(snap)
    if not found:
        sys.exit("StarCraft.exe is not running")
    return found[-1]


def list_modules(h):
    mods = []
    arr = (wt.HMODULE * 1024)()
    needed = wt.DWORD()
    if not psapi.EnumProcessModulesEx(h, arr, ctypes.sizeof(arr), ctypes.byref(needed), LIST_MODULES_ALL):
        return mods
    count = needed.value // ctypes.sizeof(wt.HMODULE)
    buf = ctypes.create_string_buffer(1024)
    for i in range(count):
        mi = MODULEINFO()
        psapi.GetModuleInformation(h, arr[i], ctypes.byref(mi), ctypes.sizeof(mi))
        psapi.GetModuleFileNameExA(h, arr[i], buf, 1024)
        mods.append((mi.lpBaseOfDll, mi.SizeOfImage, buf.value.decode("mbcs", "replace")))
    return mods


def module_of(addr, mods):
    for base, size, name in mods:
        if base <= addr < base + size:
            return name
    return None


def prot_name(p):
    kind = p & 0xFF
    names = {0x01: "NOACCESS", 0x02: "R", 0x04: "RW", 0x08: "WC", 0x10: "X",
             0x20: "RX", 0x40: "RWX", 0x80: "XWC"}
    kind_name = names.get(kind, hex(kind))
    if p & PAGE_GUARD:
        kind_name += "+GUARD"
    return kind_name


def main():
    pid = int(sys.argv[1]) if len(sys.argv) > 1 else find_pid()
    if not enable_debug_privilege():
        print("warning: could not enable SeDebugPrivilege, OpenProcess may fail")
    h = k32.OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, False, pid)
    if not h:
        sys.exit(f"OpenProcess failed: error {ctypes.get_last_error()} (run as Administrator)")

    print(f"PID {pid}")
    mods = list_modules(h)
    print(f"\n== modules ({len(mods)}) ==")
    for base, size, name in mods:
        low = name.lower()
        flag = "  <-- wmode-ish" if any(k in low for k in ("wmode", "xmod", "bwl")) else ""
        print(f"  {base:08x} {size:8x} {name}{flag}")

    print("\n== memory scan ==")
    hits = {}
    exec_outside = []
    addr = 0x10000
    mbi = MEMORY_BASIC_INFORMATION()
    CHUNK = 1 << 20
    buf = ctypes.create_string_buffer(CHUNK)
    got = ctypes.c_size_t()
    regions = 0
    while addr < 0x7FFFFFFF:
        if not k32.VirtualQueryEx(h, addr, ctypes.byref(mbi), ctypes.sizeof(mbi)):
            break
        base = mbi.BaseAddress or 0
        size = mbi.RegionSize
        if mbi.State == MEM_COMMIT and not (mbi.Protect & PAGE_GUARD) and not (mbi.Protect & PAGE_NOACCESS):
            regions += 1
            off = 0
            while off < size:
                n = min(CHUNK, size - off)
                at = base + off
                if k32.ReadProcessMemory(h, at, buf, n, ctypes.byref(got)) and got.value:
                    data = buf.raw[:got.value]
                    for m in MARKERS:
                        start = 0
                        while True:
                            i = data.find(m, start)
                            if i < 0:
                                break
                            a = at + i
                            hits.setdefault(m, []).append(a)
                            start = i + 1
                off += n
            if (mbi.Protect & EXEC_PROTECT) in EXEC_MASKS and module_of(base, mods) is None:
                exec_outside.append((base, size, prot_name(mbi.Protect), mbi.Type))
        addr = base + size

    print(f"committed regions scanned: {regions}")
    for m, lst in hits.items():
        print(f"\n  {m.decode()}: {len(lst)} hit(s)")
        for a in lst[:12]:
            # the region and its allocation base, for a manual follow-up
            if k32.VirtualQueryEx(h, a, ctypes.byref(mbi), ctypes.sizeof(mbi)):
                abase = mbi.AllocationBase or 0
                mod = module_of(a, mods)
                tag = mod if mod else f"NO MODULE, alloc base {abase:08x}, type {mbi.Type:#x}, {prot_name(mbi.Protect)}"
                print(f"    {a:08x}  {tag}")
        if len(lst) > 12:
            print(f"    ... {len(lst) - 12} more")

    print(f"\n== executable regions outside every module ({len(exec_outside)}) ==")
    for base, size, prot, typ in exec_outside[:60]:
        print(f"  {base:08x} size {size:8x} {prot:12} type {typ:#x}")
    if len(exec_outside) > 60:
        print(f"  ... {len(exec_outside) - 60} more")

    k32.CloseHandle(h)


if __name__ == "__main__":
    main()
