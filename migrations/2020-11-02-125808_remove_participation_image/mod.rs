use barrel::{Migration, Table, types};

/// Handle up migrations
fn up(m: &mut Migration) {
    m.change_table("participation", |t: &mut Table| {
        t.drop_column("picture");
    });
}

/// Handle down migrations
fn down(m: &mut Migration) {
    m.change_table("participation", |t: &mut Table| {
        t.add_column("picture", types::binary().nullable(true));
    });
}
