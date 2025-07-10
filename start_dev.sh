#!/bin/bash
# This script starts the LocalMind server in development mode with auto-reloading.

echo "--- Starting LocalMind Server (Dev Mode) ---"

# Prepare Ollama environment
bash prepare_ollama_env.sh

# Navigate to the daemon directory
cd desktop-daemon

# Start the server in dev mode
echo "Launching server with auto-reloading..."
npm run dev