use rand::Rng;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Number, Value};

const VERSION: &str = "2.0";

const PARSE_ERROR: isize = -32700;
//const INVALID_REQUEST: isize = -32600;
const METHOD_NOT_FOUND: isize = -32601;
const INVALID_PARAMS: isize = -32602;
const INTERNAL_ERROR: isize = -32603;

/// Result type.
pub type Result<T> = std::result::Result<T, Error>;

/// Enumeration of errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Service method {name} not found")]
    MethodNotFound { id: Value, name: String },

    #[error("Message parameters are invalid")]
    InvalidParams { id: Value, data: String },

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Boxed(#[from] Box<dyn std::error::Error + Send>),
}

impl<'a> Into<Response> for (&'a mut Request, Error) {
    fn into(self) -> Response {
        let (code, data): (isize, Option<String>) = match &self.1 {
            Error::MethodNotFound { .. } => (METHOD_NOT_FOUND, None),
            Error::InvalidParams { data, .. } => (INVALID_PARAMS, Some(data.to_string())),
            Error::Json(_) => (PARSE_ERROR, None),
            _ => (INTERNAL_ERROR, None),
        };
        Response {
            jsonrpc: VERSION.to_string(),
            id: self.0.id.clone(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: self.1.to_string(),
                data,
            }),
        }
    }
}

impl Into<Response> for Error {
    fn into(self) -> Response {
        Response {
            jsonrpc: VERSION.to_string(),
            id: Value::Null,
            result: None,
            error: Some(JsonRpcError {
                code: INTERNAL_ERROR,
                message: self.to_string(),
                data: None,
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcError {
    code: isize,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<String>,
}

/// Helper function to `Box` an error implementation.
///
/// Useful in service handlers that need to use the `?` operator
/// to propagate foreign errors via the service broker.
pub fn box_error(e: impl std::error::Error + Send + 'static) -> Error {
    let err: Box<dyn std::error::Error + Send> = Box::new(e);
    Error::from(err)
}

/// Trait for service implementations.
pub trait Service {
    fn handle(&self, req: &mut Request) -> Result<Option<Response>>;
}

/// Broker calls multiple services and always yields a response.
///
/// If no service handler matches the request method the broker will
/// return `METHOD_NOT_FOUND`.
pub struct Broker;
impl Broker {
    pub fn handle<'a>(
        &self,
        services: &'a Vec<&'a Box<dyn Service>>,
        req: &mut Request,
    ) -> Result<Response> {
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

/// JSON-RPC request.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Request {
    jsonrpc: String,
    id: Value,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

impl Request {

    /// The id for the request.
    pub fn id(&self) -> &Value {
        &self.id
    }

    /// The request service method name.
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Determine if the given name matches the request method.
    pub fn matches(&self, name: &str) -> bool {
        name == &self.method
    }

    /// Deserialize the message parameters into type `T`.
    ///
    /// If this request message has no parameters or the `params`
    /// payload cannot be converted to `T` this will return `INVALID_PARAMS`.
    pub fn into_params<T: DeserializeOwned>(&mut self) -> Result<T> {
        if let Some(params) = self.params.take() {
            Ok(serde_json::from_value::<T>(params).map_err(|e| {
                Error::InvalidParams {
                    id: self.id.clone(),
                    data: e.to_string()}
            })?)
        } else {
            Err(Error::InvalidParams {
                id: self.id.clone(),
                data: "No parameters given".to_string()})
        }
    }
}

impl Request {
    /// Create a new request.
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

    /// Parse a JSON payload.
    pub fn from_str(message: &str) -> Result<Self> {
        Ok(serde_json::from_str::<Request>(message)?)
    }
}

/// JSON-RPC response.
#[derive(Deserialize, Serialize, Debug)]
pub struct Response {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Value::is_null")]
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

impl<'a> From<(&'a mut Request, Value)> for Response {
    fn from(req: (&'a mut Request, Value)) -> Self {
        Self {
            jsonrpc: VERSION.to_string(),
            id: req.0.id.clone(),
            result: Some(req.1),
            error: None,
        }
    }
}

impl<'a> From<&'a mut Request> for Response {
    fn from(req: &'a mut Request) -> Self {
        Self {
            jsonrpc: VERSION.to_string(),
            result: None,
            error: None,
            id: req.id.clone(),
        }
    }
}
