//! A Model Context Protocol server that exposes the `gore` command line tool to AI agents.
//!
//! # Shape
//!
//! Three layers, each testable on its own:
//!
//! - [`rpc`] is plain JSON-RPC 2.0 over newline-delimited frames. It knows nothing about MCP.
//! - [`server`] gives those frames meaning: `initialize`, `tools/*`, `resources/*`, `ping`.
//! - [`capabilities`] is what we tell a client about ourselves, including the instructions primer.
//!
//! # Why it re-execs the CLI instead of calling the libraries
//!
//! Every tool call spawns `gore <argv>` as a child process and captures its output. Three reasons,
//! in order of severity:
//!
//! 1. **The stdio transport owns stdout.** All 21 command modules in the `gore` crate print with
//!    `println!`. Calling them in-process would interleave their output with JSON-RPC frames and
//!    corrupt the stream. A child gets its own piped stdout, so the problem cannot arise.
//! 2. **Much of the logic lives in the CLI crate, not in a library.** The cooked-asset and
//!    AngelScript command modules carry substantial receipt and validation logic of their own, and
//!    six commands have no backing library at all. Calling libraries directly would mean
//!    reimplementing them.
//! 3. **No drift.** Whatever the CLI can do, this server can do, permanently and for free.
//!
//! The cost is a process spawn per call — immaterial next to commands that scan every game
//! container or launch the game itself.
//!
//! # Invariants
//!
//! - This crate never touches the real stdin or stdout. [`serve`] is generic over its streams, and
//!   the only place a true stdout handle appears is the `gore mcp serve` wrapper in the CLI crate.
//! - Child processes are spawned with a null stdin. Our stdin is the JSON-RPC channel and must
//!   never be shared; a child that prompts must fail fast rather than hang the session.
//! - A tool that fails produces a successful response carrying `isError: true`. The JSON-RPC
//!   `error` member is reserved for protocol failures. See [`rpc::errors`] for the full rule.

pub mod argv;
pub mod capabilities;
pub mod exec;
pub mod guide;
pub mod resources;
pub mod rpc;
pub mod schema;
pub mod server;
pub mod spec;
pub mod tools;

pub use server::{serve, Options, Session, DEFAULT_MAX_STDOUT_BYTES};

/// Every tool this server advertises: one per command group, plus the guide and help tools.
///
/// Public so `gore mcp tools` can print the surface without a client having to speak JSON-RPC to
/// discover it.
pub fn tool_definitions() -> Vec<serde_json::Value> {
    let mut definitions = schema::tool_definitions();
    definitions.extend(tools::definitions());
    definitions
}
