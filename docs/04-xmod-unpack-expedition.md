# Reverse-engineering expedition into wmode.bwl

Date: 2026-11-22 (expedition), 2026-11-23 (this write-up). Scope: the attempt to
unpack `original\wmode.bwl` down to the real W-MODE code, what stopped it, and what
would work if it is ever resumed. Companion to `docs/01-decompilation-findings.md`
(which covers the static pass); this file covers the dynamic one.

## Why reverse it at all

The Rust rewrite already had two references: `WModeReadme.txt` (behavior) and BWAPI's
`WMode.cpp` (architecture). The packed binary was the potential third: the only source
of truth for how the original handled the window registry flag, doublesize, and the
BWL4 handshake. It turned out the first two questions were answered cheaper (storm's
own `Window Mode` registry path, BWAPI's code), which is what made stopping rational.

## The onion

| layer | what it is | status |
|-------|------------|--------|
| 0 | `wmode.bwl` the file: PE image plus an appended XMOD2 metadata tail (markers `XHDR`/`XMODPR`, keys `NAME`/`DESC`/`VER`/`AUTH`/`TIME`/`COMP`/`URL` in `decomp\wmode.dll.strings`) | input |
| 1 | PE stub, exactly one function (`entry` @ `0x100062e0`): LZ-decompresses a blob at `0x10005000` over `0x10001000`, rebuilds a manual IAT, scrubs packer flags in the PE header, jumps into the unpacked image | decompiled, `decomp\wmode.dll.c` (see docs/01) |
| 2 | "XeN'z XMOD loader stub for BWL": a generic BWL4 loader, nothing W-MODE-specific. 44 functions, exports `ApplyPatch` (RVA `0x1DF8`), `ApplyPatchSuspended` (`0x1DDB`), `GetData` (`0x1C21`), `GetPluginAPI` (`0x1A2E`), `OpenConfig` (`0x1A48`) | dumped (`gproj\wmode-layer2.dll`), decompiled (`decomp\wmode-unpacked.dll.c`) |
| 3 | the real W-MODE payload (window, hotkeys, wmode.ini, DirectDraw hooks) | never extracted |

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

## Artifact map

`decomp\` and `gproj\` are gitignored (regenerable); the tools are tracked.

- `scripts\decompile-wmode.ps1` - layer 1 pass (reproduces `decomp\wmode.dll.c`,
  `.strings`); `scripts\decompile-storm.ps1` for storm.dll.
- `src\bin\unpack-wmode.rs` - the host. Reproduces the layer 2 dump: run it, it
  writes `gproj\wmode-layer2.dll` / `gproj\wmode-unpacked.dll`; import that into
  Ghidra manually for the second pass (`decomp\wmode-unpacked.dll.c`).
- `docs/01-decompilation-findings.md` - layer 1 static analysis and the ini-key
  string recovery.
