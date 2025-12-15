#!/bin/bash
# LocalMind Startup Script
# Automates Python embedding server startup and Rust application launch

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
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
if ! $PYTHON_CMD -m pip show uv &> /dev/null; then
    echo -e "${YELLOW}[INFO] uv not found, installing via pip...${NC}"
    $PYTHON_CMD -m pip install --user uv > /dev/null 2>&1
    if [ $? -ne 0 ]; then
        error_exit "Failed to install uv. Please install manually: $PYTHON_CMD -m pip install uv"
    fi
    echo -e "${GREEN}[OK] uv installed successfully${NC}"
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
    $PYTHON_CMD -m uv venv .venv
    if [ $? -ne 0 ]; then
        cd ..
        error_exit "Failed to create virtual environment"
    fi
    cd ..
    echo -e "${GREEN}[OK] Virtual environment created${NC}"
else
    echo -e "${GREEN}[OK] Virtual environment already exists${NC}"
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

# Install dependencies using the venv Python directly
if ! "$VENV_PYTHON" -m uv pip install -e . > /dev/null 2>&1; then
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
MAX_ATTEMPTS=30
ATTEMPT=0

while [ $ATTEMPT -lt $MAX_ATTEMPTS ]; do
    ATTEMPT=$((ATTEMPT + 1))
    
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        error_exit "Embedding server process died. Check logs: $SERVER_LOG"
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
        echo -e "${RED}[ERROR] Server did not become ready within $MAX_ATTEMPTS seconds${NC}"
        echo "        Check logs: $SERVER_LOG"
        kill "$SERVER_PID" 2>/dev/null || true
        error_exit "Embedding server health check failed"
    fi
    
    echo -e "${YELLOW}[INFO] Waiting for server... (attempt $ATTEMPT/$MAX_ATTEMPTS)${NC}"
    sleep 1
done
echo ""

# Step 8: Launch Rust application
echo -e "${BLUE}[8/9] Launching LocalMind application...${NC}"
echo ""
echo "============================="
echo "  Services Running:"
echo "============================="
echo "  Embedding Server: http://localhost:$EMBEDDING_SERVER_PORT"
echo "  Model: google/embeddinggemma-300M"
echo "============================="
echo ""

cd localmind-rs
cargo tauri dev --release
APP_EXIT_CODE=$?
cd ..

# Cleanup will be handled by trap
exit $APP_EXIT_CODE
