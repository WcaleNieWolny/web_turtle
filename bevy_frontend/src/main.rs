mod move_plugin;
mod resize_plugin;
mod ui_plugin;
mod world_plugin;

extern crate console_error_panic_hook;

use std::f32::consts::TAU;
use std::panic;

use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_mod_raycast::{
    DefaultPluginState, DefaultRaycastingPlugin, RaycastMesh, RaycastMethod, RaycastSource,
    RaycastSystem,
};
use move_plugin::MovePlugin;
use resize_plugin::ResizePlugin;
use shared::{JsonTurtle, WorldChange};
use ui_plugin::UiPlugin;
use wasm_bindgen_futures::spawn_local;
use world_plugin::WorldPlugin;

#[derive(Reflect, Clone, Component)]
pub struct MyRaycastSet;

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct MainTurtleObject;

#[derive(Debug)]
pub struct SelectTurtleEvent(Option<JsonTurtle>);

pub struct WorldChangeEvent(WorldChange);

fn main() {
    // When building for WASM, print panics to the browser console
    use log::Level;
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(Level::Warn).unwrap();
    spawn_local(async { async_main().await });
}

async fn async_main() {
    App::new()
        .add_plugins(DefaultPlugins.build().set(WindowPlugin {
            primary_window: Some(Window {
                fit_canvas_to_parent: true,
                canvas: Some(".game_canvas".to_string()),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Msaa::Sample4)
        .add_event::<SelectTurtleEvent>()
        .add_event::<WorldChangeEvent>()
        .add_plugin(ResizePlugin)
        .add_plugin(UiPlugin)
        .add_plugin(PanOrbitCameraPlugin)
        .add_plugin(MovePlugin)
        .add_plugin(WorldPlugin)
        .add_plugin(DefaultRaycastingPlugin::<MyRaycastSet>::default())
        .add_startup_system(setup)
        .add_system(
            update_raycast_with_cursor.in_base_set(CoreSet::First).before(RaycastSystem::BuildRays::<MyRaycastSet>)
        )
        .add_system(shit.after(update_raycast_with_cursor))
        .run();
}

fn update_raycast_with_cursor(
    mut cursor: EventReader<CursorMoved>,
    mut query: Query<&mut RaycastSource<MyRaycastSet>>,
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

fn shit(
    keyboard: Res<Input<KeyCode>>,
    mut query: Query<&mut RaycastSource<MyRaycastSet>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        log::warn!("Click!");
        for entity in &query {
            if let Some((entity, _)) = entity.get_nearest_intersection() {
                log::warn!("ID: {}", entity.index());
            }
        }   
    }
}

//https://bevyengine.org/examples/3d/3d-scene/
/// set up a simple 3D scene
fn setup(mut commands: Commands, assets: Res<AssetServer>) {

    commands.insert_resource(DefaultPluginState::<MyRaycastSet>::default().with_debug_cursor());
    let gltf: Handle<Scene> = assets.load("/assets/turtle_model.glb#Scene0");
    commands.spawn((
        SceneBundle {
            scene: gltf,
            transform: Transform::from_xyz(0.0, 0.5, 0.0).with_rotation(Quat::from_euler(
                EulerRot::XYZ,
                0.0,
                (0.0_f32).to_radians(),
                //0.0,
                0.0,
            )),
            ..default()
        },
        MainTurtleObject,
        AnimationPlayer::default(),
        Name::new("turtle_model"),
    ));

    // camera
    commands.spawn((
        Camera3dBundle::default(),
        PanOrbitCamera {
            focus: Vec3::new(0.5, 0.5, 0.5),
            radius: 8.0,
            pan_sensitivity: 0.0,
            beta: TAU / 18.0,
            ..default()
        },
        MainCamera,
    ))
    .insert(RaycastSource::<MyRaycastSet>::new());

    // light
    commands.insert_resource(AmbientLight {
        color: Color::rgb(1.0, 1.0, 1.0),
        brightness: 0.8,
    });
    // background
    commands.insert_resource(ClearColor(Color::hex("5b7cb6").unwrap()));
}
