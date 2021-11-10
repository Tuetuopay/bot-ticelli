alter table participation add column picture_url text;
alter table participation alter column picture drop not null;
