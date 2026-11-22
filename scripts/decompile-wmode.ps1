<#
.SYNOPSIS
  Headless Ghidra decompilation of wmode.bwl (W-MODE v1.02, 32-bit x86 PE DLL).

.DESCRIPTION
  Copies ..\original\wmode.bwl to gproj\wmode.dll (Ghidra's PE loader sniffs content, but a
  .dll name keeps the import clean), imports it into a local Ghidra project and
  dumps decompiled C for every function to decomp\wmode.dll.c via the bundled
  DecompileAll.java postScript. Also writes decomp\wmode.dll.strings (ASCII runs,
  like `strings -n 5 | awk 'length<300'`) - hotkey hints, wmode.ini keys and
  error messages live there.

  For storm.dll use scripts\decompile-storm.ps1 instead.

  Needs: Ghidra 12.1.4 + JDK 21, looked up in this order:
    1. -Ghidra / -Jdk parameters (or -ToolsRoot for a custom tools folder)
    2. $env:GHIDRA (Ghidra only)
    3. auto-detect under the default tools root D:\tools
       (newest ghidra_*\support\analyzeHeadless.bat, jdk-21*)
  If the tools are missing, the script stops and prints download instructions.

  Idempotent: skips decompilation when decomp\wmode.dll.c already exists and is
  non-empty; -Force redoes it. The original wmode.bwl is never modified.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\decompile-wmode.ps1

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\decompile-wmode.ps1 -Force -ToolsRoot "C:\mytools"
#>
[CmdletBinding()]
param(
  [string] $ToolsRoot = 'D:\tools',
  [string] $Ghidra = '',
  [string] $Jdk = '',
  [switch] $Force
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$repo = Split-Path $PSScriptRoot -Parent      # project root, wherever it lives
Set-Location $repo

# --- tool resolution ------------------------------------------------------------
if (-not $Ghidra) {
  if ($env:GHIDRA) { $Ghidra = $env:GHIDRA }
  elseif (Test-Path $ToolsRoot) {
    $Ghidra = Get-ChildItem $ToolsRoot -Directory -Filter 'ghidra_*' -ErrorAction SilentlyContinue |
              Sort-Object Name -Descending | Select-Object -First 1 | ForEach-Object { $_.FullName }
  }
}
if (-not $Jdk -and (Test-Path $ToolsRoot)) {
  $Jdk = Get-ChildItem $ToolsRoot -Directory -Filter 'jdk-21*' -ErrorAction SilentlyContinue |
         Sort-Object Name -Descending | Select-Object -First 1 | ForEach-Object { $_.FullName }
}

$missing = @()
if (-not $Ghidra -or -not (Test-Path (Join-Path $Ghidra 'support\analyzeHeadless.bat'))) { $missing += 'Ghidra 12.1.4' }
if (-not $Jdk   -or -not (Test-Path (Join-Path $Jdk 'bin')))                               { $missing += 'JDK 21'     }
if ($missing) {
  throw @"
$($missing -join ' and ') not found.
Looked in: $ToolsRoot (ghidra_* / jdk-21*), `$env:GHIDRA, -Ghidra/-Jdk parameters.

Download and extract into $ToolsRoot`:

  # JDK 21 (Adoptium Temurin, Windows x64 zip)
  Invoke-WebRequest https://api.adoptium.net/v3/binary/latest/21/ga/windows/x64/jdk/hotspot/normal/eclipse -OutFile jdk21.zip
  Expand-Archive jdk21.zip "$ToolsRoot"        # renames to jdk-21.<vendor>.<ver> - matches jdk-21*

  # Ghidra 12.1.4 (github.com/NationalSecurityAgency/ghidra/releases)
  Invoke-WebRequest https://github.com/NationalSecurityAgency/ghidra/releases/download/Ghidra_11.3.2_build/ghidra_12.1.4_PUBLIC_20251016.zip -OutFile ghidra.zip
  Expand-Archive ghidra.zip "$ToolsRoot"        # creates ghidra_12.1.4_PUBLIC

Then re-run this script, or pass -Ghidra/-Jdk/-ToolsRoot explicitly.
"@
}

# Ghidra 12.x needs JDK 21; point JAVA_HOME at it ahead of whatever java is on PATH
$env:JAVA_HOME = $Jdk

# --- decompile ------------------------------------------------------------------
$src = Join-Path $repo 'original\wmode.bwl'
if (-not (Test-Path $src)) { throw "wmode.bwl not found at $src (expected under the original\ subdir)" }

New-Item -ItemType Directory -Force gproj, decomp | Out-Null
$dll = Join-Path $repo 'gproj\wmode.dll'
Copy-Item $src $dll -Force                 # never touched again by Ghidra: -import copies into the project

$headless = Join-Path $Ghidra 'support\analyzeHeadless.bat'
$out = Join-Path $repo 'decomp\wmode.dll.c'
if (-not $Force -and (Test-Path $out) -and (Get-Item $out).Length -gt 0) {
  Write-Host "==> decomp\wmode.dll.c already exists ($((Get-Item $out).Length) bytes); pass -Force to redo"
} else {
  # Start-Process file redirection (not `2>&1`) keeps Ghidra's stderr chatter
  # from becoming a terminating error under $ErrorActionPreference = 'Stop'.
  $log = "$repo\decomp\wmode.dll.log"; $err = "$log.err"
  Write-Host '==> decompiling wmode.bwl (minutes; first run also compiles the postScript)'
  $argLine = "gproj wmode -import `"$dll`" -overwrite -scriptPath `"$PSScriptRoot`" -postScript DecompileAll.java `"$out`""
  $p = Start-Process -FilePath $headless -ArgumentList $argLine -NoNewWindow -Wait -PassThru `
        -WorkingDirectory $repo -RedirectStandardOutput $log -RedirectStandardError $err
  Get-Content $err -ErrorAction SilentlyContinue | Add-Content $log
  Remove-Item $err -ErrorAction SilentlyContinue
  if ($p.ExitCode -ne 0) { throw "analyzeHeadless failed (see decomp\wmode.dll.log)" }
  if (-not (Test-Path $out) -or (Get-Item $out).Length -eq 0) { throw "decompile produced no output (see decomp\wmode.dll.log)" }
  Get-Content $log | Select-String 'functions=' | ForEach-Object { Write-Host "    $($_.Line)" }
}

# strings: ASCII runs len>=5, drop >=300 - hotkey/ini/error text for the ALT+F9 hunt
$stringsOut = Join-Path $repo 'decomp\wmode.dll.strings'
$text = [Text.Encoding]::GetEncoding(28591).GetString([IO.File]::ReadAllBytes($dll))
[regex]::Matches($text, '[\x20-\x7E]{5,}') |
  ForEach-Object { if ($_.Value.Length -lt 300) { $_.Value } } |
  Set-Content -Encoding ASCII $stringsOut

Write-Host "done: decomp\wmode.dll.c, decomp\wmode.dll.strings" -ForegroundColor Green
