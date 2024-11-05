# Get All Blobs

This script retrieves all blobs commitments directly from L1, this script is set up to use an EigenDA proxy connected to
_holesky_, so the chain won't work if it was set up to run with a `Memstore` client implementation.

To make use of this tool, you need to have `make` and `npm` installed.

### Start Proxy

Before running the command, you need to start the proxy. To do so, first export the variable `PRIVATE_KEY` (without 0x).
Then run the following command:

```
make start-proxy
```

### Run command:

Run in a separate terminal:

```
make get-all-blobs VALIDATOR_TIMELOCK_ADDR=<validatorTimelockAddress> COMMIT_BATCHES_SB_FUNC_SELECTOR=<commitBatchesSharedBridge_functionSelector>
```

This generates a `blob_data.json` file, where blobs and commitments are stored.

### Environment Variables

`VALIDATOR_TIMELOCK_ADDR`: The address of the validator timelock. Check the value in `zkstack init` if running a local
node.

`COMMIT_BATCHES_SB_FUNC_SELECTOR`: The function selector for commitBatchesSharedBridge. For a local node, this is
typically 0x6edd4f12.

This variables can be exported in the shell in order to run `make getallblobs` without passing them as arguments.
