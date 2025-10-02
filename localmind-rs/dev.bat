@echo off
setlocal enabledelayedexpansion

echo Starting Vite dev server...
start "Vite Dev Server" npm run dev
timeout /t 3 /nobreak > nul

echo Starting Tauri...
cargo run

REM This runs after cargo run exits (Ctrl+C)
echo.
echo Shutting down Vite dev server...
tasklist /FI "WINDOWTITLE eq Vite Dev Server*" /FO CSV | find /I "node.exe" >nul
if %errorlevel%==0 (
    for /f "tokens=2 delims=," %%a in ('tasklist /V /FI "WINDOWTITLE eq Vite Dev Server*" /FO CSV ^| find /I "node.exe"') do (
        set PID=%%a
        set PID=!PID:"=!
        taskkill /F /PID !PID! >nul 2>&1
    )
    echo Vite dev server stopped.
) else (
    echo Vite dev server already stopped.
)

endlocal
