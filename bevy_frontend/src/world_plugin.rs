use bevy::prelude::*;
use futures::channel::mpsc::{channel, Receiver, Sender};
use shared::{JsonTurtle, TurtleWorld};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Request, RequestInit, Response};

use crate::{SelectTurtleEvent, WorldChangeEvent, BlockRaycastSet};

pub struct WorldPlugin;

//Marker for a world block
#[derive(Component)]
struct WorldBlock;

#[derive(Resource)]
struct GlobalWorld {
    get_all_blocks_rx: Receiver<Option<TurtleWorld>>,
    get_all_blocks_tx: Sender<Option<TurtleWorld>>,
}

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = channel::<Option<TurtleWorld>>(8);

        app.insert_resource(GlobalWorld {
            get_all_blocks_rx: rx,
            get_all_blocks_tx: tx,
        })
        .add_system(turtle_change_listener)
        .add_system(recive_all_new_world)
        .add_system(block_change_detect);
    }
}

fn recive_all_new_world(
    mut global_world: ResMut<GlobalWorld>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    match global_world.get_all_blocks_rx.try_next() {
        Ok(val) => {
            match val {
                Some(world) => {
                    match world {
                        Some(world) => {
                            for block in &world.blocks {
                                commands.spawn((
                                    PbrBundle {
                                        mesh: meshes.add(shape::Cube { size: 1.0 }.into()),
                                        material: materials
                                            .add(Color::rgb_u8(block.r, block.g, block.b).into()),
                                        transform: Transform::from_xyz(
                                            0.5 + block.x as f32,
                                            block.y as f32 + 1.0,
                                            0.5 + block.z as f32,
                                        ),
                                        ..default()
                                    },
                                    WorldBlock,
                                    bevy_mod_raycast::RaycastMesh::<BlockRaycastSet>::default()
                                    
                                ));
                            }
                        }
                        None => return, //Something went wrong
                    }
                }
                None => {
                    panic!("The global world channel closed! This SHOULD NEVER HAPPEN!");
                }
            }
        }
        Err(_) => return,
    }
}

fn turtle_change_listener(
    mut commands: Commands,
    mut select_turtle_reader: EventReader<SelectTurtleEvent>,
    world_blocks: Query<Entity, With<WorldBlock>>,
    global_world: Res<GlobalWorld>,
) {
    for event in &mut select_turtle_reader {
        //Clean the world
        for entity in world_blocks.iter() {
            //what the fuck?
            log::warn!("aaa");
            commands.entity(entity).despawn();
        }

        match &event.0 {
            Some(new_turtle) => {
                let uuid = new_turtle.uuid;
                let mut tx = global_world.get_all_blocks_tx.clone();

                spawn_local(async move {
                    let window = web_sys::window().expect("no global `window` exists");
                    let document = window.document().expect("should have a document on window");

                    let mut url = document
                        .base_uri()
                        .expect("Base uri get fail")
                        .expect("No base uri");
                    url.push_str("turtle/");
                    url.push_str(&uuid.to_string());
                    url.push_str("/world/");

                    let mut opts = RequestInit::new();
                    opts.method("GET");

                    let request = Request::new_with_str_and_init(&url, &opts)
                        .expect("Cannot create new request");
                    let resp_value = JsFuture::from(window.fetch_with_request(&request))
                        .await
                        .expect("Cannot fetch value");

                    assert!(resp_value.is_instance_of::<Response>());
                    let resp: Response = resp_value.dyn_into().expect("Cannot cast into response");

                    if resp.status() != 200 {
                        log::error!("Something went bad! (world) :<");
                        tx.try_send(None)
                            .expect("Cannot notify bevy world system (Err)");
                        return;
                    }

                    let json = JsFuture::from(resp.json().expect("Cannot get json"))
                        .await
                        .expect("Cannot get future from JS");
                    let result: TurtleWorld =
                        serde_wasm_bindgen::from_value(json).expect("Json serde error");
                    tx.try_send(Some(result))
                        .expect("Cannot notify bevy move system (Ok)");
                })
            }
            None => {}
        }
    }
}

fn block_change_detect(
    mut commands: Commands,
    mut world_change_events: EventReader<WorldChangeEvent>,
    world_blocks: Query<(&Transform, &Handle<StandardMaterial>, Entity), With<WorldBlock>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for change in &mut world_change_events {
        let change = &change.0;
        match &change.action {
            shared::WorldChangeAction::New(new_block) => {
                commands.spawn((
                    PbrBundle {
                        mesh: meshes.add(shape::Cube { size: 1.0 }.into()),
                        material: materials
                            .add(Color::rgb_u8(new_block.r, new_block.g, new_block.b).into()),
                        transform: Transform::from_xyz(
                            0.5 + change.x as f32,
                            change.y as f32 + 1.0,
                            0.5 + change.z as f32,
                        ),
                        ..default()
                    },
                    WorldBlock,
                    bevy_mod_raycast::RaycastMesh::<BlockRaycastSet>::default()
                ));
            }
            shared::WorldChangeAction::Update(update) => {
                let block = world_blocks.iter().find(|(loc, _, _)| {
                    loc.translation.x == change.x as f32 + 0.5
                        && loc.translation.y == change.y as f32 + 1.0
                        && loc.translation.z == change.z as f32 + 0.5
                });

                if let Some((_, handle, _)) = block {
                    let color = &mut materials.get_mut(handle).unwrap();
                    color.base_color = Color::hex(&update.color).unwrap()
                }
            }
            shared::WorldChangeAction::Delete(_) => {
                let block = world_blocks.iter().find(|(loc, _, _)| {
                    loc.translation.x == change.x as f32 + 0.5
                        && loc.translation.y == change.y as f32 + 1.0
                        && loc.translation.z == change.z as f32 + 0.5
                });

                log::warn!("Some: {}", block.is_some());
                if let Some((_, _, entity)) = block {
                    commands.entity(entity).despawn();
                }
            }
        };
    }
}
