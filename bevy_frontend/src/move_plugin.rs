use std::time::Duration;

use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;
use futures::channel::mpsc::{Sender, Receiver, self};
use shared::{JsonTurtleRotation, WorldChange, TurtleMoveResponse, WorldChangeAction};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{RequestInit, Request, Response};

use crate::{ui_plugin::MainTurtle, MainTurtleObject, MainCamera, SelectTurtleEvent, WorldChangeEvent};

pub struct MovePlugin;

#[derive(Resource)]
struct MovePlugineGate {
    allow_move: bool,
    timer: Timer,
    handle_request: bool,
    move_sender: Sender<Option<TurtleMoveResponse>>,
    move_reciver: Receiver<Option<TurtleMoveResponse>>
}

impl Plugin for MovePlugin {
    fn build(&self, app: &mut App) {
        let (move_tx, move_rx) = mpsc::channel(8);
        app.insert_resource(MovePlugineGate {
            allow_move: true,
            handle_request: false,
            move_sender: move_tx,
            move_reciver: move_rx,
            timer: Timer::new(Duration::from_millis(500), TimerMode::Repeating)
        })
        .add_system(control_timer)
        .add_system(keybord_input)
        .add_system(recive_notification)
        .add_system(on_turtle_change);
    }
}

fn control_timer( 
    time: Res<Time>,
    mut gate: ResMut<MovePlugineGate>
) {
    gate.timer.tick(time.delta());
    if gate.timer.finished() {
        gate.allow_move = true
    }
}

fn keybord_input(
    keys: Res<Input<KeyCode>>,
    main_turtle: Res<MainTurtle>,
    mut gate: ResMut<MovePlugineGate>
) {
    if !gate.allow_move || gate.handle_request {
        return;
    }

    let direction = if keys.pressed(KeyCode::W) {
        JsonTurtleRotation::Forward
    } else if keys.pressed(KeyCode::S) {
        JsonTurtleRotation::Backward
    } else if keys.pressed(KeyCode::A) {
        JsonTurtleRotation::Left
    } else if keys.pressed(KeyCode::D) {
        JsonTurtleRotation::Right
    } else {
        return
    };

    let guard = main_turtle.read().expect("Cannot lock main turtle, should never happen!");
    let main_turtle = match &*guard {
        Some(val) => val,
        None => return
    };
    let uuid = main_turtle.uuid.clone();
    drop(guard);

    gate.allow_move = false;
    gate.handle_request = true;

    let mut tx = gate.move_sender.clone();

    spawn_local(async move {
        let string_direction = direction.to_string();

        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        
        let mut url = document.base_uri().expect("Base uri get fail").expect("No base uri");
        url.push_str("turtle/");
        url.push_str(&uuid.to_string());
        url.push_str("/move/");

        let mut opts = RequestInit::new();
        opts.method("PUT");
        opts.body(Some(&JsValue::from_str(&string_direction)));

        let request = Request::new_with_str_and_init(&url, &opts).expect("Cannot create new request");
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await.expect("Cannot fetch value");

        assert!(resp_value.is_instance_of::<Response>());
        let resp: Response = resp_value.dyn_into().expect("Cannot cast into response");

        if resp.status() != 200 {
            log::error!("Something went bad! :<");
            tx.try_send(None).expect("Cannot notify bevy move system (Err)");
            return;
        }

        let json = JsFuture::from(resp.json().expect("Cannot get json")).await.expect("Cannot get future from JS");
        let result: TurtleMoveResponse = serde_wasm_bindgen::from_value(json).expect("Json serde error");
        tx.try_send(Some(result)).expect("Cannot notify bevy move system (Ok)");
    })
}

fn recive_notification(
    mut gate: ResMut<MovePlugineGate>,
    mut camera_query: Query<&mut PanOrbitCamera, With<MainCamera>>,
    mut turtle_object_query: Query<(&mut AnimationPlayer, &Name), With<MainTurtleObject>>,
    mut animations: ResMut<Assets<AnimationClip>>,
    mut world_change_writer: EventWriter<WorldChangeEvent>
) {
   match gate.move_reciver.try_next() {
       Ok(val) => {
           match val {
                Some(val) => {
                    match val {
                        Some(response) => {
                            let (start_x, start_z, rot_y) = rotation_to_start_loc(&response.rotation); 

                            let (mut animation_player, name) = turtle_object_query.single_mut();
                            let mut animation = AnimationClip::default();

                            animation.add_curve_to_path(
                                EntityPath { parts: vec![name.clone()] },
                                VariableCurve {
                                    keyframe_timestamps: vec![1.0],
                                    keyframes: Keyframes::Translation(
                                        vec![
                                            Vec3::new(start_x + response.x as f32, response.y as f32 + 0.5, start_z + response.z as f32),
                                        ]
                                    )
                                }
                            );
                            animation.add_curve_to_path(
                                EntityPath { parts: vec![name.clone()] },
                                VariableCurve {
                                    keyframe_timestamps: vec![1.0],
                                    keyframes: Keyframes::Rotation(
                                        vec![
                                            Quat::from_euler(EulerRot::YXZ, rot_y, 0.0, 0.0)
                                        ]
                                    )
                                }
                            );
                            
                            animation_player.start_with_transition(animations.add(animation), Duration::from_millis(500));
                
                            //turtle_transform.translation = Vec3::new(start_x + response.x as f32, response.y as f32 + 0.5, start_z + response.z as f32);
                            //turtle_transform.rotation = Quat::from_euler(EulerRot::YXZ, rot_y, 0.0, 0.0); 

                            let mut camera = camera_query.single_mut();
                            camera.force_update = true;
                            camera.focus = Vec3::new(0.5 + response.x as f32, 0.5 + response.y as f32, 0.5 + response.z as f32);
                        
                            for change in response.changes {
                                world_change_writer.send(WorldChangeEvent(change));
                            }
                        },
                        None => {}
                    }
                    gate.handle_request = false
                }
               //This will not happen
               None => panic!("Bevy move system notify channel closed")
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
                turtle_transform.translation = Vec3::new(start_x + turtle.x as f32, turtle.y as f32 + 0.5, start_z + turtle.z as f32);
                turtle_transform.rotation = Quat::from_euler(EulerRot::YXZ, rot_y, 0.0, 0.0); 

                let mut camera = camera_query.single_mut();
                camera.focus = Vec3::new(0.5 + turtle.x as f32, 0.5 + turtle.y as f32, 0.5 + turtle.z as f32);
                camera.force_update = true;
        } 
    }
}

fn rotation_to_start_loc(rot: &JsonTurtleRotation) -> (f32, f32, f32) {
    return match rot {
        JsonTurtleRotation::Forward => (0.0, 0.0, 0.0),
        JsonTurtleRotation::Backward => (1.0, 1.0, std::f32::consts::PI),
        JsonTurtleRotation::Right => (1.0, 0.0, std::f32::consts::PI * 1.5),
        JsonTurtleRotation::Left => (0.0, 1.0, std::f32::consts::PI / 2.0),
    };
}
