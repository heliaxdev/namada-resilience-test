#!/bin/bash

set -e

RPC=${RPC:-"30.0.0.14:27658"}

echo "Using rpc: ${RPC}"

./namada-chain-check --rpc http://${RPC}