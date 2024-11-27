# EigenDA Client

EigenDA is as a high-throughput data availability layer for rollups. It is an EigenLayer AVS (Actively Validated
Service), so it leverages Ethereum's economic security instead of bootstrapping a new network with its own validators.
For more information you can check the [docs](https://docs.eigenda.xyz/).

## Scope

The scope of this first milestone is to spin up a local EigenDA dev environment, spin up a local zksync-era dev
environment and integrate them. Instead of sending 4844 blobs, the zksync-era sends blobs to EigenDA. On L1, mock the
verification logic, such that blocks continue building. Increase the blob size from 4844 size to 2MiB blob. Deploy the
integration to Holesky testnet and provide scripts to setup a network using EigenDA as DA provider.

## Temporary

The generated files are received by compiling the `.proto` files from EigenDA repo using the following function:

```rust
pub fn compile_protos() {
    let fds = protox::compile(
        [
            "proto/common.proto",
            "proto/disperser.proto",
        ],
        ["."],
    )
    .expect("protox failed to build");

    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        .skip_protoc_run()
        .out_dir("generated")
        .compile_fds(fds)
        .unwrap();
}
```

proto files are not included here to not create confusion in case they are not updated in time, so the EigenDA
[repo](https://github.com/Layr-Labs/eigenda/tree/master/api/proto) has to be a source of truth for the proto files.

The generated folder here is considered a temporary solution until the EigenDA has a library with either a protogen, or
preferably a full Rust client implementation.
