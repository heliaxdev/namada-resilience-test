#!/bin/bash

set -e

CHAIN_ID=$(find /container_ready -type f -name "devnet*")
CHAIN_ID=$(basename $CHAIN_ID)

/app/namada-chain-workload --rpc http://${RPC} --chain-id ${CHAIN_ID} --faucet-sk ${FAUCET_SK} --id ${WORKLOAD_ID} --masp-indexer-url ${MASP_INDEXER_URL} vote