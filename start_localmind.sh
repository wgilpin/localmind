#!/bin/bash
# LocalMind Startup Script
# Automates Python embedding server startup and Rust application launch

set -e

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
EMBEDDING_SERVER_PORT="${EMBEDDING_SERVER_PORT:-8000}"
SERVER_LOG="embedding-server/embedding_server.log"
SERVER_PID_FILE="/tmp/localmind_embedding_server.pid"

# Track server PID for cleanup
SERVER_PID=""

# Cleanup function
cleanup() {
    echo ""
    echo -e "${BLUE}[9/9] Cleaning up...${NC}"
    
    if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        echo -e "${YELLOW}[INFO] Stopping embedding server (PID: $SERVER_PID)...${NC}"
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
        echo -e "${GREEN}[OK] Embedding server stopped${NC}"
    else
        # Try to find and kill by port
        local port_pid=$(lsof -ti:$EMBEDDING_SERVER_PORT 2>/dev/null || true)
        if [ -n "$port_pid" ]; then
            echo -e "${YELLOW}[INFO] Stopping process on port $EMBEDDING_SERVER_PORT (PID: $port_pid)...${NC}"
            kill "$port_pid" 2>/dev/null || true
        fi
    fi
    
    rm -f "$SERVER_PID_FILE"
    echo ""
    echo "LocalMind stopped."
    echo ""
}

# Set up trap for cleanup on exit
trap cleanup EXIT INT TERM

# Error handler
error_exit() {
    echo ""
    echo -e "${RED}[ERROR] $1${NC}"
    echo ""
    exit 1
}

echo ""
echo "============================="
echo "  LocalMind Startup Script"
echo "============================="
echo ""

# Step 0: Cleanup any leftover processes from previous runs
echo -e "${BLUE}[0/9] Cleaning up any leftover processes...${NC}"
# Kill any process using the embedding server port
PORT_PID=$(lsof -ti:$EMBEDDING_SERVER_PORT 2>/dev/null || true)
if [ -n "$PORT_PID" ]; then
    echo -e "${YELLOW}[INFO] Killing process using port $EMBEDDING_SERVER_PORT: PID $PORT_PID${NC}"
    kill "$PORT_PID" 2>/dev/null || true
    sleep 2
fi
# Clear log file if it exists (should be unlocked after killing processes)
if [ -f "$SERVER_LOG" ]; then
    rm -f "$SERVER_LOG" 2>/dev/null || echo -e "${YELLOW}[WARNING] Could not delete log file, may still be in use${NC}"
fi
echo -e "${GREEN}[OK] Cleanup complete${NC}"
echo ""

# Step 1: Check Python 3.11+ installation
echo -e "${BLUE}[1/9] Checking Python installation...${NC}"
if ! command -v python3 &> /dev/null && ! command -v python &> /dev/null; then
    error_exit "Python is not installed or not in PATH. Please install Python 3.11 or later from https://www.python.org/"
fi

# Use python3 if available, otherwise python
PYTHON_CMD="python3"
if ! command -v python3 &> /dev/null; then
    PYTHON_CMD="python"
fi

PYTHON_VERSION=$($PYTHON_CMD --version 2>&1 | awk '{print $2}')
echo -e "${GREEN}[OK] Python found: $PYTHON_VERSION${NC}"

# Check Python version is 3.11+
PYTHON_MAJOR=$(echo "$PYTHON_VERSION" | cut -d. -f1)
PYTHON_MINOR=$(echo "$PYTHON_VERSION" | cut -d. -f2)

if [ "$PYTHON_MAJOR" -lt 3 ] || ([ "$PYTHON_MAJOR" -eq 3 ] && [ "$PYTHON_MINOR" -lt 11 ]); then
    error_exit "Python 3.11+ required, found $PYTHON_VERSION"
fi
echo ""

# Step 2: Check for uv
echo -e "${BLUE}[2/9] Checking for uv...${NC}"
if ! command -v uv &> /dev/null && ! $PYTHON_CMD -m pip show uv &> /dev/null; then
    echo -e "${YELLOW}[INFO] uv not found, attempting to install...${NC}"
    
    # Try pipx first (recommended for macOS)
    if command -v pipx &> /dev/null; then
        echo -e "${YELLOW}[INFO] Installing uv via pipx...${NC}"
        pipx install uv > /dev/null 2>&1
        if [ $? -eq 0 ]; then
            echo -e "${GREEN}[OK] uv installed successfully via pipx${NC}"
        else
            echo -e "${YELLOW}[WARNING] pipx installation failed, trying pip...${NC}"
            # Fall through to pip attempt
        fi
    fi
    
    # If pipx didn't work or isn't available, try pip
    if ! command -v uv &> /dev/null; then
        echo -e "${YELLOW}[INFO] Installing uv via pip...${NC}"
        $PYTHON_CMD -m pip install --user uv > /dev/null 2>&1 || {
            # If --user fails, try without --user (might work in venv)
            $PYTHON_CMD -m pip install uv > /dev/null 2>&1 || {
                error_exit "Failed to install uv. Please install manually:\n  - macOS: brew install uv (or: pipx install uv)\n  - Or: $PYTHON_CMD -m pip install --user uv"
            }
        }
        echo -e "${GREEN}[OK] uv installed successfully${NC}"
    fi
else
    echo -e "${GREEN}[OK] uv found${NC}"
fi
echo ""

# Step 3: Check for port conflicts
echo -e "${BLUE}[3/9] Checking for port conflicts...${NC}"
if lsof -ti:$EMBEDDING_SERVER_PORT &> /dev/null; then
    echo -e "${YELLOW}[WARNING] Port $EMBEDDING_SERVER_PORT is already in use${NC}"
    echo "          This may indicate the embedding server is already running"
    echo "          Continuing anyway..."
fi
echo ""

# Step 4: Create virtual environment
echo -e "${BLUE}[4/9] Setting up Python virtual environment...${NC}"
if [ ! -d "embedding-server/.venv" ]; then
    echo -e "${YELLOW}[INFO] Creating virtual environment...${NC}"
    cd embedding-server
    
    # Try uv as command first, then as Python module
    # Explicitly use the detected Python version to ensure consistency
    # This overrides .python-version if present, using system Python instead
    UV_VENV_SUCCESS=0
    if command -v uv &> /dev/null; then
        # Use --python flag to explicitly specify Python version (overrides .python-version)
        if uv venv .venv --python "$PYTHON_CMD"; then
            UV_VENV_SUCCESS=1
        elif uv venv .venv; then
            # Fallback without explicit version (respects .python-version if present)
            UV_VENV_SUCCESS=1
        fi
    fi
    
    if [ $UV_VENV_SUCCESS -eq 0 ]; then
        if $PYTHON_CMD -m uv venv .venv 2>/dev/null; then
            UV_VENV_SUCCESS=1
        elif $PYTHON_CMD -m venv .venv 2>/dev/null; then
            # Fallback to standard venv if uv fails
            UV_VENV_SUCCESS=1
        fi
    fi
    
    if [ $UV_VENV_SUCCESS -eq 0 ]; then
        cd ..
        error_exit "Failed to create virtual environment. uv may not be installed correctly."
    fi
    cd ..
    echo -e "${GREEN}[OK] Virtual environment created${NC}"
else
    echo -e "${GREEN}[OK] Virtual environment already exists${NC}"
    echo -e "${YELLOW}[INFO] To recreate with a different Python version, delete embedding-server/.venv${NC}"
fi
echo ""

# Step 5: Activate virtual environment and install dependencies
echo -e "${BLUE}[5/9] Installing dependencies...${NC}"
cd embedding-server

# Detect Python executable in virtual environment
# Windows uses Scripts/python.exe, Unix uses bin/python
if [ -f ".venv/Scripts/python.exe" ]; then
    # Windows virtual environment - use the Python executable directly
    VENV_PYTHON=".venv/Scripts/python.exe"
elif [ -f ".venv/Scripts/python" ]; then
    # Windows virtual environment (alternative)
    VENV_PYTHON=".venv/Scripts/python"
elif [ -f ".venv/bin/python" ]; then
    # Unix virtual environment
    VENV_PYTHON=".venv/bin/python"
else
    cd ..
    error_exit "Python executable not found in virtual environment. Please recreate the virtual environment."
fi

# Make VENV_PYTHON absolute path for use later
VENV_PYTHON=$(cd "$(dirname "$VENV_PYTHON")" && pwd)/$(basename "$VENV_PYTHON")

# Verify the Python executable works
if ! "$VENV_PYTHON" --version > /dev/null 2>&1; then
    cd ..
    error_exit "Virtual environment Python executable is not working. Please recreate the virtual environment."
fi

# Check if typing_extensions is installed (required for Python < 3.12)
VENV_PYTHON_MAJOR=$("$VENV_PYTHON" --version 2>&1 | awk '{print $2}' | cut -d. -f1)
VENV_PYTHON_MINOR=$("$VENV_PYTHON" --version 2>&1 | awk '{print $2}' | cut -d. -f2)
NEEDS_TYPING_EXTENSIONS=0
if [ "$VENV_PYTHON_MAJOR" -lt 3 ] || ([ "$VENV_PYTHON_MAJOR" -eq 3 ] && [ "$VENV_PYTHON_MINOR" -lt 12 ]); then
    if ! "$VENV_PYTHON" -c "import typing_extensions" 2>/dev/null; then
        NEEDS_TYPING_EXTENSIONS=1
    fi
fi

# Install dependencies using the venv Python directly (without editable package install)
# Note: typing_extensions is required for Python < 3.12 compatibility with Pydantic
# Quote all version constraints to prevent shell interpretation issues
INSTALL_SUCCESS=0
if command -v uv &> /dev/null; then
    if uv pip install --python "$VENV_PYTHON" \
        "fastapi>=0.115.0" \
        "uvicorn[standard]>=0.32.0" \
        "sentence-transformers>=3.3.0" \
        "torch>=2.0.0" \
        "typing_extensions" \
        "transformers @ git+https://github.com/huggingface/transformers@v4.56.0-Embedding-Gemma-preview" \
        > /dev/null 2>&1; then
        INSTALL_SUCCESS=1
    fi
fi

if [ $INSTALL_SUCCESS -eq 0 ]; then
    # Fallback to python -m uv pip
    if "$VENV_PYTHON" -m uv pip install \
        "fastapi>=0.115.0" \
        "uvicorn[standard]>=0.32.0" \
        "sentence-transformers>=3.3.0" \
        "torch>=2.0.0" \
        "typing_extensions" \
        "transformers @ git+https://github.com/huggingface/transformers@v4.56.0-Embedding-Gemma-preview" \
        > /dev/null 2>&1; then
        INSTALL_SUCCESS=1
    fi
fi

# Final check: ensure typing_extensions is installed if needed (for Python < 3.12)
if [ $NEEDS_TYPING_EXTENSIONS -eq 1 ] && ! "$VENV_PYTHON" -c "import typing_extensions" 2>/dev/null; then
    echo -e "${YELLOW}[INFO] Installing typing_extensions (required for Python < 3.12)...${NC}"
    if command -v uv &> /dev/null; then
        uv pip install --python "$VENV_PYTHON" "typing_extensions" > /dev/null 2>&1 || "$VENV_PYTHON" -m pip install "typing_extensions" > /dev/null 2>&1
    else
        "$VENV_PYTHON" -m pip install "typing_extensions" > /dev/null 2>&1
    fi
    # Verify it was installed
    if ! "$VENV_PYTHON" -c "import typing_extensions" 2>/dev/null; then
        echo -e "${RED}[WARNING] Failed to install typing_extensions. The server may fail to start.${NC}"
    fi
fi

# Clean up any artifact files that might have been created (files starting with = or version numbers)
# We're already in embedding-server directory, so clean up here
rm -f "="* "="*.* "="*.*.* 2>/dev/null || true

if [ $INSTALL_SUCCESS -eq 0 ]; then
    cd ..
    error_exit "Failed to install dependencies. Check embedding-server/pyproject.toml"
fi

cd ..
echo -e "${GREEN}[OK] Dependencies installed${NC}"
echo ""

# Step 6: Start Python embedding server in background
echo -e "${BLUE}[6/9] Starting Python embedding server...${NC}"
cd embedding-server
export EMBEDDING_SERVER_PORT

# Use the venv Python (should be set from step 5)
# If for some reason it's not set, fall back to system Python
if [ -z "$VENV_PYTHON" ]; then
    # Fallback: detect venv Python again
    if [ -f ".venv/Scripts/python.exe" ]; then
        VENV_PYTHON="$(pwd)/.venv/Scripts/python.exe"
    elif [ -f ".venv/Scripts/python" ]; then
        VENV_PYTHON="$(pwd)/.venv/Scripts/python"
    elif [ -f ".venv/bin/python" ]; then
        VENV_PYTHON="$(pwd)/.venv/bin/python"
    else
        VENV_PYTHON="$PYTHON_CMD"
    fi
fi

"$VENV_PYTHON" embedding_server.py > "../$SERVER_LOG" 2>&1 &
SERVER_PID=$!
cd ..

# Save PID for cleanup
echo $SERVER_PID > "$SERVER_PID_FILE"

# Wait a moment for server to start
sleep 2

echo -e "${GREEN}[OK] Python server started (PID: $SERVER_PID)${NC}"
echo "      Logs: $SERVER_LOG"
echo ""

# Step 7: Health check polling
echo -e "${BLUE}[7/9] Waiting for embedding server to be ready...${NC}"
echo -e "${YELLOW}[INFO] Model loading can take 1-3 minutes on first run (downloading ~600MB)${NC}"
echo -e "${YELLOW}[INFO] Subsequent runs will be faster as the model is cached${NC}"
MAX_ATTEMPTS=180  # 3 minutes (180 seconds) for model download and loading
ATTEMPT=0

while [ $ATTEMPT -lt $MAX_ATTEMPTS ]; do
    ATTEMPT=$((ATTEMPT + 1))
    
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo -e "${RED}[ERROR] Embedding server process died.${NC}"
        echo -e "${YELLOW}Last 20 lines of log:${NC}"
        if [ -f "$SERVER_LOG" ]; then
            tail -20 "$SERVER_LOG" | sed 's/^/  /'
        else
            echo "  Log file not found: $SERVER_LOG"
        fi
        error_exit "Check logs: $SERVER_LOG"
    fi
    
    HEALTH_RESPONSE=$(curl -s "http://localhost:$EMBEDDING_SERVER_PORT/health" 2>/dev/null || echo "")
    
    if [ -n "$HEALTH_RESPONSE" ]; then
        # Check if model is loaded
        if echo "$HEALTH_RESPONSE" | grep -q '"model_loaded":\s*true'; then
            echo -e "${GREEN}[OK] Server is ready and model is loaded${NC}"
            break
        fi
    fi
    
    if [ $ATTEMPT -eq $MAX_ATTEMPTS ]; then
        echo -e "${RED}[ERROR] Server did not become ready within $((MAX_ATTEMPTS)) seconds (${MAX_ATTEMPTS}s)${NC}"
        if [ -f "$SERVER_LOG" ]; then
            echo -e "${YELLOW}Last 20 lines of log:${NC}"
            tail -20 "$SERVER_LOG" | sed 's/^/  /'
            
            # Check for authentication error
            if grep -q "gated repo\|401\|authentication\|Access to model" "$SERVER_LOG"; then
                echo ""
                echo -e "${CYAN}========================================${NC}"
                echo -e "${CYAN}Hugging Face Authentication Required${NC}"
                echo -e "${CYAN}========================================${NC}"
                echo -e "${YELLOW}The embedding model requires Hugging Face authentication.${NC}"
                echo ""
                echo "1. Request access at: https://huggingface.co/google/embeddinggemma-300M"
                echo "2. Get your token at: https://huggingface.co/settings/tokens"
                echo "3. Set the token and restart:"
                echo ""
                echo -e "${GREEN}   export HF_TOKEN=\"your_token_here\"${NC}"
                echo -e "${GREEN}   ./start_localmind.sh${NC}"
                echo ""
                echo "Or authenticate interactively:"
                echo -e "${GREEN}   cd embedding-server${NC}"
                echo -e "${GREEN}   .venv/bin/python -m pip install huggingface_hub${NC}"
                echo -e "${GREEN}   .venv/bin/python -c \"from huggingface_hub import login; login()\"${NC}"
                echo ""
            fi
        fi
        echo "        Full logs: $SERVER_LOG"
        kill "$SERVER_PID" 2>/dev/null || true
        error_exit "Embedding server health check failed"
    fi
    
    # Show progress every 10 seconds, or every second in the last 10 attempts
    if [ $((ATTEMPT % 10)) -eq 0 ] || [ $ATTEMPT -gt $((MAX_ATTEMPTS - 10)) ]; then
        echo -e "${YELLOW}[INFO] Waiting for server... (attempt $ATTEMPT/$MAX_ATTEMPTS - ${ATTEMPT}s elapsed)${NC}"
    fi
    sleep 1
done
echo ""

# Step 8: Launch Rust application
echo -e "${BLUE}[8/9] Launching LocalMind application...${NC}"

# Try to source Cargo environment if it exists (for rustup installations)
if [ -f "$HOME/.cargo/env" ] && ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}[INFO] Loading Rust/Cargo environment...${NC}"
    source "$HOME/.cargo/env"
fi

# Check for Rust/Cargo
if ! command -v cargo &> /dev/null; then
    echo ""
    echo -e "${RED}[ERROR] Rust/Cargo is not installed or not in PATH${NC}"
    echo ""
    echo -e "${YELLOW}To install Rust on macOS:${NC}"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo ""
    echo "Or using Homebrew:"
    echo "  brew install rust"
    echo ""
    echo -e "${YELLOW}After installing, restart your terminal or run:${NC}"
    echo "  source \$HOME/.cargo/env"
    echo ""
    echo "The embedding server is running and ready. Install Rust to launch the desktop application."
    echo ""
    error_exit "Rust/Cargo not found. Please install Rust to continue."
fi

echo ""
echo "============================="
echo "  Services Running:"
echo "============================="
echo "  Embedding Server: http://localhost:$EMBEDDING_SERVER_PORT"
echo "  Model: google/embeddinggemma-300M"
echo "============================="
echo ""

cd localmind-rs
cargo run
APP_EXIT_CODE=$?
cd ..

# Cleanup will be handled by trap
exit $APP_EXIT_CODE
