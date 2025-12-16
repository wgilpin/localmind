//! Folder tree widget for displaying bookmark folders hierarchically

use crate::gui::state::BookmarkFolderView;
use egui::Ui;
use std::collections::HashSet;

/// Render a recursive folder tree with checkboxes
///
/// Displays folders hierarchically with checkboxes for exclusion selection.
/// Returns true if any checkbox state changed.
pub fn render_folder_tree(
    ui: &mut Ui,
    folders: &[BookmarkFolderView],
    excluded_folders: &mut HashSet<String>,
) -> bool {
    let mut changed = false;

    for folder in folders {
        ui.horizontal(|ui| {
            // Checkbox for this folder
            let is_excluded = excluded_folders.contains(&folder.id);
            let mut checked = is_excluded;

            if ui.checkbox(&mut checked, "").changed() {
                changed = true;
                if checked {
                    excluded_folders.insert(folder.id.clone());
                } else {
                    excluded_folders.remove(&folder.id);
                }
            }

            // Folder name with path context
            let folder_label = if folder.path.len() > 1 {
                format!("{} ({})", folder.name, folder.path.join(" > "))
            } else {
                folder.name.clone()
            };

            ui.label(folder_label);

            // Bookmark count
            if folder.bookmark_count > 0 {
                ui.weak(format!("({} bookmarks)", folder.bookmark_count));
            }
        });

        // Recursively render children with indentation
        if !folder.children.is_empty() {
            ui.indent(folder.id.clone(), |ui| {
                if render_folder_tree(ui, &folder.children, excluded_folders) {
                    changed = true;
                }
            });
        }
    }

    changed
}
