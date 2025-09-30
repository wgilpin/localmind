#!/usr/bin/env python3
"""Delete all embeddings from the database before rechunking."""

import sqlite3
import os
from pathlib import Path

# Get the database path
db_path = Path(os.environ.get('APPDATA', '~')) / 'localmind' / 'localmind.db'
db_path = db_path.expanduser()

print("Delete All Embeddings Tool")
print("=" * 50)
print()
print(f"Database: {db_path}")
print()

if not db_path.exists():
    print(f"ERROR: Database not found at {db_path}")
    exit(1)

# Connect to database
conn = sqlite3.connect(str(db_path))
cursor = conn.cursor()

# Count existing embeddings
cursor.execute("SELECT COUNT(*) FROM embeddings")
total_embeddings = cursor.fetchone()[0]

if total_embeddings == 0:
    print("No embeddings found in database.")
    conn.close()
    exit(0)

print(f"Found {total_embeddings} embeddings in database")
print()

# Ask for confirmation
response = input(f"WARNING: Delete ALL {total_embeddings} embeddings? (yes/no): ")
if response.lower() != 'yes':
    print("Aborted by user")
    conn.close()
    exit(0)

print()
print("Deleting all embeddings...")

cursor.execute("DELETE FROM embeddings")
conn.commit()

print(f"Successfully deleted {total_embeddings} embeddings!")
print()

# Verify
cursor.execute("SELECT COUNT(*) FROM embeddings")
remaining = cursor.fetchone()[0]

if remaining == 0:
    print("Verification: All embeddings deleted!")
else:
    print(f"WARNING: {remaining} embeddings still remain")

# Vacuum to reclaim space
print()
print("Vacuuming database...")
conn.execute("VACUUM")
print("Done!")

conn.close()

print()
print("All embeddings deleted!")
print("Next steps:")
print("  1. Rebuild: cd localmind-rs && cargo build --release --bin rechunk")
print("  2. Rechunk: cargo run --release --bin rechunk")
print("  3. Re-embed: cargo run --release --bin reembed_batched lmstudio http://localhost:1234 <model> 50")