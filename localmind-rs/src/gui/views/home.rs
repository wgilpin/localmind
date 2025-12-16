//! Home view showing recent documents

use egui::Ui;

use crate::gui::app::LocalMindApp;
use crate::gui::state::InitStatus;

/// Render the home view with recent documents
pub fn render_home_view(ui: &mut Ui, app: &mut LocalMindApp) {
    ui.add_space(20.0);

    // Show different content based on init status
    match &app.init_status {
        InitStatus::Starting | InitStatus::WaitingForEmbedding => {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.spinner();
                ui.add_space(10.0);
                ui.label("Initializing LocalMind...");
                ui.add_space(5.0);
                ui.weak("Connecting to embedding server");
            });
        }
        InitStatus::Error(msg) => {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.colored_label(egui::Color32::RED, "⚠ Initialization Error");
                ui.add_space(10.0);
                ui.label(msg);
                ui.add_space(20.0);
                ui.weak("Please check that the Python embedding server is running.");
            });
        }
        InitStatus::Ready => {
            // Header
            ui.horizontal(|ui| {
                ui.heading("Recent Documents");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak(format!("{} documents", app.recent_documents.len()));
                });
            });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            if app.recent_documents.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label("No documents yet");
                    ui.add_space(10.0);
                    ui.weak("Save bookmarks in Chrome to get started, or use the extension to capture pages.");
                });
            } else {
                // Clone documents to avoid borrow issues
                let docs = app.recent_documents.clone();
                let mut clicked_doc_id: Option<i64> = None;

                // Scrollable list of recent documents
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for doc in &docs {
                            ui.push_id(doc.id, |ui| {
                                // Clickable frame for the document card
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

                                        // Title
                                        ui.horizontal(|ui| {
                                            ui.strong(&doc.title);
                                        });

                                        // URL if present
                                        if let Some(ref url) = doc.url {
                                            ui.weak(truncate_url(url, 80));
                                        }

                                        ui.add_space(4.0);

                                        // Content snippet
                                        let snippet = create_snippet(&doc.content, 150);
                                        ui.label(&snippet);

                                        ui.add_space(4.0);

                                        // Metadata
                                        ui.horizontal(|ui| {
                                            ui.weak(&doc.source);
                                            ui.weak("•");
                                            ui.weak(&doc.created_at);
                                        });
                                    });

                                // Handle click to view document
                                if response.response.interact(egui::Sense::click()).clicked() {
                                    clicked_doc_id = Some(doc.id);
                                }

                                // Show pointer cursor on hover
                                if response.response.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                            });

                            ui.add_space(8.0);
                        }
                    });

                // Handle click outside the loop to avoid borrow issues
                if let Some(doc_id) = clicked_doc_id {
                    app.load_document(doc_id);
                }
            }
        }
    }
}

/// Create a content snippet, truncating at word boundaries
fn create_snippet(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        return content.to_string();
    }

    // Find a good break point (whitespace)
    let truncated = &content[..max_len];
    if let Some(last_space) = truncated.rfind(char::is_whitespace) {
        format!("{}...", &content[..last_space])
    } else {
        format!("{}...", truncated)
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


