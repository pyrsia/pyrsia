#!/usr/bin/env bash
docker compose -f installers/docker/local_code_coverage/docker-compose.yml up
docker compose -f installers/docker/local_code_coverage/docker-compose.yml down --rmi local
