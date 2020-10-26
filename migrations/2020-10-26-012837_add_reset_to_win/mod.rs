use barrel::{Migration, Table, types};

/// Handle up migrations
fn up(m: &mut Migration) {
    m.change_table("win", |t| {
        t.add_column("reset", types::boolean().default(false).nullable(false).indexed(true));
        t.inject_custom("add column reset_at timestamptz");
        t.add_column("reset_id", types::uuid().nullable(true).unique(false));
    })
}

/// Handle down migrations
fn down(m: &mut Migration) {
    m.change_table("win", |t| {
        t.drop_column("reset");
        t.drop_column("reset_at");
        t.drop_column("reset_id");
    })
}
