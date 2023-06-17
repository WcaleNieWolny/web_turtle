use std::{path::{PathBuf}, env};

use once_cell::sync::Lazy;
use thiserror::Error;
use tokio::fs::File;
use uuid::Uuid;

//this is allowed to panic, if it ever fails all of our code is usless
//this also can block but it is the best way to handle this
static DATA_DIR: Lazy<PathBuf> = Lazy::new(|| {
    let mut current_dir = env::current_dir().expect("Cannot get current_dir");
    current_dir.push("turtle_database");
    
    if !current_dir.try_exists().expect("Checking if database dir exist failed") {
        std::fs::create_dir_all(&current_dir).expect("Cannot create database dir");
    };
    current_dir
});

#[derive(Error, Debug)]
pub enum DatabaseActionError {
    #[error(transparent)]
    IoError(#[from] std::io::Error)
}

#[derive(Debug)]
pub struct TurtleDatabase {
    pub world_file: File,
    pub json_file: File
}

impl TurtleDatabase {
    pub fn new_from_id(id: Uuid) -> Result<Self, DatabaseActionError> {
        let mut path = DATA_DIR.clone();

        let new_file = path.push(id.simple().to_string());
        let json_file_path = path.with_extension("json");

        println!("Path: {:?}", json_file_path);
        unimplemented!()
    }
}
