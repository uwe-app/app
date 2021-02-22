use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Number, Value};

const VERSION: &str = "2.0";

const PARSE_ERROR: isize = -32700;
const INVALID_REQUEST: isize = -32600;
const METHOD_NOT_FOUND: isize = -32601;
const INVALID_PARAMS: isize = -32602;
const INTERNAL_ERROR: isize = -32603;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("JSON-RPC service method {name} not found (id: {id})")]
    MethodNotFound {name: String, id: Value},

    #[error("Parameters are invalid")]
    InvalidParams,

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Boxed(#[from] Box<dyn std::error::Error + Send>),
}

impl<'a> Into<JsonRpcResponse> for (&'a mut JsonRpcRequest, Error) {
    fn into(self) -> JsonRpcResponse {
        let code = match &self.1 {
            Error::MethodNotFound { .. } => METHOD_NOT_FOUND,
            Error::Json(_) => PARSE_ERROR,
            _ => INTERNAL_ERROR,
        };
        JsonRpcResponse::error(self.1.to_string(), code, self.0.id.clone())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcError {
    code: isize,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<String>,
}

/// Box an error implementation.
pub fn box_error(e: impl std::error::Error + Send + 'static) -> Error {
    let err: Box<dyn std::error::Error + Send> = Box::new(e);
    Error::from(err)
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
    fn handle(&self, req: &mut JsonRpcRequest) -> Result<Option<JsonRpcResponse>>;
}

/// Broker calls multiple services and always yields a response.
pub struct Broker;
impl Broker {
    pub fn handle<'a>(
        &self,
        services: &'a Vec<&'a Box<dyn Service>>,
        req: &mut JsonRpcRequest,
    ) -> Result<JsonRpcResponse> {
        for service in services {
            if let Some(result) = service.handle(req)? {
                return Ok(result);
            }
        }

        let err = Error::MethodNotFound {
            name: req.method().to_string(),
            id: req.id.clone()
        };

        Ok((req, err).into())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonRpcRequest {
    jsonrpc: String,
    id: Value,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
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

    pub fn into_params<T: DeserializeOwned>(&mut self) -> Result<T> {
        if let Some(params) = self.params.take() {
            Ok(serde_json::from_value::<T>(params)?)
        } else {
            Err(Error::InvalidParams)
        }
    }
}

impl JsonRpcRequest {

    pub fn from_str(message: &str) -> Result<Self> {
        Ok(serde_json::from_str::<JsonRpcRequest>(message)?)
    }

    /*
    pub fn new(method: &str, params: Option<Value>) -> Self {
        Self {
            jsonrpc: VERSION.to_string(),
            method: method.to_string(),
            params,
            id: Value::Number(Number::from(
                rand::thread_rng().gen_range(0..std::u32::MAX) + 1,
            )),
        }
    }
    */
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
            jsonrpc: VERSION.to_string(),
            result: None,
            error: None,
        }
    }
}

impl JsonRpcResponse {
    /// Reply to a request with a result.
    pub fn response(req: &JsonRpcRequest, result: Option<Value>) -> Self {
        Self {
            jsonrpc: VERSION.to_string(),
            result,
            error: None,
            id: req.id.clone(),
        }
    }

    /// Reply to a request with an empty response (no result or error).
    pub fn reply(req: &JsonRpcRequest) -> Self {
        JsonRpcResponse::response(req, None)
    }

    /// Reply to a request with an error.
    pub fn error(message: String, code: isize, id: Value) -> Self {
        Self {
            jsonrpc: VERSION.to_string(),
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
