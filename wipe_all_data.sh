#!/bin/bash
# This script is for development purposes only.
# It will wipe all data, including documents, notes, and the vector index.

# Stop on error
set -e

echo "This will permanently delete all user data and ChromaDB."
read -p "Are you sure you want to continue? [y/N] " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]
then
    exit 1
fi

echo "Stopping services..."
# Add commands to stop your services if they are running
# For example: pkill -f "ts-node" or similar

echo "Deleting user data..."
rm -rf "$HOME/.localmind"
echo "User data deleted."

echo "Uninstalling node modules..."
rm -rf desktop-daemon/node_modules
echo "Node modules uninstalled."

echo "All data has been wiped."