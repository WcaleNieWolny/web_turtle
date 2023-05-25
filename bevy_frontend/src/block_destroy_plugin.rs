use bevy::prelude::*;
use bevy_mod_raycast::{
    DefaultRaycastingPlugin, RaycastMethod, RaycastSource,
    RaycastSystem,
};

use crate::BlockRaycastSet;


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
    mut query_ray: Query<&mut RaycastSource<BlockRaycastSet>>,
) {
    if keyboard.just_pressed(MouseButton::Left) {
        log::warn!("Click!");
        for entity in &query_ray {
            if let Some((entity, _)) = entity.get_nearest_intersection() {
                log::warn!("ID: {}", entity.index());
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
