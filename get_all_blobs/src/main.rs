use std::{fs, str::FromStr};

use alloy::{
    dyn_abi::JsonAbiExt,
    json_abi::JsonAbi,
    network::Ethereum,
    primitives::Address,
    providers::{Provider, RootProvider},
};
use client::EigenClientRetriever;
use serde::{Deserialize, Serialize};

mod blob_info;
mod client;
mod generated;

#[derive(Debug, Serialize, Deserialize)]
struct BlobData {
    pub commitment: String,
    pub blob: String,
}

const EIGENDA_API_URL: &str = "https://disperser-holesky.eigenda.xyz:443";

async fn get_blob(commitment: &str) -> anyhow::Result<Vec<u8>> {
    let client = EigenClientRetriever::new(EIGENDA_API_URL).await?;
    let data = client
        .get_blob_data(&commitment)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Blob not found"))?;
    Ok(data)
}

async fn get_transactions(
    provider: &RootProvider<
        alloy::transports::http::Http<alloy::transports::http::Client>,
        Ethereum,
    >,
    validator_timelock_address: Address,
    commit_batches_selector: &str,
    block_start: u64,
) -> anyhow::Result<()> {
    let latest_block = provider.get_block_number().await?;
    let mut json_array = Vec::new();

    for block_number in block_start..=latest_block {
        if let Ok(Some(block)) = provider
            .get_block_by_number(block_number.into(), true)
            .await
        {
            for tx in block.transactions.into_transactions() {
                if let Some(to) = tx.to {
                    if to == validator_timelock_address {
                        let input = tx.input;
                        let selector = &input[0..4];
                        if selector == hex::decode(commit_batches_selector)? {
                            if let Ok(decoded) = decode_blob_data_input(&input[4..]).await {
                                json_array.push(decoded);
                            }
                        }
                    }
                }
            }
        }
    }

    if json_array.is_empty() {
        println!("\x1b[31mNo transactions found.\x1b[0m");
        return Ok(());
    }

    let json_string = serde_json::to_string_pretty(&json_array)?;
    fs::write("blob_data.json", json_string)?;
    println!("\x1b[32mData stored in blob_data.json file.\x1b[0m");

    Ok(())
}

async fn decode_blob_data_input(input: &[u8]) -> anyhow::Result<BlobData> {
    let path = "./abi/commitBatchesSharedBridge.json";
    let json = std::fs::read_to_string(path)?;
    let json_abi: JsonAbi = serde_json::from_str(&json)?;
    let function = json_abi
        .functions
        .iter()
        .find(|f| f.0 == "commitBatchesSharedBridge")
        .ok_or(anyhow::anyhow!("Function not found"))?
        .1;

    let decoded = function[0].abi_decode_input(input, true)?;
    let commit_batch_info = decoded[2].as_array().ok_or(anyhow::anyhow!(
        "CommitBatchInfo cannot be represented as an array"
    ))?[0]
        .as_tuple()
        .ok_or(anyhow::anyhow!(
            "CommitBatchInfo components cannot be represented as a tuple"
        ))?;
    let pubdata_commitments = commit_batch_info.last().ok_or(anyhow::anyhow!(
        "pubdata_commitments not found in commitBatchesSharedBridge input"
    ))?;
    let pubdata_commitments_bytes = pubdata_commitments
        .as_bytes()
        .ok_or(anyhow::anyhow!("pubdata_commitments is not a bytes array"))?;

    let commitment = hex::decode(&pubdata_commitments_bytes[1..])?;
    let commitment = hex::encode(&commitment);
    let blob = get_blob(&commitment).await?;

    Ok(BlobData {
        commitment,
        blob: hex::encode(blob),
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 4 {
        eprintln!("Usage: cargo run <validatorTimelockAddress> <rpc_url> <block_start>");
        std::process::exit(1);
    }

    let validator_timelock_address = Address::from_str(&args[1])?;

    let commit_batches_selector = "6edd4f12";

    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let url = alloy::transports::http::reqwest::Url::from_str(&args[2])?;
    let provider: RootProvider<
        alloy::transports::http::Http<alloy::transports::http::Client>,
        Ethereum,
    > = RootProvider::new_http(url);

    let block_start = args[3].parse::<u64>()?;

    get_transactions(
        &provider,
        validator_timelock_address,
        commit_batches_selector,
        block_start,
    )
    .await?;

    Ok(())
}
