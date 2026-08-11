@echo off
setlocal

cd /d "%~dp0"
set "JANET_SCHOOL_BASE_PATH=%~dp0"

if not exist "janet-school-rs.exe" (
    echo Missing janet-school-rs.exe in %~dp0
    pause
    exit /b 1
)

start "" "janet-school-rs.exe"
