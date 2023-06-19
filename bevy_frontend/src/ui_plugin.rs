use std::{
    ops::Deref,
    sync::{Arc, RwLock},
};

use uuid::Uuid;
use bevy::{prelude::*, utils::HashMap};
use shared::JsonTurtle;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{HtmlElement, MouseEvent, PointerEvent, Request, RequestInit, Response};

use crate::SelectTurtleEvent;

static mut MAIN_TURTLE: Option<Arc<RwLock<Option<JsonTurtle>>>> = None;
static mut TURTLE_VEC: Option<Arc<RwLock<HashMap<Uuid, JsonTurtle>>>> = None;
static mut ON_CLICK_CLOSURE: Option<JsValue> = None;

#[derive(Resource)]
pub struct MainTurtle(Arc<RwLock<Option<JsonTurtle>>>);

pub struct UiPlugin;

#[derive(Resource)]
struct TurtleEventChangeState {
    previous_turtle: Option<JsonTurtle>,
}

impl Deref for MainTurtle {
    type Target = Arc<RwLock<Option<JsonTurtle>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn setup_ui_system(mut commands: Commands) {
    let main_turtle: Arc<RwLock<Option<JsonTurtle>>> = Arc::new(RwLock::new(None));
    let turtle_vec: Arc<RwLock<HashMap<Uuid, JsonTurtle>>> = Arc::new(RwLock::new(HashMap::new()));
    //Again, do not care about unsafe :)
    unsafe {
        MAIN_TURTLE = Some(main_turtle.clone());
        TURTLE_VEC = Some(turtle_vec.clone())
    }

    let on_click_closure = Closure::wrap(Box::new(|e: PointerEvent| {
        let target = e.target().expect("No event target!");
        let target = target
            .dyn_ref::<HtmlElement>()
            .expect("Clicked element not a HTML element");
        let parrent_element = target.parent_element().expect("No parrent element found!");
        let id = target
            .get_attribute("data-id")
            .expect("No ID atribute, THE USER IS A HACKER, NOT COOL BRO!!!!!!!!!");
        let id = Uuid::try_parse(&id)
            .expect("Invalid UUID, THE USER HAS TAMPERED WITH THE UUID!!!!!");

        let global_turtle_vec = unsafe { TURTLE_VEC.as_mut().unwrap_unchecked() };

        let main_turtle = unsafe { MAIN_TURTLE.as_mut().unwrap_unchecked() };

        let global_turtles_guard = global_turtle_vec
            .read()
            .expect("Cannot lock global turtles!");
        let mut main_turtle_guard = main_turtle.write().expect("Cannot lock main turtle");

        if let Some(other_turtle) = main_turtle_guard.as_ref() {
            let window = web_sys::window().expect("no global `window` exists");
            let document = window.document().expect("should have a document on window");
            let other_turtle_element = document
                .query_selector(&format!("[data-id=\"{}\"]", other_turtle.uuid))
                .expect("Query select error")
                .expect("The previous main turtle element was removed");
            let other_turtle_element_parrent = other_turtle_element
                .parent_element()
                .expect("No parrent element found on other_turtle_element");
            other_turtle_element_parrent.set_class_name("navbar_item_div"); //Remove border
        }

        parrent_element.set_class_name("navbar_item_div green-border");

        let new_global_turtle = global_turtles_guard[&id].clone();
        *main_turtle_guard = Some(new_global_turtle);

        //Here we will write into MAIN_TURTLE static (This is something to be implemented)
    }) as Box<dyn FnMut(_)>);

    unsafe {
        ON_CLICK_CLOSURE = Some(on_click_closure.into_js_value());
    }

    //Setup refresh_button
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let refresh_button = document
        .query_selector(".refresh_button")
        .ok()
        .expect("No refresh button found")
        .expect("No refresh_button found");
    let refresh_button_closure = Closure::wrap(Box::new(|_: MouseEvent| {
        let main_turtle = unsafe { MAIN_TURTLE.as_ref().unwrap_unchecked() };
        let turtle_vec = unsafe { TURTLE_VEC.as_ref().unwrap_unchecked() };

        spawn_local(async move {
            //This will refresh the turtle list
            update_turtle_list(main_turtle, turtle_vec).await;
        });
    }) as Box<dyn FnMut(_)>);
    refresh_button
        .add_event_listener_with_callback("click", &refresh_button_closure.as_ref().unchecked_ref())
        .expect("Cannot add refresh_button on click event");
    refresh_button_closure.forget();

    //Register main turtle with bevy!
    commands.insert_resource(MainTurtle(main_turtle.clone()));

    //This spawn thing is expensive, but whatevet
    spawn_local(async move {
        //This will init the turtles list
        update_turtle_list(&main_turtle, &turtle_vec).await;
    });
}

async fn update_turtle_list(
    main_turtle: &Arc<RwLock<Option<JsonTurtle>>>,
    global_turtle_vec: &Arc<RwLock<HashMap<Uuid, JsonTurtle>>>,
) {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let navbar_div = document
        .query_selector(".navbar_div")
        .expect("No navbar found")
        .expect("No navbar found");
    let refresh_button = document
        .query_selector(".refresh_button")
        .ok()
        .expect("No refresh button found")
        .expect("No navbar found");

    let turtle_list = get_turtles_list().await;

    let mut global_turtles_guard = global_turtle_vec
        .write()
        .expect("Cannot lock global turtles!");
    let mut main_turtle_guard = main_turtle.write().expect("Cannot lock main turtle");
    let on_click_closure = unsafe { ON_CLICK_CLOSURE.as_ref().unwrap_unchecked() };

    if let Some(global_turtle) = main_turtle_guard.as_mut() {
        if !turtle_list.contains(global_turtle) {
            *main_turtle_guard = None;
        }
    }

    global_turtles_guard
        .iter()
        .skip(turtle_list.len())
        .for_each(|(uuid, _)| {
            //Here we remove, as it no longer exists
            let text_to_remove = match document
                .query_selector(&format!("[data-id=\"{:?}\"]", uuid))
                .expect("Query select error")
            {
                Some(val) => val,
                None => {
                    log::error!("Please stop fucking trying to hack my app!!!!!!!!");
                    return;
                }
            };

            let parrent = text_to_remove.parent_node().expect("No parrent node?");
            navbar_div
                .remove_child(&parrent)
                .expect("Couldn't remove node from navbar");
        });

    for (i, turtle) in turtle_list.iter().enumerate() {
        if let Some(global_turtle) = global_turtles_guard.get_mut(&turtle.uuid) {
            if global_turtle == turtle {
                continue;
            } else {
                *global_turtle = turtle.clone();
                continue;
            }
        }

        let turtle_navbar_div = document.create_element("div").expect("Cannot create div");
        turtle_navbar_div.set_class_name("navbar_item_div");

        let turtle_navbar_text = document
            .create_element("p")
            .expect("Cannot create P element");
        turtle_navbar_text.set_inner_html(&i.to_string());
        turtle_navbar_text.set_class_name("navbar_item_text");
        turtle_navbar_text
            .add_event_listener_with_callback("pointerdown", on_click_closure.unchecked_ref())
            .expect("Cannot set event listener");
        turtle_navbar_text
            .set_attribute("data-id", &turtle.uuid.to_string())
            .expect("Cannot set uuid atribute");

        turtle_navbar_div
            .append_child(&turtle_navbar_text)
            .expect("Cannot push text to child navbar div");
        //navbar_div.append_child(&turtle_navbar_div).expect("Cannot push child div to navbar");
        navbar_div
            .insert_before(&turtle_navbar_div, Some(&refresh_button))
            .expect("Cannot inseft before refresh_button");
    }

    *global_turtles_guard = turtle_list
        .iter()
        .map(|item| (item.uuid, item.clone()))
        .collect();
}

async fn get_turtles_list() -> Vec<JsonTurtle> {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");

    let mut get_turtles_url = document
        .base_uri()
        .expect("Base uri get fail")
        .expect("No base uri");
    get_turtles_url.push_str("turtle/list/");

    let mut opts = RequestInit::new();
    opts.method("GET");
    let request =
        Request::new_with_str_and_init(&get_turtles_url, &opts).expect("Cannot create new request");
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .expect("Cannot fetch value");

    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into().expect("Cannot cast into response");

    let json = JsFuture::from(resp.json().expect("Cannot get json"))
        .await
        .expect("Cannot get future from JS");
    return serde_wasm_bindgen::from_value(json).expect("Json serde error");
}

fn send_turtle_change_event(
    main_turtle: Res<MainTurtle>,
    mut previous_turtle: ResMut<TurtleEventChangeState>,
    mut ev_change: EventWriter<SelectTurtleEvent>,
) {
    let previous_turtle = &mut previous_turtle.previous_turtle;
    let guard = main_turtle
        .read()
        .expect("Cannot lock main turtle, should never happen!");
    match &*guard {
        Some(val) => {
            match previous_turtle {
                Some(previous_turtle) => {
                    if previous_turtle.uuid != val.uuid {
                        ev_change.send(SelectTurtleEvent(Some(val.clone())));
                        *previous_turtle = val.clone();
                    }
                }
                None => {
                    //We have a new turtle!
                    ev_change.send(SelectTurtleEvent(Some(val.clone())));
                    *previous_turtle = Some(val.clone());
                }
            }
        }
        None => {
            if previous_turtle.is_some() {
                ev_change.send(SelectTurtleEvent(None));
                *previous_turtle = None;
            }
        }
    };
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_ui_system)
            .insert_resource(TurtleEventChangeState {
                previous_turtle: None,
            })
            .add_system(send_turtle_change_event);
    }
}
