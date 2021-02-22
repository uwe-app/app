use crate::Error;
use rand::{self, Rng};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{Number, Value};

//type Result<T> = std::result::Result<T, Error>;

const JSONRPC_VERSION: &str = "2.0";
const JSONRPC_PARSE_ERROR: isize = -32700;
const JSONRPC_INVALID_REQUEST: isize = -32600;

fn is_null(value: &Value) -> bool {
    if let Value::Null = value {
        true
    } else {
        false
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonRpcRequest {
    jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    pub fn new(method: &str, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.to_string(),
            params,
            id: Value::Number(Number::from(
                rand::thread_rng().gen_range(0..std::u32::MAX) + 1,
            )),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "is_null")]
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

impl Default for JsonRpcResponse {
    fn default() -> Self {
        JsonRpcResponse::error(
            "Unknown service method".to_string(),
            0,
            Value::Null,
        )
    }
}

impl From<u32> for JsonRpcResponse {
    fn from(id: u32) -> Self {
        Self {
            id: Value::Number(Number::from(id)),
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: None,
            error: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcError {
    code: usize,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<String>,
}

impl JsonRpcResponse {
    pub fn response(req: &JsonRpcRequest, result: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result,
            error: None,
            id: req.id.clone(),
        }
    }

    pub fn result(id: Value, result: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result,
            error: None,
            id,
        }
    }

    // Construct a JsonRpcResponse containing an error response
    pub fn error(message: String, code: usize, id: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data: None,
            }),
            id,
        }
    }
}
