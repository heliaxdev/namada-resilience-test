#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

"${SCRIPT_DIR}/clean.sh"

docker compose -f config/docker-compose.ci.yml down
timestamp=$(date +%s)
docker compose -f config/docker-compose.ci.yml up --force-recreate -d

result=$(docker wait workload)
docker compose -f config/docker-compose.ci.yml stop

docker compose -f config/docker-compose.ci.yml logs --no-color > test-${timestamp}.log

if [ "${result}" -eq 0 ]; then
  echo "==== Done successfully ===="
  exit 0
else
  echo "!!!! Failed !!!!"
  exit 1
fi