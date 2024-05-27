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

    x real not null,
    y real not null,
    r real not null,
    t smallint not null default 0,
    times_seen integer not null default 1,
    times_updated integer not null default 0,
    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone not null default now()
);

create table wifi (
    bssid macaddr not null primary key,
    ssid text,
    band smallint not null,

    x real not null,
    y real not null,
    r real not null,
    t smallint not null default 0,
    times_seen integer not null default 1,
    times_updated integer not null default 0,
    created_at timestamp with time zone not null default now(),
    updated_at timestamp with time zone not null default now()
);
