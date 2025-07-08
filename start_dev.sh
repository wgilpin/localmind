#!/bin/bash
# This script starts the LocalMind server in development mode with auto-reloading.

echo "--- Starting LocalMind Server (Dev Mode) ---"

# Navigate to the daemon directory
cd desktop-daemon

# Install dependencies (if needed)
echo "Installing dependencies..."
npm install

# Start the server in dev mode
echo "Launching server with auto-reloading..."
npm run dev