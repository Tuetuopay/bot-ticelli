create table win (
	id         uuid default uuid_generate_v4() not null,
	created_at timestamptz default now() not null,
	player_id  text not null,
	winner_id  text not null,

	constraint win_pkey primary key (id)
);
