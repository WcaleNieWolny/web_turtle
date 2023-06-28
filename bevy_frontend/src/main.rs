mod move_plugin;

mod block_destroy_plugin;
mod egui_ui_plugin;
mod world_plugin;
mod chunk_material;

#[cfg(target_arch = "wasm32")]
mod resize_plugin;

//mod inventory_plugin;

#[cfg(target_arch = "wasm32")]
extern crate console_error_panic_hook;

use std::panic;
use std::sync::RwLock;
use std::{f32::consts::TAU, sync::Arc};

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_mod_raycast::RaycastSource;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use chunk_material::ChunkMaterialPlugin;
use egui_ui_plugin::UiPlugin;
//use block_destroy_plugin::BlockDestroyPlugin;
use futures::Future;
use move_plugin::MovePlugin;
use shared::{JsonTurtle, WorldChange};
#[cfg(not(target_arch = "wasm32"))]
use tokio::runtime::{Builder, Runtime};
use world_plugin::WorldPlugin;

#[cfg(not(target_arch = "wasm32"))]
static HTTP_BACKEND_URL: &str = "http://0.0.0.0:8000";
#[cfg(target_arch = "wasm32")]
static TURTLE_ASSET_LOCATION: &str = "/assets/turtle_model.glb#Scene0";
#[cfg(not(target_arch = "wasm32"))]
static TURTLE_ASSET_LOCATION: &str = "turtle_model.glb#Scene0";

#[derive(Reflect, Clone, Component)]
pub struct BlockRaycastSet;

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct MainTurtleObject;

#[derive(Debug)]
pub struct SelectTurtleEvent(Option<JsonTurtle>);

pub struct WorldChangeEvent(WorldChange);

#[cfg(target_arch = "wasm32")]
pub fn spawn_async<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    use wasm_bindgen_futures::spawn_local;
    spawn_local(future)
}

#[cfg(not(target_arch = "wasm32"))]
static TOKIO_RUNTIME: once_cell::sync::Lazy<Runtime> = once_cell::sync::Lazy::new(|| {
    Builder::new_multi_thread()
        .worker_threads(1)
        .enable_io()
        .enable_time()
        .build()
        .expect("Cannot build tokio runtime")
});

#[cfg(not(target_arch = "wasm32"))]
static REQWEST_CLIENT: once_cell::sync::Lazy<reqwest::Client> = once_cell::sync::Lazy::new(|| {
    reqwest::Client::new()
});

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_async<F>(future: F)
where
    F: Future<Output = ()> + 'static + Send,
{
    TOKIO_RUNTIME.spawn(future);
}

fn main() {
    // When building for WASM, print panics to the browser console

    #[cfg(target_arch = "wasm32")]
    {
        use log::Level;
        panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(Level::Warn).unwrap();
        log::warn!("WASM FEAT");
    }
    #[cfg(target_arch = "wasm32")]
    {
        spawn_async(async { async_main().await });
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        TOKIO_RUNTIME.block_on(async { async_main().await });
    }
}

struct PlatformIndependentPlugins;

#[derive(Resource, Deref)]
pub struct MainTurtle(Arc<RwLock<Option<JsonTurtle>>>);

impl Plugin for PlatformIndependentPlugins {
    fn build(&self, app: &mut App) {
        #[cfg(target_arch = "wasm32")]
        {
            use resize_plugin::ResizePlugin;

            app.add_plugin(ResizePlugin);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
        }
    }
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
        .add_plugin(PanOrbitCameraPlugin)
        .add_plugin(MovePlugin)
        .add_plugin(WorldPlugin)
        .add_plugin(PlatformIndependentPlugins)
        .add_plugin(UiPlugin)
        //.add_plugin(BlockDestroyPlugin)
        //.add_plugin(InventoryPlugin)
        .add_plugin(ChunkMaterialPlugin)
        .add_plugin(EguiPlugin)
        .add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugin(bevy::diagnostic::LogDiagnosticsPlugin::default())
        .add_startup_system(setup)
        .run();
}

//https://bevyengine.org/examples/3d/3d-scene/
/// set up a simple 3D scene
fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    let gltf: Handle<Scene> = assets.load(TURTLE_ASSET_LOCATION);
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
    commands
        .spawn((
            Camera3dBundle::default(),
            PanOrbitCamera {
                focus: Vec3::new(0.5, 0.5, 0.5),
                radius: 8.0,
                pan_sensitivity: 1.0,
                beta: TAU / 18.0,
                ..default()
            },
            MainCamera,
        ))
        .insert(RaycastSource::<BlockRaycastSet>::new());

    // light
    commands.insert_resource(AmbientLight {
        color: Color::rgb(1.0, 1.0, 1.0),
        brightness: 0.8,
    });
    // background
    commands.insert_resource(ClearColor(Color::hex("5b7cb6").unwrap()));
}
