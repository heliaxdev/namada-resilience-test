#!/bin/bash

# Simple entrypoint for the validator to wait for joining the network before starting
set -e

BASE_DIR=${BASE_DIR:-"/validator0"}

joined_network=0
while [ $joined_network -eq 0 ]
do
    echo "Checking if validator has joined the network"
    if [ -e "/container_ready${BASE_DIR}" ]; then
        echo "Validator ready to start"
        joined_network=1
    fi
    sleep 10
done

/namada/target/release/namadan ledger run --base-dir $BASE_DIR
