.PHONY: install getallblobs

install:
	npm install

getallblobs: install
	@if [ -z "$(VALIDATOR_TIMELOCK_ADDR)" ]; then \
		echo "Error: VALIDATOR_TIMELOCK_ADDR is not set"; \
		exit 1; \
	fi
	@if [ -z "$(COMMIT_BATCHES_SB_FUNC_SELECTOR)" ]; then \
		echo "Error: COMMIT_BATCHES_SB_FUNC_SELECTOR is not set"; \
		exit 1; \
	fi
	node getallblobs.js validatorTimelockAddress=$(VALIDATOR_TIMELOCK_ADDR) commitBatchesSharedBridge_functionSelector=$(COMMIT_BATCHES_SB_FUNC_SELECTOR)
