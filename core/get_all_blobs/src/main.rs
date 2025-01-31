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
use ethabi::{ParamType, Token};
use alloy::dyn_abi::{DynSolValue};

mod blob_info;
mod client;
mod generated;

#[derive(Debug, Serialize, Deserialize)]
struct BlobData {
    pub blob_id: String,
    pub blob: String,
}

const EIGENDA_API_URL: &str = "https://disperser-holesky.eigenda.xyz:443";
const BLOB_DATA_JSON: &str = "blob_data.json";
const ABI_JSON: &str = "./abi/commitBatchesSharedBridge.json";
const COMMIT_BATCHES_SELECTOR: &str = "98f81962";

async fn get_blob(blob_id: &str) -> anyhow::Result<Vec<u8>> {
    let client = EigenClientRetriever::new(EIGENDA_API_URL).await?;
    let blob_info = client
        .get_blob_status(blob_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Blob not found"))?;
    let data = client
        .get_blob_data(blob_info)
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
    block_start: u64,
) -> anyhow::Result<()> {
    let latest_block = provider.get_block_number().await?;
    let mut json_array = Vec::new();

    let mut i = 0;
    for block_number in block_start..=latest_block {
        i += 1;
        if i % 50 == 0 {
            println!(
                "\x1b[32mProcessed up to block {} of {}\x1b[0m",
                block_number, latest_block
            );
        }
        if let Ok(Some(block)) = provider
            .get_block_by_number(block_number.into(), true)
            .await
        {
            for tx in block.transactions.into_transactions() {
                if let Some(to) = tx.to {
                    if to == validator_timelock_address {
                        let input = tx.clone().input;
                        let selector = &input[0..4];
                        if selector == hex::decode(COMMIT_BATCHES_SELECTOR)? {
                            if let Ok(decoded) = decode_blob_data_input(&input[4..]).await {
                                for blob in decoded {
                                    json_array.push(blob);
                                }
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
    fs::write(BLOB_DATA_JSON, json_string)?;
    println!("\x1b[32mData stored in blob_data.json file.\x1b[0m");

    Ok(())
}

async fn decode_blob_data_input(input: &[u8]) -> anyhow::Result<Vec<BlobData>> {
    let json = std::fs::read_to_string(ABI_JSON)?;
    let json_abi: JsonAbi = serde_json::from_str(&json)?;
    let function = json_abi
        .functions
        .iter()
        .find(|f| f.0 == "commitBatchesSharedBridge")
        .ok_or(anyhow::anyhow!("Function not found"))?
        .1;

    let decoded = function[0].abi_decode_input(input, true)?;
    let commit_data = &decoded[3];
    let commit_data = match commit_data {
        DynSolValue::Bytes(commit_data) => commit_data,
        _ => return Err(anyhow::anyhow!("Commit data is not bytes")),
    };

    let param_types = vec![
        ParamType::Tuple(vec![ParamType::Uint(64), ParamType::FixedBytes(32), ParamType::Uint(64), ParamType::Uint(256),ParamType::FixedBytes(32), ParamType::FixedBytes(32),ParamType::Uint(256),ParamType::FixedBytes(32)]), // StoredBatchInfo
        ParamType::Array(Box::new(
            ParamType::Tuple(vec![ParamType::Uint(64),ParamType::Uint(64),ParamType::Uint(64),ParamType::FixedBytes(32),ParamType::Uint(64),ParamType::FixedBytes(32),ParamType::FixedBytes(32),ParamType::FixedBytes(32),ParamType::Bytes,ParamType::Bytes ])
        )) // CommitBatchInfo
    ];

    let decoded = ethabi::decode(&param_types, &commit_data[1..])?;

    let commit_batch_info = match &decoded[1]{
        Token::Array(commit_batch_info) => commit_batch_info,
        _ => return Err(anyhow::anyhow!("CommitBatchInfo is not a tuple")),
    };
    let mut blobs = vec![];

    for batch_info in commit_batch_info {
        match batch_info {
            Token::Tuple(batch_info) => {
                let operator_da_input = batch_info[9].clone();
                match operator_da_input {
                    Token::Bytes(operator_da_input) => {
                        if let Ok(blob_data) = get_blob_from_operator_da_input(operator_da_input).await {
                            blobs.push(blob_data)
                        }
                    },
                    _ => return Err(anyhow::anyhow!("Operator DA input is not bytes")),
                }
                
            },
            _ => return Err(anyhow::anyhow!("CommitBatchInfo components cannot be represented as a tuple")),
        }
    }

    /*let commit_batch_info = commit_batch_info.as_array().ok_or(anyhow::anyhow!(
        "CommitBatchInfo cannot be represented as an array"
    ))?[0]
        .as_tuple()
        .ok_or(anyhow::anyhow!(
            "CommitBatchInfo components cannot be represented as a tuple"
        ))?;

    let mut blobs = vec![];

    for pubdata_commitments in commit_batch_info.iter() {
        let pubdata_commitments_bytes = pubdata_commitments.as_bytes();
        if let Ok(blob_data) = get_blob_from_pubdata_commitment(pubdata_commitments_bytes).await {
            blobs.push(blob_data)
        }
    }*/
    Ok(blobs)
}

async fn get_blob_from_operator_da_input(
    operator_da_input: Vec<u8>,
) -> anyhow::Result<BlobData> {
    let blob_id = hex::decode(&operator_da_input[32..])?; // First 32 bytes are state diff hash

    let blob_id = hex::encode(&blob_id);
    let blob = get_blob(&blob_id).await?;
    Ok(BlobData {
        blob_id,
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

    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let url = alloy::transports::http::reqwest::Url::from_str(&args[2])?;
    let provider: RootProvider<
        alloy::transports::http::Http<alloy::transports::http::Client>,
        Ethereum,
    > = RootProvider::new_http(url);

    let block_start = args[3].parse::<u64>()?;

    get_transactions(&provider, validator_timelock_address, block_start).await?;

    Ok(())
}
