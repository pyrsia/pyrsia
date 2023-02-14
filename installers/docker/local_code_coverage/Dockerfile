FROM rust:1.66.1-buster
ARG USER
ARG USER_GRP
RUN apt-get update && apt-get -y install clang llvm libclang-dev jq protobuf-compiler
RUN groupadd -r ${USER_GRP}; \
    useradd -r -m -g ${USER_GRP} -s /bin/bash ${USER};
COPY --chown=${USER}:${USER_GRP} . ${HOME}/pyrsia/
WORKDIR ${HOME}/pyrsia
USER ${USER}
RUN curl -sL https://deb.nodesource.com/setup_18.x | bash -; \
    apt-get install -y -q nodejs; \
    npm i -g toml-cli; \
    rustup default $(cat Cargo.toml | toml | jq -r 'try(.package."rust-version") // "stable"'); \
    cargo install cargo-tarpaulin; \
    chmod 544  installers/docker/local_code_coverage/entrypoint.sh;
ENTRYPOINT ["non-existing.sh"]
