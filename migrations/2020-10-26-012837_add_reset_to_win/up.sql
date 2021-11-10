alter table win add column reset boolean default false not null;
alter table win add column reset_at timestamptz;
alter table win add column reset_id uuid;
