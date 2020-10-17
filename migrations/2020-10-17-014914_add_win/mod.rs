use barrel::{Migration, types};

/// Handle up migrations
fn up(m: &mut Migration) {
    m.create_table("win", |t| {
        t.inject_custom("id UUID PRIMARY KEY DEFAULT uuid_generate_v4() NOT NULL UNIQUE");
        t.inject_custom("created_at timestamptz default now() not null");
        t.add_column(
            "player_id",
            types::text().indexed(true).nullable(false),
        );
        t.add_column(
            "winner_id",
            types::text().indexed(true).nullable(false),
        );
    });
}

/// Handle down migrations
fn down(m: &mut Migration) {
    m.drop_table("win")
}
