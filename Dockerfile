# Build stage
FROM rust:1.88-bookworm AS builder

WORKDIR /app

# Enable SQLx offline mode (uses .sqlx/ cached query metadata)
ENV SQLX_OFFLINE=true

COPY Cargo.toml Cargo.lock ./
COPY .sqlx .sqlx
COPY src src
COPY migrations migrations

RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/beacondb /usr/local/bin/beacondb

EXPOSE 8080

ENTRYPOINT ["beacondb"]
CMD ["serve"]
