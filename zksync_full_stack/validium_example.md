## Validium example

### Run the server

#### Rollup mode

```sh
zk && zk clean --all && zk init && zk server
```

#### Validium mode

```sh
zk && zk clean --all && zk init --validium-mode && zk server
```

### Run the binary

Once the server is running, run this command in other terminal to run the example:
```sh
cargo run --release --bin zksync_full_stack
```
You will have an output similar to this one (depending which mode is the server running):
```sh
Deposit transaction hash: 0x19d22b44455403e121629c5892a3571e1b98934829f66090ee66df2f4f84bf1b
Deploy
Contract address: 0x094499df5ee555ffc33af07862e43c90e6fee501
transaction hash 0xcdca13d5019dfe51f3a61914e3245afb116b90007faff4484c158b4f1a5686d0
transaction gas_used 175444
L2 fee: 43861000000000
L1 Gas: 4000000
L1 Gas price: 1000000007
Token name: ToniToken
Token symbol: teth

Mint
transaction hash 0xdcd1559a6cc9d9cd5e7de63e703279910e13dfd6b84080741059261a1c78ce6c
transaction gas_used 132635
L2 fee: 33158750000000
L1 max fee per gas: 1000000010
L1 Gas: 4000000
L1 Gas price: 1000000007

Transfer 1000
transaction hash 0x7b76a9b148744dafe016c9495b33da04b3c54b1a8c304af0e0247517e4b820a9
transaction gas_used 132417
L2 fee: 33104250000000
L1 max fee per gas: 1000000010
L1 Gas: 4000000
L1 Gas price: 1000000007
```

Once we have initialized the wallet and the clients, we have deployed the `ERC20` contract.
We can see the different fields of this transaction, the important ones are:
- `Transaction gas_used`: The gas used to perform this transaction.
- `L2 fee`: The total cost of this transaction.
- `L1 gas`: The gas borrowed in order to run the transaction. Unused gas will be returned.

We use the L1 RPC Client and the L2 RPC Client to retrieve that information.

Then we have executed two transactions, two functions of this contract: mint and transfer. We mint `10000 tokens` to the address `CD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826` and then transfer `1000 tokens` from this address to the address `bBbBBBBbbBBBbbbBbbBbbbbBBbBbbbbBbBbbBBbB`.

You can observe how the diferent fields evolve depending on the operation.