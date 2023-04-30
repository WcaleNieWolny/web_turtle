use std::time::Duration;

use thiserror::Error;
use tokio::{sync::{oneshot, mpsc}, time::timeout};

use crate::database::TurtleData;

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
    RIGHT,
    BAKCWARD,
    LEFT
}

impl ToString for MoveDirection {
    fn to_string(&self) -> String {
        match self {
            MoveDirection::FORWARD => "forward".to_string(),
            MoveDirection::BAKCWARD => "backward".to_string(),
            MoveDirection::LEFT => "left".to_string(),
            MoveDirection::RIGHT => "right".to_string(),
        }
    }
}

impl MoveDirection {
    pub fn from_i32(number: i32) -> Self {
        match number {
            0 => Self::FORWARD,
            1 => Self::RIGHT,
            2 => Self::BAKCWARD,
            3 => Self::LEFT,
            _ => panic!("Invalid i32 number to MoveDirection, this should NEVER happen")
        }
    }
    pub fn to_i32(&self) -> i32 {
        match &self {
            Self::FORWARD => 0,
            Self::RIGHT => 1,
            Self::BAKCWARD => 2,
            Self::LEFT => 3,
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
            MoveDirection::FORWARD => {
                "return turtle.forward()"
            },
            MoveDirection::BAKCWARD => "return turtle.back()",
            MoveDirection::RIGHT => "return turtle.turnRight()",
            MoveDirection::LEFT => "return turtle.turnLeft()"
        };

        let result = self.command(command).await?;
        match result.as_str() {
            "true" => {
                match direction {
                    MoveDirection::RIGHT => {
                        let mut enum_number = self.turtle_data.rotation;
                        if enum_number == 3 {
                            enum_number = 0
                        } else {
                            enum_number += 1
                        }
                        self.turtle_data.rotation = enum_number
                    }
                    MoveDirection::LEFT => {
                        let mut enum_number = self.turtle_data.rotation;
                        if enum_number == 0 {
                            enum_number = 3
                        } else {
                            enum_number -= 1
                        }
                        self.turtle_data.rotation = enum_number
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
