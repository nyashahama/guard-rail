FROM rust:1.86-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

RUN cargo build --release --bin guard-rail-engine

FROM debian:bookworm-slim
WORKDIR /srv/guard-rail-engine

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system guardrail \
    && useradd --system --gid guardrail --home /srv/guard-rail-engine --create-home guardrail \
    && mkdir -p /etc/guard-rail-engine \
    && chown -R guardrail:guardrail /srv/guard-rail-engine /etc/guard-rail-engine

COPY --from=builder /app/target/release/guard-rail-engine /usr/local/bin/guard-rail-engine

USER guardrail:guardrail

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl --fail --silent http://127.0.0.1:8080/ready || exit 1

ENTRYPOINT ["guard-rail-engine"]
CMD ["serve", "--config", "/etc/guard-rail-engine/config.yaml"]
