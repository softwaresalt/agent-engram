//! JSON-RPC 2.0 wire types for IPC communication between shim and daemon.
//!
//! Each IPC connection carries exactly one request/response cycle, framed as
//! newline-delimited JSON. This module owns the serialization layer only.
//!
//! # Naming note
//!
//! [`IpcError`] here is the **wire-format struct** `{code, message, data}`.
//! It is distinct from [`crate::errors::IpcError`], the domain error enum used
//! throughout the rest of the library.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 request sent from the shim to the daemon.
///
/// Each field uses `#[serde(default)]` so that absent JSON keys produce empty
/// strings / `None` rather than a deserialization error; validation is deferred
/// to [`IpcRequest::validate`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IpcRequest {
    /// Protocol version; must be `"2.0"`. Defaults to `""` when the field is absent.
    #[serde(default)]
    pub jsonrpc: String,
    /// Request identifier echoed verbatim in the response. `None` when absent.
    #[serde(default)]
    pub id: Option<Value>,
    /// MCP tool name or internal method (`_health`, `_shutdown`).
    /// Defaults to `""` when the field is absent.
    #[serde(default)]
    pub method: String,
    /// Tool parameters, or `null` when the method takes no parameters.
    pub params: Option<Value>,
}

impl IpcRequest {
    /// Parse a (possibly newline-terminated) JSON string into an [`IpcRequest`].
    ///
    /// # Errors
    ///
    /// Returns `Err(`[`IpcResponse`]`)` with:
    /// - code `-32700` (Parse Error) if the input is not valid JSON.
    /// - code `-32600` (Invalid Request) if the JSON root is not an object.
    pub fn from_line(line: &str) -> Result<Self, IpcResponse> {
        let trimmed = line.trim();

        // Guard against obviously non-UTF-8 / unparseable content.
        let value: Value = serde_json::from_str(trimmed)
            .map_err(|e| IpcResponse::parse_error(format!("invalid JSON: {e}")))?;

        if !value.is_object() {
            return Err(IpcResponse::invalid_request(
                Value::Null,
                "request must be a JSON object".to_owned(),
            ));
        }

        serde_json::from_value(value)
            .map_err(|e| IpcResponse::parse_error(format!("failed to deserialize request: {e}")))
    }

    /// Validate JSON-RPC 2.0 field constraints.
    ///
    /// # Errors
    ///
    /// Returns `Err(`[`IpcResponse`]`)` with code `-32600` when:
    /// - `jsonrpc` is not `"2.0"`
    /// - `id` is absent from the original JSON (the error response carries `id: null`)
    /// - `method` is empty
    pub fn validate(&self) -> Result<(), IpcResponse> {
        let id = self.id.clone().unwrap_or(Value::Null);

        if self.jsonrpc != "2.0" {
            return Err(IpcResponse::invalid_request(
                id,
                format!("jsonrpc must be \"2.0\", got {:?}", self.jsonrpc),
            ));
        }

        if self.id.is_none() {
            return Err(IpcResponse::invalid_request(
                Value::Null,
                "id field is required".to_owned(),
            ));
        }

        if self.method.is_empty() {
            return Err(IpcResponse::invalid_request(
                id,
                "method field is required".to_owned(),
            ));
        }

        Ok(())
    }
}

/// JSON-RPC 2.0 response sent from the daemon back to the shim.
///
/// Exactly one of `result` or `error` is present in a valid response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IpcResponse {
    /// Always `"2.0"`.
    pub jsonrpc: String,
    /// Echoes the request `id`, or `null` for parse/invalid-request errors where
    /// no valid `id` could be extracted.
    pub id: Value,
    /// Successful result payload; mutually exclusive with `error`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error payload; mutually exclusive with `result`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<IpcError>,
}

impl IpcResponse {
    /// Construct a successful response echoing the given `id`.
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_owned(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Construct an error response echoing the given `id`.
    pub fn error(id: Value, error: IpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_owned(),
            id,
            result: None,
            error: Some(error),
        }
    }

    /// Construct a **Parse Error** response (code `-32700`).
    ///
    /// The `id` is always `null` because the request could not be parsed.
    pub fn parse_error(reason: String) -> Self {
        Self::error(
            Value::Null,
            IpcError {
                code: -32_700,
                message: reason,
                data: None,
            },
        )
    }

    /// Construct an **Invalid Request** response (code `-32600`).
    pub fn invalid_request(id: Value, reason: String) -> Self {
        Self::error(
            id,
            IpcError {
                code: -32_600,
                message: reason,
                data: None,
            },
        )
    }

    /// Construct a **Method Not Found** response (code `-32601`).
    pub fn method_not_found(id: Value, method: &str) -> Self {
        Self::error(
            id,
            IpcError {
                code: -32_601,
                message: format!("method not found: {method}"),
                data: None,
            },
        )
    }

    /// Construct an **Internal Error** response (code `-32603`).
    pub fn internal_error(id: Value, reason: String) -> Self {
        Self::error(
            id,
            IpcError {
                code: -32_603,
                message: reason,
                data: None,
            },
        )
    }

    /// Serialize this response to a JSON string with a trailing newline.
    ///
    /// The newline delimiter is required for the IPC line-framing protocol.
    ///
    /// # Errors
    ///
    /// Returns [`serde_json::Error`] if serialization fails (this should never
    /// happen for well-formed [`IpcResponse`] values).
    pub fn to_line(&self) -> Result<String, serde_json::Error> {
        let mut s = serde_json::to_string(self)?;
        s.push('\n');
        Ok(s)
    }
}

/// Wire-format JSON-RPC 2.0 error object embedded in [`IpcResponse`].
///
/// # Naming note
///
/// This is the on-wire struct with integer `code` and string `message`. The
/// domain error hierarchy is [`crate::errors::IpcError`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IpcError {
    /// JSON-RPC 2.0 error code (e.g. `-32700`, `-32600`, `-32601`, `-32603`).
    pub code: i32,
    /// Human-readable error description.
    pub message: String,
    /// Optional additional error context (e.g. Engram domain error code).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}
