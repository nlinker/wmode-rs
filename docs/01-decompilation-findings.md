# W-MODE decompilation findings

Date: 2026-11-21 · Input: `wmode.bwl` (W-MODE v1.02, XeNotRoN, 2008-01-04, 37 851 bytes)
Tooling: Ghidra 12.1.4 headless via `scripts\decompile-wmode.ps1` → `decomp\wmode.dll.c`, `decomp\wmode.dll.strings`

## Headline: the file is packed — the decompile is the unpacker stub, not the plugin

`decomp\wmode.dll.c` contains exactly **one function** (`entry` @ `0x100062e0`, 438 bytes of code). Everything else in the binary is data. What that function does:

1. **LZ-decompresses** a blob at `0x10005000` on top of `0x10001000` (bit-oriented loop: literal runs + back-references with distance/count, terminator writes length `0x141d` — an aPLib/LZSS-style custom codec; Ghidra flags parts as bad instructions because the unpacked code overlaps the stub region).
2. **Rebuilds imports**: walks a table at `0x10005000` — for each module `LoadLibraryA`, for each name `GetProcAddress`, writes the pointer into the unpacked region (manual IAT).
3. **Un-flags the PE header**: `VirtualProtect(0x10000000, 0x1000, PAGE_READWRITE)`, clears bit 7 (`&= 0x7f`) at header offsets `0x1ff` and `0x227`, restores protection — scrubbing packer markers from the section table.
4. Transfers control into the unpacked image (control flow truncated by the disassembler).

The stub's own imports are only `LoadLibraryA`, `GetProcAddress`, `VirtualProtect`, `wsprintfA` (KERNEL32) + a couple of USER32 entries — a textbook packer import set.

## It is an XMOD2 container

Strings confirm the format ("XeN'z XMOD", `xmod2bwl.dll`):

- Export set of the payload: `ApplyPatch`, `ApplyPatchSuspended`, `GetData`, `GetPluginAPI`, `OpenConfig`.
- Metadata block with `VER`/`DESC`/`AUTH`/`TIME`/`COMP`/`URL` keys: v1.02, "Windowed mode plugin for all versions of Original StarCraft and BroodWar", XeNotRoN, January 4 2008, BWProgrammers.
- Loading instructions embedded: needs an XMOD-capable loader — BWL4 or `scloader2b -xmod wmode.bwl`. (Matches `WModeReadme.txt`: original BWL4 ≤ 4.0.3.3 rejects it, ChaosLauncher's BWL4 fork accepts it.)

So the actual W-MODE logic (hotkeys, DirectDraw hooks, window management) lives **compressed inside the blob** and is not visible statically from this pass.

## wmode.ini keys recovered from strings (matches readme)

`[W-MODE]` section, defaults: `SaveWindowClientX=1`, `SaveWindowClientY=1`, `SaveClipCursor=0`, `SaveEnableWindowMove=1`, `SaveAlwaysOnTop=1`, `SaveDisableControls=1`, `WindowClientX/Y` (unset = centered), `ClipCursor=0` (ALT+F1), `EnableWindowMove=1` (ALT+F10), `AlwaysOnTop=0` (ALT+F11), `DisableControls=0` (ALT+F12), `MaxFps=100`, `MuteNotFocused=0`. ALT+F9 (doublesize) has no ini key in the strings — original stores its state only in the saved-settings keys above.

## How to get the real code (if ever needed)

| Option            | Effort | Notes                                                                                                                                                                        |
|-------------------|--------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Runtime dump      | Low    | Tiny loader `LoadLibrary`s the DLL, lets `DllMain`/entry unpack to `0x10001000+`, then dumps the region (pe-sieve/Scylla or 20-line dumper). Second Ghidra pass on the dump. |
| Static unpack     | Medium | Reimplement the bit-oriented LZ loop from `entry` in Python, feed it bytes at `0x10005000`.                                                                                  |
| **Skip (chosen)** | None   | The Rust rewrite does not need the original's internals — see below.                                                                                                         |

The runtime-dump route was later attempted and got two layers deep before being shelved; see `docs/04-xmod-unpack-expedition.md`.

## BWAPI's WMode.cpp/h as the behavioral reference

`WMode.{h,cpp}` in the BWAPI source tree (`bwapi/BWAPI/Source/`, in the workspace checkout; read in full) is BWAPI's clean-room in-process W-Mode. Key architecture, directly reusable for the Rust plugin:

- **Render path**: hooks Storm's `SDrawLockSurface` / `SDrawUnlockSurface` / `SDrawUpdatePalette`. Lock hands the game a plain 8-bpp DIB section (`CreateDIBSection`, palette from `GamePalette`); Unlock sets a dirty flag + `InvalidateRect`. `WM_PAINT` blits the DIB to the client area — **`StretchDIBits` with `HALFTONE` when client size ≠ game size** (`WMode.cpp:337-340`). That is exactly the mechanism a 1.5x scale needs.
- **Scaling is not just pixels**: `FixPoints` (`WMode.cpp:99-114`) rescales mouse coordinates by `game_size / client_size` on every mouse message; cursor pos/clip (`GetCursorPos`/`SetCursorPos`/`ClipCursor`) are hooked too.
- **Window management**: subclasses the game window (`SetWindowLong GWL_WNDPROC`), `WM_SIZING` enforces min 200x150, Ctrl-constrained 4:3 resize, snaps to native game size ±20 px; border math via `GetBorderRect`/`GetBorderSize`.
- **Important differences vs the original plugin**: BWAPI's version toggles with **ALT+ENTER** (no ALT+F9, no doublesize at all, no `wmode.ini` — config goes to `bwapi.ini` `[window]`). It also detects the original Xen W-Mode by the presence of `WS_SYSMENU` in the window style (`WMode.cpp:420,438,576`) and defers to it. So BWAPI's source tells us *how* to scale, not *how the original toggled doublesize* — the [1x → 1.5x → 2x] cycle is our own feature: ALT+F9 handler sets client size to `game_size × factor` (`SetWindowPos` with `GetBorderSize` added) and the existing paint path does the rest.

## Implication for the Rust rewrite

1. Base design on BWAPI's WMode.cpp + `WModeReadme.txt`, not on the packed original.
2. Deliverable: Rust cdylib, 32-bit x86, exports the BWL4 plugin entry points (`GetData` + optional `OpenConfig`; the XMOD `ApplyPatch*` set is only for the packed original — a plain BWL4 plugin needs neither XMOD nor `-xmod`).
3. Hook set: storm `SDraw*` (via IAT patch of the game's storm.dll imports, or direct storm export patching), user32 cursor trio, `CreateWindowExA`/window subclass for hotkeys.
4. New behavior: ALT+F9 cycles `scale = 1.0 → 1.5 → 2.0 → 1.0`, window client size `= game_size × scale`, repaint via `StretchDIBits`; keep ALT+F1/F10/F11/F12 and `wmode.ini` semantics from the readme.

Raw artifacts: `decomp\wmode.dll.c` (stub), `decomp\wmode.dll.strings`, `decomp\wmode.dll.log` (import succeeded, postScript ran).
