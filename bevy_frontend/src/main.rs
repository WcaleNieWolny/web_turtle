mod resize_system;

use std::f32::consts::TAU;
extern crate console_error_panic_hook;
use std::panic;

use log::warn;
use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use js_sys::JsString;
use resize_system::ResizePlugin;
use shared::JsonTurtle;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{window, RequestInit, Request, Response};

async fn get_turtles_list() -> Vec<JsonTurtle> {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    
    let mut get_turtles_url = document.base_uri().expect("Base uri get fail").expect("No base uri");
    get_turtles_url.push_str("turtle/list/");

    warn!("{get_turtles_url}");

    let mut opts = RequestInit::new();
    opts.method("GET");
    let request = Request::new_with_str_and_init(&get_turtles_url, &opts).expect("Cannot create new request");
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await.expect("Cannot fetch value");

    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into().expect("Cannot cast into response");

    let json = JsFuture::from(resp.json().unwrap()).await.expect("Cannot get future from JS");
    return serde_wasm_bindgen::from_value(json).expect("Json serde error");
}

async fn setup_ui() {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let navbar_div = document.query_selector(".navbar_div").ok().expect("No navbar found").expect("No navbar found");

    for turtle in get_turtles_list().await {
        let turtle_navbar_div = document.create_element("div").expect("Cannot create div"); 
        turtle_navbar_div.set_class_name("navbar_item_div");
        navbar_div.append_child(&turtle_navbar_div).unwrap();
    }
}

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
    setup_ui().await;

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
    //Export to JS
    let window: JsValue = window().unwrap().into(); 
    let shiet_str: JsValue = JsString::from("aa").into();
    

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


