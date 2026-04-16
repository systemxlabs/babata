# syntax=docker/dockerfile:1

FROM node:24-bookworm-slim AS web-builder
WORKDIR /app/web

COPY web/package.json web/package-lock.json ./
RUN npm install

COPY web/ ./
RUN npm run build

FROM rust:1-bookworm AS rust-builder
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends git libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends bash ca-certificates git libssl3 ripgrep \
    && rm -rf /var/lib/apt/lists/*

ENV HOME=/root
ENV BABATA_SERVER_ADDR=0.0.0.0:18800

WORKDIR /app

COPY --from=rust-builder /app/target/release/babata /usr/local/bin/babata
COPY --from=web-builder /app/web/dist ./web/dist

VOLUME ["/root/.babata"]
EXPOSE 18800

CMD ["babata"]
