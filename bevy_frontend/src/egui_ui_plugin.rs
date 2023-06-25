use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::error::Error;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui::{self, Align2, Color32, Frame, Layout, Align, Ui, Margin, style::WidgetVisuals, Stroke, RichText, FontId, FontFamily, FontDefinitions, FontData, Rounding}};
use egui_extras::RetainedImage;
use futures::channel::mpsc::{Sender, Receiver, channel};
use shared::JsonTurtle;
use uuid::Uuid;

use crate::{MainTurtle, spawn_async, SelectTurtleEvent};

pub struct UiPlugin;

#[derive(Resource, Deref)]
struct RefreshButtonImg(RetainedImage);

#[derive(Resource)]
struct UiGate {
    all_turtles: Vec<JsonTurtle>,
    selected_turtle_uuid: Option<Uuid>,
    fetching: AtomicBool,
    fetching_tx: Sender<Result<Vec<JsonTurtle>, Box<dyn Error + Send + Sync>>>,
    fetching_rx: Receiver<Result<Vec<JsonTurtle>, Box<dyn Error + Send + Sync>>>
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = channel(8);

        app
            .insert_resource(MainTurtle(Arc::new(RwLock::new(None))))
            .insert_resource(RefreshButtonImg(
                egui_extras::RetainedImage::from_svg_bytes_with_size(
                    "update-icon.svg",
                    include_bytes!("../assets/update-icon.svg"),
                    egui_extras::image::FitTo::Original
                ).unwrap()
            ))
            .insert_resource(UiGate {
                all_turtles: vec![],
                selected_turtle_uuid: None,
                fetching: AtomicBool::new(false),
                fetching_tx: tx,
                fetching_rx: rx
            })
            .add_system(recive_turtle_list)
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
    image: Res<RefreshButtonImg>,
    gate: Res<UiGate>,
    mut ev_change: EventWriter<SelectTurtleEvent>
) {
    egui::panel::TopBottomPanel::new(egui::panel::TopBottomSide::Top, "aaa")
        //.pivot(Align2::LEFT_TOP)
        //.fixed_pos([0f32, 0f32])
        //.title_bar(false)
        .frame(Frame {
            fill: Color32::from_rgb(41, 37, 36),
            inner_margin: Margin {
                left: 4.,
                right: 8.,
                top: 0.,
                bottom: 0.,
            },
            ..default()
        })
        .resizable(false)
        .exact_height(48.0)
        //.auto_sized()
        .show(contexts.ctx_mut(), |ui| {
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                gate
                    .all_turtles
                    .iter()
                    .enumerate()
                    .for_each(|(i, turtle)| {
                        //User clicked this button
                        if turtle_button(ui, i) {

                        }
                    });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let margin = egui::Frame::none()
                        .fill(egui::Color32::from_rgb(6, 182, 212))
                        .outer_margin(Margin::symmetric(0., 10.))
                        //.outer_margin(Margin::same(4.))
                        .inner_margin(Margin::same(1.5))
                        .rounding(Rounding::same(9999.))
                        .stroke(Stroke::new(8., Color32::from_rgb(6, 182, 212)));

                    let image = margin.show(ui, |ui| {
                        //image.show_size(ui, [32., 32.].into()); 
                        image.show_max_size(ui, [32., 32.].into());
                    });

                    let response = image.response.interact(egui::Sense::click());

                    if response.clicked() && !gate.fetching.load(Ordering::Relaxed) {
                        gate.fetching.store(true, Ordering::Relaxed);

                        let mut tx = gate.fetching_tx.clone();
                        spawn_async(async move {
                            let res = fetch_turtles().await;
                            tx.try_send(res).expect("Cannot send fetch result to bevy"); 
                        })
                    }
                });

            });
        });
}

fn turtle_button(
    ui: &mut Ui,
    i: usize,
) -> bool {
    let margin = egui::Frame::none()
        .fill(egui::Color32::from_rgb(6, 182, 212))
        .outer_margin(Margin::same(4.))
        .inner_margin(Margin::same(1.))
        .rounding(Rounding::same(4.))
        .stroke(Stroke::new(6., Color32::from_rgb(21, 128, 61)));

    margin.show(ui, |ui| {
        ui.visuals_mut().widgets.hovered = ui.visuals().widgets.inactive;
        ui.visuals_mut().widgets.active = ui.visuals().widgets.inactive;
        
        let text = RichText::new(i.to_string())
            .color(Color32::BLACK)
            .size(20.)
            .font(FontId::new(20.0, FontFamily::Name("ui-sans-serif".into())));

        let button = egui::Button::new(text).frame(false);
        let response = ui.add_sized([46., 46.0], button);

        return response.clicked();
    });

    //Hopefuly unreachable
    return false;
}

fn recive_turtle_list(mut gate: ResMut<UiGate>) {
    while let Ok(response) = gate.fetching_rx.try_next() {
        let response = response.expect("Fetch UI channel closed");

        gate.fetching.store(false, Ordering::Relaxed);
        match response {
            Ok(new_turtles) => {
                //TODO: CHECK IF SELECTED UID EXIST IN NEW TURTLES!
                log::warn!("New turtle list: {new_turtles:?}");
                gate.all_turtles = new_turtles;
            },
            Err(err) => {
                log::error!("Cannot fetch turtle list: {err}")
            },
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn fetch_turtles() -> Result<Vec<JsonTurtle>, Box<dyn Error + Send + Sync>> {
    use crate::HTTP_BACKEND_URL;

    let url = format!("{}/turtle/list/", HTTP_BACKEND_URL);

    let resp = reqwest::get(&url)
        .await?
        .json::<Vec<JsonTurtle>>()
        .await?;

    Ok(resp)
}
