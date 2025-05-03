alter table win add column score integer not null default 1;
alter table win alter column score drop default;

alter table participation add column warned_at timestamptz;
