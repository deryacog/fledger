#!/bin/bash -e

make kill

PATH_LEN=$1
RETRY=$2

if ! [[ "$RETRY" =~ ^[0-9]+$ ]]; then
  echo "Usage: $0 path_len retry"
  echo "retry must be an integer"
  exit 1
fi

(cd cli/fledger && cargo build -r)
(cd cli/flsignal && cargo build -r)

NODES=$(( ( $PATH_LEN * 2 ) + $PATH_LEN * $PATH_LEN ))
SIMUL=simul/
rm -rf $SIMUL
mkdir -p $SIMUL

./target-common/release/flsignal |& ts "Signal " &

for NODE in $( seq $NODES ); do
  NAME="NODE_$(printf "%02d" $NODE)"
  echo "Starting node $NAME"
  CONFIG="$SIMUL$NAME/"
  VERBOSITY="-v"
  PATH_LEN_ARG=""
  RETRY_ARG=""
  if [ $NODE = "1" ]; then
    PATH_LEN_ARG="--path-len $PATH_LEN"
  fi
  if [ "$RETRY" -gt 0 ]; then
    RETRY_ARG="--retry $RETRY"
  fi
  START_TIME="--start_loopix_time 45"
  SAVE_NEW_METRICS_FILE=""
  mkdir -p $CONFIG
  cp "loopix_core_config.yaml" $CONFIG
  RUST_BACKTRACE=full ./target-common/release/fledger --config $CONFIG $START_TIME $SAVE_NEW_METRICS_FILE --name $NAME $VERBOSITY -s ws://localhost:8765 $PATH_LEN_ARG $RETRY_ARG |& ts "$NAME" &
done

sleep 360

make kill