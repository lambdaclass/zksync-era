# EigenDA Client

EigenDA is as a high-throughput data availability layer for rollups. It is an EigenLayer AVS (Actively Validated
Service), so it leverages Ethereum's economic security instead of bootstrapping a new network with its own validators.
For more information you can check the [docs](https://docs.eigenda.xyz/).

## Client configuration

To set up an eigenda client, you need to modify `etc/env/file_based/overrides/validium.yaml`:

First you need to set the `use_dummy_inclusion_data` field to `true`. This is a pending solution until the necessary
contract changes are done (M1 milestone).

```yaml
da_dispatcher:
  use_dummy_inclusion_data: true
```

Then set up the client by modifying the field `da_client`, add the following fields:

- `disperser_rpc` (string): URL of the EigenDA Disperser RPC server.
- `eigenda_eth_rpc` (optional string): URL of the Ethereum RPC server. If the value is not set, the client will use the
  same rpc as the rest of the zk server.
- `authenticated` (boolean): Authenticated dispersal. If true, the client will use the authentication mechanism, using a
  whitelisted account. Using non authenticated dispersal is not recommended, as to many requests to the EigenDA
  disperser leeds to timeouts.
- `cert_verifier_addr` Address of the eigenDA cert verifier contract
- `blob_version` Blob Version used by eigenDA, currently only blob version 0 is supported
- `polynomial_form` Polynomial form used to encode data, either coeff or eval

You also need to modify `etc/env/file_based/secrets.yaml` to include the private key of the account that will be used.
You need to add the following field:

```yaml
da_client:
  client: EigenDA
  private_key: <PRIVATE_KEY>
```

> Note: the private key should be in hex format, without the `0x` prefix.

So, for example, a client setup that uses the holesky EigenDA V2 client would look like this:

`etc/env/file_based/overrides/validium.yaml`:

```yaml
da_dispatcher:
  use_dummy_inclusion_data: true
da_client:
  client: EigenDA
  disperser_rpc: https://disperser-testnet-holesky.eigenda.xyz
  eigenda_eth_rpc: https://ethereum-holesky-rpc.publicnode.com
  authenticated: true
  cert_verifier_addr: 0xfe52fe1940858dcb6e12153e2104ad0fdfbe1162
  blob_version: 0
  polynomial_form: coeff
```
