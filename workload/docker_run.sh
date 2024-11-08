#!/bin/bash

set -e

touch state-$WORKLOAD_ID.json
echo "" > state-$WORKLOAD_ID.json
touch /opt/antithesis/test/v1/namada/state-$WORKLOAD_ID.json
echo "" > /opt/antithesis/test/v1/namada/state-$WORKLOAD_ID.json

if [[ ! -v ANTITHESIS_OUTPUT_DIR ]]; then
    while true
    do
        source /opt/antithesis/test/v1/namada/first_get_chainid.sh

        source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh

        source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh

        source /opt/antithesis/test/v1/namada/parallel_driver_bond.sh
        
        source /opt/antithesis/test/v1/namada/parallel_driver_transparent_transfer.sh

        source /opt/antithesis/test/v1/namada/parallel_driver_init_account.sh

        source /opt/antithesis/test/v1/namada/parallel_driver_bond_batch.sh

        source /opt/antithesis/test/v1/namada/parallel_driver_random_batch.sh
    done
else
    echo "ANTITHESIS_OUTPUT_DIR has the value: $ANTITHESIS_OUTPUT_DIR"

    sleep infinity
fi