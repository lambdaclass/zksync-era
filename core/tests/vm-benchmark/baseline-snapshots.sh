#!/bin/sh
cargo bench --bench criterion -- --save-baseline bbase "^lambda/(call_far|deploy_simple_contract)$"
