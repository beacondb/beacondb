create table report (
    id serial not null primary key,
    submitted_at timestamp with time zone not null default now(),
    processed_at timestamp with time zone,

    -- prevent duplicate reports
    timestamp timestamp with time zone not null,
    latitude double precision not null,
    longitude double precision not null,
    unique (timestamp, latitude, longitude),
    
    user_agent text,
    raw bytea not null
);

create index report_processed_at on report (processed_at);

create table cell (
    radio smallint not null,
    country smallint not null,
    network smallint not null,
    area integer not null,
    cell bigint not null,
    unit smallint not null,
    primary key (radio, country, network, area, cell, unit),

    min_lat double precision not null,
    min_lon double precision not null,
    max_lat double precision not null,
    max_lon double precision not null
);

create table wifi (
    mac macaddr not null primary key,

    min_lat double precision not null,
    min_lon double precision not null,
    max_lat double precision not null,
    max_lon double precision not null
);

create table bluetooth (
    mac macaddr not null primary key,

    min_lat double precision not null,
    min_lon double precision not null,
    max_lat double precision not null,
    max_lon double precision not null
);

create table mls_cell (
    radio smallint not null,
    country smallint not null,
    network smallint not null,
    area integer not null,
    cell bigint not null,
    unit smallint not null,
    primary key (radio, country, network, area, cell, unit),

    lat double precision not null,
    lon double precision not null,
    radius double precision not null
);
