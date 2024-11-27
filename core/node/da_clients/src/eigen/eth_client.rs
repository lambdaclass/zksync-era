use bytes::Bytes;
use ethereum_types::{Address, U256};
/// This client is inspired in ethrex EthClient https://github.com/lambdaclass/ethrex/blob/main/crates/l2/utils/eth_client/mod.rs
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, thiserror::Error)]
pub enum EthClientError {
    #[error("Failed to serialize request body: {0}")]
    FailedToSerializeRequestBody(String),
    #[error("reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("{0}")]
    SerdeJSONError(#[from] serde_json::Error),
    #[error("{0}")]
    RPCError(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RpcRequestId {
    Number(u64),
    String(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcSuccessResponse {
    pub id: RpcRequestId,
    pub jsonrpc: String,
    pub result: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcErrorMetadata {
    pub code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcErrorResponse {
    pub id: RpcRequestId,
    pub jsonrpc: String,
    pub error: RpcErrorMetadata,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum RpcResponse {
    Success(RpcSuccessResponse),
    Error(RpcErrorResponse),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcRequest {
    pub id: RpcRequestId,
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Vec<Value>>,
}

#[derive(Debug, Clone)]
pub struct EthClient {
    client: Client,
    pub url: String,
}

impl EthClient {
    pub fn new(url: &str) -> Self {
        Self {
            client: Client::new(),
            url: url.to_string(),
        }
    }

    async fn send_request(&self, request: RpcRequest) -> Result<RpcResponse, EthClientError> {
        self.client
            .post(&self.url)
            .header("content-type", "application/json")
            .body(serde_json::ser::to_string(&request).map_err(|error| {
                EthClientError::FailedToSerializeRequestBody(format!("{error}: {request:?}"))
            })?)
            .send()
            .await?
            .json::<RpcResponse>()
            .await
            .map_err(EthClientError::from)
    }

    pub async fn get_block_number(&self) -> Result<U256, EthClientError> {
        let request = RpcRequest {
            id: RpcRequestId::Number(1),
            jsonrpc: "2.0".to_string(),
            method: "eth_blockNumber".to_string(),
            params: None,
        };

        match self.send_request(request).await {
            Ok(RpcResponse::Success(result)) => {
                serde_json::from_value(result.result).map_err(EthClientError::SerdeJSONError)
            }
            Ok(RpcResponse::Error(error_response)) => {
                Err(EthClientError::RPCError(error_response.error.message))
            }
            Err(error) => Err(error),
        }
    }

    pub async fn call(
        &self,
        to: Address,
        calldata: Bytes,
        block: Option<u64>,
    ) -> Result<String, EthClientError> {
        let request = RpcRequest {
            id: RpcRequestId::Number(1),
            jsonrpc: "2.0".to_string(),
            method: "eth_call".to_string(),
            params: Some(vec![
                json!({
                    "to": format!("{:#x}",to),
                    "input": format!("0x{:#x}", calldata),
                    "value": format!("{:#x}", 0),
                    "from": format!("{:#x}", Address::zero()),
                }),
                json!(match block {
                    Some(block) => format!("{:#x}", block),
                    None => "latest".to_string(),
                }),
            ]),
        };

        match self.send_request(request).await {
            Ok(RpcResponse::Success(result)) => {
                serde_json::from_value(result.result).map_err(EthClientError::SerdeJSONError)
            }
            Ok(RpcResponse::Error(error_response)) => {
                Err(EthClientError::RPCError(error_response.error.message))
            }
            Err(error) => Err(error),
        }
    }
}
