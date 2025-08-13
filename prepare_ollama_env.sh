#!/bin/bash
# This script starts the LocalMind server in development mode with auto-reloading.

echo "--- Starting LocalMind Server (Dev Mode) ---"

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
