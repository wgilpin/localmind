@echo off
setlocal enabledelayedexpansion

echo.
echo =============================
echo   LocalMind Startup Script
echo =============================
echo.

REM Configuration
set EMBEDDING_MODEL=text-embedding-embeddinggemma-300m-qat
set EMBEDDING_MODEL_DISPLAY=text-embedding-embeddinggemma-300m-qat
set COMPLETION_MODEL=lmstudio-community/Meta-Llama-3.1-8B-Instruct-GGUF
set COMPLETION_MODEL_DISPLAY=meta-llama-3.1-8b-instruct
set LMSTUDIO_PORT=1234
set MODELS_CACHE=%TEMP%\\lmstudio_models.json

echo 1. Checking for lms CLI...
where lms >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] lms CLI not found
    echo.
    echo Please install LM Studio from: https://lmstudio.ai/
    echo Then run LM Studio at least once to initialize the lms CLI
    pause
    exit /b 1
)
echo [OK] lms CLI found
echo.

echo 2. Starting LM Studio...
curl -s http://localhost:%LMSTUDIO_PORT%/v1/models >nul 2>&1
if %errorlevel% equ 0 (
    echo [OK] LM Studio server is already running
) else (
    echo LM Studio server not running, starting it...
    lms server start
    
    REM Wait for server to start
    set /a attempts=0
    :wait_loop
    set /a attempts+=1
    if !attempts! gtr 30 (
        echo [ERROR] LM Studio server did not start in time
        pause
        exit /b 1
    )
    timeout /t 1 /nobreak >nul
    curl -s http://localhost:%LMSTUDIO_PORT%/v1/models >nul 2>&1
    if %errorlevel% neq 0 goto wait_loop
    
    echo [OK] LM Studio server is running
)
echo.

echo 3. Checking for required models...
echo Skipping explicit checks; relying on JIT loading.
echo.

echo 4. Launching LocalMind...
echo.
echo [OK] All prerequisites met
echo.
echo Services:
echo   LM Studio:  http://localhost:%LMSTUDIO_PORT%
echo   Embedding:  %EMBEDDING_MODEL_DISPLAY%
echo   Completion: %COMPLETION_MODEL_DISPLAY%
echo.
echo Starting application...
echo.

REM Launch with Tauri (it will handle Vite via beforeDevCommand)
cargo tauri dev

echo.
echo LocalMind stopped.
echo LM Studio will continue running in the background
echo To stop LM Studio, close it manually or run: lms unload --all
echo.
pause

