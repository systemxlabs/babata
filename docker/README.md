# Docker

This directory contains the container build and Compose setup for Babata.

## Build

From the repository root:

```bash
docker build -f docker/Dockerfile -t babata .
```

## Run

From the repository root:

```bash
docker run --rm -p 18800:18800 -v "$HOME/.babata:/home/babata/.babata" babata
```

The container defaults to `BABATA_SERVER_ADDR=0.0.0.0:18800`, so the Web UI is available at `http://127.0.0.1:18800` on the host after publishing the port.

Persistent state is stored under `/home/babata/.babata` inside the container. If you want to keep tasks, agents, providers, and channel configuration across restarts, bind-mount your local `$HOME/.babata` directory there.

## Docker Compose

From the repository root:

```bash
docker compose -f docker/compose.yaml up --build
```

Run it in the background:

```bash
docker compose -f docker/compose.yaml up --build -d
```

Stop the stack:

```bash
docker compose -f docker/compose.yaml down
```

The Compose setup bind-mounts `${HOME}/.babata` to `/home/babata/.babata` and publishes the Web UI on `http://127.0.0.1:18800`.

This setup assumes a Unix-like host with `HOME` available. Windows-specific path handling is intentionally not covered here.
