#!/bin/bash

set -e

# Wait for the JSON RPC to come up for validator 2
json_rpc_ready=1
while [ $json_rpc_ready -eq 1 ]
do
    echo "Checking node rpc ${RPC}/status ..."
    json_rpc_ready=$(curl --silent --fail "http://${RPC}/status")
    echo "Node rpc query result: $json_rpc_ready"
    sleep 2
done

# Finding the CHAIN ID from the common volume mount directory
CHAIN_ID=$(find /container_ready -type f -name "devnet*")
while [[ -z $CHAIN_ID ]]
do
    echo Waiting for the chain ID
    CHAIN_ID=$(find /container_ready -type f -name "devnet*")
    sleep 2
done

CHAIN_ID=$(basename $CHAIN_ID)

# Wait for the JSON RPC to come up for masp indexer
json_rpc_ready=1
while [ $json_rpc_ready -eq 1 ]
do
    echo "Checking masp indexer ${MASP_INDEXER_URL}/api/v1/health ..."
    json_rpc_ready=$(curl --silent --fail "${MASP_INDEXER_URL}/api/v1/health")
    echo "Masp indexer query result: $json_rpc_ready"
    sleep 2
done

# Ready to start workload
echo "Ready to start the workload"