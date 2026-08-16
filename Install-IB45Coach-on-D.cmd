@echo off
setlocal
set "IB45_ROOT=D:\IB45Coach"
set "TEMP=%IB45_ROOT%\temp"
set "TMP=%IB45_ROOT%\temp"

if not exist "%TEMP%" mkdir "%TEMP%"
set "INSTALLER=%IB45_ROOT%\src-tauri\target\release\bundle\nsis\IB 45 Coach_0.1.0_x64-setup.exe"

if not exist "%INSTALLER%" (
  echo IB 45 Coach installer was not found:
  echo %INSTALLER%
  pause
  exit /b 1
)

echo Installer temporary files are redirected to %TEMP%.
echo When prompted, choose D:\IB45Coach\app as the installation folder.
start "IB 45 Coach setup" /wait "%INSTALLER%"
endlocal
