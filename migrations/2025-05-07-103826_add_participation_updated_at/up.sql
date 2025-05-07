alter table participation add column updated_at timestamptz;
update participation set updated_at = created_at;
alter table participation
	alter column updated_at set not null,
	alter column updated_at set default now();
