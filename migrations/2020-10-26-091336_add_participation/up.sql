create table participation (
	id         uuid default uuid_generate_v4() not null,
	created_at timestamptz default now() not null,
	player_id  text not null,
	picture    bytea not null,
	is_win     boolean default false not null,
	won_at     timestamptz,
	win_id     uuid,
	is_skip    boolean default false not null,
	skipped_at timestamptz,

	constraint participation_pkey primary key (id),
	constraint participation_win_id_fkey foreign key (win_id) references win(id),
	constraint participation_win_id_key unique (win_id)
);
