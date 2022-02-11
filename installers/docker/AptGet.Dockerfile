FROM ubuntu:focal

EXPOSE 7888

# Send logging to stdout and stderr
ENV RUST_LOG=info

RUN apt-get update; \
    apt-get -y install wget; \
    wget -O - https://pyrsia.io/install.sh | sh; 

ENTRYPOINT [ "/usr/local/bin/pyrsia_node" ]
