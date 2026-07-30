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

use crate::server::Options;

/// The newest revision we implement. Also what we answer with when a client asks for something we
/// do not recognise.
pub const LATEST_PROTOCOL_VERSION: &str = "2025-11-25";

/// Legacy revisions we accept verbatim.
///
/// The handshake shape is unchanged across all of these; later revisions only add features we do
/// not use (tasks, elicitation, icons). Echoing the client's own version back therefore costs
/// nothing and avoids a needless disconnect on the client side.
pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] =
    &[LATEST_PROTOCOL_VERSION, "2025-06-18", "2025-03-26", "2024-11-05"];

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
/// It depends on `Options` because the safety tiers are the part a model most needs to know up
/// front: being told "that command needs `--allow-write`" *before* attempting it saves a wasted
/// call and a confusing refusal.
pub fn instructions(opts: &Options) -> String {
    let mut text = String::from(PRIMER);

    text.push_str("\nWHAT THIS SERVER MAY DO\n");
    text.push_str(
        "Reading anything always works. Writing works when the command can show it is creating \
         something rather than replacing it: a fresh output path needs no flag, an occupied one \
         does, and so does an output aimed inside the game installation. A few commands compute \
         their targets from a file this server does not read (gen, mod build, texture replace) or \
         write a whole tree of them (stubs, audio extract, as emit-all); those are gated whatever \
         is on disk. Two tiers are gated, and the gate is decided per subcommand, not per tool:\n",
    );
    text.push_str(if opts.allow_write {
        "- Changing the game installation, or rewriting a file in place: ALLOWED (this server was \
         started with --allow-write).\n"
    } else {
        "- Changing the game installation, or rewriting a file in place: BLOCKED. Deploy, \
         undeploy, `mgr apply`, `mgr reset` and the in-place edits will be refused. If the user \
         needs them, they must restart this server with --allow-write; you cannot enable it.\n"
    });
    // Compiling needs both flags, not just --allow-game-launch: it drives the game to regenerate
    // the cache and installs the result, so `Safety::requirements` marks every GameLaunch command
    // as a write too. Announcing it as ALLOWED on the strength of one flag would promise something
    // the gate then refuses.
    text.push_str(match (opts.allow_game_launch, opts.allow_write) {
        (true, true) => {
            "- Starting the game to compile AngelScript (`gore_as` compile, compile-module): \
             ALLOWED (started with --allow-game-launch and --allow-write). It opens a real game \
             window and takes minutes.\n"
        }
        (true, false) => {
            "- Starting the game to compile AngelScript (`gore_as` compile, compile-module): \
             BLOCKED. These need BOTH --allow-game-launch and --allow-write, because compiling \
             also stages files in the installation, and this server has only the first. The user \
             must restart it with both.\n"
        }
        (false, _) => {
            "- Starting the game to compile AngelScript (`gore_as` compile, compile-module): \
             BLOCKED. These need BOTH --allow-game-launch and --allow-write. The user must \
             restart this server with them; you cannot enable them.\n"
        }
    });
    text.push_str(
        "\nMany commands avoid the gate entirely by writing somewhere new: passing an output \
         argument turns an in-place rewrite into a new file. Prefer that.\n",
    );

    text.push_str(HOW_IT_BEHAVES);
    text
}

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
  gore_catalog   Regenerate reflection models and item/NPC/knowledge catalogs from a game dump.
  gore_project   Scaffold, compile and package a UE4SS Lua mod; install the shared Lua SDK.
  gore_loc       Localized text: decrypt the .lcache to JSON, edit it, re-encrypt.
  gore_audio     FMOD sound banks: list samples, extract to WAV, inject replacements, ship patches.
  gore_voice     Voice-over archives. Strictly copy-on-write; recorded audio is never overwritten.
  gore_texture   Textures in the IoStore containers: list, extract, replace, pack, deploy.
  gore_asset     Cooked DataAssets: receipt-sealed, byte-exact edits.
  gore_mod       Build one unified mod bundle and deploy or undeploy it as a single unit.
  gore_mgr       Run several mods at once: library, load order, conflict report, composed apply.
  gore_as        AngelScript cache: inspect, decompile, patch defaults, recompile modules.

Each of the last eleven wraps a family of subcommands. Choose one with `subcommand` and put its
arguments in `args`; the tool description lists every subcommand and what it accepts.

BEFORE YOU ACT
Read the guide page for whatever you are about to touch. These commands have sharp edges that a
flag list does not convey — receipts that must match, caches that must be regenerated first, steps
whose order matters. Call gore_guide with action "search"; it ranks individual sections, so the
follow-up read stays small.

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
    use std::path::PathBuf;

    fn options(allow_write: bool, allow_game_launch: bool) -> Options {
        let mut opts = Options::new(PathBuf::from("gore"), "0.1.0");
        opts.allow_write = allow_write;
        opts.allow_game_launch = allow_game_launch;
        opts
    }

    #[test]
    fn the_primer_states_which_tiers_are_unlocked() {
        let blocked = instructions(&options(false, false));
        assert!(blocked.contains("--allow-write"));
        assert!(blocked.contains("--allow-game-launch"));
        assert!(blocked.contains("BLOCKED"));

        let allowed = instructions(&options(true, true));
        assert!(allowed.contains("ALLOWED"));
        assert!(!allowed.contains("BLOCKED"));
    }

    #[test]
    fn the_primer_does_not_promise_that_every_write_is_free() {
        // It used to open with "reading anything, and writing new files, always works", which
        // stopped being true once commands whose targets cannot be checked became mutations. The
        // primer is the one thing every client reads, so an overstatement there is the most
        // expensive kind.
        let text = instructions(&options(false, false));
        assert!(!text.contains("writing new files, always works"), "{text}");
        assert!(text.contains("Reading anything always works"));
        assert!(
            text.contains("gated whatever is on disk"),
            "the primer must say that some writes are gated regardless"
        );
    }

    #[test]
    fn the_primer_never_promises_a_compile_the_gate_would_refuse() {
        // GameLaunch implies write in `Safety::requirements`, so --allow-game-launch alone is not
        // enough. The primer used to report compiling as ALLOWED on that flag by itself, which
        // told the model it could do something every attempt then refused.
        let launch_only = instructions(&options(false, true));
        assert!(
            launch_only.contains("compile-module): BLOCKED"),
            "one flag is not enough and the primer must say so"
        );
        assert!(launch_only.contains("BOTH"));

        let both = instructions(&options(true, true));
        assert!(both.contains("compile-module): ALLOWED"));

        // What the primer claims and what the gate does must agree in every combination.
        for (write, launch) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut opts = crate::Options::new(PathBuf::from("gore"), "0.1.0");
            opts.allow_write = write;
            opts.allow_game_launch = launch;
            let compile = crate::spec::group("gore_as")
                .and_then(|group| group.command("compile"))
                .expect("as compile exists");
            let permitted = !compile.safety.requirements(&serde_json::Map::new()).write
                || (write && launch);
            let claims_allowed = instructions(&opts).contains("compile-module): ALLOWED");
            assert_eq!(claims_allowed, permitted && launch, "mismatch at ({write}, {launch})");
        }
    }

    #[test]
    fn the_primer_points_at_the_guide() {
        let text = instructions(&options(false, false));
        assert!(text.contains("gore_guide"));
        assert!(text.contains("gore://guide/"));
        assert!(text.contains("BEFORE YOU ACT"));
    }

    #[test]
    fn the_primer_names_every_tool_the_server_advertises() {
        // The primer is the model's index of this server. A tool missing from it is a tool the
        // model has to stumble onto.
        let text = instructions(&options(false, false));
        for tool in crate::tool_definitions() {
            let name = tool["name"].as_str().expect("a tool name");
            assert!(text.contains(name), "the primer does not mention {name}");
        }
    }

    #[test]
    fn the_primer_stays_short_enough_to_carry_in_every_context() {
        // It is loaded into every conversation with this server, so length is a standing cost.
        // This is a budget, not a target: if it needs to grow, move the content into the guide.
        let text = instructions(&options(true, true));
        assert!(
            text.lines().count() < 70,
            "the primer has grown to {} lines; move detail into the guide",
            text.lines().count()
        );
    }

    #[test]
    fn the_primer_tells_the_model_it_cannot_lift_the_gate_itself() {
        // Without this an agent will keep retrying a refused command, or try to restart the server.
        let text = instructions(&options(false, false));
        assert!(text.contains("you cannot enable it"), "{text}");
    }

    #[test]
    fn a_supported_version_is_echoed_back_unchanged() {
        assert_eq!(negotiate_protocol_version(Some("2025-06-18")), "2025-06-18");
        assert_eq!(negotiate_protocol_version(Some("2024-11-05")), "2024-11-05");
    }

    #[test]
    fn an_unknown_version_falls_back_to_our_latest() {
        assert_eq!(negotiate_protocol_version(Some("1900-01-01")), LATEST_PROTOCOL_VERSION);
        // A modern-era request would arrive without an `initialize` at all, but a client that asks
        // for it through the handshake still gets a usable legacy answer rather than a hang.
        assert_eq!(negotiate_protocol_version(Some("2026-07-28")), LATEST_PROTOCOL_VERSION);
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
