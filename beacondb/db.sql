create table if not exists cell (
    radio integer not null,
    country integer not null,
    network integer not null,
    area integer not null,
    cell integer not null,
    unit integer not null,

    x real not null,
    y real not null,
    r real not null,

    primary key (radio, country, network, area, cell, unit)
);

-- used as a backup - new cell submissions are not derived from mls
create table if not exists cell_mls (
    radio integer not null,
    country integer not null,
    network integer not null,
    area integer not null,
    cell integer not null,
    unit integer not null,

    x real not null,
    y real not null,
    r real not null,

    primary key (radio, country, network, area, cell, unit)
);

create table if not exists wifi (
    bssid text not null primary key,

    x real not null,
    y real not null,
    r real not null
);
