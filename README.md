# beaconDB

_A privacy focused aGPS service written in Rust._

[beaconDB](https://beacondb.net/) aims to be an alternative to Mozilla Location Services that offers public domain dumps of its WiFi database.

When [Mozilla Location Services shut down](https://github.com/mozilla/ichnaea/issues/2065), it wasn't able to publish the massive amount of access points its users had collected due to legal and privacy concerns.
`beaconDB` obfuscates the data it releases so that it is not possible to reasonably estimate the location of a single device.

Data can be contributed by using apps such as [NeoStumbler](https://github.com/mjaakko/NeoStumbler) or [TowerCollector](https://github.com/zamojski/TowerCollector)[^1].

[^1]: TowerCollector does only collect cell data.

**NOTE:** `beaconDB` is still in development!
Data exports are not ready yet.
Some data is not yet obfuscated.

## Setup

### Configuration files

In order to run `beaconDB` you need to create configuration files.
`config.toml` is read by beaconDB to configure parameters like the database address or the services port.
`.env` is setting environment variables in order configure `postgres` and `cargo` build tool.

`beaconDB` provides examples for both of these files which can be copied as a starting points.
The example files contain unsafe passwords and are not production ready.

```sh
cp .env.example .env
cp config.example.toml config.toml
```

### Dependencies

To build and run `beaconDB` the following software needs to be installed on your device.

- Rust build tool [cargo](https://www.rust-lang.org/learn/get-started)
- `sqlx-cli`
- The postgres database, access to a postgres database or podman/docker

### Database setup

`beaconDB` uses an external `PostgreSQL` database.
The user has to somehow deploy it on their own.

The easiest way to set up a database is using `Podman` or `Docker`.

```sh
podman run -p 5432:5432 --name beacondb_postgres --env-file=.env -d postgres
```

### Build

`beaconDB` is written in [Rust](https://www.rust-lang.org).
It can be build using the Rust build tool `cargo`.
Cargo creates a debug or a release build.
These can be build by running

```sh
# Debug build
cargo build

# Release build
cargo build --release
```

### Run

In order to execute the `sqlx` scripts, you need to install `sqlx-cli`.
This can be achieved by running

```sh
cargo install sqlx-cli
```

To run the server you first have to setup the database.
This can be done by running the following commands

```sh
source .env

# Create database and schema
cargo sqlx database create
cargo sqlx migrate run

# Run server in release mode
cargo run --release serve
```

The server can now be reached at `localhost:8080`.

If you are using [NeoStumbler](https://github.com/mjaakko/NeoStumbler), you can go to `Settings > Reports > Endpoint > Endpoint` and enter `http://<your-ip>:8080` and hit save.
Under `Settings > Other > Reupload data` you can reupload your collected and committed data for testing purposes.
Make sure you are definitively your test server to not reupload data to a production database.
