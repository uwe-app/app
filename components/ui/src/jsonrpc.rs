use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send>>;

const JSONRPC_VERSION: &str = "2.0";
const JSONRPC_PARSE_ERROR: isize = -32700;
const JSONRPC_INVALID_REQUEST: isize = -32600;

/// Box an error implementation.
pub fn box_error(e: impl std::error::Error + Send + 'static) -> Box<dyn std::error::Error + Send> {
    let err: Box<dyn std::error::Error + Send> = Box::new(e);
    err
}

fn is_null(value: &Value) -> bool {
    if let Value::Null = value {
        true
    } else {
        false
    }
}

/// Trait for service implementations.
pub trait Service {
    fn handle(&self, req: &JsonRpcRequest) -> Result<Option<JsonRpcResponse>>;
}

/// Broker calls multiple services and always yields a response.
pub struct Broker;
impl Broker {
    pub fn handle<'a>(
        &self,
        services: &'a Vec<&'a Box<dyn Service>>,
        req: &JsonRpcRequest,
    ) -> Result<JsonRpcResponse> {
        for service in services {
            if let Some(result) = service.handle(req)? {
                return Ok(result);
            }
        }

        //// TODO: handle method not found!!!
        Ok(JsonRpcResponse::response(&req, None))
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonRpcRequest {
    jsonrpc: String,
    id: Value,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    pub fn id(&self) -> &Value {
        &self.id
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn matches(&self, name: &str) -> bool {
        name == &self.method
    }
}

/*
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
*/

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
    /// Reply to a request with a result.
    pub fn response(req: &JsonRpcRequest, result: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result,
            error: None,
            id: req.id.clone(),
        }
    }

    /// Reply to a request with no result or error.
    pub fn reply(req: &JsonRpcRequest) -> Self {
        JsonRpcResponse::response(req, None)
    }

    /*
    pub fn result(id: Value, result: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result,
            error: None,
            id,
        }
    }
    */

    /// Reply to a request with an error.
    pub fn error(message: String, code: usize, id: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data: None,
            }),
        }
    }
}
