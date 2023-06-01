use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiContexts, egui::{self, Align2, ScrollArea, TextStyle}};

pub struct InventoryPlugin;

static ITEMS: &'static [&'static str] = &["diamond_block", "gold_block"];

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin)
            .add_system(ui_example_system);
    }
}

fn ui_example_system(
    mut contexts: EguiContexts,
    window: Query<&Window> 
) {
    let window = window.single();
    egui::Window::new("Hello")
        .pivot(Align2::CENTER_CENTER)
        .fixed_pos([window.width() / 2.0, window.height() / 2.0])
        .collapsible(false)
        .show(contexts.ctx_mut(), |ui| {
            ScrollArea::vertical().auto_shrink([false; 2]).show_rows(
                ui,
                ui.text_style_height(&TextStyle::Body),
                ITEMS.len(),
                |ui, row_range| {
                    for row in row_range {
                        ui.label(ITEMS[row]);
                    }
                }
            );
        });
}
