$ErrorActionPreference = "Stop"

param(
  [string]$ReleaseBase = $env:CHMER_RELEASE_BASE,
  [string]$InstallDir = $env:CHMER_INSTALL_DIR,
  [string]$AssetDir = $env:CHMER_ASSET_DIR,
  [string]$WithAssets = $env:CHMER_WITH_ASSETS
)

if ([string]::IsNullOrWhiteSpace($ReleaseBase)) {
  $ReleaseBase = "https://github.com/HSR-projects/chmer/releases/latest/download"
}
if ([string]::IsNullOrWhiteSpace($InstallDir)) {
  $InstallDir = Join-Path $env:LOCALAPPDATA "CHMER\bin"
}
if ([string]::IsNullOrWhiteSpace($AssetDir)) {
  $AssetDir = Join-Path $env:LOCALAPPDATA "CHMER\assets"
}
if ([string]::IsNullOrWhiteSpace($WithAssets)) {
  $WithAssets = "1"
}

function Get-Arch {
  $a = $env:PROCESSOR_ARCHITECTURE
  if ($a -eq "AMD64") { return "x86_64" }
  if ($a -eq "ARM64") { return "aarch64" }
  throw "Unsupported Windows architecture: $a"
}

function Get-ArchiveName([string]$arch) {
  switch ($arch) {
    "x86_64" { return "chmer-windows-x86_64.zip" }
    "aarch64" { return "chmer-windows-aarch64.zip" }
    default { throw "No precompiled Windows package for arch: $arch" }
  }
}

$arch = Get-Arch
$archive = Get-ArchiveName -arch $arch
$url = "$ReleaseBase/$archive"

Write-Host "CHMER installer" -ForegroundColor Green
Write-Host "Platform: windows-$arch"
Write-Host "Download: $url"

$tmp = Join-Path $env:TEMP ("chmer-install-" + [guid]::NewGuid().ToString("N"))
$zipPath = Join-Path $tmp $archive
$outPath = Join-Path $tmp "unpack"

New-Item -ItemType Directory -Force -Path $tmp | Out-Null
New-Item -ItemType Directory -Force -Path $outPath | Out-Null

Invoke-WebRequest -Uri $url -OutFile $zipPath
Expand-Archive -Path $zipPath -DestinationPath $outPath -Force

$bat = Get-ChildItem -Path $outPath -Recurse -Filter "install.bat" -File | Select-Object -First 1
if ($null -eq $bat) {
  throw "install.bat not found in downloaded archive. Include install.bat in Windows release package."
}

$env:CHMER_INSTALL_DIR = $InstallDir
$env:CHMER_ASSET_DIR = $AssetDir
$env:CHMER_WITH_ASSETS = $WithAssets

Write-Host "Delegating installation to: $($bat.FullName)"
& cmd.exe /c "`"$($bat.FullName)`" `"$($bat.Directory.FullName)`""
if ($LASTEXITCODE -ne 0) {
  throw "install.bat failed with exit code $LASTEXITCODE"
}

Write-Host "Installed: $InstallDir\chmer.exe" -ForegroundColor Green
