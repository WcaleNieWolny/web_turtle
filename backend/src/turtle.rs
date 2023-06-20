use std::{time::Duration, num::TryFromIntError};

use serde_json::Value;
use shared::{JsonTurtleDirection, WorldChange, WorldChangeAction, WorldChangeDeleteBlock, TurtleBlock, WorldChangeUpdateBlock, WorldChangeNewBlock, DestroyBlockResponse, JsonTurtle, world_structure::{TurtleWorld, TurtleVoxel, TurtleChunk, ChunkLocation}};
use thiserror::Error;
use tokio::{sync::{oneshot, mpsc::{self, Sender}}, time::timeout};
use tracing::error;
use uuid::Uuid;

use crate::{database::{DatabaseActionError, TurtleDatabase, self}, world};

//Lua inspect logic
static INSPECT_DOWN_PAYLOAD: &str = "local has_block, data = turtle.inspectDown() return textutils.serialiseJSON(data)";
static INSPECT_FORWARD_PAYLOAD: &str = "local has_block, data = turtle.inspect() return textutils.serialiseJSON(data)";
static INSPECT_UP_PAYLOAD: &str = "local has_block, data = turtle.inspectUp() return textutils.serialiseJSON(data)";
static DESTROY_BLOCK_FRONT: &str = "return turtle.dig()";

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
    #[error("Json error")]
    JsonError(#[from] serde_json::error::Error),
    #[error("Cannot rotate turtle")]
    TurtleRotationError(#[from] TurtleMoveError),
    #[error("Turtle does not have ID")]
    InvalidTurtle,
    #[error(transparent)]
    DynamicError(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error("Corrupted world {0}")]
    CorruptedWorld(String),
    #[error("Cannot convent int types")]
    IntError(#[from] TryFromIntError),
    #[error("Unreachable reached ({0})")]
    UnreachableReached(String),
    #[error(transparent)]
    DatabaseError(#[from] DatabaseActionError)
}

#[derive(Error, Debug)]
pub enum TurtleWorldShowError {
    #[error("Turtle does not contain ID")]
    InvalidTurtleError
}

#[derive(Error, Debug)] 
pub enum TurtleDestroyBlockError{
    #[error(transparent)]
    DatabaseError(#[from] DatabaseActionError),
    #[error("Request error")]
    RequestError(#[from] TurtleRequestError),
    #[error("Unexpected response ({0})")]
    UnexpectedResponse(String),
    #[error("Not yet implemented")]
    NotImplemented,
    #[error(transparent)]
    DynamicError(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Error, Debug)]
pub enum TurtleGetInventoryError {
    #[error("Request error")]
    RequestError(#[from] TurtleRequestError),
    #[error("Turtle response is not valid json")]
    TurtleResponseNotJson,
    #[error("Returned name does not contain \":\" symbol (cannot split)")]
    InvalidName
}

pub struct TurtleAsyncRequest {
    pub request: String,
    pub response: oneshot::Sender<Result<String, TurtleRequestError>>
}

#[derive(Debug)]
pub struct Turtle {
    request_queue: mpsc::Sender<TurtleAsyncRequest>,
    pub database: TurtleDatabase
}

impl Turtle {

    pub fn new(uuid: Uuid, database: TurtleDatabase, tx: Sender<TurtleAsyncRequest>) -> Self {
        Self {
            request_queue: tx,
            database,
        }
    }

    pub async fn command(&mut self, command: &str) -> Result<String, TurtleRequestError> {
        let (tx, rx) = oneshot::channel::<Result<String, TurtleRequestError>>();

        let request = TurtleAsyncRequest {
            request: command.to_string(),
            response: tx,
        };

        match timeout(Duration::from_secs(10), self.request_queue.send(request)).await {
            Ok(val) => val.or(Err(TurtleRequestError::RequestSendError))?,
            Err(_) => return Err(TurtleRequestError::TimeOut),
        }

        match timeout(Duration::from_secs(10), rx).await {
            Ok(val) => return val.or(Err(TurtleRequestError::ResponseRecvError))?,
            Err(_) => return Err(TurtleRequestError::TimeOut),
        }
    }

    pub async fn move_turtle(&mut self, direction: JsonTurtleDirection) -> Result<(), TurtleMoveError> {
        let command = match direction {
            JsonTurtleDirection::Forward => {
                "local a, b = turtle.forward() return a"
            },
            JsonTurtleDirection::Backward => "local a, b = turtle.back() return a",
            JsonTurtleDirection::Right => "local a, b = turtle.turnRight() return a",
            JsonTurtleDirection::Left => "local a, b = turtle.turnLeft() return a"
        };

        let result = self.command(command).await?;
        match result.as_str() {
            "true" => {
                match direction {
                    JsonTurtleDirection::Right | JsonTurtleDirection::Left => {
                        self.database.turtle_data.rotation.rotate_self(&direction); 
                    },
                    direction => {
                        let (x_diff, y_diff, z_diff) = direction.to_turtle_move_diff(&self.database.turtle_data.rotation);
                        self.database.turtle_data.x += x_diff;
                        self.database.turtle_data.y += y_diff;
                        self.database.turtle_data.z += z_diff;
                    }
                };
                return Ok(()); 
            },
            "false" => return Err(TurtleMoveError::CannotMove),
            _ => return Err(TurtleMoveError::InvalidTurtleResponse(result))
        }
    }

    pub async fn scan_world_changes(&mut self) -> Result<Vec<WorldChange>, TurtleWorldScanError> {
        let x = self.database.turtle_data.x;
        let y = self.database.turtle_data.y;
        let z = self.database.turtle_data.z;

        let blocks: Vec<(String, i32, i32, i32)> = vec![
            (self.command(INSPECT_DOWN_PAYLOAD).await?, x, y - 1, z),
            {
                let (x_diff, y_diff, z_diff) = JsonTurtleDirection::Forward.to_turtle_move_diff(&self.database.turtle_data.rotation);
                let forward = self.command(INSPECT_FORWARD_PAYLOAD).await?;
                (forward, x + x_diff, y + y_diff, z + z_diff)
            },
            (self.command(INSPECT_UP_PAYLOAD).await?, x, y + 1, z),
        ];

         let changes = blocks
            .into_iter()
            .map(|(block, x, y, z)| {
                let (loc, local_x, local_y, local_z) = TurtleWorld::get_chunk_loc_from_global_xyz(x, y, z)?;
              
                let (palette, chunks) = self.database.world.get_fields_mut();
                let chunk = chunks.get_mut_chunk_by_loc(&loc);
                let db_block: Result<Option<&mut TurtleVoxel>, TurtleWorldScanError> = match chunk {
                    None => Ok(None),
                    Some(chunk) => {
                        let voxel = match chunk.get_mut_block_by_local_xyz(local_x, local_y, local_z) {
                            Some(v) => v,
                            None => {
                                return Err(TurtleWorldScanError::UnreachableReached("lineralize error".into()))
                            }
                        };

                        Ok(Some(voxel))
                    }
                };

                let db_block = db_block?;

                if block == "\"No block to inspect\"" {
                    //this is ugly, but it works
                    if db_block.is_none() || match db_block {
                            None => false,
                            Some(x) => x.id == 0,
                        }
                    {
                        return Ok(None);
                    }

                    chunks.remove_global_block_by_xyz(x, y, z)?;
                    let action = WorldChangeAction::Delete(WorldChangeDeleteBlock {});
                    return Ok(Some(WorldChange { x, y, z, action }));
                }

                let name = serde_json::from_str::<TurtleBlock>(&block)?.name;
                let color = world::block_color(&name);

                let action = match db_block {
                    Some(db_block) if db_block.id != 0 => {
                        let db_block_name = palette.get_pallete_from_id(db_block.id).ok_or(TurtleWorldScanError::CorruptedWorld("Pallete does not containt voxel id".into()))?;
                        if name.as_str() == &*db_block_name {
                            return Ok(None);
                        }

                        let pallete_id = palette.get_pallete_index(&name);
                        db_block.id = pallete_id.try_into()?;

                        WorldChangeAction::Update(WorldChangeUpdateBlock {
                            color,
                        })

                        }
                    _ => {
                        let pallete_id: u16 = palette.get_pallete_index(&name).try_into()?;

                        //TODO: Get chunks and set new voxel
                        let chunk = chunks.force_get_mut_chunk_by_loc(&loc);
                        chunk.update_voxel_by_global_xyz(x, y, z, |voxel| {
                            voxel.id = pallete_id;
                            Ok(())
                        })?;

                        let color = world::block_to_rgb(&block);
                        WorldChangeAction::New(WorldChangeNewBlock {
                            r: color.0,
                            g: color.1,
                            b: color.2,
                        })
                    }
                };

                Ok(Some(WorldChange { x, y, z, action }))
            })
            .filter_map(Result::transpose)
            .collect::<Result<Vec<WorldChange>, TurtleWorldScanError>>()?;

        if changes.len() != 0 {
            self.database.save().await?;
        }

        Ok(changes)
    }

    pub async fn destroy_block(&mut self, side: JsonTurtleDirection) -> Result<DestroyBlockResponse, TurtleDestroyBlockError> {
        let payload = match side {
            JsonTurtleDirection::Forward => DESTROY_BLOCK_FRONT,
            _ => return Err(TurtleDestroyBlockError::NotImplemented)
        };

        let response = self.command(payload).await?;

        match response.as_str() {
            "true" => {
                let (x_diff, y_dif, z_diff) = side.to_turtle_move_diff(&self.database.turtle_data.rotation);
                let (x, y, z) = (self.database.turtle_data.x + x_diff, self.database.turtle_data.y + y_dif, self.database.turtle_data.z + z_diff);

                //BlockData::delete_by_xyz(connection, x, y, z)?;
                let (_, world) = self.database.world.get_fields_mut();
                let (loc, _, _, _) = TurtleWorld::get_chunk_loc_from_global_xyz(x, y, z)?;

                let chunk = world.force_get_mut_chunk_by_loc(&loc);
                chunk.update_voxel_by_global_xyz(x, y, z, |voxel| {
                    voxel.id = 0;
                    Ok(())
                })?;

                self.database.save().await?;

                return Ok(DestroyBlockResponse {
                    change: Some(WorldChange {
                        x,
                        y,
                        z,
                        action: WorldChangeAction::Delete(WorldChangeDeleteBlock()),
                    }),
                });
            }
            "false" => return Ok(DestroyBlockResponse { change: None }),
            _ => return Err(TurtleDestroyBlockError::UnexpectedResponse(response))
        }
    }

    pub async fn get_inventory(&mut self) -> Result<Vec<String>, TurtleGetInventoryError>{
        let mut res = Vec::<String>::with_capacity(16);

        for i in 1..=16 {
            let result = self.command(&format!("local item = turtle.getItemDetail({}) if (item ~= nil) then return textutils.serialiseJSON(item) else return nil end", i)).await?;
            if result == "nil" {
                continue;
            }
            let json: Value = serde_json::from_str(&result).or(Err(TurtleGetInventoryError::TurtleResponseNotJson))?;
            tracing::debug!("INV: VAL = {json:?}");

            res.push(json["name"].as_str().ok_or(TurtleGetInventoryError::TurtleResponseNotJson)?.split_once(":").ok_or(TurtleGetInventoryError::InvalidName)?.1.to_string());
        };

        Ok(res)
    }
}
