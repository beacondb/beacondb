create table cell (
    radio smallint not null,
    country smallint not null,
    check (country between 1 and 999),
    network smallint not null,
    check (network between 0 and 999),
    area integer not null,
    check (area between 1 and 65533),
    cell integer not null,
    check (cell > 0),
    unit smallint not null,
    check (unit between 0 and 511),
    primary key (radio, country, network, area, cell, unit),

    x double precision not null,
    y double precision not null,
    r double precision not null,
    times_seen integer not null default 1,
    times_updated integer not null default 0,
    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone not null default now()
);

create table cell_submissions (
    id bigserial not null primary key,
    submitted_at timestamp with time zone not null default now(),

    radio smallint not null,
    country smallint not null,
    check (country between 1 and 999),
    network smallint not null,
    check (network between 0 and 999),
    area integer not null,
    check (area between 1 and 65533),
    cell integer not null,
    check (cell > 0),
    unit smallint not null,
    check (unit between 0 and 511),

    x double precision not null,
    y double precision not null,
    r double precision not null
);

create table wifi (
    bssid macaddr not null primary key,
    ssid text,

    x double precision not null,
    y double precision not null,
    r double precision not null,
    times_seen integer not null default 1,
    times_updated integer not null default 0,
    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone not null default now()
);

create table wifi_submissions (
    id bigserial not null primary key,
    submitted_at timestamp with time zone not null default now(),

    bssid macaddr not null,
    ssid text,

    x double precision not null,
    y double precision not null,
    r double precision not null
);

