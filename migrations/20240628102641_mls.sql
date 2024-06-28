create table mls_cell (
    radio enum("gsm", "wcdma", "lte", "nr") not null,
    country smallint unsigned not null,
    network smallint unsigned not null,
    area smallint unsigned not null,
    cell integer unsigned not null,
    unit smallint unsigned not null,
    primary key (radio, country, network, area, cell, unit),

    lat double not null,
    lon double not null,
    radius double not null
);
