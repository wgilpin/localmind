//! Toast notification widget
//!
//! Provides rendering and styling for toast notifications.

use crate::gui::state::{Toast, ToastType};
use egui::{Color32, Context};

/// Render toast notifications in the bottom-right corner
///
/// Displays up to 5 toasts, with the most recent on top.
/// Toasts are automatically styled based on their type.
pub fn render_toasts(ctx: &Context, toasts: &[Toast]) {
    if toasts.is_empty() {
        return;
    }

    egui::Area::new(egui::Id::new("toast_area"))
        .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // Show most recent toasts first (reverse order)
                for toast in toasts.iter().rev().take(5) {
                    let color = get_toast_color(toast.toast_type);

                    egui::Frame::none()
                        .fill(color)
                        .rounding(4.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.colored_label(Color32::WHITE, &toast.message);
                        });
                    ui.add_space(4.0);
                }
            });
        });
}

/// Get the color for a toast based on its type
fn get_toast_color(toast_type: ToastType) -> Color32 {
    match toast_type {
        ToastType::Info => Color32::from_rgb(70, 130, 180), // Steel blue
        ToastType::Success => Color32::from_rgb(60, 179, 113), // Medium sea green
        ToastType::Error => Color32::from_rgb(220, 20, 60), // Crimson
    }
}


