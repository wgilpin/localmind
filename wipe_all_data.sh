#!/bin/bash

echo "WARNING: This script will permanently delete all LocalMind database files."
echo "This includes the main database and any test databases/indexes."
echo "Type 'YES' to continue:"
read CONFIRMATION

if [ "$CONFIRMATION" = "YES" ]; then
    echo "Deleting database files..."
    rm -f ~/.localmind/localmind.db
    rm -f desktop-daemon/test-db.sqlite
    rm -f desktop-daemon/test-index.faiss
    echo "All specified database files have been deleted."
else
    echo "Operation cancelled. No files were deleted."
fi