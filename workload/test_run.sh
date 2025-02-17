#!/bin/bash

MAX_RETRY_COUNT=5

touch state-$WORKLOAD_ID.json
echo "" > state-$WORKLOAD_ID.json
touch /opt/antithesis/test/v1/namada/state-$WORKLOAD_ID.json
echo "" > /opt/antithesis/test/v1/namada/state-$WORKLOAD_ID.json

mkdir -p base/wallet-$WORKLOAD_ID
mkdir -p base/masp-$WORKLOAD_ID

mkdir -p /opt/antithesis/test/v1/namada/wallet-$WORKLOAD_ID
mkdir -p /opt/antithesis/test/v1/namada/masp-$WORKLOAD_ID

source /opt/antithesis/test/v1/namada/init_script.sh
if [ $? -eq 0 ]
then
    echo "<OK> init" 
else
    echo "<ERROR> init"
    exit 1
fi

# create_wallet, faucet_transfer, bond, init_account have been already executed in the init script

source /opt/antithesis/test/v1/namada/parallel_driver_transparent_transfer.sh
if [ $? -eq 0 ]
then
    echo "<OK> transparent transfer" 
else
    echo "<ERROR> transparent transfer"
    exit 1
fi

source /opt/antithesis/test/v1/namada/parallel_driver_redelegate.sh
if [ $? -eq 0 ]
then
    echo "<OK> redelegate" 
else
    echo "<ERROR> redelegate"
    exit 1
fi

source /opt/antithesis/test/v1/namada/parallel_driver_unbond.sh
if [ $? -eq 0 ] 
then
    echo "<OK> unbond" 
else
    echo "<ERROR> unbond"
    exit 1
fi

source /opt/antithesis/test/v1/namada/parallel_driver_update_account.sh
if [ $? -eq 0 ]
then
    echo "<OK> update account" 
else
    echo "<ERROR> update account"
    exit 1
fi

retries=1
while [ $retries -le $MAX_RETRY_COUNT ]; do
    source /opt/antithesis/test/v1/namada/parallel_driver_shielding.sh
    if [ $? -eq 0 ]
    then
        echo "<OK> shielding" 
        break
    else
        retries=$((retries + 1))
        echo "<RETRY ${retries}/$MAX_RETRY_COUNT> shielding"
    fi
done
if [ $retries -gt $MAX_RETRY_COUNT ]; then
    echo "<ERROR> shielding"
    exit 1
fi

retries=1
while [ $retries -le $MAX_RETRY_COUNT ]; do
    source /opt/antithesis/test/v1/namada/parallel_driver_shielded.sh
    if [ $? -eq 0 ]
    then
        echo "<OK> shielded" 
        break
    else
        retries=$((retries + 1))
        echo "<RETRY ${retries}/$MAX_RETRY_COUNT> shielded"
    fi
done
if [ $retries -gt $MAX_RETRY_COUNT ]; then
    echo "<ERROR> shielded"
    exit 1
fi

retries=1
while [ $retries -le $MAX_RETRY_COUNT ]; do
    source /opt/antithesis/test/v1/namada/parallel_driver_unshielding.sh
    if [ $? -eq 0 ]
    then
        echo "<OK> unshielding" 
        break
    else
        retries=$((retries + 1))
        echo "<RETRY ${retries}/$MAX_RETRY_COUNT> unshielding"
    fi
done
if [ $retries -gt $MAX_RETRY_COUNT ]; then
    echo "<ERROR> unshielding"
    exit 1
fi

source /opt/antithesis/test/v1/namada/parallel_driver_claim_rewards.sh
if [ $? -eq 0 ] 
then
    echo "<OK> claim rewards" 
else
    echo "<ERROR> claim rewards"
    exit 1
fi

source /opt/antithesis/test/v1/namada/parallel_driver_become_validator.sh
if [ $? -eq 0 ] 
then
    echo "<OK> become validator" 
else
    echo "<ERROR> become validator"
    exit 1
fi

source /opt/antithesis/test/v1/namada/parallel_driver_change_metadata.sh
if [ $? -eq 0 ] 
then
    echo "<OK> change metadata" 
else
    echo "<ERROR> change metadata"
    exit 1
fi

source /opt/antithesis/test/v1/namada/parallel_driver_change_consensus_keys.sh
if [ $? -eq 0 ] 
then
    echo "<OK> change consensus keys" 
else
    echo "<ERROR> change consensus keys"
    exit 1
fi

source /opt/antithesis/test/v1/namada/parallel_driver_bond_batch.sh
if [ $? -eq 0 ] 
then
    echo "<OK> bond batch" 
else
    echo "<ERROR> bond batch"
    exit 1
fi

retries=1
while [ $retries -le $MAX_RETRY_COUNT ]; do
    source /opt/antithesis/test/v1/namada/parallel_driver_random_batch.sh
    if [ $? -eq 0 ] 
    then
        echo "<OK> random batch" 
        break
    else
        retries=$((retries + 1))
        echo "<RETRY ${retries}/$MAX_RETRY_COUNT> random batch"
    fi
done
if [ $retries -gt $MAX_RETRY_COUNT ]; then
    echo "<ERROR> random batch"
    exit 1
fi

echo "Test was completed successfully"
