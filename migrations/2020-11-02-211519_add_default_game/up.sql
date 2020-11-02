insert into game (guild_id, channel_id, creator_id)
	values ('766814109618012180', '766814109618012183', '186607703735402496');
update participation set game_id = (select id from game limit 1);
alter table participation alter column game_id set not null;
