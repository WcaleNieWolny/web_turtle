use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiContexts, egui::{self, Align2, ScrollArea, TextStyle, FontFamily::Proportional, FontId}};
use shared::TurtleInventoryItem;

pub struct InventoryPlugin;

#[derive(Resource)]
struct TurtleInventoryList {
    list: Vec<TurtleInventoryItem>
}

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(TurtleInventoryList {
                list: vec![
                    TurtleInventoryItem {
                        name: "gold_block".to_string(),
                        count: 1,
                        selected: false
                    },
                    TurtleInventoryItem {
                        name: "diamond_block".to_string(),
                        count: 10,
                        selected: true
                    }
                ]
            })
            .add_plugin(EguiPlugin)
            .add_startup_system(setup_text_styles)
            .add_system(ui_example_system);
    }
}

fn setup_text_styles(
    mut contexts: EguiContexts,
) {
    let ctx = contexts.ctx_mut();

    let mut style = (*ctx.style()).clone();

    style.text_styles = [
        (egui::style::TextStyle::Body, FontId::new(18.0, Proportional)),
        (egui::style::TextStyle::Heading, FontId::new(24.0, Proportional)),
        (egui::style::TextStyle::Button, FontId::new(18.0, Proportional)),
    ].into();

    ctx.set_style(style);
}

fn ui_example_system(
    mut contexts: EguiContexts,
    mut inventory: ResMut<TurtleInventoryList>,
    window: Query<&Window>,
) {
    let window = window.single();
    egui::Window::new("Turtle Inventory")
        .pivot(Align2::CENTER_CENTER)
        .fixed_pos([window.width() / 2.0, window.height() / 2.0])
        .collapsible(false)
        .resizable(false)
        .show(contexts.ctx_mut(), |ui| {
            ScrollArea::vertical().auto_shrink([false; 2]).show_rows(
                ui,
                ui.text_style_height(&TextStyle::Body),
                inventory.list.len(),
                |ui, row_range| {
                    row_range
                        .into_iter()
                        .zip(inventory.list.iter_mut())
                        .for_each(|(_row, item)| {
                            ui.toggle_value(&mut item.selected, item.name.clone());
                        });
                }
            );
        });
}
