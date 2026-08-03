//! `gore mcp serve` — the CLI entry point for the Model Context Protocol server.
//!
//! Deliberately thin. All protocol handling lives in the `gore-mcp` crate; this module only
//! decides which binary the server should re-exec, hands over the real stdio handles, and
//! translates failures into the CLI's `anyhow` convention.
//!
//! This is also the only place in the whole MCP path that touches the process-wide stdout. Keeping
//! it here — rather than letting the library reach for `io::stdout()` — is what makes it
//! structurally impossible for the server to interleave anything with JSON-RPC frames.

use std::io;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Result};

/// The `gore mcp serve` flags, kept as a struct because five of them in a row — three of which are
/// booleans — is an argument list two of which can be swapped without the compiler noticing.
#[derive(Debug)]
pub struct ServeOptions {
    pub allow_write: bool,
    pub allow_game_launch: bool,
    pub no_consent_prompts: bool,
    pub timeout_secs: u64,
    pub max_output_kib: usize,
}

/// The environment variables that stand in for the three permission flags.
///
/// A plugin cannot pass a flag. Its `.mcp.json` carries a fixed `args` array, the client starts the
/// server from exactly that, and nothing in between can add to it — so for anyone who installed the
/// plugin rather than hand-writing a client config, `--allow-write` had no reachable spelling at
/// all. Client-side `${VAR}` interpolation into `args` does exist, but an unset variable expands to
/// an empty argument and `gore mcp serve ""` is a parse error: the server would not start, and the
/// client could not say why. Reading the environment here instead costs nothing and works the same
/// in every client, plugin or not.
///
/// This widens nothing on its own. Setting one of these is the same act as typing the flag, by
/// someone who already controls how the server is launched.
const ALLOW_WRITE_ENV: &str = "GORE_MCP_ALLOW_WRITE";
const ALLOW_GAME_LAUNCH_ENV: &str = "GORE_MCP_ALLOW_GAME_LAUNCH";
const NO_CONSENT_PROMPTS_ENV: &str = "GORE_MCP_NO_CONSENT_PROMPTS";

pub fn serve(flags: ServeOptions) -> Result<()> {
    let flags = with_environment(flags, |name| std::env::var(name).ok())?;
    serve_resolved(flags)
}

/// Fold the environment into the flags that were typed.
///
/// Either source turning a permission on is enough; there is no spelling that turns one back off,
/// because both are the same person saying the same thing twice. The conflict check runs *after*
/// this, so `--allow-write` with `GORE_MCP_NO_CONSENT_PROMPTS=1` is caught exactly like the
/// all-flags version of the same contradiction.
///
/// `lookup` is a parameter rather than a direct `std::env::var` so the tests can be pure: the
/// process environment is global, and a test that sets it races every other test in the binary.
fn with_environment(
    flags: ServeOptions,
    lookup: impl Fn(&str) -> Option<String>,
) -> Result<ServeOptions> {
    Ok(ServeOptions {
        allow_write: flags.allow_write || switched_on(ALLOW_WRITE_ENV, lookup(ALLOW_WRITE_ENV))?,
        allow_game_launch: flags.allow_game_launch
            || switched_on(ALLOW_GAME_LAUNCH_ENV, lookup(ALLOW_GAME_LAUNCH_ENV))?,
        no_consent_prompts: flags.no_consent_prompts
            || switched_on(NO_CONSENT_PROMPTS_ENV, lookup(NO_CONSENT_PROMPTS_ENV))?,
        ..flags
    })
}

/// Read one environment variable as a switch, and refuse to guess.
///
/// Unset and empty are off, which is what makes the variables optional. Everything else has to be
/// one of the spellings below: a permission variable that is set to something unreadable is the one
/// case where either default is wrong. Treating it as off leaves someone believing the server is
/// pre-approved while every call stops to ask; treating it as on grants what they never wrote.
/// Refusing to start says which variable, and is the only answer that cannot mislead.
fn switched_on(name: &str, raw: Option<String>) -> Result<bool> {
    let Some(raw) = raw else { return Ok(false) };
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "0" | "false" | "no" | "off" => Ok(false),
        "1" | "true" | "yes" | "on" => Ok(true),
        other => bail!(
            "{name} is set to {other:?}, which is neither on (1, true, yes, on) nor off (0, \
             false, no, off, or unset). It decides whether this server may change your game \
             installation without asking, so it will not be guessed at."
        ),
    }
}

fn serve_resolved(flags: ServeOptions) -> Result<()> {
    // Saying "do not ask me" and "here is what you may do without asking" at once is a
    // contradiction, and silently picking one would leave someone believing the other. It matters:
    // one of the two readings runs commands that change the installation.
    if flags.no_consent_prompts && (flags.allow_write || flags.allow_game_launch) {
        bail!(
            "--no-consent-prompts refuses everything that would need confirming, so pairing it \
             with --allow-write or --allow-game-launch asks for both a stricter and a looser \
             server at once. Pass one or the other."
        );
    }

    let mut opts = gore_mcp::Options::new(resolve_self()?, env!("CARGO_PKG_VERSION"));
    opts.allow_write = flags.allow_write;
    opts.allow_game_launch = flags.allow_game_launch;
    opts.never_ask = flags.no_consent_prompts;
    opts.timeout_override_secs = flags.timeout_secs;
    opts.max_stdout_bytes = stdout_cap_bytes(flags.max_output_kib);

    let stdin = io::stdin();
    let stdout = io::stdout();
    gore_mcp::serve(opts, stdin.lock(), stdout.lock()).context("MCP server failed")
}

/// Resolve `--max-output-kib` to a byte cap.
///
/// `0` means "keep the built-in default", which is what `--timeout-secs 0` already means. Reading
/// it as a literal zero would leave a 1 KiB cap that truncates almost every result — and someone
/// passing zero to both flags is asking for defaults, not for that.
fn stdout_cap_bytes(max_output_kib: usize) -> usize {
    if max_output_kib == 0 {
        gore_mcp::DEFAULT_MAX_STDOUT_BYTES
    } else {
        max_output_kib.saturating_mul(1024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags() -> ServeOptions {
        ServeOptions {
            allow_write: false,
            allow_game_launch: false,
            no_consent_prompts: false,
            timeout_secs: 0,
            max_output_kib: 256,
        }
    }

    /// Resolve against a fixed environment, as `serve` does against the real one.
    fn with_env(flags: ServeOptions, env: &[(&str, &str)]) -> Result<ServeOptions> {
        with_environment(flags, |name| {
            env.iter().find(|(key, _)| *key == name).map(|(_, value)| (*value).to_string())
        })
    }

    #[test]
    fn asking_for_a_stricter_and_a_looser_server_at_once_is_rejected() {
        // Whichever way it were resolved, someone would be running a server that does the opposite
        // of what they typed — and one of the two readings changes the game installation.
        for (write, launch) in [(true, false), (false, true), (true, true)] {
            let contradictory = ServeOptions {
                allow_write: write,
                allow_game_launch: launch,
                no_consent_prompts: true,
                ..flags()
            };
            let error = serve_resolved(contradictory).expect_err("must not start");
            assert!(error.to_string().contains("--no-consent-prompts"), "{error}");
        }
    }

    #[test]
    fn an_environment_variable_grants_exactly_what_its_flag_would() {
        // The whole point: a plugin's `.mcp.json` carries a fixed `args` array, so this is the only
        // spelling of `--allow-write` available to anyone who installed the plugin.
        let resolved = with_env(flags(), &[(ALLOW_WRITE_ENV, "1")]).expect("valid");
        assert!(resolved.allow_write);
        assert!(!resolved.allow_game_launch, "one variable grants one permission");
        assert!(!resolved.no_consent_prompts);

        let launching = with_env(flags(), &[(ALLOW_GAME_LAUNCH_ENV, "true")]).expect("valid");
        assert!(launching.allow_game_launch);
        assert!(!launching.allow_write, "the launch permission does not imply the write one here");

        let strict = with_env(flags(), &[(NO_CONSENT_PROMPTS_ENV, "yes")]).expect("valid");
        assert!(strict.no_consent_prompts);
    }

    #[test]
    fn the_flag_and_the_variable_are_the_same_person_saying_the_same_thing() {
        // Either alone is enough, both together is not an error, and nothing that was typed is
        // taken away by the environment being quiet about it.
        let typed = ServeOptions { allow_write: true, ..flags() };
        assert!(with_env(typed, &[]).expect("valid").allow_write);

        let both = ServeOptions { allow_write: true, ..flags() };
        assert!(with_env(both, &[(ALLOW_WRITE_ENV, "1")]).expect("valid").allow_write);
    }

    #[test]
    fn an_unset_or_off_variable_leaves_the_gate_where_it_was() {
        for off in ["", "0", "false", "no", "off", "OFF", "  false  "] {
            let resolved = with_env(flags(), &[(ALLOW_WRITE_ENV, off)])
                .unwrap_or_else(|error| panic!("{off:?} should parse: {error}"));
            assert!(!resolved.allow_write, "{off:?} must not pre-approve anything");
        }
        assert!(!with_env(flags(), &[]).expect("valid").allow_write);
    }

    #[test]
    fn a_value_that_is_neither_on_nor_off_refuses_to_start() {
        // The one case where both defaults are wrong. Off leaves someone believing the server is
        // pre-approved while every call stops to ask; on grants what they never wrote. Refusing
        // names the variable, which is the only answer that cannot mislead.
        for nonsense in ["yess", "2", "enabled", "please", "-"] {
            let resolved = with_env(flags(), &[(ALLOW_WRITE_ENV, nonsense)]);
            let error = match resolved {
                Err(error) => error,
                Ok(options) => panic!("{nonsense:?} must be refused, got {options:?}"),
            };
            let text = error.to_string();
            assert!(text.contains(ALLOW_WRITE_ENV), "{text}");
            assert!(text.contains(nonsense), "the message has to quote what was set: {text}");
        }
    }

    #[test]
    fn a_contradiction_is_caught_however_its_two_halves_arrive() {
        // Splitting it across a flag and a variable is the shape that would slip through a check
        // that ran before the environment was folded in.
        let error = with_env(ServeOptions { allow_write: true, ..flags() }, &[
            (NO_CONSENT_PROMPTS_ENV, "1"),
        ])
        .and_then(serve_resolved)
        .expect_err("must not start");
        assert!(error.to_string().contains("--no-consent-prompts"), "{error}");

        let reversed = with_env(ServeOptions { no_consent_prompts: true, ..flags() }, &[
            (ALLOW_WRITE_ENV, "1"),
        ])
        .and_then(serve_resolved)
        .expect_err("must not start");
        assert!(reversed.to_string().contains("--no-consent-prompts"), "{reversed}");
    }

    #[test]
    fn zero_means_the_default_cap_just_as_it_does_for_the_timeout() {
        assert_eq!(stdout_cap_bytes(0), gore_mcp::DEFAULT_MAX_STDOUT_BYTES);
        assert_eq!(stdout_cap_bytes(256), 256 * 1024);
        assert_eq!(stdout_cap_bytes(1), 1024);
        // A cap large enough to overflow the multiply saturates rather than wrapping to nothing.
        assert_eq!(stdout_cap_bytes(usize::MAX), usize::MAX);
    }
}

/// Print what `tools/list` would return.
///
/// Exists so a client integration can be debugged, and the exposed surface reviewed, without
/// speaking JSON-RPC to a running server.
pub fn tools() -> Result<()> {
    let definitions = gore_mcp::tool_definitions();
    println!(
        "{}",
        serde_json::to_string_pretty(&definitions).context("serializing tool definitions")?
    );
    Ok(())
}

/// Find and verify the `gore` binary that tool calls will re-exec.
///
/// Resolved once, at startup, on purpose. `current_exe()` can point at something moved, renamed or
/// shimmed, and discovering that halfway through a session — after the agent has already been told
/// a command succeeded — is far worse than refusing to start. The check is a single `--version`
/// spawn, which costs milliseconds and rules out re-execing an unrelated program.
fn resolve_self() -> Result<PathBuf> {
    let exe = std::env::current_exe()
        .context("could not determine the path of the running gore executable")?;

    let output = Command::new(&exe).arg("--version").output().with_context(|| {
        format!("could not run `{}` to verify it is the gore CLI", exe.display())
    })?;
    let reported = String::from_utf8_lossy(&output.stdout);

    // Deliberately a prefix check, not an equality check: the crate pins its own version while the
    // workspace stays at 0.0.0, so comparing version strings would be comparing the wrong things.
    if !output.status.success() || !reported.starts_with("gore ") {
        bail!(
            "`{}` does not identify itself as the gore CLI (`--version` printed {:?}); \
             refusing to re-exec it",
            exe.display(),
            reported.trim()
        );
    }

    Ok(exe)
}
