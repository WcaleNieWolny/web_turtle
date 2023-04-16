use std::{net::SocketAddr, sync::Arc, collections::HashMap, time::Duration};

use axum::{Router, extract::{WebSocketUpgrade, ConnectInfo, ws::{WebSocket, Message}, State, RawBody}, response::{IntoResponse, Response}, routing::{get, post, put}, http::StatusCode, body::Full};
use thiserror::Error;
use tokio::{sync::{Mutex, oneshot, mpsc}, time::timeout};
use tower_http::trace::{TraceLayer, DefaultMakeSpan};
use tracing::error;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum TurtleRequestError {
    #[error("Invalid WebSocket client response")] 
    InvalidResponse,
    #[error("Data send error")]
    DataSendError(#[from] axum::Error),
    #[error("Recive msg from WebSocket error")]
    RecvMsgError,
    #[error("WebSocket closed")]
    WsClosed,
    #[error("Timed out")]
    TimeOut,
    #[error("Cannot send request")]
    RequestSendError,
    #[error("Response recv error")]
    ResponseRecvError
}

struct TurtleAsyncRequest {
    request: String,
    response: oneshot::Sender<Result<String, TurtleRequestError>>
}

#[derive(Debug)]
struct Turtle {
    request_queue: mpsc::Sender<TurtleAsyncRequest>
}

impl Turtle {
    async fn command(&mut self, command: &String) -> Result<String, TurtleRequestError> {
        let (tx, rx) = oneshot::channel::<Result<String, TurtleRequestError>>();

        let request = TurtleAsyncRequest {
            request: command.clone(),
            response: tx,
        };

        match timeout(Duration::from_secs(5), self.request_queue.send(request)).await {
            Ok(val) => val.or(Err(TurtleRequestError::RequestSendError))?,
            Err(_) => return Err(TurtleRequestError::TimeOut),
        }

        match timeout(Duration::from_secs(10), rx).await {
            Ok(val) => return val.or(Err(TurtleRequestError::ResponseRecvError))?,
            Err(_) => return Err(TurtleRequestError::TimeOut),
        }
    }
}

#[derive(Clone, Default)]
struct TurtlesState {
    turtles: Arc<Mutex<HashMap<Uuid, Turtle>>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "turtle_weboscket=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();


    // build our application with some routes
    let app = Router::new()
        .route("/turtle/", get(ws_handler))
        .route("/turtle/command/", put(command_turtle))
        // logging so we can see whats going on
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .with_state(TurtlesState::default());

    // run it with hyper
    //
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
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
    command: String 
) -> Result<impl IntoResponse, (StatusCode, impl IntoResponse)> {

    let mut guard = turtles.turtles.lock().await;

    for (_, v) in guard.iter_mut() {
        match v.command(&command).await {
            Ok(val) => return Ok(val),
            Err(error) => return Err((StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))
        }
    }

    return Err((StatusCode::NOT_FOUND, StatusCode::NOT_FOUND.to_string()));
}

async fn handle_socket(mut socket: WebSocket, addr: SocketAddr, turtles: TurtlesState)  {

    let (tx, mut rx) = mpsc::channel::<TurtleAsyncRequest>(64);
    let uuid = Uuid::new_v4();
    let turtle = Turtle {
        request_queue: tx,
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

                match socket.recv().await {
                    Some(msg) => {
                        match msg {
                            Ok(msg) => {
                                match msg {
                                    Message::Text(msg) => break 'response Ok(msg),
                                    Message::Close(_) => break 'response Err(TurtleRequestError::WsClosed),
                                    _ => break 'response Err(TurtleRequestError::InvalidResponse)
                                }
                            },
                            Err(_) => break 'response Err(TurtleRequestError::RecvMsgError),
                        }
                    },
                    None => break 'main_loop,
                }
            };

            if let Err(_) = request.response.send(response) {
                error!("Cannot send turtle request response!");
                break;
            };
        }
    }

    let mut guard = turtles.turtles.lock().await;
    guard.remove(&uuid);
    drop(guard);
}
