#!/usr/bin/env python3
"""Delete a document from the database by ID."""

import sqlite3
import os
import sys
from pathlib import Path

# Get the database path
db_path = Path(os.environ.get('APPDATA', '~')) / 'localmind' / 'localmind.db'
db_path = db_path.expanduser()

def delete_document(doc_id: int):
    """Delete a document and its embeddings from the database."""
    if not db_path.exists():
        print(f"ERROR: Database not found at {db_path}")
        sys.exit(1)

    # Connect to database
    conn = sqlite3.connect(str(db_path))
    cursor = conn.cursor()
    
    # Enable foreign keys to ensure cascade deletion works
    cursor.execute("PRAGMA foreign_keys = ON")
    
    # First, check if document exists and get info
    cursor.execute("SELECT id, title, url FROM documents WHERE id = ?", (doc_id,))
    doc = cursor.fetchone()
    
    if not doc:
        print(f"ERROR: Document with ID {doc_id} not found")
        conn.close()
        sys.exit(1)
    
    doc_id_db, title, url = doc
    print(f"Found document:")
    print(f"  ID: {doc_id_db}")
    print(f"  Title: {title[:60]}{'...' if len(title) > 60 else ''}")
    print(f"  URL: {url or '(no URL)'}")
    
    # Count embeddings that will be deleted
    cursor.execute("SELECT COUNT(*) FROM embeddings WHERE document_id = ?", (doc_id,))
    embedding_count = cursor.fetchone()[0]
    print(f"  Embeddings: {embedding_count}")
    print()
    
    # Ask for confirmation
    response = input(f"Delete document {doc_id}? (yes/no): ")
    if response.lower() != 'yes':
        print("Aborted by user")
        conn.close()
        sys.exit(0)
    
    print()
    print("Deleting document...")
    
    # Delete the document (embeddings will cascade delete)
    cursor.execute("DELETE FROM documents WHERE id = ?", (doc_id,))
    deleted_count = cursor.rowcount
    
    if deleted_count == 0:
        print("ERROR: Document was not deleted (may have been deleted already)")
        conn.close()
        sys.exit(1)
    
    conn.commit()
    
    # Verify embeddings were deleted
    cursor.execute("SELECT COUNT(*) FROM embeddings WHERE document_id = ?", (doc_id,))
    remaining_embeddings = cursor.fetchone()[0]
    
    if remaining_embeddings > 0:
        print(f"WARNING: {remaining_embeddings} embeddings still remain (cascade may not have worked)")
        print("Deleting orphaned embeddings...")
        cursor.execute("DELETE FROM embeddings WHERE document_id = ?", (doc_id,))
        conn.commit()
        print("Orphaned embeddings deleted")
    else:
        print(f"✓ Document and {embedding_count} embeddings deleted successfully")
    
    # Verify document is gone
    cursor.execute("SELECT COUNT(*) FROM documents WHERE id = ?", (doc_id,))
    if cursor.fetchone()[0] > 0:
        print("ERROR: Document still exists after deletion")
        conn.close()
        sys.exit(1)
    
    conn.close()
    print()
    print("Document deletion complete!")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python delete_doc.py <document_id>")
        print()
        print("Example:")
        print("  python delete_doc.py 1318")
        sys.exit(1)
    
    try:
        doc_id = int(sys.argv[1])
    except ValueError:
        print(f"ERROR: Invalid document ID: {sys.argv[1]}")
        print("Document ID must be a number")
        sys.exit(1)
    
    delete_document(doc_id)
