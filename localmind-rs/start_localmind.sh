#!/bin/bash
# LocalMind Startup Script for LM Studio
# This script ensures LM Studio is running with the required models before launching the app

set -e

echo "🚀 LocalMind Startup Script"
echo "============================"
echo ""

# Configuration
EMBEDDING_MODEL="google/embeddinggemma-300m-qat-GGUF"
COMPLETION_MODEL="lmstudio-community/gemma-2-2b-it-GGUF"
LMSTUDIO_PORT=1234

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to check if LM Studio server is running
check_lmstudio_running() {
    if curl -s "http://localhost:${LMSTUDIO_PORT}/v1/models" > /dev/null 2>&1; then
        return 0
    else
        return 1
    fi
}

# Function to wait for LM Studio to start
wait_for_lmstudio() {
    echo "⏳ Waiting for LM Studio server to start..."
    local max_attempts=30
    local attempt=0

    while [ $attempt -lt $max_attempts ]; do
        if check_lmstudio_running; then
            echo -e "${GREEN}✓ LM Studio server is running${NC}"
            return 0
        fi
        attempt=$((attempt + 1))
        sleep 1
    done

    echo -e "${RED}✗ LM Studio server did not start in time${NC}"
    return 1
}

# Step 1: Check if lms is available
echo "1️⃣ Checking for lms CLI..."
if ! command -v lms &> /dev/null; then
    echo -e "${RED}✗ lms CLI not found${NC}"
    echo ""
    echo "Please install LM Studio from: https://lmstudio.ai/"
    echo "Then run LM Studio at least once to initialize the lms CLI"
    exit 1
fi
echo -e "${GREEN}✓ lms CLI found${NC}"
echo ""

# Step 2: Start LM Studio if not running
echo "2️⃣ Starting LM Studio..."
if check_lmstudio_running; then
    echo -e "${GREEN}✓ LM Studio server is already running${NC}"
else
    echo "LM Studio server not running, starting it..."

    # Use lms CLI to start the server
    lms server start

    # Wait for server to start
    wait_for_lmstudio || {
        echo -e "${RED}✗ LM Studio server did not start${NC}"
        echo "Please ensure LM Studio is installed and try again"
        exit 1
    }
fi
echo ""

# Step 3: Check if required models are downloaded
echo "3️⃣ Checking for required models..."
DOWNLOADED_MODELS=$(lms ls 2>/dev/null | tail -n +2 || echo "")

check_model_downloaded() {
    local model_id=$1
    echo "$DOWNLOADED_MODELS" | grep -q "$model_id"
}

# Check embedding model
echo "Checking embedding model: $EMBEDDING_MODEL"
if check_model_downloaded "$EMBEDDING_MODEL"; then
    echo -e "${GREEN}✓ Embedding model already downloaded${NC}"
else
    echo -e "${YELLOW}⚠ Embedding model not found${NC}"
    echo ""
    echo "To download the embedding model, run:"
    echo "  1. Open LM Studio"
    echo "  2. Go to the 'Discover' tab"
    echo "  3. Search for: nomic-embed-text"
    echo "  4. Download: nomic-ai/nomic-embed-text-v1.5-GGUF"
    echo ""
    echo -e "${YELLOW}Press Enter when model is downloaded, or Ctrl+C to exit${NC}"
    read -r
fi

# Check completion model
echo "Checking completion model: $COMPLETION_MODEL"
if check_model_downloaded "$COMPLETION_MODEL"; then
    echo -e "${GREEN}✓ Completion model already downloaded${NC}"
else
    echo -e "${YELLOW}⚠ Completion model not found${NC}"
    echo ""
    echo "To download the completion model, run:"
    echo "  1. Open LM Studio"
    echo "  2. Go to the 'Discover' tab"
    echo "  3. Search for: Llama 3.1 8B Instruct"
    echo "  4. Download: lmstudio-community/Meta-Llama-3.1-8B-Instruct-GGUF"
    echo ""
    echo -e "${YELLOW}Press Enter when model is downloaded, or Ctrl+C to exit${NC}"
    read -r
fi
echo ""

# Step 4: Load embedding model if not loaded
echo "4️⃣ Loading models..."
LOADED_MODELS=$(lms ps 2>/dev/null || echo "")

check_model_loaded() {
    local model_id=$1
    echo "$LOADED_MODELS" | grep -q "$model_id"
}

if check_model_loaded "nomic-embed"; then
    echo -e "${GREEN}✓ Embedding model already loaded${NC}"
else
    echo "Loading embedding model..."
    lms load "$EMBEDDING_MODEL" --gpu=max --yes 2>/dev/null || {
        echo -e "${YELLOW}⚠ Could not auto-load embedding model${NC}"
        echo "Please load the embedding model manually in LM Studio"
    }
fi

if check_model_loaded "Llama-3.1-8B"; then
    echo -e "${GREEN}✓ Completion model already loaded${NC}"
else
    echo "Loading completion model..."
    lms load "$COMPLETION_MODEL" --gpu=max --yes 2>/dev/null || {
        echo -e "${YELLOW}⚠ Could not auto-load completion model${NC}"
        echo "Please load the completion model manually in LM Studio"
    }
fi
echo ""

# Cleanup function
cleanup() {
    echo ""
    echo "🛑 Shutting down..."
    if [ ! -z "$APP_PID" ] && kill -0 $APP_PID 2>/dev/null; then
        echo "Stopping LocalMind app (PID: $APP_PID)..."
        kill $APP_PID 2>/dev/null
    fi
    echo "LM Studio will continue running in the background"
    echo "To stop LM Studio, close it manually or run: lms unload --all"
    exit 0
}

# Trap Ctrl+C
trap cleanup INT TERM

# Step 5: Launch LocalMind
echo "5️⃣ Launching LocalMind..."
echo ""
echo -e "${GREEN}✓ All prerequisites met${NC}"
echo ""
echo "Services:"
echo "  LM Studio:    http://localhost:${LMSTUDIO_PORT}"
echo "  Embedding:    $EMBEDDING_MODEL"
echo "  Completion:   $COMPLETION_MODEL"
echo ""

# Build and run the app
cd "$(dirname "$0")"

echo "Building and starting LocalMind app..."
echo "Press Ctrl+C to stop"
echo ""

cargo tauri dev &
APP_PID=$!

# Wait for the app process
wait $APP_PID

echo ""
echo -e "${GREEN}✓ LocalMind stopped${NC}"
