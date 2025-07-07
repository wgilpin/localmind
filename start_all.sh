#!/bin/bash

# Start the desktop daemon
echo "Starting desktop-daemon..."
(cd desktop-daemon && npm start) &

# Start the search UI
echo "Starting search-ui..."
(cd search-ui && npm start) &

echo "All components started."