//! Document detail view showing full content

use egui::Ui;
use egui_commonmark::CommonMarkViewer;
use egui_remixicon::icons;

use crate::gui::app::LocalMindApp;
use crate::gui::state::View;

/// Render the document detail view
pub fn render_document_view(ui: &mut Ui, app: &mut LocalMindApp) {
    ui.add_space(10.0);

    // Get the selected document (must exist if we're on this view)
    let doc = match &app.selected_document {
        Some(doc) => doc.clone(),
        None => {
            // Shouldn't happen, but handle gracefully
            ui.label("No document selected");
            let back_button = ui.button(icons::ARROW_LEFT_LINE);

            if back_button.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            if back_button.clicked() {
                app.current_view = View::SearchResults;
            }
            return;
        }
    };

    // Header with back button
    ui.horizontal(|ui| {
        // Back button with icon
        let back_button = ui.button(icons::ARROW_LEFT_LINE);

        if back_button.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        if back_button.clicked() {
            app.current_view = View::SearchResults;
            app.selected_document = None;
        }

        ui.add_space(10.0);

        // Title
        ui.heading(&doc.title);
    });

    ui.add_space(10.0);

    // Auth-required banner
    if doc.is_needs_auth {
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(255, 243, 205))
            .rounding(4.0)
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(
                        egui::Color32::from_rgb(150, 100, 0),
                        format!("{} This page requires authentication.", icons::LOCK_LINE),
                    );
                });
                ui.colored_label(
                    egui::Color32::from_rgb(150, 100, 0),
                    "Open the link below in your browser, log in, then use the Chrome extension to re-capture the content.",
                );
            });
        ui.add_space(8.0);
    }

    // URL as clickable link
    if let Some(ref url) = doc.url {
        ui.horizontal(|ui| {
            ui.weak("Source: ");
            if ui.link(url).clicked() {
                // Open URL in default browser
                if let Err(e) = open::that(url) {
                    eprintln!("Failed to open URL: {}", e);
                }
            }
        });
    }

    // Metadata row
    ui.horizontal(|ui| {
        ui.weak(&doc.source);
        ui.weak("•");
        ui.weak(&doc.created_at);
    });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    // Determine if this is a local markdown file
    let is_local_md = doc
        .url
        .as_deref()
        .map(|u| u.starts_with("file://") && u.ends_with(".md"))
        .unwrap_or(false);

    // For local .md files, read and render from disk; fall back to DB content if unavailable.
    let markdown_source: Option<String> = if is_local_md {
        doc.url
            .as_deref()
            .and_then(|u| u.strip_prefix("file://"))
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|s| prepare_markdown(&s))
    } else {
        None
    };

    // Scrollable content area
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if let Some(md) = markdown_source {
                // Render Markdown for local .md files
                CommonMarkViewer::new().show(ui, &mut app.markdown_cache, &md);
            } else {
                // Extract actual content, skipping bookmark metadata if present
                let display_content = if doc.content.starts_with("Bookmark:") {
                    if let Some(content_start) = doc.content.find("\n\n") {
                        let actual_content = doc.content[content_start + 2..].trim();
                        if actual_content.is_empty() {
                            None
                        } else {
                            Some(actual_content.to_string())
                        }
                    } else {
                        None
                    }
                } else {
                    Some(doc.content.clone())
                };

                if let Some(mut content) = display_content {
                    ui.add(
                        egui::TextEdit::multiline(&mut content)
                            .desired_width(f32::INFINITY)
                            .font(egui::TextStyle::Body)
                            .interactive(false),
                    );
                } else {
                    ui.label("No content available for this bookmark.");
                }
            }
        });
}

/// Prepare Markdown content for rendering:
/// - Strip YAML frontmatter (`---` … `---` at the start of the file)
/// - Convert HTML `<br>` / `<br/>` tags to Markdown line breaks (two spaces + newline)
///   since egui_commonmark does not interpret inline HTML
fn prepare_markdown(content: &str) -> String {
    let body = strip_frontmatter(content);
    // <br> and variants → two trailing spaces then newline (CommonMark hard line break)
    let re_br = regex::Regex::new(r"(?i)<br\s*/?>").unwrap();
    re_br.replace_all(body, "  \n").into_owned()
}

/// Remove a YAML frontmatter block if the file starts with `---`.
fn strip_frontmatter(content: &str) -> &str {
    let s = content.trim_start();
    if !s.starts_with("---") {
        return content;
    }
    // Skip the opening `---` line and find the closing `---`
    let after_open = &s["---".len()..];
    // Closing delimiter must be `---` on its own line
    if let Some(close) = after_open.find("\n---") {
        let after_close = &after_open[close + "\n---".len()..];
        // Skip optional trailing newline after closing delimiter
        after_close.trim_start_matches('\n')
    } else {
        content
    }
}
