FROM rust:1.66.1-buster

RUN apt-get update && apt-get -y -q install clang llvm libclang-dev jq protobuf-compiler

COPY . /home/pyrsia/
WORKDIR /home/pyrsia
RUN curl -sL https://deb.nodesource.com/setup_18.x | bash -; \
    apt-get install -y -q nodejs; \
    npm i -g toml-cli; \
    rustup default $(cat Cargo.toml | toml | jq -r 'try(.package."rust-version") // "stable"'); \
    cargo install cargo-tarpaulin;

ENTRYPOINT ["cargo", "tarpaulin", "--workspace"]
