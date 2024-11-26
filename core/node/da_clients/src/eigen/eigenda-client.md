# EigenDA Client

EigenDA is as a high-throughput data availability layer for rollups. It is an EigenLayer AVS (Actively Validated
Service), so it leverages Ethereum's economic security instead of bootstrapping a new network with its own validators.
For more information you can check the [docs](https://docs.eigenda.xyz/).

## Scope

The scope of this first milestone is to spin up a local EigenDA dev environment, spin up a local zksync-era dev
environment and integrate them. Instead of sending 4844 blobs, the zksync-era sends blobs to EigenDA. On L1, mock the
verification logic, such that blocks continue building. Increase the blob size from 4844 size to 2MiB blob. Deploy the
integration to Holesky testnet and provide scripts to setup a network using EigenDA as DA provider.

