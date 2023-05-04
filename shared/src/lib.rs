use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub enum JsonTurtleRotation {
    Forward,
    Right,
    Backward,
    Left
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonTurtle {
    pub id: usize,
    pub uuid: Uuid, 
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub rotation: JsonTurtleRotation
}
