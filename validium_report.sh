#!/bin/bash

if [ "$#" -eq 0 ]; then
  echo "Error: No mode selected"
  exit 1
fi

MODE="$1"

if [ "$MODE" == "validium" ]; then
  MODE="--validium-mode"
elif [ "$MODE" == "rollup" ]; then
  MODE=""
else
  echo "Error: Invalid mode specified"
  exit 1
fi

zk clean && \
zk init $MODE && \
echo "------- START SERVER -------"
zk server > /dev/null 2>&1 &

sleep 10

cargo run --release --bin validium_mode_example -- $MODE && \
ps -ef | grep 'zksync_server' | grep -v grep | awk '{print $2}' | xargs kill
