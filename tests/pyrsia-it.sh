#!/bin/bash -i

alias dockertls="docker --tls --tlskey /certs/client/key.pem --tlscacert /certs/client/ca.pem --tlscert /certs/client/cert.pem"

echo "Pulling alpine:3.15.4 on Pyrsia node 1, fetching it from docker.io"
ln -sfT /certs/client1 /certs/client
export DOCKER_HOST=tcp://docker1:2376
dockertls images
dockertls pull alpine:3.15.4
dockertls images

echo "Pulling alpine:3.15.4 on Pyrsia node 2, fetching it from Pyrsia node 1"
ln -sfT /certs/client2 /certs/client
export DOCKER_HOST=tcp://docker2:2376
dockertls images
dockertls pull alpine:3.15.4
dockertls images
