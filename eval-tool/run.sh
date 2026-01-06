#!/bin/bash
# Evaluation Tool Runner (macOS)
# Runs the evaluation pipeline with specified parameters

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Check for Python
if ! command -v python3 &> /dev/null && ! command -v python &> /dev/null; then
    echo -e "${RED}Error: Python is not installed or not in PATH${NC}" >&2
    exit 1
fi

# Use python3 if available, otherwise python
PYTHON_CMD="python3"
if ! command -v python3 &> /dev/null; then
    PYTHON_CMD="python"
fi

# Check if main.py exists
if [ ! -f "main.py" ]; then
    echo -e "${RED}Error: main.py not found in current directory${NC}" >&2
    exit 1
fi

# Check for GPU availability (Mac uses Metal, not CUDA)
echo -e "${BLUE}Checking system information...${NC}"
if command -v system_profiler &> /dev/null; then
    echo "System:"
    system_profiler SPHardwareDataType | grep -E "Chip|Processor" || true
fi

# Note: Mac doesn't have nvidia-smi, GPU acceleration uses Metal
# If you need GPU acceleration, ensure PyTorch is installed with Metal support
echo ""
echo -e "${BLUE}Starting pipeline...${NC}"
echo ""

# Run the evaluation pipeline
# Note: Removed Ollama-specific environment variables as they're no longer part of the project
# If you need GPU acceleration, ensure your Python environment has Metal-accelerated PyTorch
$PYTHON_CMD main.py run-chunk-pipeline --sample-size 20 --llm-model qwen3:4b-gpu --embedding-model all-MiniLM-L6-v2 --top-k 5 --reset
