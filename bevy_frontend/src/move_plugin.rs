use std::time::Duration;

use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;
use futures::channel::mpsc::{self, Receiver, Sender};
use gloo_net::http::Request;
use shared::{JsonTurtleDirection, TurtleMoveResponse};
use wasm_bindgen_futures::spawn_local;

use crate::{
    ui_plugin::MainTurtle, MainCamera, MainTurtleObject, SelectTurtleEvent, WorldChangeEvent,
};

pub struct MovePlugin;

#[derive(Resource)]
struct MovePlugineGate {
    allow_move: bool,
    timer: Timer,
    handle_request: bool,
    move_sender: Sender<Option<TurtleMoveResponse>>,
    move_reciver: Receiver<Option<TurtleMoveResponse>>,
}

impl Plugin for MovePlugin {
    fn build(&self, app: &mut App) {
        let (move_tx, move_rx) = mpsc::channel(8);
        app.insert_resource(MovePlugineGate {
            allow_move: true,
            handle_request: false,
            move_sender: move_tx,
            move_reciver: move_rx,
            timer: Timer::new(Duration::from_millis(500), TimerMode::Repeating),
        })
        .add_system(control_timer)
        .add_system(keybord_input)
        .add_system(recive_notification)
        .add_system(on_turtle_change);
    }
}

fn control_timer(time: Res<Time>, mut gate: ResMut<MovePlugineGate>) {
    gate.timer.tick(time.delta());
    if gate.timer.finished() {
        gate.allow_move = true
    }
}

fn keybord_input(
    keys: Res<Input<KeyCode>>,
    main_turtle: Res<MainTurtle>,
    mut gate: ResMut<MovePlugineGate>,
) {
    if !gate.allow_move || gate.handle_request {
        return;
    }

    let direction = if keys.pressed(KeyCode::W) {
        JsonTurtleDirection::Forward
    } else if keys.pressed(KeyCode::S) {
        JsonTurtleDirection::Backward
    } else if keys.pressed(KeyCode::A) {
        JsonTurtleDirection::Left
    } else if keys.pressed(KeyCode::D) {
        JsonTurtleDirection::Right
    } else {
        return;
    };

    let guard = main_turtle
        .read()
        .expect("Cannot lock main turtle, should never happen!");
    let main_turtle_ref = match &*guard {
        Some(val) => val,
        None => return,
    };

    let uuid = main_turtle_ref.uuid.clone();
    drop(guard);

    gate.allow_move = false;
    gate.handle_request = true;

    let mut tx = gate.move_sender.clone();
    let main_turtle = main_turtle.clone();

    spawn_local(async move {
        let string_direction = direction.to_string();
        let path = format!("/turtle/{uuid}/move/");
        
        let resp = Request::put(&path)
            .body(string_direction)
            .send()
            .await;

        match resp {
            Ok(resp) => {
                let result = match resp.json::<TurtleMoveResponse>().await {
                    Ok(val) => val,
                    Err(err) => {
                        log::error!("Cannot parse move response as JSON! Err: {err}");
                        tx.try_send(None)
                            .expect("Cannot notify bevy move system (Err json)");
                        return;
                    },
                };

                tx.try_send(Some(result))
                    .expect("Cannot notify bevy move system (Ok)");
            },
            Err(err) => {
                log::error!("Put move request went wrong {err}");
                tx.try_send(None)
                    .expect("Cannot notify bevy move system (Err)");
                return;
            },
        }

        main_turtle
            .write()
            .expect("Cannot lock main turtle, should never happen!")
            .as_mut()
            .and_then(|main_turtle| {
                match direction {
                    JsonTurtleDirection::Right | JsonTurtleDirection::Left => {
                        main_turtle.rotation.rotate_self(&direction);
                    },
                    JsonTurtleDirection::Backward | JsonTurtleDirection::Forward => {
                        let (x_change, y_change, z_change) = direction.to_turtle_move_diff(&main_turtle.rotation);
                        main_turtle.x += x_change;
                        main_turtle.y += y_change;
                        main_turtle.z += z_change;
                    },
                };
                None::<()>
            });
    })
}

fn recive_notification(
    mut gate: ResMut<MovePlugineGate>,
    mut camera_query: Query<&mut PanOrbitCamera, With<MainCamera>>,
    mut turtle_object_query: Query<(&mut AnimationPlayer, &Name), With<MainTurtleObject>>,
    mut animations: ResMut<Assets<AnimationClip>>,
    mut world_change_writer: EventWriter<WorldChangeEvent>,
) {
    match gate.move_reciver.try_next() {
        Ok(val) => {
            match val {
                Some(val) => {
                    match val {
                        Some(response) => {
                            let (start_x, start_z, rot_y) =
                                rotation_to_start_loc(&response.rotation);

                            let (mut animation_player, name) = turtle_object_query.single_mut();
                            let mut animation = AnimationClip::default();

                            animation.add_curve_to_path(
                                EntityPath {
                                    parts: vec![name.clone()],
                                },
                                VariableCurve {
                                    keyframe_timestamps: vec![1.0],
                                    keyframes: Keyframes::Translation(vec![Vec3::new(
                                        start_x + response.x as f32,
                                        response.y as f32 + 0.5,
                                        start_z + response.z as f32,
                                    )]),
                                },
                            );
                            animation.add_curve_to_path(
                                EntityPath {
                                    parts: vec![name.clone()],
                                },
                                VariableCurve {
                                    keyframe_timestamps: vec![1.0],
                                    keyframes: Keyframes::Rotation(vec![Quat::from_euler(
                                        EulerRot::YXZ,
                                        rot_y,
                                        0.0,
                                        0.0,
                                    )]),
                                },
                            );

                            animation_player.start_with_transition(
                                animations.add(animation),
                                Duration::from_millis(500),
                            );

                            //turtle_transform.translation = Vec3::new(start_x + response.x as f32, response.y as f32 + 0.5, start_z + response.z as f32);
                            //turtle_transform.rotation = Quat::from_euler(EulerRot::YXZ, rot_y, 0.0, 0.0);

                            let mut camera = camera_query.single_mut();
                            camera.force_update = true;
                            camera.focus = Vec3::new(
                                0.5 + response.x as f32,
                                0.5 + response.y as f32,
                                0.5 + response.z as f32,
                            );

                            for change in response.changes {
                                world_change_writer.send(WorldChangeEvent(change));
                            }
                        }
                        None => {}
                    }
                    gate.handle_request = false
                }
                //This will not happen
                None => panic!("Bevy move system notify channel closed"),
            };
        }
        Err(_) => return,
    };
}

fn on_turtle_change(
    mut ev_change: EventReader<SelectTurtleEvent>,
    mut camera_query: Query<&mut PanOrbitCamera, With<MainCamera>>,
    mut turtle_object_query: Query<&mut Transform, With<MainTurtleObject>>,
) {
    for ev in ev_change.iter() {
        log::warn!("{:?}", ev);

        if let Some(turtle) = &ev.0 {
            let (start_x, start_z, rot_y) = rotation_to_start_loc(&turtle.rotation);

            let mut turtle_transform = turtle_object_query.single_mut();
            turtle_transform.translation = Vec3::new(
                start_x + turtle.x as f32,
                turtle.y as f32 + 0.5,
                start_z + turtle.z as f32,
            );
            turtle_transform.rotation = Quat::from_euler(EulerRot::YXZ, rot_y, 0.0, 0.0);

            let mut camera = camera_query.single_mut();
            camera.focus = Vec3::new(
                0.5 + turtle.x as f32,
                0.5 + turtle.y as f32,
                0.5 + turtle.z as f32,
            );
            camera.force_update = true;
        }
    }
}

fn rotation_to_start_loc(rot: &JsonTurtleDirection) -> (f32, f32, f32) {
    return match rot {
        JsonTurtleDirection::Forward => (0.0, 0.0, 0.0),
        JsonTurtleDirection::Backward => (1.0, 1.0, std::f32::consts::PI),
        JsonTurtleDirection::Right => (1.0, 0.0, std::f32::consts::PI * 1.5),
        JsonTurtleDirection::Left => (0.0, 1.0, std::f32::consts::PI / 2.0),
    };
}
