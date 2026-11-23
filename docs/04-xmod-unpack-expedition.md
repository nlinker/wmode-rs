# Reverse-engineering expedition into wmode.bwl

Date: 2026-11-22 (first expedition), 2026-11-23 (write-up), 2026-11-24 (layer 3
extracted - see the last section). Scope: unpacking `original\wmode.bwl` down to
the real W-MODE code. Companion to `docs/01-decompilation-findings.md` (the static
pass); this file covers the dynamic one.

## Why reverse it at all

The Rust rewrite already had two references: `WModeReadme.txt` (behavior) and BWAPI's
`WMode.cpp` (architecture). The packed binary was the potential third: the only source
of truth for how the original handled the window registry flag, doublesize, and the
BWL4 handshake. It turned out the first two questions were answered cheaper (storm's
own `Window Mode` registry path, BWAPI's code), which is what made stopping rational.

## The onion

| layer | what it is                                                                                                                                                                                                                                      | status                                                                                                          |
|-------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------|
| 0     | `wmode.bwl` the file: PE image plus an appended XMOD2 metadata tail (markers `XHDR`/`XMODPR`, keys `NAME`/`DESC`/`VER`/`AUTH`/`TIME`/`COMP`/`URL` in `decomp\wmode.dll.strings`)                                                                | input                                                                                                           |
| 1     | PE stub, exactly one function (`entry` @ `0x100062e0`): LZ-decompresses a blob at `0x10005000` over `0x10001000`, rebuilds a manual IAT, scrubs packer flags in the PE header, jumps into the unpacked image                                    | decompiled, `decomp\wmode.dll.c` (see docs/01)                                                                  |
| 2     | "XeN'z XMOD loader stub for BWL": a generic BWL4 loader, nothing W-MODE-specific. 44 functions, exports `ApplyPatch` (RVA `0x1DF8`), `ApplyPatchSuspended` (`0x1DDB`), `GetData` (`0x1C21`), `GetPluginAPI` (`0x1A2E`), `OpenConfig` (`0x1A48`) | dumped (`gproj\wmode-layer2.dll`), decompiled (`decomp\wmode-unpacked.dll.c`)                                   |
| 3     | the real W-MODE payload (window, hotkeys, wmode.ini, DirectDraw hooks)                                                                                                                                                                          | **extracted 2026-11-24**: `gproj\dumps\layer3_wmode.bin`, decompiled to `decomp\layer3_wmode.c` (236 functions) |

Concrete packer evidence on the file itself: blank section names, first section
`VirtualSize 0x4000 / RawSize 0`, entry RVA `0x62E0` inside the stub.

A side find worth keeping: layer 2's `GetPluginAPI` writes
`iStarCraftBuild = 0xFFFFFFFF` - literally the readme's "-1 build" bug that made
original BWL4 builds up to 4.0.3.3 reject the plugin, confirmed in the binary.

## Tools built for the attempt

- `scripts\decompile-wmode.ps1` + `scripts\DecompileAll.java` - headless Ghidra 12.1.4
  (JDK 21) pass over the file, plus an ASCII strings dump.
- `src\bin\unpack-wmode.rs` - sacrificial host: `LoadLibraryA`s the .bwl, waits for the
  image to go stable, dumps it, then emulates the BWL4 launcher (spawns a suspended
  dummy process, resolves layer 2's exports from memory, calls
  `ApplyPatchSuspended`, watches for the payload).

## How far it got

1. Static pass on the raw file: one function - the layer 1 stub (docs/01).
2. Host run with a dump-on-first-change watcher captured the stub mid-work, not the
   result. Taught the watcher to wait for image stability instead.
3. First useful dump was file-layout and Ghidra refused the unpacked code
   ("Disassembly not permitted within uninitialized memory block") - the unpacked
   bytes lived beyond the section's raw size. Fix: dump with raw = virtual layout
   (file offsets equal RVAs, rawsize = virtualsize). Lesson for any future dump.
4. Second Ghidra pass: 44 functions, 0 failures - layer 2 fully readable.
5. Protocol emulation attempts failed informatively:
   - the first manual export resolution returned pointers instead of BOOLs - the
     ordinal lookup missed the export table's `OrdinalBase` subtraction;
   - with the real RVAs, `ApplyPatchSuspended(dummy_process, ...)` still did nothing.
     Reading the code showed why: the real work sits in `FUN_10001cb9`, gated by an
     init flag (`DAT_10001fb4`) and a magic argument `param_1 == 0x4E4` - anything
     else is a silent early-out;
   - the injector core (`FUN_100011b6`) is deliberately obfuscated: kernel32
     addresses come from a byte-signature scan of 64 KB of memory, handles are
     XOR-decoded with a key derived from the PEB (`FS:[0x30]`), and the strings
     include `DebugActiveProcess`.

## The obstacle

Layer 3 is locked behind the obfuscated injector plus an undocumented BWL4/XMOD2
handshake. Two roads existed, both hours of work:

- emulate the protocol faithfully - reverse the obfuscated init chain far enough to
  satisfy the gates and drive the injector from our own host;
- parse the XMOD2 container statically - the metadata tail is visible in the strings,
  but the payload blob's framing and compression would be a fresh format reverse.

Meanwhile the rewrite's actual questions had already been answered from storm's
decompilation and BWAPI. Expedition shelved on 2026-11-22; artifacts kept (see the
map at the end).

## If resumed: hypotheses, cheapest first

1. **Let the real launcher do the unpacking, no debugger.** A byte-identical
   `wmode.bwl` ships at `D:\scai\scbw\Chaoslauncher\Plugins\wmode.bwl`. Enable it in
   ChaosLauncher (alongside or instead of wmode-rs), start StarCraft, and while the
   game runs look for the dropped `xmod2bwl.dll` - layer 2's strings ("Error opening
   plugin data file", `hXMODPR`) suggest the payload DLL materializes on disk before
   injection. Check `%TEMP%`, the game dir, the Chaoslauncher dir. Process Monitor
   (filter `Path contains xmod2bwl`) catches the drop even if the file is deleted
   right after - then it is a copy and one Ghidra pass, done.
2. **ChaosLauncher under a debugger** (the hypothesis this section is named for).
   Tool: x32dbg from the x64dbg suite - ChaosLauncher is a 32-bit x86 process, and
   x32dbg is the standard lightweight choice for that era; ScyllaHide (x64dbg plugin)
   is the mask for the anti-debug smells listed above (PEB `BeingDebugged`,
   `NtGlobalFlag`, handle checks). Method: the BWL4 loader `LoadLibrary`s the .bwl
   into the launcher process itself, so layers 1 and 2 unpack there. Run
   ChaosLauncher under x32dbg, let the wmode module appear, then either
   `bp wmode+0x1DDB` (`ApplyPatchSuspended`, RVA from the layer 2 dump), or
   `bp kernel32.WriteProcessMemory` - the buffer written into the suspended
   StarCraft.exe IS the layer 3 image; dump it from the breakpoint. Alternative if
   single-stepping is preferred: break on layer 1's entry and step past the LZ loop.
3. **Dump from the game side after a real injection.** The payload is manually
   mapped, so it has no module entry. pe-sieve (hasherezade) run against the live
   StarCraft.exe detects and dumps manually mapped PEs; System Informer (Process
   Hacker) shows memory regions - a string scan for "W-MODE" locates the payload,
   dump the region. Scylla rebuilds the IAT if the dump needs one.
4. **Static container parse.** Locate the payload blob via the `XHDR`/`XMODPR`
   markers in the file tail, then port the decompressor (the same bit-oriented LZ
   family as layer 1, sketched in docs/01) or breakpoint it during a debugger run and
   dump its output. Fully offline and reproducible once written, but it is the
   largest one-time effort.
5. **Ghidra's own Debugger** (Windows user-mode via the WinDbg engine) can drive the
   same breakpoints with the layer 2 listing already annotated. Heavier setup than
   x32dbg; take it if the annotation matters.

Debugger choice in one line: x32dbg + ScyllaHide first, WinDbg only if
loader/kernel-level views are needed, Ghidra Debugger to stay inside the existing
listing.

## Resolution: layer 3 extracted from the live game (2026-11-24)

The approach that worked was hypothesis 3 from the list above ("dump from the
game side after a real injection"): let ChaosLauncher and the genuine XMOD
loader do the unpacking, then read the result out of the game process. Nothing
was defeated at runtime - the protection is architectural, see the note at the
end of this section.

Two false leads came first:

- pe-sieve against the running game found nothing suspicious (`Implanted: 0`,
  73 modules, 701 regions). Not a packer countermeasure: the payload has no PE
  header in memory, so it does not look like an implanted PE to a scanner built
  around PE headers.
- `OpenProcess` on the game failed with error 5 even from an Administrator
  console. Also not anti-debug: ChaosLauncher starts the game with a raised
  integrity level, and an Administrator token carries `SeDebugPrivilege`
  disabled by default. pe-sieve enables it itself; a plain Python ctypes script
  has to call `AdjustTokenPrivileges` first. After that one call `OpenProcess`
  with `PROCESS_QUERY_INFORMATION | PROCESS_VM_READ` succeeded - no debugger
  was attached to the game, no `IsDebuggerPresent`/`CheckRemoteDebuggerPresent`
  check stood in the way.

On the `DebugActiveProcess` string in layer 2: it belongs to the generic XMOD
stub and was never exercised here. The loader's obfuscation (XOR keys off the
PEB, signature-scanned kernel32 addresses, the magic `0x4E4` handshake) protects
the *loader's own* injection path - it is what stopped the sacrificial-host
attempt - but it does nothing once the payload is already mapped and running.
Bypassing it meant not fighting it: the real launcher walked that path for us.

What found it: `gproj\scan_wmode.py` (ctypes, read-only, `PROCESS_QUERY_INFORMATION
| PROCESS_VM_READ`) walking every committed region looking for the marker strings
`W-MODE`, `wmode.ini`, `SaveWindowClientX`, `DblSized`, `XMOD`. Two private RWX
regions outside every module answered, both `0x11000` bytes:

| base         | markers                     | what it is                                                                                  |
|--------------|-----------------------------|---------------------------------------------------------------------------------------------|
| `0x001d0000` | `XMOD` + all W-MODE strings | the XMOD loader stub (layer 2) as it sits in the game, with the payload still packed inside |
| `0x028b0000` | W-MODE strings only         | **layer 3, the real W-MODE**                                                                |

Both are headerless bodies: XMOD maps its images without the MZ/PE headers, so
the file offsets equal virtual offsets from the allocation base. Code starts at
`+0x1000` (the executable sub-region), the manual IAT lives at `+0x9200..+0x9D40`
(KERNEL32/USER32/GDI32/MSVCRT names in plain text, resolved at load), the
`ddraw.dll`/`dsound.dll` runtime-resolution strings and the `wmode.xmod` /
`get_wmode_interface` / `scl_entry` export names sit at `+0x9DAC..+0x9DCB`, the
caption-button bitmaps (the `R`/`J`/`z`/`I`/`Q`/`[` runs) from `+0xA811` on.

Dump with `gproj\dump_region.py` (same privilege dance), then one headless Ghidra
pass as a raw x86 binary at base `0x028B0000`:

```
analyzeHeadless gproj layer3 -import gproj\dumps\layer3_wmode.bin -loader BinaryLoader -processor x86:LE:32:default -loader-baseAddr 0x028B0000 -scriptPath scripts -postScript DecompileAll.java decomp\layer3_wmode.c
```

236 functions, 0 failures. First findings, straight from the decompile:

- `FUN_028b4130` (4405 bytes) is the W-MODE window procedure, installed through
  `SetPropA` with the property name `WMODE_WND_PROC_PROP` (the strings also name
  the game's window classes `SWarClass`, `SDlgDialog`, `SDlgStatic`,
  `StormComboBox`, `StormScrollbar`).
- The hotkey arm dispatches into one button handler `FUN_028b6dea(button, state)`
  where the button index is the caption icon: F1 -> 3 (cursor clip), F9 -> 1
  (doublesize), F10 -> 5 (window move, and only while ALT is already released:
  `GetKeyState(VK_MENU) < 0` skips it), F11 -> 4 (always on top), F12 -> 6
  (caption controls). So the readme's hotkeys and the caption icons are one
  mechanism in the original.
- The row has eight button slots, not six: 2 has no hotkey, 7 and 8 are gated by
  a refcount helper (`FUN_028b702a`). Button 7 flips `DAT_028c09d4`, which the
  WM_MOUSEMOVE path tests together with the "over the frame" state - the
  original's own cursor-confinement switch.
- All `WM_NC*BUTTON*` messages (`0xA0..0xA9`) are swallowed, so the caption is
  entirely custom-drawn (`CreateIconFromResource`, `SetWindowRgn`,
  `BeginDeferWindowPos`).

Open questions left for a later pass: the exact meaning of buttons 2 and 8, and
the doublesize implementation (`FUN_028b692b` / `FUN_028b708f`) versus our scale
ladder.

## Artifact map

`decomp\` and `gproj\` are gitignored (regenerable); the tools are tracked.

- `scripts\scan-wmode-process.py` and `scripts\dump-region.py` - the live-game
  route to layer 3 (Administrator console, Python 3, no dependencies). Scan first
  to find the private region carrying the W-MODE strings, then dump it; the
  Ghidra command line above turns the dump into `decomp\layer3_wmode.c`.
- `scripts\decompile-wmode.ps1` - layer 1 pass (reproduces `decomp\wmode.dll.c`,
  `.strings`); `scripts\decompile-storm.ps1` for storm.dll.
- `src\bin\unpack-wmode.rs` - the host. Reproduces the layer 2 dump: run it, it
  writes `gproj\wmode-layer2.dll` / `gproj\wmode-unpacked.dll`; import that into
  Ghidra manually for the second pass (`decomp\wmode-unpacked.dll.c`).
- `docs/01-decompilation-findings.md` - layer 1 static analysis and the ini-key
  string recovery.
