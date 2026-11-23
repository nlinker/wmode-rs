# wmode-rs

Rust rewrite of W-MODE v1.02 (XeNotRoN, 2008) — windowed-mode plugin for StarCraft: Brood War 1.16.1. Loads as a ChaosLauncher (BWL4) plugin. New: ALT+F9 cycles scale 1x → 1.5x → 2x instead of toggling 2x.

```
original\   original third-party drop (wmode.bwl, WModeReadme.txt) - untouched
scripts\    deploy.ps1 (build + install the plugin), decompile-{wmode,storm}.ps1 (Ghidra headless dumps)
docs\       findings + rewrite plan (step-by-step)
src\        Rust source (cdylib, target i686-pc-windows-msvc)
```

Status: steps 0-14 complete - see `docs/02-rust-rewrite-plan.md`.
Getting started: `docs/00_get-started.md`.

## Build & deploy

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
| Caption icon buttons (clickable, grey with controls off)                | not implemented                            | open item, optional                                                                             |
| Mouse resize of the window                                              | none                                       | parity: original has none either (step 13 ruling)                                               |
| Windows 11 DPI awareness                                                | not declared                               | untested; original predates it                                                                  |

