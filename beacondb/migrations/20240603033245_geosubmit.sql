create table geosubmission (
    id serial primary key not null,
    submitted_at timestamp with time zone not null default now(),
    status smallint not null default 1,

    -- prevent duplicate reports
    timestamp bigint not null,
    latitude real not null,
    longitude real not null,
    unique (timestamp, latitude, longitude),
    
    user_agent text,
    raw text not null
);
