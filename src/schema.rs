table! {
    win (id) {
        id -> Uuid,
        created_at -> Timestamptz,
        player_id -> Text,
        winner_id -> Text,
    }
}
