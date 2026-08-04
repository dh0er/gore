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
//! [`consent`] cuts across all three: a command that would overwrite or install something asks the
//! user first, over the same connection, and runs only if they agree.
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
pub mod consent;
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

#[cfg(test)]
mod result_shape {
    use serde_json::{json, Value};

    /// Walk every value in a result, so a `structuredContent` nested anywhere is still found.
    fn mentions_structured_content(value: &Value) -> bool {
        match value {
            Value::Object(map) => {
                map.contains_key("structuredContent")
                    || map.values().any(mentions_structured_content)
            }
            Value::Array(items) => items.iter().any(mentions_structured_content),
            _ => false,
        }
    }

    #[test]
    fn no_tool_declares_an_output_schema() {
        // The premise the next test rests on. If a tool ever gains one, `structuredContent` becomes
        // legitimate *for that tool* — and it must then carry the whole answer, because a client
        // may show it instead of `content` rather than alongside it.
        for tool in crate::tool_definitions() {
            assert!(
                tool.get("outputSchema").is_none(),
                "{} declares an outputSchema; revisit the rule below",
                tool["name"]
            );
        }
    }

    #[test]
    fn no_result_carries_structured_content() {
        // This is not a style rule. `structuredContent` is the schema-backed channel, and a client
        // that sees it is entitled to treat it as *the* result: Claude Code does exactly that and
        // drops `content` entirely. While these tools returned a summary of byte counts there, the
        // model received `{"exit_code":0,"stdout_bytes":488}` in place of every command's output,
        // and `gore_guide read` answered with an anchor instead of the page.
        let command = crate::spec::group("gore_config")
            .and_then(|group| group.command("path"))
            .expect("config path exists");
        let invocation = crate::argv::Invocation {
            argv: vec!["config".into(), "path".into()],
            path: "config path".into(),
            timeout: std::time::Duration::from_secs(60),
            display: "gore config path".into(),
            consent: None,
        };

        let results = [
            ("a command that succeeded", crate::exec::to_call_result(
                &invocation, command, &crate::exec::Outcome::success("C:/x/config.json\n"),
            )),
            ("a command that failed", crate::exec::to_call_result(
                &invocation, command, &crate::exec::Outcome::failure(1, "boom"),
            )),
            ("an argument error", crate::exec::to_error_result("nope")),
            ("guide list", crate::tools::guide::call(&json!({ "action": "list" }).as_object().unwrap().clone())),
            ("guide read", crate::tools::guide::call(
                &json!({ "action": "read", "page": "mcp" }).as_object().unwrap().clone(),
            )),
            ("guide read one section", crate::tools::guide::call(
                &json!({ "action": "read", "page": "mcp", "section": "the-tools" })
                    .as_object().unwrap().clone(),
            )),
            ("guide search", crate::tools::guide::call(
                &json!({ "action": "search", "query": "texture" }).as_object().unwrap().clone(),
            )),
            ("guide search with no hits", crate::tools::guide::call(
                &json!({ "action": "search", "query": "zzzznothingmatchesthis" })
                    .as_object().unwrap().clone(),
            )),
        ];

        for (what, result) in results {
            assert!(
                !mentions_structured_content(&result),
                "{what} carries structuredContent: {result}"
            );
            let content = result["content"].as_array().unwrap_or_else(|| panic!("{what}: {result}"));
            assert!(!content.is_empty(), "{what} has no content blocks");
            assert!(
                content.iter().any(|block| block["text"].as_str().is_some_and(|t| !t.is_empty())),
                "{what} has no text for the model to read: {result}"
            );
        }
    }

    #[test]
    fn the_help_tool_answers_with_its_text_and_nothing_structured() {
        let spawn = crate::exec::FakeSpawn::new(crate::exec::Outcome::success(
            "Usage: gore config path\n\nOptions:\n  -h, --help  Print help\n",
        ));
        let result = crate::tools::help::call(
            &json!({ "command": "config path" }).as_object().unwrap().clone(),
            &spawn,
        )
        .expect("the fake spawn always starts");

        assert!(!mentions_structured_content(&result), "{result}");
        assert!(
            result["content"][1]["text"].as_str().unwrap().contains("Print help"),
            "the help text is the whole tool: {result}"
        );
    }
}
