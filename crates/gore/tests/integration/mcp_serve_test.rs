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
        self.initialize_with(json!({}))
    }

    /// Hand-shake declaring the `elicitation` capability, so the server may ask questions.
    fn initialize_able_to_answer(&mut self) -> Value {
        self.initialize_with(json!({ "elicitation": {} }))
    }

    fn initialize_with(&mut self, capabilities: Value) -> Value {
        self.send(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": capabilities,
                "clientInfo": { "name": "gore-integration-test", "version": "0" },
            },
        }));
        let response = self.recv();
        self.send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        response
    }
}

/// A `tools/call` that trips the safety gate but is harmless if it does run.
///
/// `loc import` without `out` rewrites its input in place, which is what earns the question. The
/// paths do not exist, so the command it would run fails on the missing input before touching
/// anything — the test can therefore answer "yes" for real rather than mocking the outcome.
fn a_gated_call(id: u32, tmp: &Path) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {
            "name": "gore_loc",
            "arguments": {
                "subcommand": "import",
                "args": {
                    "lcache": tmp.join("absent.lcache").to_string_lossy(),
                    "edits": tmp.join("absent.json").to_string_lossy(),
                },
            },
        },
    })
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
    // The line names the binary the server re-execs, which under test is the built `gore.exe`
    // by absolute path rather than a bare `gore` — that is the point of showing it.
    let shown = result["content"][0]["text"].as_str().expect("a command line");
    assert!(shown.ends_with(" config path"), "{shown}");
    assert!(shown.contains("gore"), "{shown}");
    let stdout = result["content"][1]["text"].as_str().unwrap();
    assert!(stdout.contains("config.json"), "{stdout}");
    // Everything the model needs is in `content`, and nothing is anywhere else — a client that
    // prefers `structuredContent` would otherwise be handed a byte count instead of this path.
    assert!(result.get("structuredContent").is_none(), "{result}");

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
    let rendered = result["content"].to_string();
    assert!(rendered.contains("exit code 1"), "{rendered}");
    assert!(rendered.contains("game-path is not set"), "{rendered}");

    let (code, stderr) = server.shutdown();
    assert_eq!(code, Some(0), "stderr was: {stderr}");
}

#[test]
fn a_client_that_can_be_asked_gets_a_question_and_the_command_runs_on_yes() {
    // The whole point of the change, end to end over a real pipe: no flag, no restart, and the
    // command still runs once a person agrees.
    let tmp = TempDir::new().unwrap();
    let mut server = Server::spawn(tmp.path());
    server.initialize_able_to_answer();

    server.send(a_gated_call(30, tmp.path()));

    // What arrives first is not the reply — it is the server asking.
    let question = server.recv();
    assert_eq!(question["method"], "elicitation/create", "{question}");
    assert_eq!(question["jsonrpc"], "2.0");
    let question_id = question["id"].clone();
    assert!(question_id.is_string(), "the server has to correlate our answer: {question}");
    let message = question["params"]["message"].as_str().expect("a message");
    assert!(message.contains("gore loc import"), "{message}");
    assert!(message.contains("overwrite its input in place"), "{message}");
    // Which file it would overwrite is the one thing this arm's reason cannot say — it is
    // identified by the argument that was left out — so the command line has to carry it.
    assert!(message.contains("absent.lcache"), "{message}");
    // Two named choices, so nothing can be agreed to by submitting an untouched default.
    let field = &question["params"]["requestedSchema"]["properties"]["decision"];
    assert_eq!(field["enum"], json!(["run", "cancel"]), "{question}");

    server.send(json!({
        "jsonrpc": "2.0",
        "id": question_id,
        "result": { "action": "accept", "content": { "decision": "run" } },
    }));

    let result = server.recv()["result"].clone();
    // The command ran and failed on its missing input, which is a completely different thing from
    // being refused: a refusal never reaches a process, so it shows no command line and no exit
    // code. Both are the discriminator here.
    let ran = result["content"][0]["text"].as_str().expect("a first block");
    assert!(ran.contains("loc import"), "a run leads with the command line: {result}");
    assert!(
        result["content"].to_string().contains("exit code 1"),
        "the command should have run and failed: {result}"
    );

    let (code, stderr) = server.shutdown();
    assert_eq!(code, Some(0), "stderr was: {stderr}");
}

#[test]
fn saying_no_leaves_the_command_unrun_and_the_session_healthy() {
    let tmp = TempDir::new().unwrap();
    let mut server = Server::spawn(tmp.path());
    server.initialize_able_to_answer();

    server.send(a_gated_call(31, tmp.path()));
    let question = server.recv();
    assert_eq!(question["method"], "elicitation/create");

    server.send(json!({
        "jsonrpc": "2.0",
        "id": question["id"].clone(),
        "result": { "action": "decline" },
    }));

    let response = server.recv();
    assert_eq!(response["id"], 31, "the answer settles the call it belonged to");
    let result = &response["result"];
    assert_eq!(result["isError"], json!(true), "{result}");
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.starts_with("refused:"), "nothing ran: {text}");
    assert!(!text.contains("exit code"), "nothing ran, so there is no exit code: {text}");
    // The answer is reported as what it was — an action on the wire — and not as a decision some
    // person is claimed to have made, which from this side of the socket is unknowable.
    assert!(text.contains("`decline`"), "{text}");
    assert!(!text.contains("the user was asked"), "{text}");

    // A declined call is not a broken session.
    server.send(json!({ "jsonrpc": "2.0", "id": 32, "method": "ping" }));
    assert_eq!(server.recv()["id"], 32);

    let (code, stderr) = server.shutdown();
    assert_eq!(code, Some(0), "stderr was: {stderr}");
}

#[test]
fn a_client_that_answers_for_the_user_is_not_reported_as_the_user_deciding() {
    // This is what Claude Code does when it is driven non-interactively: it advertises the
    // `elicitation` capability, then answers within milliseconds without showing anybody anything.
    // The refusal used to open "the user was asked about this call and said no" — a sentence that
    // was simply untrue, and that had a real reader conclude they had dismissed a dialog they never
    // saw. From here the two cases are indistinguishable, so the message must claim neither.
    let tmp = TempDir::new().unwrap();
    let mut server = Server::spawn(tmp.path());
    server.initialize_able_to_answer();

    server.send(a_gated_call(35, tmp.path()));
    let question = server.recv();
    assert_eq!(question["method"], "elicitation/create");

    server.send(json!({
        "jsonrpc": "2.0",
        "id": question["id"].clone(),
        "result": { "action": "cancel" },
    }));

    let result = server.recv()["result"].clone();
    let text = result["content"][0]["text"].as_str().unwrap();

    assert!(text.contains("`cancel`"), "the raw answer is named: {text}");
    assert!(text.contains("dismissed"), "{text}");
    for claim in ["the user was asked", "the user said", "said no", "the user declined"] {
        assert!(!text.contains(claim), "{claim:?} is not something this server can know: {text}");
    }
    // And it has to leave a way forward, because a dismissal may mean nobody was ever asked.
    assert!(text.contains("gore mcp serve --allow-write"), "{text}");

    let (code, stderr) = server.shutdown();
    assert_eq!(code, Some(0), "stderr was: {stderr}");
}

#[test]
fn a_relayed_approval_runs_the_command_without_putting_a_question() {
    // The way out of the case above. A client that answers its own dialogs leaves the model one
    // move: ask the user in the conversation, then send the call again carrying their words. No
    // dialog is put — the point is precisely that a dialog reaches nobody here — and the result
    // records that it ran on a claim.
    let tmp = TempDir::new().unwrap();
    let mut server = Server::spawn(tmp.path());
    server.initialize_able_to_answer();

    let mut call = a_gated_call(36, tmp.path());
    call["params"]["arguments"]["user_approved"] = json!("ja, überschreib die Datei");
    server.send(call);

    // The next frame is the reply, not a question: had the server asked, this would be an
    // `elicitation/create` and the id would not match.
    let response = server.recv();
    assert_eq!(response["id"], 36, "no question may precede this: {response}");
    let result = &response["result"];

    let ran = result["content"][0]["text"].as_str().expect("a first block");
    assert!(ran.contains("loc import"), "a run leads with the command line: {result}");
    assert!(
        result["content"].to_string().contains("exit code 1"),
        "the command should have run and failed on its missing input: {result}"
    );
    let recorded = result["content"].to_string();
    assert!(recorded.contains("assertion"), "the result must record the claim: {recorded}");
    assert!(recorded.contains("ja, überschreib die Datei"), "{recorded}");

    let (code, stderr) = server.shutdown();
    assert_eq!(code, Some(0), "stderr was: {stderr}");
}

#[test]
fn what_the_client_says_while_a_question_is_open_is_answered_afterwards() {
    // The server reads those frames off the wire to find its answer among them. Dropping one would
    // leave the client waiting forever on a request it did send.
    let tmp = TempDir::new().unwrap();
    let mut server = Server::spawn(tmp.path());
    server.initialize_able_to_answer();

    server.send(a_gated_call(33, tmp.path()));
    let question = server.recv();
    assert_eq!(question["method"], "elicitation/create");

    // Sent while the dialog is notionally open, i.e. before the answer.
    server.send(json!({ "jsonrpc": "2.0", "id": 34, "method": "ping" }));
    server.send(json!({
        "jsonrpc": "2.0",
        "id": question["id"].clone(),
        "result": { "action": "cancel" },
    }));

    // The tool call is settled first — it is what the server was in the middle of — and the ping it
    // set aside is answered right after.
    let first = server.recv();
    assert_eq!(first["id"], 33, "the call that was in flight: {first}");
    let second = server.recv();
    assert_eq!(second["id"], 34, "the deferred ping still gets its reply: {second}");

    let (code, stderr) = server.shutdown();
    assert_eq!(code, Some(0), "stderr was: {stderr}");
}

#[test]
fn a_destructive_command_is_refused_and_never_reaches_the_game() {
    // `mgr reset` undeploys everything. This client cannot be asked — it declared no `elicitation`
    // capability — so the server must refuse before a process is spawned, and say what would
    // let the user allow it.
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
    assert!(!message.contains("exit code"), "{message}");

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
    // Read out of the text, because that is the only channel every client passes to the model.
    let hits = result["content"][0]["text"].as_str().expect("a text block");
    assert!(hits.contains("read with:"), "{hits}");
    assert!(hits.contains("textures#"), "the top hit is the textures page: {hits}");
    assert!(result.get("structuredContent").is_none(), "{result}");

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
    let shown = result["content"][0]["text"].as_str().expect("a command line");
    assert!(shown.ends_with(" as patch-default --help"), "{shown}");
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
