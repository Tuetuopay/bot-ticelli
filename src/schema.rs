table! {
    participation (id) {
        id -> Uuid,
        created_at -> Timestamptz,
        player_id -> Text,
        picture -> Nullable<Bytea>,
        is_win -> Bool,
        won_at -> Nullable<Timestamptz>,
        win_id -> Nullable<Uuid>,
        is_skip -> Bool,
        skipped_at -> Nullable<Timestamptz>,
        picture_url -> Nullable<Text>,
    }
}

table! {
    win (id) {
        id -> Uuid,
        created_at -> Timestamptz,
        player_id -> Text,
        winner_id -> Text,
        reset -> Bool,
        reset_at -> Nullable<Timestamptz>,
        reset_id -> Nullable<Uuid>,
    }
}

joinable!(participation -> win (win_id));

allow_tables_to_appear_in_same_query!(
    participation,
    win,
);
