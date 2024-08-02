create table submission (
    id integer unsigned not null primary key auto_increment,
    submitted_at timestamp not null default now(),
    processed_at timestamp,

    -- prevent duplicate reports
    timestamp timestamp not null,
    latitude double not null,
    longitude double not null,
    unique (timestamp, latitude, longitude),
    
    user_agent text,
    raw text not null
);

create table cell (
    radio enum("gsm", "wcdma", "lte", "nr") not null,
    country smallint unsigned not null,
    network smallint unsigned not null,
    area smallint unsigned not null,
    cell integer unsigned not null,
    unit smallint unsigned not null,
    primary key (radio, country, network, area, cell, unit),

    min_lat double not null,
    min_lon double not null,
    max_lat double not null,
    max_lon double not null
);

create table wifi (
    mac binary(6) not null primary key,

    min_lat double not null,
    min_lon double not null,
    max_lat double not null,
    max_lon double not null
);

create table bluetooth (
    mac binary(6) not null primary key,

    min_lat double not null,
    min_lon double not null,
    max_lat double not null,
    max_lon double not null
);
