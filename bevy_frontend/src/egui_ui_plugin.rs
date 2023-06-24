use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui::{self, Align2, Color32, Frame, Layout, Align, Ui, Margin, style::WidgetVisuals, Stroke, RichText, FontId, FontFamily, FontDefinitions, FontData, Rounding}};

use crate::MainTurtle;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(MainTurtle(Arc::new(RwLock::new(None))))
            .add_startup_system(setup_font)
            .add_system(draw_egui_ui);
    }
}

fn setup_font(mut contexts: EguiContexts) {
    let ctx = contexts.ctx_mut();
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert("ui-sans-serif".to_owned(),
       FontData::from_static(include_bytes!("../OpenSans-Regular.ttf")));

    fonts.families.insert(FontFamily::Name("ui-sans-serif".into()), vec!["ui-sans-serif".into()]);

    ctx.set_fonts(fonts);
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
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                turtle_button(ui);
                turtle_button(ui);
            });
        });
}

fn turtle_button(ui: &mut Ui) {
    let margin = egui::Frame::none()
        .fill(egui::Color32::from_rgb(6, 182, 212))
        .outer_margin(Margin::same(4.))
        .inner_margin(Margin::same(1.))
        .rounding(Rounding::same(4.))
        .stroke(Stroke::new(6., Color32::from_rgb(21, 128, 61)));

    margin.show(ui, |ui| {
        ui.visuals_mut().widgets.hovered = ui.visuals().widgets.inactive;
        ui.visuals_mut().widgets.active = ui.visuals().widgets.inactive;
        
        let text = RichText::new("8")
            .color(Color32::BLACK)
            .size(20.)
            .font(FontId::new(20.0, FontFamily::Name("ui-sans-serif".into())));

        ui.add_sized([46., 46.0], egui::Button::new(text).frame(false));
    });
}
