//! Running a `gore` subcommand and turning what came back into a tool result.
//!
//! The [`Spawn`] trait exists so the whole `tools/call` path — schema, argv, safety gate, result
//! rendering — can be exercised without a child process or a game installation. [`ProcessSpawn`] is
//! the real thing; [`FakeSpawn`] is what the tests use.

use std::ffi::OsStr;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::argv::Invocation;
use crate::spec::{Class, CommandSpec};

/// How often we check whether the child has finished.
///
/// Polling rather than a blocking wait with a timeout: the alternatives are a new dependency or
/// raw platform handles, and this crate deliberately carries neither. 25 ms is invisible next to
/// commands that take between milliseconds and half an hour.
const POLL_INTERVAL: Duration = Duration::from_millis(25);

/// Read buffer for draining the child's pipes.
const PIPE_CHUNK_BYTES: usize = 16 * 1024;

/// How long to keep waiting for the output readers once the child itself is gone.
///
/// Killing a process does not kill its children, and a grandchild inherits the pipe handles. So a
/// killed `gore as compile` can leave the game holding our stdout pipe open, and a reader thread
/// blocked on it would never return. Waiting for the readers unconditionally would turn that into a
/// hung MCP session — the exact failure the timeout exists to prevent. Instead we give them a short
/// grace period and then take whatever they have captured so far.
const READER_GRACE: Duration = Duration::from_millis(500);

/// stderr is progress chatter and error text, never a payload, so it gets a fixed small cap.
pub const MAX_STDERR_BYTES: usize = 32 * 1024;

/// What a finished (or killed) child left behind.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    /// `None` when the process was killed or died without a code.
    pub status: Option<i32>,
    pub stdout: String,
    pub stdout_truncated: bool,
    pub stdout_total: usize,
    pub stderr: String,
    pub stderr_truncated: bool,
    pub stderr_total: usize,
    pub timed_out: bool,
    pub duration_ms: u128,
}

impl Outcome {
    pub fn succeeded(&self) -> bool {
        !self.timed_out && self.status == Some(0)
    }

    /// A successful run that printed `stdout`. For tests.
    pub fn success(stdout: impl Into<String>) -> Self {
        let stdout = stdout.into();
        let stdout_total = stdout.len();
        Self { status: Some(0), stdout, stdout_total, ..Self::default() }
    }

    /// A failed run that printed `stderr`. For tests.
    pub fn failure(code: i32, stderr: impl Into<String>) -> Self {
        let stderr = stderr.into();
        let stderr_total = stderr.len();
        Self { status: Some(code), stderr, stderr_total, ..Self::default() }
    }

    /// A run that hit its deadline and was killed. For tests.
    pub fn timed_out() -> Self {
        Self { status: None, timed_out: true, ..Self::default() }
    }
}

pub trait Spawn: Send + Sync {
    fn run(&self, invocation: &Invocation) -> io::Result<Outcome>;

    /// The program this runner starts, for the reproducible command line shown to the reader.
    ///
    /// Not derivable from the invocation: an `Invocation` carries only the arguments, and the
    /// binary is chosen once at startup.
    fn display_exe(&self) -> String;
}

/// Lets a caller keep a handle on the runner it gave the session — which is how a test inspects
/// what was actually spawned after the fact.
impl<T: Spawn + ?Sized> Spawn for std::sync::Arc<T> {
    fn display_exe(&self) -> String {
        (**self).display_exe()
    }

    fn run(&self, invocation: &Invocation) -> io::Result<Outcome> {
        (**self).run(invocation)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_stdout_bytes: usize,
    pub max_stderr_bytes: usize,
}

/// Spawns the real `gore` binary.
pub struct ProcessSpawn {
    exe: PathBuf,
    limits: Limits,
}

impl ProcessSpawn {
    pub fn new(exe: PathBuf, max_stdout_bytes: usize) -> Self {
        Self { exe, limits: Limits { max_stdout_bytes, max_stderr_bytes: MAX_STDERR_BYTES } }
    }
}

impl Spawn for ProcessSpawn {
    fn display_exe(&self) -> String {
        crate::argv::invoke_program(&self.exe)
    }

    fn run(&self, invocation: &Invocation) -> io::Result<Outcome> {
        let started = Instant::now();

        let mut child = Command::new(&self.exe)
            .args(invocation.argv.iter().map(OsStr::new))
            // Our own stdin is the JSON-RPC channel and must never be shared. A child that prompts
            // then reads EOF and fails fast instead of deadlocking the whole session — which is
            // what `gore loc extract` would do without its forced `-y`.
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout_pipe = child.stdout.take().expect("stdout was piped");
        let stderr_pipe = child.stderr.take().expect("stderr was piped");
        let stdout_cap = self.limits.max_stdout_bytes;
        let stderr_cap = self.limits.max_stderr_bytes;

        // Drain both pipes on their own threads. A child that fills a pipe we are not reading
        // blocks forever, which would turn a large output into a hang rather than a truncation.
        // The captures are shared rather than returned so that their contents stay reachable even
        // if a reader is still blocked when we give up on it — see READER_GRACE.
        let stdout_capture = Arc::new(Capture::default());
        let stderr_capture = Arc::new(Capture::default());
        thread::spawn({
            let capture = Arc::clone(&stdout_capture);
            move || drain_into(stdout_pipe, stdout_cap, &capture)
        });
        thread::spawn({
            let capture = Arc::clone(&stderr_capture);
            move || drain_into(stderr_pipe, stderr_cap, &capture)
        });

        let deadline = started + invocation.timeout;
        let mut timed_out = false;
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status.code();
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                timed_out = true;
                break None;
            }
            thread::sleep(POLL_INTERVAL);
        };

        await_readers(&[&stdout_capture, &stderr_capture]);
        let (stdout, stdout_truncated, stdout_total) = stdout_capture.harvest();
        let (stderr, stderr_truncated, stderr_total) = stderr_capture.harvest();

        Ok(Outcome {
            status,
            stdout,
            stdout_truncated,
            stdout_total,
            stderr,
            stderr_truncated,
            stderr_total,
            timed_out,
            duration_ms: started.elapsed().as_millis(),
        })
    }
}

/// A pipe's captured prefix, readable while the reader thread is still running.
#[derive(Default)]
struct Capture {
    kept: Mutex<Vec<u8>>,
    total: AtomicUsize,
    finished: AtomicBool,
}

impl Capture {
    fn harvest(&self) -> (String, bool, usize) {
        let kept = self.kept.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let total = self.total.load(Ordering::Relaxed);
        (String::from_utf8_lossy(&kept).into_owned(), total > kept.len(), total)
    }
}

/// Read up to `cap` bytes into `capture`, but keep draining afterwards so the writer never blocks.
fn drain_into(mut reader: impl Read, cap: usize, capture: &Capture) {
    let mut buf = vec![0u8; PIPE_CHUNK_BYTES];

    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(read) => {
                capture.total.fetch_add(read, Ordering::Relaxed);
                let mut kept = capture.kept.lock().unwrap_or_else(|p| p.into_inner());
                if kept.len() < cap {
                    let take = (cap - kept.len()).min(read);
                    kept.extend_from_slice(&buf[..take]);
                }
            }
            Err(ref error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    capture.finished.store(true, Ordering::Release);
}

/// Give the readers a bounded chance to finish.
///
/// If a grandchild is still holding the pipe open, the reader never returns; we take the prefix it
/// has already captured and leave the thread to exit on its own. That leaks at most one thread per
/// timed-out call, which is a far better failure than an MCP session that never answers again.
fn await_readers(captures: &[&Arc<Capture>]) {
    let deadline = Instant::now() + READER_GRACE;
    while Instant::now() < deadline {
        if captures.iter().all(|capture| capture.finished.load(Ordering::Acquire)) {
            return;
        }
        thread::sleep(POLL_INTERVAL);
    }
}

/// Read a stream to completion with a cap. Used by tests; the spawn path uses [`drain_into`]
/// directly so that a blocked reader cannot hold up the result.
#[cfg(test)]
fn read_capped(reader: impl Read, cap: usize) -> (String, bool, usize) {
    let capture = Capture::default();
    drain_into(reader, cap, &capture);
    capture.harvest()
}

/// A scripted [`Spawn`] that records what it was asked to run.
pub struct FakeSpawn {
    outcome: Outcome,
    calls: Mutex<Vec<Invocation>>,
}

impl FakeSpawn {
    pub fn new(outcome: Outcome) -> Self {
        Self { outcome, calls: Mutex::new(Vec::new()) }
    }

    pub fn calls(&self) -> Vec<Invocation> {
        self.calls.lock().expect("fake spawn lock").clone()
    }
}

impl Spawn for FakeSpawn {
    fn display_exe(&self) -> String {
        "gore".to_string()
    }

    fn run(&self, invocation: &Invocation) -> io::Result<Outcome> {
        self.calls.lock().expect("fake spawn lock").push(invocation.clone());
        Ok(self.outcome.clone())
    }
}

/// Render an outcome as an MCP `CallToolResult`.
///
/// A failed command is reported as a *successful* response carrying `isError: true`. That is the
/// specification's design: the model has to be able to read the failure and adapt, which a
/// JSON-RPC error does not reliably allow.
pub fn to_call_result(
    invocation: &Invocation,
    command: &CommandSpec,
    outcome: &Outcome,
) -> Value {
    let mut content = vec![text_block(&invocation.display)];

    if !outcome.succeeded() {
        content.push(text_block(&failure_summary(
            command,
            outcome,
            invocation.timeout,
            &invocation.path,
        )));
    }

    content.push(text_block(&stream_block("stdout", &outcome.stdout, outcome.stdout_truncated, outcome.stdout_total, true)));

    if !outcome.stderr.trim().is_empty() {
        content.push(text_block(&stream_block(
            "stderr",
            &outcome.stderr,
            outcome.stderr_truncated,
            outcome.stderr_total,
            false,
        )));
    }

    // Deliberately no `structuredContent`. See `no_result_carries_structured_content` in lib.rs:
    // a client that sees that member is entitled to treat it as *the* result and ignore `content`,
    // and a summary of byte counts is not a result. Everything worth knowing — the command line,
    // the exit code, what was printed, what was cut — is in the blocks above.
    json!({
        "content": content,
        "isError": !outcome.succeeded(),
    })
}

/// Add one more text block to a rendered result.
///
/// Used for what is true of the *call* rather than of the command's output — today, that it ran on
/// a claim of approval nobody here verified. It goes last so that the command line stays the first
/// thing a reader meets, and so that no existing block moves.
pub fn append_note(result: &mut Value, note: String) {
    if let Some(content) = result.get_mut("content").and_then(Value::as_array_mut) {
        content.push(text_block(&note));
    }
}

/// A tool error that never reached a child process — a bad argument or a refusal.
pub fn to_error_result(message: impl Into<String>) -> Value {
    json!({
        "content": [text_block(&message.into())],
        "isError": true,
    })
}

fn failure_summary(
    command: &CommandSpec,
    outcome: &Outcome,
    timeout: Duration,
    path: &str,
) -> String {
    let mut summary = if outcome.timed_out {
        let mut text = format!(
            "`gore {path}` did not finish within {}s and was killed. Any work it had already \
             written to disk is still there.",
            timeout.as_secs()
        );
        // Killing the CLI does not kill anything it started. Saying so matters here: the user may
        // have a game window open that nobody is going to close for them.
        if command.safety.base == Class::GameLaunch {
            text.push_str(
                " This command starts the game, and that process is not stopped by the timeout — \
                 check for a running game before trying again.",
            );
        }
        text
    } else {
        match outcome.status {
            Some(code) => format!("`gore {path}` failed with exit code {code}."),
            None => format!("`gore {path}` was terminated without an exit code."),
        }
    };

    if let Some(page) = command.guide {
        summary.push_str(&format!(
            " If the cause is not obvious from the output below, read `gore://guide/{page}`."
        ));
    }

    summary
}

fn stream_block(
    name: &str,
    body: &str,
    truncated: bool,
    total: usize,
    show_when_empty: bool,
) -> String {
    if body.trim().is_empty() {
        return if show_when_empty { format!("({name} was empty)") } else { String::new() };
    }

    let mut block = if name == "stdout" { body.to_string() } else { format!("{name}:\n{body}") };
    if truncated {
        block.push_str(&format!(
            "\n\n… [truncated: {name} produced {total} bytes and only the first part is shown. \
             Narrow the query with a filter, or write the full output to a file with the \
             command's own output argument.]"
        ));
    }
    block
}

fn text_block(text: &str) -> Value {
    json!({ "type": "text", "text": text })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::argv;
    use crate::server::Options;
    use crate::spec;
    use serde_json::json;

    fn options() -> Options {
        let mut opts = Options::new(PathBuf::from("gore"), "0.1.0");
        opts.allow_write = true;
        opts
    }

    fn invocation(tool: &str, sub: &str, args: Value) -> (Invocation, &'static CommandSpec) {
        let group = spec::group(tool).expect("group");
        let invocation = argv::build(group, sub, &args, &options()).expect("build");
        (invocation, group.command(sub).expect("command"))
    }

    #[test]
    fn a_successful_run_is_not_an_error_and_leads_with_the_command_line() {
        let (inv, command) = invocation("gore_config", "path", json!({}));
        let result = to_call_result(&inv, command, &Outcome::success("C:/x/config.json\n"));

        assert_eq!(result["isError"], json!(false));
        assert_eq!(result["content"][0]["text"], "gore config path");
        assert_eq!(result["content"][1]["text"], "C:/x/config.json\n");
        // A success says nothing further: the exit code is only worth a line when it is not zero.
        assert_eq!(result["content"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn a_non_zero_exit_is_a_tool_error_that_surfaces_stderr() {
        let (inv, command) = invocation("gore_config", "get", json!({ "key": "game-path" }));
        let result =
            to_call_result(&inv, command, &Outcome::failure(1, "error: game-path is not set\n"));

        assert_eq!(result["isError"], json!(true));
        assert!(result["content"][1]["text"].as_str().unwrap().contains("exit code 1"));
        let stderr = result["content"][3]["text"].as_str().unwrap();
        assert!(stderr.contains("game-path is not set"), "{stderr}");
    }

    #[test]
    fn a_failure_points_at_the_guide_page_for_that_command() {
        let (inv, command) = invocation("gore_config", "get", json!({ "key": "game-path" }));
        let result = to_call_result(&inv, command, &Outcome::failure(1, "boom"));
        assert!(result["content"][1]["text"]
            .as_str()
            .unwrap()
            .contains("gore://guide/getting-started"));
    }

    #[test]
    fn a_timeout_says_so_and_reports_the_limit_that_was_hit() {
        let (inv, command) = invocation("gore_config", "path", json!({}));
        let result = to_call_result(&inv, command, &Outcome::timed_out());

        assert_eq!(result["isError"], json!(true));
        let summary = result["content"][1]["text"].as_str().unwrap();
        // The text is the only channel, so "it was killed" has to be readable in it rather than
        // inferable from a null exit code in a member no model may ever see.
        assert!(summary.contains("did not finish within 60s"), "{summary}");
    }

    #[test]
    fn truncated_output_says_how_much_was_dropped_and_what_to_do() {
        let (inv, command) = invocation("gore_config", "path", json!({}));
        let outcome = Outcome {
            status: Some(0),
            stdout: "first part".into(),
            stdout_truncated: true,
            stdout_total: 5_000_000,
            ..Outcome::default()
        };
        let result = to_call_result(&inv, command, &outcome);
        let body = result["content"][1]["text"].as_str().unwrap();

        assert!(body.contains("truncated"), "{body}");
        assert!(body.contains("5000000"), "{body}");
    }

    #[test]
    fn empty_output_is_stated_rather_than_shown_as_a_blank_block() {
        let (inv, command) = invocation("gore_config", "path", json!({}));
        let result = to_call_result(&inv, command, &Outcome::success(""));
        assert_eq!(result["content"][1]["text"], "(stdout was empty)");
        // An empty stderr adds no block at all.
        assert_eq!(result["content"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn the_fake_spawn_records_what_it_was_asked_to_run() {
        let (inv, _) = invocation("gore_config", "path", json!({}));
        let spawn = FakeSpawn::new(Outcome::success("ok"));

        let outcome = spawn.run(&inv).expect("fake run");
        assert!(outcome.succeeded());
        assert_eq!(spawn.calls().len(), 1);
        assert_eq!(spawn.calls()[0].display, "gore config path");
    }

    #[test]
    fn read_capped_keeps_the_prefix_and_still_reports_the_full_size() {
        let source = "abcdefghij".repeat(10); // 100 bytes
        let (kept, truncated, total) = read_capped(source.as_bytes(), 16);

        assert_eq!(kept.len(), 16);
        assert!(truncated);
        assert_eq!(total, 100);
    }

    #[test]
    fn read_capped_reports_no_truncation_when_everything_fits() {
        let (kept, truncated, total) = read_capped("short".as_bytes(), 1024);
        assert_eq!(kept, "short");
        assert!(!truncated);
        assert_eq!(total, 5);
    }

    /// Real-process coverage for the spawn, capture and kill paths.
    ///
    /// These use the OS shell rather than `gore` so they stay in this crate, which has no
    /// dependency on the CLI. Windows-only because that is the platform this toolkit targets and
    /// the only one CI runs; the code under test is platform-neutral.
    #[cfg(windows)]
    mod real_processes {
        use super::*;

        fn shell(command: &str, timeout: Duration) -> (ProcessSpawn, Invocation) {
            let spawn = ProcessSpawn::new(PathBuf::from("cmd"), 64 * 1024);
            let invocation = Invocation {
                argv: vec!["/C".into(), command.into()],
                path: "shell".into(),
                timeout,
                display: format!("cmd /C {command}"),
                consent: None,
            };
            (spawn, invocation)
        }

        #[test]
        fn a_child_that_finishes_reports_its_output_and_exit_code() {
            let (spawn, invocation) = shell("echo hello", Duration::from_secs(30));
            let outcome = spawn.run(&invocation).expect("spawn");

            assert!(outcome.succeeded(), "{outcome:?}");
            assert!(outcome.stdout.contains("hello"), "{:?}", outcome.stdout);
            assert!(!outcome.timed_out);
        }

        #[test]
        fn a_non_zero_exit_is_reported_rather_than_raised() {
            let (spawn, invocation) = shell("exit 3", Duration::from_secs(30));
            let outcome = spawn.run(&invocation).expect("spawn");

            assert_eq!(outcome.status, Some(3));
            assert!(!outcome.succeeded());
        }

        #[test]
        fn a_child_that_outruns_its_deadline_is_killed() {
            // The child would run for about half a minute; the deadline is one second. The gap is
            // deliberately huge: with a child that finishes near the assertion bound, a slow
            // machine makes "killed on time" and "ran to completion" indistinguishable, and the
            // test flakes under parallel load instead of reporting anything.
            let (spawn, invocation) = shell("ping -n 30 127.0.0.1 > NUL", Duration::from_secs(1));
            let started = Instant::now();
            let outcome = spawn.run(&invocation).expect("spawn");

            assert!(outcome.timed_out, "{outcome:?}");
            assert_eq!(outcome.status, None);
            assert!(
                started.elapsed() < Duration::from_secs(15),
                "the deadline should have cut this short, took {:?}",
                started.elapsed()
            );
        }

        #[test]
        fn output_larger_than_the_cap_is_truncated_without_blocking_the_child() {
            // The child writes far more than the cap. If the pipes were not drained past the cap,
            // it would block on a full pipe and this test would hang rather than fail.
            let spawn = ProcessSpawn::new(PathBuf::from("cmd"), 256);
            let invocation = Invocation {
                argv: vec!["/C".into(), "for /L %i in (1,1,2000) do @echo aaaaaaaaaaaaaaaaaaaa".into()],
                path: "shell".into(),
                timeout: Duration::from_secs(60),
                display: "cmd /C …".into(),
                consent: None,
            };
            let outcome = spawn.run(&invocation).expect("spawn");

            assert!(outcome.succeeded(), "{outcome:?}");
            assert!(outcome.stdout_truncated);
            assert!(outcome.stdout.len() <= 256);
            assert!(outcome.stdout_total > 10_000, "total was {}", outcome.stdout_total);
        }

        #[test]
        fn a_child_that_reads_stdin_gets_eof_instead_of_hanging() {
            // Our stdin is the JSON-RPC channel. A child must never be able to wait on it — this is
            // what stops `loc extract`'s confirmation prompt from deadlocking the session.
            let (spawn, invocation) = shell("set /p answer=", Duration::from_secs(10));
            let outcome = spawn.run(&invocation).expect("spawn");
            assert!(!outcome.timed_out, "the child blocked on stdin instead of seeing EOF");
        }

        #[test]
        fn a_binary_that_does_not_exist_is_an_error_rather_than_a_failed_outcome() {
            let spawn = ProcessSpawn::new(PathBuf::from("gore-does-not-exist-xyz"), 1024);
            let invocation = Invocation {
                argv: vec!["--version".into()],
                path: "shell".into(),
                timeout: Duration::from_secs(5),
                display: "…".into(),
                consent: None,
            };
            assert!(spawn.run(&invocation).is_err());
        }
    }

    #[test]
    fn a_build_error_renders_as_a_tool_error_without_a_process() {
        let result = to_error_result("refused: nope");
        assert_eq!(result["isError"], json!(true));
        assert_eq!(result["content"][0]["text"], "refused: nope");
    }
}
