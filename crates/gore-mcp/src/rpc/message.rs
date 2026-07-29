//! JSON-RPC 2.0 wire types.
//!
//! Deliberately minimal: MCP only ever puts objects on the wire, and this server never sends
//! requests of its own, so there is no client-side request type and no batching support (MCP
//! removed JSON-RPC batching in the 2025-06-18 revision).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::errors;

/// The only `jsonrpc` value MCP permits.
pub const JSONRPC_VERSION: &str = "2.0";

/// An incoming request or notification.
///
/// `id` distinguishes the two: a request carries one, a notification does not. Note that serde
/// maps both an absent `id` and an explicit `"id": null` to `None`. That collapse is intentional —
/// JSON-RPC discourages null ids, no MCP client sends them, and treating such a message as a
/// notification (i.e. answering nothing) is the safe reading either way.
#[derive(Debug, Clone, Deserialize)]
pub struct Request {
    #[serde(default)]
    pub jsonrpc: Option<String>,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

impl Request {
    /// A message with no `id` must never be answered.
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }

    /// The id to echo back. JSON-RPC requires the member to be present even on errors, where the
    /// convention is `null` when the id could not be determined.
    pub fn response_id(&self) -> Value {
        self.id.clone().unwrap_or(Value::Null)
    }

    /// `params` as an object, or an empty map when absent. MCP params are always objects, and
    /// callers uniformly want "give me the fields" rather than "tell me it was missing".
    pub fn params_object(&self) -> serde_json::Map<String, Value> {
        match &self.params {
            Value::Object(map) => map.clone(),
            _ => serde_json::Map::new(),
        }
    }

    /// Whether the `jsonrpc` member is acceptable.
    ///
    /// Strictly the member is mandatory, but rejecting a request purely because a client omitted
    /// it would trade interoperability for pedantry. We reject only a *wrong* value, which is a
    /// genuine signal that the peer is speaking a different protocol.
    pub fn version_ok(&self) -> bool {
        match self.jsonrpc.as_deref() {
            None => true,
            Some(version) => version == JSONRPC_VERSION,
        }
    }
}

/// An outgoing response. Exactly one of `result` / `error` is populated.
///
/// Modelled as two `Option` fields rather than a flattened enum: the wire shape is identical, and
/// this way there is no chance of serde emitting a nested `{"result": {"result": ...}}` if the
/// enum representation is ever adjusted.
#[derive(Debug, Clone, Serialize)]
pub struct Response {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl Response {
    pub fn ok(id: Value, result: Value) -> Self {
        Self { jsonrpc: JSONRPC_VERSION, id, result: Some(result), error: None }
    }

    pub fn error(id: Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            id,
            result: None,
            error: Some(RpcError { code, message: message.into(), data: None }),
        }
    }

    /// Input that could not be parsed. The id is unknowable, so it is `null` per JSON-RPC.
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::error(Value::Null, errors::PARSE_ERROR, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &str) -> Request {
        serde_json::from_str(raw).expect("request should parse")
    }

    #[test]
    fn numeric_and_string_ids_round_trip() {
        assert_eq!(parse(r#"{"jsonrpc":"2.0","id":7,"method":"ping"}"#).response_id(), Value::from(7));
        assert_eq!(
            parse(r#"{"jsonrpc":"2.0","id":"abc","method":"ping"}"#).response_id(),
            Value::from("abc")
        );
    }

    #[test]
    fn absent_id_is_a_notification() {
        assert!(parse(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_notification());
    }

    #[test]
    fn explicit_null_id_is_also_treated_as_a_notification() {
        assert!(parse(r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#).is_notification());
    }

    #[test]
    fn missing_params_yields_an_empty_object() {
        assert!(parse(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#).params_object().is_empty());
    }

    #[test]
    fn version_is_accepted_when_absent_and_rejected_when_wrong() {
        assert!(parse(r#"{"id":1,"method":"ping"}"#).version_ok());
        assert!(parse(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#).version_ok());
        assert!(!parse(r#"{"jsonrpc":"1.0","id":1,"method":"ping"}"#).version_ok());
    }

    #[test]
    fn a_response_serializes_exactly_one_of_result_or_error() {
        let ok = serde_json::to_value(Response::ok(Value::from(1), serde_json::json!({}))).unwrap();
        assert!(ok.get("result").is_some());
        assert!(ok.get("error").is_none());

        let err = serde_json::to_value(Response::error(Value::from(1), -32601, "nope")).unwrap();
        assert!(err.get("result").is_none());
        assert_eq!(err["error"]["code"], -32601);
        assert_eq!(err["error"]["message"], "nope");
        // `data` is optional and must not appear when unset.
        assert!(err["error"].get("data").is_none());
    }
}
