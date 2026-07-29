//! `gore_help` — run `gore <command> --help` and return what it printed.
//!
//! The guide explains *why* and the tool schemas summarise the arguments, but clap's own help is
//! the only description of the CLI that cannot drift, because it is generated from the definitions
//! themselves. When the two disagree, this is the one to believe.
//!
//! The command path is validated against the command table rather than passed through. That is not
//! about untrusted input — the caller could reach the same commands through the namespace tools
//! anyway — but about failing usefully: an agent that mistypes a path gets the list of real ones
//! back instead of clap's parse error.

use std::time::Duration;

use serde_json::{json, Map, Value};

use crate::argv::Invocation;
use crate::exec::{to_error_result, Spawn};
use crate::spec::{self, GroupShape};

pub const NAME: &str = "gore_help";

/// Printing help never takes long; if it does, something is badly wrong.
const TIMEOUT: Duration = Duration::from_secs(30);

/// Help output is small, but a corrupt binary could print anything.
const MAX_OUTPUT_BYTES: usize = 64 * 1024;

pub fn definition() -> Value {
    json!({
        "name": NAME,
        "title": "gore --help",
        "description":
            "Print the exact, current help for a `gore` command — every flag, its value name and \
             its default, straight from the CLI itself.\n\n\
             Use it when a tool description is not specific enough about an argument, or to check \
             what a command accepts before constructing a call. Pass an empty `command` for the \
             top-level list of commands, a group name such as \"as\" for its subcommands, or a \
             full path such as \"as patch-default\" for one command.\n\n\
             For what a command is *for*, and the order to run things in, use gore_guide instead.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Command path, space separated: \"\" for the top level, \"as\" \
                                    for a group, \"as patch-default\" for one command.",
                },
            },
            "required": ["command"],
            "additionalProperties": false,
        },
        "annotations": {
            "title": "gore --help",
            "readOnlyHint": true,
            "openWorldHint": false,
        },
    })
}

pub fn call(arguments: &Map<String, Value>, spawn: &dyn Spawn) -> Value {
    for key in arguments.keys() {
        if key != "command" {
            return to_error_result(format!("`{key}` is not an argument of {NAME}."));
        }
    }

    let Some(command) = arguments.get("command").and_then(Value::as_str) else {
        return to_error_result(
            "`command` is required. Pass an empty string for the top-level command list.",
        );
    };

    let path: Vec<&str> = command.split_whitespace().collect();
    if let Err(message) = validate(&path) {
        return to_error_result(message);
    }

    let display = if path.is_empty() {
        "gore --help".to_string()
    } else {
        format!("gore {} --help", path.join(" "))
    };
    let mut argv: Vec<std::ffi::OsString> =
        path.iter().map(std::ffi::OsString::from).collect();
    argv.push("--help".into());

    let invocation =
        Invocation { argv, path: path.join(" "), timeout: TIMEOUT, display: display.clone() };
    match spawn.run(&invocation) {
        Ok(outcome) => {
            // clap prints help to stdout on success; when a path is wrong it goes to stderr.
            let body = if outcome.stdout.trim().is_empty() {
                outcome.stderr.clone()
            } else {
                outcome.stdout.clone()
            };
            json!({
                "content": [
                    { "type": "text", "text": display },
                    { "type": "text", "text": truncate(&body) },
                ],
                "structuredContent": { "exit_code": outcome.status },
                "isError": !outcome.succeeded(),
            })
        }
        Err(error) => to_error_result(format!("could not run `{display}`: {error}")),
    }
}

/// Accept only paths that exist in the command table.
fn validate(path: &[&str]) -> Result<(), String> {
    match path {
        [] => Ok(()),
        [first] => {
            if spec::GROUPS.iter().any(|group| {
                (group.shape == GroupShape::Nested && group.cli == *first)
                    || (group.shape == GroupShape::Flat && group.command(first).is_some())
            }) {
                Ok(())
            } else {
                Err(format!("`gore {first}` is not a command. {}", available()))
            }
        }
        [first, second] => {
            let Some(group) =
                spec::GROUPS.iter().find(|group| group.cli == *first && !group.cli.is_empty())
            else {
                return Err(format!("`gore {first}` has no subcommands. {}", available()));
            };
            if group.command(second).is_some() {
                Ok(())
            } else {
                Err(format!(
                    "`gore {first}` has no subcommand `{second}`. It accepts: {}.",
                    group.subcommands().join(", ")
                ))
            }
        }
        _ => Err(format!(
            "`{}` is deeper than any gore command. Paths are at most two words.",
            path.join(" ")
        )),
    }
}

fn available() -> String {
    let mut names: Vec<&str> = Vec::new();
    for group in spec::GROUPS {
        match group.shape {
            GroupShape::Nested => names.push(group.cli),
            GroupShape::Flat => names.extend(group.subcommands()),
        }
    }
    names.sort_unstable();
    format!("Available commands: {}.", names.join(", "))
}

fn truncate(body: &str) -> String {
    if body.len() <= MAX_OUTPUT_BYTES {
        return body.to_string();
    }
    let mut end = MAX_OUTPUT_BYTES;
    while end > 0 && !body.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n… [truncated]", &body[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exec::{FakeSpawn, Outcome};

    fn call_with(arguments: Value, spawn: &dyn Spawn) -> Value {
        let Value::Object(map) = arguments else { panic!("test arguments must be an object") };
        call(&map, spawn)
    }

    fn text_of(result: &Value, index: usize) -> String {
        result["content"][index]["text"].as_str().expect("a text block").to_string()
    }

    #[test]
    fn the_top_level_path_runs_a_bare_help() {
        let spawn = FakeSpawn::new(Outcome::success("Usage: gore <COMMAND>"));
        let result = call_with(json!({ "command": "" }), &spawn);

        assert_eq!(result["isError"], json!(false));
        assert_eq!(text_of(&result, 0), "gore --help");
        let argv: Vec<String> = spawn.calls()[0]
            .argv
            .iter()
            .map(|token| token.to_string_lossy().into_owned())
            .collect();
        assert_eq!(argv, vec!["--help"]);
    }

    #[test]
    fn a_group_and_a_full_path_both_resolve() {
        let spawn = FakeSpawn::new(Outcome::success("help text"));

        call_with(json!({ "command": "as" }), &spawn);
        call_with(json!({ "command": "as patch-default" }), &spawn);
        // A flat group's subcommand is a top-level command in the CLI.
        call_with(json!({ "command": "scaffold" }), &spawn);

        let paths: Vec<String> = spawn.calls().iter().map(|call| call.display.clone()).collect();
        assert_eq!(
            paths,
            vec!["gore as --help", "gore as patch-default --help", "gore scaffold --help"]
        );
    }

    #[test]
    fn an_unknown_command_lists_the_real_ones_without_spawning_anything() {
        let spawn = FakeSpawn::new(Outcome::success(""));
        let result = call_with(json!({ "command": "frobnicate" }), &spawn);

        assert_eq!(result["isError"], json!(true));
        let message = text_of(&result, 0);
        assert!(message.contains("texture"), "{message}");
        assert!(spawn.calls().is_empty());
    }

    #[test]
    fn an_unknown_subcommand_lists_its_siblings() {
        let spawn = FakeSpawn::new(Outcome::success(""));
        let result = call_with(json!({ "command": "mgr obliterate" }), &spawn);

        let message = text_of(&result, 0);
        assert!(message.contains("no subcommand `obliterate`"), "{message}");
        assert!(message.contains("analyze"), "{message}");
        assert!(spawn.calls().is_empty());
    }

    #[test]
    fn a_path_deeper_than_the_cli_is_rejected() {
        let spawn = FakeSpawn::new(Outcome::success(""));
        let result = call_with(json!({ "command": "as compile module extra" }), &spawn);
        assert!(text_of(&result, 0).contains("at most two words"));
        assert!(spawn.calls().is_empty());
    }

    #[test]
    fn help_printed_to_stderr_is_still_surfaced() {
        let spawn = FakeSpawn::new(Outcome::failure(2, "error: unexpected argument"));
        let result = call_with(json!({ "command": "as" }), &spawn);

        assert_eq!(result["isError"], json!(true));
        assert!(text_of(&result, 1).contains("unexpected argument"));
    }

    #[test]
    fn unknown_arguments_are_rejected() {
        let spawn = FakeSpawn::new(Outcome::success(""));
        let result = call_with(json!({ "cmd": "as" }), &spawn);
        assert!(text_of(&result, 0).contains("`cmd`"));
    }

    #[test]
    fn every_command_in_the_table_is_reachable_through_this_tool() {
        // If a path the namespace tools accept were rejected here, the two surfaces would disagree
        // about what exists.
        for group in spec::GROUPS {
            for command in group.commands {
                let path: Vec<&str> = match group.shape {
                    GroupShape::Nested => vec![group.cli, command.sub],
                    GroupShape::Flat => vec![command.sub],
                };
                assert!(validate(&path).is_ok(), "{path:?} was rejected");
            }
        }
    }
}
