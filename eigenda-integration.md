# Zksync-era <> EigenDA Integration

## Local Setup

1. Install `zk_inception` & `zk_supervisor`

```bash
./bin/zkt
```

2. Start containers

```bash
zk_inception containers --observability true
```

3. Create `eigen_da` chain

```bash
zk_inception chain create \
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

4. Initialize created ecosystem

```bash
zk_inception ecosystem init \
          --deploy-paymaster true \
          --deploy-erc20 true \
          --deploy-ecosystem true \
          --l1-rpc-url http://127.0.0.1:8545 \
          --server-db-url=postgres://postgres:notsecurepassword@localhost:5432 \
          --server-db-name=zksync_server_localhost_eigen_da \
          --prover-db-url=postgres://postgres:notsecurepassword@localhost:5432 \
          --prover-db-name=zksync_prover_localhost_eigen_da \
          --chain eigen_da \
          --verbose
```

You may enable observability here if you want to.

5. Start the server

```bash
zk_inception server --chain eigen_da
```

### Testing

Modify the following flag in `core/lib/config/src/configs/da_dispatcher.rs` (then restart the server)

```rs
pub const DEFAULT_USE_DUMMY_INCLUSION_DATA: bool = true;
```

And with the server running on one terminal, you can run the server integration tests on a separate terminal with the
following command:

```bash
zk_supervisor test integration --chain eigen_da
```

### Metrics

Access Grafana at [http://localhost:3000/](http://localhost:3000/), go to dashboards and select `EigenDA`.

## Holesky Setup

### Used wallets

Modify `etc/env/file_based/wallets.yaml` with the following wallets:

```yaml
# Use your own holesky wallets, be sure they have enough funds
```

### EigenProxy RPC

Get `EIGEN_SIGNER_PK` from 1password and set it as an `env` var:

```bash
export EIGEN_SIGNER_PK=<VALUE_HERE>
```

Modify `docker-compose.yml` to use holesky RPCs:

```rust
  eigenda-proxy:
    image: ghcr.io/layr-labs/eigenda-proxy
    environment:
      - EIGEN_SIGNER_PK=$EIGEN_SIGNER_PK
    ports:
      - "4242:4242"
    command: ./eigenda-proxy --addr 0.0.0.0 --port 4242 --eigenda-disperser-rpc disperser-holesky.eigenda.xyz:443 --eigenda-signer-private-key-hex $EIGEN_SIGNER_PK --eigenda-eth-rpc https://ethereum-holesky-rpc.publicnode.com --eigenda-svc-manager-addr 0xD4A7E1Bd8015057293f0D0A557088c286942e84b --eigenda-eth-confirmation-depth 0
```

### Create and initialize the ecosystem

```bash
zk_inception chain create \
          --chain-name holesky_eigen_da \
          --chain-id 275 \
          --prover-mode no-proofs \
          --wallet-creation localhost \
          --l1-batch-commit-data-generator-mode validium \
          --base-token-address 0x0000000000000000000000000000000000000001 \
          --base-token-price-nominator 1 \
          --base-token-price-denominator 1 \
          --set-as-default false

zk_inception ecosystem init \
          --deploy-paymaster true \
          --deploy-erc20 true \
          --deploy-ecosystem true \
          --l1-rpc-url https://ethereum-holesky-rpc.publicnode.com \
          --server-db-url=postgres://postgres:notsecurepassword@localhost:5432 \
          --server-db-name=zksync_server_holesky_eigen_da \
          --prover-db-url=postgres://postgres:notsecurepassword@localhost:5432 \
          --prover-db-name=zksync_prover_holesky_eigen_da \
          --chain holesky_eigen_da \
          --verbose
```

## Backup and restoration

It's possible to run the zk stack on one computer, and then migrate it to another, this is specially useful for holesky testing.

### Backup

Set up the following command to streamline the process:

```bash
export PGPASSWORD=notsecurepassword
```

1. Let's assume that you have set up an `holesky_eigen_da` chain to run in holesky, you can backup the database with the following commands (make sure the databases are named likewise):

```bash
pg_dump -U postgres -h localhost zksync_server_holesky_eigen_da > zksync_server_holesky_eigen_da_backup.sql
pg_dump -U postgres -h localhost zksync_prover_holesky_eigen_da > zksync_prover_holesky_eigen_da_backup.sql
```

2. You also need to backup the chain configuration, make a copy of the folder `ZKSYNC_HOME/chains/hiolesky_eigen_da`

### Restoration

On the new computer you can restore the databases with the following commands:

1. Initialize postgres containers:

```bash
zk_inception containers
```

2. Create the `eigen_da` chain with the same parameters as before (for example):

```bash
zk_inception chain create \
          --chain-name holesky_eigen_da \
          --chain-id 275 \
          --prover-mode no-proofs \
          --wallet-creation localhost \
          --l1-batch-commit-data-generator-mode validium \
          --base-token-address 0x0000000000000000000000000000000000000001 \
          --base-token-price-nominator 1 \
          --base-token-price-denominator 1 \
          --set-as-default false
```

4. Restore the databases:

```bash
createdb -U postgres -h localhost zksync_server_holesky_eigen_da
psql -U postgres -h localhost -d zksync_server_holesky_eigen_da -f zksync_server_holesky_eigen_da_backup.sql

createdb -U postgres -h localhost zksync_prover_holesky_eigen_da
psql -U postgres -h localhost -d zksync_prover_holesky_eigen_da -f zksync_prover_holesky_eigen_da_backup.sql
```

> ⚠️ The following step may and should be simplified

5. Create and init a dummy ecosystem to build necessary contracts:

```bash
zk_inception chain create \
          --chain-name dummy_chain \
          --chain-id sequential \
          --prover-mode no-proofs \
          --wallet-creation localhost \
          --l1-batch-commit-data-generator-mode validium \
          --base-token-address 0x0000000000000000000000000000000000000001 \
          --base-token-price-nominator 1 \
          --base-token-price-denominator 1 \
          --set-as-default false

zk_inception ecosystem init \
          --deploy-paymaster true \
          --deploy-erc20 true \
          --deploy-ecosystem true \
          --l1-rpc-url http://127.0.0.1:8545 \
          --server-db-url=postgres://postgres:notsecurepassword@localhost:5432 \
          --server-db-name=zksync_server_dummy_chain \
          --prover-db-url=postgres://postgres:notsecurepassword@localhost:5432 \
          --prover-db-name=zksync_prover_dummy_chain \
          --chain dummy_chain \
          --verbose
```

6. Start the server of the restored chain:

```bash
zk_inception server --chain zksync_server_holesky_eigen_da
```

If everything went well, you should see the server logging a message like:

```
2024-09-30T13:29:41.723324Z  INFO zksync_state_keeper::keeper: Starting state keeper. Next l1 batch to seal: 44, next L2 block to seal: 138
```
