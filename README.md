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

## Usage

Please see [`config.toml`](./config.example.toml) for beaconDB's configuration options.

```sh
# serve the API on http://localhost:8080
cargo run serve
```

If you are using [NeoStumbler](https://github.com/mjaakko/NeoStumbler), you can send your data to machine's database by configuring a custom endpoint and reuploading collected data in settings.
