create table cell (
    radio integer not null,
    country integer not null,
    network integer not null,
    area integer not null,
    cell integer not null,
    unit integer not null,

    lon real not null,
    lat real not null,
    range real not null,
    samples integer not null,
    created integer not null,
    updated integer not null,

    primary key (radio, country, network, area, cell, unit)
);
