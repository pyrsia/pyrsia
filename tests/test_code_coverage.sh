#!/usr/bin/env bash

docker build -t code_coverage:1.0 -f code_coverage_docker .
docker run --rm --security-opt seccomp=unconfined -it code_coverage:1.0
