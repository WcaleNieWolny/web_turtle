use bevy::prelude::*;
use bevy_mod_raycast::{
    DefaultRaycastingPlugin, RaycastMethod, RaycastSource,
    RaycastSystem,
};
use shared::JsonTurtleRotation;

use crate::{BlockRaycastSet, ui_plugin::MainTurtle};


pub struct BlockDestroyPlugin;

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
) {
    if keyboard.just_pressed(MouseButton::Middle) {
        log::warn!("Click!");
        for entity in &query_ray {
            if let Some((entity, _)) = entity.get_nearest_intersection() {
                let transform = transform_query.get_component::<Transform>(entity).expect("Entity does not contain transform!");
                let transform = transform.translation;
                let (x, y, z) = (transform.x - 0.5, transform.y - 1.0, transform.z - 0.5);
                let guard = main_turtle
                    .read()
                    .expect("Cannot lock main turtle, should never happen!");
                let main_turtle_ref = match &*guard {
                    Some(val) => val,
                    None => return,
                }; 

                let (forward_x, forward_y, forward_z) = JsonTurtleRotation::Forward.to_turtle_move_diff(&main_turtle_ref.rotation);
                let (forward_x, forward_y, forward_z) = (forward_x as f32 + main_turtle_ref.x as f32, main_turtle_ref.y as f32 + forward_y as f32, forward_z as f32 + main_turtle_ref.z as f32);
                let (back_x, back_y, back_z) = JsonTurtleRotation::Backward.to_turtle_move_diff(&main_turtle_ref.rotation);
                let (back_x, back_y, back_z) = (back_x as f32 + main_turtle_ref.x as f32, main_turtle_ref.y as f32 + back_y as f32, back_z as f32 + main_turtle_ref.z as f32);

                if forward_x == x && forward_z == z && forward_y == y {
                    log::warn!("SHIT");
                } else if back_z == z && back_y == y && back_x == x {
                    log::warn!("Bsc");
                } else if x == main_turtle_ref.x as f32 &&  y == main_turtle_ref.y as f32 + 1.0 && z == main_turtle_ref.z as f32 {
                    log::warn!("UI");
                }
            }
        }   
    }
}

impl Plugin for BlockDestroyPlugin {
    fn build(&self, app: &mut App) { 
        app.add_plugin(DefaultRaycastingPlugin::<BlockRaycastSet>::default())
            .add_system(
                update_raycast_with_cursor.in_base_set(CoreSet::First).before(RaycastSystem::BuildRays::<BlockRaycastSet>)
            )
            .add_system(detect_block_destroy_from_mouse.after(update_raycast_with_cursor));
    }
}
