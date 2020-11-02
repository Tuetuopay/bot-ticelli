use barrel::{Migration, Table, types};

/// Handle up migrations
fn up(m: &mut Migration) {
    m.create_table("game", |t: &mut Table| {
        t.inject_custom("id uuid primary key default uuid_generate_v4() not null unique");
        t.inject_custom("created_at timestamptz default now() not null");
        t.add_column("guild_id", types::text().nullable(false));
        t.add_column("channel_id", types::text().nullable(false));
        t.add_column("creator_id", types::text().nullable(false));
    });
    m.change_table("participation", |t: &mut Table| {
        t.add_column("game_id", types::uuid().nullable(true).unique(false));
        t.inject_custom(
            "add constraint participation_game_id_fkey foreign key (game_id) references game(id)"
        );
    });
}

/// Handle down migrations
fn down(m: &mut Migration) {
    m.change_table("participation", |t: &mut Table| {
        t.drop_column("game_id");
    });
    m.drop_table("game");
}
