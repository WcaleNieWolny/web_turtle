use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

fn main() {
    // When building for WASM, print panics to the browser console
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
    }
    
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

    println!("TESAT!");
}
