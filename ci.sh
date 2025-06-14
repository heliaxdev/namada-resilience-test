#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

"${SCRIPT_DIR}/clean.sh"

docker compose -f config/docker-compose.ci.yml down
timestamp=$(date +%s)
docker compose -f config/docker-compose.ci.yml up --force-recreate -d

result=$(docker wait workload)
docker compose -f config/docker-compose.ci.yml stop

docker compose -f config/docker-compose.ci.yml logs --no-color > test-${timestamp}.log

summary=$(docker logs workload | sed -n '/Summary:/,$p')
if [ "${result}" -eq 0 ]; then
  echo "${summary}"
  exit 0
else
  echo "${summary}"
  exit 1
fi