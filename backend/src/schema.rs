// @generated automatically by Diesel CLI.
#[derive(diesel_derive_enum::DbEnum, Debug, Clone, Copy)]
pub enum MoveDirection {
    Forward,
    Right,
    Backward,
    Left
}

diesel::table! {
    use super::MoveDirectionMapping;
    use diesel::sql_types::{Integer, Nullable, Text};
    turtles (id) {
        id -> Nullable<Integer>,
        uuid -> Text,
        x -> Integer,
        y -> Integer,
        z -> Integer,
        rotation -> MoveDirectionMapping,
    }
}

diesel::table! {
    worlds_data (id) {
        id -> Nullable<Integer>,
        turtle_id -> Integer,
        x -> Integer,
        y -> Integer,
        z -> Integer,
        chunk_x -> Integer,
        chunk_y -> Integer,
        chunk_z -> Integer,
        name -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    turtles,
    worlds_data,
);
