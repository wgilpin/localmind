#!/bin/bash
# Quick rebuild script for Rust app only
# Assumes embedding server is already running from start_localmind.sh

echo
echo "============================="
echo "  Quick Rebuild - Rust App"
echo "============================="
echo

cd localmind-rs
echo "[INFO] Rebuilding Rust application..."
cargo build
if [ $? -ne 0 ]; then
    echo "[ERROR] Build failed"
    cd ..
    exit 1
fi

echo "[OK] Build successful"
echo "[INFO] Starting application..."
echo

cargo run
cd ..

echo
echo "Application stopped."
