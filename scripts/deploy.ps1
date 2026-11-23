<#
.SYNOPSIS
  Build wmode-rs and deploy it as the ChaosLauncher plugin.

.DESCRIPTION
  Runs cargo build --release for i686-pc-windows-msvc and copies the DLL
  into the ChaosLauncher Plugins folder as wmode-rs.bwl. The game must not
  be running, Windows locks a loaded plugin file. With -ClearLog the
  %TEMP%\wmode-rs.log is removed too, so the next launch starts fresh.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\deploy.ps1

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\deploy.ps1 -ClearLog

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\deploy.ps1 -PluginsDir "C:\other\Chaoslauncher\Plugins"
#>
[CmdletBinding()]
param(
  [string] $PluginsDir = 'D:\scai\scbw\Chaoslauncher\Plugins',
  [switch] $ClearLog
)
$ErrorActionPreference = 'Stop'

$repo = Split-Path $PSScriptRoot -Parent
Set-Location $repo

cargo build --release --target i686-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

$dll = Join-Path $repo 'target\i686-pc-windows-msvc\release\wmode_rs.dll'
if (-not (Test-Path $dll)) { throw "build output not found: $dll" }

$dst = Join-Path $PluginsDir 'wmode-rs.bwl'
Copy-Item $dll $dst -Force
if ($ClearLog) {
  Remove-Item "$env:TEMP\wmode-rs.log" -ErrorAction SilentlyContinue
}

Get-Item $dst | Select-Object FullName, Length, LastWriteTime
