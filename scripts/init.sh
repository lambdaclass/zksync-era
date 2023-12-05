# ----------------------------------

if [ -z "$1" ]; then
    cd $ZKSYNC_HOME
    yarn && yarn zk build
else
    # can't start this with yarn since it has quirks with `--` as an argument
    node -- $ZKSYNC_HOME/infrastructure/zk/build/index.js "$@"
fi

# ----------------------------------

echo "> Pulling images"
docker-compose pull 

# ----------------------------------

echo "> Check environment"
COMMANDS=('node' 'yarn' 'docker' 'docker-compose' 'cargo')
for c in ${COMMANDS[@]}; do
        command -v $c >/dev/null 2>&1 || { echo "require ${c} but it's not installed." >&2; exit 1; }
done

# ----------------------------------

echo "> Setting up containers"
docker-compose up -d geth postgres

# ----------------------------------

echo "> Check Plonk setup"
URL="https://storage.googleapis.com/universal-setup"
mkdir -p keys/setup
cd keys/setup
for power in {20..26}; do   
        if [ ! -e setup_2^${power}.key ]; then 
            curl -LO "${URL}/setup_2^${power}.key"
        fi
done
cd .. && cd ..

# ----------------------------------

echo "> Check submodule update"

git submodule init
git submodule update

# ----------------------------------

echo "> Compiling contracts"
make compile

# ----------------------------------

echo "> Drop postgress db"
cargo sqlx database drop -y

# ----------------------------------

echo "> Setup postgres db"
cd core/lib/dal
DATABASE_URL="postgres://postgres@localhost/zksync_local"

# ----------------------------------

echo "Using localhost database:"
echo $DATABASE_URL
cargo sqlx database create
cargo sqlx migrate run
cargo sqlx prepare --check -- --tests || cargo sqlx prepare -- --tests

# ----------------------------------

echo "> Clean rocksdb"
rm -rf db
echo "Successfully removed db/"

# ----------------------------------

echo "> Clean backups"
rm -rf backups
echo "Successfully removed backups/"

# ----------------------------------

echo "> Building contracts"
yarn l1-contracts build
yarn l2-contracts build

# ----------------------------------

echo "> Deploying localhost ERC20 tokens"
yarn --silent --cwd contracts/ethereum deploy-erc20 add-multi '
            [
                { "name": "DAI",  "symbol": "DAI",  "decimals": 18 },
                { "name": "wBTC", "symbol": "wBTC", "decimals":  8, "implementation": "RevertTransferERC20" },
                { "name": "BAT",  "symbol": "BAT",  "decimals": 18 },
                { "name": "GNT",  "symbol": "GNT",  "decimals": 18 },
                { "name": "MLTT", "symbol": "MLTT", "decimals": 18 },
                { "name": "DAIK",  "symbol": "DAIK",  "decimals": 18 },
                { "name": "wBTCK", "symbol": "wBTCK", "decimals":  8, "implementation": "RevertTransferERC20" },
                { "name": "BATK",  "symbol": "BATS",  "decimals": 18 },
                { "name": "GNTK",  "symbol": "GNTS",  "decimals": 18 },
                { "name": "MLTTK", "symbol": "MLTTS", "decimals": 18 },
                { "name": "DAIL",  "symbol": "DAIL",  "decimals": 18 },
                { "name": "wBTCL", "symbol": "wBTCP", "decimals":  8, "implementation": "RevertTransferERC20" },
                { "name": "BATL",  "symbol": "BATW",  "decimals": 18 },
                { "name": "GNTL",  "symbol": "GNTW",  "decimals": 18 },
                { "name": "MLTTL", "symbol": "MLTTW", "decimals": 18 },
                { "name": "Wrapped Ether", "symbol": "WETH", "decimals": 18, "implementation": "WETH9"}
            ]' > etc/tokens/localhost.json