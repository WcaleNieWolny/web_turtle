mod turtle;
mod database;
mod schema;

use std::{net::SocketAddr, sync::Arc, collections::HashMap, time::Duration, error::Error};
use axum::{Router, extract::{WebSocketUpgrade, ConnectInfo, ws::{WebSocket, Message}, State, Path}, response::IntoResponse, routing::{get, put}, http::StatusCode, Json};
use database::{SqlitePool, TurtleData, Connection, DatabaseActionError};
use tokio::{sync::{Mutex, mpsc}, time::timeout};
use tower_http::{trace::{TraceLayer, DefaultMakeSpan}, cors::{CorsLayer, Any}};
use tracing::{error, warn};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};
use turtle::{Turtle, TurtleRequestError, TurtleAsyncRequest, MoveDirection};
use uuid::Uuid;

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

    return turtle.move_turtle(direction).await.map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()));
}

async fn list_turtles(
    State(turtles): State<TurtlesState>
) -> Json<Vec<String>>{
    let turtles = turtles.turtles.lock().await;
    return Json(turtles.iter()
        .map(|(uuid, _)| uuid.to_string())
        .collect());
}

async fn handle_socket(mut socket: WebSocket, _addr: SocketAddr, turtles: TurtlesState)  {

    let (tx, mut rx) = mpsc::channel::<TurtleAsyncRequest>(64);
    let uuid = Uuid::new_v4();
    let turtle = Turtle {
        request_queue: tx,
    };

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

    let turtle_data = TurtleData::read_by_uuid(&mut conn, &uuid);
    println!("Some: {}", turtle_data.is_ok());

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
