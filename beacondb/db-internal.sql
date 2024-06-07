create table if not exists cell (
    radio integer not null,
    country integer not null,
    network integer not null,
    area integer not null,
    cell integer not null,
    unit integer not null,
    primary key (radio, country, network, area, cell, unit),

    x real not null,
    y real not null,
    r real not null,

    first_seen integer not null default unixepoch(),
    last_seen integer not null default unixepoch(),
    days_seen integer not null default 1,
    last_changed integer not null default unixepoch(),
    times_changed integer not null default 0,
    first_blocked integer,
    last_blocked integer,
    times_blocked not null default 0
);

create table if not exists wifi (
    key integer not null,
    secret integer not null,
    primary key (key, secret),

    x real not null,
    y real not null,
    r real not null,

    first_seen integer not null default unixepoch(),
    last_seen integer not null default unixepoch(),
    days_seen integer not null default 1,
    last_changed integer not null default unixepoch(),
    days_changed integer not null default 0,
    first_blocked integer,
    last_blocked integer,
    times_blocked not null default 0
);
