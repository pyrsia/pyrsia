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
ENV DEV_MODE=on
ENV PYRSIA_ARTIFACT_PATH=pyrsia
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
        ca-certificates
    rm -rf /var/lib/apt/lists/*
EOT
COPY --from=dbuild /out/pyrsia_node /usr/local/bin/

FROM debian:buster-slim AS node-it
ARG P2P_KEYPAIR
ENTRYPOINT ["pyrsia_node"]
ENV RUST_LOG=info
RUN <<EOT bash
    set -e
    apt-get update
    apt-get install -y \
        ca-certificates
    rm -rf /var/lib/apt/lists/*
    mkdir /pyrsia
EOT
COPY tests/${P2P_KEYPAIR} /pyrsia/p2p_keypair.ser
COPY --from=dbuild /out/pyrsia_node /usr/local/bin/

FROM debian:buster-slim AS it-test
RUN apt update && apt install -y apt-transport-https ca-certificates curl gnupg2 software-properties-common
RUN curl -fsSL https://download.docker.com/linux/debian/gpg | gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg
RUN echo "deb [arch=amd64 signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/debian $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list
RUN apt update && apt install -y docker-ce docker-ce-cli containerd.io
COPY tests/pyrsia-it.sh /pyrsia-it.sh
CMD [ "./pyrsia-it.sh" ]
