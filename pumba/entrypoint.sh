#!/bin/sh

BASE_SEED=${TEST_SEED:-42}
# TODO: skip fullnode for https://github.com/anoma/namada/issues/4689
TARGET_CONTAINERS="validator0 validator1 validator2 masp-chain masp-webserver masp-block-filter gaia hermes"
CONTAINERS=$(echo "$TARGET_CONTAINERS")
FAULTS="kill pause delay loss rate duplicate corrupt"

echo "[INFO] Using base seed: $BASE_SEED"

rand_range() {
  seed="$1"
  min="$2"
  max="$3"
  awk -v seed="$seed" -v min="$min" -v max="$max" 'BEGIN {
    srand(seed);
    print int(min + rand() * (max - min + 1))
  }'
}

# Wait for workload initialization
while [ ! -f /container_ready/workload ]
do
    echo "Waiting for workload initialization..."
    sleep 5
done

LOOP=0
while true; do
  echo "[INFO] ==== Starting fault injection loop $LOOP ===="

  SEED_TARGET=$((BASE_SEED + LOOP * 10 + 1))
  TARGETS=$(echo "$CONTAINERS" | tr ' ' '\n' | awk -v seed="$SEED_TARGET" '
    BEGIN {
      srand(seed);
    }
    {
      a[NR] = $0;
    }
    END {
      n = NR;
      count = int(rand() * n) + 1;
      used_count = 0;
      while (used_count < count) {
        i = int(rand() * n) + 1;
        if (!(i in used)) {
          print a[i];
          used[i] = 1;
          used_count++;
        }
      }
    }
  ')

  SEED_FAULT=$((BASE_SEED + LOOP * 10 + 2))
  FAULT=$(echo "$FAULTS" | tr ' ' '\n' | awk -v seed="$SEED_FAULT" '
    BEGIN {
      srand(seed);
    }
    {
      a[NR]=$0;
    }
    END {
      print a[int(rand()*NR)+1];
    }
  ')

  SEED_DURATION=$((BASE_SEED + LOOP * 10 + 3))
  DURATION=$(rand_range "$SEED_DURATION" 1 10)s

  echo "[INFO] Injecting fault '$FAULT' into [$TARGETS] for $DURATION"

  case "$FAULT" in
    kill)
      # Containers couldn't restart when `pumba kill`
      pumba kill $TARGETS
      sleep $DURATION
      docker restart $TARGETS
      ;;
    pause)
      pumba pause --duration "$DURATION" $TARGETS
      ;;
    delay)
      pumba netem --duration "$DURATION" delay --time 1000 $TARGETS
      ;;
    loss)
      pumba netem --duration "$DURATION" loss --percent 30 $TARGETS
      ;;
    rate)
      pumba netem --duration "$DURATION" rate --rate 128kbit $TARGETS
      ;;
    duplicate)
      pumba netem --duration "$DURATION" duplicate --percent 10 $TARGETS
      ;;
    corrupt)
      pumba netem --duration "$DURATION" corrupt --percent 5 $TARGETS
      ;;
    *)
      echo "[WARN] Unknown fault type: $FAULT"
      ;;
  esac

  SEED_INTERVAL=$((BASE_SEED + LOOP * 10 + 4))
  INTERVAL=$(( $(rand_range "$SEED_INTERVAL" 1 120) + 60 ))
  echo "[INFO] ==== Sleeping for $INTERVAL seconds ===="
  sleep "$INTERVAL"

  LOOP=$((LOOP + 1))
done
