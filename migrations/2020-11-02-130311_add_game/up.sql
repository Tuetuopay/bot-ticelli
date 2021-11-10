create table game (
	id         uuid default uuid_generate_v4() not null,
	created_at timestamptz default now() not null,
	guild_id   text not null,
	channel_id text not null,
	creator_id text not null,

	constraint game_pkey primary key (id)
);

alter table participation add column game_id uuid;
alter table participation
	add constraint participation_game_id_fkey
	foreign key (game_id) references game(id);
