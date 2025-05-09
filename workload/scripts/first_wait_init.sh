#!/bin/bash

set -e

for id in $(seq 0 $((WORKLOAD_NUM - 1))); do
    while [ ! -f "/container_ready/workload-${id}" ]; do
        echo "Waiting for workload-${id} initialization..."
        sleep 2
    done
    echo "workload-${id} is ready"
done

echo "All workloads are ready"
