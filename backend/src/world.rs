use std::mem::ManuallyDrop;

use serde::{Serialize, Deserialize};
use serde_json::{Value, json};

#[derive(Serialize)]
pub enum WorldChangeAction {
    New(WorldChangeNewBlock),
    Update(WorldChangeUpdateBlock),
    Delete(WorldChangeDeleteBlock)
}

#[derive(Serialize)]
pub struct WorldChange {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub action: WorldChangeAction,
}

#[derive(Deserialize)]
pub struct TurtleBlock {
    pub name: String
}

#[derive(Serialize)]
pub struct WorldChangeNewBlock {
    pub color: String
}

//This might change in the future
#[derive(Serialize)]
pub struct WorldChangeUpdateBlock {
    pub color: String
}

#[derive(Serialize)]
pub struct WorldChangeDeleteBlock();

impl ToString for WorldChangeAction {
    fn to_string(&self) -> String {
        return match self {
            WorldChangeAction::New(_) => "new", 
            WorldChangeAction::Update(_) => "update",
            WorldChangeAction::Delete(_) => "delete",
        }.to_string()
    }
}

/// Converts material string into hex string of color
pub fn block_color(material: &str) -> String {
    let hash = seahash::hash(material.as_bytes());
    let hash: [u8; 8] = hash.to_le_bytes();
    
    return format!("#{:02x}{:02x}{:02x}", hash[0], hash[4], hash[7]);
}
