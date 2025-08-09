alter table bluetooth add column lat double precision not null default 0;
alter table bluetooth add column lon double precision not null default 0;
alter table bluetooth add column accuracy double precision not null default 0;
alter table bluetooth add column total_weight double precision not null default 0;
