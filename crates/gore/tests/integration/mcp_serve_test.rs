//! End-to-end tests for `gore mcp serve` over the real stdio transport.
//!
//! These drive an actual child process rather than the in-memory streams the `gore-mcp` unit tests
//! use. That is the point: they prove the things only a real process can prove — that the server
//! re-execs a binary it recognises, that nothing except JSON-RPC reaches stdout, and that closing
//! stdin is a clean shutdown rather than an error exit.
//!
//! The shared config dir is redirected into a TempDir and Steam auto-detect is disabled, exactly as
//! in `config_test.rs`, so a developer's real installation is never consulted.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};
use tempfile::TempDir;

/// Generous enough that a cold-start debug binary on a loaded CI machine cannot flake, short
/// enough that a genuine deadlock fails the run instead of hanging it. The test harness has no
/// per-test timeout of its own, so this is the only thing standing between a protocol bug and a
/// stalled CI job.
const REPLY_TIMEOUT: Duration = Duration::from_secs(30);

struct Server {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
}

impl Server {
    fn spawn(home: &Path) -> Self {
        let mut child = Command::new(assert_cmd::cargo::cargo_bin("gore"))
            .args(["mcp", "serve"])
            .env("LOCALAPPDATA", home)
            .env("APPDATA", home)
            .env("XDG_DATA_HOME", home)
            .env("HOME", home)
            .env("GORE_DISABLE_GAME_AUTODETECT", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("gore mcp serve should start");

        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");

        // Read on a worker so that a missing reply times out instead of blocking forever.
        let (tx, lines) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { return };
                if tx.send(line).is_err() {
                    return;
                }
            }
        });

        Self { child, stdin: Some(stdin), lines }
    }

    fn send(&mut self, message: Value) {
        let stdin = self.stdin.as_mut().expect("stdin is still open");
        writeln!(stdin, "{message}").expect("write request");
        stdin.flush().expect("flush request");
    }

    fn recv(&mut self) -> Value {
        match self.lines.recv_timeout(REPLY_TIMEOUT) {
            Ok(line) => serde_json::from_str(&line)
                .unwrap_or_else(|error| panic!("stdout must carry only JSON-RPC: {error} in {line:?}")),
            Err(RecvTimeoutError::Timeout) => panic!("no reply within {REPLY_TIMEOUT:?}"),
            Err(RecvTimeoutError::Disconnected) => panic!("the server closed stdout unexpectedly"),
        }
    }

    /// Close stdin — the stdio shutdown handshake — and report how the server exited.
    fn shutdown(mut self) -> (Option<i32>, String) {
        drop(self.stdin.take());
        let output = self.child.wait_with_output().expect("wait for the server to exit");
        (output.status.code(), String::from_utf8_lossy(&output.stderr).into_owned())
    }

    fn initialize(&mut self) -> Value {
        self.send(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "gore-integration-test", "version": "0" },
            },
        }));
        let response = self.recv();
        self.send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        response
    }
}

#[test]
fn initialize_negotiates_and_identifies_the_server() {
    let tmp = TempDir::new().unwrap();
    let mut server = Server::spawn(tmp.path());

    let response = server.initialize();
    assert_eq!(response["id"], 1);

    let result = &response["result"];
    assert_eq!(result["protocolVersion"], "2025-11-25");
    assert_eq!(result["serverInfo"]["name"], "gore");
    assert!(
        result["serverInfo"]["version"].as_str().is_some_and(|v| !v.is_empty()),
        "serverInfo.version should report the gore CLI version"
    );
    assert!(result["capabilities"].get("tools").is_some());
    assert!(
        result["instructions"].as_str().is_some_and(|text| text.contains("gore_guide")),
        "the primer must point at the guide"
    );

    let (code, stderr) = server.shutdown();
    assert_eq!(code, Some(0), "stderr was: {stderr}");
}

#[test]
fn the_initialized_notification_is_not_answered() {
    let tmp = TempDir::new().unwrap();
    let mut server = Server::spawn(tmp.path());
    server.initialize();

    // `initialize` already sent the notification. If the server wrongly answered it, that reply
    // would be sitting in the stream and this ping would read it instead — so checking the id is a
    // deterministic silence check that needs no sleep.
    server.send(json!({ "jsonrpc": "2.0", "id": 2, "method": "ping" }));
    let response = server.recv();
    assert_eq!(response["id"], 2, "a notification must produce no reply");
    assert_eq!(response["result"], json!({}));

    let (code, stderr) = server.shutdown();
    assert_eq!(code, Some(0), "stderr was: {stderr}");
}

#[test]
fn tools_list_advertises_every_group_in_the_table() {
    let tmp = TempDir::new().unwrap();
    let mut server = Server::spawn(tmp.path());
    server.initialize();

    server.send(json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/list" }));
    let response = server.recv();
    assert_eq!(response["id"], 3);

    let tools = response["result"]["tools"].as_array().expect("an array of tools").clone();
    let names: Vec<&str> = tools.iter().map(|tool| tool["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"gore_config"), "{names:?}");
    assert!(names.contains(&"gore_as"), "{names:?}");
    assert!(names.contains(&"gore_mgr"), "{names:?}");
    assert!(
        names.len() >= 11,
        "expected one tool per command group, got {}: {names:?}",
        names.len()
    );

    let (code, stderr) = server.shutdown();
    assert_eq!(code, Some(0), "stderr was: {stderr}");
}

#[test]
fn a_tool_call_runs_the_real_cli_and_returns_its_output() {
    // The whole path for real: schema -> argv -> safety gate -> child process -> result. `config
    // path` is chosen because it is read-only, needs no game installation, and prints something
    // deterministic.
    let tmp = TempDir::new().unwrap();
    let mut server = Server::spawn(tmp.path());
    server.initialize();

    server.send(json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "tools/call",
        "params": { "name": "gore_config", "arguments": { "subcommand": "path" } },
    }));
    let result = server.recv()["result"].clone();

    assert_eq!(result["isError"], json!(false), "{result}");
    assert_eq!(result["content"][0]["text"], "gore config path");
    let stdout = result["content"][1]["text"].as_str().unwrap();
    assert!(stdout.contains("config.json"), "{stdout}");
    assert_eq!(result["structuredContent"]["exit_code"], 0);

    let (code, stderr) = server.shutdown();
    assert_eq!(code, Some(0), "stderr was: {stderr}");
}

#[test]
fn arguments_reach_the_cli_in_the_right_order() {
    // `config set` takes two positionals. If the argv builder emitted them out of order, or lost
    // the `--` separator, this round trip would store the wrong value — or fail outright.
    let tmp = TempDir::new().unwrap();
    let mut server = Server::spawn(tmp.path());
    server.initialize();

    server.send(json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "tools/call",
        "params": {
            "name": "gore_config",
            "arguments": {
                "subcommand": "set",
                "args": { "key": "game-path", "value": "D:/Games/G1R" },
            },
        },
    }));
    let set = server.recv()["result"].clone();
    assert_eq!(set["isError"], json!(false), "{set}");

    server.send(json!({
        "jsonrpc": "2.0",
        "id": 12,
        "method": "tools/call",
        "params": {
            "name": "gore_config",
            "arguments": { "subcommand": "get", "args": { "key": "game-path" } },
        },
    }));
    let get = server.recv()["result"].clone();
    assert_eq!(get["isError"], json!(false), "{get}");
    assert!(
        get["content"][1]["text"].as_str().unwrap().contains("G1R"),
        "the value did not round trip: {get}"
    );

    let (code, stderr) = server.shutdown();
    assert_eq!(code, Some(0), "stderr was: {stderr}");
}

#[test]
fn a_failing_command_is_reported_as_a_tool_error_with_its_message() {
    // Nothing is configured in this TempDir, so `config get` exits non-zero.
    let tmp = TempDir::new().unwrap();
    let mut server = Server::spawn(tmp.path());
    server.initialize();

    server.send(json!({
        "jsonrpc": "2.0",
        "id": 13,
        "method": "tools/call",
        "params": {
            "name": "gore_config",
            "arguments": { "subcommand": "get", "args": { "key": "game-path" } },
        },
    }));
    let response = server.recv();

    assert!(response.get("error").is_none(), "a failing command is not a protocol error");
    let result = &response["result"];
    assert_eq!(result["isError"], json!(true), "{result}");
    assert_eq!(result["structuredContent"]["exit_code"], 1);
    let rendered = result["content"].to_string();
    assert!(rendered.contains("game-path is not set"), "{rendered}");

    let (code, stderr) = server.shutdown();
    assert_eq!(code, Some(0), "stderr was: {stderr}");
}

#[test]
fn a_destructive_command_is_refused_and_never_reaches_the_game() {
    // `mgr reset` undeploys everything. Without --allow-write the server must refuse it before a
    // process is spawned, and say what would unlock it.
    let tmp = TempDir::new().unwrap();
    let mut server = Server::spawn(tmp.path());
    server.initialize();

    server.send(json!({
        "jsonrpc": "2.0",
        "id": 14,
        "method": "tools/call",
        "params": { "name": "gore_mgr", "arguments": { "subcommand": "reset" } },
    }));
    let result = server.recv()["result"].clone();

    assert_eq!(result["isError"], json!(true), "{result}");
    let message = result["content"][0]["text"].as_str().unwrap();
    assert!(message.starts_with("refused:"), "{message}");
    assert!(message.contains("--allow-write"), "{message}");
    // A refusal never runs anything, so there is no exit code to report.
    assert!(result.get("structuredContent").is_none(), "{result}");

    let (code, stderr) = server.shutdown();
    assert_eq!(code, Some(0), "stderr was: {stderr}");
}

#[test]
fn a_game_launching_command_is_refused_without_its_own_flag() {
    let tmp = TempDir::new().unwrap();
    let mut server = Server::spawn(tmp.path());
    server.initialize();

    server.send(json!({
        "jsonrpc": "2.0",
        "id": 15,
        "method": "tools/call",
        "params": { "name": "gore_as", "arguments": { "subcommand": "compile" } },
    }));
    let result = server.recv()["result"].clone();

    assert_eq!(result["isError"], json!(true), "{result}");
    let message = result["content"][0]["text"].as_str().unwrap();
    assert!(message.contains("--allow-game-launch"), "{message}");
    assert!(message.contains("launches the game"), "{message}");

    let (code, stderr) = server.shutdown();
    assert_eq!(code, Some(0), "stderr was: {stderr}");
}

#[test]
fn an_unknown_method_is_reported_without_ending_the_session() {
    let tmp = TempDir::new().unwrap();
    let mut server = Server::spawn(tmp.path());
    server.initialize();

    server.send(json!({ "jsonrpc": "2.0", "id": 4, "method": "does/not/exist" }));
    let response = server.recv();
    assert_eq!(response["error"]["code"], -32601);

    server.send(json!({ "jsonrpc": "2.0", "id": 5, "method": "ping" }));
    assert_eq!(server.recv()["id"], 5, "the session survives an unknown method");

    let (code, stderr) = server.shutdown();
    assert_eq!(code, Some(0), "stderr was: {stderr}");
}

#[test]
fn the_guide_is_served_as_resources_and_reads_back() {
    let tmp = TempDir::new().unwrap();
    let mut server = Server::spawn(tmp.path());
    server.initialize();

    server.send(json!({ "jsonrpc": "2.0", "id": 20, "method": "resources/list" }));
    let listed = server.recv()["result"]["resources"].as_array().unwrap().clone();
    assert!(listed.len() >= 21, "expected the whole guide, got {}", listed.len());

    let uri = listed[0]["uri"].as_str().unwrap().to_string();
    server.send(json!({
        "jsonrpc": "2.0",
        "id": 21,
        "method": "resources/read",
        "params": { "uri": uri },
    }));
    let contents = server.recv()["result"]["contents"].clone();
    assert_eq!(contents[0]["mimeType"], "text/markdown");
    assert!(!contents[0]["text"].as_str().unwrap().is_empty());

    let (code, stderr) = server.shutdown();
    assert_eq!(code, Some(0), "stderr was: {stderr}");
}

#[test]
fn the_guide_tool_finds_a_page_without_touching_the_filesystem() {
    // The guide is compiled into the binary, so this works from a TempDir with no docs/ anywhere
    // near it — which is the situation after a user unpacks the release zip somewhere else.
    let tmp = TempDir::new().unwrap();
    let mut server = Server::spawn(tmp.path());
    server.initialize();

    server.send(json!({
        "jsonrpc": "2.0",
        "id": 22,
        "method": "tools/call",
        "params": {
            "name": "gore_guide",
            "arguments": { "action": "search", "query": "replace a texture" },
        },
    }));
    let result = server.recv()["result"].clone();

    assert_eq!(result["isError"], json!(false), "{result}");
    let hits = result["structuredContent"]["hits"].as_array().unwrap();
    assert!(!hits.is_empty());
    assert_eq!(hits[0]["page"], "textures", "{hits:?}");

    let (code, stderr) = server.shutdown();
    assert_eq!(code, Some(0), "stderr was: {stderr}");
}

#[test]
fn the_help_tool_returns_the_cli_own_help() {
    let tmp = TempDir::new().unwrap();
    let mut server = Server::spawn(tmp.path());
    server.initialize();

    server.send(json!({
        "jsonrpc": "2.0",
        "id": 23,
        "method": "tools/call",
        "params": { "name": "gore_help", "arguments": { "command": "as patch-default" } },
    }));
    let result = server.recv()["result"].clone();

    assert_eq!(result["isError"], json!(false), "{result}");
    assert_eq!(result["content"][0]["text"], "gore as patch-default --help");
    let help = result["content"][1]["text"].as_str().unwrap();
    assert!(help.contains("--expected-hex"), "{help}");

    let (code, stderr) = server.shutdown();
    assert_eq!(code, Some(0), "stderr was: {stderr}");
}

#[test]
fn closing_stdin_exits_cleanly_without_writing_to_stderr() {
    // The client shuts a stdio server down by closing its stdin. If that surfaced as an error the
    // CLI would print `error: …` and exit non-zero, which clients report to users as a crash.
    let tmp = TempDir::new().unwrap();
    let mut server = Server::spawn(tmp.path());
    server.initialize();

    let (code, stderr) = server.shutdown();
    assert_eq!(code, Some(0));
    assert!(stderr.trim().is_empty(), "a clean shutdown must be silent, got: {stderr}");
}
