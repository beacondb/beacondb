alter table map add column new boolean not null default true;

create index on map (h3) where new;
