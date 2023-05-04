mod resize_plugin;
mod ui_plugin;


extern crate console_error_panic_hook;

use std::f32::consts::TAU;
use std::panic;

use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use resize_plugin::ResizePlugin;
use ui_plugin::UiPlugin;
use wasm_bindgen_futures::spawn_local;

fn main() {
    // When building for WASM, print panics to the browser console
    use log::Level;
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(Level::Warn).unwrap();
    spawn_local(async {
        async_main().await
    });
}

async fn async_main() {
    App::new()
        .add_plugins(
            DefaultPlugins.build()
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        fit_canvas_to_parent: true,
                        canvas: Some(".game_canvas".to_string()), 
                        ..default()
                    }),
                    ..default()
                })
        )
        .insert_resource(Msaa::Sample4)
        .add_plugin(ResizePlugin)
        .add_plugin(UiPlugin)
        .add_plugin(PanOrbitCameraPlugin)
        .add_startup_system(setup)
        .run();
}

//https://bevyengine.org/examples/3d/3d-scene/
/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(5.0).into()),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    // camera
    commands.spawn((
        Camera3dBundle::default(),
        PanOrbitCamera {
            focus: Vec3::new(0.0, 0.5, 0.0),
            radius: 8.0,
            pan_sensitivity: 0.0,
            beta: TAU / 18.0,
            ..default()
        },
    ));

    // light
    commands.insert_resource(AmbientLight {
        color: Color::rgb(1.0, 1.0, 1.0),
        brightness: 0.6
    });
    // background
    commands.insert_resource(ClearColor(Color::hex("5b7cb6").unwrap()));
}


