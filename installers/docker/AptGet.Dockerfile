FROM ubuntu:focal

EXPOSE 7888
EXPOSE 44000

# Send logging to stdout and stderr
ENV RUST_LOG=info

RUN apt-get update; \
    apt-get -y install wget gnupg2; \
    wget -O - https://pyrsia.io/install.sh | sh; 

ENTRYPOINT [ "/usr/local/bin/pyrsia_node" , "-L", "/ip4/0.0.0.0/tcp/44000" ]
