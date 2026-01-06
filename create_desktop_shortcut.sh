#!/bin/bash
# Create Desktop Shortcut for LocalMind (macOS)
# This script creates a launcher that can be added to the Dock or Applications folder

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Get the project root directory (where this script is located)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"

# Get Desktop path
DESKTOP_PATH="$HOME/Desktop"
APPLICATIONS_PATH="$HOME/Applications"

# Paths
START_SCRIPT="$PROJECT_ROOT/start_localmind.sh"
LAUNCHER_NAME="LocalMind"
LAUNCHER_SCRIPT="$DESKTOP_PATH/$LAUNCHER_NAME.command"

# Check if start script exists
if [ ! -f "$START_SCRIPT" ]; then
    echo -e "${RED}Error: start_localmind.sh not found at: $START_SCRIPT${NC}" >&2
    exit 1
fi

# Ensure start script is executable
chmod +x "$START_SCRIPT" 2>/dev/null || true

echo -e "${CYAN}Creating LocalMind launcher...${NC}"
echo -e "${BLUE}  Project root: $PROJECT_ROOT${NC}"
echo -e "${BLUE}  Launcher: $LAUNCHER_SCRIPT${NC}"
echo ""

# Create the launcher script
cat > "$LAUNCHER_SCRIPT" << 'LAUNCHER_EOF'
#!/bin/bash
# LocalMind Launcher
# This script launches LocalMind from the project directory

# Get the directory where this launcher is located
LAUNCHER_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Get the project root (parent of Desktop, or find start_localmind.sh)
# Try to find start_localmind.sh in common locations
PROJECT_ROOT=""
if [ -f "$LAUNCHER_DIR/../start_localmind.sh" ]; then
    PROJECT_ROOT="$(cd "$LAUNCHER_DIR/../" && pwd)"
elif [ -f "$HOME/Projects/localmind/start_localmind.sh" ]; then
    PROJECT_ROOT="$HOME/Projects/localmind"
else
    # Try to find it by searching
    PROJECT_ROOT=$(find "$HOME" -name "start_localmind.sh" -type f 2>/dev/null | head -1 | xargs dirname)
fi

if [ -z "$PROJECT_ROOT" ] || [ ! -f "$PROJECT_ROOT/start_localmind.sh" ]; then
    osascript -e 'display dialog "Could not find LocalMind project directory. Please run this launcher from the project directory or update the path in the script." buttons {"OK"} default button "OK" with icon stop'
    exit 1
fi

# Change to project directory and run startup script
cd "$PROJECT_ROOT"
exec ./start_localmind.sh
LAUNCHER_EOF

# Make launcher executable
chmod +x "$LAUNCHER_SCRIPT"

echo -e "${GREEN}Launcher created successfully!${NC}"
echo -e "${GREEN}Location: $LAUNCHER_SCRIPT${NC}"
echo ""

# Optionally copy to Applications folder
if [ -d "$APPLICATIONS_PATH" ]; then
    echo -e "${CYAN}Would you like to also create a copy in Applications folder? (y/n)${NC}"
    read -r response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        cp "$LAUNCHER_SCRIPT" "$APPLICATIONS_PATH/$LAUNCHER_NAME.command"
        chmod +x "$APPLICATIONS_PATH/$LAUNCHER_NAME.command"
        echo -e "${GREEN}Launcher also created in: $APPLICATIONS_PATH/$LAUNCHER_NAME.command${NC}"
        echo ""
    fi
fi

# Instructions for adding to Dock
echo -e "${CYAN}========================================${NC}"
echo -e "${CYAN}To add to Dock:${NC}"
echo -e "${CYAN}========================================${NC}"
echo -e "${YELLOW}1. Find the '$LAUNCHER_NAME.command' file on your Desktop${NC}"
echo -e "${YELLOW}2. Drag it to the Dock${NC}"
echo ""
echo -e "${YELLOW}OR${NC}"
echo ""
echo -e "${YELLOW}1. Right-click the '$LAUNCHER_NAME.command' file on your Desktop${NC}"
echo -e "${YELLOW}2. Select 'Options' > 'Keep in Dock'${NC}"
echo ""
echo -e "${GREEN}The launcher is ready to use!${NC}"
echo ""
