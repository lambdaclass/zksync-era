#!/bin/bash

if [ $# -eq 0 ]; then
    echo "Usage: compile.sh <test_name>"
    exit 1
fi

test_name="$1"
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

OUTPUT_DIR="${SCRIPT_DIR}/${test_name}"

zksolc --overwrite --bin "${SCRIPT_DIR}/deployment_benchmarks_sources/${test_name}.sol" -O 3 -o "${OUTPUT_DIR}"
mv "${OUTPUT_DIR}/${test_name}.sol/benchmark.zbin" "${SCRIPT_DIR}/deployment_benchmarks/${test_name}"
rm -rf ${OUTPUT_DIR}

