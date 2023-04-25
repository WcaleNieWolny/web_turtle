use std::time::Duration;

use thiserror::Error;
use tokio::{sync::{oneshot, mpsc}, time::timeout};

//Lua inspect logic
//local has_block, data = turtle.inspectDown() return textutils.serialise(data)

#[derive(Error, Debug)]
pub enum TurtleRequestError {
    #[error("Invalid WebSocket client response")] 
    InvalidResponse,
    #[error("Data send error")]
    DataSendError(#[from] axum::Error),
    #[error("WebSocket closed")]
    WsClosed,
    #[error("Timed out")]
    TimeOut,
    #[error("Cannot send request")]
    RequestSendError,
    #[error("Response recv error")]
    ResponseRecvError
}

#[derive(Error, Debug)]
pub enum TurtleMoveError {
    #[error("Request error")]
    RequestError(#[from] TurtleRequestError),
    #[error("Cannot move turtle")]
    CannotMove,
    #[error("Invalid turtle response ({0})")]
    InvalidTurtleResponse(String),
    #[error("Not yet implemented")]
    NotImplemented
}


pub struct TurtleAsyncRequest {
    pub request: String,
    pub response: oneshot::Sender<Result<String, TurtleRequestError>>
}

pub enum MoveDirection {
    FORWARD,
    BAKCWARD,
    LEFT,
    RIGHT
}

#[derive(Debug)]
pub struct Turtle {
    pub request_queue: mpsc::Sender<TurtleAsyncRequest>
}

impl Turtle {
    pub async fn command(&mut self, command: &str) -> Result<String, TurtleRequestError> {
        let (tx, rx) = oneshot::channel::<Result<String, TurtleRequestError>>();

        let request = TurtleAsyncRequest {
            request: command.to_string(),
            response: tx,
        };

        match timeout(Duration::from_secs(3), self.request_queue.send(request)).await {
            Ok(val) => val.or(Err(TurtleRequestError::RequestSendError))?,
            Err(_) => return Err(TurtleRequestError::TimeOut),
        }

        match timeout(Duration::from_secs(10), rx).await {
            Ok(val) => return val.or(Err(TurtleRequestError::ResponseRecvError))?,
            Err(_) => return Err(TurtleRequestError::TimeOut),
        }
    }

    pub async fn move_turtle(&mut self, direction: MoveDirection) -> Result<(), TurtleMoveError> {
        let command = match direction {
            MoveDirection::FORWARD => {
                "return turtle.forward()"
            },
            MoveDirection::BAKCWARD => "return turtle.back()",
            _ => return Err(TurtleMoveError::NotImplemented)
        };

        let result = self.command(command).await?;
        match result.as_str() {
            "true" => return Ok(()),
            "false" => return Err(TurtleMoveError::CannotMove),
            _ => return Err(TurtleMoveError::InvalidTurtleResponse(result))
        }
    }
}
