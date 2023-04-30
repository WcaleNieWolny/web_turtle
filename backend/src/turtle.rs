use std::time::Duration;

use thiserror::Error;
use tokio::{sync::{oneshot, mpsc}, time::timeout};

use crate::{database::TurtleData, schema::MoveDirection};

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
}


pub struct TurtleAsyncRequest {
    pub request: String,
    pub response: oneshot::Sender<Result<String, TurtleRequestError>>
}

impl ToString for MoveDirection {
    fn to_string(&self) -> String {
        match self {
            MoveDirection::Forward => "forward".to_string(),
            MoveDirection::Backward => "backward".to_string(),
            MoveDirection::Left => "left".to_string(),
            MoveDirection::Right => "right".to_string(),
        }
    }
}

impl MoveDirection {
    pub fn from_i32(number: i32) -> Self {
        match number {
            0 => Self::Forward,
            1 => Self::Right,
            2 => Self::Backward,
            3 => Self::Left,
            _ => panic!("Invalid i32 number to MoveDirection, this should NEVER happen")
        }
    }
    pub fn to_i32(&self) -> i32 {
        match &self {
            Self::Forward => 0,
            Self::Right => 1,
            Self::Backward => 2,
            Self::Left => 3,
        }
    }
}

#[derive(Debug)]
pub struct Turtle {
    pub request_queue: mpsc::Sender<TurtleAsyncRequest>,
    pub turtle_data: TurtleData
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
            MoveDirection::Forward => {
                "return turtle.forward()"
            },
            MoveDirection::Backward => "return turtle.back()",
            MoveDirection::Right => "return turtle.turnRight()",
            MoveDirection::Left => "return turtle.turnLeft()"
        };

        let result = self.command(command).await?;
        match result.as_str() {
            "true" => {
                match direction {
                    MoveDirection::Right => {
                        let mut enum_number = self.turtle_data.rotation.to_i32();
                        if enum_number == 3 {
                            enum_number = 0
                        } else {
                            enum_number += 1
                        }
                        self.turtle_data.rotation = MoveDirection::from_i32(enum_number)
                    }
                    MoveDirection::Left => {
                        let mut enum_number = self.turtle_data.rotation.to_i32();
                        if enum_number == 0 {
                            enum_number = 3
                        } else {
                            enum_number -= 1
                        }
                        self.turtle_data.rotation = MoveDirection::from_i32(enum_number)
                    },
                    _ => {}
                };
                return Ok(()); 
            },
            "false" => return Err(TurtleMoveError::CannotMove),
            _ => return Err(TurtleMoveError::InvalidTurtleResponse(result))
        }
    }
}
