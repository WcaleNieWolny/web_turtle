use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render::{
        extract_component::ExtractComponent,
        mesh::MeshVertexAttribute,
        render_resource::{AsBindGroup, ShaderType, VertexFormat},
    },
};
use shared::world_structure::TurtleWorld;

use crate::world_plugin::GlobalWorld;

#[derive(Component, Clone, Default, ExtractComponent)]
/// A marker component for voxel meshes.
pub struct VoxelTerrainMesh;

impl VoxelTerrainMesh {
    pub const ATTRIBUTE_DATA: MeshVertexAttribute =
        MeshVertexAttribute::new("Vertex_Data", 0x696969, VertexFormat::Uint32);
}

#[derive(Resource, Deref, Default)]
struct GpuMaterialGate {
    last_world_pallete_size: Option<usize>,
}

#[derive(ShaderType, Clone, Copy, Default)]
pub struct GpuVoxelMaterial {
    base_color: Color,
    flags: u32,
    emissive: Color,
    perceptual_roughness: f32,
    metallic: f32,
    reflectance: f32,
}

#[derive(AsBindGroup, ShaderType, Clone, TypeUuid)]
#[uuid = "1e31e29e-73d8-419c-8293-876ae81d2636"]
pub struct GpuTerrainUniforms {
    #[uniform(0)]
    pub render_distance: u32,
    #[uniform(1)]
    pub materials: [GpuVoxelMaterial; 256],
}

impl Default for GpuTerrainUniforms {
    fn default() -> Self {
        Self {
            render_distance: 16,
            materials: [default(); 256],
        }
    }
}

impl Material for GpuTerrainUniforms {
    fn vertex_shader() -> bevy::render::render_resource::ShaderRef {
        "shaders/terrain_pipeline.wgsl".into()
    }

    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        "shaders/terrain_pipeline_frag.wgsl".into()
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        layout: &bevy::render::mesh::MeshVertexBufferLayout,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        let vertex_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            VoxelTerrainMesh::ATTRIBUTE_DATA.at_shader_location(1),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}

fn update_chunk_material_singleton(
    mut commands: Commands,
    mut materials: ResMut<Assets<GpuTerrainUniforms>>,
    mut chunk_entities: Query<(Entity, &mut Handle<GpuTerrainUniforms>)>,
    world: Res<GlobalWorld>,
    mut gate: ResMut<GpuMaterialGate>,
) {
    let world_option: &Option<TurtleWorld> = &*world;

    if world_option.as_ref().is_some_and(|world| {
            if let Some(old_world) = gate.last_world_pallete_size {
                return old_world != world.pallete.len();
            };
            return true;
        })
    {
        println!("CHANGE RERENDER");
        let mut gpu_mats = GpuTerrainUniforms {
            materials: [GpuVoxelMaterial {
                base_color: Color::RED,
                emissive: Color::BLACK,
                perceptual_roughness: 0.85,
                metallic: 0.0,
                reflectance: 0.5,
                ..Default::default()
            }; 256],
            render_distance: 32,
        };

        if let Some(world) = world_option {
            let TurtleWorld { pallete, .. } = world;

            pallete
                .iter()
                .enumerate()
                .skip(1) //Skip air block
                .take_while(|(i, _)| *i < 256)
                .map(|(i, item_name)| {
                    let hash = seahash::hash(item_name.as_bytes());
                    let hash: [u8; 8] = hash.to_le_bytes();

                    (i, Color::rgb_u8(hash[0], hash[4], hash[7]))
                })
                .for_each(|(i, color)| {
                    gpu_mats.materials[i] = GpuVoxelMaterial {
                        base_color: color,
                        emissive: Color::BLACK,
                        perceptual_roughness: 0.85,
                        metallic: 0.0,
                        reflectance: 0.5,
                        ..Default::default()
                    };
                });
        }

        let chunk_material = materials.add(gpu_mats);
        commands.insert_resource(ChunkMaterialSingleton(chunk_material.clone()));

        for (_, mut mat) in &mut chunk_entities {
            *mat = chunk_material.clone();
        }
    }

    gate.last_world_pallete_size = world_option.as_ref().map(|world| world.pallete.len());
}

#[derive(Resource, Deref, DerefMut)]
pub struct ChunkMaterialSingleton(Handle<GpuTerrainUniforms>);

impl FromWorld for ChunkMaterialSingleton {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.resource_mut::<Assets<GpuTerrainUniforms>>();
        Self(materials.add(GpuTerrainUniforms::default()))
    }
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, SystemSet)]
/// Systems that prepare the global [ChunkMaterialSingleton] value.
pub struct ChunkMaterialSet;

pub struct ChunkMaterialPlugin;

impl Plugin for ChunkMaterialPlugin {
    fn build(&self, app: &mut App) {
        // @todo: figure out race conditions w/ other systems
        app.add_plugin(MaterialPlugin::<GpuTerrainUniforms>::default())
            .init_resource::<ChunkMaterialSingleton>()
            .insert_resource(GpuMaterialGate::default())
            .add_system(
                update_chunk_material_singleton
                    .in_set(ChunkMaterialSet)
                    .in_base_set(CoreSet::Update),
            );
    }
}
