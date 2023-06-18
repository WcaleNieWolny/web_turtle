mod turtle;
mod database;
mod world;

use std::{net::SocketAddr, sync::Arc, collections::HashMap, time::Duration, error::Error, str::FromStr};
use axum::{Router, extract::{WebSocketUpgrade, ConnectInfo, ws::{WebSocket, Message}, State, Path, Query}, response::IntoResponse, routing::{get, put}, http::StatusCode, Json};
use database::DatabaseActionError;
use shared::{JsonTurtle, TurtleMoveResponse, JsonTurtleDirection};
use tokio::{sync::{Mutex, mpsc}, time::timeout};
use tower_http::{trace::{TraceLayer, DefaultMakeSpan}, cors::{CorsLayer, Any}};
use tracing::{error, warn, debug};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};
use turtle::{Turtle, TurtleRequestError, TurtleAsyncRequest};
use uuid::Uuid;

use crate::database::TurtleDatabase;

static GET_OS_LABEL_PAYLOAD: &str = "local ok, err = os.computerLabel() return ok";

#[derive(Clone)]
struct TurtlesState {
    turtles: Arc<Mutex<HashMap<Uuid, Turtle>>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = TurtlesState {
        turtles: Default::default(),
    };

    // build our application with some routes
    let app = Router::new()
        .route("/turtle/", get(ws_handler))
        .route("/turtle/:id/command/", put(command_turtle))
        .route("/turtle/:id/move/", put(move_turtle))
        .route("/turtle/list/", get(list_turtles))
        .route("/turtle/:id/world/", get(get_world))
        .route("/turtle/:id/destroy/", put(destroy_block))
        .route("/turtle/:id/inventory/", get(get_inventory))
        // logging so we can see whats going on
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any),
        )
        .with_state(state);

    // run it with hyper
    //
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;

    return Ok(());
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(turtles): State<TurtlesState>
) -> impl IntoResponse {
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, addr, turtles))
}

async fn command_turtle(
    State(turtles): State<TurtlesState>,
    Path(uuid): Path<String>,
    command: String 
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    let mut guard = turtles.turtles.lock().await;

    let uuid = Uuid::parse_str(&uuid).or(Err((StatusCode::BAD_REQUEST, StatusCode::BAD_REQUEST.to_string())))?;
    let turtle = match guard.get_mut(&uuid) {
        Some(v) => v,
        None => return Err((StatusCode::NOT_FOUND, StatusCode::NOT_FOUND.to_string())) 
    };

    return turtle.command(&command).await.map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()));
}

async fn move_turtle(
    State(turtles): State<TurtlesState>,
    Path(uuid): Path<String>,
    command: String 
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    let mut guard = turtles.turtles.lock().await;

    let uuid = Uuid::parse_str(&uuid).or(Err((StatusCode::BAD_REQUEST, StatusCode::BAD_REQUEST.to_string())))?;
    let turtle = match guard.get_mut(&uuid) {
        Some(v) => v,
        None => return Err((StatusCode::NOT_FOUND, StatusCode::NOT_FOUND.to_string())) 
    };

    let direction = match command.as_str() {
        "forward" => JsonTurtleDirection::Forward,
        "backward" => JsonTurtleDirection::Backward,
        "left" => JsonTurtleDirection::Left,
        "right" => JsonTurtleDirection::Right,
        _ => return Err((StatusCode::BAD_REQUEST, StatusCode::BAD_REQUEST.to_string())),
    };

    turtle.move_turtle(direction).await.map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    //turtle.turtle_data.update(&mut conn).map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let changes = turtle.scan_world_changes().await.map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    Ok(Json(TurtleMoveResponse {
        x: turtle.database.turtle_data.x,
        y: turtle.database.turtle_data.y,
        z: turtle.database.turtle_data.z,
        rotation: turtle.database.turtle_data.rotation.clone(),
        changes,
    }))
}

async fn list_turtles(
    State(turtles): State<TurtlesState>
) -> Json<Vec<JsonTurtle>>{
    let turtles = turtles.turtles.lock().await;

    return Json(
        turtles.iter()
        .enumerate()
        .map(|(id, (uuid, turtle))| {
            JsonTurtle {
                uuid: *uuid,
                x: turtle.database.turtle_data.x, 
                y: turtle.database.turtle_data.y,
                z: turtle.database.turtle_data.z,
                rotation: turtle.database.turtle_data.rotation.clone(),
            }
        })
        .collect());
}

async fn get_world(
    State(turtles): State<TurtlesState>,
    Path(uuid): Path<Uuid>,
    Query(params): Query<HashMap<String, i32>>
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    let mut guard = turtles.turtles.lock().await;

    let turtle = match guard.get_mut(&uuid) {
        Some(v) => v,
        None => return Err((StatusCode::NOT_FOUND, StatusCode::NOT_FOUND.to_string())) 
    };

    let world = turtle.database.raw_world();
    Ok(world)
}

async fn destroy_block(
    State(turtles): State<TurtlesState>,
    Path(uuid): Path<String>,
    command: String 
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    let mut guard = turtles.turtles.lock().await;

    let uuid = Uuid::parse_str(&uuid).or(Err((StatusCode::BAD_REQUEST, StatusCode::BAD_REQUEST.to_string())))?;
    let turtle = match guard.get_mut(&uuid) {
        Some(v) => v,
        None => return Err((StatusCode::NOT_FOUND, StatusCode::NOT_FOUND.to_string())) 
    };

    let side = JsonTurtleDirection::from_str(&command).or(Err((StatusCode::BAD_REQUEST, StatusCode::BAD_REQUEST.to_string())))?;

    match turtle.destroy_block(side).await {
        Ok(val) => return Ok(Json(val)),
        Err(err) => Err((StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
    }
}

async fn get_inventory(
    State(turtles): State<TurtlesState>,
    Path(uuid): Path<String>
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {
    let mut guard = turtles.turtles.lock().await;

    let uuid = Uuid::parse_str(&uuid).or(Err((StatusCode::BAD_REQUEST, StatusCode::BAD_REQUEST.to_string())))?;
    let turtle = match guard.get_mut(&uuid) {
        Some(v) => v,
        None => return Err((StatusCode::NOT_FOUND, StatusCode::NOT_FOUND.to_string())) 
    };

    let inventory = turtle.get_inventory().await.map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    Ok(Json(inventory))
}

async fn handle_socket(mut socket: WebSocket, _addr: SocketAddr, turtles: TurtlesState)  {
    macro_rules! close_socket {
        () => {
             if let Err(close_err) = socket.close().await {
                error!("Cannot close WebSocket {close_err}") 
            };
        };
    }
    macro_rules! send_payload {
        ($payload:expr) => {
            'main: {
                //For some reason the user disconected
                if let Err(_) = socket.send(Message::Text($payload.to_string())).await {
                    close_socket!();
                    return; 
                }

                let socket_msg = match timeout(Duration::from_secs(5), socket.recv()).await {
                    Ok(val) => {
                        match val {
                            Some(val) => {
                                match val {
                                    Ok(val) => {
                                        match val {
                                            Message::Text(val) => val,
                                            _ => {
                                                close_socket!();
                                                return;
                                            },
                                        }
                                    }
                                    Err(err) => {
                                        warn!("After recv something went wrong {}", err);
                                        close_socket!();
                                        return;
                                    }
                                }
                            },
                            None => {
                                //Socket closed so no closing
                                return;
                            }
                        }
                    },
                    Err(_) => {
                        //That is a time out
                        close_socket!();
                        return;           
                    }
                };
                break 'main socket_msg
            }
        };
    }

    let (tx, mut rx) = mpsc::channel::<TurtleAsyncRequest>(64);

    //TODO: Attempt to get turtle by uuid

    let uuid = {
        let socket_msg = send_payload!(GET_OS_LABEL_PAYLOAD);
        let parsed_uuid = Uuid::try_parse(&socket_msg);

        if socket_msg == "nil" || parsed_uuid.is_err() {
            //We have a unknown turtle
            let new_uuid = Uuid::new_v4();
            let set_payload = format!("return os.setComputerLabel(\"{}\")", new_uuid.simple().to_string());
            let _ = send_payload!(set_payload);

            new_uuid
        } else {
            parsed_uuid.unwrap()
        }
    };

    let database = match TurtleDatabase::create_from_id(uuid).await {
        Ok(val) => val,
        Err(err) => {
            error!("Database error for turtle {uuid:?} Err: {err}");
            close_socket!();
            return;
        }
    };

    let turtle = Turtle::new(uuid, database, tx);

    //Add new turtle
    let mut guard = turtles.turtles.lock().await;
    if let Some(_) = guard.get(&uuid) {
        warn!("Turtle overwrite attempt");
        close_socket!();
        return;
    }

    guard.insert(uuid.clone(), turtle);
    drop(guard);

    'main_loop: loop {
        if let Some(request) = rx.recv().await {
            let response: Result<String, TurtleRequestError> = 'response: {

                if let Err(err) = socket.send(Message::Text(request.request)).await {
                    break 'response Err(TurtleRequestError::DataSendError(err))
                };


                let socket_msg = match timeout(Duration::from_secs(5), socket.recv()).await {
                    Ok(val) => val,
                    Err(_) => {
                        if let Err(_) = request.response.send(Err(TurtleRequestError::TimeOut)) {
                            error!("Cannot send turtle request response!");
                        };
                        
                        //We do not care if the socket close goes well!
                        let _ = socket.close().await;
                        break 'main_loop;
                    }
                };

                match socket_msg {
                    Some(msg) => {
                        match msg {
                            Ok(msg) => {
                                match msg {
                                    Message::Text(msg) => break 'response Ok(msg),
                                    Message::Close(_) => {
                                         if let Err(_) = request.response.send(Err(TurtleRequestError::WsClosed)) {
                                            error!("Cannot send turtle request response!");
                                        };
                                        break 'main_loop
                                    },
                                    _ => break 'response Err(TurtleRequestError::InvalidResponse)
                                }
                            },
                            Err(_) => {
                                if let Err(_) = request.response.send(Err(TurtleRequestError::WsClosed)) {
                                    error!("Cannot send turtle request response!");
                                };
                                break 'main_loop
                            },
                        }
                    },
                    None => {
                        if let Err(_) = request.response.send(Err(TurtleRequestError::WsClosed)) {
                            error!("Cannot send turtle request response!");
                        };
                        break 'main_loop; 
                    },
                }
            };

            if let Err(_) = request.response.send(response) {
                error!("Cannot send turtle request response!");
                break 'main_loop;
            };
        }
    }

    let mut guard = turtles.turtles.lock().await;
    guard.remove(&uuid);
    drop(guard);
}
