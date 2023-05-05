use std::time::Duration;

use shared::{JsonTurtleRotation, WorldChange, WorldChangeAction, WorldChangeDeleteBlock, TurtleBlock, WorldChangeUpdateBlock, WorldChangeNewBlock};
use thiserror::Error;
use tokio::{sync::{oneshot, mpsc}, time::timeout};
use tracing::error;

use crate::{database::{TurtleData, DatabaseActionError, Connection, BlockData}, schema::MoveDirection, world};

//Lua inspect logic
static INSPECT_DOWN_PAYLOAD: &str = "local has_block, data = turtle.inspectDown() return textutils.serialiseJSON(data)";
static INSPECT_FORWARD_PAYLOAD: &str = "local has_block, data = turtle.inspect() return textutils.serialiseJSON(data)";
static INSPECT_UP_PAYLOAD: &str = "local has_block, data = turtle.inspectUp() return textutils.serialiseJSON(data)";

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

#[derive(Error, Debug)]
pub enum TurtleWorldScanError {
    #[error("Request error")]
    RequestError(#[from] TurtleRequestError),
    #[error("Database error")]
    DatabaseError(#[from] DatabaseActionError),
    #[error("Json error")]
    JsonError(#[from] serde_json::error::Error),
    #[error("Cannot rotate turtle")]
    TurtleRotationError(#[from] TurtleMoveError),
    #[error("Turtle does not have ID")]
    InvalidTurtle
}


pub struct TurtleAsyncRequest {
    pub request: String,
    pub response: oneshot::Sender<Result<String, TurtleRequestError>>
}

impl ToString for MoveDirection {
    fn to_string(&self) -> String {
        self.to_json_enum().to_string()
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

    /// # Returns
    /// A tuple (x, y, z)
    /// # Safety 
    /// NEVER CALL THIS FUNCTION WITH MoveDirection::Left OR MoveDirection:Right
    fn to_turtle_move_diff(&self, turtle: &Turtle) -> (i32, i32, i32) {
        return self.to_json_enum().to_turtle_move_diff(turtle.turtle_data.rotation.to_json_enum())
    }

    pub fn to_json_enum(&self) -> JsonTurtleRotation {
        match self {
            MoveDirection::Forward => JsonTurtleRotation::Forward,
            MoveDirection::Right => JsonTurtleRotation::Right,
            MoveDirection::Backward => JsonTurtleRotation::Backward,
            MoveDirection::Left => JsonTurtleRotation::Left,
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
                    direction => {
                        let (x_diff, y_diff, z_diff) = direction.to_turtle_move_diff(&self);
                        self.turtle_data.x += x_diff;
                        self.turtle_data.y += y_diff;
                        self.turtle_data.z += z_diff;
                    }
                };
                return Ok(()); 
            },
            "false" => return Err(TurtleMoveError::CannotMove),
            _ => return Err(TurtleMoveError::InvalidTurtleResponse(result))
        }
    }

    pub async fn scan_world_changes(&mut self, connection: &mut Connection) -> Result<Vec<WorldChange>, TurtleWorldScanError> {
        let x = self.turtle_data.x;
        let y = self.turtle_data.y;
        let z = self.turtle_data.z;
        let turtle_id = self.turtle_data.id.ok_or(TurtleWorldScanError::InvalidTurtle)?;

        let blocks: Vec<(String, i32, i32, i32)> = vec![
            (self.command(INSPECT_DOWN_PAYLOAD).await?, x, y - 1, z),
            {
                let (x_diff, y_diff, z_diff) = MoveDirection::Forward.to_turtle_move_diff(&self);
                let forward = self.command(INSPECT_FORWARD_PAYLOAD).await?;
                (forward, x + x_diff, y + y_diff, z + z_diff)
            },
            (self.command(INSPECT_UP_PAYLOAD).await?, x, y + 1, z),
        ];

         let changes = blocks
            .into_iter()
            .map(|(block, x, y, z)| {
                let db_block = BlockData::read_by_xyz(connection, x, y, z).ok();

                if block == "\"No block to inspect\"" {
                    if db_block.is_none() {
                        return Ok(None);
                    }

                    BlockData::delete_by_xyz(connection, x, y, z)?;
                    let action = WorldChangeAction::Delete(WorldChangeDeleteBlock {});
                    return Ok(Some(WorldChange { x, y, z, action }));
                }

                let name = serde_json::from_str::<TurtleBlock>(&block)?.name;
                let color = world::block_color(&name);

                let action = if let Some(mut db_block) = db_block {
                    if name == db_block.name {
                        return Ok(None);
                    }

                    db_block.name = name;
                    db_block.update(connection)?;
                    WorldChangeAction::Update(WorldChangeUpdateBlock {
                        color,
                    })
                } else {
                    let new_db_block = BlockData {
                        id: None,
                        turtle_id,
                        x,
                        y,
                        z,
                        name,
                    };
                    new_db_block.insert(connection)?;
                    WorldChangeAction::New(WorldChangeNewBlock {
                        color,
                    })
                };

                Ok(Some(WorldChange { x, y, z, action }))
            })
            .filter_map(Result::transpose)
            .collect::<Result<Vec<WorldChange>, TurtleWorldScanError>>()?;       

        Ok(changes)
    }
}
