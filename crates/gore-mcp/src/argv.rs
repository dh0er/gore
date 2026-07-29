//! Turning a `tools/call` into a command line, and refusing the ones this server may not run.
//!
//! Everything here produces messages aimed at a language model rather than at a log: a rejected
//! call comes back as a tool error, and the model's next attempt is only as good as what it was
//! told. So the errors name the argument, state the expected shape, and — for a refusal — say
//! exactly which flag the server would have to be restarted with.

use std::ffi::OsString;
use std::fmt;
use std::time::Duration;

use serde_json::{Map, Value};

use crate::server::Options;
use crate::spec::{ArgForm, ArgKind, ArgSpec, CommandSpec, GroupShape, GroupSpec, JsonSupport};

/// A fully built child-process invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub argv: Vec<OsString>,
    /// The command path without arguments, e.g. `mgr reset` or `dump`. Kept separately from
    /// `display` so a message can name the command without quoting a whole command line back.
    pub path: String,
    pub timeout: Duration,
    /// The command line as a person would type it, echoed in the tool result so the transcript is
    /// reproducible in a shell.
    pub display: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    UnknownSubcommand { tool: &'static str, given: String, available: Vec<&'static str> },
    ArgsNotAnObject { got: &'static str },
    UnknownArgument { sub: &'static str, given: String, known: Vec<&'static str> },
    MissingRequired { sub: &'static str, name: &'static str, kind: String },
    WrongType { sub: &'static str, name: &'static str, expected: String, got: &'static str },
    NotInEnum { sub: &'static str, name: &'static str, allowed: Vec<&'static str>, got: String },
    NotHex { sub: &'static str, name: &'static str, got: String },
    OutOfRange { sub: &'static str, name: &'static str, min: Option<i64>, max: Option<i64>, got: i64 },
    ExclusiveSet {
        sub: &'static str,
        set: Vec<&'static str>,
        given: Vec<String>,
        exactly_one: bool,
    },
    Refused { path: String, reason: String, flag: &'static str },
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::UnknownSubcommand { tool, given, available } => write!(
                f,
                "{tool} has no subcommand `{given}`. Available: {}.",
                available.join(", ")
            ),
            BuildError::ArgsNotAnObject { got } => {
                write!(f, "`args` must be an object, got {got}.")
            }
            BuildError::UnknownArgument { sub, given, known } => {
                write!(f, "`{sub}` has no argument `{given}`.")?;
                if known.is_empty() {
                    write!(f, " It takes no arguments.")
                } else {
                    write!(f, " It accepts: {}.", known.join(", "))
                }
            }
            BuildError::MissingRequired { sub, name, kind } => {
                write!(f, "`{sub}` requires the argument `{name}` ({kind}).")
            }
            BuildError::WrongType { sub, name, expected, got } => {
                write!(f, "`{sub}` argument `{name}` must be {expected}, got {got}.")
            }
            BuildError::NotInEnum { sub, name, allowed, got } => write!(
                f,
                "`{sub}` argument `{name}` must be one of {}, got `{got}`.",
                allowed.join(", ")
            ),
            BuildError::NotHex { sub, name, got } => write!(
                f,
                "`{sub}` argument `{name}` must be lowercase hex with an even number of digits, \
                 got `{got}`."
            ),
            BuildError::OutOfRange { sub, name, min, max, got } => {
                write!(f, "`{sub}` argument `{name}` is out of range (got {got}")?;
                match (min, max) {
                    (Some(min), Some(max)) => write!(f, ", allowed {min}..={max})."),
                    (Some(min), None) => write!(f, ", minimum {min})."),
                    (None, Some(max)) => write!(f, ", maximum {max})."),
                    (None, None) => write!(f, ")."),
                }
            }
            BuildError::ExclusiveSet { sub, set, given, exactly_one } => {
                let names = set.join(" or ");
                if *exactly_one {
                    if given.is_empty() {
                        write!(f, "`{sub}` requires exactly one of {names}; neither was given.")
                    } else {
                        write!(
                            f,
                            "`{sub}` requires exactly one of {names}, but {} were given.",
                            given.join(" and ")
                        )
                    }
                } else {
                    write!(
                        f,
                        "`{sub}` accepts at most one of {names}, but {} were given.",
                        given.join(" and ")
                    )
                }
            }
            BuildError::Refused { path, reason, flag } => write!(
                f,
                "refused: `gore {path}` {reason}, and this MCP server was started without \
                 {flag}.\n\n\
                 Only the user can allow it, by restarting the server with that flag:\n\
                 \n    gore mcp serve {flag}\n\n\
                 Read-only commands and commands that only write new files need no flag."
            ),
        }
    }
}

/// Build the invocation for one `tools/call`.
pub fn build(
    group: &GroupSpec,
    subcommand: &str,
    args: &Value,
    opts: &Options,
) -> Result<Invocation, BuildError> {
    let Some(command) = group.command(subcommand) else {
        return Err(BuildError::UnknownSubcommand {
            tool: group.tool,
            given: subcommand.to_string(),
            available: group.subcommands(),
        });
    };

    let args = match args {
        Value::Object(map) => map.clone(),
        Value::Null => Map::new(),
        other => return Err(BuildError::ArgsNotAnObject { got: type_name(other) }),
    };

    let path = match group.shape {
        GroupShape::Nested => format!("{} {}", group.cli, command.sub),
        GroupShape::Flat => command.sub.to_string(),
    };

    reject_unknown_arguments(command, &args)?;
    check_argument_sets(command, &args)?;
    gate(command, &args, opts, &path)?;

    let mut flags: Vec<OsString> = Vec::new();
    let mut positionals: Vec<(u8, Vec<OsString>)> = Vec::new();

    for spec in command.args {
        let Some(value) = args.get(spec.name) else {
            if spec.required {
                return Err(BuildError::MissingRequired {
                    sub: command.sub,
                    name: spec.name,
                    kind: describe_kind(&spec.kind),
                });
            }
            continue;
        };
        match spec.form {
            ArgForm::Long(flag) => {
                flags.push(long(flag));
                flags.push(scalar(command, spec, value)?.into());
            }
            ArgForm::Switch(flag) => {
                let Value::Bool(enabled) = value else {
                    return Err(BuildError::WrongType {
                        sub: command.sub,
                        name: spec.name,
                        expected: "a boolean".into(),
                        got: type_name(value),
                    });
                };
                // A false switch is simply absent. Emitting `--flag false` would make clap treat
                // the word as the next positional.
                if *enabled {
                    flags.push(long(flag));
                }
            }
            ArgForm::LongRepeated(flag) => {
                for element in list(command, spec, value)? {
                    flags.push(long(flag));
                    flags.push(element.into());
                }
            }
            ArgForm::Positional { order } => {
                positionals.push((order, vec![scalar(command, spec, value)?.into()]));
            }
            ArgForm::PositionalRepeated { order } => {
                let elements =
                    list(command, spec, value)?.into_iter().map(OsString::from).collect();
                positionals.push((order, elements));
            }
        }
    }

    let mut argv: Vec<OsString> = Vec::new();
    if group.shape == GroupShape::Nested {
        argv.push(group.cli.into());
    }
    argv.push(command.sub.into());
    argv.extend(flags);
    argv.extend(command.forced_argv.iter().map(OsString::from));
    if command.json == JsonSupport::Stdout {
        argv.push("--json".into());
    }

    if !positionals.is_empty() {
        // Everything after `--` is a value, never a flag. Model-authored strings reach us
        // unfiltered, and a path that happens to start with a dash would otherwise be parsed as an
        // option and produce a confusing error far from its cause.
        argv.push("--".into());
        positionals.sort_by_key(|(order, _)| *order);
        for (_, values) in positionals {
            argv.extend(values);
        }
    }

    let timeout = Duration::from_secs(if opts.timeout_override_secs > 0 {
        opts.timeout_override_secs
    } else {
        command.timeout_secs
    });

    Ok(Invocation { display: render(&argv), argv, path, timeout })
}

fn reject_unknown_arguments(
    command: &CommandSpec,
    args: &Map<String, Value>,
) -> Result<(), BuildError> {
    for key in args.keys() {
        if command.arg(key).is_none() {
            return Err(BuildError::UnknownArgument {
                sub: command.sub,
                given: key.clone(),
                known: command.args.iter().map(|arg| arg.name).collect(),
            });
        }
    }
    Ok(())
}

/// Enforce the "exactly one of" / "at most one of" constraints clap declares with
/// `required_unless_present` and `conflicts_with`.
fn check_argument_sets(
    command: &CommandSpec,
    args: &Map<String, Value>,
) -> Result<(), BuildError> {
    let present = |set: &[&'static str]| -> Vec<&'static str> {
        set.iter().copied().filter(|name| args.contains_key(*name)).collect()
    };

    for set in command.exactly_one_of {
        let given = present(set);
        if given.len() != 1 {
            return Err(BuildError::ExclusiveSet {
                sub: command.sub,
                set: set.to_vec(),
                given: given.iter().map(|name| name.to_string()).collect(),
                exactly_one: true,
            });
        }
    }
    for set in command.at_most_one_of {
        let given = present(set);
        if given.len() > 1 {
            return Err(BuildError::ExclusiveSet {
                sub: command.sub,
                set: set.to_vec(),
                given: given.iter().map(|name| name.to_string()).collect(),
                exactly_one: false,
            });
        }
    }
    Ok(())
}

fn gate(
    command: &CommandSpec,
    args: &Map<String, Value>,
    opts: &Options,
    path: &str,
) -> Result<(), BuildError> {
    let required = command.safety.requirements(args);

    if required.game_launch && !opts.allow_game_launch {
        return Err(BuildError::Refused {
            path: path.to_string(),
            reason: "launches the game executable".into(),
            flag: "--allow-game-launch",
        });
    }
    if required.write && !opts.allow_write {
        let reason = if required.rewrites_in_place {
            let escape = command.safety.in_place_without.unwrap_or("out");
            format!(
                "would overwrite its input in place because `{escape}` was omitted (pass `{escape}` \
                 to write a new file instead)"
            )
        } else {
            "modifies the game installation".into()
        };
        return Err(BuildError::Refused { path: path.to_string(), reason, flag: "--allow-write" });
    }
    Ok(())
}

fn scalar(command: &CommandSpec, spec: &ArgSpec, value: &Value) -> Result<String, BuildError> {
    match spec.kind {
        ArgKind::Path | ArgKind::Str => text(command, spec, value).map(str::to_string),
        ArgKind::Enum(allowed) => {
            let given = text(command, spec, value)?;
            if allowed.contains(&given) {
                Ok(given.to_string())
            } else {
                Err(BuildError::NotInEnum {
                    sub: command.sub,
                    name: spec.name,
                    allowed: allowed.to_vec(),
                    got: given.to_string(),
                })
            }
        }
        ArgKind::Hex => {
            let given = text(command, spec, value)?;
            let valid = !given.is_empty()
                && given.len() % 2 == 0
                && given.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
            if valid {
                Ok(given.to_string())
            } else {
                Err(BuildError::NotHex {
                    sub: command.sub,
                    name: spec.name,
                    got: given.to_string(),
                })
            }
        }
        ArgKind::Int { min, max } => {
            let Some(given) = value.as_i64() else {
                return Err(BuildError::WrongType {
                    sub: command.sub,
                    name: spec.name,
                    expected: "an integer".into(),
                    got: type_name(value),
                });
            };
            if min.is_some_and(|min| given < min) || max.is_some_and(|max| given > max) {
                return Err(BuildError::OutOfRange {
                    sub: command.sub,
                    name: spec.name,
                    min,
                    max,
                    got: given,
                });
            }
            Ok(given.to_string())
        }
        ArgKind::Bool => Err(BuildError::WrongType {
            sub: command.sub,
            name: spec.name,
            expected: "declared as a switch, not a value".into(),
            got: type_name(value),
        }),
        ArgKind::StrList | ArgKind::IntList => Err(BuildError::WrongType {
            sub: command.sub,
            name: spec.name,
            expected: "declared as a list but used in a scalar position".into(),
            got: type_name(value),
        }),
    }
}

fn list(command: &CommandSpec, spec: &ArgSpec, value: &Value) -> Result<Vec<String>, BuildError> {
    let Some(elements) = value.as_array() else {
        return Err(BuildError::WrongType {
            sub: command.sub,
            name: spec.name,
            expected: "an array".into(),
            got: type_name(value),
        });
    };

    elements
        .iter()
        .map(|element| match spec.kind {
            ArgKind::StrList => text(command, spec, element).map(str::to_string),
            ArgKind::IntList => element.as_i64().map(|n| n.to_string()).ok_or_else(|| {
                BuildError::WrongType {
                    sub: command.sub,
                    name: spec.name,
                    expected: "an array of integers".into(),
                    got: type_name(element),
                }
            }),
            _ => Err(BuildError::WrongType {
                sub: command.sub,
                name: spec.name,
                expected: "a scalar, not an array".into(),
                got: type_name(value),
            }),
        })
        .collect()
}

fn text<'a>(
    command: &CommandSpec,
    spec: &ArgSpec,
    value: &'a Value,
) -> Result<&'a str, BuildError> {
    match value.as_str() {
        Some(text) if !text.is_empty() => Ok(text),
        Some(_) => Err(BuildError::WrongType {
            sub: command.sub,
            name: spec.name,
            expected: "a non-empty string".into(),
            got: "an empty string",
        }),
        None => Err(BuildError::WrongType {
            sub: command.sub,
            name: spec.name,
            expected: "a string".into(),
            got: type_name(value),
        }),
    }
}

fn describe_kind(kind: &ArgKind) -> String {
    match kind {
        ArgKind::Enum(allowed) => format!("one of {}", allowed.join(", ")),
        other => other.label().to_string(),
    }
}

fn long(flag: &str) -> OsString {
    format!("--{flag}").into()
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// Render the command line for display. Quoting is cosmetic — the child never sees a shell — but a
/// token with a space in it has to look quoted or the echoed line is not copy-pasteable.
fn render(argv: &[OsString]) -> String {
    let mut line = String::from("gore");
    for token in argv {
        let token = token.to_string_lossy();
        line.push(' ');
        if token.is_empty() || token.contains(char::is_whitespace) || token.contains('"') {
            line.push('"');
            line.push_str(&token.replace('"', "\\\""));
            line.push('"');
        } else {
            line.push_str(&token);
        }
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec;
    use serde_json::json;
    use std::path::PathBuf;

    fn options() -> Options {
        Options::new(PathBuf::from("gore"), "0.1.0")
    }

    fn permissive() -> Options {
        let mut opts = options();
        opts.allow_write = true;
        opts.allow_game_launch = true;
        opts
    }

    fn build_with(
        tool: &str,
        sub: &str,
        args: Value,
        opts: &Options,
    ) -> Result<Invocation, BuildError> {
        build(spec::group(tool).expect("group exists"), sub, &args, opts)
    }

    fn argv_of(tool: &str, sub: &str, args: Value) -> Vec<String> {
        build_with(tool, sub, args, &permissive())
            .expect("should build")
            .argv
            .iter()
            .map(|token| token.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn a_nested_group_emits_its_cli_token_and_a_flat_group_does_not() {
        assert_eq!(argv_of("gore_config", "path", json!({})), vec!["config", "path"]);
        assert_eq!(
            argv_of("gore_catalog", "dump", json!({ "sdk_dir": "SDK", "out": "model.json" })),
            vec!["dump", "--out", "model.json", "--", "SDK"]
        );
    }

    #[test]
    fn positionals_are_ordered_and_separated_from_flags() {
        assert_eq!(
            argv_of(
                "gore_catalog",
                "catalog",
                json!({ "kind": "knowledge", "dump": "d.txt", "out": "c.json", "script_cache": "S.Cache" })
            ),
            vec![
                "catalog",
                "--kind",
                "knowledge",
                "--script-cache",
                "S.Cache",
                "--out",
                "c.json",
                "--",
                "d.txt",
            ]
        );
    }

    #[test]
    fn multiple_positionals_keep_their_declared_order() {
        assert_eq!(
            argv_of("gore_config", "set", json!({ "key": "game-path", "value": "D:/G1R" })),
            vec!["config", "set", "--", "game-path", "D:/G1R"]
        );
    }

    #[test]
    fn omitted_optional_arguments_are_simply_absent() {
        assert_eq!(
            argv_of("gore_catalog", "stubs", json!({ "model": "m.json", "out": "stubs" })),
            vec!["stubs", "--out", "stubs", "--", "m.json"]
        );
    }

    #[test]
    fn a_missing_required_argument_names_it_and_its_type() {
        let error = build_with("gore_catalog", "dump", json!({ "sdk_dir": "SDK" }), &permissive())
            .unwrap_err();
        assert!(matches!(error, BuildError::MissingRequired { name: "out", .. }));
        assert!(error.to_string().contains("`out`"));
    }

    #[test]
    fn an_unknown_argument_lists_the_ones_that_exist() {
        let error =
            build_with("gore_config", "path", json!({ "nope": 1 }), &permissive()).unwrap_err();
        assert!(error.to_string().contains("takes no arguments"), "{error}");

        let error = build_with("gore_catalog", "dump", json!({ "sdkdir": "x" }), &permissive())
            .unwrap_err();
        assert!(error.to_string().contains("sdk_dir"), "{error}");
    }

    #[test]
    fn an_unknown_subcommand_lists_the_available_ones() {
        let error =
            build_with("gore_config", "delete", json!({}), &permissive()).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("delete"));
        assert!(message.contains("unset"), "{message}");
    }

    #[test]
    fn a_value_outside_the_enum_is_rejected_with_the_allowed_set() {
        let error = build_with(
            "gore_catalog",
            "catalog",
            json!({ "kind": "weapon", "dump": "d.txt", "out": "c.json" }),
            &permissive(),
        )
        .unwrap_err();
        let message = error.to_string();
        assert!(message.contains("weapon"));
        assert!(message.contains("knowledge"), "{message}");
    }

    #[test]
    fn wrong_json_types_are_reported_rather_than_stringified() {
        let error = build_with(
            "gore_catalog",
            "dump",
            json!({ "sdk_dir": 42, "out": "m.json" }),
            &permissive(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("must be a string"), "{error}");
    }

    #[test]
    fn args_must_be_an_object_but_may_be_omitted() {
        assert!(build_with("gore_config", "path", Value::Null, &permissive()).is_ok());
        let error =
            build_with("gore_config", "path", json!([1, 2]), &permissive()).unwrap_err();
        assert!(matches!(error, BuildError::ArgsNotAnObject { .. }));
    }

    #[test]
    fn an_install_mutating_command_is_refused_without_allow_write() {
        let error =
            build_with("gore_project", "deploy-shared", json!({}), &options()).unwrap_err();
        let message = error.to_string();
        assert!(message.starts_with("refused:"), "{message}");
        assert!(message.contains("--allow-write"), "{message}");
        assert!(message.contains("gore mcp serve --allow-write"), "{message}");
    }

    #[test]
    fn a_refusal_names_the_command_the_way_a_user_would_type_it() {
        // "`reset` modifies the game installation" is ambiguous — several groups have a command
        // that could be called that. The full path is what a user can act on.
        let nested = build_with("gore_mgr", "reset", json!({}), &options()).unwrap_err();
        assert!(nested.to_string().contains("`gore mgr reset`"), "{nested}");

        let flat = build_with("gore_project", "deploy-shared", json!({}), &options()).unwrap_err();
        assert!(flat.to_string().contains("`gore deploy-shared`"), "{flat}");
    }

    #[test]
    fn a_game_launching_command_needs_both_flags_when_it_also_writes_in_place() {
        // `as compile` without `out` installs the fresh cache over the game's own. The launch gate
        // is reported first because it is the more surprising of the two.
        let mut launch_only = options();
        launch_only.allow_game_launch = true;

        let blocked = build_with("gore_as", "compile", json!({}), &options()).unwrap_err();
        assert!(blocked.to_string().contains("--allow-game-launch"), "{blocked}");

        let still_blocked = build_with("gore_as", "compile", json!({}), &launch_only).unwrap_err();
        assert!(still_blocked.to_string().contains("--allow-write"), "{still_blocked}");

        let mut both = launch_only.clone();
        both.allow_write = true;
        assert!(build_with("gore_as", "compile", json!({}), &both).is_ok());
    }

    #[test]
    fn the_invocation_carries_the_command_path_separately_from_the_command_line() {
        let nested = build_with(
            "gore_config",
            "set",
            json!({ "key": "game-path", "value": "D:/G1R" }),
            &permissive(),
        )
        .unwrap();
        assert_eq!(nested.path, "config set");

        let flat = build_with("gore_catalog", "dump", json!({ "sdk_dir": "S", "out": "m.json" }), &permissive())
            .unwrap();
        assert_eq!(flat.path, "dump");
    }

    #[test]
    fn the_same_command_builds_once_allow_write_is_set() {
        let mut opts = options();
        opts.allow_write = true;
        let invocation =
            build_with("gore_project", "deploy-shared", json!({}), &opts).expect("permitted");
        assert_eq!(invocation.display, "gore deploy-shared");
    }

    #[test]
    fn commands_that_only_write_new_files_need_no_flag() {
        assert!(build_with(
            "gore_project",
            "gen",
            json!({ "overrides": "o.toml", "out": "Mods" }),
            &options()
        )
        .is_ok());
    }

    #[test]
    fn the_timeout_override_replaces_the_per_command_default() {
        let fast = build_with("gore_config", "path", json!({}), &options()).unwrap();
        assert_eq!(fast.timeout, Duration::from_secs(spec::T_FAST));

        let mut opts = options();
        opts.timeout_override_secs = 5;
        let overridden = build_with("gore_config", "path", json!({}), &opts).unwrap();
        assert_eq!(overridden.timeout, Duration::from_secs(5));
    }

    #[test]
    fn the_display_line_is_a_command_a_person_could_paste() {
        let invocation = build_with(
            "gore_config",
            "set",
            json!({ "key": "game-path", "value": "D:/Program Files/G1R" }),
            &permissive(),
        )
        .unwrap();
        assert_eq!(
            invocation.display,
            "gore config set -- game-path \"D:/Program Files/G1R\""
        );
    }

    #[test]
    fn every_argument_pairs_a_form_with_a_compatible_kind() {
        // The argv builder relies on this: a `Switch` reads a bool, a repeated form reads an array,
        // and everything else reads a scalar. A mismatched pair in the table would surface as a
        // confusing runtime type error instead of a compile-time one, so assert it here.
        for group in spec::GROUPS {
            for command in group.commands {
                for arg in command.args {
                    let ok = match arg.form {
                        ArgForm::Switch(_) => arg.kind == ArgKind::Bool,
                        ArgForm::LongRepeated(_) | ArgForm::PositionalRepeated { .. } => {
                            matches!(arg.kind, ArgKind::StrList | ArgKind::IntList)
                        }
                        ArgForm::Long(_) | ArgForm::Positional { .. } => !matches!(
                            arg.kind,
                            ArgKind::Bool | ArgKind::StrList | ArgKind::IntList
                        ),
                    };
                    assert!(
                        ok,
                        "{}/{}: argument `{}` pairs {:?} with {:?}",
                        group.tool, command.sub, arg.name, arg.form, arg.kind
                    );
                }
            }
        }
    }
}
