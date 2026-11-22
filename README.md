# wmode-rs

Rust rewrite of W-MODE v1.02 (XeNotRoN, 2008) — windowed-mode plugin for StarCraft: Brood War 1.16.1. Loads as a ChaosLauncher (BWL4) plugin. New: ALT+F9 cycles scale 1x → 1.5x → 2x instead of toggling 2x.

```
original\   original third-party drop (wmode.bwl, WModeReadme.txt) - untouched
scripts\    decompile-wmode.ps1 (Ghidra headless dump; needs Ghidra 12.1.4 + JDK 21 in D:\tools)
docs\       findings + rewrite plan (step-by-step)
src\        Rust source (cdylib, target i686-pc-windows-msvc)
```

Status: planned — see `docs/02-rust-rewrite-plan.md` (steps 0–14, one review + commit each).
Getting started: `docs/00_get-started.md`.
