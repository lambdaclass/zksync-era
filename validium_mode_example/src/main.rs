use std::{str::FromStr, time::Duration};

use colored::Colorize;
use ethers::{
    abi::Abi, core::k256::ecdsa::SigningKey, providers::Http, types::TransactionReceipt,
    utils::parse_units,
};
use loadnext::config::LoadtestConfig;
use tokio::time::sleep;
use zksync_types::{api::TransactionDetails, H160, H256, U256};
use zksync_web3_decl::{jsonrpsee::http_client::HttpClientBuilder, namespaces::ZksNamespaceClient};
use zksync_web3_rs::{
    eip712::Eip712TransactionRequest,
    providers::{Middleware, Provider},
    signers::{LocalWallet, Signer},
    zks_provider::ZKSProvider,
    zks_wallet::{DeployRequest, DepositRequest},
    ZKSWallet,
};

static ERA_PROVIDER_URL: &str = "http://127.0.0.1:3050";
static PRIVATE_KEY: &str = "7726827caac94a7f9e1b160f7ea819f172f7b6f9d2a97f992c38edeab82d4110";

static CONTRACT_BIN: &str = include_str!("../ERC20.bin");
static CONTRACT_ABI: &str = include_str!("../ERC20.abi");

static L1_URL: &str = "http://localhost:8545";

fn l1_provider() -> Provider<Http> {
    Provider::<Http>::try_from(L1_URL).expect("Could not instantiate L1 Provider")
}

fn l2_provider() -> Provider<Http> {
    Provider::try_from(ERA_PROVIDER_URL).unwrap()
}

async fn zks_wallet(
    l1_provider: &Provider<Http>,
    l2_provider: &Provider<Http>,
) -> ZKSWallet<Provider<Http>, SigningKey> {
    let chain_id = l2_provider.get_chainid().await.unwrap();
    let l2_wallet = LocalWallet::from_str(PRIVATE_KEY)
        .unwrap()
        .with_chain_id(chain_id.as_u64());
    ZKSWallet::new(
        l2_wallet,
        None,
        Some(l2_provider.clone()),
        Some(l1_provider.clone()),
    )
    .unwrap()
}

async fn deposit(zks_wallet: &ZKSWallet<Provider<Http>, SigningKey>) -> H256 {
    let amount = parse_units("11", "ether").unwrap();
    let request = DepositRequest::new(amount.into());
    zks_wallet
        .deposit(&request)
        .await
        .expect("Failed to perform deposit transaction")
}

async fn deploy(zks_wallet: &ZKSWallet<Provider<Http>, SigningKey>) -> (H160, TransactionReceipt) {
    println!("{}", "Deploy".bright_magenta());

    // Read both files from disk:
    let abi = Abi::load(CONTRACT_ABI.as_bytes()).unwrap();
    let contract_bin = hex::decode(CONTRACT_BIN).unwrap().to_vec();

    // DeployRequest sets the parameters for the constructor call and the deployment transaction.
    let request = DeployRequest::with(
        abi,
        contract_bin,
        vec!["ToniToken".to_owned(), "teth".to_owned()],
    )
    .from(zks_wallet.l2_address());

    let eip712_request: Eip712TransactionRequest = request.clone().try_into().unwrap();

    let l2_deploy_tx_receipt = zks_wallet
        .get_era_provider()
        .unwrap()
        .clone()
        .send_transaction_eip712(&zks_wallet.l2_wallet, eip712_request)
        .await
        .unwrap()
        .await
        .unwrap()
        .unwrap();

    (
        l2_deploy_tx_receipt.contract_address.unwrap(),
        l2_deploy_tx_receipt,
    )
}

async fn mint(
    zks_wallet: &ZKSWallet<Provider<Http>, SigningKey>,
    erc20_address: H160,
) -> TransactionReceipt {
    println!("{}", "Mint".bright_magenta());

    zks_wallet
        .get_era_provider()
        .unwrap()
        .clone()
        .send_eip712(
            &zks_wallet.l2_wallet,
            erc20_address,
            "_mint(address, uint256)",
            Some(
                [
                    "CD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826".into(),
                    "100000".into(),
                ]
                .into(),
            ),
            None,
        )
        .await
        .unwrap()
        .await
        .unwrap()
        .unwrap()
}

async fn transfer(
    zks_wallet: &ZKSWallet<Provider<Http>, SigningKey>,
    erc20_address: H160,
) -> TransactionReceipt {
    println!("{}", "Transfer".bright_magenta());

    zks_wallet
        .get_era_provider()
        .unwrap()
        .clone()
        .send_eip712(
            &zks_wallet.l2_wallet,
            erc20_address,
            "_transfer(address, address, uint256)",
            Some(
                [
                    "CD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826".into(),
                    "bBbBBBBbbBBBbbbBbbBbbbbBBbBbbbbBbBbbBBbB".into(),
                    "1000".into(),
                ]
                .into(),
            ),
            None,
        )
        .await
        .unwrap()
        .await
        .unwrap()
        .unwrap()
}

async fn wait_for_l2_tx_details(tx_hash: H256) -> TransactionDetails {
    let config = LoadtestConfig::from_env()
        .expect("Config parameters should be loaded from env or from default values");

    let client = HttpClientBuilder::default()
        .build(config.l2_rpc_address)
        .unwrap();
    loop {
        let details = client
            .get_transaction_details(tx_hash)
            .await
            .unwrap()
            .unwrap();

        if details.eth_commit_tx_hash.is_some()
            && details.eth_prove_tx_hash.is_some()
            && details.eth_execute_tx_hash.is_some()
        {
            break details;
        }

        sleep(Duration::from_secs(1)).await;
    }
}

async fn wait_for_tx_receipt(client: &Provider<Http>, tx_hash: H256) -> TransactionReceipt {
    loop {
        let receipt = client
            .get_transaction_receipt(tx_hash.clone())
            .await
            .unwrap();
        if receipt.is_some() {
            break receipt.unwrap();
        }
        sleep(Duration::from_secs(1)).await;
    }
}

async fn tx_gas_used(client: &Provider<Http>, tx_hash: H256) -> U256 {
    let receipt = wait_for_tx_receipt(client, tx_hash).await;
    receipt.gas_used.unwrap()
}

async fn display_gas_and_hash(name: &str, client: &Provider<Http>, tx_hash: H256) {
    let gas_used = tx_gas_used(client, tx_hash).await;
    println!("{}", name.bright_red());
    println!(
        "Hash: {formatted_hash}",
        formatted_hash = format!("{tx_hash:#?}").bright_green()
    );
    println!(
        "Gas used: {formatted_gas_used}",
        formatted_gas_used = format!("{gas_used:#?}").bright_cyan()
    );
}

async fn display_tx_details(
    l1_provider: &Provider<Http>,
    l2_provider: &Provider<Http>,
    tx_receipt: TransactionReceipt,
) {
    let l2_deploy_tx_details = wait_for_l2_tx_details(tx_receipt.transaction_hash).await;

    display_gas_and_hash("L2 tx", &l2_provider, tx_receipt.transaction_hash).await;
    display_gas_and_hash(
        "L1 commit tx",
        &l1_provider,
        l2_deploy_tx_details.eth_commit_tx_hash.unwrap(),
    )
    .await;
    display_gas_and_hash(
        "L1 prove tx",
        &l1_provider,
        l2_deploy_tx_details.eth_prove_tx_hash.unwrap(),
    )
    .await;
    display_gas_and_hash(
        "L1 execute tx",
        &l1_provider,
        l2_deploy_tx_details.eth_execute_tx_hash.unwrap(),
    )
    .await;

    println!();
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let l1_provider = l1_provider();
    let l2_provider = l2_provider();
    let zks_wallet = zks_wallet(&l1_provider, &l2_provider).await;

    // Fund the wallet
    let _deposit_tx_hash = deposit(&zks_wallet).await;

    // Deploy ERC20 contract
    let (l2_deployed_contract_address, l2_deploy_tx_receipt) = deploy(&zks_wallet).await;
    display_tx_details(&l1_provider, &l2_provider, l2_deploy_tx_receipt).await;

    // Mint tokens
    let l2_mint_tx_receipt = mint(&zks_wallet, l2_deployed_contract_address).await;
    display_tx_details(&l1_provider, &l2_provider, l2_mint_tx_receipt).await;

    // Transfer tokens
    let l2_transfer_tx_receipt = transfer(&zks_wallet, l2_deployed_contract_address).await;
    display_tx_details(&l1_provider, &l2_provider, l2_transfer_tx_receipt).await;
}
