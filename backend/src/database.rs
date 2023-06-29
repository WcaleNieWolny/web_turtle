use std::{env, num::TryFromIntError, path::PathBuf, str::Utf8Error, io::SeekFrom};

use bytes::{Bytes, BytesMut};
use once_cell::sync::Lazy;
use shared::{JsonTurtle, world_structure::TurtleWorld};
use tempfile::NamedTempFile;
use thiserror::Error;
use tokio::{fs::{File, OpenOptions}, io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt}};
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
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error("Cannot convert int types")]
    IntError(#[from] TryFromIntError),
    #[error(transparent)]
    DynamicError(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error("Data in json file is not UTF-8")]
    UtfError(#[from] Utf8Error)
}

#[derive(Debug)]
pub struct TurtleDatabase {
    world_file: File,
    json_file: File,
    raw_world_bytes: Bytes,
    pub turtle_data: JsonTurtle,
    pub world: TurtleWorld,
}

impl TurtleDatabase {
    pub async fn create_from_id(id: Uuid) -> Result<Self, DatabaseActionError> {
        let mut path = DATA_DIR.clone();
        path.push(id.simple().to_string());

        let json_file_path = path.with_extension("json");
        let world_file_path = path.with_extension("world");
     
        let mut json_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(json_file_path)
            .await?;
       
        let json_len = json_file.metadata().await?.len();

        let json_turtle: JsonTurtle = if json_len == 0 {
            println!("New turtle!");
            JsonTurtle {
                uuid: id.clone(),
                x: 0,
                y: 0,
                z: 0,
                rotation: shared::JsonTurtleDirection::Forward,
            }
        } else {
            let mut bytes = Vec::new();
            json_file.read_to_end(&mut bytes).await?;
            println!("LL: {bytes:x?}");
            println!("??: {}", std::str::from_utf8(&bytes).unwrap());
            serde_json::from_slice(&bytes)?
        };

        let mut world_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(world_file_path)
            .await?;

        let world_len = world_file.metadata().await?.len();
        let (turtle_world, turtle_bytes) = if world_len == 0 {
            (TurtleWorld::new(), Bytes::new())
        } else {
            let mut bytes = Vec::with_capacity(world_len.try_into()?);
            world_file.read_to_end(&mut bytes).await?;
            let bytes: Bytes = bytes.into();
            (TurtleWorld::from_bytes(bytes.clone())?, bytes)
        };

        let mut database = Self {
            world_file,
            json_file,
            raw_world_bytes: turtle_bytes,
            turtle_data: json_turtle,
            world: turtle_world
        };

        if json_len == 0 && world_len == 0 {
            database.save().await?;
        }

        Ok(database)
    }

    pub async fn save(&mut self) -> Result<(), DatabaseActionError> {
        let json_str = serde_json::to_vec(&self.turtle_data)?;
        let mut world_bytes = self.world.to_bytes()?;

        self.raw_world_bytes = world_bytes.clone();

        let named_tmp_world_file = NamedTempFile::new_in(DATA_DIR.clone())?;
        let named_tmp_json_file = NamedTempFile::new_in(DATA_DIR.clone())?;
        let (named_tmp_json_handle, named_tmp_json_path) = named_tmp_json_file.into_parts(); 
        let (named_tmp_world_handle, named_tmp_world_path) = named_tmp_world_file.into_parts(); 

        let mut tmp_world_file = File::from_std(named_tmp_world_handle);
        let mut tmp_json_file = File::from_std(named_tmp_json_handle);

        tmp_world_file.write_all(&mut world_bytes).await?;
        tmp_json_file.write_all(&json_str).await?;

        tmp_world_file.flush().await?;
        tmp_json_file.flush().await?;

        let mut real_path = DATA_DIR.clone();
        real_path.push(self.turtle_data.uuid.simple().to_string());

        tokio::fs::rename(named_tmp_json_path, real_path.with_extension("json")).await?;
        tokio::fs::rename(named_tmp_world_path, real_path.with_extension("world")).await?;

        Ok(())
    }

    pub fn raw_world(&self) -> Bytes {
        self.raw_world_bytes.clone()
    }
}
