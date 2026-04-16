# Docker

This directory contains the container build and Compose setup for Babata.

## Build image

From the repository root:

```bash
docker build --build-arg http_proxy=http://<ip>:<port> --build-arg https_proxy=http://<ip>:<port> -f docker/Dockerfile -t babata:latest .
```

## Compose

From the repository root:

```bash
docker compose -f docker/compose.yaml up -d
docker compose -f docker/compose.yaml down
```

The Compose setup bind-mounts `${HOME}/.babata` to `/home/babata/.babata` and publishes the Web UI on `http://127.0.0.1:18800`.

This setup assumes a Unix-like host with `HOME` available. Windows-specific path handling is intentionally not covered here.

## FAQ

1.Docker compose setup failed with permission denied error liki `Failed to build file appender for logger: failed to open current log, source: Permission denied (os error 13)`
```bash
chmod -R a+rwX ~/.babata
```