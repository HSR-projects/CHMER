@echo off
setlocal

if "%LOCALAPPDATA%"=="" (
  echo error: LOCALAPPDATA is not set
  exit /b 1
)

set "INSTALL_DIR=%LOCALAPPDATA%\CHMER\bin"
set "ASSET_DIR=%LOCALAPPDATA%\CHMER\assets"
set "TARGET=%INSTALL_DIR%\chmer.exe"

if exist "%TARGET%" (
  del /f /q "%TARGET%" >nul 2>&1
)
if exist "%INSTALL_DIR%\chmer.png" (
  del /f /q "%INSTALL_DIR%\chmer.png" >nul 2>&1
)
if exist "%ASSET_DIR%" (
  rmdir /s /q "%ASSET_DIR%" >nul 2>&1
)

for /f "usebackq delims=" %%P in (`powershell -NoProfile -Command "[Environment]::GetEnvironmentVariable('Path','User')"`) do set "USERPATH=%%P"
if defined USERPATH (
  powershell -NoProfile -Command "$p=[Environment]::GetEnvironmentVariable('Path','User');$parts=$p -split ';' | Where-Object { $_ -and ($_ -ne '%INSTALL_DIR%') };[Environment]::SetEnvironmentVariable('Path',($parts -join ';'),'User')"
)

echo CHMER removed from %INSTALL_DIR%.
echo Open a new terminal for PATH changes.
exit /b 0
