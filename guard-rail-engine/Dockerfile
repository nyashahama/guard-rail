FROM rust:1.86-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY config ./config
COPY migrations ./migrations

RUN cargo build --release --bin guard-rail-engine

FROM debian:bookworm-slim
WORKDIR /srv/guard-rail-engine

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/guard-rail-engine /usr/local/bin/guard-rail-engine
COPY config ./config

EXPOSE 8080

CMD ["guard-rail-engine", "serve", "--config", "./config/config.yaml"]
