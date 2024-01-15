## Validium

In order to start the node as a validium:

- Make sure `zk` has been built and then run
`zk && zk clean --all && zk init --validium-mode`
This will set up the Ethereum node with the validium contracts, and also define an env var which the server will pick up in order to run as a validium node. 
- Start the server (`zk server`)
- Execute transactions. For testing, run `cargo run --release --bin validium_mode_example`, this test does the following: 
  - Inits a wallet 
  - Deposits some funds into the wallet
  - Deploys a sample ERC20 contract
  - Query the contract for the token name and symbol
  - Mint 100000 tokens into the address `CD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826`
  - Transfer 1000 tokens from `CD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826` to `bBbBBBBbbBBBbbbBbbBbbbbBBbBbbbbBbBbbBBbB`

### Logs and prints 
- For each transaction, we use the rpc client to query the transaction details and print them out. The following fields are printed:
  - `Transaction hash`: The hash of the transaction
  - `Transaction gas used`: The gas used to perform this transaction.
  - `L2 fee`: The total cost of this transaction.
  - `L1 Gas`: The gas borrowed in order to run the transaction. Unused gas will be returned.
  - `L1 Gas price`: The gas price used to run the transaction.

### Example output
```
Deposit transaction hash: 0xc01cf32c699943f8d751047514393a5e98d8cbeaa128fa50c32a3d7804b876a5
Deploy
Contract address: 0xf2fcc18ed5072b48c0a076693eca72fe840b3981
Transaction hash 0x9e817fcc8eeeda001793c9142161a11e3fd3ef3c64523be1f5c11b6cbff7b64f
Transaction gas used 161163
L2 fee: 40290750000000
L1 Gas: 4000000
L1 Gas price: 1000000007

Mint
Transaction hash 0x0e9bcc26addf1edfe0993767cc2d6ec959a135dc3087b63b5fc9d54d7ed854ef
Transaction gas used 124046
L2 fee: 31011500000000
L1 max fee per gas: 1000000010
L1 Gas: 4000000
L1 Gas price: 1000000007

Transfer 1000
Transaction hash 0x5a1f7130024b73c2d3de5256a72bddbc703983d69d3ad0f3f64d8e6122e0e85a
Transaction gas used 125466
L2 fee: 31366500000000
L1 max fee per gas: 1000000010
L1 Gas: 4000000
L1 Gas price: 1000000007
```
