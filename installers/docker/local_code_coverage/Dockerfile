FROM rust:1.63.0-buster
ARG USER
ARG USER_GRP
RUN apt-get update && apt-get -y install clang llvm libclang-dev protobuf-compiler
RUN groupadd -r ${USER_GRP}; \
    useradd -r -m -g ${USER_GRP} -s /bin/bash ${USER};
USER ${USER}
RUN rustup --version; \
    cargo --version; \
    rustc --version; \
    cargo install cargo-tarpaulin;
COPY --chown=${USER}:${USER_GRP} . ${HOME}/pyrsia/
WORKDIR ${HOME}/pyrsia
ENTRYPOINT ["cargo", "tarpaulin", "--all-targets", "--no-fail-fast", "--workspace", "--out", "Html", "--output-dir", "/home/pyrsiaapp/pyrsia/code-coverage-data"]
