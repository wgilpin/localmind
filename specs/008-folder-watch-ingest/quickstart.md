# Quickstart: Testing Folder Watch and Ingest

**Branch**: `008-folder-watch-ingest` | **Date**: 2026-04-07

## Prerequisites

- LocalMind Rust app built and running (`cargo run` from `localmind-rs/`)
- Python embedding server running (started automatically by the app, or manually via
  `./start_localmind.sh` from the repo root)
- At least one test folder with PDF, MD, or TXT files

## Validate: Adding a Watched Folder (User Story 1)

1. Create a test folder with known files:

   ```bash
   mkdir /tmp/localmind-test
   echo "# Test Note\n\nThis is a test markdown file for LocalMind." > /tmp/localmind-test/note.md
   echo "Hello world. This is a plain text document." > /tmp/localmind-test/hello.txt
   ```

2. Launch the app: `cargo run` from `localmind-rs/`

3. Open **Settings** (gear icon or settings button in the UI)

4. Navigate to the **Watched Folders** section

5. Click **Add Folder** and enter `/tmp/localmind-test` (or use the folder picker)

6. **Expected**: Folder appears in the watched list with status `active`; progress
   indicator shows files being scanned; completes within 60 seconds

7. In the main search view, search for "plain text document"

8. **Expected**: `hello.txt` appears in results

---

## Validate: Automatic Re-ingestion on Change (User Story 2)

1. With the folder from above already watched and both files ingested:

2. Modify the text file:

   ```bash
   echo "Updated content: the sky is green." >> /tmp/localmind-test/hello.txt
   ```

3. Wait up to 30 seconds (watcher debounce + re-ingest time)

4. Search for "sky is green"

5. **Expected**: `hello.txt` appears in results with updated content

6. Add a new file:

   ```bash
   echo "Brand new document about elephants." > /tmp/localmind-test/elephants.txt
   ```

7. Wait up to 30 seconds; search for "elephants"

8. **Expected**: `elephants.txt` appears in results

9. Delete a file:

   ```bash
   rm /tmp/localmind-test/hello.txt
   ```

10. Search for "sky is green"

11. **Expected**: No results — file content removed from knowledge base

---

## Validate: Removing a Watched Folder (User Story 3)

1. With the test folder still watched:

2. Open **Settings** → **Watched Folders**

3. Click **Remove** next to `/tmp/localmind-test`

4. **Expected**: Folder disappears from the watched list within a few seconds

5. Search for "elephants"

6. **Expected**: No results — all content from the folder has been removed

7. Modify `/tmp/localmind-test/elephants.txt`:

   ```bash
   echo "More elephant facts." >> /tmp/localmind-test/elephants.txt
   ```

8. Wait 30 seconds; search for "elephant facts"

9. **Expected**: No results — folder is no longer monitored

---

## Validate: Error Handling

1. Attempt to add a non-existent path:
   - Enter `/tmp/this-does-not-exist` in the Add Folder dialog
   - **Expected**: Error message shown; folder not added to list

2. Attempt to add the same folder twice:
   - Add `/tmp/localmind-test`, then try to add it again
   - **Expected**: Error or info message; no duplicate entry in list

3. Add a folder containing an unsupported file type:

   ```bash
   cp /some/image.png /tmp/localmind-test/
   ```

   - **Expected**: Image file silently ignored; no error shown; supported files still
     ingested normally

---

## Run Tests

```bash
cd localmind-rs
cargo test folder_watcher
cargo test --all
```

All `folder_watcher` module tests should pass. No tests require a running embedding
server — the `LocalEmbeddingClient` is mocked via trait injection.
