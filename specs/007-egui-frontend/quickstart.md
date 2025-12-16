# Quickstart: Native egui Desktop GUI

**Feature**: 007-egui-frontend
**Date**: 2025-12-15

## Prerequisites

- Rust 1.75+ installed
- Python 3.11+ with embedding server dependencies
- Existing LocalMind database (optional, will be created if missing)

## Development Setup

### 1. Switch to Feature Branch

```bash
cd localmind
git checkout 007-egui-frontend
```

### 2. Update Dependencies

After modifying `Cargo.toml` to add egui/eframe and remove Tauri:

```bash
cd localmind-rs
cargo check
```

### 3. Start the Embedding Server

In a separate terminal:

```bash
cd embedding-server
uv run python embedding_server.py
```

Wait for "Server ready to accept requests" message.

### 4. Run the Application

```bash
cd localmind-rs
cargo run
```

The LocalMind window should appear within 3 seconds.

## Build Commands

### Development Build

```bash
cargo build
```

### Release Build

```bash
cargo build --release
```

Binary output: `target/release/localmind-rs` (or `.exe` on Windows)

### Run Tests

```bash
cargo test
```

### Check Code Quality

```bash
cargo fmt --check
cargo clippy
```

## Project Structure After Migration

```
localmind-rs/
├── Cargo.toml           # Updated dependencies
├── src/
│   ├── main.rs          # egui/eframe entry point
│   ├── gui/
│   │   ├── mod.rs       # GUI module exports
│   │   ├── app.rs       # LocalMindApp implementation
│   │   ├── state.rs     # State types and enums
│   │   ├── views/
│   │   │   ├── mod.rs
│   │   │   ├── home.rs
│   │   │   ├── search.rs
│   │   │   └── document.rs
│   │   └── widgets/
│   │       ├── mod.rs
│   │       ├── toast.rs
│   │       ├── settings.rs
│   │       └── folder_tree.rs
│   ├── lib.rs           # Backend exports (unchanged)
│   ├── db.rs            # Database (unchanged + get_recent_documents)
│   ├── rag.rs           # RAG pipeline (unchanged)
│   └── [other backend modules]
└── icons/               # Application icons
```

## Deleted Files

These files from the Svelte/Tauri setup should be removed:

```
localmind-rs/
├── src-ui/              # DELETE entire directory
├── package.json         # DELETE
├── package-lock.json    # DELETE
├── vite.config.js       # DELETE
├── svelte.config.js     # DELETE
├── tauri.conf.json      # DELETE
└── build.rs             # DELETE or rewrite
```

## Key Implementation Notes

### 1. Entry Point Change

Old (Tauri):
```rust
fn main() {
    tauri::Builder::default()
        .invoke_handler(...)
        .run(tauri::generate_context!())
}
```

New (eframe):
```rust
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("LocalMind")
            .with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native("LocalMind", options, Box::new(|cc| {
        Ok(Box::new(LocalMindApp::new(cc)))
    }))
}
```

### 2. HTTP Server Integration

The HTTP server for Chrome extension compatibility runs as a tokio task:

```rust
impl LocalMindApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        // Create shared RAG state
        let rag_state = Arc::new(RwLock::new(None));
        
        // Spawn HTTP server
        let rag_for_http = rag_state.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                start_http_server(rag_for_http).await
            });
        });
        
        // Initialize app
        Self {
            rag: rag_state,
            // ...
        }
    }
}
```

### 3. Async Search Pattern

```rust
fn perform_search(&mut self, ctx: &egui::Context) {
    let query = self.search_query.clone();
    let rag = self.rag.clone();
    let ctx = ctx.clone();
    
    self.pending_search = Some(Promise::spawn_async(async move {
        let rag_lock = rag.read().await;
        if let Some(ref rag) = *rag_lock {
            rag.search(&query, 20).await.ok()
        } else {
            None
        }
    }));
}

fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    // Check for completed search
    if let Some(promise) = &self.pending_search {
        if let Some(result) = promise.ready() {
            self.search_results = result.clone();
            self.pending_search = None;
            self.current_view = View::SearchResults;
        }
    }
    
    // Request repaint while search pending
    if self.pending_search.is_some() {
        ctx.request_repaint();
    }
}
```

## Testing the Migration

1. **Launch test**: App window appears within 3 seconds
2. **Search test**: Enter query, results appear within 2 seconds
3. **Document view test**: Click result, content displays, back works
4. **Settings test**: Open settings, folder tree loads, save persists
5. **Toast test**: Trigger bookmark processing, progress shows
6. **Chrome extension test**: Use extension, document ingests successfully

## Troubleshooting

### Window doesn't appear
- Check for panics in console output
- Verify egui/eframe dependencies are correct version

### Search doesn't work
- Verify embedding server is running on port 8000
- Check console for connection errors

### HTTP server port conflict
- Default range 3000-3010
- Check if another instance is running

### Build fails
- Run `cargo clean` and rebuild
- Verify all Tauri references removed from code


