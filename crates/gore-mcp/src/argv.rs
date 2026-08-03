//! Turning a `tools/call` into a command line, and spotting the ones a person should agree to first.
//!
//! Everything here produces messages aimed at a language model rather than at a log: a rejected
//! call comes back as a tool error, and the model's next attempt is only as good as what it was
//! told. So the errors name the argument and state the expected shape.
//!
//! The safety gate is the exception, because its output is read by a *person*. It does not reject;
//! it attaches a [`Consent`] to the invocation describing what the command would overwrite, install
//! or launch, and [`crate::consent`] puts that in front of the user. Whether the answer even gets
//! asked for is decided by how the server was started — see [`Options::pre_approves`].

use std::ffi::OsString;
use std::fmt;
use std::time::Duration;

use serde_json::{Map, Value};

use crate::consent::{Consent, Needs};
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
    /// Set when a person has to agree before this runs. `None` means the call is either harmless or
    /// already pre-approved by the flags the server was started with.
    ///
    /// Carried on the invocation rather than returned as an error because it is not a failure: the
    /// command line is complete and correct, and the only open question is whether to run it.
    pub consent: Option<Consent>,
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
    // Dropped rather than never computed, so that turning a flag on cannot change which arm the
    // gate would have matched — only whether anyone is asked about it. The command line it shows is
    // filled in below, once there is one.
    let mut consent = consent_for(command, &args, &path)
        .filter(|consent| !opts.pre_approves(&consent.needs));

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

    // One rendering, used in both places: what the user is asked about and what the tool result
    // reports as having run are then the same line by construction, not by agreement.
    let display = render(&opts.exe, &argv);
    if let Some(consent) = consent.as_mut() {
        consent.command_line = display.clone();
    }

    Ok(Invocation { display, argv, path, timeout, consent })
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

/// What a person would have to agree to before this call may run, if anything.
///
/// Deliberately independent of how the server was started. Whether the question is *asked* is a
/// separate decision made by the caller, and keeping the two apart means a pre-approved server
/// still computes the same answer — so `--allow-write` widens who may say yes, never what the gate
/// can see.
///
/// The arms are ordered most-specific-first and the first match wins. A command that trips arm two
/// would also trip arm four, but what the command itself does is the truer sentence than "the
/// output file already exists", and only one question gets asked.
fn consent_for(
    command: &CommandSpec,
    args: &Map<String, Value>,
    path: &str,
) -> Option<Consent> {
    let required = command.safety.requirements(args);
    let question = |reason: String, remedy: Option<String>, needs: Needs| {
        Some(Consent {
            path: path.to_string(),
            reason,
            remedy,
            // Not known here; `build` fills it in once the argv is assembled.
            command_line: String::new(),
            needs,
        })
    };

    if required.game_launch {
        // Every GameLaunch command is a write too: compiling drives the game to regenerate the
        // cache and then installs the result. `Needs::flags` is what keeps the launch flag from
        // ever being named on its own.
        let reason = if required.write {
            "launches the game executable and stages the result in the installation"
        } else {
            "launches the game executable"
        };
        return question(
            reason.into(),
            None,
            Needs { write: required.write, game_launch: true },
        );
    }

    if required.write {
        let needs = Needs { write: true, game_launch: false };
        if required.rewrites_in_place {
            let escape = command.safety.in_place_without.unwrap_or("out");
            return question(
                format!("would overwrite its input in place because `{escape}` was omitted"),
                Some(format!("Pass `{escape}` to write a new file instead")),
                needs,
            );
        }
        // The command's own sentence, from the table. The fallback is only reachable for a command
        // that escalated into this arm without a base class the spec test covers; every command
        // classified as a mutation carries its own.
        let reason = command
            .gated_because
            .unwrap_or("changes the game installation or the shared catalogs the tools read");
        return question(reason.into(), None, needs);
    }

    // Some outputs are only harmless because of where they usually point. Aim one at the game tree
    // and the command has installed something, which is what the question is really about.
    if let Some((name, target)) = installs_into_game_tree(command, args) {
        return question(
            format!(
                "`{name}` points at `{target}`, inside the game installation, so writing there \
                 installs the result instead of producing a file to deploy later"
            ),
            Some(format!(
                "Point `{name}` outside the installation to produce a file to deploy later"
            )),
            Needs { write: true, game_launch: false },
        );
    }

    // A `Write` command needs no question because it creates new files. When its output is already
    // there, it does not create anything — it truncates — so the promise that made it harmless no
    // longer holds and the call has to be treated as a mutation.
    if let Some((name, occupancy)) = occupied_target(command, args) {
        let needs = Needs { write: true, game_launch: false };
        return match occupancy {
            Occupancy::Existing(target) => question(
                format!(
                    "`{name}` already exists at `{target}`, and this command overwrites its output \
                     rather than refusing"
                ),
                Some("Choose a path that does not exist yet".into()),
                needs,
            ),
            // Kept apart from the arm above, which reads "`out` already exists at …" and would then
            // print a path that is not `out`'s value — `mod build` names the bundle directory and
            // writes the folder underneath it.
            Occupancy::ExistingDerived(target) => question(
                format!(
                    "writes `{target}`, a path it derives from `{name}` rather than being given, \
                     and that path already exists — so this command replaces it rather than \
                     refusing"
                ),
                Some(format!("Point `{name}` somewhere that does not hold it yet")),
                needs,
            ),
            Occupancy::NonEmptyDir(target) => question(
                format!(
                    "would write into `{target}`, which is not empty, under names it takes from \
                     the data it reads rather than from its arguments — so anything already there \
                     that it collides with is overwritten"
                ),
                Some(format!("Point `{name}` at a new or empty directory")),
                needs,
            ),
            Occupancy::Unreadable { source, target } => question(
                format!(
                    "writes into `{target}` under a name it reads out of `{source}`, and that name \
                     could not be read from here — so what it would replace cannot be checked first"
                ),
                Some(format!("Make `{source}` readable, or check `{target}` yourself")),
                needs,
            ),
        };
    }

    None
}

/// The first output argument aimed inside a game installation.
///
/// Recognised two ways, because the server never resolves the game path itself: the call passed an
/// explicit `game` and the output sits under it, or the path simply contains a `G1R` component,
/// which is the folder every Gothic 1 Remake install is identified by throughout this toolkit.
fn installs_into_game_tree(
    command: &CommandSpec,
    args: &Map<String, Value>,
) -> Option<(&'static str, String)> {
    let game = args
        .get("game")
        .and_then(Value::as_str)
        .and_then(|root| resolve(std::path::Path::new(root)));

    // Every declared output, not just the ones registered here. `installs_via` names the outputs
    // whose destination is the *only* thing that matters; a truncating or derived output is just as
    // much an installation change when it lands in the game tree, and enumerating those separately
    // is how each of the last several rounds found one more command that had been missed.
    //
    // Derived paths are computed before they are checked. Mapping them back to the argument name
    // would test the base path — and `texture extract`'s sidecar is a different file, which may be
    // a link into the installation while the PNG beside it is not.
    output_paths(command, args).into_iter().find_map(|(name, path)| {
        // Resolved first, always. A relative output is resolved by the *child* against this
        // process's working directory, so comparing it lexically would miss `--out .` run from
        // inside the installation — which is the same deployment by a shorter name.
        let Some(path) = resolve(&path) else {
            // Too many links to follow. Where the write lands is unknown, so it is gated.
            return Some((name, "a symlink chain too deep to follow".to_string()));
        };

        let under_game = game.as_ref().is_some_and(|root| path.starts_with(root));
        let names_the_game_folder =
            path.components().any(|part| part.as_os_str().eq_ignore_ascii_case("G1R"));

        (under_game || names_the_game_folder).then(|| (name, path.to_string_lossy().into_owned()))
    })
}

/// Every path this call writes: the ones an argument names, and the ones it derives from them.
fn output_paths(
    command: &CommandSpec,
    args: &Map<String, Value>,
) -> Vec<(&'static str, std::path::PathBuf)> {
    let named = command
        .safety
        .installs_via
        .iter()
        .copied()
        .chain(command.safety.truncates.iter().copied())
        // A directory filled with names of the command's own choosing is still a directory being
        // written into, and aiming one at the installation installs whatever lands there.
        .chain(command.safety.clobbers_dir.iter().copied());

    let mut paths: Vec<(&'static str, std::path::PathBuf)> = named
        .filter_map(|name| {
            let given = args.get(name)?.as_str()?;
            Some((name, std::path::PathBuf::from(given)))
        })
        .collect();

    paths.extend(command.safety.derives.iter().filter_map(|(name, how)| {
        let given = args.get(*name)?.as_str()?;
        // An underivable last component leaves the directory it would have gone in, which is the
        // part that decides whether this lands in the game tree.
        let derived = match derived_target(args, std::path::Path::new(given), *how) {
            DerivedTarget::At(path) => path,
            DerivedTarget::Unknown { .. } => std::path::PathBuf::from(given),
        };
        Some((*name, derived))
    }));

    paths
}

/// Make a path absolute, and resolve symlinks as far down as it already exists.
///
/// `canonicalize` alone is not enough: an output directory is usually the thing about to be
/// created, so it does not exist yet and canonicalizing fails. Resolving the deepest existing
/// ancestor and re-attaching the rest gets the symlinked parents — a junction pointing into the
/// game folder is exactly how this check would otherwise be walked around.
/// `None` when the link chain is too deep to follow.
///
/// The caller must treat that as "could be anywhere", not as "fine": returning the unresolved path
/// would let a chain one hop longer than the budget walk straight past the gate.
fn resolve(path: &std::path::Path) -> Option<std::path::PathBuf> {
    resolve_following_links(path, 0)
}

/// How many symlink hops to follow before giving up. Loops are the reason there is a limit at all.
const MAX_LINK_HOPS: u8 = 8;

fn resolve_following_links(path: &std::path::Path, hops: u8) -> Option<std::path::PathBuf> {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());

    let mut existing = absolute.as_path();
    let mut trailing: Vec<&std::ffi::OsStr> = Vec::new();
    loop {
        if let Ok(real) = existing.canonicalize() {
            let mut resolved = real;
            resolved.extend(trailing.iter().rev());

            // The last component can be a symlink whose target does not exist *yet*. `canonicalize`
            // refuses those, so the loop above stops at the parent and reattaches the link's own
            // name — which is not where the write lands. `fs::write` follows the link, so a
            // dangling link pointing into the installation would otherwise pass both this check
            // and the existence check and then create the file the game reads.
            if let Ok(metadata) = std::fs::symlink_metadata(&resolved) {
                if metadata.file_type().is_symlink() {
                    if hops >= MAX_LINK_HOPS {
                        // Out of budget with a link still in front of us: where this lands is
                        // unknown, and unknown must not read as safe.
                        return None;
                    }
                    if let Ok(target) = std::fs::read_link(&resolved) {
                        let followed = if target.is_absolute() {
                            target
                        } else {
                            resolved.parent().unwrap_or(std::path::Path::new("")).join(target)
                        };
                        return resolve_following_links(&followed, hops + 1);
                    }
                    return None;
                }
            }
            return Some(resolved);
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                trailing.push(name);
                existing = parent;
            }
            _ => return Some(absolute),
        }
    }
}

/// What a `Write` command would destroy after all, and how the gate found out.
enum Occupancy {
    /// A path an argument names is already a file or a directory.
    Existing(String),
    /// The same, for a path the command works out for itself. Separate because the sentence has to
    /// be: naming the argument as the thing that exists is false as soon as the two differ.
    ExistingDerived(String),
    /// A directory this call fills with names of its own choosing already holds something.
    NonEmptyDir(String),
    /// A path could not be worked out, because the file its last component is named in did not
    /// yield one. Not knowing is not the same as nothing being there, and only one of the two is
    /// safe to assume.
    Unreadable { source: String, target: String },
}

/// The first output this call would destroy something at — named by an argument, derived from one,
/// or sitting inside a directory it fills under names of its own.
fn occupied_target(
    command: &CommandSpec,
    args: &Map<String, Value>,
) -> Option<(&'static str, Occupancy)> {
    let named = command.safety.truncates.iter().copied().find_map(|name| {
        let given = args.get(name)?.as_str()?;
        std::path::Path::new(given)
            .exists()
            .then(|| (name, Occupancy::Existing(given.to_string())))
    });
    if named.is_some() {
        return named;
    }

    // A derived path is written just as unconditionally as the one the caller named, and the
    // caller cannot avoid it by choosing a different argument value for something else.
    let derived = command.safety.derives.iter().copied().find_map(|(name, how)| {
        let given = args.get(name)?.as_str()?;
        match derived_target(args, std::path::Path::new(given), how) {
            DerivedTarget::At(path) => path
                .exists()
                .then(|| (name, Occupancy::ExistingDerived(path.to_string_lossy().into_owned()))),
            DerivedTarget::Unknown { source } => Some((
                name,
                Occupancy::Unreadable { source, target: given.to_string() },
            )),
        }
    });
    if derived.is_some() {
        return derived;
    }

    // Last, because it is the weakest claim of the three: something is in the way, but which file
    // the command will collide with is exactly what cannot be known before it runs.
    command.safety.clobbers_dir.iter().copied().find_map(|name| {
        let given = args.get(name)?.as_str()?;
        let entries = std::fs::read_dir(given).ok()?;
        // `read_dir` failing is not occupancy: a directory that is not there yet is the ordinary
        // case this whole check exists to let through ungated.
        entries
            .flatten()
            .next()
            .map(|_| (name, Occupancy::NonEmptyDir(given.to_string())))
    })
}

/// Where a derived output lands, or why the gate could not work it out.
enum DerivedTarget {
    At(std::path::PathBuf),
    /// Names the file that was supposed to supply the missing component.
    Unknown { source: String },
}

/// Resolve one [`Derived`] shape against the arguments it may need.
fn derived_target(
    args: &Map<String, Value>,
    base: &std::path::Path,
    how: Derived,
) -> DerivedTarget {
    match how {
        Derived::ChildOfArg(other) => match args.get(other).and_then(Value::as_str) {
            Some(child) => DerivedTarget::At(base.join(child)),
            // A missing argument is a call clap will reject anyway; nothing is derived from it.
            None => DerivedTarget::At(base.to_path_buf()),
        },
        Derived::ChildNamedInJson { arg, pointer } => {
            let source = args.get(arg).and_then(Value::as_str).unwrap_or(arg).to_string();
            match name_in_json(&source, pointer) {
                Some(name) => DerivedTarget::At(base.join(name)),
                None => DerivedTarget::Unknown { source },
            }
        }
        Derived::Extension(extension) => DerivedTarget::At(base.with_extension(extension)),
        Derived::Child(child) => DerivedTarget::At(base.join(child)),
    }
}

/// Read one string out of a JSON file, and refuse anything that would not be a single path
/// component.
///
/// A name carrying a separator, a drive letter or `..` would make the derived path point somewhere
/// other than inside the directory the caller named — so the honest answer there is "unknown",
/// which fails closed, rather than a path the check would then look for in the wrong place.
fn name_in_json(path: &str, pointer: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let document: Value = serde_json::from_str(&text).ok()?;
    let name = document.pointer(pointer)?.as_str()?;
    let mut components = std::path::Path::new(name).components();
    match (components.next(), components.next()) {
        (Some(std::path::Component::Normal(only)), None) if only == name => Some(name.to_string()),
        _ => None,
    }
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
/// The command line as a person would type it into PowerShell.
///
/// Every guide example is PowerShell and the tool result presents this as something to re-run, so
/// it has to be safe to paste. The child is never started through a shell — `Command` takes the
/// argv directly — but a displayed token like `C:\mods;whoami` would end the command at the
/// semicolon and run the rest, and these tokens come from a model.
///
/// Anything that is not plainly a bare word is single-quoted. PowerShell treats a single-quoted
/// string as literal, so `'` doubled is the only escape there is.
pub fn render(exe: &std::path::Path, argv: &[OsString]) -> String {
    let mut line = invoke_program(exe);
    for token in argv {
        line.push(' ');
        line.push_str(&quote_for_powershell(&token.to_string_lossy()));
    }
    line
}

/// The program as PowerShell needs it written in order to actually run.
///
/// The documented setup points a client at an unpacked `gore.exe` by absolute path, which need not
/// be on PATH — so the line has to name that binary. But a quoted path is a *string literal* to
/// PowerShell, not a command: pasted as-is it prints the path and stops. `&` is the call operator
/// that turns it back into an invocation. A bare word runs on its own and gains nothing from it.
pub fn invoke_program(exe: &std::path::Path) -> String {
    let quoted = quote_for_powershell(&exe.to_string_lossy());
    if quoted.starts_with('\'') {
        format!("& {quoted}")
    } else {
        quoted
    }
}

pub fn quote_for_powershell(token: &str) -> String {
    // A bare word is left alone: quoting `--out` or `textures` would only make the line harder to
    // read, and the whole point of showing it is that a person can use it.
    let bare = !token.is_empty()
        && token.chars().all(|character| {
            // `@` is deliberately absent: PowerShell reads a leading `@` as splatting, so `@args`
            // pasted bare expands a variable instead of passing the characters the child got.
            character.is_ascii_alphanumeric() || "-_./\\:=+,".contains(character)
        });
    if bare {
        return token.to_string();
    }
    format!("'{}'", token.replace('\'', "''"))
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

    /// The question a call would raise, or `None` when it may simply run.
    ///
    /// Building must succeed either way: needing someone's agreement is not a malformed call, it is
    /// a complete command line with one thing left to settle.
    #[track_caller]
    fn question(tool: &str, sub: &str, args: Value, opts: &Options) -> Option<Consent> {
        build_with(tool, sub, args, opts)
            .expect("the command line itself is valid")
            .consent
    }

    /// Whether a call asks about a write - the shape all but one gate arm produces.
    fn asks_about_a_write(raised: Option<Consent>) -> bool {
        raised.is_some_and(|consent| consent.needs == Needs { write: true, game_launch: false })
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
            question("gore_catalog", "dump", call(&fresh), &options()).is_none(),
            "a path that does not exist yet is nobody's business"
        );
        assert!(
            asks_about_a_write(question("gore_catalog", "dump", call(&occupied), &options())),
            "an existing target must be put to the user"
        );
        assert!(
            question("gore_catalog", "dump", call(&occupied), &permissive()).is_none(),
            "--allow-write pre-approves the overwrite, so nobody is asked"
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
            question("gore_texture", "extract", call.clone(), &options()).is_none(),
            "nothing exists yet"
        );

        std::fs::write(&sidecar, b"{}").expect("write sidecar");
        assert!(
            asks_about_a_write(question("gore_texture", "extract", call.clone(), &options())),
            "the sidecar exists and would be overwritten"
        );
        assert!(question("gore_texture", "extract", call, &permissive()).is_none());
    }

    #[test]
    fn packing_into_the_game_tree_is_a_deployment_and_is_gated() {
        // `texture pack -o ./build` is an artifact; `texture pack -o <game>/G1R/Content/Paks/~mods`
        // is the live override the game mounts, which is what `texture deploy` needs a flag for.
        let call = |out: &str| {
            json!({ "game": "D:/Games/G1R", "mod_dir": "mod", "name": "zzz_Mine_P", "out": out })
        };

        assert!(question("gore_texture", "pack", call("build/triplet"), &options()).is_none());

        for inside in [
            "D:/Games/G1R/G1R/Content/Paks/~mods",
            "D:/Games/G1R/anything",              // under the explicitly passed `game`
            "E:/elsewhere/G1R/Content/Paks/~mods", // names the game folder outright
        ] {
            assert!(
                asks_about_a_write(question("gore_texture", "pack", call(inside), &options())),
                "{inside} should be treated as an installation change"
            );
        }

        assert!(question(
            "gore_texture",
            "pack",
            call("D:/Games/G1R/G1R/Content/Paks/~mods"),
            &permissive()
        )
        .is_none());
    }

    #[test]
    fn no_error_message_carries_stray_whitespace() {
        // These strings are written across source lines with a trailing `\`, and dropping one
        // leaves the continuation's indentation inside the message. The model reads these; a
        // sentence with twenty spaces in the middle of it is not what it should be parsing.
        let errors = [
            BuildError::UnknownSubcommand {
                tool: "gore_as",
                given: "nope".into(),
                available: vec!["compile"],
            },
            BuildError::ArgsNotAnObject { got: "a string" },
            BuildError::UnknownArgument { sub: "dump", given: "x".into(), known: vec!["out"] },
            BuildError::MissingRequired { sub: "dump", name: "out", kind: "a path".into() },
            BuildError::WrongType {
                sub: "dump",
                name: "out",
                expected: "a path".into(),
                got: "a number",
            },
            BuildError::NotInEnum {
                sub: "catalog",
                name: "kind",
                allowed: vec!["item"],
                got: "x".into(),
            },
            BuildError::NotHex { sub: "patch-fixed", name: "expected_hex", got: "zz".into() },
            BuildError::OutOfRange {
                sub: "inspect",
                name: "export_index",
                min: Some(0),
                max: None,
                got: -1,
            },
            BuildError::ExclusiveSet {
                sub: "extract",
                set: vec!["basename", "path"],
                given: vec!["basename".into(), "path".into()],
                exactly_one: true,
            },
        ];

        for error in errors {
            let rendered = error.to_string();
            for line in rendered.lines() {
                let body = line.trim_start();
                assert!(
                    !body.contains("  "),
                    "{error:?} renders a run of spaces mid-line: {line:?}"
                );
            }
        }
    }

    #[test]
    fn a_question_states_what_this_command_does_and_not_what_the_family_does() {
        // The refusal a real session produced: `audio extract --out <a temp folder>` came back as
        // "changes the game installation or the shared catalogs the tools read". It changes
        // neither, and the assistant — which had read the arguments — had to contradict its own
        // server to the user it was working for. The truth was in the table all along, as a comment
        // above the entry.
        let dir = tempfile::tempdir().expect("tempdir");
        let occupied = dir.path().join("sfx");
        std::fs::create_dir_all(&occupied).expect("mkdir");
        std::fs::write(occupied.join("0_Kept.wav"), b"edited").expect("write");

        let raised = question(
            "gore_audio",
            "extract",
            json!({ "bank": "SFX.bank", "out": occupied.to_string_lossy() }),
            &options(),
        )
        .expect("extracting over files already in the directory is asked about");

        assert!(
            !raised.reason.contains("changes the game installation or the shared catalogs"),
            "the one-size-fits-all reason is back: {}",
            raised.reason
        );
        assert!(raised.reason.contains("is not empty"), "{}", raised.reason);
        assert!(raised.reason.contains(&*occupied.to_string_lossy()), "{}", raised.reason);

        // And a command that really does touch the installation still says so, in its own terms.
        let deploy = question("gore_mod", "deploy", json!({ "bundle": "b" }), &options())
            .expect("deploying is asked about");
        assert!(deploy.reason.contains("installs the bundle into the game"), "{}", deploy.reason);
        assert_ne!(deploy.reason, raised.reason, "two commands, two reasons");
    }

    #[test]
    fn a_name_choosing_writer_aimed_at_an_empty_directory_asks_nobody() {
        // The reason this whole facet exists. Three confirmations were spent building one test mod,
        // and two of them were for commands that could not have destroyed anything: the scratch
        // directories they wrote into did not exist yet. Every one of those questions was answered
        // by the client itself, in milliseconds, without reaching a person — so the cost was not a
        // dialog, it was a refusal the assistant then had to work around.
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("does-not-exist-yet");
        let empty = dir.path().join("empty");
        std::fs::create_dir_all(&empty).expect("mkdir");

        for out in [&missing, &empty] {
            assert!(
                question(
                    "gore_audio",
                    "extract",
                    json!({ "bank": "SFX.bank", "out": out.to_string_lossy() }),
                    &options(),
                )
                .is_none(),
                "{} holds nothing to overwrite",
                out.display()
            );
        }

        // The directory being outside the installation is not what makes it safe — an empty one
        // inside the game tree is still a directory the game reads.
        let inside = dir.path().join("G1R").join("Content");
        std::fs::create_dir_all(&inside).expect("mkdir");
        assert!(
            question(
                "gore_audio",
                "extract",
                json!({ "bank": "SFX.bank", "out": inside.to_string_lossy() }),
                &options(),
            )
            .is_some(),
            "an empty destination inside the installation is still an installation change"
        );
    }

    #[test]
    fn building_a_bundle_asks_about_the_rebuild_rather_than_the_build() {
        // `mod build` deletes `<out>/<meta.name>` before writing it. The name is in the spec, which
        // is JSON, so the gate reads it: a first build destroys nothing and says nothing.
        let dir = tempfile::tempdir().expect("tempdir");
        let spec = dir.path().join("spec.json");
        std::fs::write(&spec, br#"{"meta":{"name":"DaniTestMod","version":"1.0.0"}}"#).expect("write");
        let out = dir.path().join("build");
        let call = || {
            json!({ "spec": spec.to_string_lossy(), "out": out.to_string_lossy() })
        };

        assert!(
            question("gore_mod", "build", call(), &options()).is_none(),
            "a bundle folder that is not there yet is not something to confirm deleting"
        );

        std::fs::create_dir_all(out.join("DaniTestMod")).expect("mkdir");
        let raised = question("gore_mod", "build", call(), &options())
            .expect("rebuilding over an existing bundle folder is asked about");
        assert!(raised.reason.contains("DaniTestMod"), "{}", raised.reason);
        // The sentence for a *named* output would read "`out` already exists at …" and then print
        // a path that is not what `out` was set to. A person reading that would go looking for the
        // wrong folder.
        assert!(
            !raised.reason.starts_with("`out` already exists"),
            "the derived path is being described as the argument's own: {}",
            raised.reason
        );
        assert!(raised.reason.contains("derives from `out`"), "{}", raised.reason);

        // A different mod name in the same output directory is a different folder, and untouched.
        std::fs::write(&spec, br#"{"meta":{"name":"Other","version":"1.0.0"}}"#).expect("write");
        assert!(
            question("gore_mod", "build", call(), &options()).is_none(),
            "the collision is with one folder, not with the output directory"
        );
    }

    #[test]
    fn a_bundle_name_that_cannot_be_read_is_treated_as_occupied() {
        // Fail closed, in every direction the file can disappoint. "Could not check" must never
        // read as "nothing there" — that is the one mistake this facet could introduce, and it
        // would land on exactly the calls whose spec is malformed.
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join("build");
        let spec = dir.path().join("spec.json");

        let unreadable: [&[u8]; 5] = [
            b"{ not json",
            br#"{"meta":{}}"#,
            br#"{"meta":{"name":42}}"#,
            // A name that is not one path component would put the deletion somewhere else entirely.
            br#"{"meta":{"name":"../escape"}}"#,
            br#"{"meta":{"name":"nested/mod"}}"#,
        ];
        for body in unreadable {
            std::fs::write(&spec, body).expect("write");
            let raised = question(
                "gore_mod",
                "build",
                json!({ "spec": spec.to_string_lossy(), "out": out.to_string_lossy() }),
                &options(),
            )
            .unwrap_or_else(|| panic!("{:?} must not pass as an empty destination", body));
            assert!(raised.reason.contains("could not be read"), "{}", raised.reason);
        }

        // Including the file simply not being there.
        std::fs::remove_file(&spec).expect("remove");
        assert!(
            question(
                "gore_mod",
                "build",
                json!({ "spec": spec.to_string_lossy(), "out": out.to_string_lossy() }),
                &options(),
            )
            .is_some(),
            "a spec that cannot be opened tells the gate nothing about what it would delete"
        );
    }

    #[test]
    fn a_truncating_output_in_the_game_tree_is_an_installation_change() {
        // `loc import --out` is copy-on-write, so a fresh path needs no flag — but the `.lcache`
        // lives in the installation, and writing the edited cache back there is a deployment
        // however new the path is. The check covers every declared output, not a hand-kept list.
        let dir = tempfile::tempdir().expect("tempdir");
        let outside = dir.path().join("edited.lcache");
        let call = |out: String| {
            json!({ "lcache": "in.lcache", "edits": "edits.json", "out": out })
        };

        assert!(question(
            "gore_loc",
            "import",
            call(outside.to_string_lossy().into_owned()),
            &options()
        )
        .is_none());

        assert!(
            asks_about_a_write(question(
                "gore_loc",
                "import",
                call("D:/Games/G1R/G1R/Content/Localization/Alkimia.lcache".into()),
                &options()
            )),
            "writing the cache back into the installation is a deployment"
        );
    }

    #[test]
    fn generating_a_mod_into_the_live_mods_folder_is_gated() {
        // `dump-mod` and `scaffold` write a mod folder containing an executable Scripts/main.lua.
        // Into a scratch directory that is a build artifact; into the game's own ue4ss/Mods it is
        // an installed, enabled mod.
        let dir = tempfile::tempdir().expect("tempdir");
        let scratch = dir.path().to_string_lossy().into_owned();
        let live = "D:/Games/G1R/G1R/Binaries/Win64/ue4ss/Mods";

        for (tool, sub, extra) in [
            ("gore_catalog", "dump-mod", json!({ "model": "m.json", "catalog": "c.json" })),
            ("gore_project", "scaffold", json!({ "mod_name": "MyMod" })),
        ] {
            let call = |out: &str| {
                let mut args = extra.as_object().expect("object").clone();
                args.insert("out".into(), Value::String(out.to_string()));
                Value::Object(args)
            };
            assert!(
                question(tool, sub, call(&scratch), &options()).is_none(),
                "{sub} into a scratch directory is nobody's business"
            );
            assert!(
                asks_about_a_write(question(tool, sub, call(live), &options())),
                "{sub} into the live Mods folder installs a mod"
            );
            assert!(question(tool, sub, call(live), &permissive()).is_none());
        }
    }

    #[test]
    fn a_link_chain_too_deep_to_follow_is_refused_rather_than_waved_through() {
        // The hop budget used to return the unresolved path, so a chain one link longer than the
        // budget looked like an ordinary outside path. Unknown must not read as safe.
        let dir = tempfile::tempdir().expect("tempdir");
        let install = dir.path().join("G1R");
        std::fs::create_dir_all(&install).expect("install-like tree");

        // hop_0 -> hop_1 -> ... -> hop_11 -> <install>/absent.json
        let mut target = install.join("absent.json");
        for hop in (0..12).rev() {
            let link = dir.path().join(format!("hop_{hop}.json"));
            if !symlink_file(&target, &link) {
                eprintln!("skipping: this platform/user cannot create symlinks");
                return;
            }
            target = link;
        }

        let call = json!({ "sdk_dir": "SDK", "out": target.to_string_lossy() });
        let raised = question("gore_catalog", "dump", call, &options());
        assert!(
            asks_about_a_write(raised.clone()),
            "a chain past the budget must be asked about, got {raised:?}"
        );
    }

    #[test]
    fn a_derived_output_is_checked_where_it_lands_not_where_its_base_does() {
        // `texture extract` writes the PNG and `<out>.png.json` beside it. Those are two files: the
        // PNG can sit safely outside while the sidecar is a link into the installation.
        let dir = tempfile::tempdir().expect("tempdir");
        let install = dir.path().join("G1R").join("Content");
        std::fs::create_dir_all(&install).expect("install-like tree");

        let png = dir.path().join("cursor.png");
        let sidecar = dir.path().join("cursor.png.json");
        if !symlink_file(&install.join("absent.json"), &sidecar) {
            eprintln!("skipping: this platform/user cannot create symlinks");
            return;
        }

        let call = json!({ "game": "G", "asset": "/Game/T", "out": png.to_string_lossy() });
        assert!(
            asks_about_a_write(question("gore_texture", "extract", call, &options())),
            "the sidecar lands in the installation even though the PNG does not"
        );
    }

    #[test]
    fn a_dangling_output_symlink_is_followed_to_where_the_write_lands() {
        // `canonicalize` refuses a link whose target does not exist yet, so an outside link aimed
        // at an absent file under G1R looked like an ordinary outside path — and `fs::write`
        // follows it, creating the file the game reads.
        let dir = tempfile::tempdir().expect("tempdir");
        let install = dir.path().join("G1R").join("Content");
        std::fs::create_dir_all(&install).expect("install-like tree");

        let link = dir.path().join("innocent.json");
        let target = install.join("not-there-yet.json");
        if !symlink_file(&target, &link) {
            eprintln!("skipping: this platform/user cannot create symlinks");
            return;
        }
        assert!(!link.exists(), "the link must be dangling for this to mean anything");

        let call = json!({ "sdk_dir": "SDK", "out": link.to_string_lossy() });
        assert!(
            asks_about_a_write(question("gore_catalog", "dump", call, &options())),
            "the write lands inside the installation, whatever the link is called"
        );
    }

    /// Create a file symlink, reporting whether the platform and user allow it.
    fn symlink_file(target: &std::path::Path, link: &std::path::Path) -> bool {
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(target, link).is_ok()
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link).is_ok()
        }
    }

    #[test]
    fn a_relative_output_is_resolved_before_the_game_tree_is_judged() {
        // The child resolves a relative path against this process's working directory, so judging
        // it lexically would wave through `--out .` run from inside the installation.
        let dir = tempfile::tempdir().expect("tempdir");
        let inside = dir.path().join("G1R").join("Content").join("Paks").join("~mods");
        std::fs::create_dir_all(&inside).expect("create install-like tree");

        let call = |out: &str| json!({ "mod_dir": "mod", "name": "zzz_Mine_P", "out": out });
        let absolute = inside.to_string_lossy().into_owned();
        assert!(
            question("gore_texture", "pack", call(&absolute), &options()).is_some(),
            "the absolute form was already caught"
        );

        // The same directory named relatively, from a working directory inside it.
        let previous = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&inside).expect("enter the install tree");
        let relative = question("gore_texture", "pack", call("."), &options());
        std::env::set_current_dir(previous).expect("restore cwd");

        assert!(
            relative.is_some(),
            "`--out .` inside the installation is the same deployment by a shorter name"
        );
    }

    #[test]
    fn scaffolding_over_an_existing_mod_folder_is_gated_but_a_fresh_name_is_not() {
        // The CLI only refuses when `Scripts/main.lua` exists, so an existing non-Lua mod under
        // the same name is entered and its `enabled.txt` truncated. The folder is `<out>/<mod_name>`
        // -- both arguments -- so the collision can be caught without gating ordinary scaffolding.
        let dir = tempfile::tempdir().expect("tempdir");
        let call = |name: &str| json!({ "mod_name": name, "out": dir.path().to_string_lossy() });

        assert!(question("gore_project", "scaffold", call("BrandNew"), &options()).is_none());

        std::fs::create_dir(dir.path().join("Existing")).expect("create mod dir");
        assert!(
            asks_about_a_write(question("gore_project", "scaffold", call("Existing"), &options())),
            "an occupied mod folder must be asked about"
        );
        assert!(question("gore_project", "scaffold", call("Existing"), &permissive()).is_none());
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

        assert!(question("gore_loc", "import", call(&fresh), &options()).is_none());
        assert!(
            asks_about_a_write(question("gore_loc", "import", call(&lcache), &options())),
            "out == in is a rewrite, not a creation"
        );
    }

    #[test]
    fn compiling_asks_about_the_launch_and_the_write_together() {
        // Compiling drives the game to regenerate its cache and then stages the result, so it is
        // both. Naming only the launch would send someone off to restart with a flag set that
        // still does not cover the call.
        let call = || json!({ "game": "G", "out": "fresh.Cache" });
        let raised = question("gore_as", "compile", call(), &options()).expect("must be asked");
        assert_eq!(raised.needs, Needs { write: true, game_launch: true });
        assert_eq!(raised.needs.flags(), "--allow-game-launch --allow-write");

        // With write pre-approved the launch is still outstanding, so the question is still put —
        // and the flags it names stay the *complete* set the call needs rather than the remainder.
        // A partial set is not something anyone can paste into a shell.
        let mut write_only = options();
        write_only.allow_write = true;
        let raised = question("gore_as", "compile", call(), &write_only).expect("must be asked");
        assert_eq!(raised.needs.flags(), "--allow-game-launch --allow-write");

        // Both pre-approved: nothing left to settle.
        assert!(question("gore_as", "compile", call(), &permissive()).is_none());
    }

    #[test]
    fn every_gate_arm_asks_a_question_a_person_can_read() {
        // These reasons are written across source lines with a trailing `\`. Dropping one folds the
        // continuation's indentation into the middle of a sentence — and unlike the other messages
        // in this file, this one is read by a human in a dialog rather than by a model.
        let dir = tempfile::tempdir().expect("tempdir");
        let occupied = dir.path().join("taken.json");
        std::fs::write(&occupied, b"{}").expect("write fixture");

        let arms = [
            // launches the game, and stages the result
            ("gore_as", "compile", json!({})),
            // rewrites its input in place
            ("gore_loc", "import", json!({ "lcache": "a.lcache", "edits": "e.json" })),
            // changes the installation outright
            ("gore_mgr", "reset", json!({})),
            // lands inside the game tree
            ("gore_texture", "pack", json!({
                "mod_dir": "m", "name": "zzz_P", "out": "D:/Games/G1R/G1R/Content/Paks/~mods",
            })),
            // its output is already there
            ("gore_catalog", "dump", json!({
                "sdk_dir": "SDK", "out": occupied.to_string_lossy(),
            })),
        ];

        for (tool, sub, args) in arms {
            let raised = question(tool, sub, args, &options())
                .unwrap_or_else(|| panic!("{sub} should raise a question"));
            let shown = crate::consent::elicitation_params(&raised);
            let message = shown["message"].as_str().expect("a message");

            for line in message.lines() {
                assert!(!line.contains("  "), "{sub} double-spaces {line:?}");
            }
            assert!(message.contains(&format!("gore {}", raised.path)), "{sub}: {message}");
            // Whatever the arm, the line that would run is in front of the person deciding.
            assert!(!raised.command_line.is_empty(), "{sub} shows no command line");
            assert!(message.contains(&raised.command_line), "{sub}: {message}");
            assert!(message.ends_with("Run it?"), "{sub}: {message}");
        }
    }

    #[test]
    fn pre_approval_changes_who_is_asked_and_never_what_the_gate_sees() {
        // The gate runs identically whatever the server was started with; the flags only decide
        // whether the answer is already known. Computing it lazily would make a permissive server
        // blind to a classification bug that a strict one would catch.
        let dir = tempfile::tempdir().expect("tempdir");
        let occupied = dir.path().join("taken.json");
        std::fs::write(&occupied, b"{}").expect("write fixture");
        let call = json!({ "sdk_dir": "SDK", "out": occupied.to_string_lossy() });

        let strict = question("gore_catalog", "dump", call.clone(), &options());
        assert!(strict.is_some());

        let mut write_only = options();
        write_only.allow_write = true;
        assert!(
            question("gore_catalog", "dump", call, &write_only).is_none(),
            "--allow-write covers a plain overwrite, so the same finding raises no question"
        );
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
    fn an_install_mutating_command_asks_and_names_the_flag_when_it_cannot() {
        let raised =
            question("gore_project", "deploy-shared", json!({}), &options()).expect("must ask");
        assert_eq!(raised.needs, Needs { write: true, game_launch: false });

        // On a client that cannot show a dialog the question becomes the old refusal, which has to
        // carry a command line the user can act on — there is no other route to yes.
        let message = crate::consent::refusal(
            &raised,
            &crate::consent::Decision::NotAsked(crate::consent::Policy::CannotAsk),
        );
        assert!(message.starts_with("refused:"), "{message}");
        assert!(message.contains("gore mcp serve --allow-write"), "{message}");
    }

    #[test]
    fn a_refusal_carries_the_very_line_a_result_would_have_led_with() {
        // The in-place arm is the one whose reason names no file at all — the rewrite is identified
        // by the argument that was left out. Pinning the refusal to `display` is what stops the
        // line the model relays and the line a successful result leads with from becoming two
        // renderings, only one of which would carry the real binary and the real quoting.
        let invocation = build_with(
            "gore_loc",
            "import",
            json!({ "lcache": "Alkimia.lcache", "edits": "edits.json" }),
            &options(),
        )
        .expect("the command line itself is valid");
        let raised = invocation.consent.as_ref().expect("a rewrite in place must be asked about");

        let message = crate::consent::refusal(raised, &crate::consent::Decision::Dismissed);
        assert!(message.starts_with("refused:"), "{message}");
        // Whole and on a line of its own: a `display` folded into a sentence is one the user cannot
        // select and paste, which is the only reason it is relayed at all.
        assert!(message.lines().any(|line| line == invocation.display), "{message}");
    }

    #[test]
    fn a_question_names_the_command_the_way_a_user_would_type_it() {
        // "`reset` modifies the game installation" is ambiguous — several groups have a command
        // that could be called that. The full path is what a user can act on, and this is the
        // sentence they read in the dialog before deciding.
        for (tool, sub, typed) in [
            ("gore_mgr", "reset", "gore mgr reset"),
            ("gore_project", "deploy-shared", "gore deploy-shared"),
        ] {
            let raised = question(tool, sub, json!({}), &options()).expect("must ask");
            let shown = crate::consent::elicitation_params(&raised);
            let message = shown["message"].as_str().expect("a message");
            assert!(message.contains(typed), "{sub}: {message}");
        }
    }

    #[test]
    fn a_game_launching_command_that_also_rewrites_in_place_asks_about_both() {
        // `as compile` without `out` installs the fresh cache over the game's own. One question
        // covers both, and it stays outstanding until *both* flags pre-approve it.
        let raised = question("gore_as", "compile", json!({}), &options()).expect("must ask");
        assert_eq!(raised.needs, Needs { write: true, game_launch: true });

        let mut launch_only = options();
        launch_only.allow_game_launch = true;
        assert!(
            question("gore_as", "compile", json!({}), &launch_only).is_some(),
            "the write half is still unapproved"
        );

        let mut write_only = options();
        write_only.allow_write = true;
        assert!(
            question("gore_as", "compile", json!({}), &write_only).is_some(),
            "the launch half is still unapproved"
        );

        let mut both = launch_only.clone();
        both.allow_write = true;
        assert!(question("gore_as", "compile", json!({}), &both).is_none());
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
        assert_eq!(invocation.display, "gore config set -- game-path 'D:/Program Files/G1R'");

        // The documented setup launches an unpacked binary by absolute path, which need not be on
        // PATH. A line beginning with a bare `gore` would not run when pasted.
        let mut installed = permissive();
        installed.exe = PathBuf::from(r"C:\Program Files\gore\gore.exe");
        let elsewhere = build_with(
            "gore_config",
            "set",
            json!({ "key": "game-path", "value": "D:/G1R" }),
            &installed,
        )
        .unwrap();
        // `&` is PowerShell's call operator. Without it the quoted path is a string literal that
        // prints itself, so the line would not run when pasted.
        assert_eq!(
            elsewhere.display,
            r"& 'C:\Program Files\gore\gore.exe' config set -- game-path D:/G1R"
        );

        // A bare program name is already an invocation and gains nothing from the operator.
        assert!(!invocation.display.starts_with('&'), "{}", invocation.display);

        // The guide is PowerShell throughout and this line is presented as something to re-run, so
        // a token carrying shell syntax must not end the command when it is pasted.
        let dangerous = build_with(
            "gore_config",
            "set",
            json!({ "key": "game-path", "value": "C:/mods;whoami" }),
            &permissive(),
        )
        .unwrap();
        assert_eq!(dangerous.display, "gore config set -- game-path 'C:/mods;whoami'");

        // A single quote is the only character a PowerShell literal string escapes, by doubling.
        let quoted = build_with(
            "gore_config",
            "set",
            json!({ "key": "game-path", "value": "it's here" }),
            &permissive(),
        )
        .unwrap();
        assert_eq!(quoted.display, "gore config set -- game-path 'it''s here'");

        // Ordinary paths and flags stay bare; quoting everything would make the line unreadable.
        assert_eq!(quote_for_powershell("--out"), "--out");
        assert_eq!(quote_for_powershell(r"D:\Games\G1R"), r"D:\Games\G1R");
        for hostile in
            ["", "a b", "a;b", "a|b", "a&b", "$env:PATH", "a`b", "a(b)", "a\"b", "@args", "user@host"]
        {
            let rendered = quote_for_powershell(hostile);
            assert!(rendered.starts_with('\'') && rendered.ends_with('\''), "{hostile:?} -> {rendered}");
        }
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
