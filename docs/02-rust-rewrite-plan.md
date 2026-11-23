# Plan: wmode-rs — Rust rewrite of W-MODE v1.02

Companion to `01-decompilation-findings.md`. Deliverable: a Rust cdylib that replaces `wmode.bwl`
as a ChaosLauncher (BWL4) plugin for StarCraft 1.16.1, feature-equal to the original
**except** window scaling: ALT+- / ALT+= walk a scale ladder `[1.0 .. 2.0]` and ALT+F9
resets to 1x, instead of the original doublesize toggle.

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
  original\                                            # original third-party drop - untouched
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

## Window scaling (the new feature)

```
scale.rs: const SCALES: [f32; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];  idx: AtomicUsize
```
ALT+- and ALT+= step the ladder down/up (clamped at the ends), ALT+F9 resets to 1x. New
client size `round(game_w * s) × round(game_h * s)`; `SetWindowPos` sized via
`AdjustWindowRectEx`; top left clamped to the visible desktop. Persist to `wmode.ini` when
`SaveScale=1` (new key, default 1). All other hotkeys (ALT+F1 clip, F10 move, F11 topmost,
F12 caption controls) and their ini keys exactly per `WModeReadme.txt` /
`docs/01-decompilation-findings.md` ini table.

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
| 12 | `MaxFps` throttle on the unlock→repaint path                                                                   | fps cap observable                                                                      |
| 13 | `DisableControls` extras (ALT+F4 block, screensaver), desktop clamp                                            | manual checklist clean vs original                                                      |
| 14 | `scripts\deploy.ps1` (build + copy `.bwl` into ChaosLauncher Plugins) + final parity checklist                 | one-command deploy; checklist signed off                                                |
| 15 | plugin list info: version in the name, full hotkey description                                               | launcher plugin page shows both                                                          |
| 16 | custom narrow black caption with icon buttons (clip state visible), original look                             | caption matches the original screenshot, buttons toggle features                        |

## Step notes (decisions taken during implementation)

- Step 2: the StarCraft directory is found via the registry Game Path
  (`HKCU\Software\Battle.net\Configuration`) first, else derived from this DLL's own location
  (`<sc>\Chaoslauncher\Plugins\<name>.bwl`, three parents up). No `hinst` is stored: the module
  handle is resolved on demand with `GetModuleHandleExA(FROM_ADDRESS)`, keeping `DllMain` a
  pure no-op. Visibility inside the crate is plain `pub`, not `pub(crate)`.
- Step 2: the ini template is the verbatim upstream sample (CRLF, all comments) plus the new
  `SaveScale=1` key. Existing ini files are never overwritten by OpenConfig.
- Step 3: launcher-facing exports must be `extern "C"` (cdecl), not `extern "system"` (stdcall)
  on x86. stdcall made ChaosLauncher (a Delphi app) crash on plugin query (APPCRASH, exception
  `0x0eedfade`) because the callee popped arguments the caller expected to pop. `extern "system"`
  remains correct for OS-contract entry points (`DllMain`) and later for Win32 hook replacements.
- Step 3: injection outcome is logged to `%TEMP%\wmode-rs.log` (module path, `LoadLibraryA` exit
  code). The remote thread's exit code is the HMODULE inside the game process, a stronger proof
  of injection than module enumeration.
- Step 3 (verification note): 64-bit PowerShell and `tasklist /m` cannot enumerate modules of
  the 32-bit StarCraft process (WOW64 gap, empty `.Modules`). Verify from 32-bit PowerShell:
  `& $env:windir\SysWOW64\WindowsPowerShell\v1.0\powershell.exe -NoProfile -Command
  "(Get-Process StarCraft).Modules | Where-Object ModuleName -like 'wmode*'"`.

- Step 4: poll kept, `CreateWindowExA` IAT hook not needed (window reliably found within one
  poll cycle after resume). The init thread is spawned only when the host EXE is
  `StarCraft.exe` (`GetModuleFileNameA(NULL)` name check): the launcher also loads the plugin
  into itself, and its copy must stay dormant, otherwise both processes race for the same
  game window (duplicate HWND finds in the log).

- Step 5: import names are read from the original name table (OriginalFirstThunk), not the
  IAT, so entries stay correct after loader resolution. `patch_import` flips page protection
  around the write and restores it. Forwarding design (decided): before patching, read and
  save the slot's current value into an `Old`-style static, same shape as BWAPI's
  `*Old` pointers (WMode.cpp:29-34). While windowing is inactive (startup, or if a disable
  path is ever added) every hook forwards to the saved original with unchanged convention
  and arguments.

- Step 6 (finding): StarCraft.exe imports EVERYTHING from storm.dll by ordinal (verified by
  dumping its import table), and storm.dll exports by ordinal only (zero names, ordinal base
  101). Name-based IAT lookup therefore finds no SDraw functions. Ordinals per BWAPI
  CodePatch.cpp:73-75: `SDrawLockSurface` = 350, `SDrawUnlockSurface` = 356,
  `SDrawUpdatePalette` = 357. All three resolve in the live game process (verified in the
  runtime log), so the IAT patch covers the palette hook too. An earlier static file dump
  suggested 357 was absent - the runtime check wins, no direct storm.dll export patch needed.
- Step 6: `iat.rs` gained ordinal-aware walking (`ImportEntry.ordinal`), `find_import_ordinal`
  and `patch_import_ordinal`. `patch_import*` now return the previous slot value (the saved
  original to forward to) instead of a count.
- Step 6: the window subclass uses `SetWindowLongA(GWL_WNDPROC)` (the x86-safe flavor) and a
  pure pass-through proc; the first 30 messages are logged as a one-shot diagnostic (atomic
  budget), then it stays silent and forwards via `CallWindowProcA`.
- Step 6: the patch unit test moved to a synthetic in-memory PE image: patching the test
  binary's live IAT raced with the harness threads and crashed the test process.

- Step 7 (finding): the SDraw hooks must NEVER forward to storm while windowed. The first
  forwarded call makes storm lazily initialize real DirectDraw fullscreen-exclusive and flip
  the desktop mode. The whole fake-DirectDraw layer exists to keep storm satisfied without it.
- Step 7: real DirectDraw is never created. `DirectDrawCreate` is intercepted via
  GetProcAddress/LoadLibraryA IAT hooks (storm.dll, its child libraries like smackw32.dll,
  and the game exe) and returns fake COM objects (23-slot DirectDraw vtable, 35-slot surface
  vtable, arity-stubbed). The storm DirectDraw object slot (storm base + RVA 0x5E664) is
  replaced in place; the storm module is resolved with
  `GetModuleHandleExA(FROM_ADDRESS | UNCHANGED_REFCOUNT)` off the saved original
  `SDrawLockSurface` address, so no GetModuleHandle-by-name races with the loader.
- Step 7: the launcher side sets the storm registry flag `Window Mode=1`
  (HKCU\Software\Blizzard Entertainment\Internal) in ApplyPatchSuspended before injecting -
  storm then creates a plain cooperative window instead of a fullscreen exclusive one - and
  restores the previous value in ApplyPatch. Readiness is confirmed cross-process by waiting
  for the `res: hooks installed` marker in the shared `%TEMP%\wmode-rs.log`. A mode guard
  additionally IAT-patches ChangeDisplaySettings* inside ddraw.dll.
- Step 8: the game positions and clips the cursor in a 640x480 space rooted at the desktop
  origin. The user32 trio is translated onto the client area with game/client scaling built
  in (ready for 1.5x/2x). Two shapes arrive: raw game rects (0,0,640,480) and pre-translated
  screen rects anchored at the client origin (on refocus) - the hook detects the anchor and
  strips it, so both translate identically. The last game clip rect is remembered and
  re-applied after window moves (on deactivate the game unclips, then there is nothing to
  re-apply).
- Step 8: window sizing goes through `AdjustWindowRectEx` (style flags in, outer size out).
  Measuring the live frame was a trap: `SetWindowLongA(style)` does not recalculate the
  frame until a SetWindowPos, so a just-restyled window measured a zero border and an
  unchanged outer size sends no WM_SIZE at all. A WM_SIZE corrector still reverts storm's
  own outer-640x480 resizes on every activation.
- Step 8 (deferred to step 10): WM_MOUSEMOVE lParam scaling (the FixPoints part) is not
  needed while client == game size and lands together with the scale cycle.

- Step 9: the hotkeys live in the subclass WM_SYSKEYDOWN arm. windows-sys type widths decided
  the matching style: VK_* constants are u16 (matched via `wparam as u16`), HT_*/SC_* are u32
  (`wparam as u32`). Every toggle logs a `hotkey:` line.
- Step 9: the cursor clip switch defaults to OFF per the sample ini. While off, the game's own
  ClipCursor requests are stored but answered with NULL. ALT inverts the switch for as long as
  it is held (XOR); releasing ALT re-applies the clip immediately - the readme's deferred
  activation (only when the cursor returns over the window) is a recorded deviation.
- Step 9: ALT+F10 swallows WM_NCLBUTTONDOWN on HTCAPTION, ALT+F11 is a plain
  TOPMOST/NOTOPMOST SetWindowPos, ALT+F12 greys SC_CLOSE/SC_MAXIMIZE/SC_MINIMIZE via
  EnableMenuItem and swallows WM_SYSCOMMAND SC_CLOSE plus NC clicks on the caption buttons.
  The screensaver half of the original ALT+F12 waits for step 13.

- Step 10 (ruling): the scale ladder is [1.0, 1.25, 1.5, 1.75, 2.0] walked by ALT+-
  and ALT+= (clamped at the ends), ALT+F9 resets to 1x. An earlier [1x, 1.5x, 2x]
  cycle and a wider ladder with 0.5/0.75 were both dropped by user ruling during the
  step. Ladder keys are the physical VK_OEM_MINUS / VK_OEM_PLUS.
- Step 10: mouse message coordinates are rescaled from client pixels to game units
  (WM_MOUSEFIRST..WM_MOUSELAST except the wheel, whose lParam is screen space).
- Step 10: the cursor clip moved onto the original's model. While windowed every
  game ClipCursor request is swallowed and the clip belongs to the ALT+F1 switch
  alone (confined to the client box, or free). Two reliability findings: the game
  never calls GetCursorPos, and it swallows some ALT keyups, so key events cannot
  drive the inversion - the physical state is read via GetAsyncKeyState and synced
  once per frame from the SDrawUnlockSurface hook. The ALT+F1 toggle itself applies
  the switch directly, ignoring the inversion, so it lands the moment it is pressed.

- Step 11: config parsing is byte-level and case-insensitive; unknown keys and
  comment lines survive both parsing and saving. The original's compact ini
  (with its DblSized keys) is read and rewritten in place, coexistence verified.
- Step 11: the exit save rides DLL_PROCESS_DETACH. The game exits via
  ExitProcess and delivers neither WM_CLOSE nor WM_DESTROY (verified in the
  log), so process detach is the only exit notification an injected DLL sees -
  the original must rely on it too. The client position is cached into atomics
  on every WM_MOVE (minimized windows report -32000 and are ignored), and the
  detach save touches only the file system and those statics, no window
  queries under the loader lock. A save-on-every-change variant was tried and
  reverted by user ruling: saving happens strictly at exit.
- Step 11: over the caption and frame the system arrow is shown again
  (WM_NCMOUSEMOVE shows, WM_MOUSEMOVE hides, one ShowCursor call per state
  change), because the game hides the OS cursor and draws its own only inside
  the client.

- Step 12: MaxFps rides the unlock-to-repaint path (tick check, default 100,
  minimum 30). GetCursorPos became focus-aware per user spec: focused = real
  position scaled into game units and clamped to the game rect (edge scroll
  never stalls), unfocused = always the center (the game keeps polling while
  deactivated and a stray mouse elsewhere must not scroll its map; the click
  that raises the window focuses it before the button message lands).
- Step 12 (follow-up commit f05d3a3): the plain-ALT reactions were dropped by
  user ruling (useless feature). ALT no longer inverts the clip switch and no
  longer forces real coordinates through GetCursorPos - this supersedes the
  ALT-inversion notes of steps 9/10, whose per-frame GetAsyncKeyState sync
  went away with them. apply_switch and refresh_clip merged into one
  refresh_clip. Diagnostics added: `mouse:` lines sample screen (absolute),
  client-relative and game coordinates plus focus/clip flags (every 8th
  changed position, budget 400 lines), `clip:` lines log every confine and
  release with the rectangle.
- Step 12 (fix): Windows itself releases ClipCursor confinement when the
  window deactivates, and nothing restores it on the way back. WM_ACTIVATE
  now reapplies the clip on every activation while the switch is on - before,
  Alt+Tab away and back left the cursor free forever with the switch still
  ON. Log proof: one `clip: confined` at the hotkey, no second one after the
  Alt+Tab round trip.
- Step 12 (external finding): "cursor never reaches the window edge"
  reproduced only with the BWAPI plugin co-injected (BWAPI hooks the same
  cursor trio and confines to its own rectangle); clean without BWAPI. See
  the Risks entry.

- Step 13 (ruling): no mouse resize. The original does not allow resizing
  (the readme documents only the doublesize toggle), so the WM_SIZING
  min-size/snap machinery - lifted from BWAPI's WMode, where resize is a
  feature - was removed before commit. The window keeps its size from the
  scale ladder alone, and correct_client_size already reverts any foreign
  resize attempt, which is also why free dragging never worked. What step
  13 ships instead: the screensaver half of DisableControls (readme: while
  the controls are disabled, the screensaver is off while the StarCraft
  window is in foreground) via SPI_GET/SETSCREENSAVEACTIVE on activation,
  F12 toggling and process exit, plus the ALT+F4 block and the desktop
  clamp landed earlier.
- Step 13 (verification): scenario run clean. The screensaver log lines
  stay silent while the system screensaver is off (SPI_GETSCREENSAVEACTIVE
  returns 0, nothing to disable or restore) - verifying the guard visibly
  needs a screensaver enabled in Windows first.

- Step 15: the plugin list name now carries the crate version
  (`concat!` with `CARGO_PKG_VERSION`, so it follows Cargo.toml), and the
  description lists the full hotkey set plus the ini pointer. Steps 16+
  (custom caption, icon buttons) were added to the step table by user
  request after the 0-14 ladder closed.

- Step 16: custom caption in the original's style. The window style became
  `WS_POPUP | WS_SYSMENU`, `WM_NCCALCSIZE` carves a 18 px bar plus a 3 px
  border (constants `BAR_H`/`FRAME` in render.rs; outer-size math is now
  constant-based, `AdjustWindowRectEx` dropped), `WM_NCPAINT` paints the
  red frame, black bar, title and a button row (clip, move, topmost,
  controls, minimize, close, codes 0x100+), `WM_NCHITTEST` serves the bar
  as HTCAPTION and the buttons, button clicks call the same toggle
  functions as the hotkeys, glyphs carry per-switch on/off colors and the
  whole row goes grey and inert while the caption controls are off.
- Step 16 (cursor battles, all verified against the log): the game hides
  the OS cursor in three ways, each needing its own counter. (1) repeated
  `ShowCursor(FALSE)` - countered by saturating the counter instead of a
  single step. (2) `SetCapture` in menus: captured moves over the caption
  arrive as plain `WM_MOUSEMOVE`, nonclient messages never come - countered
  by deciding from the physical cursor position (window rect vs client
  rect), not from the message flavor, on every mouse message and at the
  end of every frame (`SDrawUnlockSurface` tail calls arrow_frame_check,
  so the arrow over the frame wins within one frame). (3) `SetCursor(NULL)`
  on focus switches, which no ShowCursor amount can fix - countered by
  intercepting `WM_SETCURSOR` over the frame and by `SetCursor(IDC_ARROW)`
  in the frame check. On `WM_ACTIVATE` the arrow state is re-derived from
  the cursor position.
- Step 16 (minimize/restore): Windows parks a minimized window at -32000
  with stub geometry (client 1232x26). correct_client_size skips iconic
  windows, mapping() rejects them, GetCursorPos feeds the game its center
  while minimized, and SetCursorPos is swallowed both when unmeasurable
  and while unfocused (the game recenters the physical cursor on
  deactivate/refocus, which warped it into the screen corner or yanked it
  out of the focused app). The hunt-only diagnostics (mouse/setcursor/
  arrow sample logs) were removed before commit.

## Risks / open questions

- **BWAPI interplay**: BWAPI's own W-Mode detects Xen W-Mode via `WS_SYSMENU` in the window
  style and defers (WMode.cpp:420,438,576). Our window style must keep `WS_SYSMENU`
  (`WS_OVERLAPPEDWINDOW` does) so co-injection with BWAPI stays consistent. Confirmed live:
  with BWAPI co-injected the physical cursor stops short of the window edge (BWAPI confines
  and translates through its own hooks, invisible to ours) - clean without BWAPI. Document
  "don't enable both".
- **Loader lock**: init strictly on a spawned thread; hooks installed only after window exists.
- **HALFTONE StretchDIBits cost** at 2x on weak machines — `MaxFps` default 100 covers it.
- **Windows 11 DPI**: declare DPI-aware via manifest? Original predates it; test raw first.
- **Cursor clip on ALT release (known deviation, minor)**: the readme defers re-activating the
  clip until the cursor comes back over the window when ALT is released while the cursor is
  outside ("it will be actually activated only when you move the cursor back"). We re-apply
  the clip immediately, so the cursor is pulled back inside by the system. Deferred-activation
  machinery was tried and dropped as non-obvious behavior.
- **Toolchain**: MSVC link target for x86 — needs VS BuildTools x86 libs installed; fallback
  `i686-pc-windows-gnu` if not.
- **Calling conventions**: everything `extern "system"`; storm hooks are `__stdcall` (`system`
  on x86 == stdcall). Structure layouts `#[repr(C)]`.

## Sound on focus loss

The game sound mutes when the window is unfocused. This is parity with upstream v1.02 (its
readme admits it in known limitations), not a regression. A workaround attempt (COM wrapper
granting DSBCAPS_GLOBALFOCUS via the GetProcAddress interception) crashed three times and
was reverted; full account with logs, addresses and retry advice lives in
`docs/03-sound-focus-loss-postmortem.md`. The MuteNotFocused ini extension was dropped as
redundant under OS muting.

## Parity checklist (step 14)

Feature-by-feature against `original\WModeReadme.txt`:

| readme feature | wmode-rs | status |
|----------------|----------|--------|
| ChaosLauncher/BWL4 plugin, name/description/config dialog | bwl.rs exports, steps 1-3 | done |
| `wmode.ini` full key set, Save* switches, save on exit | ini.rs, step 11 | done |
| Windowed 8-bpp render, palette, no desktop mode change | render.rs + fake DirectDraw, step 7 | done |
| `MaxFps` frame cap (default 100, min 30) | unlock-path throttle, step 12 | done |
| ALT+F1 cursor clip (+ release behavior) | step 9, reworked 12 | done, deviation: immediate re-clip on ALT release vs readme's deferred activation (step 9 note) |
| ALT+F10 window move toggle | step 9 | done |
| ALT+F11 always-on-top toggle | step 9 | done |
| ALT+F12 caption controls + ALT+F4 block + screensaver off while focused | steps 9, 13 | done |
| ALT+F9 doublesize | scale ladder ALT+-/ALT+=/ALT+F9 [1.0..2.0] | declared project difference |
| Sound mutes when unfocused | unchanged (OS-level) | parity with v1.02, see docs/03 |
| Caption icon buttons (clickable, grey with controls off) | not implemented | open item, optional |
| Mouse resize of the window | none | parity: original has none either (step 13 ruling) |
| Windows 11 DPI awareness | not declared | untested; original predates it |

## Build & deploy

```powershell
rustup target add i686-pc-windows-msvc
powershell -ExecutionPolicy Bypass -File scripts\deploy.ps1            # build + copy to Plugins
powershell -ExecutionPolicy Bypass -File scripts\deploy.ps1 -ClearLog  # also start a fresh %TEMP% log
```
`deploy.ps1` defaults to `D:\scai\scbw\Chaoslauncher\Plugins` (`-PluginsDir` overrides); the game
must not be running while deploying, Windows locks a loaded plugin.
