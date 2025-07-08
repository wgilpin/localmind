#!/bin/bash
# This script builds and starts the unified LocalMind server.

echo "--- Starting LocalMind Server ---"

# Navigate to the daemon directory
cd desktop-daemon

# Install dependencies
echo "Installing dependencies..."
npm install

# Build the TypeScript source
echo "Building from source..."
npm run build

# Start the server
echo "Launching server..."
npm start