use std::sync::{Arc, RwLock};

use bevy::prelude::*;

use crate::MainTurtle;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MainTurtle(Arc::new(RwLock::new(None))));
    }
}
