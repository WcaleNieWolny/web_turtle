#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::pbr_bindings
#import bevy_pbr::mesh_bindings
#import bevy_pbr::mesh_functions

#import bevy_pbr::utils
#import bevy_pbr::clustered_forward
#import bevy_pbr::lighting
#import bevy_pbr::pbr_ambient
#import bevy_pbr::shadows
#import bevy_pbr::fog
#import bevy_pbr::pbr_functions

#import "shaders/voxel_data.wgsl"
#import "shaders/terrain_uniforms.wgsl"

struct Fragment {
    @builtin(position) frag_coord: vec4<f32>,
    @builtin(front_facing) front_facing: bool,
    /// The normalized normal of the voxel.
    @location(0) voxel_normal: vec3<f32>,
    /// The voxel data.
    @location(1) voxel_data: u32,
    /// The world position of the voxel vertex.
    @location(2) world_position: vec3<f32>,
};

fn prepare_pbr_input_from_voxel_mat(voxel_mat: VoxelMat, frag: Fragment) -> PbrInput {
    var base_color: vec4<f32> = voxel_mat.base_color;
    //base_color = base_color + hash(vec4<f32>(floor(frag.world_position - frag.voxel_normal * 0.5), 1.0)) * 0.0226;

    var pbr_input: PbrInput = pbr_input_new();
    pbr_input.material.metallic = voxel_mat.metallic;
    pbr_input.material.perceptual_roughness = voxel_mat.perceptual_roughness;
    pbr_input.material.emissive = voxel_mat.emissive;
    pbr_input.material.reflectance = voxel_mat.reflectance;
    pbr_input.material.base_color = base_color;

    pbr_input.frag_coord = frag.frag_coord;
    pbr_input.world_position = vec4<f32>(frag.world_position, 1.0);
    pbr_input.world_normal = (f32(frag.front_facing) * 2.0 - 1.0) * mesh_normal_local_to_world(frag.voxel_normal);
    
    pbr_input.is_orthographic = view.projection[3].w == 1.0;
    pbr_input.N = normalize(mesh_normal_local_to_world(frag.voxel_normal));
    pbr_input.V = calculate_view(vec4<f32>(frag.world_position, 1.0), pbr_input.is_orthographic);
    pbr_input.flags = mesh.flags;
    return pbr_input;
}

@fragment
fn fragment(frag: Fragment) -> @location(0) vec4<f32> {
    let material = voxel_materials[voxel_data_extract_material_index(frag.voxel_data)];

    /// PBR lighting input data preparation
    var pbr_input = prepare_pbr_input_from_voxel_mat(material, frag);
    let pbr_colour = tone_mapping(pbr(pbr_input));

	return pbr_colour;
}
