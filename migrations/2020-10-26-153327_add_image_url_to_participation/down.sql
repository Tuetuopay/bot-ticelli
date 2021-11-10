alter table participation drop column picture_url;
alter table participation alter column picture set not null;
