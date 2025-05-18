[![backend](https://github.com/mkornaukhov03/rurl-shortener/actions/workflows/backend.yaml/badge.svg?branch=master)](https://github.com/mkornaukhov03/rurl-shortener/actions/workflows/backend.yaml)

# Rust URL shortener

## How to run
Just
```bash
docker compose up
```

There is a feature: AI-generating short links. We use `openrouter` with `llama` model. You may provide openrouter token via env, see `docker-compose.yaml`.

## Usage
By default, it deploys frontend on `4444` port:
![Frontend example](images/front_example.png)

Grafana is deployed on port `3000`:
![Grafana](images/grafana_example.png)

## General Architecture
TODO

## Telemetry Architecture
TODO
