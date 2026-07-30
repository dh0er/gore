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

pub fn call(arguments: &Map<String, Value>, spawn: &dyn Spawn) -> Result<Value, String> {
    for key in arguments.keys() {
        if key != "command" {
            return Ok(to_error_result(format!("`{key}` is not an argument of {NAME}.")));
        }
    }

    let Some(command) = arguments.get("command").and_then(Value::as_str) else {
        return Ok(to_error_result(
            "`command` is required. Pass an empty string for the top-level command list.",
        ));
    };

    let path: Vec<&str> = command.split_whitespace().collect();
    if let Err(message) = validate(&path) {
        return Ok(to_error_result(message));
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
            let (body, clipped, total) = if outcome.stdout.trim().is_empty() {
                (outcome.stderr.clone(), outcome.stderr_truncated, outcome.stderr_total)
            } else {
                (outcome.stdout.clone(), outcome.stdout_truncated, outcome.stdout_total)
            };

            // The spawn layer applies the server's own --max-output-kib cap before this tool ever
            // sees the text, so on a low cap `truncate` gets a prefix that is already under its
            // limit and passes it through silently. Presenting a prefix as the exact help is the
            // one failure this tool cannot afford: the flags it drops are invisible.
            let mut text = truncate(&body);
            let mut truncated = text.ends_with(TRUNCATION_MARKER);
            if clipped && !truncated {
                text.push_str(&format!(
                    "\n… [truncated: the server captured {} of {total} bytes. Raise \
                     --max-output-kib, or ask for a narrower command.]",
                    body.len()
                ));
                truncated = true;
            }

            Ok(json!({
                "content": [
                    { "type": "text", "text": display },
                    { "type": "text", "text": text },
                ],
                "structuredContent": { "exit_code": outcome.status, "truncated": truncated },
                "isError": !outcome.succeeded(),
            }))
        }
        // A process that will not start is this server's problem, not the model's — the same
        // judgement `Session::call_tool` makes for every other tool. Returning it as a tool result
        // here would have one broken binary look like two different failures.
        Err(error) => Err(format!("could not run `{display}`: {error}")),
    }
}

/// Commands the CLI has but no tool wraps.
///
/// `mcp` is this server itself and `guide` renders documentation for a human to read, so neither is
/// exposed as a namespace tool. They are still real commands that `gore --help` lists, and this
/// tool promises the exact help for any command — refusing them would send a model looking for a
/// typo in something it read correctly.
/// `help` is clap's own, and takes any command as its argument — `gore help mgr` is how the CLI
/// itself suggests exploring. Its second token is validated as a first token would be, below.
const META_COMMANDS: &[(&str, &[&str])] =
    &[("mcp", &["serve", "tools"]), ("guide", &["html"]), ("help", &[])];

fn meta_command(name: &str) -> Option<&'static [&'static str]> {
    META_COMMANDS.iter().find(|(command, _)| *command == name).map(|(_, subs)| *subs)
}

/// Accept only paths that exist in the command table.
fn validate(path: &[&str]) -> Result<(), String> {
    // `gore help <path>` is clap's own explorer and takes a whole command path, not just one
    // token — `gore help as compile` is valid. Strip the prefix and judge what follows; each step
    // shortens the slice, so this terminates.
    if let [first, rest @ ..] = path {
        if *first == "help" && !rest.is_empty() {
            return validate(rest);
        }
    }
    match path {
        [] => Ok(()),
        [first] => {
            if meta_command(first).is_some()
                || spec::GROUPS.iter().any(|group| {
                    (group.shape == GroupShape::Nested && group.cli == *first)
                        || (group.shape == GroupShape::Flat && group.command(first).is_some())
                })
            {
                Ok(())
            } else {
                Err(format!("`gore {first}` is not a command. {}", available()))
            }
        }
        [first, second] => {
            if let Some(subcommands) = meta_command(first) {
                return if subcommands.contains(second) {
                    Ok(())
                } else {
                    Err(format!(
                        "`gore {first}` has no subcommand `{second}`. It accepts: {}.",
                        subcommands.join(", ")
                    ))
                };
            }
            let Some(group) =
                spec::GROUPS.iter().find(|group| group.cli == *first && !group.cli.is_empty())
            else {
                // Two different mistakes end up here and they need different answers. `catalog`
                // and `gen` are real top-level commands that simply take no subcommand. `project`
                // is not a command at all — it only exists as the `gore_project` tool name, and an
                // agent mapping tool names onto the CLI will write `project gen` and be told the
                // command "has no subcommands", which implies it exists.
                let is_flat_command = spec::GROUPS.iter().any(|group| {
                    group.shape == GroupShape::Flat && group.command(first).is_some()
                });
                return Err(if is_flat_command {
                    format!(
                        "`gore {first}` takes no subcommand — ask for `{first}` on its own. \
                         (`{second}` is a separate command; the tool groups them, the CLI does \
                         not.)"
                    )
                } else {
                    format!("`gore {first}` is not a command. {}", available())
                });
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
    names.extend(META_COMMANDS.iter().map(|(command, _)| *command));
    names.sort_unstable();
    format!("Available commands: {}.", names.join(", "))
}

const TRUNCATION_MARKER: &str = "… [truncated]";

fn truncate(body: &str) -> String {
    if body.len() <= MAX_OUTPUT_BYTES {
        return body.to_string();
    }
    let mut end = MAX_OUTPUT_BYTES;
    while end > 0 && !body.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n{TRUNCATION_MARKER}", &body[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exec::{FakeSpawn, Outcome};

    /// A runner that cannot start anything, standing in for a missing or unrunnable binary.
    struct FailingSpawn;
    impl Spawn for FailingSpawn {
        fn run(&self, _: &Invocation) -> std::io::Result<crate::exec::Outcome> {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "no such file"))
        }
    }

    fn call_with(arguments: Value, spawn: &dyn Spawn) -> Value {
        try_call(arguments, spawn).expect("the spawn succeeded, so this is a tool result")
    }

    fn try_call(arguments: Value, spawn: &dyn Spawn) -> Result<Value, String> {
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
    fn a_process_that_will_not_start_is_reported_as_an_internal_error() {
        // Every other tool surfaces a failed spawn as a JSON-RPC INTERNAL_ERROR, because no change
        // of arguments makes an unrunnable binary work. Returning it as a tool result here would
        // make one broken binary look like two different failures depending on the tool used.
        let spawn = FailingSpawn;
        let outcome = try_call(json!({ "command": "mgr" }), &spawn);
        let Err(message) = outcome else { panic!("a failed spawn must not be a tool result") };
        assert!(message.contains("could not run"), "{message}");
    }

    #[test]
    fn help_clipped_by_the_server_output_cap_says_so() {
        // A low --max-output-kib makes the spawn layer hand this tool a prefix that is already
        // under its own 64 KiB limit, so `truncate` passes it through untouched. Without the
        // upstream flag the result would present a prefix of `gore as --help` as the exact help,
        // and the flags it dropped would simply not exist as far as the model is concerned.
        let mut outcome = Outcome::success("Usage: gore as <COMMAND>\n  decode-header");
        outcome.stdout_truncated = true;
        outcome.stdout_total = 40_000;

        let spawn = FakeSpawn::new(outcome);
        let result = call_with(json!({ "command": "as" }), &spawn);

        assert_eq!(result["isError"], json!(false));
        assert_eq!(result["structuredContent"]["truncated"], json!(true));
        let body = text_of(&result, 1);
        assert!(body.contains("truncated"), "{body}");
        assert!(body.contains("40000"), "the real size belongs in the message: {body}");
        assert!(body.contains("--max-output-kib"), "{body}");
    }

    #[test]
    fn untruncated_help_is_not_labelled_as_clipped() {
        let spawn = FakeSpawn::new(Outcome::success("Usage: gore mgr <COMMAND>"));
        let result = call_with(json!({ "command": "mgr" }), &spawn);

        assert_eq!(result["structuredContent"]["truncated"], json!(false));
        assert!(!text_of(&result, 1).contains("truncated"));
    }

    #[test]
    fn the_meta_commands_have_help_even_though_no_tool_wraps_them() {
        // `gore --help` lists `mcp` and `guide`, and this tool promises the exact help for any
        // command. Refusing them would send a model hunting for a typo in something it read right.
        let spawn = FakeSpawn::new(Outcome::success("Usage: gore guide html"));
        for path in
            [
                "mcp",
                "mcp serve",
                "mcp tools",
                "guide",
                "guide html",
                "help",
                "help mgr",
                "help gen",
                // clap's explorer takes a whole path, not one token.
                "help as compile",
                "help mgr reset",
                "help guide html",
            ]
        {
            let result = call_with(json!({ "command": path }), &spawn);
            assert_eq!(result["isError"], json!(false), "`{path}` should resolve");
        }

        // `gore help <path>` takes any command path, so a bogus one is still rejected — at either
        // depth.
        for bogus in ["help frobnicate", "help as frobnicate", "help mgr reset extra"] {
            let bad = call_with(json!({ "command": bogus }), &spawn);
            assert_eq!(bad["isError"], json!(true), "`{bogus}` should be rejected");
        }

        // A wrong subcommand under one of them still lists the real ones.
        let result = call_with(json!({ "command": "guide pdf" }), &spawn);
        assert_eq!(result["isError"], json!(true));
        assert!(text_of(&result, 0).contains("html"), "{}", text_of(&result, 0));
    }

    #[test]
    fn a_tool_name_used_as_a_cli_group_is_told_it_is_not_a_command() {
        // `gore_catalog` and `gore_project` group top-level commands that the CLI keeps separate,
        // so an agent reading tool names will try `project gen`. There is no `gore project`, and
        // saying it "has no subcommands" would imply there is.
        let spawn = FakeSpawn::new(Outcome::success(""));
        let result = call_with(json!({ "command": "project gen" }), &spawn);

        assert_eq!(result["isError"], json!(true));
        let message = text_of(&result, 0);
        assert!(message.contains("is not a command"), "{message}");
        assert!(message.contains("gen"), "the real command list must be offered: {message}");
        assert!(spawn.calls().is_empty());
    }

    #[test]
    fn a_flat_command_given_a_subcommand_is_pointed_back_at_itself() {
        // `gore catalog` is real and takes no subcommand, which is a different mistake from the
        // one above and deserves a different answer.
        let spawn = FakeSpawn::new(Outcome::success(""));
        let result = call_with(json!({ "command": "catalog dump" }), &spawn);

        assert_eq!(result["isError"], json!(true));
        let message = text_of(&result, 0);
        assert!(message.contains("takes no subcommand"), "{message}");
        assert!(!message.contains("is not a command"), "catalog does exist: {message}");
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
