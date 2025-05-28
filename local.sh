rm -rf config/validator-0
rm -rf config/validator-1
rm -rf config/validator-2
rm -rf config/fullnode
rm -rf config/gaia-0
rm -rf config/container_ready

mkdir -p config/validator-0
mkdir -p config/validator-1
mkdir -p config/validator-2
mkdir -p config/fullnode
mkdir -p config/gaia-0
mkdir -p config/container_ready

touch config/validator-0/DO_NOT_REMOVE
touch config/validator-1/DO_NOT_REMOVE
touch config/validator-2/DO_NOT_REMOVE
touch config/fullnode/DO_NOT_REMOVE
touch config/gaia-0/DO_NOT_REMOVE
touch config/container_ready/DO_NOT_REMOVE

docker-compose -f config/docker-compose.yml down

#NAMADA_GENESIS_IMAGE="ghcr.io/heliaxdev/ant-namada-genesis:main" \
#NAMADA_IMAGE="ghcr.io/heliaxdev/ant-namada:main" \
#MASP_INDEXER_IMAGE_PREFIX="ghcr.io/heliaxdev/ant-masp-indexer" \
#MASP_INDEXER_IMAGE_TAG="master" \
#WORKLOAD_IMAGE="ghcr.io/heliaxdev/ant-workload:master" \
#CHECK_IMAGE="ghcr.io/heliaxdev/ant-check:latest" \
WORKLOAD_IMAGE="local-workload:latest" \
TEST_SEED=${RANDOM} \
TEST_TIME_SEC=300 \
docker-compose -f config/docker-compose.yml up --force-recreate