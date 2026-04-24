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

function Find-Binary([string]$root) {
  $direct = Join-Path $root "chmer.exe"
  if (Test-Path $direct) { return $direct }
  $bin = Join-Path $root "bin\chmer.exe"
  if (Test-Path $bin) { return $bin }
  $found = Get-ChildItem -Path $root -Recurse -Filter "chmer.exe" -File | Select-Object -First 1
  if ($null -eq $found) { return $null }
  return $found.FullName
}

$arch = Get-Arch
$archive = Get-ArchiveName -arch $arch
$url = "$ReleaseBase/$archive"
$logoUrl = "$ReleaseBase/chmer.png"
$assetUrl = "$ReleaseBase/chmer-assets.zip"

Write-Host "CHMER installer" -ForegroundColor Green
Write-Host "Logo: chmer.png (included in release assets/docs)"
Write-Host "Platform: windows-$arch"
Write-Host "Download: $url"

$tmp = Join-Path $env:TEMP ("chmer-install-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $tmp | Out-Null
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $AssetDir | Out-Null

$zipPath = Join-Path $tmp $archive
$outPath = Join-Path $tmp "unpack"
New-Item -ItemType Directory -Force -Path $outPath | Out-Null

Invoke-WebRequest -Uri $url -OutFile $zipPath
Expand-Archive -Path $zipPath -DestinationPath $outPath -Force

$bin = Find-Binary -root $outPath
if ($null -eq $bin) {
  throw "chmer.exe not found in downloaded archive"
}

Copy-Item -Force $bin (Join-Path $InstallDir "chmer.exe")

try {
  Invoke-WebRequest -Uri $logoUrl -OutFile (Join-Path $InstallDir "chmer.png")
} catch {
  # optional logo download
}

if ($WithAssets -eq "1") {
  try {
    $assetZip = Join-Path $tmp "chmer-assets.zip"
    Invoke-WebRequest -Uri $assetUrl -OutFile $assetZip
    Expand-Archive -Path $assetZip -DestinationPath $AssetDir -Force
    Write-Host "Assets installed: $AssetDir (images/text/emoji packs)" -ForegroundColor Green
  } catch {
    Write-Host "Assets pack not found in release (skipping): $assetUrl"
  }
}

Write-Host ""
Write-Host "Installed: $InstallDir\chmer.exe" -ForegroundColor Green
Write-Host "Run: $InstallDir\chmer.exe"
Write-Host "Asset dir: $AssetDir"

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallDir*") {
  $newPath = if ([string]::IsNullOrWhiteSpace($userPath)) { $InstallDir } else { "$userPath;$InstallDir" }
  [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
  Write-Host "User PATH updated." -ForegroundColor Green
}
