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
