use std::str::FromStr;

use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum JsonTurtleRotation {
    Forward,
    Right,
    Backward,
    Left
}

impl ToString for JsonTurtleRotation {
    fn to_string(&self) -> String {
        match self {
            JsonTurtleRotation::Forward => "forward".to_string(),
            JsonTurtleRotation::Backward => "backward".to_string(),
            JsonTurtleRotation::Left => "left".to_string(),
            JsonTurtleRotation::Right => "right".to_string(),
        }
    }
}

impl FromStr for JsonTurtleRotation {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        return Ok(match s {
            "forward" => JsonTurtleRotation::Forward,
            "backward" => JsonTurtleRotation::Backward,
            "left" => JsonTurtleRotation::Left,
            "right" => JsonTurtleRotation::Right,
            _ => return Err(())
        })
    }
}

impl JsonTurtleRotation {
    /// # Returns
    /// A tuple (x, y, z)
    /// # Safety 
    /// NEVER CALL THIS FUNCTION WITH MoveDirection::Left OR MoveDirection:Right
    pub fn to_turtle_move_diff(&self, turtle_rotation: &JsonTurtleRotation) -> (i32, i32, i32) {
        match turtle_rotation {
            JsonTurtleRotation::Right => {
                match self {
                    JsonTurtleRotation::Forward => (1, 0, 0),
                    JsonTurtleRotation::Backward => (-1, 0, 0),
                    _ => unreachable!()
                }
            },
            JsonTurtleRotation::Left => {
                match self {
                    JsonTurtleRotation::Forward => (-1, 0, 0),
                    JsonTurtleRotation::Backward => (1, 0, 0),
                    _ => unreachable!()
                }
            },
            JsonTurtleRotation::Forward => {
                match self {
                    JsonTurtleRotation::Forward => (0, 0, -1),
                    JsonTurtleRotation::Backward => (0, 0, 1),
                    _ => unreachable!()
                }
            },
            JsonTurtleRotation::Backward => {
                match self {
                    JsonTurtleRotation::Forward => (0, 0, 1),
                    JsonTurtleRotation::Backward => (0, 0, -1),
                    _ => unreachable!()
                }
            },
        }
    }

    fn from_i32(number: i32) -> Self {
        match number {
            0 => Self::Forward,
            1 => Self::Right,
            2 => Self::Backward,
            3 => Self::Left,
            _ => panic!("Invalid i32 number to MoveDirection, this should NEVER happen")
        }
    }
    fn to_i32(&self) -> i32 {
        match &self {
            Self::Forward => 0,
            Self::Right => 1,
            Self::Backward => 2,
            Self::Left => 3,
        }
    }

    pub fn rotate_self(&mut self, rotation: &JsonTurtleRotation) {
        let mut enum_number = self.to_i32();
        match rotation {
            JsonTurtleRotation::Right => {
                if enum_number == 3 {
                    enum_number = 0
                } else {
                    enum_number += 1
                }
            },
            JsonTurtleRotation::Left => {
                if enum_number == 0 {
                    enum_number = 3
                } else {
                    enum_number -= 1
                }
            },
            _ => panic!("Invalid rotation")
        };
        *self = Self::from_i32(enum_number)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct JsonTurtle {
    pub id: usize,
    pub uuid: Uuid, 
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub rotation: JsonTurtleRotation
}

#[derive(Deserialize)]
pub struct TurtleBlock {
    pub name: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorldBlock {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TurtleWorld {
    pub blocks: Vec<WorldBlock>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum WorldChangeAction {
    New(WorldChangeNewBlock),
    Update(WorldChangeUpdateBlock),
    Delete(WorldChangeDeleteBlock)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorldChange {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub action: WorldChangeAction,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorldChangeNewBlock {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

//This might change in the future
#[derive(Serialize, Deserialize, Debug)]
pub struct WorldChangeUpdateBlock {
    pub color: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorldChangeDeleteBlock();

#[derive(Serialize, Deserialize, Debug)]
pub struct TurtleMoveResponse {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub rotation: JsonTurtleRotation,
    pub changes: Vec<WorldChange>
}
