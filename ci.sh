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

docker compose -f config/docker-compose.yml down
WORKLOAD_NUM=3 \
TEST_SEED=${RANDOM} \
TEST_TIME_SEC=300 \
docker compose -f config/docker-compose.yml up --abort-on-container-exit

cnt=$(docker logs workload | grep "Done successfully" | wc -l)
if [ $cnt -eq ${WORKLOAD_NUM} ]; then
  exit 0
else
  echo "!!!! Test failed !!!!"
  exit 1
fi
