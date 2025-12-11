@echo off
REM LocalMind Startup Script for LM Studio (Windows)
REM This script ensures LM Studio is running with the required models before launching the app

setlocal enabledelayedexpansion

echo.
echo LocalMind Startup Script
echo ============================
echo.

REM Configuration
set EMBEDDING_MODEL=google/embeddinggemma-300m-qat-GGUF
set COMPLETION_MODEL=lmstudio-community/gemma-2-2b-it-GGUF
set LMSTUDIO_PORT=1234

REM Step 1: Check if lms is available
echo [1/5] Checking for lms CLI...
where lms >nul 2>nul
if %errorlevel% neq 0 (
    echo [X] lms CLI not found
    echo.
    echo Please install LM Studio from: https://lmstudio.ai/
    echo Then run LM Studio at least once to initialize the lms CLI
    pause
    exit /b 1
)
echo [OK] lms CLI found
echo.

REM Step 2: Check if LM Studio is running
echo [2/5] Starting LM Studio server...
curl -s http://localhost:%LMSTUDIO_PORT%/v1/models >nul 2>nul
if %errorlevel% neq 0 (
    echo LM Studio server not running, starting it...
    start /B lms server start

    REM Wait for server to start (max 30 seconds)
    set /a attempts=0

    :wait_loop
    if !attempts! geq 30 goto server_timeout
    ping -n 2 127.0.0.1 >nul 2>nul
    curl -s http://localhost:%LMSTUDIO_PORT%/v1/models >nul 2>nul
    if !errorlevel! equ 0 goto server_ready
    set /a attempts+=1
    goto wait_loop

    :server_timeout
    echo [X] LM Studio server did not start in time
    echo Please start LM Studio manually
    pause
    exit /b 1

    :server_ready
    echo [OK] LM Studio server started
)
if %errorlevel% equ 0 (
    echo [OK] LM Studio server is running
)
echo.

REM Step 3: Check for required models
echo [3/5] Checking for required models...

REM Create temp file for model list
set TEMP_FILE=%TEMP%\lms_models_%RANDOM%.txt
lms ls > "%TEMP_FILE%" 2>nul

echo Checking embedding model: %EMBEDDING_MODEL%
findstr /C:"embeddinggemma" "%TEMP_FILE%" >nul 2>nul
if %errorlevel% neq 0 (
    echo [!] Embedding model not found
    echo.
    echo To download the embedding model:
    echo   1. Open LM Studio
    echo   2. Go to the "Discover" tab
    echo   3. Search for: embeddinggemma
    echo   4. Download: google/embeddinggemma-300m-qat-GGUF
    echo.
    echo Press any key when model is downloaded...
    pause >nul
) else (
    echo [OK] Embedding model found
)

echo Checking completion model: %COMPLETION_MODEL%
findstr /C:"gemma-2-2b" "%TEMP_FILE%" >nul 2>nul
if %errorlevel% neq 0 (
    echo [!] Completion model not found
    echo.
    echo To download the completion model:
    echo   1. Open LM Studio
    echo   2. Go to the "Discover" tab
    echo   3. Search for: gemma 2 2b
    echo   4. Download: lmstudio-community/gemma-2-2b-it-GGUF
    echo.
    echo Press any key when model is downloaded...
    pause >nul
) else (
    echo [OK] Completion model found
)

del "%TEMP_FILE%" 2>nul
echo.

REM Step 4: Load models
echo [4/5] Checking loaded models...
set TEMP_FILE=%TEMP%\lms_ps_%RANDOM%.txt
lms ps > "%TEMP_FILE%" 2>nul

findstr /C:"embeddinggemma" "%TEMP_FILE%" >nul 2>nul
if %errorlevel% neq 0 (
    echo Loading embedding model...
    lms load "%EMBEDDING_MODEL%" --gpu=max --yes 2>nul
    if !errorlevel! neq 0 (
        echo [!] Could not auto-load embedding model
        echo Please load it manually in LM Studio
    ) else (
        echo [OK] Embedding model loaded
    )
) else (
    echo [OK] Embedding model already loaded
)

findstr /C:"gemma-2-2b" "%TEMP_FILE%" >nul 2>nul
if %errorlevel% neq 0 (
    echo Loading completion model...
    lms load "%COMPLETION_MODEL%" --gpu=max --yes 2>nul
    if !errorlevel! neq 0 (
        echo [!] Could not auto-load completion model
        echo Please load it manually in LM Studio
    ) else (
        echo [OK] Completion model loaded
    )
) else (
    echo [OK] Completion model already loaded
)

del "%TEMP_FILE%" 2>nul
echo.

REM Step 5: Launch LocalMind
echo [5/5] Launching LocalMind...
echo.
echo [OK] All prerequisites met
echo.
echo Services:
echo   LM Studio:    http://localhost:%LMSTUDIO_PORT%
echo   Embedding:    %EMBEDDING_MODEL%
echo   Completion:   %COMPLETION_MODEL%
echo.

cd /d "%~dp0"

echo Building and starting LocalMind app...
echo Press Ctrl+C to stop
echo.

start /B cargo tauri dev

echo.
echo Both LM Studio and LocalMind are now running
echo LM Studio will continue running in the background
echo To stop LM Studio, close it manually or run: lms unload --all
echo.
pause
