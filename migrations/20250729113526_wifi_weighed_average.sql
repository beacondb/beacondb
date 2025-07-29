alter table wifi add column lat double precision not null default 0;
alter table wifi add column lon double precision not null default 0;
alter table wifi add column accuracy double precision not null default 0;
alter table wifi add column total_weight double precision not null default 0;
