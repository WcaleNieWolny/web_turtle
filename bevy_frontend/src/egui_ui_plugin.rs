use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use bevy_egui::egui::Id;
use bevy_egui::{
    egui::{
        self, style::WidgetVisuals, Align, Align2, Color32, FontData, FontDefinitions, FontFamily,
        FontId, Frame, Layout, Margin, RichText, Rounding, Stroke, Ui,
    },
    EguiContexts,
};
use crossbeam_channel::{Sender, Receiver, bounded};
use egui_extras::RetainedImage;
use shared::JsonTurtle;

use crate::{spawn_async, MainTurtle, SelectTurtleEvent};

type DynError = Box<dyn Error + Sync + Send>;

pub struct UiPlugin;

#[derive(Resource, Deref)]
struct RefreshButtonImg(RetainedImage);

#[derive(Resource)]
struct UiGate {
    all_turtles: Vec<JsonTurtle>,
    selected_turtle: Option<usize>,
    fetching: AtomicBool,
    fetching_tx: Sender<Result<Vec<JsonTurtle>, DynError>>,
    fetching_rx: Receiver<Result<Vec<JsonTurtle>, DynError>>,
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = bounded(8);

        app.insert_resource(MainTurtle(Arc::new(RwLock::new(None))))
            .insert_resource(RefreshButtonImg(
                egui_extras::RetainedImage::from_svg_bytes_with_size(
                    "update-icon.svg",
                    include_bytes!("../assets/update-icon.svg"),
                    egui_extras::image::FitTo::Original,
                )
                .unwrap(),
            ))
            .insert_resource(UiGate {
                all_turtles: vec![],
                selected_turtle: None,
                fetching: AtomicBool::new(false),
                fetching_tx: tx,
                fetching_rx: rx,
            })
            .add_system(recive_turtle_list.after(draw_egui_ui))
            .add_startup_system(setup_font)
            .add_system(draw_egui_ui);
    }
}

fn setup_font(mut contexts: EguiContexts) {
    let ctx = contexts.ctx_mut();
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "ui-sans-serif".to_owned(),
        FontData::from_static(include_bytes!("../OpenSans-Regular.ttf")),
    );

    fonts.families.insert(
        FontFamily::Name("ui-sans-serif".into()),
        vec!["ui-sans-serif".into()],
    );

    ctx.set_fonts(fonts);
}

fn draw_egui_ui(
    mut contexts: EguiContexts,
    image: Res<RefreshButtonImg>,
    mut gate: ResMut<UiGate>,
    main_turtle: Res<MainTurtle>,
    mut ev_change: EventWriter<SelectTurtleEvent>,
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
                let UiGate {
                    all_turtles,
                    selected_turtle,
                    ..
                } = &mut *gate;
                let mut write_changed_turtle: Option<(usize, JsonTurtle)> = None;

                all_turtles.iter().enumerate().for_each(|(i, turtle)| {
                    //check if this is the main turtle
                    let is_main_turtle = match selected_turtle {
                        Some(main_id) => *main_id == i,
                        None => false,
                    };

                    //User clicked this button
                    if turtle_button(ui, i, is_main_turtle) {
                        if is_main_turtle {
                            return;
                        }

                        //We will wrtie to vec so the changes to main_turtle dont get lost
                        if let Some(id) = selected_turtle {
                            //We cannot block in the UI or it will be a terrible UX
                            if let Some(main_turtle_clone) =
                                &*main_turtle.try_read().expect("Cannot lock main_turtle")
                            {
                                write_changed_turtle = Some((*id, main_turtle_clone.clone()));
                            }
                        };

                        let mut writable_turtle = main_turtle
                            .try_write()
                            .expect("Cannot lock main_turtle for writing");
                        *writable_turtle = Some(turtle.clone());
                        *selected_turtle = Some(i);
                        ev_change.send(SelectTurtleEvent(Some(turtle.clone())))
                    }
                });

                if let Some((id, turtle)) = write_changed_turtle {
                    all_turtles[id] = turtle;
                }

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

fn turtle_button(ui: &mut Ui, i: usize, is_main_turtle: bool) -> bool {
    //This took some anoying testing but it SHOULD work (hopefuly some update does not break this)
    let (start_x, start_y) = (ui.cursor().left_top().x, ui.cursor().left_top().y);
    let hovered = ui
        .interact(
            bevy_egui::egui::Rect::from_x_y_ranges(
                start_x..=start_x + 56.,
                start_y..=start_y + 56.,
            ),
            Id::null(),
            egui::Sense::hover(),
        )
        .hovered();

    let bg_color = if hovered && !is_main_turtle {
        egui::Color32::from_rgb(34, 197, 94)
    } else {
        egui::Color32::from_rgb(6, 182, 212)
    };

    let (border_color, rounding) = if is_main_turtle {
        (Color32::from_rgb(21, 128, 61), 4.)
    } else {
        (bg_color.clone(), 0.)
    };

    let mut return_val = false;

    let margin = egui::Frame::none()
        .fill(bg_color)
        .outer_margin(Margin::same(4.))
        .inner_margin(Margin::same(1.))
        .rounding(Rounding::same(rounding))
        .stroke(Stroke::new(6., border_color));

    margin.show(ui, |ui| {
        ui.visuals_mut().widgets.hovered = ui.visuals().widgets.inactive;
        ui.visuals_mut().widgets.active = ui.visuals().widgets.inactive;

        let text = RichText::new(i.to_string())
            .color(Color32::BLACK)
            .size(20.)
            .font(FontId::new(20.0, FontFamily::Name("ui-sans-serif".into())));

        let button = egui::Button::new(text).frame(false);
        let response = ui.add_sized([46., 46.0], button);

        return_val = response.clicked();
    });

    return return_val;
}

fn recive_turtle_list(mut gate: ResMut<UiGate>) {
    while let Ok(response) = gate.fetching_rx.recv() {

        gate.fetching.store(false, Ordering::Relaxed);
        match response {
            Ok(new_turtles) => {
                //TODO: CHECK IF SELECTED UID EXIST IN NEW TURTLES!
                log::warn!("New turtle list: {new_turtles:?}");
                gate.all_turtles = new_turtles;
            }
            Err(err) => {
                log::error!("Cannot fetch turtle list: {err}")
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
async fn fetch_turtles() -> Result<Vec<JsonTurtle>, DynError> {
    use gloo_net::http::Request;

    let resp = Request::get("/turtle/list/")
        .send()
        .await?
        .json::<Vec<JsonTurtle>>()
        .await?;

    Ok(resp)
}

#[cfg(not(target_arch = "wasm32"))]
async fn fetch_turtles() -> Result<Vec<JsonTurtle>, Box<dyn Error + Send + Sync>> {
    use crate::HTTP_BACKEND_URL;

    let url = format!("{}/turtle/list/", HTTP_BACKEND_URL);

    let resp = reqwest::get(&url).await?.json::<Vec<JsonTurtle>>().await?;

    Ok(resp)
}
