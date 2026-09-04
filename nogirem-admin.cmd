@echo off
setlocal

fltmc >nul 2>&1
if errorlevel 1 (
  echo Requesting administrator privileges...
  powershell.exe -NoProfile -NonInteractive -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
  exit /b
)

cd /d "%~dp0"
call npm start
pause
