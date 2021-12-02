FROM rust:buster

RUN rustup --version; \
    cargo --version; \
    rustc --version; \
    cargo install cargo-tarpaulin;
COPY pyrsia_node/ pyrsia
WORKDIR pyrsia
CMD ["cargo", "tarpaulin", "-v"]
