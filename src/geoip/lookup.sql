-- geoip lookups use postgres' cidr type as a gist index
select country, latitude, longitude from geoip
where $1 <<= cidr and $1 between range_start and range_end;
