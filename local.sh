#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

#NAMADA_GENESIS_IMAGE="ghcr.io/heliaxdev/ant-namada-genesis:main" \
#NAMADA_IMAGE="ghcr.io/heliaxdev/ant-namada:main" \
#MASP_INDEXER_IMAGE_PREFIX="ghcr.io/heliaxdev/ant-masp-indexer" \
#MASP_INDEXER_IMAGE_TAG="master" \
#WORKLOAD_IMAGE="ghcr.io/heliaxdev/ant-workload:master" \
#CHECK_IMAGE="ghcr.io/heliaxdev/ant-check:latest" \
#WORKLOAD_NUM=3 \
WORKLOAD_IMAGE="local-workload:latest" \
TEST_SEED=${RANDOM} \
TEST_TIME_SEC=300 \
"${SCRIPT_DIR}/run.sh"