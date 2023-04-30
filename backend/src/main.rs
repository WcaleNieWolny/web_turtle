mod turtle;
mod database;
mod schema;

use std::{net::SocketAddr, sync::Arc, collections::HashMap, time::Duration, error::Error};
use axum::{Router, extract::{WebSocketUpgrade, ConnectInfo, ws::{WebSocket, Message}, State, Path}, response::IntoResponse, routing::{get, put}, http::StatusCode, Json};
use database::{SqlitePool, TurtleData, Connection, DatabaseActionError};
use tokio::{sync::{Mutex, mpsc}, time::timeout};
use tower_http::{trace::{TraceLayer, DefaultMakeSpan}, cors::{CorsLayer, Any}};
use tracing::{error, warn, debug};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};
use turtle::{Turtle, TurtleRequestError, TurtleAsyncRequest, MoveDirection};
use uuid::Uuid;
use serde_json::{json, Value};

static GET_OS_LABEL_PAYLOAD: &str = "local ok, err = os.computerLabel() return ok";

#[derive(Clone)]
struct TurtlesState {
    turtles: Arc<Mutex<HashMap<Uuid, Turtle>>>,
    pool: Arc<SqlitePool>
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

    let pool = database::init()?; 
    let state = TurtlesState {
        turtles: Default::default(),
        pool: Arc::new(pool) 
    };

    // build our application with some routes
    let app = Router::new()
        .route("/turtle/", get(ws_handler))
        .route("/turtle/:id/command/", put(command_turtle))
        .route("/turtle/:id/move/", put(move_turtle))
        .route("/turtle/list/", get(list_turtles))
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
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
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
        "forward" => MoveDirection::FORWARD,
        "backward" => MoveDirection::BAKCWARD,
        "left" => MoveDirection::LEFT,
        "right" => MoveDirection::RIGHT,
        _ => return Err((StatusCode::BAD_REQUEST, StatusCode::BAD_REQUEST.to_string())),
    };

    turtle.move_turtle(direction).await.map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let conn: Result<Connection, DatabaseActionError> = turtles.pool.clone().try_into();
    let mut conn = conn.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Connection pool empty".to_string()))?;

    turtle.turtle_data.update(&mut conn).map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    Ok(StatusCode::OK)
}

async fn list_turtles(
    State(turtles): State<TurtlesState>
) -> Json<Vec<Value>>{
    let turtles = turtles.turtles.lock().await;

    return Json(
        turtles.iter()
        .enumerate()
        .map(|(id, (uuid, turtle))| {
            json!({
                "id": id,
                "uuid": uuid.to_string(),
                "x": turtle.turtle_data.x,
                "y": turtle.turtle_data.y,
                "z": turtle.turtle_data.z,
                "rotation": MoveDirection::from_i32(turtle.turtle_data.rotation).to_string()
            })
        })
        .collect());
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

    //Attempt to get turtle by uuid
    let conn: Result<Connection, DatabaseActionError> = turtles.pool.clone().try_into();
    let mut conn = match conn {
        Ok(val) => val,
        Err(err) => {
            warn!("Socket poll empty ({})", err.to_string());
            if let Err(close_err) = socket.close().await {
                error!("Cannot close WebSocket {close_err}") 
            };
            return;
        }
    };

    let (turtle_data, uuid) = 'turtle_data: {
        let socket_msg = send_payload!(GET_OS_LABEL_PAYLOAD);
        let parsed_uuid = Uuid::try_parse(&socket_msg);

        if socket_msg == "nil" || parsed_uuid.is_err() {
            //We have a unknown turtle
            let new_uuid = Uuid::new_v4();
            let set_payload = format!("return os.setComputerLabel(\"{}\")", new_uuid.simple().to_string());
            let _ = send_payload!(set_payload);

            //Now we try to insert that uuid into the db
            let turtle_data = TurtleData {
                id: None,
                uuid: new_uuid.to_string(),
                x: 0,
                y: 0,
                z: 0,
                rotation: 0  //Forward,
            };

           match turtle_data.put(&mut conn) {
               Ok(val) => break 'turtle_data (val, new_uuid),
               Err(err) => {
                   error!("Database error {err}");
                   close_socket!();
                   return;
               }
           };
        } else {
            //Cannot failed, we checked for parsing error above
            let uuid = unsafe { 
                parsed_uuid.unwrap_unchecked()
            };

            let db_turtle_data = TurtleData::read_by_uuid(&mut conn, &uuid);
            match db_turtle_data {
                Ok(val) => break 'turtle_data (val, uuid),
                Err(err) => {
                    debug!("Database error ({err})");
                    close_socket!();
                    return;
                }
            }
        }
    };

    let turtle = Turtle {
        request_queue: tx,
        turtle_data
    };

    //Add new turtle
    let mut guard = turtles.turtles.lock().await;
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
