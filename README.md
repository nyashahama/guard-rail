# Guard Rail

`guard-rail-engine` is the Rust runtime in this repo. Stage 5 adds production-hardening support for readiness, metrics, trace-context-aware request logging, graceful drain on shutdown, and baseline deployment artifacts.

## Guard Rail Engine Operations

Run migrations:

```bash
cd guard-rail-engine
cargo run -- migrate --config ./config/config.yaml
```

Serve locally:

```bash
cd guard-rail-engine
cargo run -- serve --config ./config/config.yaml
```

Build the container image:

```bash
cd guard-rail-engine
docker build -t guard-rail-engine .
```

Install the systemd unit:

```bash
sudo cp guard-rail-engine/deploy/systemd/guard-rail-engine.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now guard-rail-engine
```

Operational endpoints:
- `GET /health`
- `GET /ready`
- `GET /metrics`

Container and service artifacts:
- `guard-rail-engine/Dockerfile`
- `guard-rail-engine/.dockerignore`
- `guard-rail-engine/deploy/systemd/guard-rail-engine.service`
