# retreive-blobs

In order to run this first deploy a local zksync env with eigen da

On eigenda-proxy:
```
make
./bin/eigenda-proxy \
    --addr 127.0.0.1 \
    --port 3100 \
    --memstore.enabled
```

On zksync-era:

```
export ZKSYNC_HOME=$(pwd) && export PATH=$ZKSYNC_HOME/bin:$PATH
zk init
export $(xargs < etc/env/target/dev.env)
export EIGEN_DA_CLIENT_API_NODE_URL=http://127.0.0.1:3100
cargo run --bin zksync_server --features da-eigen
```

To have more blobs commited you can run this tests-

```
zk test i server
```

Then run this script

```
cargo run
```
