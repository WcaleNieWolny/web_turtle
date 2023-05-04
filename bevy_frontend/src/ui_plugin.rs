use std::sync::{Arc, RwLock};

use shared::JsonTurtle;
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use bevy::{prelude::*, utils::HashMap};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{RequestInit, Request, Response, PointerEvent, HtmlElement};

static mut MAIN_TURTLE: Option<Arc<RwLock<Option<JsonTurtle>>>> = None;
static mut TURTLE_VEC: Option<Arc<RwLock<Vec<JsonTurtle>>>> = None;

#[derive(Component)]
struct MainTurtle(Arc<RwLock<Option<JsonTurtle>>>);

#[derive(Resource)]
struct BombsSpawnConfig {
    /// How often to spawn a new bomb? (repeating timer)
    timer: Timer,
}

pub struct UiPlugin;

fn setup_ui_system(mut commands: Commands) {
    commands.insert_resource(BombsSpawnConfig {
        // create the repeating timer
        timer: Timer::new(std::time::Duration::from_secs(3), TimerMode::Repeating),
    });
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let navbar_div = document.query_selector(".navbar_div").ok().expect("No navbar found").expect("No navbar found");

    let main_turtle: Arc<RwLock<Option<JsonTurtle>>> = Arc::new(RwLock::new(None));
    let turtle_vec: Arc<RwLock<Vec<JsonTurtle>>> = Arc::new(RwLock::new(Vec::new()));
    //Again, do not care about unsafe :)
    unsafe {
        MAIN_TURTLE = Some(main_turtle.clone());
        TURTLE_VEC = Some(turtle_vec)
    }

    let on_click_closure = Closure::wrap(Box::new(|e: PointerEvent| {
        let target = e.target().expect("No event target!");
        let target = target.dyn_ref::<HtmlElement>().expect("Clicked element not a HTML element");
        let id = target.get_attribute("data-id").expect("No ID atribute, THE USER IS A HACKER, NOT COOL BRO!!!!!!!!!");
        let id = id.parse::<usize>().expect("Invalid UUID, THE USER HAS TAMPERED WITH THE UUID!!!!!");

        let global_turtle_vec = unsafe {
            TURTLE_VEC.as_mut().unwrap_unchecked()
        };

        let main_turtle = unsafe{ 
            MAIN_TURTLE.as_mut().unwrap_unchecked() 
        };

        let global_turtles_guard = global_turtle_vec.read().expect("Cannot lock global turtles!");
        let mut main_turtle_guard = main_turtle.write().expect("Cannot lock main turtle");

        if id >= global_turtles_guard.len() {
            log::error!("DO NOT FUCKING TRY TO HACK MY APP!!!!!!!!!!!!!");
            return;
        };

        let new_global_turtle = global_turtles_guard[id].clone();
        *main_turtle_guard = Some(new_global_turtle);

        //Here we will write into MAIN_TURTLE static (This is something to be implemented) 
    }) as Box<dyn FnMut(_)>);

    //Register main turtle with bevy!
    commands.spawn(MainTurtle(main_turtle));

    //This spawn thing is expensive, but whatevet
    spawn_local(async move {
        let turtle_list = get_turtles_list().await;
        let global_turtle_vec = unsafe {
            TURTLE_VEC.as_mut().unwrap_unchecked()
        };

        let main_turtle = unsafe{ 
            MAIN_TURTLE.as_mut().unwrap_unchecked() 
        };

        let mut global_turtles_guard = global_turtle_vec.write().expect("Cannot lock global turtles!");
        let mut main_turtle_guard = main_turtle.write().expect("Cannot lock main turtle");

        if let Some(global_turtle) = main_turtle_guard.as_mut() {
            if !turtle_list.contains(global_turtle) {
                *main_turtle_guard = None;
            }
        }

        for (i, turtle) in turtle_list.iter().enumerate() {
            if let Some(global_turtle) = global_turtles_guard.get(i) {
                if global_turtle == turtle {
                    continue;
                }
            }

            let turtle_navbar_div = document.create_element("div").expect("Cannot create div"); 
            turtle_navbar_div.set_class_name("navbar_item_div");
            turtle_navbar_div.add_event_listener_with_callback("pointerdown", &on_click_closure.as_ref().unchecked_ref()).expect("Cannot set event listener");
            turtle_navbar_div.set_attribute("data-id", &i.to_string()).expect("Cannot set uuid atribute");
            navbar_div.append_child(&turtle_navbar_div).unwrap();
        }

        *global_turtles_guard = turtle_list.clone();

        //Required or BAD things will happen!
        on_click_closure.forget();
    });
}

async fn get_turtles_list() -> Vec<JsonTurtle> {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    
    let mut get_turtles_url = document.base_uri().expect("Base uri get fail").expect("No base uri");
    get_turtles_url.push_str("turtle/list/");

    let mut opts = RequestInit::new();
    opts.method("GET");
    let request = Request::new_with_str_and_init(&get_turtles_url, &opts).expect("Cannot create new request");
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await.expect("Cannot fetch value");

    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into().expect("Cannot cast into response");

    let json = JsFuture::from(resp.json().unwrap()).await.expect("Cannot get future from JS");
    return serde_wasm_bindgen::from_value(json).expect("Json serde error");
}

fn check_turtle(
    time: Res<Time>,
    mut config: ResMut<BombsSpawnConfig>,
    main_turtle: Query<&MainTurtle>
) {
    config.timer.tick(time.delta());

    if config.timer.finished() {
        let main_turtle = main_turtle.single();
        let guard = main_turtle.0.read().expect("Cannot read from main_turtle!");
        log::warn!("Turtle is some: {}", guard.is_some());
    }
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_ui_system)
            .add_system(check_turtle);
    }
}
