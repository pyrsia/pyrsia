#!/usr/bin/env bash

docker build -t code_coverage:1.0 -f code_coverage_docker .
docker run --security-opt seccomp=unconfined -it code_coverage:1.0 cargo tarpaulin -v
