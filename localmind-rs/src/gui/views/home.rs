//! Home view showing recent documents

use egui::Ui;
use egui_remixicon::icons;

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
                                let card_fill = if doc.is_needs_auth {
                                    if ui.visuals().dark_mode {
                                        egui::Color32::from_rgb(50, 40, 20) // Dark amber tint
                                    } else {
                                        egui::Color32::from_rgb(255, 248, 230) // Light amber tint
                                    }
                                } else if ui.visuals().dark_mode {
                                    egui::Color32::from_rgb(30, 40, 60) // Dark blue-gray
                                } else {
                                    egui::Color32::from_gray(245)
                                };

                                let response = egui::Frame::none()
                                    .fill(card_fill)
                                    .rounding(4.0)
                                    .inner_margin(12.0)
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());

                                        // Title row with optional auth badge
                                        ui.horizontal(|ui| {
                                            if doc.is_needs_auth {
                                                ui.colored_label(
                                                    egui::Color32::from_rgb(200, 150, 0),
                                                    icons::LOCK_LINE,
                                                );
                                            }
                                            ui.strong(&doc.title);
                                        });

                                        // URL if present
                                        if let Some(ref url) = doc.url {
                                            ui.weak(truncate_url(url, 80));
                                        }

                                        ui.add_space(4.0);

                                        // Content snippet (extract after bookmark metadata if present)
                                        let snippet_text = if doc.content.starts_with("Bookmark:") {
                                            // Find the first double newline (end of metadata section)
                                            if let Some(content_start) = doc.content.find("\n\n") {
                                                let actual_content =
                                                    doc.content[content_start + 2..].trim();
                                                if !actual_content.is_empty() {
                                                    Some(create_snippet(actual_content, 150))
                                                } else {
                                                    None
                                                }
                                            } else {
                                                None
                                            }
                                        } else {
                                            Some(create_snippet(&doc.content, 150))
                                        };

                                        if let Some(snippet) = snippet_text {
                                            ui.label(&snippet);
                                        }

                                        ui.add_space(4.0);

                                        // Metadata
                                        ui.horizontal(|ui| {
                                            ui.weak(&doc.source);
                                            ui.weak("•");
                                            ui.weak(&doc.created_at);
                                        });

                                        if doc.is_needs_auth {
                                            ui.add_space(4.0);
                                            ui.colored_label(
                                                egui::Color32::from_rgb(200, 150, 0),
                                                "Login required - open link and use extension to re-capture",
                                            );
                                        }
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

/// Create a content snippet, truncating at char boundaries.
/// Strips YAML frontmatter so `---\n{}\n---` never leaks into the UI.
fn create_snippet(content: &str, max_len: usize) -> String {
    let content = strip_frontmatter(content);
    if content.len() <= max_len {
        return content.to_string();
    }

    // Walk back to a valid char boundary so we never slice mid-codepoint.
    let boundary = content
        .char_indices()
        .map(|(i, _)| i)
        .take_while(|&i| i <= max_len)
        .last()
        .unwrap_or(0);

    let truncated = &content[..boundary];
    if let Some(last_space) = truncated.rfind(char::is_whitespace) {
        format!("{}...", &content[..last_space])
    } else {
        format!("{}...", truncated)
    }
}

fn strip_frontmatter(s: &str) -> &str {
    if !s.starts_with("---") {
        return s;
    }
    let after_open = &s["---".len()..];
    if let Some(close) = after_open.find("\n---") {
        after_open[close + "\n---".len()..].trim_start_matches('\n')
    } else {
        s
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
