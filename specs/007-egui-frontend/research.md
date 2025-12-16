# Research: Native egui Desktop GUI

**Feature**: 007-egui-frontend
**Date**: 2025-12-15

## Research Questions

### 1. egui/eframe Framework Selection

**Decision**: Use egui 0.29+ with eframe backend

**Rationale**:
- egui is the most mature immediate-mode GUI for Rust
- eframe provides cross-platform window management (Windows, macOS, Linux)
- Single crate dependency for core UI needs
- Active development, frequent releases
- Excellent documentation and examples

**Alternatives Considered**:
- **iced**: Elm architecture, more complex state management, steeper learning curve
- **Slint**: Separate markup language, commercial license concerns
- **Dioxus**: React-like but native renderer less mature
- **gtk-rs**: Heavy dependency, GTK runtime required

### 2. Async Integration with egui

**Decision**: Use `poll_promise` crate or manual polling pattern

**Rationale**:
- egui is single-threaded immediate mode
- Long operations (search, document fetch) must not block UI
- `poll_promise` provides clean async-to-sync bridge
- Alternative: use channels with `try_recv()` in update loop

**Pattern**:
```rust
// In update():
if let Some(promise) = &self.pending_search {
    if let Some(result) = promise.ready() {
        self.search_results = result.clone();
        self.pending_search = None;
    }
}
```

**Alternatives Considered**:
- **Full async egui**: Not supported natively
- **Spawn blocking**: Would freeze UI during operations

### 3. HTML to Plain Text Conversion

**Decision**: Use `html2text` crate version 0.12+

**Rationale**:
- Lightweight (~50KB compiled)
- Handles common HTML structures (lists, paragraphs, links)
- Configurable line width
- Well-maintained, active development

**Alternatives Considered**:
- **scraper + manual extraction**: More code, error-prone
- **ammonia + strip**: Sanitizer, not converter
- **regex stripping**: Fragile, misses nested tags

### 4. Opening URLs in Default Browser

**Decision**: Use `open` crate version 5+

**Rationale**:
- Cross-platform (Windows, macOS, Linux)
- Minimal dependencies
- Simple API: `open::that(url)`
- Well-established, stable

**Alternatives Considered**:
- **webbrowser crate**: Similar but less maintained
- **Platform-specific commands**: Not portable

### 5. Application Icon and Window Configuration

**Decision**: Use eframe's `NativeOptions` with existing icon files

**Rationale**:
- eframe supports window icon via `icon_data`
- Existing icons in `localmind-rs/icons/` can be reused
- Window title, size, and position configurable

**Configuration**:
```rust
let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
        .with_title("LocalMind")
        .with_inner_size([1200.0, 800.0])
        .with_min_inner_size([800.0, 600.0])
        .with_icon(load_icon()),
    ..Default::default()
};
```

### 6. Toast Notification System

**Decision**: Custom toast widget with timer-based dismissal

**Rationale**:
- egui has no built-in toast system
- Simple to implement: Vec<Toast> + timestamp checking
- Render as overlay in bottom-right corner
- Auto-dismiss via elapsed time check

**Implementation Approach**:
```rust
struct Toast {
    id: u64,
    message: String,
    toast_type: ToastType,
    created_at: Instant,
    duration: Duration, // 0 = persistent
}

// In update(): filter out expired toasts
self.toasts.retain(|t| {
    t.duration.is_zero() || t.created_at.elapsed() < t.duration
});
```

### 7. Folder Tree Widget for Settings

**Decision**: Custom recursive tree widget using egui's CollapsingHeader

**Rationale**:
- egui has `CollapsingHeader` for expandable sections
- Combine with checkboxes for selection
- Recursive rendering for nested folders

**Implementation Approach**:
```rust
fn render_folder_tree(ui: &mut Ui, folder: &BookmarkFolder, excluded: &mut HashSet<String>) {
    let mut is_excluded = excluded.contains(&folder.id);
    ui.horizontal(|ui| {
        if ui.checkbox(&mut is_excluded, "").changed() {
            if is_excluded { excluded.insert(folder.id.clone()); }
            else { excluded.remove(&folder.id); }
        }
        if folder.children.is_empty() {
            ui.label(&folder.name);
        } else {
            ui.collapsing(&folder.name, |ui| {
                for child in &folder.children {
                    render_folder_tree(ui, child, excluded);
                }
            });
        }
    });
}
```

### 8. State Persistence (Window Position/Size)

**Decision**: Use eframe's built-in persistence

**Rationale**:
- eframe can save/restore window state automatically
- Stores in platform-appropriate location
- Minimal configuration required

**Configuration**:
```rust
let options = eframe::NativeOptions {
    persist_window: true,
    ..Default::default()
};
```

## Dependency Versions

| Crate | Version | Purpose |
|-------|---------|---------|
| eframe | 0.29 | Window management, egui integration |
| egui | 0.29 | Immediate-mode GUI |
| egui_extras | 0.29 | Additional widgets (tables, images) |
| open | 5.0 | Open URLs in browser |
| html2text | 0.12 | HTML to plain text conversion |
| poll-promise | 0.3 | Async-to-sync bridge for egui |

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| egui styling limitations | Medium | Low | Accept functional over fancy; dark theme sufficient |
| Async complexity | Low | Medium | Well-documented patterns; poll_promise handles most cases |
| Tree widget complexity | Medium | Low | Start simple; iterate if needed |
| Cross-platform issues | Low | Medium | eframe handles; test on all platforms |

## Conclusion

All technical questions resolved. Ready for Phase 1 design and implementation.


