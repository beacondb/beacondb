# BeaconDB Changelog

## 0.2.0 (2026-06-07)

If you're running a private instance, this release includes algorithm changes that requires [reprocessing all reports](./docs/reprocessing.md).

- **WiFi weighted algorithm** ([#134](https://codeberg.org/beacondb/beacondb/pulls/134)): BeaconDB now uses a weighted WiFi algorithm which should be significantly more accurate than the previous bounding box model. Additionally, this algorithm won't degrade over time - the bounding box algorithm would lose accuracy with more reports, as outliers would continually add up and never be corrected. Thank you to [@yaya-cout](https://codeberg.org/Yaya-Cout)!

- **GeoIP rewrite** ([3d1762b](https://codeberg.org/beacondb/beacondb/commit/3d1762be1c91efcaf2869a54f1a48302013f3094)): BeaconDB now loads GeoIP data into memory instead of storing and querying it in Postgres. This was implemented to help improve peformance on BeaconDB's servers, and in theory should be ~109x faster. In @joelkoen's testing, the Postgres implementation could handle ~2.2k geolocate requests per second on his machine (requesting with no WiFi/AP data, random IPv4 for each request, no reverse proxy), where as the new implementation can handle around 220k rq/s.

  This means the server process will now use 130MB more memory. GeoIP can instead be disabled by not setting `geoip_path` in the server configuration.

- **bulk processing** ([59a9bcf](https://codeberg.org/beacondb/beacondb/commit/59a9bcfb7fd46ef4a2eac1303a9160a21b72b0ae)): Exported data can now be reprocessed easily with `beacondb bulk export`. More info in [./docs/exporting-data.md](./docs/exporting-data.md)

- **`/v1/country` endpoint removal** ([3d1762b](https://codeberg.org/beacondb/beacondb/commit/3d1762be1c91efcaf2869a54f1a48302013f3094)): the [`/v1/country`](https://ichnaea.readthedocs.io/en/stable/api/region.html) endpoint would estimate the country code and country name of the requesting client using its IP address. It was implemented for compatibilty with MLS, but in the past 28 days there have been a grand total of zero requests for this endpoint. It was removed as part of the GeoIP database changes for simplicity.
