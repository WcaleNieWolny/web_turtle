#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_bindings
#import bevy_pbr::mesh_functions

#import "shaders/voxel_data.wgsl"

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) voxel_data: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) voxel_normal: vec3<f32>,
    @location(1) voxel_data: u32,
    @location(2) world_position: vec3<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let world_position = mesh_position_local_to_world(mesh.model, vec4<f32>(vertex.position, 1.0));

    var out: VertexOutput;
    out.clip_position = mesh_position_world_to_clip(world_position);
    out.voxel_normal = voxel_data_extract_normal(vertex.voxel_data);
    out.voxel_data = vertex.voxel_data;
    out.world_position = world_position.xyz;

    return out;
}
