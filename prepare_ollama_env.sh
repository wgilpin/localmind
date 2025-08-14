#!/bin/bash
# This script starts the LocalMind server in development mode with auto-reloading.

echo "--- Prepare Ollama env (Dev Mode) ---"

# Navigate to the daemon directory
cd desktop-daemon

# Install dependencies (if needed)
echo "Installing dependencies..."
npm install

# Set OLLAMA_NUM_PARALLEL for concurrent model loading
export OLLAMA_NUM_PARALLEL=2
export OLLAMA_KEEP_ALIVE=-1
export OLLAMA_MAX_LOADED_MODELS=2

# Check if Ollama is running and start it if not (cross-platform)
echo "Checking Ollama service status..."
if [[ "$OSTYPE" == "darwin"* ]]; then
  # macOS
  if ! pgrep -x "Ollama" > /dev/null; then
    echo "Ollama not running. Starting Ollama service..."
    ollama serve & > /dev/null 2>&1
    sleep 5 # Give Ollama a moment to start
  else
    echo "Ollama service is already running."
  fi
elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
  # Windows (Git Bash)
  if ! tasklist | findstr /i "ollama.exe" > /dev/null; then
    echo "Ollama not running. Starting Ollama service..."
    start ollama serve & > /dev/null 2>&1
    sleep 5 # Give Ollama a moment to start
  else
    echo "Ollama service is already running."
  fi
else
  echo "Unsupported OS for automatic Ollama startup. Please ensure Ollama is running."
fi

# Check if ChromaDB is installed, install if not
echo "Checking ChromaDB installation..."
if ! command -v chroma &> /dev/null; then
    echo "ChromaDB not found. Installing ChromaDB..."
    pip install chromadb
else
    echo "ChromaDB found. Version:"
    chroma --version 2>/dev/null || echo "Could not get ChromaDB version"
    
    # Update ChromaDB to ensure compatibility with JS client v3.0.11
    echo "Updating ChromaDB to latest version for API v2 compatibility..."
    pip install --upgrade chromadb --pre  # Include pre-release versions for latest API
fi

# Start ChromaDB server
echo "Checking ChromaDB service status..."

# First, try to connect to existing server
if curl -s http://localhost:8000/api/v1/heartbeat > /dev/null 2>&1 || curl -s http://localhost:8000/api/v2/heartbeat > /dev/null 2>&1; then
    echo "ChromaDB server is already running and healthy."
else
    echo "ChromaDB server not responding, starting fresh instance..."
    
    # Kill any existing ChromaDB processes on Windows
    if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
        taskkill //F //IM python.exe //FI "WINDOWTITLE eq chroma*" 2>/dev/null || true
        taskkill //F //IM python.exe //FI "COMMANDLINE eq *chroma*" 2>/dev/null || true
    else
        pkill -f "chroma run" 2>/dev/null || true
    fi
    
    # Wait a moment for cleanup
    sleep 2
    
    # Ensure data directory exists
    mkdir -p ~/.localmind/chromadb
    
    # Start ChromaDB server
    echo "Starting ChromaDB server..."
    chroma run --path ~/.localmind/chromadb --host localhost --port 8000 &
    
    # Wait for ChromaDB to be ready with health check
    echo "Waiting for ChromaDB to be ready..."
    for i in {1..30}; do
        if curl -s http://localhost:8000/api/v1/heartbeat > /dev/null 2>&1 || curl -s http://localhost:8000/api/v2/heartbeat > /dev/null 2>&1; then
            echo "ChromaDB server is ready!"
            break
        fi
        if [ $i -eq 30 ]; then
            echo "ERROR: ChromaDB server failed to start after 30 seconds"
            exit 1
        fi
        sleep 1
    done
fi
