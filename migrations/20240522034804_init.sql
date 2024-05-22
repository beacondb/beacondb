create table wifi_grid (
    bssid macaddr not null,
    ssid text not null,
    latitude integer not null,
    longitude integer not null,
    date_first_seen integer not null,
    date_last_seen integer not null,
    days_seen integer not null,
    primary key (bssid, ssid, latitude, longitude)
);

create table bluetooth_grid (
    mac macaddr not null,
    name text not null,
    latitude integer not null,
    longitude integer not null,
    date_first_seen integer not null,
    date_last_seen integer not null,
    days_seen integer not null,
    primary key (mac, name, latitude, longitude)
);
