FROM ubuntu:focal

# Default Ports
EXPOSE 7888
EXPOSE 44000

# Send logging to stdout and stderr
ENV RUST_LOG=info

RUN apt-get update; \
    apt-get -y install wget gnupg2; \
    wget -O - https://pyrsia.io/install.sh | sh; 

# Need to run an entrypoint script that will determine if the docker container
# is runnin under Kubernetes or not.  This is done to derive the external ip address
# assigned to the service/pod
COPY installers/docker/node-entrypoint.sh /tmp/entrypoint.sh

ENTRYPOINT [ "/tmp/entrypoint.sh", "--host", "0.0.0.0", "--listen", "/ip4/0.0.0.0/tcp/44000" ]
