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
-- create index if not exists cell_x_y on cell (x, y);

-- used as a fallback, new cell submissions are not derived from mls
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
-- create index if not exists cell_mls_x_y on cell_mls (x, y);

create table if not exists wifi (
    key integer not null,

    x real not null,
    y real not null,
    r real not null
);
create index if not exists wifi_key on wifi (key);
-- create index if not exists wifi_x_y on wifi (x, y);
