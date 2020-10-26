use barrel::{Migration, Table, types};

/// Handle up migrations
fn up(m: &mut Migration) {
    m.change_table("participation", |t: &mut Table| {
        t.add_column("picture_url", types::text().nullable(true));
        t.inject_custom("alter column picture drop not null");
    });
}

/// Handle down migrations
fn down(m: &mut Migration) {
    m.change_table("participation", |t: &mut Table| {
        t.drop_column("picture_url");
        t.inject_custom("alter column picture set not null");
    })
}
