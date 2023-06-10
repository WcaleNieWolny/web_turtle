use bevy::render::mesh::{MeshVertexAttribute, Indices};
use bevy::render::render_resource::{PrimitiveTopology, VertexFormat};
use bevy::{prelude::*, pbr::wireframe::Wireframe};
use futures::channel::mpsc::{channel, Receiver, Sender};
use shared::{JsonTurtle, TurtleWorld};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Request, RequestInit, Response};
use block_mesh::ndshape::{ConstShape, ConstShape3u32};
use block_mesh::{greedy_quads, GreedyQuadsBuffer, MergeVoxel, Voxel, VoxelVisibility, RIGHT_HANDED_Y_UP_CONFIG};

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

#[derive(Clone, Copy, Eq, PartialEq)]
struct BoolVoxel(bool);

const EMPTY: BoolVoxel = BoolVoxel(false);
const FULL: BoolVoxel = BoolVoxel(true);

impl Voxel for BoolVoxel {
    fn get_visibility(&self) -> VoxelVisibility {
        if *self == EMPTY {
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
                            //chunk_x, chunk_z >> 4 = down left corner 
                            let mut voxels = [EMPTY; ChunkShape::SIZE as usize];

                            //down left corner
                            let (chunk_top_x, chunk_top_z) = (world.chunk_x << 4, world.chunk_z << 4);
                            log::warn!("corner {chunk_top_x} {chunk_top_z}");

                            for block in &world.blocks {
                                let (x, y, z) = ((block.x - chunk_top_x).abs() as u32, (block.y - (world.chunk_y << 4)) as u32, (block.z - chunk_top_z).abs() as u32);

                                log::warn!("{x} {y} {z}");
                                //this is the right down voxel

                                voxels[ChunkShape::linearize([x + 1, y + 1, z + 1]) as usize] = FULL;
                            }

                            let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;

                            log::warn!("Faces: {faces:?}");

                            let samples = voxels;
                            let mut buffer = GreedyQuadsBuffer::new(samples.len());
                            greedy_quads(
                                &samples,
                                &ChunkShape {},
                                [0; 3],
                                [17; 3],
                                &faces,
                                &mut buffer,
                            );
                            let num_indices = buffer.quads.num_quads() * 6;
                            let num_vertices = buffer.quads.num_quads() * 4;
                            let mut indices = Vec::with_capacity(num_indices);
                            let mut positions = Vec::with_capacity(num_vertices);
                            let mut normals = Vec::with_capacity(num_vertices);
                            for (group, face) in buffer.quads.groups.into_iter().zip(faces.into_iter()) {
                                for quad in group.into_iter() {
                                    indices.extend_from_slice(&face.quad_mesh_indices(positions.len() as u32));
                                    positions.extend_from_slice(&face.quad_mesh_positions(&quad, 1.0));
                                    normals.extend_from_slice(&face.quad_mesh_normals());
                                }
                            }

                            let mut render_mesh = Mesh::new(PrimitiveTopology::TriangleList);

                            render_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
                            render_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                            render_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0; 2]; num_vertices]);
                            render_mesh.set_indices(Some(Indices::U32(indices.clone())));

                            let mesh = meshes.add(render_mesh);

                        let mut material = StandardMaterial::from(Color::rgb(0.0, 0.0, 0.0));
                        material.perceptual_roughness = 0.9;

                        commands.spawn(PbrBundle {
                            mesh,
                            material: materials.add(material),
                            transform: Transform::from_xyz(chunk_top_x as f32, (world.chunk_y * 16) as f32, (chunk_top_z) as f32),
                            ..Default::default()
                        });
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

                //-1 becouse idk yet
                let (chunk_x, chunk_y, chunk_z) = (new_turtle.x >> 4, (new_turtle.y >> 4) - 1, new_turtle.z >> 4);

                spawn_local(async move {
                    let window = web_sys::window().expect("no global `window` exists");

                    let url = format!("/turtle/{uuid}/chunk/?x={chunk_x}&y={chunk_y}&z={chunk_z}");

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
