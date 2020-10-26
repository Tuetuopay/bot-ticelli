use barrel::{Migration, Table, types};

/// Handle up migrations
fn up(m: &mut Migration) {
    m.create_table("participation", |t| {
        t.inject_custom("id UUID PRIMARY KEY DEFAULT uuid_generate_v4() NOT NULL UNIQUE");
        t.inject_custom("created_at timestamptz default now() not null");
        t.add_column(
            "player_id",
            types::text().indexed(true).nullable(false),
        );
        t.add_column("picture", types::binary().nullable(false));
        t.add_column("is_win", types::boolean().nullable(false).default(false));
        t.add_column("won_at", types::custom("timestamptz").nullable(true));
        t.add_column("win_id", types::uuid().nullable(true));
        t.add_column("is_skip", types::boolean().nullable(false).default(false));
        t.add_column("skipped_at", types::custom("timestamptz").nullable(true));
        t.inject_custom("constraint participation_win_id_fkey foreign key (win_id) references win(id)");
    });
}

/// Handle down migrations
fn down(m: &mut Migration) {
    m.drop_table("participation");
}
