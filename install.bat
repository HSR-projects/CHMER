@echo off
setlocal

set "SOURCE_DIR=%~1"
if "%SOURCE_DIR%"=="" set "SOURCE_DIR=%~dp0"

set "INSTALL_DIR=%CHMER_INSTALL_DIR%"
if "%INSTALL_DIR%"=="" set "INSTALL_DIR=%LOCALAPPDATA%\CHMER\bin"
set "ASSET_DIR=%CHMER_ASSET_DIR%"
if "%ASSET_DIR%"=="" set "ASSET_DIR=%LOCALAPPDATA%\CHMER\assets"
set "WITH_ASSETS=%CHMER_WITH_ASSETS%"
if "%WITH_ASSETS%"=="" set "WITH_ASSETS=1"

if not exist "%SOURCE_DIR%\chmer.exe" if not exist "%SOURCE_DIR%\bin\chmer.exe" (
  echo error: chmer.exe not found in "%SOURCE_DIR%"
  exit /b 1
)

if not exist "%INSTALL_DIR%" mkdir "%INSTALL_DIR%" >nul 2>&1
if not exist "%ASSET_DIR%" mkdir "%ASSET_DIR%" >nul 2>&1

if exist "%SOURCE_DIR%\chmer.exe" (
  copy /y "%SOURCE_DIR%\chmer.exe" "%INSTALL_DIR%\chmer.exe" >nul
) else (
  copy /y "%SOURCE_DIR%\bin\chmer.exe" "%INSTALL_DIR%\chmer.exe" >nul
)

if exist "%SOURCE_DIR%\chmer.png" (
  copy /y "%SOURCE_DIR%\chmer.png" "%INSTALL_DIR%\chmer.png" >nul
)

if "%WITH_ASSETS%"=="1" (
  if exist "%SOURCE_DIR%\chmer-assets.zip" (
    powershell -NoProfile -Command "Expand-Archive -Path '%SOURCE_DIR%\chmer-assets.zip' -DestinationPath '%ASSET_DIR%' -Force" >nul 2>&1
  )
)

powershell -NoProfile -Command "$p=[Environment]::GetEnvironmentVariable('Path','User');if([string]::IsNullOrWhiteSpace($p)){[Environment]::SetEnvironmentVariable('Path','%INSTALL_DIR%','User')}elseif($p -notlike '*%INSTALL_DIR%*'){[Environment]::SetEnvironmentVariable('Path',($p + ';%INSTALL_DIR%'),'User')}" >nul 2>&1

echo CHMER install complete.
echo Installed: %INSTALL_DIR%\chmer.exe
echo Open a new terminal for PATH changes.
exit /b 0
