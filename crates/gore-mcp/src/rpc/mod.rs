//! The JSON-RPC 2.0 layer: error codes, wire types, and newline framing.
//!
//! Nothing in here knows what MCP is. `server.rs` sits on top and supplies the meaning.

pub mod errors;
pub mod message;
pub mod transport;

pub use message::{Request, Response, RpcError};
pub use transport::{Frame, Transport, MAX_FRAME_BYTES};
