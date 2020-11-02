alter table participation alter column game_id drop not null;
update participation set game_id = null;
