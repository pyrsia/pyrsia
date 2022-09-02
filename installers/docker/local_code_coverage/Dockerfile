FROM rust:1.63.0-buster
RUN apt-get update && apt-get -y install clang llvm libclang-dev protobuf-compiler
RUN rustup --version; \
    cargo --version; \
    rustc --version; \
    cargo install cargo-tarpaulin;
COPY . /home/pyrsia/
WORKDIR /home/pyrsia
ENTRYPOINT ["cargo", "tarpaulin", "--all-targets", "--no-fail-fast", "--workspace", "--out", "Html", "--output-dir", "/home/pyrsia/code-coverage-data"]
