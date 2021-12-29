#!/usr/bin/env bash

docker build --no-cache -t code_coverage -f ./tests/code_coverage.Dockerfile .
docker run --rm --security-opt seccomp=unconfined -it code_coverage
