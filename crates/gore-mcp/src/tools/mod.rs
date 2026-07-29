//! Tools that are not CLI commands.
//!
//! The eleven namespace tools are generated from [`crate::spec`]; these are the ones that exist
//! only inside the server — documentation lookup and `--help` passthrough. They are assembled here
//! so `tools/list` has a single place to gather everything.

pub mod guide;
pub mod help;

use serde_json::Value;

/// Tool definitions that do not come from the command table.
pub fn definitions() -> Vec<Value> {
    vec![guide::definition(), help::definition()]
}

/// Whether `name` is one of them.
pub fn is_extra_tool(name: &str) -> bool {
    name == guide::NAME || name == help::NAME
}
