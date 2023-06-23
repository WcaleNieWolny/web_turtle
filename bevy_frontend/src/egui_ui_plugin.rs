use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui::{self, Align2, Color32, Frame}};

use crate::MainTurtle;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(MainTurtle(Arc::new(RwLock::new(None))))
            .add_system(draw_egui_ui);
    }
}

fn draw_egui_ui(
    mut contexts: EguiContexts,
) {
    egui::panel::TopBottomPanel::new(egui::panel::TopBottomSide::Top, "aaa")
        //.pivot(Align2::LEFT_TOP)
        //.fixed_pos([0f32, 0f32])
        //.title_bar(false)
        .frame(Frame {
            fill: Color32::from_rgb(41, 37, 36),
            ..default()
        })
        .resizable(false)
        .exact_height(48.0)
        //.auto_sized()
        .show(contexts.ctx_mut(), |ui| {
            ui.set_height(ui.available_height() / 8.0);
            println!("WIDTH: {:?}", ui.available_height());
        });

}
