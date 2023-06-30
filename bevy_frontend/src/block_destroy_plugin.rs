use bevy::prelude::*;
use bevy_mod_raycast::{DefaultRaycastingPlugin, RaycastMethod, RaycastSource, RaycastSystem};
use futures::channel::mpsc::{channel, Receiver, Sender};
use shared::{DestroyBlockResponse, JsonTurtleDirection};
use std::error::Error;
use uuid::Uuid;

#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen::{JsCast, JsValue},
    wasm_bindgen_futures::{spawn_local, JsFuture},
    web_sys::{Request, RequestInit, Response},
};

use crate::{spawn_async, BlockRaycastSet, MainTurtle, WorldChangeEvent};

pub struct BlockDestroyPlugin;

#[derive(Resource)]
struct BlockDestroyGate {
    destroy_sender: Sender<DestroyBlockResponse>,
    destroy_reciver: Receiver<DestroyBlockResponse>,
}

fn update_raycast_with_cursor(
    mut cursor: EventReader<CursorMoved>,
    mut query: Query<&mut RaycastSource<BlockRaycastSet>>,
) {
    // Grab the most recent cursor event if it exists:
    let cursor_position = match cursor.iter().last() {
        Some(cursor_moved) => cursor_moved.position,
        None => return,
    };

    for mut pick_source in &mut query {
        pick_source.cast_method = RaycastMethod::Screenspace(cursor_position);
    }
}

fn detect_block_destroy_from_mouse(
    keyboard: Res<Input<MouseButton>>,
    query_ray: Query<&mut RaycastSource<BlockRaycastSet>>,
    transform_query: Query<&Transform>,
    main_turtle: Res<MainTurtle>,
    destroy_block_gate: Res<BlockDestroyGate>,
) {
    if keyboard.just_pressed(MouseButton::Middle) {
        log::warn!("Click!");
        for entity in &query_ray {
            if let Some((entity, _)) = entity.get_nearest_intersection() {
                let transform = transform_query
                    .get_component::<Transform>(entity)
                    .expect("Entity does not contain transform!");
                let transform = transform.translation;
                let (x, y, z) = (transform.x - 0.5, transform.y - 1.0, transform.z - 0.5);
                let guard = main_turtle
                    .read()
                    .expect("Cannot lock main turtle, should never happen!");
                let main_turtle_ref = match &*guard {
                    Some(val) => val,
                    None => return,
                };

                let uuid = main_turtle_ref.uuid;

                let (forward_x, forward_y, forward_z) =
                    JsonTurtleDirection::Forward.to_turtle_move_diff(&main_turtle_ref.rotation);
                let (forward_x, forward_y, forward_z) = (
                    forward_x as f32 + main_turtle_ref.x as f32,
                    main_turtle_ref.y as f32 + forward_y as f32,
                    forward_z as f32 + main_turtle_ref.z as f32,
                );
                let (back_x, back_y, back_z) =
                    JsonTurtleDirection::Backward.to_turtle_move_diff(&main_turtle_ref.rotation);
                let (back_x, back_y, back_z) = (
                    back_x as f32 + main_turtle_ref.x as f32,
                    main_turtle_ref.y as f32 + back_y as f32,
                    back_z as f32 + main_turtle_ref.z as f32,
                );

                let direction = if forward_x == x && forward_z == z && forward_y == y {
                    JsonTurtleDirection::Forward
                } else if back_z == z && back_y == y && back_x == x {
                    JsonTurtleDirection::Backward
                } else if x == main_turtle_ref.x as f32
                    && y == main_turtle_ref.y as f32 + 1.0
                    && z == main_turtle_ref.z as f32
                {
                    //TODO: ??;
                    return;
                } else {
                    return;
                };

                drop(guard);

                let mut tx = destroy_block_gate.destroy_sender.clone();

                spawn_async(async move {
                    let string_direction = direction.to_string();

                    #[cfg(target_arch = "wasm32")]
                    {
                        let window = web_sys::window().expect("no global `window` exists");
                        let document = window.document().expect("should have a document on window");

                        let mut url = document
                            .base_uri()
                            .expect("Base uri get fail")
                            .expect("No base uri");
                        url.push_str("turtle/");
                        url.push_str(&uuid.to_string());
                        url.push_str("/destroy/");

                        let mut opts = RequestInit::new();
                        opts.method("PUT");
                        opts.body(Some(&JsValue::from_str(&string_direction)));

                        let request = Request::new_with_str_and_init(&url, &opts)
                            .expect("Cannot create new request");
                        let resp_value = JsFuture::from(window.fetch_with_request(&request))
                            .await
                            .expect("Cannot fetch value");

                        assert!(resp_value.is_instance_of::<Response>());
                        let resp: Response =
                            resp_value.dyn_into().expect("Cannot cast into response");

                        if resp.status() != 200 {
                            log::error!("Something went bad! :<");
                            return;
                        }

                        let json = JsFuture::from(resp.json().expect("Cannot get json"))
                            .await
                            .expect("Cannot get future from JS");
                        let result: DestroyBlockResponse =
                            serde_wasm_bindgen::from_value(json).expect("Json serde error");
                        tx.try_send(result)
                            .expect("Cannot send block destroy result to bevy");
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        todo!()
                    }
                });
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
async fn send_block_destroy_request(
    direction: &JsonTurtleDirection,
    uuid: &Uuid,
) -> Result<DestroyBlockResponse, Box<dyn Error>> {
    use gloo_net::http::Request;

    let response = Request::put(&format!("/turtle/{uuid}/destroy/"))
        .body(direction.to_string())
        .send()
        .await?
        .json::<DestroyBlockResponse>()
        .await?;

    Ok(response)
}

fn detect_block_destroy_response(
    mut world_change_writer: EventWriter<WorldChangeEvent>,
    mut gate: ResMut<BlockDestroyGate>,
) {
    if let Ok(response) = gate.destroy_reciver.try_next() {
        let response = response.expect("DestroyBlockResponse channel SHOULD never be closed!");

        if let Some(change) = response.change {
            world_change_writer.send(WorldChangeEvent(change))
        }
    }
}

impl Plugin for BlockDestroyPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = channel::<DestroyBlockResponse>(8);
        app.add_plugin(DefaultRaycastingPlugin::<BlockRaycastSet>::default())
            .insert_resource(BlockDestroyGate {
                destroy_sender: tx,
                destroy_reciver: rx,
            })
            .add_system(
                update_raycast_with_cursor
                    .in_base_set(CoreSet::First)
                    .before(RaycastSystem::BuildRays::<BlockRaycastSet>),
            )
            .add_system(detect_block_destroy_from_mouse.after(update_raycast_with_cursor))
            .add_system(detect_block_destroy_response);
    }
}
