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
use crate::spec::{
    ArgForm, ArgKind, ArgSpec, CommandSpec, Derived, GroupShape, GroupSpec, JsonSupport,
};

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
                "`{sub}` argument `{name}` must be hex with an even number of digits, \
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
    // A switch counts as given only when it is `true`. The argv builder omits a `false` switch
    // entirely, so `{"no_diagnostics": false, "diagnostics_hook": "x"}` produces a command line
    // carrying only `--diagnostics-hook`, which clap accepts — rejecting it here would refuse a
    // call that is valid the moment it reaches the CLI. Value arguments keep key-presence
    // semantics: naming one at all is the choice clap conflicts on.
    let present = |set: &[&'static str]| -> Vec<&'static str> {
        set.iter()
            .copied()
            .filter(|name| match args.get(*name) {
                None => false,
                Some(value) => {
                    let is_switch = command
                        .arg(name)
                        .is_some_and(|spec| matches!(spec.form, ArgForm::Switch(_)));
                    !is_switch || value.as_bool() != Some(false)
                }
            })
            .collect()
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
        // Every GameLaunch command is a write too: compiling drives the game to regenerate the
        // cache and then installs the result. Naming only `--allow-game-launch` would send the
        // user to restart with a flag set that this same gate refuses one line further down.
        let (reason, flag) = if required.write && !opts.allow_write {
            (
                "launches the game executable and stages the result in the installation",
                "--allow-game-launch --allow-write",
            )
        } else {
            ("launches the game executable", "--allow-game-launch")
        };
        return Err(BuildError::Refused { path: path.to_string(), reason: reason.into(), flag });
    }
    if required.write && !opts.allow_write {
        let reason = if required.rewrites_in_place {
            let escape = command.safety.in_place_without.unwrap_or("out");
            format!(
                "would overwrite its input in place because `{escape}` was omitted (pass `{escape}` \
                 to write a new file instead)"
            )
        } else {
            "changes the game installation or the shared catalogs the tools read".into()
        };
        return Err(BuildError::Refused { path: path.to_string(), reason, flag: "--allow-write" });
    }

    // A `Write` command is ungated because it creates new files. When its output is already there,
    // it does not create anything — it truncates — so the promise the gate rests on no longer
    // holds and the call has to be treated as a mutation.
    if !opts.allow_write {
        if let Some((name, target)) = existing_target(command, args) {
            return Err(BuildError::Refused {
                path: path.to_string(),
                reason: format!(
                    "`{name}` already exists at `{target}`, and this command overwrites its output \
                     rather than refusing (choose a path that does not exist yet)"
                ),
                flag: "--allow-write",
            });
        }
    }
    Ok(())
}

/// The first output path that is already occupied — named by an argument, or derived from one.
fn existing_target(
    command: &CommandSpec,
    args: &Map<String, Value>,
) -> Option<(&'static str, String)> {
    let named = command.safety.truncates.iter().copied().find_map(|name| {
        let given = args.get(name)?.as_str()?;
        std::path::Path::new(given).exists().then(|| (name, given.to_string()))
    });
    if named.is_some() {
        return named;
    }

    // A derived path is written just as unconditionally as the one the caller named, and the
    // caller cannot avoid it by choosing a different argument value for something else.
    command.safety.derives.iter().copied().find_map(|(name, how)| {
        let given = args.get(name)?.as_str()?;
        let base = std::path::Path::new(given);
        let derived = match how {
            Derived::Extension(extension) => base.with_extension(extension),
            Derived::Child(child) => base.join(child),
            Derived::ChildOfArg(other) => base.join(args.get(other)?.as_str()?),
        };
        derived.exists().then(|| (name, derived.to_string_lossy().into_owned()))
    })
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
            // Either case. The CLI does not agree with itself here — `asset patch-fixed` accepts
            // uppercase (`cmd/asset.rs`, `is_ascii_hexdigit`) while `as patch-default` does not
            // (`cmd/as_cache.rs`, `a`–`f` only) — and this pre-check must not be stricter than the
            // command it guards. Rejecting `1A2B` before spawn would refuse a value the shell
            // accepts; letting it through costs one spawn and yields the CLI's own error, which is
            // the authority. The per-argument help still says lowercase where the CLI insists.
            let valid = !given.is_empty()
                && given.len() % 2 == 0
                && given.bytes().all(|b| b.is_ascii_hexdigit());
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
    fn a_write_command_is_gated_once_its_output_already_exists() {
        // `Class::Write` runs ungated because it creates new files. `gore catalog dump` calls
        // fs::write, so pointing it at an existing path truncates that file — which is a mutation
        // however the class is labelled.
        let dir = tempfile::tempdir().expect("tempdir");
        let fresh = dir.path().join("model.json");
        let occupied = dir.path().join("precious.json");
        std::fs::write(&occupied, b"{}").expect("write fixture");

        let call = |out: &std::path::Path| {
            json!({ "sdk_dir": "SDK", "out": out.to_string_lossy() })
        };

        assert!(
            build_with("gore_catalog", "dump", call(&fresh), &options()).is_ok(),
            "a path that does not exist yet needs no flag"
        );
        assert!(
            matches!(
                build_with("gore_catalog", "dump", call(&occupied), &options()),
                Err(BuildError::Refused { flag: "--allow-write", .. })
            ),
            "an existing target must be gated"
        );
        assert!(
            build_with("gore_catalog", "dump", call(&occupied), &permissive()).is_ok(),
            "--allow-write still permits the overwrite"
        );
    }

    #[test]
    fn a_derived_output_is_gated_even_when_the_named_one_is_fresh() {
        // `texture extract` writes `out` and a sidecar at out.with_extension("png.json"). Deleting
        // the PNG and re-extracting leaves the sidecar behind, so the named output looks fresh
        // while the derived one is about to be replaced.
        let dir = tempfile::tempdir().expect("tempdir");
        let png = dir.path().join("cursor.png");
        let sidecar = dir.path().join("cursor.png.json");

        let call = json!({ "game": "G", "asset": "/Game/T", "out": png.to_string_lossy() });
        assert!(
            build_with("gore_texture", "extract", call.clone(), &options()).is_ok(),
            "nothing exists yet"
        );

        std::fs::write(&sidecar, b"{}").expect("write sidecar");
        let refused = build_with("gore_texture", "extract", call.clone(), &options());
        assert!(
            matches!(refused, Err(BuildError::Refused { flag: "--allow-write", .. })),
            "the sidecar exists and would be overwritten"
        );
        assert!(build_with("gore_texture", "extract", call, &permissive()).is_ok());
    }

    #[test]
    fn scaffolding_over_an_existing_mod_folder_is_gated_but_a_fresh_name_is_not() {
        // The CLI only refuses when `Scripts/main.lua` exists, so an existing non-Lua mod under
        // the same name is entered and its `enabled.txt` truncated. The folder is `<out>/<mod_name>`
        // -- both arguments -- so the collision can be caught without gating ordinary scaffolding.
        let dir = tempfile::tempdir().expect("tempdir");
        let call = |name: &str| json!({ "mod_name": name, "out": dir.path().to_string_lossy() });

        assert!(build_with("gore_project", "scaffold", call("BrandNew"), &options()).is_ok());

        std::fs::create_dir(dir.path().join("Existing")).expect("create mod dir");
        assert!(
            matches!(
                build_with("gore_project", "scaffold", call("Existing"), &options()),
                Err(BuildError::Refused { flag: "--allow-write", .. })
            ),
            "an occupied mod folder must be gated"
        );
        assert!(build_with("gore_project", "scaffold", call("Existing"), &permissive()).is_ok());
    }

    #[test]
    fn a_switch_set_to_false_does_not_trip_an_exclusivity_rule() {
        // The argv builder omits a false switch, so the command line clap sees carries only
        // `--diagnostics-hook` — a call this check used to refuse and the CLI would have accepted.
        let with_false = json!({
            "game": "G",
            "no_diagnostics": false,
            "diagnostics_hook": "hook.dll",
        });
        assert!(build_with("gore_as", "compile", with_false, &permissive()).is_ok());

        let with_true = json!({
            "game": "G",
            "no_diagnostics": true,
            "diagnostics_hook": "hook.dll",
        });
        assert!(
            matches!(
                build_with("gore_as", "compile", with_true, &permissive()),
                Err(BuildError::ExclusiveSet { .. })
            ),
            "a switch that is actually on still conflicts"
        );
    }

    #[test]
    fn passing_the_input_path_as_the_output_does_not_launder_an_in_place_rewrite() {
        // Supplying `out` is what turns `loc import` from "rewrite the .lcache" into "write a new
        // file" — but only if it names somewhere new. Handing it the input's own path is the
        // in-place case again, and the writer underneath replaces that file.
        let dir = tempfile::tempdir().expect("tempdir");
        let lcache = dir.path().join("Alkimia.lcache");
        std::fs::write(&lcache, b"x").expect("write fixture");
        let edits = dir.path().join("edits.json");
        let fresh = dir.path().join("new.lcache");

        let call = |out: &std::path::Path| {
            json!({
                "lcache": lcache.to_string_lossy(),
                "edits": edits.to_string_lossy(),
                "out": out.to_string_lossy(),
            })
        };

        assert!(build_with("gore_loc", "import", call(&fresh), &options()).is_ok());
        assert!(
            matches!(
                build_with("gore_loc", "import", call(&lcache), &options()),
                Err(BuildError::Refused { flag: "--allow-write", .. })
            ),
            "out == in is a rewrite, not a creation"
        );
    }

    #[test]
    fn a_compile_refusal_names_every_flag_the_call_still_needs() {
        // Reporting only --allow-game-launch would send the user off to restart with a flag set
        // this same gate refuses: every GameLaunch command is a write too.
        let neither = build_with(
            "gore_as",
            "compile",
            json!({ "game": "G", "out": "fresh.Cache" }),
            &options(),
        );
        let Err(error) = neither else { panic!("must be refused") };
        let message = error.to_string();
        assert!(message.contains("--allow-game-launch"), "{message}");
        assert!(message.contains("--allow-write"), "{message}");

        // With write already granted, only the launch flag is missing and only it is named.
        let mut write_only = options();
        write_only.allow_write = true;
        let Err(error) =
            build_with("gore_as", "compile", json!({ "game": "G", "out": "fresh.Cache" }), &write_only)
        else {
            panic!("must be refused")
        };
        let message = error.to_string();
        assert!(message.contains("--allow-game-launch"), "{message}");
        assert!(!message.contains("--allow-write"), "{message}");
    }

    #[test]
    fn hex_arguments_accept_either_case_but_still_require_pairs() {
        // `gore asset patch-fixed` validates with `is_ascii_hexdigit`, so uppercase is a value the
        // shell accepts. This pre-check refusing it would be a rejection the CLI never makes.
        let base = |hex: &str| {
            json!({
                "uasset": "in.uasset", "usmap": "m.usmap", "extract_receipt": "r.json",
                "selector": "s.json", "expected_hex": hex, "replacement_hex": "00000000",
                "out": "out.uasset",
            })
        };
        for hex in ["deadbeef", "DEADBEEF", "DeAdBeEf", "1a2B"] {
            assert!(
                build_with("gore_asset", "patch-fixed", base(hex), &permissive()).is_ok(),
                "{hex} should be accepted"
            );
        }
        for bad in ["abc", "zz", "0x1234", "12 34"] {
            assert!(
                matches!(
                    build_with("gore_asset", "patch-fixed", base(bad), &permissive()),
                    Err(BuildError::NotHex { .. })
                ),
                "{bad:?} should be rejected"
            );
        }
        // An empty string never reaches the hex check; `text` rejects it as the wrong type first.
        assert!(matches!(
            build_with("gore_asset", "patch-fixed", base(""), &permissive()),
            Err(BuildError::WrongType { .. })
        ));
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
        // `gen` is deliberately not here: it rewrites a mod folder inside the directory it is
        // given, so it is a mutation. `scaffold` refuses to clobber an existing mod itself.
        assert!(build_with(
            "gore_project",
            "scaffold",
            json!({ "mod_name": "MyMod", "out": "Mods" }),
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
