use bevy::{prelude::*, window::WindowResolution};
use futures::channel::mpsc::{Receiver, Sender, self};
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::UiEvent;

static mut RESIZE_TX: Option<Sender<(f32, f32)>> = None;

pub struct ResizePlugin;

#[derive(Component)]
struct ResizeComponent {
    rx: Receiver<(f32, f32)>
}

fn init_resize_system(mut commands: Commands) {
    let (resize_tx, resize_rx) = mpsc::channel::<(f32, f32)>(8); 
    let component = ResizeComponent { rx: resize_rx };

    //Do not care about unsafe :)
    unsafe {
        RESIZE_TX = Some(resize_tx);
    }

    let window = web_sys::window().expect("no global `window` exists");

    let resize_closure = Closure::wrap(Box::new(|e: UiEvent| {
        let tx = unsafe {
            RESIZE_TX.as_mut().unwrap()
        };

        let local_window = web_sys::window().expect("no global `window` exists");
        let document = local_window.document().expect("should have a document on window");
        let navbar_div = document.query_selector(".navbar_div").ok().expect("No navbar found").expect("No navbar found");
        let height = local_window.inner_height().expect("No inner height").as_f64().expect("Inner height not a number") as f32 - navbar_div.client_height() as f32;
        let width = navbar_div.client_width() as f32;
       
        tx.try_send((width, height)).expect("Cannot send data!")
    }) as Box<dyn FnMut(_)>);

    window.add_event_listener_with_callback("resize", &resize_closure.as_ref().unchecked_ref()).expect("Cannot add resize listener");

    //Required or BAD things will happen!
    resize_closure.forget();

    //Make sure bevy can see our component
    commands.spawn(component);
}

fn check_window_size(mut resize_component: Query<&mut ResizeComponent>, mut windows: Query<&mut Window>) {
    let mut component = resize_component.single_mut();
   
    while let Ok(val) = component.rx.try_next() {
        match val {
            Some((width, height)) => {
                let mut window = windows.single_mut();
                window.resolution.set(width, height)
            },
            None => unreachable!() //This channel will never be closed
        }
    }
}

impl Plugin for ResizePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_startup_system(init_resize_system)
            .add_system(check_window_size);
    }
}
