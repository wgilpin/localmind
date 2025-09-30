#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Temporary script to deduplicate embeddings in the LocalMind database.

This script finds and removes duplicate chunk embeddings that have the same:
- document_id
- chunk_index
- chunk_start
- chunk_end

It keeps the embedding with the lowest ID (oldest) and removes the duplicates.
"""

import sqlite3
import os
from pathlib import Path

# Get the database path
db_path = Path(os.environ.get('APPDATA', '~')) / 'localmind' / 'localmind.db'
db_path = db_path.expanduser()

print(f"🔍 LocalMind Embedding Deduplication Tool")
print(f"=" * 50)
print()
print(f"Database: {db_path}")
print()

if not db_path.exists():
    print(f"❌ Database not found at {db_path}")
    exit(1)

# Connect to database
conn = sqlite3.connect(str(db_path))
cursor = conn.cursor()

# Find duplicates
print("🔎 Finding duplicate embeddings...")
cursor.execute("""
    SELECT
        document_id,
        chunk_index,
        chunk_start,
        chunk_end,
        COUNT(*) as count,
        GROUP_CONCAT(id) as ids
    FROM embeddings
    GROUP BY document_id, chunk_index, chunk_start, chunk_end
    HAVING COUNT(*) > 1
    ORDER BY document_id, chunk_index
""")

duplicates = cursor.fetchall()

if not duplicates:
    print("✅ No duplicate embeddings found!")
    conn.close()
    exit(0)

print(f"📊 Found {len(duplicates)} groups of duplicate embeddings")
print()

# Calculate total duplicates to remove
total_to_remove = sum(count - 1 for _, _, _, _, count, _ in duplicates)
print(f"📊 Statistics:")
print(f"   Duplicate groups: {len(duplicates)}")
print(f"   Embeddings to remove: {total_to_remove}")
print()

# Show some examples
print("📋 Sample duplicates:")
for i, (doc_id, chunk_idx, chunk_start, chunk_end, count, ids) in enumerate(duplicates[:5]):
    id_list = ids.split(',')
    print(f"   Group {i+1}: Doc {doc_id}, Chunk {chunk_idx} ({chunk_start}..{chunk_end})")
    print(f"            {count} copies: IDs {ids}")
    print(f"            Keeping: {id_list[0]}, Removing: {', '.join(id_list[1:])}")

if len(duplicates) > 5:
    print(f"   ... and {len(duplicates) - 5} more groups")
print()

# Ask for confirmation
response = input(f"⚠️  Remove {total_to_remove} duplicate embeddings? (yes/no): ")
if response.lower() != 'yes':
    print("❌ Aborted by user")
    conn.close()
    exit(0)

print()
print("🗑️  Removing duplicates...")

removed_count = 0
for doc_id, chunk_idx, chunk_start, chunk_end, count, ids in duplicates:
    id_list = [int(id_str) for id_str in ids.split(',')]

    # Keep the first (lowest ID), remove the rest
    keep_id = id_list[0]
    remove_ids = id_list[1:]

    for remove_id in remove_ids:
        cursor.execute("DELETE FROM embeddings WHERE id = ?", (remove_id,))
        removed_count += 1

    if removed_count % 100 == 0:
        print(f"   Progress: {removed_count}/{total_to_remove} removed...")

conn.commit()

print()
print(f"✅ Successfully removed {removed_count} duplicate embeddings!")
print()

# Show final stats
cursor.execute("SELECT COUNT(*) FROM embeddings")
total_embeddings = cursor.fetchone()[0]

cursor.execute("SELECT COUNT(DISTINCT document_id) FROM embeddings")
total_docs = cursor.fetchone()[0]

print(f"📊 Final database state:")
print(f"   Total embeddings: {total_embeddings}")
print(f"   Documents with embeddings: {total_docs}")
print(f"   Average embeddings per document: {total_embeddings / total_docs:.1f}")
print()

# Verify no duplicates remain
cursor.execute("""
    SELECT COUNT(*)
    FROM (
        SELECT document_id, chunk_index, chunk_start, chunk_end, COUNT(*) as count
        FROM embeddings
        GROUP BY document_id, chunk_index, chunk_start, chunk_end
        HAVING COUNT(*) > 1
    )
""")
remaining_dupes = cursor.fetchone()[0]

if remaining_dupes == 0:
    print("✅ Verification: No duplicates remaining!")
else:
    print(f"⚠️  Warning: {remaining_dupes} duplicate groups still remain")

# Vacuum to reclaim space
print()
print("🧹 Vacuuming database to reclaim space...")
conn.execute("VACUUM")
print("✅ Vacuum complete!")

conn.close()

print()
print("🎉 Deduplication complete!")
print("💡 You may want to restart the application to reload the vector store")