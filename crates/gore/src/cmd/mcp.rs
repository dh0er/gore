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

pub fn serve(
    allow_write: bool,
    allow_game_launch: bool,
    timeout_secs: u64,
    max_output_kib: usize,
) -> Result<()> {
    let mut opts = gore_mcp::Options::new(resolve_self()?, env!("CARGO_PKG_VERSION"));
    opts.allow_write = allow_write;
    opts.allow_game_launch = allow_game_launch;
    opts.timeout_override_secs = timeout_secs;
    opts.max_stdout_bytes = stdout_cap_bytes(max_output_kib);

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
