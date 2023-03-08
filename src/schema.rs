// @generated automatically by Diesel CLI.

diesel::table! {
    binance_accounts (id) {
        id -> Integer,
        name -> Text,
        selected -> Bool,
        is_paper -> Bool,
        api_key -> Text,
        secret -> Text,
        active_clock_stub -> Nullable<Integer>,
        active_reservation -> Nullable<Integer>,
    }
}

diesel::table! {
    clock_stubs (id) {
        id -> Integer,
        start_time -> Timestamp,
        end_time -> Nullable<Timestamp>,
        user_id -> BigInt,
        last_interaction -> Timestamp,
        active_transaction -> Nullable<Integer>,
    }
}

diesel::table! {
    configs (key) {
        section -> Text,
        key -> Text,
        value_type -> Integer,
        description -> Text,
        value -> Nullable<Text>,
    }
}

diesel::table! {
    reservations (id) {
        id -> Integer,
        end_time -> Timestamp,
        alerted -> Bool,
        user_id -> BigInt,
        start_time -> Nullable<Timestamp>,
    }
}

diesel::table! {
    transactions (id) {
        id -> Integer,
        clock_stub_id -> Integer,
        buyOrderTime -> Timestamp,
        buyOrderIds -> Text,
        buyReady -> Bool,
        buyAvgPrice -> Nullable<Double>,
        sellOrderIds -> Text,
        sellReady -> Bool,
        sellAvgPrice -> Nullable<Double>,
    }
}

diesel::table! {
    users (id) {
        id -> BigInt,
        tag -> Text,
    }
}

diesel::joinable!(binance_accounts -> clock_stubs (active_clock_stub));
diesel::joinable!(binance_accounts -> reservations (active_reservation));
diesel::joinable!(clock_stubs -> users (user_id));
diesel::joinable!(reservations -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    binance_accounts,
    clock_stubs,
    configs,
    reservations,
    transactions,
    users,
);
