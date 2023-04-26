// @generated automatically by Diesel CLI.

diesel::table! {
    turtles (id) {
        id -> Nullable<Integer>,
        uuid -> Text,
        x -> Integer,
        y -> Integer,
        z -> Integer,
        rotation -> Integer,
    }
}

diesel::table! {
    worlds_data (id) {
        id -> Nullable<Integer>,
        turtle_id -> Integer,
        x -> Integer,
        y -> Integer,
        z -> Integer,
        data -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    turtles,
    worlds_data,
);
