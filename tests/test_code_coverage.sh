#!/usr/bin/env bash

docker build --tag codecoverage -f installers/docker/CodeCoverageTest.Dockerfile .
docker run --rm --security-opt seccomp=unconfined -it codecoverage
