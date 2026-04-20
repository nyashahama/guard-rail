# Guard Rail Observability

This directory contains repo-native observability artifacts for the Guard Rail runtime.

## Metrics Endpoint

The runtime exposes Prometheus metrics at `/metrics` on the main listener when `observability.metrics_enabled` is `true` in the config.

Example scrape config:

```yaml
- job_name: 'guard-rail'
  static_configs:
    - targets: ['guard-rail:8080']
  metrics_path: '/metrics'
```

## Alert Rules

Load `prometheus-alerts.yml` into your Prometheus configuration. See the alert rules for operational thresholds.

## Dashboard

Import `grafana-dashboard.json` into Grafana for a pre-built operational dashboard.

## Runbooks

See `RUNBOOKS.md` for operational procedures.