create table geoip (
    cidr cidr not null,
    range_start inet not null,
    range_end inet not null,
    country char(2) not null,
    latitude double precision not null,
    longitude double precision not null
);

create index geoip_range on geoip using gist (cidr inet_ops);
