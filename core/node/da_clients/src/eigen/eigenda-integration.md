# Zksync-era <> EigenDA Integration

EigenDA is as a high-throughput data availability layer for rollups. It is an EigenLayer AVS (Actively Validated
Service), so it leverages Ethereum's economic security instead of bootstrapping a new network with its own validators.
For more information you can check the [docs](https://docs.eigenda.xyz/).

## Scope

The scope of this first milestone is to spin up a local EigenDA dev environment, spin up a local zksync-era dev
environment and integrate them. Instead of sending 4844 blobs, the zksync-era sends blobs to EigenDA. On L1, mock the
verification logic, such that blocks continue building. Increase the blob size from 4844 size to 2MiB blob. Deploy the
integration to Holesky testnet and provide scripts to setup a network using EigenDA as DA provider.

## Common changes

Changes needed both for local and mainnet/testnet setup.

1. Add `da_client` to `etc/env/file_based/general.yaml`:

If you want to use memstore:

```yaml
da_client:
  eigen:
    mem_store:
      max_blob_size_bytes: 2097152
      blob_expiration: 100000
      get_latency: 100
      put_latency: 100
```

If you want to use disperser:

```yaml
da_client:
  eigen:
    disperser:
      disperser_rpc: <your_desired_disperser>
      eth_confirmation_depth: -1
      eigenda_eth_rpc: <your_desired_rpc>
      eigenda_svc_manager_address: '0xD4A7E1Bd8015057293f0D0A557088c286942e84b'
      blob_size_limit: 2097152
      status_query_timeout: 1800000 # ms
      status_query_interval: 5 # ms
      wait_for_finalization: false
      authenticated: false
      verify_cert: true
      path_to_points: ./resources
```

Also set the private key in `etc/env/file_based/secrets.yaml`:

```yaml
da:
  eigen:
    private_key: '<your_private_key>'
```

2. (optional) for using pubdata with 2MiB (as per specification), modify `etc/env/file_based/general.yaml`:

```yaml
max_pubdata_per_batch: 2097152
```

## Local Setup

1. Install `zkstack`

```bash
cargo install --path zkstack_cli/crates/zkstack --force --locked
```

2. Start containers

```bash
zkstack containers --observability true
```

3. Temporary metrics setup (until `era-observabilty` changes are also merged)

a. Setup the observability container at least once so the `era-observability` directory is cloned.

```bash
zkstack containers --observability true
```

b. Add `lambda` remote to the `era-observability` project:

```bash
cd era-observability && git remote add lambda https://github.com/lambdaclass/era-observability.git
```

c. Fetch and checkout the `eigenda` branch:

```bash
git fetch lambda && git checkout eigenda
```

4. Create `eigen_da` chain

```bash
zkstack chain create \
          --chain-name eigen_da \
          --chain-id sequential \
          --prover-mode no-proofs \
          --wallet-creation localhost \
          --l1-batch-commit-data-generator-mode validium \
          --base-token-address 0x0000000000000000000000000000000000000001 \
          --base-token-price-nominator 1 \
          --base-token-price-denominator 1 \
          --set-as-default false
```

5. Initialize created ecosystem

```bash
zkstack ecosystem init \
          --deploy-paymaster true \
          --deploy-erc20 true \
          --deploy-ecosystem true \
          --l1-rpc-url http://127.0.0.1:8545 \
          --server-db-url=postgres://postgres:notsecurepassword@localhost:5432 \
          --server-db-name=zksync_server_localhost_eigen_da \
          --chain eigen_da \
          --verbose
```

You may enable observability here if you want to.

6. Setup grafana dashboard for Data Availability

a. Get the running port of the eigen_da chain in the `chains/eigen_da/configs/general.yaml` file:

```yaml
prometheus:
  listener_port: 3414 # <- this is the port
```

(around line 108)

Then modify the `era-observability/etc/prometheus/prometheus.yml` with the retrieved port:

```yaml
- job_name: 'zksync'
  scrape_interval: 5s
  honor_labels: true
  static_configs:
    - targets: ['host.docker.internal:3312'] # <- change this to the port
```

b. Enable the Data Availability Grafana dashboard

```bash
mv era-observability/additional_dashboards/EigenDA.json era-observability/dashboards/EigenDA.json
```

c. Restart the era-observability container

```bash
docker ps --filter "label=com.docker.compose.project=era-observability" -q | xargs docker restart
```

(this can also be done through the docker dashboard)

7. Start the server

```bash
zkstack server --chain eigen_da
```

### Testing

Modify the following flag in `core/lib/config/src/configs/da_dispatcher.rs` (then restart the server)

```rs
pub const DEFAULT_USE_DUMMY_INCLUSION_DATA: bool = true;
```

And with the server running on one terminal, you can run the server integration tests on a separate terminal with the
following command:

```bash
zkstack dev test integration --chain eigen_da
```

## Mainnet/Testnet setup

### Modify localhost chain id number

Modify line 32 in `zk_toolbox/crates/types/src/l1_network.rs`:

```rs
L1Network::Localhost => 17000,
```

Then recompile the zkstack:

```bash
cargo install --path zkstack_cli/crates/zkstack --force --locked
```

### Used wallets

Modify `etc/env/file_based/wallets.yaml` and `configs/wallets.yaml` with the following wallets:

```yaml
# Use your own holesky wallets, be sure they have enough funds
```

> ⚠️ Some steps distribute ~5000ETH to some wallets, modify `AMOUNT_FOR_DISTRIBUTION_TO_WALLETS` to a lower value if
> needed.

### Create and initialize the ecosystem

(be sure to have postgres container running on the background)

```bash
zkstack chain create \
          --chain-name holesky_eigen_da \
          --chain-id 114411 \
          --prover-mode no-proofs \
          --wallet-creation localhost \
          --l1-batch-commit-data-generator-mode validium \
          --base-token-address 0x0000000000000000000000000000000000000001 \
          --base-token-price-nominator 1 \
          --base-token-price-denominator 1 \
          --set-as-default false

zkstack ecosystem init \
          --deploy-paymaster true \
          --deploy-erc20 true \
          --deploy-ecosystem true \
          --l1-rpc-url $HOLESKY_RPC_URL \
          --server-db-url=postgres://postgres:notsecurepassword@localhost:5432 \
          --server-db-name=zksync_server_holesky_eigen_da \
          --prover-db-url=postgres://postgres:notsecurepassword@localhost:5432 \
          --prover-db-name=zksync_prover_holesky_eigen_da \
          --chain holesky_eigen_da \
          --verbose
```

### Start the server

```bash
zkstack server --chain holesky_eigen_da
```