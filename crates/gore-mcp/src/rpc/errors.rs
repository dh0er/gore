//! JSON-RPC 2.0 error codes, and the one rule that governs when we may use them.
//!
//! # Protocol errors are not tool errors
//!
//! This is the single distinction implementers of MCP servers get wrong most often, so it is
//! stated here rather than buried in `server.rs`.
//!
//! A *tool* that fails — `gore` exiting non-zero, a command timing out, an argument that does not
//! validate, a command refused by the safety gate — produces a **successful** `tools/call`
//! response whose result carries `isError: true`. The model can read that, understand what went
//! wrong, and retry with different arguments.
//!
//! The JSON-RPC `error` member is reserved for failures of the *protocol*: unparseable input, a
//! request that is not a JSON-RPC request, an unknown method, an unknown tool name, or an internal
//! failure on our side. The MCP specification is explicit that anything a model could plausibly
//! self-correct belongs in the result, because a protocol error is not routed back to the model by
//! every client and can abort the turn outright.
//!
//! The practical split we apply:
//!
//! | Situation                                   | Mechanism                |
//! |---------------------------------------------|--------------------------|
//! | unparseable JSON on stdin                   | `PARSE_ERROR`            |
//! | valid JSON that is not a JSON-RPC request   | `INVALID_REQUEST`        |
//! | unknown MCP method                          | `METHOD_NOT_FOUND`       |
//! | unknown tool / unknown resource URI         | `INVALID_PARAMS`         |
//! | we could not spawn the child process at all | `INTERNAL_ERROR`         |
//! | missing or malformed *tool argument*        | result with `isError`    |
//! | command exited non-zero, timed out, refused | result with `isError`    |

/// Input could not be parsed as JSON at all.
pub const PARSE_ERROR: i32 = -32700;

/// Valid JSON, but not a well-formed JSON-RPC 2.0 request object.
pub const INVALID_REQUEST: i32 = -32600;

/// The method is not one this server implements.
pub const METHOD_NOT_FOUND: i32 = -32601;

/// The method exists but its params are unusable — an unknown tool name, an unknown resource URI.
pub const INVALID_PARAMS: i32 = -32602;

/// Something failed on our side that the caller cannot fix by changing the request.
pub const INTERNAL_ERROR: i32 = -32603;
