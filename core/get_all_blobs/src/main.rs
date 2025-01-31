use std::{fs, str::FromStr};

use alloy::{
    dyn_abi::{DynSolValue, JsonAbiExt},
    json_abi::JsonAbi,
    network::Ethereum,
    primitives::Address,
    providers::{Provider, RootProvider},
};
use blob_info::{
    BatchHeader, BatchMetadata, BlobHeader, BlobQuorumParam, BlobVerificationProof, G1Commitment,
};
use client::EigenClientRetriever;
use ethabi::{ParamType, Token};
use serde::{Deserialize, Serialize};

use crate::blob_info::BlobInfo;

mod blob_info;
mod client;
mod generated;

#[derive(Debug, Serialize, Deserialize)]
struct BlobData {
    pub blob_info: String,
    pub blob: String,
}

const EIGENDA_API_URL: &str = "https://disperser-holesky.eigenda.xyz:443";
const BLOB_DATA_JSON: &str = "blob_data.json";
const ABI_JSON: &str = "./abi/commitBatchesSharedBridge.json";
const COMMIT_BATCHES_SELECTOR: &str = "98f81962";

async fn get_blob(blob_info: BlobInfo) -> anyhow::Result<Vec<u8>> {
    let client = EigenClientRetriever::new(EIGENDA_API_URL).await?;
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
        ParamType::Tuple(vec![
            ParamType::Uint(64),
            ParamType::FixedBytes(32),
            ParamType::Uint(64),
            ParamType::Uint(256),
            ParamType::FixedBytes(32),
            ParamType::FixedBytes(32),
            ParamType::Uint(256),
            ParamType::FixedBytes(32),
        ]), // StoredBatchInfo
        ParamType::Array(Box::new(ParamType::Tuple(vec![
            ParamType::Uint(64),
            ParamType::Uint(64),
            ParamType::Uint(64),
            ParamType::FixedBytes(32),
            ParamType::Uint(64),
            ParamType::FixedBytes(32),
            ParamType::FixedBytes(32),
            ParamType::FixedBytes(32),
            ParamType::Bytes,
            ParamType::Bytes,
        ]))), // CommitBatchInfo
    ];

    let decoded = ethabi::decode(&param_types, &commit_data[1..])?;

    let commit_batch_info = match &decoded[1] {
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
                        if let Ok(blob_data) =
                            get_blob_from_operator_da_input(operator_da_input).await
                        {
                            blobs.push(blob_data)
                        }
                    }
                    _ => return Err(anyhow::anyhow!("Operator DA input is not bytes")),
                }
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "CommitBatchInfo components cannot be represented as a tuple"
                ))
            }
        }
    }

    Ok(blobs)
}

/// Helper functions for safe extraction
fn extract_tuple(token: &Token) -> anyhow::Result<&Vec<Token>> {
    match token {
        Token::Tuple(inner) => Ok(inner),
        _ => Err(anyhow::anyhow!("Not a tuple")),
    }
}

fn extract_array(token: &Token) -> anyhow::Result<Vec<Token>> {
    match token {
        Token::Array(tokens) => Ok(tokens.clone()),
        _ => Err(anyhow::anyhow!("Not a uint")),
    }
}

fn extract_uint(token: &Token) -> anyhow::Result<u32> {
    match token {
        Token::Uint(value) => Ok(value.as_u32()),
        _ => Err(anyhow::anyhow!("Not a uint")),
    }
}

fn extract_fixed_bytes<const N: usize>(token: &Token) -> anyhow::Result<Vec<u8>> {
    match token {
        Token::FixedBytes(bytes) => Ok(bytes.clone()),
        _ => Err(anyhow::anyhow!("Not fixed bytes")),
    }
}

fn extract_bytes(token: &Token) -> anyhow::Result<Vec<u8>> {
    match token {
        Token::Bytes(bytes) => Ok(bytes.clone()),
        _ => Err(anyhow::anyhow!("Not bytes")),
    }
}

async fn get_blob_from_operator_da_input(operator_da_input: Vec<u8>) -> anyhow::Result<BlobData> {
    let param_types = vec![ParamType::Tuple(vec![
        // BlobHeader
        ParamType::Tuple(vec![
            ParamType::Tuple(vec![ParamType::Uint(256), ParamType::Uint(256)]), // G1Commitment
            ParamType::Uint(32),                                                // data_length
            ParamType::Array(Box::new(ParamType::Tuple(vec![
                ParamType::Uint(32),
                ParamType::Uint(32),
                ParamType::Uint(32),
                ParamType::Uint(32),
            ]))), // BlobQuorumParam
        ]),
        // BlobVerificationProof
        ParamType::Tuple(vec![
            ParamType::Uint(32), // batch_id
            ParamType::Uint(32), // blob_index
            ParamType::Tuple(vec![
                ParamType::Tuple(vec![
                    ParamType::FixedBytes(32),
                    ParamType::Bytes,
                    ParamType::Bytes,
                    ParamType::Uint(32),
                ]), // BatchHeader
                ParamType::FixedBytes(32), // signatory_record_hash
                ParamType::Uint(32),       // confirmation_block_number
                ParamType::Bytes,          // batch_header_hash
                ParamType::Bytes,          // fee
            ]), // BatchMetadata
            ParamType::Bytes,    // inclusion_proof
            ParamType::Bytes,    // quorum_indexes
        ]),
    ])];

    let decoded = ethabi::decode(&param_types, &operator_da_input[32..])?;
    let blob_info = extract_tuple(&decoded[0])?;

    // Extract BlobHeader
    let blob_header_tokens = extract_tuple(&blob_info[0])?;
    let commitment_tokens = extract_tuple(&blob_header_tokens[0])?;

    let x = commitment_tokens[0].clone().into_uint().unwrap();
    let y = commitment_tokens[1].clone().into_uint().unwrap();

    let mut x_bytes = vec![0u8; 32];
    let mut y_bytes = vec![0u8; 32];
    x.to_big_endian(&mut x_bytes);
    y.to_big_endian(&mut y_bytes);

    let data_length = extract_uint(&blob_header_tokens[1])?;
    let blob_quorum_params_tokens = extract_array(&blob_header_tokens[2])?;

    let blob_quorum_params: Vec<BlobQuorumParam> = blob_quorum_params_tokens
        .iter()
        .map(|param| {
            let tuple = extract_tuple(param)?;
            Ok(BlobQuorumParam {
                quorum_number: extract_uint(&tuple[0])?,
                adversary_threshold_percentage: extract_uint(&tuple[1])?,
                confirmation_threshold_percentage: extract_uint(&tuple[2])?,
                chunk_length: extract_uint(&tuple[3])?,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let blob_header = BlobHeader {
        commitment: G1Commitment {
            x: x_bytes,
            y: y_bytes,
        },
        data_length,
        blob_quorum_params,
    };

    // Extract BlobVerificationProof
    let blob_verification_tokens = extract_tuple(&blob_info[1])?;

    let batch_id = extract_uint(&blob_verification_tokens[0])?;
    let blob_index = extract_uint(&blob_verification_tokens[1])?;


    let batch_metadata_tokens = extract_tuple(&blob_verification_tokens[2])?;
    let batch_header_tokens = extract_tuple(&batch_metadata_tokens[0])?;

    let batch_header = BatchHeader {
        batch_root: extract_fixed_bytes::<32>(&batch_header_tokens[0])?,
        quorum_numbers: extract_bytes(&batch_header_tokens[1])?,
        quorum_signed_percentages: extract_bytes(&batch_header_tokens[2])?,
        reference_block_number: extract_uint(&batch_header_tokens[3])?,
    };

    let batch_metadata = BatchMetadata {
        batch_header,
        signatory_record_hash: extract_fixed_bytes::<32>(&batch_metadata_tokens[1])?,
        confirmation_block_number: extract_uint(&batch_metadata_tokens[2])?,
        batch_header_hash: extract_bytes(&batch_metadata_tokens[3])?,
        fee: extract_bytes(&batch_metadata_tokens[4])?,
    };

    let blob_verification_proof = BlobVerificationProof {
        batch_id,
        blob_index,
        batch_metadata,
        inclusion_proof: extract_bytes(&blob_verification_tokens[3])?,
        quorum_indexes: extract_bytes(&blob_verification_tokens[4])?,
    };

    let blob_info = BlobInfo {
        blob_header,
        blob_verification_proof,
    };
    let blob = get_blob(blob_info).await?;

    Ok(BlobData {
        blob_info: blob_index.to_string(),
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
