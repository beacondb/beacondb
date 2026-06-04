# BeaconDB Changelog

## 0.2.0 (unreleased)

- **GeoIP rewrite**: BeaconDB now loads GeoIP data into memory instead of storing and querying it in Postgres. This was implemented to help improve peformance on BeaconDB's servers, and in theory should be ~109x faster. In @joelkoen's testing, the Postgres implementation could handle ~2.2k geolocate requests per second on his machine (requesting with no WiFi/AP data, random IPv4 for each request, no reverse proxy), where as the new implementation can handle around 220k rq/s.

  This means the server process will now use 130MB more memory. GeoIP can instead be disabled by not setting `geoip_path` in the server configuration.

- **`/v1/country` endpoint removal**: the [`/v1/country`](https://ichnaea.readthedocs.io/en/stable/api/region.html) endpoint would estimate the country code and country name of the requesting client using its IP address. It was implemented for compatibilty with MLS, but in the past 28 days there have been a grand total of zero requests for this endpoint. It was removed as part of the GeoIP database changes for simplicity.
