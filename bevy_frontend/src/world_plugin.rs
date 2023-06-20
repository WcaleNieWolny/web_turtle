use bevy::render::mesh::{MeshVertexAttribute, Indices};
use bevy::render::render_resource::{PrimitiveTopology, VertexFormat};
use bevy::{prelude::*, pbr::wireframe::Wireframe};
use bytes::Bytes;
use futures::channel::mpsc::{channel, Receiver, Sender, unbounded, UnboundedReceiver, UnboundedSender};
use gloo_net::http::Request;
use shared::world_structure::{TurtleWorld, TurtleVoxel, ChunkLocation};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use block_mesh::ndshape::{ConstShape, ConstShape3u32};
use block_mesh::{greedy_quads, GreedyQuadsBuffer, MergeVoxel, Voxel, VoxelVisibility, RIGHT_HANDED_Y_UP_CONFIG};

use crate::{SelectTurtleEvent, WorldChangeEvent, BlockRaycastSet};

static CHUNKS_PER_FRAME_CAP: usize = 4;

pub struct WorldPlugin;

//Marker for a world block
#[derive(Component)]
struct WorldBlock;

#[derive(Resource)]
struct GlobalWorldGate {
    get_all_blocks_rx: Receiver<Option<TurtleWorld>>,
    get_all_blocks_tx: Sender<Option<TurtleWorld>>,
    chunk_load_rx: UnboundedReceiver<ChunkLocation>,
    chunk_load_tx: UnboundedSender<ChunkLocation>,
}

#[derive(Resource)]
struct GlobalWorld {
    world: Option<TurtleWorld>
}

#[derive(Clone, Copy, Eq, PartialEq)]
#[repr(transparent)]
struct BoolVoxel(TurtleVoxel);

impl Voxel for BoolVoxel {
    fn get_visibility(&self) -> VoxelVisibility {
        if self.0.id == 0 {
            VoxelVisibility::Empty
        } else {
            VoxelVisibility::Opaque
        }
    }
}

impl MergeVoxel for BoolVoxel {
    type MergeValue = Self;

    fn merge_value(&self) -> Self::MergeValue {
        *self
    }
}

// A 16^3 chunk with 1-voxel boundary padding.
type ChunkShape = ConstShape3u32<18, 18, 18>;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = channel::<Option<TurtleWorld>>(8);
        let (chunk_tx, chunk_rx) = unbounded::<ChunkLocation>();

        app.insert_resource(GlobalWorldGate {
            get_all_blocks_rx: rx,
            get_all_blocks_tx: tx,
            chunk_load_rx: chunk_rx,
            chunk_load_tx: chunk_tx,
        })
        .insert_resource(GlobalWorld {
            world: None
        })
        .add_system(turtle_change_listener)
        .add_system(recive_all_new_world)
        .add_system(block_change_detect)
        .add_system(load_chunk_from_queue.after(recive_all_new_world));
    }
}

fn load_chunk_from_queue(
    mut global_world_gate: ResMut<GlobalWorldGate>,
    mut global_world: ResMut<GlobalWorld>,
) {
    let world = match global_world.world.as_mut() {
        Some(world) => world,
        None => return
    };

    let (_, world_data) = world.get_fields_mut();

    //We will only ever load CHUNKS_PER_FRAME_CAP chunks per 1 frame
    let mut i = 0usize;

    while let Ok(chunk_loc) = global_world_gate.chunk_load_rx.try_next() {
        let chunk_loc = match chunk_loc {
            Some(val) => val,
            None => {
                log::error!("Closed chunk_loc channel. THIS SHOULD NEVER HAPPEN!!!");
                return;
            }
        };

        let chunk = match world_data.get_mut_chunk_by_loc(&chunk_loc) {
            Some(val) => val,
            None => {
                log::error!("Chunk {chunk_loc:?} does not exist client side");
                return;
            }
        };

        i += 1;
        if i == CHUNKS_PER_FRAME_CAP {
            return;
        }
    }
}

fn recive_all_new_world(
    mut global_world_gate: ResMut<GlobalWorldGate>,
    mut global_world: ResMut<GlobalWorld>,
) {
    match global_world_gate.get_all_blocks_rx.try_next() {
        Ok(val) => {
            match val {
                Some(world) => {
                    match world {
                        Some(mut world) => {
                            log::warn!("New world: {world:?}");
                            let (_, world_data) = world.get_fields_mut();
                            let res = world_data
                                .iter()
                                .map(|(loc, _)| loc)
                                .try_for_each(|loc| global_world_gate.chunk_load_tx.unbounded_send(loc.clone()));
                                
                            if let Err(err) = res {
                                log::error!("Cannot send turtle chunks loc into further processing. Err: {err}");
                                return;
                            }

                            global_world.world = Some(world);
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
    global_world_gate: Res<GlobalWorldGate>,
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
                let mut tx = global_world_gate.get_all_blocks_tx.clone();

                //-1 becouse idk yet
                //let (chunk_x, chunk_y, chunk_z) = (new_turtle.x >> 4, (new_turtle.y >> 4) - 1, new_turtle.z >> 4);

                spawn_local(async move {
                    let url = format!("/turtle/{uuid}/world/");

                    let resp = Request::get(&url)
                        .send()
                        .await;

                    match resp {
                        Ok(response) => {
                            let bytes_vec: Bytes = match response.binary().await {
                                Ok(val) => val.into(),
                                Err(err) => {
                                    log::error!("Something went wrong when converting response into bytes. Error: {err}");
                                    tx.try_send(None).expect("Cannot send world result");
                                    return;
                                }
                            };

                            let world = match TurtleWorld::from_bytes(bytes_vec) {
                                Ok(val) => val,
                                Err(err) => {
                                    log::error!("Cannot convert backend response into world. Error: {err}");
                                    tx.try_send(None).expect("Cannot send world result");
                                    return;
                                }
                            };

                            tx.try_send(Some(world)).expect("Cannot pass world into bevy system");
                        },
                        Err(err) => {
                            log::error!("Something went wrong when fetching world. Error: {err}");
                            tx.try_send(None).expect("Cannot send world result");
                        },
                    }
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
