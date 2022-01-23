#!/usr/bin/env bash

docker build --tag codecoverage -f installers/docker/DockerfileCodeCoverageTest .
docker run --rm --security-opt seccomp=unconfined -it codecoverage
