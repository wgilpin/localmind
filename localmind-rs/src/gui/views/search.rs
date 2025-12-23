//! Search results view

use egui::Ui;
use egui_remixicon::icons;

use crate::gui::app::LocalMindApp;
use crate::gui::state::View;

/// Render the search results view
pub fn render_search_results(ui: &mut Ui, app: &mut LocalMindApp) {
    ui.add_space(10.0);

    // Header with back button and query
    ui.horizontal(|ui| {
        // Back button with icon
        let back_button = ui.button(icons::ARROW_LEFT_LINE);
        
        if back_button.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        
        if back_button.clicked() {
            app.current_view = View::Home;
            app.search_results.clear();
            app.all_results.clear();
        }

        ui.add_space(10.0);
        ui.heading(format!("Results for \"{}\"", app.search_query));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.weak(format!("{} results", app.search_results.len()));
        });
    });

    ui.add_space(10.0);

    // Similarity cutoff slider
    ui.horizontal(|ui| {
        ui.label("Similarity threshold:");
        let old_cutoff = app.similarity_cutoff;
        ui.add(egui::Slider::new(&mut app.similarity_cutoff, 0.0..=1.0).step_by(0.05));

        // Re-filter results if cutoff changed
        if (old_cutoff - app.similarity_cutoff).abs() > 0.001 {
            app.search_results = app
                .all_results
                .iter()
                .filter(|r| r.similarity >= app.similarity_cutoff)
                .cloned()
                .collect();
        }
    });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    // Check if search is in progress
    if app.is_search_pending() {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.spinner();
            ui.add_space(10.0);
            ui.label("Searching...");
        });
        return;
    }

    // No results message
    if app.search_results.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.label("No results found");
            ui.add_space(10.0);
            ui.weak("Try a different search query or lower the similarity threshold.");

            if app.similarity_cutoff > 0.1 {
                ui.add_space(20.0);
                if ui.button("Lower threshold and retry").clicked() {
                    app.similarity_cutoff = (app.similarity_cutoff - 0.1).max(0.0);
                    app.search_results = app
                        .all_results
                        .iter()
                        .filter(|r| r.similarity >= app.similarity_cutoff)
                        .cloned()
                        .collect();
                }
            }
        });
        return;
    }

    // Scrollable results list
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for result in &app.search_results.clone() {
                ui.push_id(result.doc_id, |ui| {
                    // Clickable result card
                    let response = egui::Frame::none()
                        .fill(if ui.visuals().dark_mode {
                            egui::Color32::from_gray(30)
                        } else {
                            egui::Color32::from_gray(245)
                        })
                        .rounding(4.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());

                            // Title row with similarity score
                            ui.horizontal(|ui| {
                                ui.strong(&result.title);

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        // Similarity score badge
                                        let score_color = similarity_color(result.similarity);
                                        egui::Frame::none()
                                            .fill(score_color)
                                            .rounding(3.0)
                                            .inner_margin(egui::vec2(6.0, 2.0))
                                            .show(ui, |ui| {
                                                ui.colored_label(
                                                    egui::Color32::WHITE,
                                                    format!("{:.0}%", result.similarity * 100.0),
                                                );
                                            });
                                    },
                                );
                            });

                            // URL if present
                            if let Some(ref url) = result.url {
                                ui.weak(truncate_url(url, 70));
                            }

                            ui.add_space(4.0);

                            // Content snippet (skip if snippet starts with "Bookmark:")
                            if !result.snippet.starts_with("Bookmark:") {
                                ui.label(&result.snippet);
                            }
                        });

                    // Handle click to view document
                    if response.response.interact(egui::Sense::click()).clicked() {
                        println!(
                            "Clicked search result: {} (id={})",
                            result.title, result.doc_id
                        );
                        app.load_document(result.doc_id);
                    }

                    // Hover effect
                    if response.response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                });

                ui.add_space(8.0);
            }

            // Load more button if there are hidden results
            let hidden_count = app.all_results.len() - app.search_results.len();
            if hidden_count > 0 {
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    if ui
                        .button(format!(
                            "Show {} more results (lower threshold)",
                            hidden_count
                        ))
                        .clicked()
                    {
                        app.similarity_cutoff = (app.similarity_cutoff - 0.1).max(0.0);
                        app.search_results = app
                            .all_results
                            .iter()
                            .filter(|r| r.similarity >= app.similarity_cutoff)
                            .cloned()
                            .collect();
                    }
                });
            }
        });
}

/// Get color based on similarity score
fn similarity_color(score: f32) -> egui::Color32 {
    if score >= 0.5 {
        egui::Color32::from_rgb(34, 139, 34) // Forest green - excellent
    } else if score >= 0.4 {
        egui::Color32::from_rgb(60, 179, 113) // Medium sea green - good
    } else if score >= 0.3 {
        egui::Color32::from_rgb(255, 165, 0) // Orange - moderate
    } else {
        egui::Color32::from_rgb(178, 34, 34) // Firebrick - low
    }
}

/// Truncate a URL for display
fn truncate_url(url: &str, max_len: usize) -> String {
    if url.len() <= max_len {
        url.to_string()
    } else {
        format!("{}...", &url[..max_len])
    }
}
