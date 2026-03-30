# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS build
WORKDIR /src

# System deps for sqlx sqlite (non-bundled)
RUN apt-get update && apt-get install -y --no-install-recommends \
  libsqlite3-dev pkg-config \
  && rm -rf /var/lib/apt/lists/*

# Cache deps
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && printf "fn main(){}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Build real binary
COPY . .
RUN cargo build --release

# Small runtime image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
  ca-certificates libsqlite3-0 \
  && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /src/target/release/pomo-tui /app/pomo-tui

# HTTP API port
EXPOSE 1881
# TCP port
EXPOSE 1880

# Run server-only, bind to all interfaces
ENTRYPOINT ["/app/pomo-tui","--server","--tcp-addr","0.0.0.0:1880","--http-addr","0.0.0.0:1881"]
