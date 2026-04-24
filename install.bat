@echo off
setlocal

set "SCRIPT_DIR=%~dp0"
set "PS_SCRIPT=%SCRIPT_DIR%install.ps1"

if not exist "%PS_SCRIPT%" (
  echo error: install.ps1 not found beside install.bat
  exit /b 1
)

powershell -NoProfile -ExecutionPolicy Bypass -File "%PS_SCRIPT%"
set "RC=%ERRORLEVEL%"
if not "%RC%"=="0" (
  echo error: installation failed with exit code %RC%
  exit /b %RC%
)

echo.
echo CHMER install complete.
echo If command is not found, open a new terminal.
exit /b 0
