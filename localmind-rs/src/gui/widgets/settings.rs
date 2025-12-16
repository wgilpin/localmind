//! Settings modal widget for managing exclusion rules

use crate::bookmark_exclusion::ExclusionRules;
use crate::gui::app::LocalMindApp;
use egui::Ui;

/// Render the settings modal content
///
/// Displays folder tree for exclusion selection and domain pattern management.
/// Returns true if settings should be closed (Save or Cancel clicked).
pub fn render_settings_modal(ui: &mut Ui, app: &mut LocalMindApp) -> bool {
    let mut should_close = false;
    ui.vertical(|ui| {
        ui.heading("Exclusion Rules");
        ui.add_space(10.0);

        // Folder exclusions section
        ui.collapsing("Exclude Bookmark Folders", |ui| {
            ui.add_space(5.0);

            if app.bookmark_folders.is_empty() {
                ui.weak("No bookmark folders found. Make sure Chrome bookmarks are available.");
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(200.0)
                    .show(ui, |ui| {
                        use crate::gui::widgets::folder_tree;
                        folder_tree::render_folder_tree(
                            ui,
                            &app.bookmark_folders,
                            &mut app.excluded_folders,
                        );
                    });
            }
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Domain exclusions section
        ui.collapsing("Exclude Domain Patterns", |ui| {
            ui.add_space(5.0);

            // Domain input field
            ui.horizontal(|ui| {
                ui.label("Domain pattern:");
                ui.text_edit_singleline(&mut app.pending_domain);

                if ui.button("Add").clicked() {
                    let pattern = app.pending_domain.trim().to_string();
                    if !pattern.is_empty() {
                        // Validate pattern
                        match ExclusionRules::validate_pattern(&pattern) {
                            Ok(_) => {
                                // Check for duplicates (case-insensitive)
                                let pattern_lower = pattern.to_lowercase();
                                if !app
                                    .excluded_domains
                                    .iter()
                                    .any(|d| d.to_lowercase() == pattern_lower)
                                {
                                    app.excluded_domains.push(pattern);
                                    app.pending_domain.clear();
                                } else {
                                    // Show error toast for duplicate
                                    let id = app.next_toast_id();
                                    app.add_toast(crate::gui::state::Toast::error(
                                        id,
                                        format!("Domain pattern '{}' already exists", pattern),
                                    ));
                                }
                            }
                            Err(e) => {
                                // Show validation error toast
                                let id = app.next_toast_id();
                                app.add_toast(crate::gui::state::Toast::error(
                                    id,
                                    format!("Invalid pattern: {}", e),
                                ));
                            }
                        }
                    }
                }
            });

            ui.add_space(5.0);

            // Domain pattern list
            if app.excluded_domains.is_empty() {
                ui.weak("No domain patterns excluded");
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(150.0)
                    .show(ui, |ui| {
                        let mut to_remove = None;
                        for (idx, domain) in app.excluded_domains.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(domain);
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button("Remove").clicked() {
                                            to_remove = Some(idx);
                                        }
                                    },
                                );
                            });
                        }

                        if let Some(idx) = to_remove {
                            app.excluded_domains.remove(idx);
                        }
                    });
            }

            ui.add_space(5.0);
            ui.weak("Examples: example.com, *.internal.com, localhost:*");
        });

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);

        // Action buttons
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Cancel").clicked() {
                    should_close = true;
                    // Reset to original values (will be reloaded on next open)
                }

                ui.add_space(10.0);

                // Check if save is in progress
                let save_in_progress = app.is_saving_exclusion_rules();

                ui.add_enabled_ui(!save_in_progress, |ui| {
                    if ui
                        .button(if save_in_progress {
                            "Saving..."
                        } else {
                            "Save"
                        })
                        .clicked()
                    {
                        // Save exclusion rules (async)
                        if let Err(e) = app.save_exclusion_rules() {
                            let id = app.next_toast_id();
                            app.add_toast(crate::gui::state::Toast::error(
                                id,
                                format!("Failed to start save: {}", e),
                            ));
                        }
                    }
                });
            });
        });
    });

    should_close
}
