//! Watched Folders widget — list, add, and remove watched directories.
//!
//! Filled in during Phase 3 (T025-T026, T036, T038-T039).

use crate::gui::app::LocalMindApp;
use egui::Ui;

/// Render the watched-folders management UI.
///
/// Shows the list of currently watched folders with status badges,
/// per-folder error lists, a progress bar during scanning, and
/// Add / Remove controls.
pub fn render_watched_folders(ui: &mut Ui, app: &mut LocalMindApp) {
    ui.heading("Watched Folders");
    ui.add_space(6.0);
    ui.weak("Add local folders to automatically index PDF, Markdown, and text files.");
    ui.add_space(10.0);

    // --- Add folder input row ---
    ui.horizontal(|ui| {
        ui.label("Folder path:");
        let response = ui.text_edit_singleline(&mut app.add_folder_input);
        let add_clicked = ui.button("Add").clicked();
        let enter_pressed = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

        if add_clicked || enter_pressed {
            let raw = app.add_folder_input.trim().trim_matches('\'').to_string();
            if raw.is_empty() {
                app.add_folder_error = Some("Please enter a folder path.".to_string());
            } else {
                let path = std::path::PathBuf::from(&raw);
                if !path.exists() {
                    app.add_folder_error =
                        Some("Path does not exist. Check the path and try again.".to_string());
                } else if !path.is_dir() {
                    app.add_folder_error =
                        Some("Path is not a directory. Select a folder, not a file.".to_string());
                } else {
                    // Clear error, send add command to service
                    app.add_folder_error = None;
                    if let Some(ref tx) = app.add_folder_tx {
                        let _ = tx.send(path);
                        app.add_folder_input.clear();
                    }
                }
            }
        }
    });

    // Show inline error if present
    if let Some(ref err) = app.add_folder_error.clone() {
        ui.add_space(2.0);
        ui.colored_label(egui::Color32::from_rgb(200, 60, 60), err);
    }

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);

    // --- Watched folder list ---
    if app.watched_folders.is_empty() {
        ui.weak("No folders are being watched. Add a folder above to get started.");
    } else {
        let folders = app.watched_folders.clone();
        for folder in &folders {
            render_folder_row(ui, app, folder);
            ui.add_space(4.0);
        }
    }
}

fn render_folder_row(
    ui: &mut Ui,
    app: &mut LocalMindApp,
    folder: &crate::folder_watcher::WatchedFolder,
) {
    use crate::folder_watcher::FolderStatus;

    egui::Frame::none()
        .fill(egui::Color32::from_gray(28))
        .inner_margin(egui::Margin::same(8.0))
        .rounding(egui::Rounding::same(4.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Status badge (T038)
                let (badge_color, badge_text) = match &folder.status {
                    FolderStatus::Active => (egui::Color32::from_rgb(60, 180, 80), "active"),
                    FolderStatus::Unavailable => {
                        (egui::Color32::from_rgb(200, 140, 40), "unavailable")
                    }
                    FolderStatus::Error(_) => (egui::Color32::from_rgb(200, 60, 60), "error"),
                };
                ui.colored_label(badge_color, format!("[{}]", badge_text));
                ui.label(folder.path.display().to_string());

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Remove button (T036)
                    if ui.button("Remove").clicked() {
                        if let Some(ref tx) = app.remove_folder_tx {
                            let _ = tx.send(folder.path.clone());
                        }
                    }
                });
            });

            // Progress bar during initial scan
            if let Some(progress) = app.folder_watch_progress.get(&folder.path) {
                let frac = if progress.files_total > 0 {
                    progress.files_done as f32 / progress.files_total as f32
                } else {
                    0.0
                };
                ui.add(egui::ProgressBar::new(frac).show_percentage());
                if let Some(ref cur) = progress.current_file {
                    ui.weak(format!("Indexing: {}", cur.display()));
                }
            }

            // Per-folder error list (T039)
            let error_files: Vec<_> = app
                .folder_file_errors
                .get(&folder.path)
                .cloned()
                .unwrap_or_default();
            if !error_files.is_empty() {
                ui.collapsing(format!("{} file error(s)", error_files.len()), |ui| {
                    for (file, err) in &error_files {
                        ui.horizontal(|ui| {
                            ui.colored_label(
                                egui::Color32::from_rgb(200, 60, 60),
                                file.display().to_string(),
                            );
                            ui.weak(format!(": {}", err));
                        });
                    }
                });
            }
        });
}
