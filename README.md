# beaconDB

_A privacy focused assisted GPS service written in Rust._

[beaconDB](https://beacondb.net/) aims to be an alternative to Mozilla Location Services that offers public domain dumps of its WiFi database.

When [Mozilla Location Services shut down](https://github.com/mozilla/ichnaea/issues/2065), it wasn't able to publish the massive amount of WiFi APs its users had collected due to legal and privacy concerns. beaconDB plans to obfuscate the data it releases to mitigate these issues.

For information on how to use and contribute to beaconDB, please see [beaconDB's website].

Please note that beaconDB is experimental and under active development. Data exports are not ready yet. Some data is not yet obfuscated.

## Development

To compile beaconDB, you'll need the following on your system:

- [Rust](https://www.rust-lang.org/)
- [SQLx](https://github.com/launchbadge/sqlx) (`cargo install sqlx-cli`)
- [PostgreSQL](https://www.postgresql.org/)

beaconDB relies on SQLx to manage database migrations and provide compile-time type checking. SQLx uses the `DATABASE_URL` environment variable to connect to Postgres. If your account is configured as a superuser, the configuration in [`.env.example`](./.env.example) should work as-is. Otherwise, please see [SQLx's Postgres connection options](https://docs.rs/sqlx/latest/sqlx/postgres/struct.PgConnectOptions.html) for more information.

```sh
cp .env.example .env

cargo sqlx database create
cargo sqlx migrate run
```

## Docker

```sh
cp config.docker.toml config.toml
docker compose up -d
```

This starts PostgreSQL and beaconDB on `http://localhost:8080`. Migrations run automatically on startup.

## Usage

Please see [`config.toml`](./config.example.toml) for beaconDB's configuration options.

```sh
# serve the API on http://localhost:8080
cargo run serve
```

### Example

Submit WiFi observations from two slightly different positions, then query the location:

```sh
# submit two reports
curl -s -X POST http://localhost:8080/v2/geosubmit -H 'Content-Type: application/json' -H 'User-Agent: test' -d '{"items":[{"timestamp":1711700001000,"position":{"latitude":48.8566,"longitude":2.3522,"accuracy":10},"wifiAccessPoints":[{"macAddress":"AA:BB:CC:DD:EE:01","ssid":"TestWifi1","signalStrength":-60},{"macAddress":"AA:BB:CC:DD:EE:02","ssid":"TestWifi2","signalStrength":-70}]}]}'

curl -s -X POST http://localhost:8080/v2/geosubmit -H 'Content-Type: application/json' -H 'User-Agent: test' -d '{"items":[{"timestamp":1711700002000,"position":{"latitude":48.8568,"longitude":2.3525,"accuracy":10},"wifiAccessPoints":[{"macAddress":"AA:BB:CC:DD:EE:01","ssid":"TestWifi1","signalStrength":-55},{"macAddress":"AA:BB:CC:DD:EE:02","ssid":"TestWifi2","signalStrength":-75}]}]}'

# process the submitted reports
beacondb process

# query the location based on nearby WiFi networks
curl -s -X POST http://localhost:8080/v1/geolocate -H 'Content-Type: application/json' -d '{"wifiAccessPoints":[{"macAddress":"AA:BB:CC:DD:EE:01","signalStrength":-60},{"macAddress":"AA:BB:CC:DD:EE:02","signalStrength":-70}]}'
# {"location":{"lat":48.8567,"lng":2.35235},"accuracy":50}
```

If you are using [NeoStumbler](https://github.com/mjaakko/NeoStumbler), you can send your data to machine's database by configuring a custom endpoint and reuploading collected data in settings.
