@echo off
REM Quick rebuild script for Rust app only
REM Assumes embedding server is already running from start_localmind.bat

echo.
echo =============================
echo   Quick Rebuild - Rust App
echo =============================
echo.

cd localmind-rs
echo [INFO] Rebuilding Rust application...
cargo build
if %errorlevel% neq 0 (
    echo [ERROR] Build failed
    cd ..
    pause
    exit /b 1
)

echo [OK] Build successful
echo [INFO] Starting application...
echo.

cargo run
cd ..

echo.
echo Application stopped.
pause

