#!/bin/sh
cargo bench --bench criterion -- --baseline bbase "^lambda/(access_memory|heap_read_write|load_test)$"
