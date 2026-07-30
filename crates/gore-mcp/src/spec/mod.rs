//! The declarative description of every `gore` command this server exposes.
//!
//! # One table, two consumers
//!
//! Everything a tool call needs is described once here and read twice: [`crate::schema`] turns it
//! into the JSON Schema a client sees, and [`crate::argv`] turns it into the argument vector a
//! child process receives. Those two must agree — a schema promising an argument the argv builder
//! ignores is a silent failure — and the only reliable way to make them agree is to give them a
//! single source.
//!
//! # This table is a transcription, so drift is the failure mode
//!
//! The entries below mirror clap definitions in the `gore` crate by hand. Nothing in the type
//! system stops them from falling out of step when a flag is renamed. The guard is
//! `mcp_spec_sync.rs` in the CLI crate, which runs `gore <group> <sub> --help` for every leaf and
//! checks the flag sets agree in both directions. Treat that test as part of this module: without
//! it, this file is a promise; with it, it is a checked fact.
//!
//! Argument names use the clap field spelling (snake_case) as their JSON property name, and carry
//! the exact on-the-wire flag spelling separately in [`ArgForm`]. Keeping both means a reviewer can
//! diff this file against `main.rs` mechanically.

pub mod groups;

use serde_json::{Map, Value};

/// How one argument reaches the command line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgForm {
    /// `--flag <value>`.
    Long(&'static str),
    /// `--flag`, emitted only when the JSON value is `true`.
    Switch(&'static str),
    /// `--flag <value>` once per array element.
    LongRepeated(&'static str),
    /// A bare value. `order` is its index among this command's positionals.
    Positional { order: u8 },
    /// A trailing variadic positional: every array element becomes one bare value.
    PositionalRepeated { order: u8 },
}

impl ArgForm {
    /// The flag spelling without dashes, for the flag-set comparison against `--help`.
    pub fn flag(&self) -> Option<&'static str> {
        match self {
            ArgForm::Long(name) | ArgForm::Switch(name) | ArgForm::LongRepeated(name) => Some(name),
            ArgForm::Positional { .. } | ArgForm::PositionalRepeated { .. } => None,
        }
    }

    pub fn positional_order(&self) -> Option<u8> {
        match self {
            ArgForm::Positional { order } | ArgForm::PositionalRepeated { order } => Some(*order),
            _ => None,
        }
    }
}

/// The JSON type of an argument, plus the validation applied before spawning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgKind {
    /// A filesystem path. Passed through verbatim; the CLI owns existence and permission checks.
    Path,
    Str,
    /// Hex with an even number of digits, as the content-addressed patch commands expect. Case is
    /// left to the command: `asset patch-fixed` takes either, `as patch-default` insists on
    /// lowercase, and this pre-check refuses neither.
    Hex,
    Int { min: Option<i64>, max: Option<i64> },
    Bool,
    /// A closed set of string values, rendered into the schema as an `enum`.
    Enum(&'static [&'static str]),
    StrList,
    IntList,
}

impl ArgKind {
    /// Short type name used in generated tool descriptions.
    pub fn label(&self) -> &'static str {
        match self {
            ArgKind::Path => "path",
            ArgKind::Str => "string",
            ArgKind::Hex => "hex",
            ArgKind::Int { .. } => "integer",
            ArgKind::Bool => "boolean",
            ArgKind::Enum(_) => "enum",
            ArgKind::StrList => "string[]",
            ArgKind::IntList => "integer[]",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ArgSpec {
    /// JSON property name. Mirrors the clap field, so `--script-cache` is `script_cache`.
    pub name: &'static str,
    pub form: ArgForm,
    pub kind: ArgKind,
    /// Copied verbatim from the clap doc comment.
    pub help: &'static str,
    pub required: bool,
    /// Shown in the schema so the model knows what happens when it omits the argument.
    ///
    /// Never passed on the command line. clap owns the real default; restating it here would mean
    /// two places to change and one of them would eventually be wrong.
    pub default_hint: Option<&'static str>,
}

impl ArgSpec {
    pub const fn new(
        name: &'static str,
        form: ArgForm,
        kind: ArgKind,
        help: &'static str,
        required: bool,
    ) -> Self {
        Self { name, form, kind, help, required, default_hint: None }
    }

    pub const fn with_default(mut self, hint: &'static str) -> Self {
        self.default_hint = Some(hint);
        self
    }
}

/// What a command does to the machine it runs on. Ordered least to most consequential, so that
/// `max` gives the worst case over a set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    /// Reads only. Prints, lists, inspects.
    Read,
    /// Creates new files. Never touches the game installation or an existing input.
    Write,
    /// Modifies the game installation, or rewrites an existing file in place.
    Mutate,
    /// Undoes or discards work wholesale.
    Destructive,
    /// Launches the game executable.
    GameLaunch,
}

impl Class {
    /// Whether `--allow-write` is needed.
    pub fn needs_write_permission(&self) -> bool {
        matches!(self, Class::Mutate | Class::Destructive)
    }

    /// Whether `--allow-game-launch` is needed.
    pub fn needs_game_launch_permission(&self) -> bool {
        matches!(self, Class::GameLaunch)
    }

    pub fn label(&self) -> &'static str {
        match self {
            Class::Read => "read-only",
            Class::Write => "writes new files",
            Class::Mutate => "MODIFIES THE INSTALL — needs --allow-write",
            Class::Destructive => "DESTRUCTIVE — needs --allow-write",
            // Both flags, not one: `requirements` marks every GameLaunch command as a write too,
            // because compiling stages its result in the installation. This label is the third
            // surface that states it — the primer and the refusal message are the others — and all
            // three have to agree with the gate.
            Class::GameLaunch => {
                "LAUNCHES THE GAME — needs --allow-game-launch AND --allow-write"
            }
        }
    }
}

/// How a command builds a second output path out of one it was given.
///
/// Only shapes the gate can compute from the arguments alone belong here. `gore gen` derives its
/// target from the mod name inside `overrides.toml`, which this layer cannot see without parsing
/// the file, so that command is classified as a mutation instead of being described here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Derived {
    /// `Path::with_extension(_)` — `texture extract` writes `out` and `out` + `.png.json`.
    Extension(&'static str),
    /// `Path::join(_)` — `dump-mod` writes the `gore-dump/` folder inside the directory it is given.
    Child(&'static str),
    /// `Path::join(<value of the named argument>)` — `scaffold` writes `<out>/<mod_name>/`.
    ///
    /// The difference from [`Derived::Child`] is where the last component comes from: a literal
    /// here would be wrong, because the caller chooses it. This is what keeps `scaffold` ungated
    /// for a fresh mod name while still catching a collision with an existing mod folder.
    ChildOfArg(&'static str),
}

/// A command's safety class, including the conditional case.
#[derive(Clone, Copy, Debug)]
pub struct Safety {
    pub base: Class,
    /// When set, omitting this argument escalates the class to [`Class::Mutate`].
    ///
    /// This models a family of commands exactly: `audio replace`, `audio apply-patch`, `loc import`
    /// and `as compile` all write a new file when given `-o` and overwrite their input in place
    /// when not. Treating them as unconditionally dangerous would block the safe usage; treating
    /// them as unconditionally safe would let an agent overwrite the game's own files.
    pub in_place_without: Option<&'static str>,
    /// Arguments naming a path this command overwrites if it is already there.
    ///
    /// [`Class::Write`] promises "creates new files", and that is what lets it run ungated. A
    /// command that reaches for `fs::write` on its output breaks the promise the moment the path
    /// exists -- a rerun, or an agent picking an unrelated existing file, silently truncates it.
    /// Naming the argument here makes an existing target count as a mutation, so the common case
    /// (a fresh path) stays free and only the destructive one asks for `--allow-write`.
    ///
    /// This is a permission boundary, not a lock. The check runs before the child is spawned, and
    /// the child writes some time later, so a file appearing in between is still overwritten. That
    /// window cannot be closed from here — only the CLI's own publication step could, by refusing
    /// an occupied destination outright, and these are the commands whose whole purpose is to be
    /// re-run to regenerate their output. The gate answers "may this agent aim at a path it can
    /// see is occupied", which is the question an agent's mistake actually turns on.
    ///
    /// A `Write` command needs no entry here for exactly two reasons, and every one that has none
    /// was checked against its CLI writer: it refuses an occupied destination itself (the `asset`
    /// and `voice` families, `texture pack`, `as patch-default`, `as patch-tag-map`),
    /// or it maintains the toolkit's own reversible state (`config set`/`unset`/`detect`,
    /// `mgr enable`/`disable`/`order`). Everything else is a mutation.
    ///
    /// "It writes into a directory" is deliberately *not* a third reason. The directory may
    /// ordinarily exist, but the files inside it are still truncated one by one — `audio extract`,
    /// `as emit-all` and `stubs` each overwrite entries whose names come from data this layer
    /// never reads. Not being able to preflight a target is a reason to gate the command, not a
    /// reason to let it through.
    pub truncates: &'static [&'static str],
    /// Paths this command writes that no argument names, as `(argument, how it is derived)`.
    ///
    /// `texture extract` writes its PNG to `out` and a metadata sidecar to
    /// `out.with_extension("png.json")`. Only the first is an argument, so [`Safety::truncates`]
    /// cannot see the second — and a fresh PNG beside an existing sidecar would pass the gate and
    /// truncate it. Anything a command derives from an argument and then overwrites belongs here.
    pub derives: &'static [(&'static str, Derived)],
    /// Arguments whose value turns this call into an installation change when it points inside the
    /// game tree.
    ///
    /// Some commands are only a plain write because of *where* they are usually aimed. `texture
    /// pack` writes `<out>/<name>.{utoc,ucas,pak}`, which is a scratch artifact next to a build --
    /// unless `out` is the game's `~mods` folder, in which case those three files are the live
    /// override the game mounts, and the call has quietly done what `texture deploy` is gated for.
    pub installs_via: &'static [&'static str],
}

impl Safety {
    pub const fn read() -> Self {
        Self { base: Class::Read, in_place_without: None, truncates: &[], derives: &[], installs_via: &[] }
    }
    pub const fn write() -> Self {
        Self { base: Class::Write, in_place_without: None, truncates: &[], derives: &[], installs_via: &[] }
    }
    pub const fn mutate() -> Self {
        Self { base: Class::Mutate, in_place_without: None, truncates: &[], derives: &[], installs_via: &[] }
    }
    pub const fn destructive() -> Self {
        Self { base: Class::Destructive, in_place_without: None, truncates: &[], derives: &[], installs_via: &[] }
    }
    pub const fn game_launch() -> Self {
        Self { base: Class::GameLaunch, in_place_without: None, truncates: &[], derives: &[], installs_via: &[] }
    }

    /// [`Class::Write`] when `out_arg` is supplied, [`Class::Mutate`] when it is not.
    ///
    /// The same argument is registered as truncating. Supplying it is what makes the call a write
    /// rather than an in-place rewrite, but only if it names somewhere new: passing the input's own
    /// path as the output turns "write a new file" back into "replace that file", and the atomic
    /// writer underneath does exactly that.
    pub const fn write_or_in_place(out_arg: &'static [&'static str; 1]) -> Self {
        Self {
            base: Class::Write,
            in_place_without: Some(out_arg[0]),
            truncates: out_arg,
            derives: &[],
            installs_via: &[],
        }
    }

    /// [`Class::Write`], but the named arguments are overwritten rather than newly created when
    /// they already exist. See [`Safety::truncates`].
    pub const fn write_truncating(outputs: &'static [&'static str]) -> Self {
        Self {
            base: Class::Write,
            in_place_without: None,
            truncates: outputs,
            derives: &[],
            installs_via: &[],
        }
    }

    /// Register arguments that make this an installation change when they point into the game
    /// tree. See [`Safety::installs_via`].
    pub const fn installs_via(mut self, args: &'static [&'static str]) -> Self {
        self.installs_via = args;
        self
    }

    /// Register paths the command derives from an argument and overwrites. See [`Safety::derives`].
    pub const fn also_writes(mut self, derived: &'static [(&'static str, Derived)]) -> Self {
        self.derives = derived;
        self
    }

    /// Escalate an existing class with the same in-place rule (used by `as compile`).
    ///
    /// Registers the argument as truncating for the same reason
    /// [`Safety::write_or_in_place`] does: an output that names an existing path is a replacement,
    /// not a creation. Redundant on a `GameLaunch` command, which is gated either way — but the
    /// builder must not be the one spelling where the rule silently stops applying.
    pub const fn in_place_without(mut self, out_arg: &'static [&'static str; 1]) -> Self {
        self.in_place_without = Some(out_arg[0]);
        self.truncates = out_arg;
        self
    }

    /// The class this specific call falls into.
    pub fn effective(&self, args: &Map<String, Value>) -> Class {
        match self.in_place_without {
            Some(escape) if !args.contains_key(escape) => self.base.max(Class::Mutate),
            _ => self.base,
        }
    }

    /// The worst case, used for descriptions and annotations where no arguments are known yet.
    pub fn worst_case(&self) -> Class {
        match self.in_place_without {
            Some(_) => self.base.max(Class::Mutate),
            None => self.base,
        }
    }

    /// Which permission flags this specific call needs.
    ///
    /// Computed from independent facts rather than from the ordered [`Class`], because a
    /// game-launching command is also install-mutating and the ordering would hide one behind the
    /// other. `as compile` without `-o` both launches the game and overwrites the installed script
    /// cache, and it should require both flags, not just the more severe one.
    ///
    /// [`Class::GameLaunch`] implies `write` even when the command writes its output elsewhere:
    /// driving the game's own compiler stages a source tree into the installation and restores it
    /// afterwards, so the installation is touched either way.
    pub fn requirements(&self, args: &Map<String, Value>) -> Requirements {
        let rewrites_in_place =
            self.in_place_without.is_some_and(|escape| !args.contains_key(escape));
        Requirements {
            write: rewrites_in_place
                || matches!(self.base, Class::Mutate | Class::Destructive | Class::GameLaunch),
            game_launch: matches!(self.base, Class::GameLaunch),
            rewrites_in_place,
        }
    }
}

/// The permission flags a call needs before it may run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Requirements {
    /// Needs `--allow-write`.
    pub write: bool,
    /// Needs `--allow-game-launch`.
    pub game_launch: bool,
    /// The call omitted its output argument and will therefore overwrite its input.
    pub rewrites_in_place: bool,
}

/// Whether a command has a `--json` flag, and what it means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsonSupport {
    None,
    /// A boolean switch that makes the command print JSON to stdout.
    ///
    /// The server passes it unconditionally and parses the result into `structuredContent`, so a
    /// model gets machine-readable output without having to know the flag exists.
    Stdout,
    /// `--json <PATH>` writes a report file — only `as bytediff` behaves this way.
    ///
    /// Declared as an ordinary path argument instead, and never auto-passed: appending it would
    /// silently create a file nobody asked for.
    OutputFile,
}

/// One leaf command.
#[derive(Clone, Copy, Debug)]
pub struct CommandSpec {
    /// The clap subcommand spelling.
    pub sub: &'static str,
    /// First line of the clap doc comment.
    pub summary: &'static str,
    pub args: &'static [ArgSpec],
    pub safety: Safety,
    pub json: JsonSupport,
    /// Arguments appended unconditionally and never exposed to the caller.
    ///
    /// Exists for exactly one reason today: `gore loc extract` prompts on stdin unless given `-y`,
    /// and under the stdio transport our stdin is the JSON-RPC channel, so an unsuppressed prompt
    /// would deadlock the session.
    pub forced_argv: &'static [&'static str],
    pub timeout_secs: u64,
    /// The guide page to read first, surfaced in descriptions and in failure messages.
    pub guide: Option<&'static str>,
    /// Sets from which exactly one argument must be supplied.
    ///
    /// Mirrors clap's `required_unless_present` + `conflicts_with` pairs — `voice extract` takes
    /// either `--basename` or `--path` and refuses both. JSON Schema can say this with `oneOf`, but
    /// enough MCP clients ignore conditional subschemas when building a call that relying on it
    /// would leave the constraint unenforced; it is checked here instead, and stated in the
    /// argument descriptions.
    pub exactly_one_of: &'static [&'static [&'static str]],
    /// Sets from which at most one argument may be supplied.
    pub at_most_one_of: &'static [&'static [&'static str]],
}

impl CommandSpec {
    pub const fn new(
        sub: &'static str,
        summary: &'static str,
        args: &'static [ArgSpec],
        safety: Safety,
        timeout_secs: u64,
    ) -> Self {
        Self {
            sub,
            summary,
            args,
            safety,
            json: JsonSupport::None,
            forced_argv: &[],
            timeout_secs,
            guide: None,
            exactly_one_of: &[],
            at_most_one_of: &[],
        }
    }

    pub const fn exactly_one(mut self, sets: &'static [&'static [&'static str]]) -> Self {
        self.exactly_one_of = sets;
        self
    }

    pub const fn at_most_one(mut self, sets: &'static [&'static [&'static str]]) -> Self {
        self.at_most_one_of = sets;
        self
    }

    pub const fn json(mut self, json: JsonSupport) -> Self {
        self.json = json;
        self
    }

    pub const fn forced(mut self, argv: &'static [&'static str]) -> Self {
        self.forced_argv = argv;
        self
    }

    pub const fn guide(mut self, page: &'static str) -> Self {
        self.guide = Some(page);
        self
    }

    pub fn arg(&self, name: &str) -> Option<&'static ArgSpec> {
        self.args.iter().find(|arg| arg.name == name)
    }
}

/// Whether a group corresponds to a real CLI subcommand or is a synthetic bundle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupShape {
    /// `gore <cli> <sub> …`
    Nested,
    /// `gore <sub> …` — the group exists only to keep the tool list navigable.
    Flat,
}

#[derive(Clone, Copy, Debug)]
pub struct GroupSpec {
    /// MCP tool name.
    pub tool: &'static str,
    /// Human-readable name, shown by clients that display one.
    pub title: &'static str,
    /// The CLI subcommand this group maps onto. Empty for [`GroupShape::Flat`] groups.
    pub cli: &'static str,
    pub summary: &'static str,
    pub shape: GroupShape,
    pub commands: &'static [CommandSpec],
}

impl GroupSpec {
    pub fn command(&self, sub: &str) -> Option<&'static CommandSpec> {
        self.commands.iter().find(|command| command.sub == sub)
    }

    pub fn subcommands(&self) -> Vec<&'static str> {
        self.commands.iter().map(|command| command.sub).collect()
    }

    /// The worst safety class across this group's commands.
    ///
    /// MCP annotations live on the tool, not on the subcommand, so a namespace tool that bundles a
    /// read-only inspector with a command that launches the game has to advertise the dangerous
    /// one. Claiming `readOnlyHint` for `gore_as` because most of its leaves only read would be a
    /// lie a client's approval UI relies on. The per-subcommand truth is in the description and is
    /// enforced by the safety gate regardless of what the annotation says.
    pub fn worst_case(&self) -> Class {
        self.commands.iter().map(|command| command.safety.worst_case()).max().unwrap_or(Class::Read)
    }
}

/// Wall-clock caps. Three tiers rather than a number per command: the distinction that matters is
/// "prints something", "rewrites some files" and "walks the entire game", and inventing a precise
/// budget for each of 77 commands would be false precision.
pub const T_FAST: u64 = 60;
pub const T_NORMAL: u64 = 300;
pub const T_LONG: u64 = 1800;
/// The two commands that drive the game to regenerate the script cache.
///
/// Deliberately longer than [`T_LONG`]. `gore-as` gives the generator its own 30-minute deadline
/// (`compile.rs`, `Duration::from_secs(30 * 60)`), but that clock only starts once the game is
/// launched, while this one starts before the child has run preflight or staged the Script tree.
/// Matching the two would guarantee the outer kill lands first on any long compile — and killing
/// the wrapper mid-transaction skips `CompileTransaction::restore_install`, leaving the
/// installation staged. The extra quarter hour is the headroom that lets the inner timeout, the
/// game's termination and the restore all happen before this one is reached.
pub const T_COMPILE: u64 = 2700;

/// Every group this server exposes, in the order they appear in `tools/list`.
///
/// Ordered roughly by how early a user meets them: configure, then edit content, then package and
/// install, then the deeper script tooling.
pub const GROUPS: &[GroupSpec] = &[
    groups::core::CONFIG,
    groups::core::CATALOG,
    groups::core::PROJECT,
    groups::files::LOC,
    groups::files::AUDIO,
    groups::files::VOICE,
    groups::deploy::TEXTURE,
    groups::deploy::ASSET,
    groups::deploy::MOD,
    groups::deploy::MGR,
    groups::script::AS,
];

/// The number of CLI leaf commands this server is expected to cover.
///
/// A literal, not a computed value: it is a claim about the CLI, and the integration test compares
/// it against what clap actually exposes. Changing it should be a deliberate act.
pub const EXPECTED_LEAF_COUNT: usize = 77;

pub fn group(tool: &str) -> Option<&'static GroupSpec> {
    GROUPS.iter().find(|group| group.tool == tool)
}

/// Total number of CLI leaf commands reachable through this server.
pub fn leaf_count() -> usize {
    GROUPS.iter().map(|group| group.commands.len()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn the_table_covers_every_leaf_of_the_cli() {
        assert_eq!(leaf_count(), EXPECTED_LEAF_COUNT);
        assert_eq!(GROUPS.len(), 11);
    }

    #[test]
    fn exclusive_sets_name_real_arguments() {
        for group in GROUPS {
            for command in group.commands {
                for set in command.exactly_one_of.iter().chain(command.at_most_one_of) {
                    assert!(set.len() >= 2, "{}/{}: a set of one is not a set", group.tool, command.sub);
                    for name in *set {
                        assert!(
                            command.arg(name).is_some(),
                            "{}/{}: exclusive set names `{name}`, which is not an argument",
                            group.tool,
                            command.sub
                        );
                    }
                }
                // An argument in an "exactly one" set cannot also be individually required, or the
                // two rules contradict each other.
                for set in command.exactly_one_of {
                    for name in *set {
                        assert!(
                            !command.arg(name).expect("checked above").required,
                            "{}/{}: `{name}` is both required and part of an exclusive set",
                            group.tool,
                            command.sub
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn tool_names_are_unique_and_client_safe() {
        let mut seen = HashSet::new();
        for group in GROUPS {
            assert!(seen.insert(group.tool), "duplicate tool name: {}", group.tool);
            // MCP restricts tool names to letters, digits, underscore, hyphen and dot.
            assert!(
                group.tool.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')),
                "tool name is not client-safe: {}",
                group.tool
            );
            assert!(group.tool.len() <= 128);
        }
    }

    #[test]
    fn every_group_has_commands_with_unique_subcommand_names() {
        for group in GROUPS {
            assert!(!group.commands.is_empty(), "{} has no commands", group.tool);
            let mut seen = HashSet::new();
            for command in group.commands {
                assert!(
                    seen.insert(command.sub),
                    "{} declares {} twice",
                    group.tool,
                    command.sub
                );
            }
        }
    }

    #[test]
    fn argument_names_are_unique_within_a_command() {
        for group in GROUPS {
            for command in group.commands {
                let mut seen = HashSet::new();
                for arg in command.args {
                    assert!(
                        seen.insert(arg.name),
                        "{}/{} declares {} twice",
                        group.tool,
                        command.sub,
                        arg.name
                    );
                }
            }
        }
    }

    #[test]
    fn positional_orders_are_dense_and_start_at_zero() {
        // A gap or a duplicate would silently reorder the command line, which is the kind of bug
        // that produces a plausible-looking wrong result rather than an error.
        for group in GROUPS {
            for command in group.commands {
                let mut orders: Vec<u8> =
                    command.args.iter().filter_map(|arg| arg.form.positional_order()).collect();
                orders.sort_unstable();
                let expected: Vec<u8> = (0..orders.len() as u8).collect();
                assert_eq!(
                    orders, expected,
                    "{}/{} has non-dense positional ordering",
                    group.tool, command.sub
                );
            }
        }
    }

    #[test]
    fn only_one_trailing_variadic_positional_and_it_is_last() {
        for group in GROUPS {
            for command in group.commands {
                let variadic: Vec<u8> = command
                    .args
                    .iter()
                    .filter(|arg| matches!(arg.form, ArgForm::PositionalRepeated { .. }))
                    .filter_map(|arg| arg.form.positional_order())
                    .collect();
                assert!(variadic.len() <= 1, "{}/{} has two variadics", group.tool, command.sub);
                if let Some(order) = variadic.first() {
                    let positionals =
                        command.args.iter().filter_map(|arg| arg.form.positional_order()).count();
                    assert_eq!(
                        *order as usize,
                        positionals - 1,
                        "{}/{}: a variadic must be the last positional",
                        group.tool,
                        command.sub
                    );
                }
            }
        }
    }

    #[test]
    fn an_in_place_escape_hatch_names_a_real_argument() {
        for group in GROUPS {
            for command in group.commands {
                if let Some(escape) = command.safety.in_place_without {
                    assert!(
                        command.arg(escape).is_some(),
                        "{}/{}: in_place_without names {escape}, which is not an argument",
                        group.tool,
                        command.sub
                    );
                }
            }
        }
    }

    #[test]
    fn a_stdout_json_switch_is_never_also_declared_as_an_argument() {
        // The server passes `--json` itself for these; declaring it as well would emit it twice.
        for group in GROUPS {
            for command in group.commands {
                if command.json == JsonSupport::Stdout {
                    assert!(
                        command.arg("json").is_none(),
                        "{}/{} declares `json` and also auto-passes it",
                        group.tool,
                        command.sub
                    );
                }
            }
        }
    }

    #[test]
    fn flat_groups_have_no_cli_token_and_nested_groups_do() {
        for group in GROUPS {
            match group.shape {
                GroupShape::Flat => assert!(group.cli.is_empty(), "{} is flat", group.tool),
                GroupShape::Nested => assert!(!group.cli.is_empty(), "{} is nested", group.tool),
            }
        }
    }

    #[test]
    fn in_place_commands_are_reported_as_mutating_only_when_the_output_is_omitted() {
        let safety = Safety::write_or_in_place(&["out"]);
        let mut with_out = Map::new();
        with_out.insert("out".into(), Value::from("new.bank"));

        assert_eq!(safety.effective(&with_out), Class::Write);
        assert_eq!(safety.effective(&Map::new()), Class::Mutate);
        assert_eq!(safety.worst_case(), Class::Mutate);
    }

    #[test]
    fn a_game_launching_command_stays_game_launching_when_it_also_writes_in_place() {
        let safety = Safety::game_launch().in_place_without(&["out"]);
        assert_eq!(safety.effective(&Map::new()), Class::GameLaunch);
        assert_eq!(safety.worst_case(), Class::GameLaunch);
    }

    #[test]
    fn no_prose_names_the_launch_flag_without_the_write_flag() {
        // The same fact has now been wrong in four places: the primer, the refusal message, the
        // class label, and a group summary. Every one of them is prose a client reads, so check
        // the prose itself rather than fixing them one review at a time.
        let mut prose: Vec<(&str, &str)> = Vec::new();
        for group in GROUPS {
            prose.push((group.tool, group.summary));
            for command in group.commands {
                prose.push((command.sub, command.summary));
                for arg in command.args {
                    prose.push((arg.name, arg.help));
                }
            }
        }

        for (where_, text) in prose {
            if text.contains("--allow-game-launch") {
                assert!(
                    text.contains("--allow-write"),
                    "{where_} names --allow-game-launch without --allow-write: {text:?}"
                );
            }
        }
    }

    #[test]
    fn every_surface_that_names_a_permission_agrees_with_the_gate() {
        // The same fact is stated in three places — this label, the instructions primer, and the
        // refusal message — and it has now been wrong in each of them once. A command that needs
        // write permission must say so wherever it says anything.
        for class in [Class::Read, Class::Write, Class::Mutate, Class::Destructive, Class::GameLaunch]
        {
            let label = class.label();
            assert_eq!(
                label.contains("--allow-write"),
                class.needs_write_permission() || class == Class::GameLaunch,
                "{label:?} disagrees with the gate about --allow-write"
            );
            assert_eq!(
                label.contains("--allow-game-launch"),
                class.needs_game_launch_permission(),
                "{label:?} disagrees with the gate about --allow-game-launch"
            );
        }

        // And the authority those labels describe: GameLaunch really does require both.
        let requirements = Safety::game_launch().requirements(&Map::new());
        assert!(requirements.game_launch && requirements.write);
    }

    #[test]
    fn an_in_place_command_also_guards_the_output_it_was_given() {
        // Supplying `out` is what downgrades these from "rewrite the input" to "write a new file",
        // but only when it names somewhere new. Passing the input's own path as the output is the
        // in-place case wearing the safe class.
        let safety = Safety::write_or_in_place(&["out"]);
        assert_eq!(safety.in_place_without, Some("out"));
        assert_eq!(safety.truncates, &["out"]);
    }

    /// Every command that overwrites a named output file, as one list.
    ///
    /// This exists because getting it partly right is the failure mode: the first version of the
    /// truncation gate covered the catalog commands and missed `package`, `loc export`,
    /// `audio export-patch` and `texture extract`, all of which truncate just the same. A new
    /// command that writes a file has to be added here deliberately.
    ///
    /// Directory outputs are deliberately absent. `stubs`, `audio extract`, `texture pack`,
    /// `as emit-all` and the Mods-directory commands write *into* a directory that ordinarily
    /// already exists, so there is no single path to gate and gating the directory would refuse
    /// routine calls. Commands whose CLI refuses an existing output on its own -- the whole
    /// `asset` and `voice` families, `as patch-default`, `as patch-tag-map` -- are absent because
    /// the CLI is already the guard.
    #[test]
    fn exactly_the_known_file_writers_gate_an_existing_output() {
        let mut gated: Vec<(&str, &str, &[&'static str])> = Vec::new();
        for group in GROUPS {
            for command in group.commands {
                if !command.safety.truncates.is_empty() {
                    gated.push((group.tool, command.sub, command.safety.truncates));
                }
            }
        }
        gated.sort_unstable();

        let expected: Vec<(&str, &str, &[&'static str])> = vec![
            ("gore_audio", "apply-patch", &["out"]),
            ("gore_audio", "export-patch", &["out"]),
            ("gore_audio", "replace", &["out"]),
            ("gore_as", "compile", &["out"]),
            ("gore_catalog", "catalog", &["out"]),
            ("gore_catalog", "dump", &["out"]),
            ("gore_catalog", "gui-model", &["out"]),
            ("gore_catalog", "story-catalog", &["out"]),
            ("gore_catalog", "sync", &["out"]),
            ("gore_loc", "export", &["out"]),
            ("gore_loc", "import", &["out"]),
            ("gore_project", "package", &["out"]),
            ("gore_texture", "extract", &["out"]),
            ("gore_texture", "index", &["out"]),
            ("gore_as", "replace", &["out"]),
            ("gore_as", "splice", &["out"]),
            ("gore_as", "extract", &["out"]),
            ("gore_as", "extract-remap", &["out"]),
            ("gore_as", "bytediff", &["json"]),
        ];
        let mut expected = expected;
        expected.sort_unstable();

        assert_eq!(gated, expected);

        // Every named argument must actually exist on its command, or the gate silently never fires.
        for (tool, sub, args) in &gated {
            let command = group(tool).and_then(|g| g.command(sub)).expect("command exists");
            for arg in *args {
                assert!(command.arg(arg).is_some(), "{tool} {sub} has no argument `{arg}`");
            }
        }
    }

    /// Every command that names an output must appear in at least one of the three lists.
    ///
    /// This is the guard the previous rounds needed. Each of them found one more command whose
    /// output was reachable but unchecked — because "the CLI refuses an existing output" excuses
    /// the *truncation* question and says nothing about the *destination* one, and the two kept
    /// being conflated. Anything with an `out` argument has to be accounted for explicitly.
    #[test]
    fn every_command_with_an_output_argument_is_accounted_for() {
        let mut unchecked: Vec<String> = Vec::new();
        for group in GROUPS {
            for command in group.commands {
                let outputs: Vec<&str> = command
                    .args
                    .iter()
                    .filter(|arg| matches!(arg.kind, ArgKind::Path))
                    .map(|arg| arg.name)
                    .filter(|name| *name == "out")
                    .collect();
                if outputs.is_empty() {
                    continue;
                }
                let covered = |name: &str| {
                    command.safety.truncates.contains(&name)
                        || command.safety.installs_via.contains(&name)
                        || command.safety.derives.iter().any(|(arg, _)| *arg == name)
                        // A command gated outright needs no per-argument check. Asked of the
                        // gate rather than of `Class`, because a GameLaunch command requires write
                        // permission even though its class alone does not say so.
                        || command.safety.requirements(&Map::new()).write
                };
                for name in outputs {
                    if !covered(name) {
                        unchecked.push(format!("{} {}", group.tool, command.sub));
                    }
                }
            }
        }
        // `asset extract` is the one exemption. Its CLI resolves the destination through
        // `prepare_absent_output_directory`, which refuses anything inside the live game tree
        // outright — so the command is already the guard, and adding it here would only replace a
        // precise CLI error with a permission refusal.
        unchecked.retain(|name| name != "gore_asset extract");
        assert!(
            unchecked.is_empty(),
            "these name an output that no safety list covers: {unchecked:?}"
        );
    }

    /// And the same for outputs whose *destination* decides the classification.
    ///
    /// Kept beside the other two for the same reason: these three mechanisms guard one promise
    /// between them, and every round that added a command to one of them found the next round
    /// adding it to another.
    #[test]
    fn exactly_the_known_installation_sensitive_outputs_are_gated() {
        let mut sensitive: Vec<(&str, &str, &[&'static str])> = Vec::new();
        for group in GROUPS {
            for command in group.commands {
                if !command.safety.installs_via.is_empty() {
                    sensitive.push((group.tool, command.sub, command.safety.installs_via));
                }
            }
        }
        sensitive.sort_unstable();

        let mut expected: Vec<(&str, &str, &[&'static str])> = vec![
            // Both write a mod folder carrying an executable Scripts/main.lua.
            ("gore_catalog", "dump-mod", &["out"]),
            ("gore_project", "scaffold", &["out"]),
            // Both produce a Zen triplet that is a build artifact anywhere but `~mods`.
            ("gore_asset", "pack", &["out"]),
            // Writes a package pair, its sidecars and a receipt; `asset extract` refuses a
            // game-tree destination itself, this one does not.
            ("gore_asset", "patch-fixed", &["out"]),
            // Copy-on-write, so never truncating — but the cache they write is the one the game
            // loads, and a fresh path inside `Script/` installs it.
            ("gore_as", "patch-default", &["out"]),
            ("gore_as", "patch-tag-map", &["out"]),
            // Voice archives live in the installation, so a fresh output there is an installed
            // archive however new the path is.
            ("gore_voice", "extract", &["out"]),
            ("gore_voice", "add", &["out"]),
            ("gore_voice", "replace", &["out"]),
            ("gore_voice", "apply-manifest", &["out"]),
            ("gore_texture", "pack", &["out"]),
        ];
        expected.sort_unstable();

        assert_eq!(sensitive, expected);

        for (tool, sub, args) in &sensitive {
            let command = group(tool).and_then(|g| g.command(sub)).expect("command exists");
            for arg in *args {
                assert!(command.arg(arg).is_some(), "{tool} {sub} has no argument `{arg}`");
            }
        }
    }

    /// The same list for paths a command computes rather than is handed. Kept beside the one above
    /// because the two mechanisms guard the same promise and have to be extended together.
    #[test]
    fn exactly_the_known_derived_outputs_are_gated() {
        let mut derived: Vec<(&str, &str, &[(&'static str, Derived)])> = Vec::new();
        for group in GROUPS {
            for command in group.commands {
                if !command.safety.derives.is_empty() {
                    derived.push((group.tool, command.sub, command.safety.derives));
                }
            }
        }
        derived.sort_unstable_by_key(|(tool, sub, _)| (*tool, *sub));

        let expected: Vec<(&str, &str, &[(&'static str, Derived)])> = vec![
            ("gore_catalog", "dump-mod", &[("out", Derived::Child("gore-dump"))]),
            ("gore_project", "scaffold", &[("out", Derived::ChildOfArg("mod_name"))]),
            ("gore_texture", "extract", &[("out", Derived::Extension("png.json"))]),
        ];
        assert_eq!(derived, expected);

        for (tool, sub, entries) in &derived {
            let command = group(tool).and_then(|g| g.command(sub)).expect("command exists");
            for (arg, _) in *entries {
                assert!(command.arg(arg).is_some(), "{tool} {sub} has no argument `{arg}`");
            }
        }
    }
}
