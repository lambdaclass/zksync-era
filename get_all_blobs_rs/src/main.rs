use std::{collections::HashMap, error::Error, fs, str::FromStr};

use alloy::{
    json_abi::{Function, InternalType, JsonAbi, Param},
    network::Ethereum,
    primitives::{Address, Bytes, U256},
    providers::{Provider, ProviderBuilder, RootProvider},
    rpc::types::{Block, Transaction},
};
use client::EigenClientRetriever;
use futures::StreamExt;
use reqwest;
use serde::{Deserialize, Serialize};

mod blob_info;
mod client;
mod generated;

#[derive(Debug, Serialize, Deserialize)]
struct BlobData {
    commitment: String,
    blob: String,
}

#[derive(Debug)]
struct StoredBatchInfo {
    batch_number: u64,
    batch_hash: [u8; 32],
    index_repeated_storage_changes: u64,
    number_of_layer1_txs: U256,
    priority_operations_hash: [u8; 32],
    l2_logs_tree_root: [u8; 32],
    timestamp: U256,
    commitment: [u8; 32],
}

#[derive(Debug)]
struct CommitBatchInfo {
    batch_number: u64,
    timestamp: u64,
    index_repeated_storage_changes: u64,
    new_state_root: [u8; 32],
    number_of_layer1_txs: U256,
    priority_operations_hash: [u8; 32],
    bootloader_heap_initial_contents_hash: [u8; 32],
    events_queue_state_hash: [u8; 32],
    system_logs: Bytes,
    pubdata_commitments: Bytes,
}

const EIGENDA_API_URL: &str = "https://disperser-holesky.eigenda.xyz:443";

async fn get_blob(commitment: &str) -> anyhow::Result<Vec<u8>> {
    let client = EigenClientRetriever::new(EIGENDA_API_URL).await.unwrap();
    let data = client
        .get_blob_data(&commitment)
        .await
        .unwrap()
        .unwrap_or_default(); // TODO: Remove unwrap
    Ok(data)
}

fn hex_to_utf8(hex: &str) -> Result<String, Box<dyn Error>> {
    let hex = hex.trim_start_matches("0x");
    let bytes = hex::decode(hex)?;
    String::from_utf8(bytes).map_err(|e| e.into())
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

async fn get_transactions(
    provider: &RootProvider<
        alloy::transports::http::Http<alloy::transports::http::Client>,
        Ethereum,
    >,
    validator_timelock_address: Address,
    commit_batches_selector: &str,
) -> Result<(), Box<dyn Error>> {
    let latest_block = provider.get_block_number().await?;
    let mut json_array = Vec::new();

    // Define ABI for the function
    // let abi = JsonAbi::new(vec![Function {
    //     name: "commitBatchesSharedBridge".to_string(),
    //     inputs: vec![
    //         Param {
    //             name: "_chainId".to_string(),
    //             ty: "uint256".to_string(),
    //             components: vec![],
    //             internal_type: Some(InternalType::Uint(256)),
    //         },
    //         // Add other parameters as needed
    //         // This is simplified for brevity
    //     ],
    //     outputs: vec![],
    //     state_mutability: "nonpayable".to_string(),
    // }]);

    for block_number in 0..=latest_block {
        if let Ok(Some(block)) = provider
            .get_block_by_number(block_number.into(), true)
            .await
        {
            for tx in block.transactions.into_transactions() {
                if let Some(to) = tx.to {
                    if to == validator_timelock_address {
                        println!("Validator timelock match!");
                        // if let Some(input) = tx.input {
                        let input = tx.input;
                        let selector = &input[0..4];
                        if selector == hex::decode(commit_batches_selector).unwrap() {
                            println!("Commit batches selector match!");
                            // Decode parameters
                            // Note: This is simplified. You'll need to implement proper ABI decoding
                            if let Ok(decoded) = decode_input(&input[4..]) {
                                let commitment =
                                    hex::decode(&decoded.pubdata_commitments[4..]).unwrap();
                                let commitment = hex::encode(&commitment);
                                let blob = get_blob(&commitment).await?;

                                json_array.push(BlobData {
                                    commitment,
                                    blob: bytes_to_hex(&blob),
                                });
                            }
                        }
                        // }
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

// This is a placeholder for actual ABI decoding implementation
fn decode_input(input: &[u8]) -> Result<CommitBatchInfo, Box<dyn Error>> {
    // Implement proper ABI decoding here
    // This is just a placeholder
    todo!("Implement proper ABI decoding")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // let args: Vec<String> = std::env::args().collect();

    // if args.len() != 3 {
    //     eprintln!("Usage: {} validatorTimelockAddress=<address> commitBatchesSharedBridge_functionSelector=<selector>", args[0]);
    //     std::process::exit(1);
    // }

    let validator_timelock_address =
        Address::from_str("0x95af79aAB990f9740c029013ef18f3D3d666B4e8")?;
    let commit_batches_selector = "6edd4f12";

    let url = alloy::transports::http::reqwest::Url::from_str(&"http://127.0.0.1:8545").unwrap();
    let provider: RootProvider<
        alloy::transports::http::Http<alloy::transports::http::Client>,
        Ethereum,
    > = RootProvider::new_http(url);

    get_transactions(
        &provider,
        validator_timelock_address,
        commit_batches_selector,
    )
    .await?;

    Ok(())
}
