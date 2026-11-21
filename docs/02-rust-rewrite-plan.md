# Plan: wmode-rs — Rust rewrite of W-MODE v1.02

Companion to `01-decompilation-findings.md`. Deliverable: a Rust cdylib that replaces `wmode.bwl`
as a ChaosLauncher (BWL4) plugin for StarCraft 1.16.1, feature-equal to the original **except**
ALT+F9, which cycles window scale `1x → 1.5x → 2x → 1x` instead of toggling 2x.

## Crate layout (Cargo.toml at the repo root)

```
<repo>\
  Cargo.toml            # crate wmode-rs, crate-type ["cdylib"], no build.rs needed
  src\
    lib.rs              # DllMain (no_mangle extern "system"), init-thread spawn
    bwl.rs              # BWL4 exports: GetPluginAPI/GetData/OpenConfig/ApplyPatch(Suspended)
    inject.rs           # remote LoadLibraryA injector (VAlloc + RemoteThread pattern)
    iat.rs              # generic IAT patcher for a loaded module's imports
    hooks.rs            # SDrawLockSurface/SDrawUnlockSurface/SDrawUpdatePalette + cursor trio
    window.rs           # window subclass: WM_* handling, hotkeys, WM_PAINT blit, WM_SIZING
    render.rs           # 8-bpp DIB section, palette, StretchDIBits (HALFTONE) paint path
    scale.rs            # SCALE state machine [1.0, 1.5, 2.0], window resize math
    ini.rs              # wmode.ini read/save (from the StarCraft dir, Windows-1252 safe)
    ringbuf.rs          # lock-free SPSC for cross-thread state if needed (avoid if possible)
  scripts\ decomp\ docs\                            # first-party: tooling, dump, docs
  wmode\                                               # original third-party drop — untouched
```

Toolchain: stable Rust, target `i686-pc-windows-msvc` (`rustup target add i686-pc-windows-msvc`).
Dependency: `windows-sys` (raw Win32, features Win32_Foundation/UI_WindowsAndMessaging/Graphics_Gdi/System_Threading). No async, no std bloat concerns — plugin must be small and fast.

## BWL4 loader contract (verified against BWAPI's injector, BWAPI_PluginInjector\chaoslauncher.{h,cpp})

The `.bwl` is loaded by ChaosLauncher in **its own** process; the plugin's job is to inject the
real payload into the (suspended) StarCraft process. Exports, all `extern "system"` (stdcall):

| Export                | Signature                                                  | Behavior                                                                                                                                                                                                     |
|-----------------------|------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `GetPluginAPI`        | `fn(&mut ExchangeData)`                                    | `iPluginAPI=4`, `iStarCraftBuild=13` (1.16.1), `bConfigDialog=TRUE`, `bNotSCBWmodule=FALSE`. `ExchangeData` = `#[repr(C)] { i32, i32, i32, i32 }` (16 bytes).                                                |
| `GetData`             | `fn(*mut c_char /*64*/, *mut c_char /*512*/, *mut c_char)` | name/description/updateurl, strcpy semantics.                                                                                                                                                                |
| `OpenConfig`          | `fn() -> i32`                                              | open `wmode.ini` (create from template if missing) in notepad; return 1.                                                                                                                                     |
| `ApplyPatch`          | `fn(HANDLE, u32) -> i32`                                   | no-op, return 1 (process already running path — unused for us).                                                                                                                                              |
| `ApplyPatchSuspended` | `fn(HANDLE, u32) -> i32`                                   | inject **our own DLL path** into suspended `StarCraft.exe`: `VirtualAllocEx` MAX_PATH → `WriteProcessMemory` → `CreateRemoteThread(LoadLibraryA)` → wait, check exit code (chaoslauncher.cpp:50-93 pattern). |

Field names keep the upstream camelCase (`iPluginAPI`): the struct ABI is positional, names never enter the binary, and mirroring `chaoslauncher.h` eases review (`#![allow(non_snake_case)]` covers the lint). Only the export names above are exact-match.

Build artifact renamed `target\i686-pc-windows-msvc\release\wmode_rs.dll` → `wmode-rs.bwl`
in the ChaosLauncher `Plugins\` folder. Keep original `wmode.bwl` alongside until parity.

## In-process architecture (inside StarCraft.exe)

`DllMain` does **nothing but** `CreateThread(init)` — loader-lock rule (no LoadLibrary, no COM,
no window calls in DllMain). The init thread then:

1. **Find/await the game window**: poll `FindWindowA("SWarClass", NULL)` (SC 1.16.1 window class)
   with a bounded retry loop; alternatively IAT-hook `CreateWindowExA` in `StarCraft.exe` first,
   subclass at creation — decided at step 4, start with polling (simpler, works since we inject
   before entry point).
2. **IAT patch** (`iat.rs` walks the PE of `StarCraft.exe`, redirects import thunks):
   - from `storm.dll`: `SDrawLockSurface`, `SDrawUnlockSurface`, `SDrawUpdatePalette`
   - from `user32.dll`: `GetCursorPos`, `SetCursorPos`, `ClipCursor`
   Keep `*Old` originals — identical shape to WMode.cpp:29-34.
3. **Subclass** the game window (`SetWindowLongPtrW GWL_WNDPROC`), keep `wOriginalProc`.

### Render path (render.rs — mirrors WMode.cpp)

- `SDrawLockSurface` → return our DIB section bits (`CreateDIBSection`, 8-bpp, negative height),
  pitch = game width. Game size discovered from the lock call's dest rect (default 640x480).
- `SDrawUnlockSurface` → set dirty flag + `InvalidateRect`; throttled by `MaxFps` (tick check
  like WMode.cpp:315-319, default 100, min 30).
- `SDrawUpdatePalette` → copy into `BITMAPINFO256` palette + `SetDIBColorTable`.
- `WM_PAINT`: client == game size → `BitBlt` (1:1 from memory DC); else `StretchDIBits`
  with `HALFTONE` (WMode.cpp:337-340) — this single branch *is* the 1.5x/2x feature.

### Input scaling (hooks.rs)

`FixPoints` equivalent: on every mouse message, rescale coords by `game_size / client_size`
(WMode.cpp:99-114). `GetCursorPos` → game coords (center 320,240 when cursor hidden / not
holding ALT, real mapped pos when holding ALT); `SetCursorPos`/`ClipCursor` → no-op while
windowed (WMode.cpp:463-517).

## ALT+F9 scale cycle (the new feature)

```
scale.rs: const SCALES: [f32; 3] = [1.0, 1.5, 2.0];  idx: AtomicUsize
```
On `WM_SYSKEYDOWN` `VK_F9` with ALT down: `idx = (idx+1) % 3`; new client size
`round(game_w * s) × round(game_h * s)`; `SetWindowPos` with `GetBorderSize` added; clamp to
visible desktop (WMode.cpp:607-627 math). Persist to `wmode.ini` when `SaveScale=1` (new key,
default 1). All other hotkeys (ALT+F1 clip, F10 move, F11 topmost, F12 caption controls) and
their ini keys exactly per `WModeReadme.txt` / `docs/01-decompilation-findings.md` ini table.

## Steps (each ends testable, one review + commit each)

Workflow: after each step the user reviews the result and commits it. Work on the next step
starts only after the review verdict — never auto-advance.

| #  | Scope                                                                                                          | Milestone                                                                               |
|----|----------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------|
| 0  | Scaffold: `Cargo.toml` at repo root, crate `wmode-rs` cdylib, `windows-sys` dep, `lib.rs` with no-op `DllMain` | `cargo build --release --target i686-pc-windows-msvc` green, zero warnings              |
| 1  | `bwl.rs`: `ExchangeData` struct + `GetPluginAPI` + `GetData` exports                                           | `.bwl` listed in ChaosLauncher plugin page with name + description                      |
| 2  | `OpenConfig`: write `wmode.ini` from template (if missing) + open in notepad                                   | Config button produces a populated ini in the StarCraft dir                             |
| 3  | `inject.rs`: real `ApplyPatchSuspended` (remote `LoadLibraryA`); `ApplyPatch` no-op                            | launch SC via ChaosLauncher → `wmode_rs.dll` in SC process (`tasklist /m`, staging log) |
| 4  | `lib.rs` init thread (loader-lock rule) + `FindWindowA("SWarClass")` bounded poll; staging log helper          | log names the game HWND; decide poll vs `CreateWindowExA` IAT hook                      |
| 5  | `iat.rs` generic IAT patcher, unit-tested against own module                                                   | log lists `SDraw*` + cursor imports resolved in `StarCraft.exe`                         |
| 6  | `window.rs` subclass (`GWL_WNDPROC`), pure pass-through to original                                            | log shows `WM_` traffic; game behavior unchanged                                        |
| 7  | `render.rs` DIB section + `SDrawLock/Unlock/UpdatePalette` hooks, `WM_PAINT` `BitBlt`                          | windowed game renders 1:1; palette correct in menu and in game                          |
| 8  | cursor trio hooks + `FixPoints` coordinate scaling                                                             | mouse coords correct when client ≠ game size                                            |
| 9  | hotkeys ALT+F1 / F10 / F11 / F12 per readme                                                                    | each hotkey toggles its feature; log line per toggle                                    |
| 10 | `scale.rs` ALT+F9 cycle + `StretchDIBits` HALFTONE branch + resize/border math                                 | 1x → 1.5x → 2x cycle renders and input stays correct                                    |
| 11 | `ini.rs`: read full key set on start, save-on-exit semantics (`Save*` keys)                                    | config parity with readme table                                                         |
| 12 | `MaxFps` throttle on the unlock→repaint path + `MuteNotFocused`                                                | fps cap observable; sound mutes on focus loss                                           |
| 13 | `WM_SIZING` min-size/snap, `DisableControls` extras (ALT+F4 block, screensaver), desktop clamp                 | manual checklist clean vs original                                                      |
| 14 | `scripts\deploy.ps1` (build + copy `.bwl` into ChaosLauncher Plugins) + final parity checklist                 | one-command deploy; checklist signed off                                                |

## Step notes (decisions taken during implementation)

- Step 2: the StarCraft directory is found via the registry Game Path
  (`HKCU\Software\Battle.net\Configuration`) first, else derived from this DLL's own location
  (`<sc>\Chaoslauncher\Plugins\<name>.bwl`, three parents up). No `hinst` is stored: the module
  handle is resolved on demand with `GetModuleHandleExA(FROM_ADDRESS)`, keeping `DllMain` a
  pure no-op. Visibility inside the crate is plain `pub`, not `pub(crate)`.
- Step 2: the ini template is the verbatim upstream sample (CRLF, all comments) plus the new
  `SaveScale=1` key. Existing ini files are never overwritten by OpenConfig.

## Risks / open questions

- **BWAPI interplay**: BWAPI's own W-Mode detects Xen W-Mode via `WS_SYSMENU` in the window
  style and defers (WMode.cpp:420,438,576). Our window style must keep `WS_SYSMENU`
  (`WS_OVERLAPPEDWINDOW` does) so co-injection with BWAPI stays consistent. If BWAPI's wmode
  is ON too, behavior undefined — document "don't enable both".
- **Loader lock**: init strictly on a spawned thread; hooks installed only after window exists.
- **HALFTONE StretchDIBits cost** at 2x on weak machines — `MaxFps` default 100 covers it.
- **Windows 11 DPI**: declare DPI-aware via manifest? Original predates it; test raw first.
- **Toolchain**: MSVC link target for x86 — needs VS BuildTools x86 libs installed; fallback
  `i686-pc-windows-gnu` if not.
- **Calling conventions**: everything `extern "system"`; storm hooks are `__stdcall` (`system`
  on x86 == stdcall). Structure layouts `#[repr(C)]`.

## Build & deploy

```powershell
rustup target add i686-pc-windows-msvc
cargo build --release --target i686-pc-windows-msvc
Copy-Item target\i686-pc-windows-msvc\release\wmode_rs.dll <ChaosLauncher>\Plugins\wmode-rs.bwl
```
(plus a `scripts\deploy.ps1` wrapping this — arrives at step 14)
