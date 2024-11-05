.PHONY: install getallblobs

install:
	npm install

clone-proxy:
	@if [ ! -d eigenda-proxy ]; then \
        git clone https://github.com/Layr-Labs/eigenda-proxy.git; \
    fi

setup-proxy: clone-proxy
	@echo "Installing proxy dependencies"
	cd eigenda-proxy && make

run-proxy: setup-proxy
	@if [ -z "$(PRIVATE_KEY)" ]; then \
        echo "Error: PRIVATE_KEY is not set"; \
        exit 1; \
    fi
	cd eigenda-proxy && ./bin/eigenda-proxy \
        --addr 127.0.0.1 \
        --port 4242 \
        --eigenda.disperser-rpc disperser-holesky.eigenda.xyz:443 \
        --eigenda.signer-private-key-hex $(PRIVATE_KEY) \
        --eigenda.eth-rpc https://ethereum-holesky-rpc.publicnode.com \
        --eigenda.svc-manager-addr 0xD4A7E1Bd8015057293f0D0A557088c286942e84b

get-all-blobs: install
	@if [ -z "$(VALIDATOR_TIMELOCK_ADDR)" ]; then \
		echo "Error: VALIDATOR_TIMELOCK_ADDR is not set"; \
		exit 1; \
	fi
	@if [ -z "$(COMMIT_BATCHES_SB_FUNC_SELECTOR)" ]; then \
		echo "Error: COMMIT_BATCHES_SB_FUNC_SELECTOR is not set"; \
		exit 1; \
	fi
	node getallblobs.js validatorTimelockAddress=$(VALIDATOR_TIMELOCK_ADDR) commitBatchesSharedBridge_functionSelector=$(COMMIT_BATCHES_SB_FUNC_SELECTOR)
