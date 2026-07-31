//! JSON-RPC 2.0 wire types.
//!
//! Deliberately minimal: MCP only ever puts objects on the wire, so these are objects.
//!
//! Traffic runs both ways. The client sends [`Request`]s and this server answers them with
//! [`Response`]s — but the server also asks questions of its own, as [`OutRequest`]s, and reads the
//! client's replies back off the same stream. Consent for a destructive command is the only thing
//! that does so today (see `consent.rs`), and it is why a frame with no `method` is not
//! automatically a malformed request.

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
    /// Whether the frame carried an `id` member at all.
    ///
    /// Not deserialized — `Option<Value>` cannot tell an omitted `id` from `"id": null`, and only
    /// the first is a notification. The transport sets this from the raw object, because the
    /// difference decides whether a request that has already run gets an answer.
    #[serde(skip)]
    pub id_present: bool,
}

impl Request {
    /// Whether a value is a shape JSON-RPC allows as an id.
    ///
    /// Strings, numbers and null. An object, array or boolean is not correlatable by a client, so
    /// dispatching one would run the call and answer with something the caller cannot match to it.
    pub fn id_shape_ok(id: &Value) -> bool {
        id.is_string() || id.is_number() || id.is_null()
    }

    /// A message with no `id` member must never be answered.
    ///
    /// Presence, not value. `"id": null` is a request with a null id — discouraged by JSON-RPC but
    /// still a request — and treating it as a notification would run the call, swallow the reply,
    /// and leave a client waiting on something it may then retry and run a second time.
    pub fn is_notification(&self) -> bool {
        !self.id_present
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

/// A request this server sends *to* the client.
///
/// The id is ours to choose and must not collide with one the client is using for its own calls.
/// JSON-RPC keeps the two directions in separate id spaces — a client request `1` and a server
/// request `1` are unrelated — but a client that correlates sloppily would still be confused by an
/// overlap, so [`crate::consent`] draws from a namespace no client would mint.
#[derive(Debug, Clone, Serialize)]
pub struct OutRequest {
    pub jsonrpc: &'static str,
    pub id: Value,
    pub method: &'static str,
    pub params: Value,
}

impl OutRequest {
    pub fn new(id: impl Into<Value>, method: &'static str, params: Value) -> Self {
        Self { jsonrpc: JSONRPC_VERSION, id: id.into(), method, params }
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
