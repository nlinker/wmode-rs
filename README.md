# wmode-rs

Rust rewrite of W-MODE v1.02 (XeNotRoN, 2008) — windowed-mode plugin for StarCraft: Brood War 1.16.1. Loads as a ChaosLauncher (BWL4) plugin. Differences from the original: window scale ladder ALT+- / ALT+= (1.0-2.0, ALT+F9 resets) instead of the doublesize toggle, and a custom narrow black caption with clickable state buttons.

![wmode.avif](docs/wmode.avif)

```
original\   original third-party drop (wmode.bwl, WModeReadme.txt) - untouched
scripts\    deploy.ps1 (build + install the plugin), decompile-{wmode,storm}.ps1 (Ghidra headless dumps)
docs\       findings + rewrite plan (step-by-step)
src\        Rust source (cdylib, target i686-pc-windows-msvc)
```

Status: steps 0-14 complete - see `docs/02-rust-rewrite-plan.md`.

## Build & deploy

Just build the dll:
```powershell
cargo build --release --target i686-pc-windows-msvc
```

Build and copy to the destination:
```powershell
powershell -ExecutionPolicy Bypass -File scripts\deploy.ps1            # build + copy to Plugins
powershell -ExecutionPolicy Bypass -File scripts\deploy.ps1 -ClearLog  # also start a fresh %TEMP%\wmode-rs.log
```

Builds the release DLL (`i686-pc-windows-msvc`) and copies it as `wmode-rs.bwl` into the ChaosLauncher Plugins folder -
default `D:\scai\scbw\Chaoslauncher\Plugins`, override with `-PluginsDir`. The game must not be running while deploying:
Windows locks a loaded plugin file.

## Parity checklist

Feature-by-feature against `original\WModeReadme.txt`:

| readme feature                                                          | wmode-rs                                   | status                                                                                          |
|-------------------------------------------------------------------------|--------------------------------------------|-------------------------------------------------------------------------------------------------|
| ChaosLauncher/BWL4 plugin, name/description/config dialog               | bwl.rs exports, steps 1-3                  | done                                                                                            |
| `wmode.ini` full key set, Save* switches, save on exit                  | ini.rs, step 11                            | done                                                                                            |
| Windowed 8-bpp render, palette, no desktop mode change                  | render.rs + fake DirectDraw, step 7        | done                                                                                            |
| `MaxFps` frame cap (default 100, min 30)                                | unlock-path throttle, step 12              | done                                                                                            |
| ALT+F1 cursor clip (+ release behavior)                                 | step 9, reworked 12                        | done, deviation: immediate re-clip on ALT release vs readme's deferred activation (step 9 note) |
| ALT+F10 window move toggle                                              | step 9                                     | done                                                                                            |
| ALT+F11 always-on-top toggle                                            | step 9                                     | done                                                                                            |
| ALT+F12 caption controls + ALT+F4 block + screensaver off while focused | steps 9, 13                                | done                                                                                            |
| ALT+F9 doublesize                                                       | scale ladder ALT+-/ALT+=/ALT+F9 [1.0..2.0] | declared project difference                                                                     |
| Sound mutes when unfocused                                              | unchanged (OS-level)                       | parity with v1.02, see docs/03                                                                  |
| Caption icon buttons (clickable, grey with controls off)                | custom caption, step 16                    | done: narrow black bar, red frame, state buttons, click = toggle                                |
| Mouse resize of the window                                              | none                                       | parity: original has none either (step 13 ruling)                                               |
| Windows 11 DPI awareness                                                | not declared                               | untested; original predates it                                                                  |
| Cursor over the caption                                                 | position-based arrow logic, step 16        | done: survives Alt+Tab, game focus switches and mouse capture                                   |

## Reverse-engineering tooling

Requires Ghidra 12.1.4 + JDK 21 installed (auto-detected under `D:\tools`, or pass `-Ghidra`/`-Jdk`;
the scripts print download instructions when missing). Full story of the layered packing:
`docs/04-xmod-unpack-expedition.md`.

```powershell
# layer 1 (the packer stub inside wmode.bwl) and storm.dll
powershell -ExecutionPolicy Bypass -File scripts\decompile-wmode.ps1    # -> decomp\wmode.dll.c
powershell -ExecutionPolicy Bypass -File scripts\decompile-storm.ps1    # -> decomp\storm.dll.c

# layer 2 dump (the "XeN'z XMOD loader stub"): the sacrificial host loads
# wmode.bwl and dumps the unpacked image
cargo run --release --target i686-pc-windows-msvc --bin unpack-wmode     # -> gproj\wmode-unpacked.dll
Copy-Item gproj\wmode-unpacked.dll original\wmode-unpacked.dll           # optional convenience copy

# layer 2 decompile: the same headless pattern, pointed at the dump
& (Get-ChildItem D:\tools\ghidra_*\support\analyzeHeadless.bat)[0] `
  gproj wmode-unpacked -import gproj\wmode-unpacked.dll `
  -scriptPath scripts -postScript DecompileAll.java decomp\wmode-unpacked.dll.c
```
