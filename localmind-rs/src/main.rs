// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! LocalMind Desktop Application
//!
//! A native Rust desktop application using egui/eframe for the GUI.
//! Provides semantic search over bookmarks and documents.

use eframe::egui;
use localmind_rs::gui::LocalMindApp;

fn main() -> eframe::Result<()> {
    println!("Starting LocalMind application");

    // Try to load icon from embedded bytes (gracefully handle errors)
    let icon = load_icon(include_bytes!("../icons/icon.png"));

    // Configure window options
    let mut viewport_builder = egui::ViewportBuilder::default()
        .with_title("LocalMind")
        .with_inner_size([1024.0, 768.0])
        .with_min_inner_size([800.0, 600.0]);
    
    if let Some(icon) = icon {
        viewport_builder = viewport_builder.with_icon(icon);
    } else {
        eprintln!("Warning: Failed to load icon, continuing without icon");
    }

    let options = eframe::NativeOptions {
        viewport: viewport_builder,
        ..Default::default()
    };

    println!("Launching egui window");

    // Run the application
    eframe::run_native(
        "LocalMind",
        options,
        Box::new(|cc| {
            // Apply dark theme
            cc.egui_ctx.set_visuals(egui::Visuals::dark());

            // Create the app with creation context
            Ok(Box::new(LocalMindApp::new(cc)))
        }),
    )
}

/// Load icon from PNG bytes
/// Returns None if the icon cannot be loaded (corrupted file, etc.)
fn load_icon(png_data: &[u8]) -> Option<std::sync::Arc<egui::IconData>> {
    let image = match image::load_from_memory(png_data) {
        Ok(img) => img.into_rgba8(),
        Err(e) => {
            eprintln!("Failed to load icon: {}", e);
            return None;
        }
    };
    let (width, height) = image.dimensions();

    Some(std::sync::Arc::new(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }))
}
