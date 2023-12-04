cd $ZKSYNC_HOME
yarn && yarn zk build

echo "> Pulling images"
docker-compose pull 

echo "> Setting up containers"
docker-compose up -d geth postgres

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

echo "> Compiling contracts"
make compile

echo "> Drop postgress db"
cargo sqlx database drop -y

echo "> Setup postgres db"
cd core/lib/dal
DATABASE_URL="postgres://postgres@localhost/zksync_local"

echo "Using localhost database:"
echo $DATABASE_URL
cargo sqlx database create
cargo sqlx migrate run
cargo sqlx prepare --check -- --tests || cargo sqlx prepare -- --tests

echo "> Clean rocksdb"
rm -rf db
echo "Successfully removed db/"

echo "> Clean backups"
rm -rf backups
echo "Successfully removed backups/"

echo "> Building contracts"
yarn l1-contracts build
yarn l2-contracts build

echo "> Deploying localhost ERC20 tokens"
