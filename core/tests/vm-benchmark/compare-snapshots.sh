#!/bin/sh
cargo bench --bench criterion -- --baseline bbase "^lambda/(call_far|deploy_simple_contract)$"
