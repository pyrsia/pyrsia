FROM ubuntu:focal

# Default Ports
EXPOSE 7888
EXPOSE 44000

# Send logging to stdout and stderr
ENV RUST_LOG=info
ENV DEBIAN_FRONTEND=noninteractive
ENV PYRSIA_BOOTDNS=boot.pyrsia.link

RUN apt-get update; \
    apt-get -y install ca-certificates wget gnupg2 jq curl dnsutils; \
    wget -O - https://pyrsia.io/install.sh | sh; 

# Need to run an entrypoint script that will determine if the docker container
# is running under Kubernetes or not.  This is done to derive the external ip address
# assigned to the service/pod
COPY installers/docker/node-entrypoint.sh /tmp/entrypoint.sh
RUN chmod 755 /tmp/entrypoint.sh

ENTRYPOINT [ "/tmp/entrypoint.sh", "--host", "0.0.0.0", "--listen", "/ip4/0.0.0.0/tcp/44000" ]
