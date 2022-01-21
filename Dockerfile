# syntax=docker/dockerfile:1.3-labs

FROM rust:1.57-buster AS builder
ENV CARGO_TARGET_DIR=/target
WORKDIR /src
RUN apt-get update && apt-get install -y \
    clang \
    libclang-dev

FROM builder AS debug
COPY . .

FROM builder AS dupdatelock
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/cache \
    --mount=type=cache,target=/usr/local/cargo/registry/index \
    cargo update

FROM scratch AS updatelock
COPY --from=dupdatelock /src/Cargo.lock .

FROM builder AS dbuild
RUN mkdir -p /out
ENV RUST_BACKTRACE=1
RUN --mount=target=/src \
    --mount=type=cache,target=/target \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/cache \
    --mount=type=cache,target=/usr/local/cargo/registry/index \
    cargo build --profile=release --package=pyrsia_node && cp /target/release/pyrsia_node /out/

FROM debian:buster-slim AS node
ENTRYPOINT ["pyrsia_node"]
ENV RUST_LOG=info
RUN <<EOT bash
    set -e
    apt-get update
    apt-get install -y \
        libssl1.1
    rm -rf /var/lib/apt/lists/*
EOT
COPY --from=dbuild /out/pyrsia_node /usr/local/bin/
