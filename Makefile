.PHONY: setup compile deploy setup-node update-node run-node tx-output-local init

# Main Commands

setup: setup-node
	yarn install

init:
	./scripts/init.sh

clean: clean-hardhat clean-node-modules clean-era-test-node

# Node Commands

download-node:
	[ -d "./era-test-node" ] || git clone git@github.com:matter-labs/era-test-node.git

build-node:
	cd era-test-node && make rust-build && make build-contracts

setup-node: download-node build-node

update-node:
	cd era-test-node && git pull && make rust-build

run-node:
	./era-test-node/target/release/era_test_node --show-calls=all --resolve-hashes --show-gas-details=all run

run-node-light:
	./era-test-node/target/release/era_test_node run

# Cleanzine Commands

clean-hardhat:
	yarn hardhat clean && \
	rm -rf cache-zk

clean-node-modules:
	rm -rf node_modules

clean-era-test-node:
	rm -rf era-test-node

# L3 Command

compile:
	yarn hardhat compile

deploy-base-layer-contracts:
	yarn hardhat deploy-zksync --script deploy/base-layer-contracts.ts

run-l3:
	cd core/bin/zksync_server && \
	cargo +nightly run --bin zksync_server --release

fund:
	yarn hardhat deploy-zksync --script deploy/init-rich-accounts.ts
