//! What this server tells a client about itself during `initialize`.
//!
//! # Protocol era
//!
//! MCP has two eras. *Legacy* revisions (2025-11-25 and earlier) open with an `initialize`
//! handshake that negotiates one version for the whole session. *Modern* revisions (2026-07-28 and
//! later) drop the handshake: every request declares its own version in `_meta`, and servers must
//! implement `server/discover`.
//!
//! This server implements the legacy handshake only. Per the specification's own compatibility
//! matrix that serves legacy clients and dual-era clients (which probe, see a non-modern reply, and
//! fall back to `initialize`); it does not serve modern-only clients. Every shipping MCP client is
//! legacy or dual-era, so the practical cost today is nil, and the modern era can be added later
//! without disturbing anything here — the session is already effectively stateless.

use serde_json::{json, Value};

use crate::consent::{Needs, Policy};
use crate::server::Options;
use crate::spec::{Class, GroupShape};

/// The newest revision we implement. Also what we answer with when a client asks for something we
/// do not recognise.
pub const LATEST_PROTOCOL_VERSION: &str = "2025-11-25";

/// Legacy revisions we accept verbatim.
///
/// The handshake shape is unchanged across all of these; later revisions only add features we do
/// not use (tasks, elicitation, icons). Echoing the client's own version back therefore costs
/// nothing and avoids a needless disconnect on the client side.
pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &[
    LATEST_PROTOCOL_VERSION,
    "2025-06-18",
    "2025-03-26",
    "2024-11-05",
];

pub const SERVER_NAME: &str = "gore";
pub const SERVER_TITLE: &str = "GORE — Gothic 1 Remake modding toolkit";
pub const SERVER_DESCRIPTION: &str =
    "Drives the full gore CLI: items, localization, audio, voice, textures, cooked data assets, \
     AngelScript, mod bundles and the multi-mod manager.";
pub const SERVER_WEBSITE: &str = "https://github.com/dh0er/gore";

/// Choose the protocol version to answer with.
///
/// The rule from the specification: if we support what the client asked for, answer with exactly
/// that; otherwise answer with our own latest and let the client decide whether to continue.
pub fn negotiate_protocol_version(requested: Option<&str>) -> &'static str {
    let Some(requested) = requested else {
        return LATEST_PROTOCOL_VERSION;
    };
    SUPPORTED_PROTOCOL_VERSIONS
        .iter()
        .find(|supported| **supported == requested)
        .copied()
        .unwrap_or(LATEST_PROTOCOL_VERSION)
}

/// Server capabilities.
///
/// `listChanged` is deliberately absent on both: the tool table is a compile-time constant and the
/// guide is embedded in the binary, so neither list can change while the process is alive.
/// Advertising a notification we will never send would be a lie a client might wait on.
pub fn capabilities() -> Value {
    json!({
        "tools": {},
        "resources": {},
    })
}

/// Identity reported back to the client.
///
/// `version` is the version of the `gore` binary we re-exec, not of this crate — the crate inherits
/// the workspace version (`0.0.0`), which would tell a user nothing.
pub fn server_info(server_version: &str) -> Value {
    json!({
        "name": SERVER_NAME,
        "title": SERVER_TITLE,
        "version": server_version,
        "description": SERVER_DESCRIPTION,
        "websiteUrl": SERVER_WEBSITE,
    })
}

/// The primer a client loads automatically on connect.
///
/// This is the highest-leverage text in the whole server: it is the only thing guaranteed to reach
/// the model without a tool call, so it has to carry orientation rather than prose. It is also a
/// standing cost — it sits in every context this client opens — which is why it stays an index and
/// never becomes documentation. The documentation is the guide, reachable through `gore_guide`.
///
/// It depends on `Options` and on the session's [`Policy`] because what happens to a destructive
/// call is the part a model most needs to know up front. "That one will pause while the user
/// decides" and "that one will be refused outright" call for completely different behaviour, and
/// learning which by attempting it costs a wasted call either way.
pub fn instructions(opts: &Options, policy: Policy) -> String {
    let mut text = String::from(PRIMER);

    text.push_str("\nWHAT THIS SERVER MAY DO\n");
    text.push_str(&format!(
        "Reading anything is unremarkable, and so is writing to a path that is free and outside \
         the game installation. What needs agreement is changing something that already exists: an \
         occupied output path, a directory that already holds files, an output aimed inside the \
         installation, an in-place rewrite. Aim those somewhere new and they cost nothing. A second \
         group asks whatever is on disk, because what it changes is not a path you chose — the \
         installation itself, the shared catalogs and library the tools keep, or a target worked \
         out from a file this server does not read: {always}. The decision is per subcommand, not \
         per tool:\n",
        always = always_gated().join(", "),
    ));
    // Both lines ask `Options::pre_approves` — the same function the gate itself uses — rather than
    // reading the flags directly. The primer promising something the gate then refuses is the
    // failure mode this whole section keeps re-learning, and sharing one predicate ends it.
    text.push_str(&format!(
        "- Changing the game installation, or rewriting a file in place: {}.\n",
        will_be(&WRITES, opts, policy)
    ));
    // Strict standalone is genuinely offline. The other policies are not merely a possible launch:
    // they may drive the game to regenerate the cache and stage sources in the installation, so
    // they need write pre-approval too.
    text.push_str(&format!(
        "- `gore_as_compile` and `gore_as_compile_module` always run offline. With a fresh \
         `work_dir/tree` and outputs outside the installation they need no consent. The mixed \
         `gore_as` tool does the same with explicit `backend: standalone`. \
         A `game` or `standalone-then-game` backend, including the omitted default, may open a real \
         game window and stage sources in the installation: {}.\n",
        will_be(&LAUNCHES, opts, policy)
    ));
    text.push_str(match policy {
        Policy::Ask => CONSENT_ASK,
        Policy::CannotAsk => CONSENT_CANNOT_ASK,
        Policy::NeverAsk => CONSENT_NEVER_ASK,
    });
    if policy != Policy::NeverAsk {
        text.push_str(CONSENT_RELAY);
    }
    text.push_str(
        "\nMany commands sidestep the question entirely by writing somewhere new: passing an \
         output argument turns an in-place rewrite into a new file. Prefer that.\n",
    );

    text.push_str(HOW_IT_BEHAVES);
    text
}

/// The commands no argument can make harmless, named the way a caller would write them.
///
/// Read out of the spec table rather than typed here, for the reason [`will_be`] shares a predicate
/// with the gate: a primer that names commands the gate no longer treats that way is a lie told to
/// every client on connect, and it is the kind that survives a green test run. This sentence has
/// already been wrong once — it still listed `mod build`, `stubs`, `audio extract` and `as emit-all`
/// after those four learned to check their destination and ask only when something is in the way.
///
/// The bar is deliberately narrow: protected Manager-state writes, installation mutations, and
/// destructive operations, which ask no matter what they are handed. Commands that ask only about
/// an occupied destination are covered by the sentence above this list, and naming them here would
/// tell a model to avoid calls that are free.
///
/// `GameLaunch` is excluded because the line below this one is about exactly those, by name.
fn always_gated() -> Vec<String> {
    crate::spec::GROUPS
        .iter()
        .flat_map(|group| group.commands.iter().map(move |command| (group, command)))
        .filter(|(_, command)| {
            matches!(
                command.safety.base,
                Class::ManagerWrite | Class::Mutate | Class::Destructive
            )
        })
        .map(|(group, command)| match group.shape {
            GroupShape::Nested => format!("{} {}", group.cli, command.sub),
            GroupShape::Flat => command.sub.to_string(),
        })
        .collect()
}

/// The two tiers the primer reports on, as the gate itself would describe them.
const WRITES: Needs = Needs {
    write: true,
    game_launch: false,
};
const LAUNCHES: Needs = Needs {
    write: true,
    game_launch: true,
};

/// What becomes of a call in one tier, in one phrase.
///
/// The refusing postures name the flags from [`Needs::flags`] rather than spelling them out, so the
/// launch line cannot end up offering `--allow-game-launch` on its own — a flag set that would
/// still leave the call refused.
fn will_be(needs: &Needs, opts: &Options, policy: Policy) -> String {
    if opts.pre_approves(needs) {
        return "PRE-APPROVED, so it runs without interrupting anyone".into();
    }
    match policy {
        Policy::Ask => "CONFIRMED WITH THE USER before anything runs".into(),
        Policy::CannotAsk | Policy::NeverAsk => format!(
            "REFUSED; only the user can change that, by restarting this server with {}",
            needs.flags()
        ),
    }
}

const CONSENT_ASK: &str =
    "\nWhat is not pre-approved is not forbidden. This server puts it to the \
client, which is meant to show it to the user, so such a call takes longer while they decide. Do \
not read that delay as a hang and do not send the call again. A refusal comes back as an ordinary \
tool error naming the exact answer the client gave. Read it rather than assuming a person chose: \
some clients answer on the user's behalf, in milliseconds and without showing anything, and the \
error says what to do when that is what happened.\n";

const CONSENT_CANNOT_ASK: &str = "\nThis client cannot put a question to the user — it did not \
advertise the `elicitation` capability — so anything not pre-approved is refused instead. The \
refusal names the flag the user would have to restart this server with; you cannot enable it.\n";

/// The route that works wherever the dialog does not, offered to both refusing-by-accident
/// postures.
///
/// Held apart from the two strings above because the posture that must *not* see it is the third
/// one: under `--no-consent-prompts` a claim is refused as well, and pointing at it there would
/// send a model round a loop it cannot leave.
const CONSENT_RELAY: &str = "\nWhen such a call is refused, you still have a move: ask the user \
yourself, in the conversation, showing them the command line from the refusal. If they agree, send \
the exact same call again with both the refusal's opaque `approval_request_id` and `user_approved` \
set to their own words. The id expires, works once, and is bound to the normalized command; never \
invent it, change the retry, or fill the words without having asked.\n";

const CONSENT_NEVER_ASK: &str = "\nThis server was started with --no-consent-prompts, so anything \
not pre-approved is refused without asking. The refusal names the flag the user would have to \
restart it with; you cannot enable it.\n";

/// The standing part of the primer.
///
/// Every client loads this into context on connect, so it is a permanent cost and stays an index
/// rather than documentation. The documentation is the guide, one `gore_guide` call away. The one
/// thing it must accomplish is that a model knows the guide exists and reaches for it before
/// running something it has not run before.
const PRIMER: &str = r#"GORE is a modding toolkit for Gothic 1 Remake (Unreal Engine 5). This
server exposes the whole `gore` command line tool: every tool call runs a real `gore` subcommand as
a child process and returns its output, with the exact command line shown first so a user can
reproduce it in a shell.

TOOLS
  gore_guide     Search and read the modding guide and the technical reference. Start here.
  gore_help      The CLI's own `--help` for any command: exact flags, always current.
  gore_config    The shared configuration, above all where the game is installed.
  gore_doctor    One read-only pass over the setup. Run it when a mod deployed and nothing changed.
  gore_find      Look an id, a class name or a display name up across every offline catalog.
  gore_catalog   Regenerate reflection models and item/NPC/knowledge catalogs from a game dump.
  gore_location  Check a waypoint or spot name before a script uses it. Offline, no install.
  gore_dialog    Read and safely author bounded same-module dialog edits: complete defaults, bodies, topics, and new conversations.
  gore_npc       Read the game's characters, and author new ones: class chain, spawn sites, workspace.
  gore_project   Scaffold, compile and package a UE4SS Lua mod; install the shared Lua SDK.
  gore_loc       Localized text: decrypt the .lcache to JSON, edit it, re-encrypt.
  gore_audio     FMOD sound banks: list samples, extract to WAV, inject replacements, ship patches.
  gore_voice     Voice-over archives. Strictly copy-on-write; recorded audio is never overwritten.
  gore_texture   Textures in the IoStore containers: list, extract, replace, pack, deploy.
  gore_asset     Cooked DataAssets: receipt-sealed, byte-exact edits.
  gore_mod       Build, inspect, deploy, or undeploy one unified mod bundle.
  gore_mod_inspect  Read-only bundle validation; pass the directory or ZIP directly on this tool.
  gore_mgr       Manage a loadout end to end: import, order, preflight, recover, apply, status, reset.
  gore_mgr_preflight  Read-only Manager readiness check; arguments go directly on this tool.
  gore_as_compile  Strict standalone full-tree compilation. No game launch; a fresh work tree needs no consent.
  gore_as_compile_module  Strict standalone one-module compilation. No game launch; a fresh work tree and outside-install outputs need no consent.
  gore_as        AngelScript cache: inspect, decompile, patch defaults, recompile modules.
`gore_doctor`, `gore_find`, `gore_mod_inspect`, `gore_mgr_preflight`, `gore_as_compile`, and
`gore_as_compile_module` each select one command
already: pass their typed arguments directly, with no redundant `subcommand`. The other CLI tools
wrap command families: choose a `subcommand` and put that command's arguments in `args`.

BEFORE YOU ACT
Read the guide page for whatever you are about to touch. These commands have sharp edges that a
flag list does not convey — receipts that must match, caches that must be regenerated first, steps
whose order matters. Call gore_guide with action "search"; it ranks individual sections, so the
follow-up read stays small. A page too long for one result comes back in numbered parts, each
naming what the others hold: ask for the next `part` rather than reading the page again.

gore_guide covers two bodies and labels every hit. The guide says which command to reach for; the
reference records what a receipt seals and why a command refuses something, so read a reference
page when a command fails in a way the guide does not explain. Both are also resources, at
gore://guide/<page> and gore://reference/<page>.

WHERE THE GAME IS
Most commands locate the game themselves: an explicit `game` argument wins, then the configured
path, then Steam auto-detection. If something fails because it cannot find the game, set it once
with gore_config (subcommand "set", key "game-path") instead of passing `game` every time. That
one needs no flag even though it rewrites an existing file: it stores a preference, not content,
and it is what clears the most common setup failure.
"#;

const HOW_IT_BEHAVES: &str = r#"
HOW IT BEHAVES
- One command runs at a time and a call blocks until it finishes. Some walk the whole installation
  and take minutes.
- Every command has a wall-clock limit and is killed if it exceeds it.
- Output is capped. A truncated result says so and suggests how to narrow the query.
- A command that fails comes back as an ordinary result with isError set, carrying the CLI's own
  error text. Read it: it is usually precise about what was wrong.
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::GROUPS;
    use serde_json::Map;
    use std::path::PathBuf;

    fn options(allow_write: bool, allow_game_launch: bool) -> Options {
        let mut opts = Options::new(PathBuf::from("gore"), "0.1.0");
        opts.allow_write = allow_write;
        opts.allow_game_launch = allow_game_launch;
        opts
    }

    /// The primer as a client that can show a dialog receives it — the ordinary case.
    fn asking(allow_write: bool, allow_game_launch: bool) -> String {
        instructions(&options(allow_write, allow_game_launch), Policy::Ask)
    }

    #[test]
    fn the_primer_states_what_becomes_of_each_tier() {
        let asked = asking(false, false);
        assert!(asked.contains("CONFIRMED WITH THE USER"), "{asked}");
        assert!(
            !asked.contains("REFUSED"),
            "nothing is refused when the user can be asked"
        );

        let pre_approved = asking(true, true);
        assert!(pre_approved.contains("PRE-APPROVED"), "{pre_approved}");
        assert!(
            !pre_approved.contains("CONFIRMED WITH THE USER"),
            "{pre_approved}"
        );

        for policy in [Policy::CannotAsk, Policy::NeverAsk] {
            let text = instructions(&options(false, false), policy);
            assert!(text.contains("REFUSED"), "{policy:?}: {text}");
            assert!(text.contains("--allow-write"), "{policy:?}: {text}");
        }
    }

    #[test]
    fn a_delay_while_the_user_decides_is_announced_as_such() {
        // Otherwise an agent reads the pause as a hung server and retries, which puts a second
        // dialog on top of the first for the same call.
        let text = asking(false, false);
        assert!(text.contains("takes longer while they decide"), "{text}");
        assert!(text.contains("do not send the call again"), "{text}");
    }

    #[test]
    fn the_primer_does_not_promise_that_every_write_is_free() {
        // It used to open with "reading anything, and writing new files, always works", which
        // stopped being true once commands whose targets cannot be checked became mutations. The
        // primer is the one thing every client reads, so an overstatement there is the most
        // expensive kind.
        let text = asking(false, false);
        assert!(!text.contains("writing new files, always works"), "{text}");
        assert!(text.contains("Reading anything is unremarkable"), "{text}");
        assert!(
            text.contains("asks whatever is on disk"),
            "the primer must say that some writes are gated regardless: {text}"
        );
        // And the other half of the same truth, added when four commands stopped being in that
        // group: aiming a write somewhere new is the way past the question, and a primer that only
        // ever warns teaches a model to ask permission it does not need.
        assert!(
            text.contains("somewhere new and they cost nothing"),
            "{text}"
        );
    }

    #[test]
    fn the_commands_the_primer_names_are_the_ones_the_gate_gates_unconditionally() {
        // The failure this exists for happened. The sentence was prose, and it went on naming
        // `mod build`, `stubs`, `audio extract` and `as emit-all` as gated "whatever is on disk"
        // after all four learned to check their destination first — so the one text every client
        // reads on connect was telling every model to avoid four calls that had become free.
        //
        // Nothing caught it. `every_surface_that_names_a_permission_agrees_with_the_gate` compares
        // `Class::label()` against the gate and never looks at a command name, and the whole test
        // suite stayed green through the change that falsified this line. Deriving the list is what
        // fixes that; this test only holds the derivation to the gate's own definition.
        let named = always_gated();
        let text = asking(false, false);

        let expected: Vec<String> = GROUPS
            .iter()
            .flat_map(|group| group.commands.iter().map(move |command| (group, command)))
            .filter(|(_, command)| {
                // "No argument can make this harmless." A command that merely asks about an
                // occupied destination must not be here: the primer would be telling a model to
                // avoid a call that costs nothing on a fresh path.
                let empty = Map::new();
                command.safety.requirements(&empty).write
                    && command.safety.in_place_without.is_none()
                    && command.safety.truncates.is_empty()
                    && command.safety.clobbers_dir.is_empty()
                    && command.safety.derives.is_empty()
            })
            .filter(|(_, command)| !matches!(command.safety.base, Class::GameLaunch))
            .map(|(group, command)| match group.shape {
                GroupShape::Nested => format!("{} {}", group.cli, command.sub),
                GroupShape::Flat => command.sub.to_string(),
            })
            .collect();

        assert_eq!(named, expected, "the primer's list drifted from the gate");
        assert!(
            !named.is_empty(),
            "an empty list would make the sentence nonsense"
        );

        for command in &named {
            assert!(
                text.contains(command.as_str()),
                "{command} is missing from: {text}"
            );
        }

        // And the four that moved must not have come back. Named individually because they are the
        // exact regression, and because a reader of this file should be able to see which claim was
        // wrong without going through the history.
        for freed in ["mod build", "stubs", "audio extract", "as emit-all"] {
            assert!(
                !named.iter().any(|command| command == freed),
                "`{freed}` asks only about an occupied destination and must not be listed as \
                 unconditional"
            );
        }
    }

    #[test]
    fn the_primer_never_claims_a_compile_runs_unattended_unless_it_would() {
        // GameLaunch implies write in `Safety::requirements`, so --allow-game-launch alone is not
        // enough. The primer used to report compiling as unlocked on that flag by itself, which
        // told the model it could do something every attempt then refused.
        assert!(
            asking(false, true).contains("installation: CONFIRMED WITH THE USER"),
            "one flag is not enough and the primer must say so"
        );
        assert!(asking(true, true).contains("installation: PRE-APPROVED"));
        let standalone = asking(false, false);
        assert!(standalone.contains("always run offline"), "{standalone}");
        assert!(standalone.contains("fresh `work_dir/tree`"), "{standalone}");
        assert!(
            !standalone.contains("always run offline without consent"),
            "{standalone}"
        );

        // What the primer claims and what the gate does must agree in every combination. The gate
        // is consulted here rather than restated, so a change to `Safety` that the primer does not
        // follow fails this test instead of shipping as a false promise.
        let compile = crate::spec::group("gore_as")
            .and_then(|group| group.command("compile"))
            .expect("as compile exists");
        let required = compile.safety.requirements(&serde_json::Map::new());
        assert!(required.game_launch, "this test is about the launch tier");

        for (write, launch) in [(false, false), (true, false), (false, true), (true, true)] {
            let opts = options(write, launch);
            let gate_stays_silent = opts.pre_approves(&Needs {
                write: required.write,
                game_launch: required.game_launch,
            });
            let claims_unattended =
                instructions(&opts, Policy::Ask).contains("installation: PRE-APPROVED");
            assert_eq!(
                claims_unattended, gate_stays_silent,
                "mismatch at (write={write}, launch={launch})"
            );
        }
    }

    #[test]
    fn the_primer_offers_the_route_that_needs_no_dialog() {
        // Both refusing-by-accident postures leave one move that works: ask the user in the
        // conversation and relay their words. A model that learns this only from a refusal wastes a
        // call first — and in the posture where no dialog can be shown, wastes it every time.
        for policy in [Policy::Ask, Policy::CannotAsk] {
            let text = instructions(&options(false, false), policy);
            assert!(text.contains("user_approved"), "{policy:?}: {text}");
            assert!(text.contains("approval_request_id"), "{policy:?}: {text}");
            assert!(text.contains("works once"), "{policy:?}: {text}");
        }

        // Not under --no-consent-prompts: there the field is refused too, and offering it would
        // send the model into a loop it cannot win.
        let never = instructions(&options(false, false), Policy::NeverAsk);
        assert!(!never.contains("user_approved"), "{never}");
    }

    #[test]
    fn the_primer_points_at_the_guide() {
        let text = asking(false, false);
        assert!(text.contains("gore_guide"));
        assert!(text.contains("gore://guide/"));
        assert!(text.contains("BEFORE YOU ACT"));
    }

    #[test]
    fn the_primer_names_every_tool_the_server_advertises() {
        // The primer is the model's index of this server. A tool missing from it is a tool the
        // model has to stumble onto.
        let text = asking(false, false);
        for tool in crate::tool_definitions() {
            let name = tool["name"].as_str().expect("a tool name");
            assert!(text.contains(name), "the primer does not mention {name}");
        }
    }

    #[test]
    fn the_primer_stays_short_enough_to_carry_in_every_context() {
        // It is loaded into every conversation with this server, so length is a standing cost.
        // This is a budget, not a target: if it needs to grow, move the content into the guide.
        //
        // The one thing that may move it is the tool index, which carries a line per tool and is
        // required to name every one of them by the test above. Prose does not get that licence.
        let text = instructions(&options(true, true), Policy::Ask);
        assert!(
            text.lines().count() < 71,
            "the primer has grown to {} lines; move detail into the guide",
            text.lines().count()
        );
    }

    #[test]
    fn the_primer_tells_the_model_it_cannot_lift_the_gate_itself() {
        // Without this an agent will keep retrying a refused command, or try to restart the server.
        // Only the two postures that refuse say it: where the user *can* be asked, telling the
        // model it is powerless would push it to nag for flags instead of simply asking.
        for policy in [Policy::CannotAsk, Policy::NeverAsk] {
            let text = instructions(&options(false, false), policy);
            assert!(text.contains("you cannot enable it"), "{policy:?}: {text}");
        }
    }

    #[test]
    fn a_supported_version_is_echoed_back_unchanged() {
        assert_eq!(negotiate_protocol_version(Some("2025-06-18")), "2025-06-18");
        assert_eq!(negotiate_protocol_version(Some("2024-11-05")), "2024-11-05");
    }

    #[test]
    fn an_unknown_version_falls_back_to_our_latest() {
        assert_eq!(
            negotiate_protocol_version(Some("1900-01-01")),
            LATEST_PROTOCOL_VERSION
        );
        // A modern-era request would arrive without an `initialize` at all, but a client that asks
        // for it through the handshake still gets a usable legacy answer rather than a hang.
        assert_eq!(
            negotiate_protocol_version(Some("2026-07-28")),
            LATEST_PROTOCOL_VERSION
        );
        assert_eq!(negotiate_protocol_version(None), LATEST_PROTOCOL_VERSION);
    }

    #[test]
    fn our_latest_is_the_first_supported_entry() {
        assert_eq!(SUPPORTED_PROTOCOL_VERSIONS[0], LATEST_PROTOCOL_VERSION);
    }

    #[test]
    fn server_info_reports_the_cli_version_it_was_given() {
        let info = server_info("0.1.0");
        assert_eq!(info["name"], "gore");
        assert_eq!(info["version"], "0.1.0");
    }
}
