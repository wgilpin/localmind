//! Document detail view showing full content

use egui::Ui;

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
            if ui.button("← Back").clicked() {
                app.current_view = View::SearchResults;
            }
            return;
        }
    };

    // Header with back button
    ui.horizontal(|ui| {
        if ui.button("← Back").clicked() {
            app.current_view = View::SearchResults;
            app.selected_document = None;
        }

        ui.add_space(10.0);

        // Title
        ui.heading(&doc.title);
    });

    ui.add_space(10.0);

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

    // Scrollable content area
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Document content with better formatting
            ui.add(
                egui::TextEdit::multiline(&mut doc.content.as_str())
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Body)
                    .interactive(false), // Read-only
            );
        });
}


