#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

"${SCRIPT_DIR}/clean.sh"

docker-compose -f config/docker-compose.yml down
docker-compose -f config/docker-compose.yml up --force-recreate -d

result=$(docker wait workload)
docker compose -f config/docker-compose.yml stop

if [ "${result}" -eq 0 ]; then
  echo "==== Done successfully ===="
  exit 0
else
  echo "!!!! Failed !!!!"
  exit 1
fi