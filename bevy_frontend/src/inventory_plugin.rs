use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiContexts, egui::{self, Align2, ScrollArea, TextStyle, FontFamily::Proportional, FontId, Visuals, Color32}};
use bevy_panorbit_camera::PanOrbitCamera;
use shared::TurtleInventoryItem;

pub struct InventoryPlugin;

#[derive(Resource)]
struct TurtleInventoryResource {
    list: Vec<TurtleInventoryItem>,
    open: bool
}

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(TurtleInventoryResource {
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
                ],
                open: false
            })
            .add_plugin(EguiPlugin)
            .add_startup_system(setup_text_styles)
            .add_system(open_ui_based_on_keyboard)
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

    let mut visuals = Visuals::default();
    visuals.panel_fill = Color32::from_rgb(41, 37, 36);
    ctx.set_visuals(visuals);
}

fn open_ui_based_on_keyboard(
    keys: Res<Input<KeyCode>>,
    mut inventory_res: ResMut<TurtleInventoryResource>,
    mut camera_query: Query<&mut PanOrbitCamera>
) {
    if keys.just_pressed(KeyCode::E) {
        inventory_res.open = !inventory_res.open;
    }

    let mut camera = camera_query.single_mut();

    if inventory_res.open {
        camera.orbit_sensitivity = 0.0;
        camera.zoom_sensitivity = 0.0;
    } else {
        camera.orbit_sensitivity = 1.0;
        camera.zoom_sensitivity = 1.0;
    }
}

fn ui_example_system(
    mut contexts: EguiContexts,
    mut inventory_res: ResMut<TurtleInventoryResource>,
    window: Query<&Window>,
) {
    let window = window.single();
    let open = &mut inventory_res.open.clone();

    egui::Window::new("Turtle Inventory")
        .pivot(Align2::CENTER_CENTER)
        .fixed_pos([window.width() / 2.0, window.height() / 2.0])
        .collapsible(false)
        .resizable(false)
        .open(open)
        .show(contexts.ctx_mut(), |ui| {
            ScrollArea::vertical().auto_shrink([false; 2]).show_rows(
                ui,
                ui.text_style_height(&TextStyle::Body),
                inventory_res.list.len(),
                |ui, row_range| {
                    row_range
                        .into_iter()
                        .for_each(|row| {
                            let item = &inventory_res.list[row];
                            let lablel = ui.selectable_label(item.selected, inventory_res.list[row].name.clone());

                            if !lablel.clicked() {
                                return;
                            }

                            inventory_res.list
                                .iter_mut()
                                .enumerate()
                                .for_each(|(id, item)| {
                                    item.selected = id == row;
                                })
                        });
                }
            );
        });

    inventory_res.open = *open;
}
