//! Proves the MCP command table still matches the CLI it claims to describe.
//!
//! `gore-mcp` describes all of `gore` in a hand-written table. Nothing in the type system keeps
//! that table in step with the clap definitions it mirrors — rename a flag in `main.rs` and the
//! table silently starts building command lines that no longer parse. This test closes that gap by
//! asking clap itself: for every leaf command, run `--help` and compare the flag sets in both
//! directions.
//!
//! `--help` is a good oracle here because it touches no filesystem, needs no game installation, and
//! is generated from the very definitions the table mirrors. The whole sweep is a few seconds.
//!
//! When this fails, the table is wrong until proven otherwise — clap is the source of truth.

use std::collections::BTreeSet;

use assert_cmd::Command;
use gore_mcp::spec::{self, GroupShape, GroupSpec, JsonSupport};

/// Flags clap adds to every command, which the table deliberately does not model.
const KNOWN_OMISSIONS: &[&str] = &["help", "version"];

/// Run `gore … --help` and return its stdout.
fn help(argv: &[&str]) -> String {
    let assert = Command::cargo_bin("gore")
        .expect("the gore binary is built for integration tests")
        .args(argv)
        .arg("--help")
        .assert()
        // A failure here usually means the subcommand does not exist under that name any more.
        .success();
    String::from_utf8(assert.get_output().stdout.clone()).expect("help output is utf-8")
}

/// The command path a leaf lives at, e.g. `["config", "set"]` or `["dump"]`.
fn path<'a>(group: &'a GroupSpec, sub: &'a str) -> Vec<&'a str> {
    match group.shape {
        GroupShape::Nested => vec![group.cli, sub],
        GroupShape::Flat => vec![sub],
    }
}

/// Long flags **declared** by clap, as opposed to merely mentioned in help prose.
///
/// The distinction matters: several doc comments in `gore` reference other flags by name (for
/// instance `default-sites --json`), and a naive scan for `--word` would treat those as
/// declarations and demand the table carry flags that belong to a different command. Declarations
/// are indented lines whose first token is itself a flag, so that is what we match.
fn declared_flags(help: &str) -> BTreeSet<String> {
    let mut flags = BTreeSet::new();

    for line in help.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if indent < 2 || !trimmed.starts_with('-') {
            continue;
        }
        // `  -o, --out <OUT>  Output path` or `      --json  Emit JSON`. Only the first long form
        // on the line is the declaration; anything later is help text.
        for token in trimmed.split_whitespace() {
            let Some(name) = token.strip_prefix("--") else {
                continue;
            };
            let name = name.trim_end_matches(',');
            if !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                flags.insert(name.to_string());
            }
            break;
        }
    }

    flags
}

/// Positional placeholders from the `Usage:` line, in order.
///
/// The subtlety is that a required option contributes a placeholder too — `gore dump` shows as
/// `Usage: gore dump --out <OUT> <SDK_DIR>`, where `<OUT>` is the value of `--out` and only
/// `<SDK_DIR>` is positional. A placeholder is therefore skipped when the token before it was a
/// flag.
fn usage_positionals(help: &str) -> Vec<String> {
    let Some(usage) = help
        .lines()
        .map(str::trim_start)
        .find(|line| line.starts_with("Usage:"))
    else {
        return Vec::new();
    };

    let mut positionals = Vec::new();
    let mut after_flag = false;

    for token in usage.split_whitespace() {
        // Optional options appear bracketed, as `[--src <SRC>]`.
        if token.trim_start_matches('[').starts_with('-') {
            after_flag = true;
            continue;
        }
        let is_placeholder = token.starts_with('<') || token.starts_with('[');
        let was_after_flag = std::mem::replace(&mut after_flag, false);
        if !is_placeholder || was_after_flag {
            continue;
        }

        let name = token.trim_matches(|c| matches!(c, '<' | '>' | '[' | ']' | '.'));
        // clap's own placeholders for the option block and for nested subcommands.
        if name != "OPTIONS" && name != "COMMAND" {
            positionals.push(name.to_string());
        }
    }

    positionals
}

/// Every long flag the table is responsible for, including the ones it passes implicitly.
fn table_flags(command: &spec::CommandSpec) -> BTreeSet<String> {
    let mut flags: BTreeSet<String> = command
        .args
        .iter()
        .filter_map(|arg| arg.form.flag())
        .map(str::to_string)
        .collect();

    if command.json == JsonSupport::Stdout {
        flags.insert("json".to_string());
    }
    let mut previous_was_flag = false;
    for forced in command.forced_argv {
        // Forced arguments are written in long form precisely so this comparison works; a short
        // flag cannot be mapped back to the long name clap reports.
        if forced.starts_with("--") {
            flags.insert(forced.trim_start_matches("--").to_string());
            previous_was_flag = true;
        } else {
            assert!(
                previous_was_flag,
                "{}: forced value {forced:?} does not follow a long flag",
                command.sub
            );
            previous_was_flag = false;
        }
    }
    flags.extend(
        command
            .cli_only_flags
            .iter()
            .map(|flag| (*flag).to_string()),
    );

    flags
}

#[test]
fn every_declared_flag_exists_in_the_cli() {
    for group in spec::GROUPS {
        for command in group.commands {
            let path = path(group, command.sub);
            let declared = declared_flags(&help(&path));

            for flag in table_flags(command) {
                assert!(
                    declared.contains(&flag),
                    "`gore {}` has no --{flag}, but the MCP table declares it.\nclap declares: {:?}",
                    path.join(" "),
                    declared
                );
            }
        }
    }
}

#[test]
fn every_cli_flag_is_accounted_for_in_the_table() {
    for group in spec::GROUPS {
        for command in group.commands {
            let path = path(group, command.sub);
            let declared = declared_flags(&help(&path));
            let known = table_flags(command);

            for flag in declared {
                if KNOWN_OMISSIONS.contains(&flag.as_str()) {
                    continue;
                }
                assert!(
                    known.contains(&flag),
                    "`gore {} --{flag}` exists but the MCP table does not expose it, so agents \
                     cannot reach it. Add it to the table, or add it to KNOWN_OMISSIONS with a \
                     reason.",
                    path.join(" ")
                );
            }
        }
    }
}

#[test]
fn positional_counts_match_the_cli() {
    for group in spec::GROUPS {
        for command in group.commands {
            let path = path(group, command.sub);
            let expected = usage_positionals(&help(&path));
            let actual = command
                .args
                .iter()
                .filter(|arg| arg.form.positional_order().is_some())
                .count();

            assert_eq!(
                actual,
                expected.len(),
                "`gore {}` takes {} positional argument(s) {expected:?}, but the MCP table \
                 declares {actual}. A miscount silently shifts every later value.",
                path.join(" "),
                expected.len()
            );
        }
    }
}

#[test]
fn positional_order_matches_the_cli() {
    for group in spec::GROUPS {
        for command in group.commands {
            let path = path(group, command.sub);
            let expected = usage_positionals(&help(&path));

            let mut ours: Vec<(u8, String)> = command
                .args
                .iter()
                .filter_map(|arg| {
                    arg.form
                        .positional_order()
                        .map(|order| (order, arg.name.to_uppercase()))
                })
                .collect();
            ours.sort_by_key(|(order, _)| *order);
            let ours: Vec<String> = ours.into_iter().map(|(_, name)| name).collect();

            // clap lets a command override a placeholder with a custom `value_name`, in which case
            // the names legitimately differ and only the count (checked above) is comparable. Order
            // is verified whenever the two name sets do agree, which is the common case.
            let comparable: BTreeSet<&String> = ours.iter().collect();
            let theirs: BTreeSet<&String> = expected.iter().collect();
            if comparable != theirs {
                continue;
            }

            assert_eq!(
                ours,
                expected,
                "`gore {}` expects its positionals in the order {expected:?}, but the MCP table \
                 orders them {ours:?}.",
                path.join(" ")
            );
        }
    }
}

#[test]
fn mcp_tools_prints_the_whole_advertised_surface_as_json() {
    // `gore mcp tools` exists so the exposed surface can be reviewed, and a client integration
    // debugged, without speaking JSON-RPC to a running server.
    let assert = Command::cargo_bin("gore")
        .unwrap()
        .args(["mcp", "tools"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    let tools: Vec<serde_json::Value> =
        serde_json::from_str(&stdout).expect("mcp tools must print valid JSON");
    assert_eq!(
        tools.len(),
        spec::GROUPS.len() + 2,
        "one tool per group, plus gore_guide and gore_help"
    );

    let names: Vec<&str> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    for group in spec::GROUPS {
        assert!(names.contains(&group.tool), "{} is missing", group.tool);
    }
    assert!(names.contains(&"gore_guide"));
    assert!(names.contains(&"gore_help"));

    for tool in &tools {
        assert_eq!(tool["inputSchema"]["type"], "object", "{}", tool["name"]);
        assert!(tool["description"]
            .as_str()
            .is_some_and(|text| !text.is_empty()));
    }
}

#[test]
fn every_subcommand_appears_in_its_tool_description() {
    // The description is the only place a model learns a subcommand's arguments, since the schema
    // keeps `args` open. A leaf missing from it is a leaf the model will never call correctly.
    for tool in serde_json::from_str::<Vec<serde_json::Value>>(
        &String::from_utf8(
            Command::cargo_bin("gore")
                .unwrap()
                .args(["mcp", "tools"])
                .assert()
                .success()
                .get_output()
                .stdout
                .clone(),
        )
        .unwrap(),
    )
    .unwrap()
    {
        let name = tool["name"].as_str().unwrap();
        let Some(group) = spec::GROUPS.iter().find(|group| group.tool == name) else {
            continue;
        };
        let description = tool["description"].as_str().unwrap();
        for command in group.commands {
            assert!(
                description.contains(command.sub),
                "{name} does not document its `{}` subcommand",
                command.sub
            );
        }
    }
}

#[test]
fn the_table_covers_the_expected_number_of_leaves() {
    assert_eq!(
        spec::leaf_count(),
        spec::EXPECTED_LEAF_COUNT,
        "the MCP table no longer covers the number of leaf commands it claims to"
    );
}

/// Top-level `gore` subcommands the MCP surface deliberately does not expose.
///
/// `mcp` is this server itself — offering an agent a tool that starts another copy of the server it
/// is already talking to would be a loop, not a feature.
///
/// `guide` renders the guide to an HTML file for a human to read in a browser. An agent already has
/// strictly better access to the same pages through the `gore_guide` tool, and cannot read the file
/// it would produce, so exposing it would add surface for no capability.
const UNEXPOSED_TOP_LEVEL: &[&str] = &["mcp", "guide", "help"];

#[test]
fn no_top_level_command_family_is_missing_from_the_table() {
    // The check the per-leaf tests cannot make: they iterate the table, so a command family added
    // to the CLI and never added to the table is invisible to them. This one iterates the CLI.
    let root = help(&[]);

    let mut in_commands_section = false;
    let mut cli_commands: BTreeSet<String> = BTreeSet::new();
    for line in root.lines() {
        if line.starts_with("Commands:") {
            in_commands_section = true;
            continue;
        }
        if in_commands_section {
            // The section ends at the first unindented line (`Options:`).
            if !line.starts_with("  ") {
                if line.trim().is_empty() {
                    continue;
                }
                break;
            }
            if let Some(name) = line.split_whitespace().next() {
                if name.starts_with('-') {
                    continue;
                }
                cli_commands.insert(name.to_string());
            }
        }
    }
    assert!(
        !cli_commands.is_empty(),
        "could not parse the Commands: section of `gore --help`"
    );

    let mut covered: BTreeSet<String> = BTreeSet::new();
    for group in spec::GROUPS {
        match group.shape {
            GroupShape::Nested => {
                covered.insert(group.cli.to_string());
            }
            GroupShape::Flat => {
                covered.extend(group.commands.iter().map(|command| command.sub.to_string()));
            }
        }
    }

    for name in &cli_commands {
        if UNEXPOSED_TOP_LEVEL.contains(&name.as_str()) {
            continue;
        }
        assert!(
            covered.contains(name),
            "`gore {name}` exists but no MCP tool reaches it. Add it to the table, or to \
             UNEXPOSED_TOP_LEVEL with a reason."
        );
    }
}
