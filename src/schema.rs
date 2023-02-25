// @generated automatically by Diesel CLI.

diesel::table! {
    configs (key) {
        section -> Text,
        key -> Text,
        value_type -> Integer,
        value -> Nullable<Text>,
    }
}

diesel::table! {
    reservations (id) {
        id -> Integer,
        start_time -> Timestamp,
        end_time -> Timestamp,
        user_id -> BigInt,
    }
}

diesel::table! {
    users (id) {
        id -> BigInt,
        tag -> Text,
    }
}

diesel::joinable!(reservations -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    configs,
    reservations,
    users,
);
