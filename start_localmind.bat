@echo off
setlocal enabledelayedexpansion

echo.
echo =============================
echo   LocalMind Startup Script
echo =============================
echo.

REM Configuration
set EMBEDDING_SERVER_PORT=%EMBEDDING_SERVER_PORT%
if "%EMBEDDING_SERVER_PORT%"=="" set EMBEDDING_SERVER_PORT=8000
set SERVER_LOG=embedding-server\embedding_server.log
set SERVER_PID_FILE=%TEMP%\localmind_embedding_server.pid

REM Track Python server PID for cleanup
set SERVER_PID=

REM Error handler
goto :main

:error_exit
echo.
echo [ERROR] %~1
echo.
pause
exit /b 1

:main

REM Step 1: Check Python 3.11+ installation
echo [1/9] Checking Python installation...

REM Find system Python (not from any venv)
REM Try python.exe first, then python
where python.exe >nul 2>&1
if %errorlevel% equ 0 (
    for /f "delims=" %%i in ('where python.exe') do set SYSTEM_PYTHON=%%i
) else (
    where python >nul 2>&1
    if %errorlevel% equ 0 (
        for /f "delims=" %%i in ('where python') do set SYSTEM_PYTHON=%%i
    )
)

REM If we found a Python in a venv, try to find the system one
echo !SYSTEM_PYTHON! | findstr /i "\.venv" >nul 2>&1
if %errorlevel% equ 0 (
    REM Python is in a venv, try py launcher or python3
    where py >nul 2>&1
    if %errorlevel% equ 0 (
        for /f "delims=" %%i in ('where py') do set SYSTEM_PYTHON=%%i -3
    ) else (
        where python3 >nul 2>&1
        if %errorlevel% equ 0 (
            for /f "delims=" %%i in ('where python3') do set SYSTEM_PYTHON=%%i
        )
    )
)

REM Fallback to just "python" if we couldn't find system Python
if not defined SYSTEM_PYTHON (
    set SYSTEM_PYTHON=python
)

%SYSTEM_PYTHON% --version >nul 2>&1
if %errorlevel% neq 0 (
    call :error_exit "Python is not installed or not in PATH. Please install Python 3.11 or later from https://www.python.org/"
)

for /f "tokens=2" %%i in ('%SYSTEM_PYTHON% --version 2^>^&1') do set PYTHON_VERSION=%%i
echo [OK] Python found: !PYTHON_VERSION!

REM Check Python version is 3.11+
for /f "tokens=1,2 delims=." %%a in ("!PYTHON_VERSION!") do (
    set MAJOR=%%a
    set MINOR=%%b
)
if !MAJOR! lss 3 (
    call :error_exit "Python 3.11+ required, found !PYTHON_VERSION!"
)
if !MAJOR! equ 3 if !MINOR! lss 11 (
    call :error_exit "Python 3.11+ required, found !PYTHON_VERSION!"
)
echo.

REM Step 2: Check for uv
echo [2/9] Checking for uv...
%SYSTEM_PYTHON% -m pip show uv >nul 2>&1
if %errorlevel% neq 0 (
    echo [INFO] uv not found, installing via pip...
    %SYSTEM_PYTHON% -m pip install --user uv
    if %errorlevel% neq 0 (
        call :error_exit "Failed to install uv. Please install manually: %SYSTEM_PYTHON% -m pip install uv"
    )
    echo [OK] uv installed successfully
) else (
    echo [OK] uv found
)
echo.

REM Step 3: Check for port conflicts
echo [3/9] Checking for port conflicts...
netstat -an | findstr ":%EMBEDDING_SERVER_PORT% " >nul 2>&1
if %errorlevel% equ 0 (
    echo [WARNING] Port %EMBEDDING_SERVER_PORT% is already in use
    echo           This may indicate the embedding server is already running
    echo           Continuing anyway...
)
echo.

REM Step 4: Create virtual environment
echo [4/9] Setting up Python virtual environment...
if not exist "embedding-server\.venv" (
    echo [INFO] Creating virtual environment...
    cd embedding-server
    %SYSTEM_PYTHON% -m uv venv .venv
    if %errorlevel% neq 0 (
        cd ..
        call :error_exit "Failed to create virtual environment"
    )
    cd ..
    echo [OK] Virtual environment created
) else (
    echo [OK] Virtual environment already exists
)
echo.

REM Step 5: Activate virtual environment and install dependencies
echo [5/9] Installing dependencies...
cd embedding-server

REM Use venv Python directly (more reliable than activation)
if exist ".venv\Scripts\python.exe" (
    set VENV_PYTHON=.venv\Scripts\python.exe
) else if exist ".venv\Scripts\python.bat" (
    set VENV_PYTHON=.venv\Scripts\python.bat
) else if exist ".venv\bin\python.exe" (
    set VENV_PYTHON=.venv\bin\python.exe
) else (
    cd ..
    call :error_exit "Python executable not found in virtual environment. Please recreate the virtual environment."
)

REM Verify Python works
%VENV_PYTHON% --version >nul 2>&1
if %errorlevel% neq 0 (
    cd ..
    call :error_exit "Virtual environment Python executable is not working. Please recreate the virtual environment."
)

REM Install dependencies using venv Python
echo [INFO] Installing dependencies (this may take a few minutes)...
%VENV_PYTHON% -m uv pip install -e .
if %errorlevel% neq 0 (
    cd ..
    call :error_exit "Failed to install dependencies. Check embedding-server\pyproject.toml"
)
cd ..
echo [OK] Dependencies installed
echo.

REM Step 6: Start Python embedding server in background
echo [6/9] Starting Python embedding server...
echo [INFO] This may take 1-2 minutes on first run (model download ~600MB)
echo [INFO] Server logs: %SERVER_LOG%
cd embedding-server
set EMBEDDING_SERVER_PORT=%EMBEDDING_SERVER_PORT%

REM Use venv Python if available, otherwise system Python
if not defined VENV_PYTHON (
    if exist ".venv\Scripts\python.exe" (
        set VENV_PYTHON=.venv\Scripts\python.exe
    ) else if exist ".venv\Scripts\python.bat" (
        set VENV_PYTHON=.venv\Scripts\python.bat
    ) else (
        set VENV_PYTHON=python
    )
)

REM Use absolute path for log file since we're in embedding-server directory
set ABS_SERVER_LOG=%CD%\..\%SERVER_LOG%

REM Start server and capture output to log file
start /B %VENV_PYTHON% embedding_server.py > "%ABS_SERVER_LOG%" 2>&1
set SERVER_PID=%errorlevel%
cd ..

REM Wait a moment for server to start
timeout /t 2 /nobreak >nul

REM Try to get the actual PID from the log or process list
for /f "tokens=2" %%i in ('tasklist /FI "IMAGENAME eq python.exe" /FO LIST ^| findstr "PID:"') do (
    set SERVER_PID=%%i
)

echo [OK] Python server started (PID: !SERVER_PID!)
echo       Logs: %SERVER_LOG%
echo.

REM Step 7: Health check polling
echo [7/9] Waiting for embedding server to be ready...
echo [INFO] Model loading can take 1-2 minutes on first run...
set /a attempts=0
set /a max_attempts=120
:health_check_loop
set /a attempts+=1
if !attempts! gtr !max_attempts! (
    echo [ERROR] Server did not become ready within !max_attempts! seconds
    echo         Check logs: %SERVER_LOG%
    if exist "%SERVER_LOG%" (
        echo.
        echo Last 20 lines of log:
        powershell -Command "Get-Content '%SERVER_LOG%' -Tail 20"
    )
    if not "!SERVER_PID!"=="" (
        taskkill /PID !SERVER_PID! /F >nul 2>&1
    )
    call :error_exit "Embedding server health check failed"
)

curl -s http://localhost:%EMBEDDING_SERVER_PORT%/health >nul 2>&1
if %errorlevel% equ 0 (
    REM Check if model is loaded
    for /f "delims=" %%i in ('curl -s http://localhost:%EMBEDDING_SERVER_PORT%/health') do set HEALTH_RESPONSE=%%i
    echo !HEALTH_RESPONSE! | findstr "model_loaded.*true" >nul 2>&1
    if %errorlevel% equ 0 (
        echo [OK] Server is ready and model is loaded
        goto :server_ready
    )
)

REM Show progress every 10 seconds
set /a show_progress=!attempts! %% 10
if !show_progress! equ 0 (
    echo [INFO] Still loading... (attempt !attempts!/!max_attempts! - this is normal on first run)
    if exist "%SERVER_LOG%" (
        REM Show last log line if available
        for /f "delims=" %%i in ('powershell -Command "Get-Content ''%SERVER_LOG%'' -Tail 1 2>$null"') do echo        %%i
    )
)
timeout /t 1 /nobreak >nul
goto :health_check_loop

:server_ready
echo.

REM Step 8: Launch Rust application
echo [8/9] Launching LocalMind application...
echo.
echo =============================
echo   Services Running:
echo =============================
echo   Embedding Server: http://localhost:%EMBEDDING_SERVER_PORT%
echo   Model: google/embeddinggemma-300M
echo =============================
echo.

cd localmind-rs
echo.
echo [INFO] Starting LocalMind application...
echo.
cargo tauri dev
set APP_EXIT_CODE=%errorlevel%
cd ..

REM Step 9: Cleanup
echo.
echo [9/9] Cleaning up...
if not "!SERVER_PID!"=="" (
    echo [INFO] Stopping embedding server (PID: !SERVER_PID!)...
    taskkill /PID !SERVER_PID! /F >nul 2>&1
    if %errorlevel% equ 0 (
        echo [OK] Embedding server stopped
    ) else (
        echo [WARNING] Failed to stop server, it may have already exited
    )
) else (
    REM Try to find and kill by port
    for /f "tokens=5" %%i in ('netstat -ano ^| findstr ":%EMBEDDING_SERVER_PORT%"') do (
        taskkill /PID %%i /F >nul 2>&1
    )
)

echo.
echo LocalMind stopped.
echo.

if %APP_EXIT_CODE% neq 0 (
    echo [WARNING] Application exited with code %APP_EXIT_CODE%
    pause
    exit /b %APP_EXIT_CODE%
)

exit /b 0
