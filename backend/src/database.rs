use std::{env, io, ops::{Deref, DerefMut}, sync::Arc};

use diesel::{prelude::*, r2d2::{Pool, ConnectionManager, PooledConnection}};
use thiserror::Error;
use uuid::Uuid;

use crate::schema::{worlds_data, turtles, MoveDirection};

pub type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

#[derive(Error, Debug)]
#[allow(unused)]
pub enum DatabaseInitError {
    #[error("IO error")]
    IoError(#[from] io::Error),
    #[error("Invalid database file path")]
    DatabasePathError,
    #[error("r2d2 error ({0})")]
    PoolError(String)
}

#[derive(Error, Debug)]
pub enum DatabaseActionError {
    #[error("Diesel ORM error")]
    DieselError(#[from] diesel::result::Error),
    #[error("Empty connection pool")]
    EmptyConnectionPool,
    #[error("Attempted to update a field that is not in the DB (does not have valid ID)")]
    NotInsertedUpdate
}

#[derive(Queryable, Insertable, AsChangeset)]
#[diesel(table_name = turtles)]
#[derive(Debug)]
pub struct TurtleData {
    pub id: Option<i32>,
    pub uuid: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub rotation: MoveDirection,
}

#[derive(Queryable, Insertable, AsChangeset)]
#[diesel(table_name = worlds_data)]
pub struct BlockData {
    pub id: Option<i32>,
    pub turtle_id: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub name: String,
}

pub struct Connection(pub PooledConnection<ConnectionManager<SqliteConnection>>);
// For the convenience of using an &Connection as an &SqliteConnection.
impl Deref for Connection {
    type Target = SqliteConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Connection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TryFrom<Arc<SqlitePool>> for Connection {
    type Error = DatabaseActionError;
    fn try_from(value: Arc<SqlitePool>) -> Result<Self, Self::Error> {
        let pooled_conn = value.try_get().ok_or(DatabaseActionError::EmptyConnectionPool)?;
        return Ok(Self(pooled_conn))
    }
}

pub fn init() -> Result<SqlitePool, DatabaseInitError> {
    let mut path = env::current_dir()?;
    path.push("worlds.db");
    let str = path.to_str().ok_or(DatabaseInitError::DatabasePathError)?;

    let manager = ConnectionManager::<SqliteConnection>::new(str.to_owned());
    let pool = Pool::new(manager).or_else(|e| Err(DatabaseInitError::PoolError(e.to_string())))?;

    Ok(pool)
}

impl TurtleData {
    pub fn read_by_uuid(connection: &mut SqliteConnection, uuid: &Uuid) -> QueryResult<Self> {
        return turtles::table.filter(turtles::uuid.eq(uuid.to_string())).first(connection)
    }

    pub fn put(&self, connection: &mut SqliteConnection) -> Result<Self, DatabaseActionError> {
        diesel::insert_into(turtles::table)
            .values(self)
            .execute(connection)?;

        Ok(turtles::table.order(turtles::id.desc()).first(connection)?)
    }

    pub fn update(&self, connection: &mut SqliteConnection) -> Result<(), DatabaseActionError> {
        let id: i32 = self.id.ok_or(DatabaseActionError::NotInsertedUpdate)?;
        diesel::update(turtles::table.filter(turtles::id.eq(id))).set(self).execute(connection)?;
        Ok(())
    }
}

impl BlockData {
    pub fn read_by_xyz(connection: &mut SqliteConnection, x: i32, y: i32, z: i32) -> QueryResult<Self> {
        worlds_data::table.filter(worlds_data::x.eq(x).and(worlds_data::y.eq(y).and(worlds_data::z.eq(z)))).first(connection)
    }

    pub fn delete_by_xyz(connection: &mut SqliteConnection, x: i32, y: i32, z: i32) -> Result<(), DatabaseActionError> {
        diesel::delete(worlds_data::table.filter(worlds_data::x.eq(x).and(worlds_data::y.eq(y).and(worlds_data::z.eq(z))))).execute(connection)?;
        Ok(())
    }

    pub fn update(&self, connection: &mut SqliteConnection) -> Result<(), DatabaseActionError> {
        let id: i32 = self.id.ok_or(DatabaseActionError::NotInsertedUpdate)?;
        diesel::update(worlds_data::table.filter(worlds_data::id.eq(id))).set(self).execute(connection)?;
        Ok(())
    }

    pub fn insert(&self, connection: &mut SqliteConnection) -> Result<(), DatabaseActionError> {
        diesel::insert_into(worlds_data::table)
            .values(self)
            .execute(connection)?;
        Ok(())
    }

    pub fn list_by_turtle_id(connection: &mut SqliteConnection, id: i32) -> Result<Vec<BlockData>, diesel::result::Error> {
        worlds_data::table.filter(worlds_data::turtle_id.eq(id)).load(connection)
    }
}
