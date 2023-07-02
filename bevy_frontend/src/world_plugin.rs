use std::error::Error;

use bevy::render::mesh::{Indices, MeshVertexAttribute, VertexAttributeValues};
use bevy::render::render_resource::{PrimitiveTopology, VertexFormat};
use bevy::{pbr::wireframe::Wireframe, prelude::*};
use block_mesh::ndshape::{ConstShape, ConstShape3u32};
use block_mesh::{
    greedy_quads, GreedyQuadsBuffer, MergeVoxel, Voxel, VoxelVisibility, RIGHT_HANDED_Y_UP_CONFIG,
};
use bytes::Bytes;
use futures::channel::mpsc::{
    channel, unbounded, Receiver, Sender, UnboundedReceiver, UnboundedSender,
};
use shared::{WorldChangePaletteEnum, WorldChange};
use shared::world_structure::{ChunkLocation, TurtleVoxel, TurtleWorld, TurtleWorldPalette, TurtleWorldData};
use uuid::Uuid;

use crate::chunk_material::{ChunkMaterialSingleton, VoxelTerrainMesh};
use crate::{spawn_async, BlockRaycastSet, SelectTurtleEvent, WorldChangeEvent};

static CHUNKS_PER_FRAME_CAP: usize = 4;

pub struct WorldPlugin;

//Marker for a world block
#[derive(Component, Deref)]
struct WorldChunk {
    location: ChunkLocation
}

#[derive(Resource)]
struct GlobalWorldGate {
    get_all_blocks_rx: Receiver<Option<TurtleWorld>>,
    get_all_blocks_tx: Sender<Option<TurtleWorld>>,
    chunk_load_rx: UnboundedReceiver<ChunkLocation>,
    chunk_load_tx: UnboundedSender<ChunkLocation>,
}

#[derive(Resource, Deref)]
pub struct GlobalWorld {
    world: Option<TurtleWorld>,
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

impl From<TurtleVoxel> for BoolVoxel {
    fn from(value: TurtleVoxel) -> Self {
        Self(value)
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
        .insert_resource(GlobalWorld { world: None })
        .add_system(turtle_change_listener)
        .add_system(recive_all_new_world)
        .add_system(block_change_detect)
        .add_system(load_chunk_from_queue.after(recive_all_new_world));
    }
}

fn load_chunk_from_queue(
    mut global_world_gate: ResMut<GlobalWorldGate>,
    mut global_world: ResMut<GlobalWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<crate::chunk_material::ChunkMaterialSingleton>,
    world_chunks: Query<(Entity, &WorldChunk)>,
    mut commands: Commands,
) {
    let world = match global_world.world.as_mut() {
        Some(world) => world,
        None => return,
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

        let previous_mesh = world_chunks
            .iter()
            .find(|(_, loc)| loc.location == chunk_loc);

        if let Some((previous_mesh, ..)) = previous_mesh {
            commands.entity(previous_mesh).despawn();
        }

        let chunk = match world_data.get_mut_chunk_by_loc(&chunk_loc) {
            Some(val) => val,
            None => {
                log::error!("Chunk {chunk_loc:?} does not exist client side");
                return;
            }
        };

        let samples = chunk.voxels().map(BoolVoxel::from);
        let mut buffer = GreedyQuadsBuffer::new(samples.len());
        greedy_quads(
            &samples,
            &ChunkShape {},
            [0; 3],
            [17; 3],
            &RIGHT_HANDED_Y_UP_CONFIG.faces,
            &mut buffer,
        );
        let num_indices = buffer.quads.num_quads() * 6;
        let num_vertices = buffer.quads.num_quads() * 4;
        let mut indices = Vec::with_capacity(num_indices);
        let mut positions = Vec::with_capacity(num_vertices);
        let mut normals = Vec::with_capacity(num_vertices);

        let mut data = Vec::with_capacity(num_vertices);
        for (block_face_normal_index, (group, face)) in buffer
            .quads
            .groups
            .as_ref()
            .iter()
            .zip(RIGHT_HANDED_Y_UP_CONFIG.faces.iter())
            .enumerate()
        {
            for quad in group.iter() {
                indices.extend_from_slice(&face.quad_mesh_indices(positions.len() as u32));
                positions.extend_from_slice(&face.quad_mesh_positions(quad, 1.0));
                normals.extend_from_slice(&face.quad_mesh_normals());
                data.extend_from_slice(
                    &[(block_face_normal_index as u32) << 8u32
                        | chunk
                            .raw_voxel(&quad.minimum)
                            .as_mat_id() as u32; 4],
                );
            }
        }

        let mut render_mesh = Mesh::new(PrimitiveTopology::TriangleList);

        render_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        render_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        //render_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0; 2]; num_vertices]);
        render_mesh.insert_attribute(
            VoxelTerrainMesh::ATTRIBUTE_DATA,
            VertexAttributeValues::Uint32(data),
        );
        render_mesh.set_indices(Some(Indices::U32(indices.clone())));

        let mesh = meshes.add(render_mesh);

        //let mut material = StandardMaterial::from(Color::RED);
        //material.perceptual_roughness = 0.85;

        commands.spawn((MaterialMeshBundle {
            mesh,
            material: (**material).clone(),
            transform: Transform::from_xyz(
                (chunk_loc.x * 16) as f32 - 1.,
                (chunk_loc.y * 16) as f32 - 0.5,
                (chunk_loc.z * 16) as f32 - 1.,
            ),
            ..Default::default()
        }, WorldChunk { location: chunk_loc.clone() }));

        i += 1;
        if i == CHUNKS_PER_FRAME_CAP {
            log::warn!("CAP");
            return;
        }
    }
}

fn recive_all_new_world(
    mut global_world_gate: ResMut<GlobalWorldGate>,
    mut global_world: ResMut<GlobalWorld>,
    //mut material_singletone: ResMut<ChunkMaterialSingleton>
) {
    match global_world_gate.get_all_blocks_rx.try_next() {
        Ok(val) => {
            match val {
                Some(world) => {
                    match world {
                        Some(mut world) => {
                            let (_, world_data) = world.get_fields_mut();
                            let res = world_data.iter().map(|(loc, _)| loc).try_for_each(|loc| {
                                global_world_gate.chunk_load_tx.unbounded_send(loc.clone())
                            });

                            if let Err(err) = res {
                                log::error!("Cannot send turtle chunks loc into further processing. Err: {err}");
                                return;
                            }

                            global_world.world = Some(world);
                            //material_singletone.set_changed();
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
    world_blocks: Query<Entity, With<WorldChunk>>,
    global_world_gate: Res<GlobalWorldGate>,
) {
    for event in &mut select_turtle_reader {
        //Clean the world
        for entity in world_blocks.iter() {
            commands.entity(entity).despawn();
        }

        match &event.0 {
            Some(new_turtle) => {
                let uuid = new_turtle.uuid;
                let mut tx = global_world_gate.get_all_blocks_tx.clone();

                //-1 becouse idk yet
                //let (chunk_x, chunk_y, chunk_z) = (new_turtle.x >> 4, (new_turtle.y >> 4) - 1, new_turtle.z >> 4);

                spawn_async(async move {
                    let resp = send_get_world_request(&uuid).await;

                    match resp {
                        Ok(response) => {
                            let world = match TurtleWorld::from_bytes(response) {
                                Ok(val) => val,
                                Err(err) => {
                                    log::error!(
                                        "Cannot convert backend response into world. Error: {err}"
                                    );
                                    tx.try_send(None).expect("Cannot send world result");
                                    return;
                                }
                            };

                            tx.try_send(Some(world))
                                .expect("Cannot pass world into bevy system");
                        }
                        Err(err) => {
                            log::error!("Something went wrong when fetching world. Error: {err}");
                            tx.try_send(None).expect("Cannot send world result");
                        }
                    }
                })
            }
            None => {}
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn send_get_world_request(uuid: &Uuid) -> Result<Bytes, Box<dyn Error + Send + Sync>> {
    use crate::{HTTP_BACKEND_URL, REQWEST_CLIENT};

    let path = format!("{}/turtle/{uuid}/world/", HTTP_BACKEND_URL);
    let response = REQWEST_CLIENT.get(path).send().await?.bytes().await?;

    Ok(response)
}

#[cfg(target_arch = "wasm32")]
async fn send_get_world_request(uuid: &Uuid) -> Result<Bytes, Box<dyn Error>> {
    use gloo_net::http::Request;
    let response = Request::get(&format!("/turtle/{uuid}/world/"))
        .send()
        .await?
        .binary()
        .await?
        .into();

    Ok(response)
}

fn update_voxel_color(
    palette_enum: &WorldChangePaletteEnum,
    palette: &mut TurtleWorldPalette,
    world_data: &mut TurtleWorldData,
    chunk_loc: &ChunkLocation,
    change: &WorldChange
) {
    if let shared::WorldChangePaletteEnum::Insert { i, name } = palette_enum {
        palette.insert(*i, name.clone());
    }

    let id: u16 = match palette_enum {
        shared::WorldChangePaletteEnum::Insert { i, .. } => *i,
        shared::WorldChangePaletteEnum::GetOld { i } => *i,
    }.try_into().expect("Usize bigger then u16");

    let chunk = world_data.force_get_mut_chunk_by_loc(chunk_loc);
    chunk.update_voxel_by_global_xyz(change.x, change.y, change.z, |voxel| {
        voxel.id = id;
        Ok(())
    }).unwrap();
}

fn block_change_detect(
    mut world_change_events: EventReader<WorldChangeEvent>,
    mut global_world: ResMut<GlobalWorld>,
    global_world_gate: ResMut<GlobalWorldGate>,
) {
    if world_change_events.is_empty() {
        return
    };

    let mut chunks_to_rerender = Vec::<ChunkLocation>::with_capacity(world_change_events.len());
    let world = global_world.world.as_mut().expect("Cannot get mutable turtle world");
    let (palette, world_data) = world.get_fields_mut();

    for change in &mut world_change_events {
        let change = &change.0;
        let chunk_loc = ChunkLocation::from_global_xyz(change.x, change.y, change.z);

        match &change.action {
            shared::WorldChangeAction::New(new_block) => {
                update_voxel_color(&new_block.palette, palette, world_data, &chunk_loc, change);
            }
            shared::WorldChangeAction::Update(update) => {
                update_voxel_color(&update.palette, palette, world_data, &chunk_loc, change);
            }
            shared::WorldChangeAction::Delete(_) => {
                let chunk = world_data.force_get_mut_chunk_by_loc(&chunk_loc);
                chunk.update_voxel_by_global_xyz(change.x, change.y, change.z, |voxel| {
                    voxel.id = 0;
                    Ok(())
                }).unwrap();
            }
        };

        chunks_to_rerender.push(chunk_loc);
    }

    log::warn!("TO REM: {:?}", chunks_to_rerender);

    chunks_to_rerender.sort();
    chunks_to_rerender.dedup();

    log::warn!("AFT TO REM: {:?}", chunks_to_rerender);

    for location in chunks_to_rerender {
        global_world_gate.chunk_load_tx.unbounded_send(location).expect("Cannot send chunk to rerender");
    }
}
