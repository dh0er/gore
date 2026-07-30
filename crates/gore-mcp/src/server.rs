//! The MCP session: it turns JSON-RPC frames into MCP semantics.
//!
//! The loop is strictly sequential — one request is served to completion before the next is read.
//! For this server that is a deliberate simplification rather than an oversight: every tool call
//! runs a child `gore` process, and running several of those concurrently against one game
//! installation is exactly the kind of thing the CLI's own install-mutation guard exists to
//! prevent. The cost is that a long command (`texture index`, `as emit-all`) blocks the session
//! until it finishes or hits its timeout; that is stated in the instructions primer so a client
//! knows what to expect.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use serde_json::{json, Map, Value};

use crate::exec::{self, ProcessSpawn, Spawn};
use crate::rpc::{errors, Frame, Request, Response, Transport, MAX_FRAME_BYTES};
use crate::{argv, capabilities, resources, spec, tools};

/// How the server was started. Everything the session needs that is not compile-time constant.
#[derive(Debug, Clone)]
pub struct Options {
    /// The `gore` binary to re-exec for tool calls. Resolved once by the caller so that a moved or
    /// renamed executable fails loudly at startup rather than silently on the first call.
    pub exe: PathBuf,
    /// Version of that binary, reported as `serverInfo.version`.
    pub server_version: String,
    /// Permit commands that modify the game installation or rewrite files in place.
    pub allow_write: bool,
    /// Permit commands that launch the game executable.
    pub allow_game_launch: bool,
    /// Wall-clock cap applied to every command, overriding the per-command defaults. `0` keeps them.
    pub timeout_override_secs: u64,
    /// Cap on captured stdout per command.
    pub max_stdout_bytes: usize,
}

/// 256 KiB: enough for any human-shaped listing, small enough that a runaway scan cannot flood a
/// model's context.
pub const DEFAULT_MAX_STDOUT_BYTES: usize = 256 * 1024;

impl Options {
    pub fn new(exe: PathBuf, server_version: impl Into<String>) -> Self {
        Self {
            exe,
            server_version: server_version.into(),
            allow_write: false,
            allow_game_launch: false,
            timeout_override_secs: 0,
            max_stdout_bytes: DEFAULT_MAX_STDOUT_BYTES,
        }
    }
}

pub struct Session {
    opts: Options,
    spawn: Box<dyn Spawn>,
    protocol_version: &'static str,
    initialized: bool,
}

impl Session {
    pub fn new(opts: Options) -> Self {
        let spawn = ProcessSpawn::new(opts.exe.clone(), opts.max_stdout_bytes);
        Self::with_spawn(opts, Box::new(spawn))
    }

    /// Build a session against a substitute process runner. Used by tests to exercise the whole
    /// `tools/call` path without spawning anything.
    pub fn with_spawn(opts: Options, spawn: Box<dyn Spawn>) -> Self {
        Self {
            opts,
            spawn,
            protocol_version: capabilities::LATEST_PROTOCOL_VERSION,
            initialized: false,
        }
    }

    /// The version agreed during `initialize`, or our latest before that has happened.
    pub fn protocol_version(&self) -> &'static str {
        self.protocol_version
    }

    /// Whether the client has sent `notifications/initialized`.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Handle one message. `None` means "write nothing back".
    ///
    /// Free of IO by construction, which makes it the primary unit-test seam: a test constructs a
    /// `Request`, calls this, and inspects the `Response` without a pipe or a child process in
    /// sight.
    pub fn handle(&mut self, request: &Request) -> Option<Response> {
        let id = request.response_id();
        let params = request.params_object();

        match request.method.as_str() {
            "initialize" => Some(Response::ok(id, self.initialize(&params))),

            "notifications/initialized" => {
                self.initialized = true;
                None
            }
            // Anything else in the notification namespace (`cancelled`, `progress`, a future
            // addition) is accepted and ignored. Notifications must never be answered, not even
            // with an error, so an unknown one cannot be reported as `METHOD_NOT_FOUND`.
            method if method.starts_with("notifications/") => None,

            "ping" => Some(Response::ok(id, json!({}))),

            "tools/list" => Some(Response::ok(id, json!({ "tools": self.tool_definitions() }))),
            "tools/call" => Some(self.call_tool(id, &params)),

            "resources/list" => Some(Response::ok(id, json!({ "resources": self.resources() }))),
            "resources/templates/list" => {
                Some(Response::ok(id, json!({ "resourceTemplates": self.resource_templates() })))
            }
            "resources/read" => Some(self.read_resource(id, &params)),

            other => Some(Response::error(
                id,
                errors::METHOD_NOT_FOUND,
                format!("unknown method: {other}"),
            )),
        }
    }

    fn initialize(&mut self, params: &Map<String, Value>) -> Value {
        let requested = params.get("protocolVersion").and_then(Value::as_str);
        self.protocol_version = capabilities::negotiate_protocol_version(requested);
        json!({
            "protocolVersion": self.protocol_version,
            "capabilities": capabilities::capabilities(),
            "serverInfo": capabilities::server_info(&self.opts.server_version),
            "instructions": capabilities::instructions(&self.opts),
        })
    }

    fn tool_definitions(&self) -> Vec<Value> {
        crate::tool_definitions()
    }

    fn resources(&self) -> Vec<Value> {
        resources::list()
    }

    fn resource_templates(&self) -> Vec<Value> {
        resources::templates()
    }

    /// Run one tool call.
    ///
    /// Note where the two kinds of failure go. An unknown tool is a JSON-RPC error, because no
    /// change of arguments fixes it. Everything downstream of that — an unknown subcommand, a
    /// missing argument, a refusal, a command that exits non-zero — comes back as a successful
    /// response carrying `isError: true`, because those are exactly the failures a model can read
    /// and correct.
    fn call_tool(&mut self, id: Value, params: &Map<String, Value>) -> Response {
        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return Response::error(id, errors::INVALID_PARAMS, "`name` is required");
        };
        if !tools::is_extra_tool(name) && spec::group(name).is_none() {
            return Response::error(id, errors::INVALID_PARAMS, format!("unknown tool: {name}"));
        }

        let arguments = match params.get("arguments") {
            None | Some(Value::Null) => Map::new(),
            Some(Value::Object(map)) => map.clone(),
            Some(_) => {
                return Response::ok(id, exec::to_error_result("`arguments` must be an object."))
            }
        };

        if name == tools::guide::NAME {
            return Response::ok(id, tools::guide::call(&arguments));
        }
        if name == tools::help::NAME {
            return match tools::help::call(&arguments, self.spawn.as_ref()) {
                Ok(result) => Response::ok(id, result),
                // Same rule as below: a process that cannot be started is an internal error, not
                // something the model can fix by calling differently.
                Err(message) => Response::error(id, errors::INTERNAL_ERROR, message),
            };
        }
        let group = spec::group(name).expect("checked above");

        for key in arguments.keys() {
            if key != "subcommand" && key != "args" {
                return Response::ok(
                    id,
                    exec::to_error_result(format!(
                        "`{key}` is not accepted here. Pass `subcommand` and put the command's own \
                         arguments inside `args`."
                    )),
                );
            }
        }

        let Some(subcommand) = arguments.get("subcommand").and_then(Value::as_str) else {
            return Response::ok(
                id,
                exec::to_error_result(format!(
                    "`subcommand` is required and must be a string. {name} accepts: {}.",
                    group.subcommands().join(", ")
                )),
            );
        };

        let args = arguments.get("args").cloned().unwrap_or(Value::Null);
        let invocation = match argv::build(group, subcommand, &args, &self.opts) {
            Ok(invocation) => invocation,
            Err(error) => return Response::ok(id, exec::to_error_result(error.to_string())),
        };
        let command = group.command(subcommand).expect("argv::build validated the subcommand");

        match self.spawn.run(&invocation) {
            Ok(outcome) => Response::ok(id, exec::to_call_result(&invocation, command, &outcome)),
            // Failing to start the process at all is our problem, not the model's: no change of
            // arguments makes a missing or unrunnable binary work.
            Err(error) => Response::error(
                id,
                errors::INTERNAL_ERROR,
                format!("could not run `{}`: {error}", invocation.display),
            ),
        }
    }

    fn read_resource(&self, id: Value, params: &Map<String, Value>) -> Response {
        let Some(uri) = params.get("uri").and_then(Value::as_str) else {
            return Response::error(id, errors::INVALID_PARAMS, "`uri` is required");
        };
        match resources::read(uri) {
            Some(contents) => Response::ok(id, contents),
            // An unresolvable URI is a protocol error, matching the treatment of an unknown tool:
            // the caller named something that does not exist, and no retry with different content
            // helps.
            None => Response::error(
                id,
                errors::INVALID_PARAMS,
                format!(
                    "unknown resource: {uri}. Guide pages are gore://guide/<page>, where <page> \
                     is one of: {}. Reference pages are gore://reference/<page>: {}.",
                    slugs_of(crate::guide::Kind::Guide),
                    slugs_of(crate::guide::Kind::Reference),
                ),
            ),
        }
    }
}

/// Comma-separated slugs of one documentation body, for an error that has to say what *is* valid.
fn slugs_of(kind: crate::guide::Kind) -> String {
    crate::guide::pages_of(kind).map(|page| page.slug).collect::<Vec<_>>().join(", ")
}

/// Run the server until the client closes our input.
///
/// A clean end of input is the ordinary stdio shutdown handshake — the client closes our stdin and
/// waits for us to exit — so it returns `Ok(())`. Returning an error there would make every normal
/// disconnect print `error:` and exit non-zero, which some clients report to the user as a crash.
pub fn serve<R: BufRead, W: Write>(opts: Options, reader: R, writer: W) -> io::Result<()> {
    let mut session = Session::new(opts);
    let mut transport = Transport::new(reader, writer);

    while let Some(frame) = transport.read_frame()? {
        match frame {
            // A batch answers with an array of exactly the replies its members earned. If every
            // member was a notification there is nothing to say, and JSON-RPC 2.0 requires silence
            // rather than an empty array.
            Frame::Batch(members) => {
                let replies: Vec<Response> =
                    members.into_iter().filter_map(|member| reply_to(&mut session, member)).collect();
                if !replies.is_empty() {
                    transport.write_message(&replies)?;
                }
            }
            single => {
                if let Some(response) = reply_to(&mut session, single) {
                    transport.write_message(&response)?;
                }
            }
        }
    }

    Ok(())
}

/// The reply one frame earns, or `None` when it earns silence.
fn reply_to(session: &mut Session, frame: Frame) -> Option<Response> {
    match frame {
        Frame::Message(request) => {
            let is_notification = request.is_notification();
            let response = session.handle(&request);
            // Belt and braces: whatever a handler returns, a message without an id gets no reply.
            // Answering one is a protocol violation that confuses strict clients.
            if is_notification {
                None
            } else {
                response
            }
        }
        Frame::Invalid { id, reason } => Some(Response::error(id, errors::INVALID_REQUEST, reason)),
        Frame::Malformed { reason } => Some(Response::parse_error(reason)),
        Frame::Oversized => Some(Response::parse_error(format!(
            "request exceeds the {MAX_FRAME_BYTES} byte frame limit"
        ))),
        // The transport never nests batches; a member that is itself an array is reported as an
        // invalid request by `parse_object`.
        Frame::Batch(_) => Some(Response::error(
            Value::Null,
            errors::INVALID_REQUEST,
            "a batch may not contain another batch",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn options() -> Options {
        Options::new(PathBuf::from("gore"), "0.1.0")
    }

    /// Drive `serve` over in-memory pipes and return one parsed response per written line.
    fn exchange(input: &str) -> Vec<Value> {
        let mut output = Vec::new();
        serve(options(), Cursor::new(input.as_bytes().to_vec()), &mut output).expect("serve");
        String::from_utf8(output)
            .expect("utf-8")
            .lines()
            .map(|line| serde_json::from_str(line).expect("each written line is one JSON value"))
            .collect()
    }

    fn request(method: &str, params: Value) -> Request {
        serde_json::from_value(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        }))
        .expect("request")
    }

    #[test]
    fn initialize_reports_a_version_capabilities_identity_and_instructions() {
        let mut session = Session::new(options());
        let response = session
            .handle(&request("initialize", json!({ "protocolVersion": "2025-11-25" })))
            .expect("initialize is answered");
        let result = response.result.expect("result");

        assert_eq!(result["protocolVersion"], "2025-11-25");
        assert_eq!(result["serverInfo"]["name"], "gore");
        assert_eq!(result["serverInfo"]["version"], "0.1.0");
        assert!(result["capabilities"].get("tools").is_some());
        assert!(
            result["instructions"].as_str().is_some_and(|text| !text.trim().is_empty()),
            "clients load `instructions` automatically; an empty one wastes the slot"
        );
    }

    #[test]
    fn an_unsupported_protocol_version_is_answered_with_ours() {
        let mut session = Session::new(options());
        let response = session
            .handle(&request("initialize", json!({ "protocolVersion": "1900-01-01" })))
            .expect("initialize is answered");
        assert_eq!(
            response.result.unwrap()["protocolVersion"],
            capabilities::LATEST_PROTOCOL_VERSION
        );
    }

    #[test]
    fn the_initialized_notification_is_recorded_and_never_answered() {
        let mut session = Session::new(options());
        assert!(!session.is_initialized());

        let notification: Request = serde_json::from_value(json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
        }))
        .unwrap();

        assert!(session.handle(&notification).is_none());
        assert!(session.is_initialized());
    }

    #[test]
    fn unknown_notifications_are_swallowed_rather_than_reported() {
        let mut session = Session::new(options());
        let notification: Request = serde_json::from_value(json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": { "requestId": 4 },
        }))
        .unwrap();
        assert!(session.handle(&notification).is_none());
    }

    #[test]
    fn ping_answers_with_an_empty_result() {
        let mut session = Session::new(options());
        let response = session.handle(&request("ping", json!({}))).expect("ping is answered");
        assert_eq!(response.result.unwrap(), json!({}));
    }

    #[test]
    fn an_unknown_method_is_a_protocol_error() {
        let mut session = Session::new(options());
        let response = session.handle(&request("prompts/list", json!({}))).expect("answered");
        assert_eq!(response.error.unwrap().code, errors::METHOD_NOT_FOUND);
    }

    /// A session whose child processes are scripted rather than spawned.
    fn faked(outcome: exec::Outcome) -> (Session, std::sync::Arc<exec::FakeSpawn>) {
        let spawn = std::sync::Arc::new(exec::FakeSpawn::new(outcome));
        let session = Session::with_spawn(options(), Box::new(std::sync::Arc::clone(&spawn)));
        (session, spawn)
    }

    #[test]
    fn tools_list_advertises_every_group_with_a_schema_and_annotations() {
        let mut session = Session::new(options());
        let response = session.handle(&request("tools/list", json!({}))).expect("answered");
        let listed = response.result.unwrap()["tools"].as_array().unwrap().clone();

        // One tool per command group, plus the tools that only exist inside the server.
        assert_eq!(listed.len(), spec::GROUPS.len() + tools::definitions().len());
        for tool in &listed {
            assert!(tool["name"].as_str().unwrap().starts_with("gore_"));
            assert_eq!(tool["inputSchema"]["type"], "object");
            assert!(tool["annotations"]["readOnlyHint"].is_boolean());
        }

        let names: Vec<&str> =
            listed.iter().map(|tool| tool["name"].as_str().unwrap()).collect();
        for group in spec::GROUPS {
            assert!(names.contains(&group.tool), "{} is not advertised", group.tool);
        }
        assert!(names.contains(&tools::guide::NAME));
    }

    #[test]
    fn the_guide_tool_is_reachable_through_tools_call() {
        let (mut session, spawn) = faked(exec::Outcome::success(""));
        let response = session
            .handle(&request(
                "tools/call",
                json!({
                    "name": "gore_guide",
                    "arguments": { "action": "search", "query": "replace a texture" },
                }),
            ))
            .expect("answered");

        let result = response.result.expect("a result");
        assert_eq!(result["isError"], json!(false));
        assert!(!result["structuredContent"]["hits"].as_array().unwrap().is_empty());
        assert!(spawn.calls().is_empty(), "the guide is embedded; nothing is spawned");
    }

    #[test]
    fn a_tool_call_builds_the_command_line_and_returns_what_the_command_printed() {
        let (mut session, spawn) = faked(exec::Outcome::success("C:/x/gore/config.json\n"));
        let response = session
            .handle(&request(
                "tools/call",
                json!({ "name": "gore_config", "arguments": { "subcommand": "path" } }),
            ))
            .expect("answered");

        let result = response.result.expect("a tool call is answered with a result");
        assert_eq!(result["isError"], json!(false));
        assert_eq!(result["content"][0]["text"], "gore config path");
        assert_eq!(result["content"][1]["text"], "C:/x/gore/config.json\n");

        assert_eq!(spawn.calls().len(), 1);
        let argv: Vec<String> = spawn.calls()[0]
            .argv
            .iter()
            .map(|token| token.to_string_lossy().into_owned())
            .collect();
        assert_eq!(argv, vec!["config", "path"]);
    }

    #[test]
    fn a_refused_command_is_a_tool_error_and_never_reaches_a_process() {
        let (mut session, spawn) = faked(exec::Outcome::success(""));
        let response = session
            .handle(&request(
                "tools/call",
                json!({ "name": "gore_project", "arguments": { "subcommand": "deploy-shared" } }),
            ))
            .expect("answered");

        let result = response.result.expect("a refusal is a result, not a protocol error");
        assert_eq!(result["isError"], json!(true));
        let message = result["content"][0]["text"].as_str().unwrap();
        assert!(message.contains("--allow-write"), "{message}");
        assert!(spawn.calls().is_empty(), "a refused command must not be spawned");
    }

    #[test]
    fn a_bad_argument_is_a_tool_error_the_model_can_act_on() {
        let (mut session, spawn) = faked(exec::Outcome::success(""));
        let response = session
            .handle(&request(
                "tools/call",
                json!({
                    "name": "gore_catalog",
                    "arguments": { "subcommand": "catalog", "args": { "dump": "d.txt" } },
                }),
            ))
            .expect("answered");

        let result = response.result.unwrap();
        assert_eq!(result["isError"], json!(true));
        let message = result["content"][0]["text"].as_str().unwrap();
        assert!(message.contains("`kind`"), "{message}");
        assert!(spawn.calls().is_empty());
    }

    #[test]
    fn an_unknown_subcommand_is_a_tool_error_listing_the_real_ones() {
        let (mut session, _) = faked(exec::Outcome::success(""));
        let response = session
            .handle(&request(
                "tools/call",
                json!({ "name": "gore_config", "arguments": { "subcommand": "reset" } }),
            ))
            .expect("answered");

        let message = response.result.unwrap()["content"][0]["text"].as_str().unwrap().to_string();
        assert!(message.contains("no subcommand `reset`"), "{message}");
        assert!(message.contains("detect"), "{message}");
    }

    #[test]
    fn a_missing_subcommand_says_which_ones_exist() {
        let (mut session, _) = faked(exec::Outcome::success(""));
        let response = session
            .handle(&request("tools/call", json!({ "name": "gore_config", "arguments": {} })))
            .expect("answered");

        let result = response.result.unwrap();
        assert_eq!(result["isError"], json!(true));
        assert!(result["content"][0]["text"].as_str().unwrap().contains("`subcommand` is required"));
    }

    #[test]
    fn stray_top_level_arguments_are_rejected_with_a_hint_about_args() {
        let (mut session, _) = faked(exec::Outcome::success(""));
        let response = session
            .handle(&request(
                "tools/call",
                json!({
                    "name": "gore_config",
                    "arguments": { "subcommand": "path", "key": "game-path" },
                }),
            ))
            .expect("answered");

        let message = response.result.unwrap()["content"][0]["text"].as_str().unwrap().to_string();
        assert!(message.contains("inside `args`"), "{message}");
    }

    #[test]
    fn a_command_that_exits_non_zero_is_a_tool_error_not_a_protocol_error() {
        let (mut session, _) = faked(exec::Outcome::failure(1, "error: game-path is not set\n"));
        let response = session
            .handle(&request(
                "tools/call",
                json!({
                    "name": "gore_config",
                    "arguments": { "subcommand": "get", "args": { "key": "game-path" } },
                }),
            ))
            .expect("answered");

        let result = response.result.expect("still a result");
        assert_eq!(result["isError"], json!(true));
        assert_eq!(result["structuredContent"]["exit_code"], 1);
    }

    #[test]
    fn a_null_id_is_a_request_not_a_notification() {
        // Only an omitted `id` denotes a notification. Treating `"id": null` as one would run the
        // call — spawning a child, with whatever it does — and then swallow the reply, leaving the
        // client to wait and very likely retry.
        let input = Cursor::new(
            "{\"jsonrpc\":\"2.0\",\"id\":null,\"method\":\"ping\"}
".as_bytes().to_vec(),
        );
        let mut output = Vec::new();
        serve(options(), input, &mut output).expect("clean shutdown");

        let text = String::from_utf8(output).expect("utf-8");
        assert!(!text.trim().is_empty(), "a null-id request must be answered");
        let reply: Value = serde_json::from_str(text.trim()).expect("json");
        assert!(reply["id"].is_null(), "the answer echoes the null id: {reply}");
        assert!(reply["result"].is_object());

        // An omitted id is still a notification, and still silent.
        let notification =
            Cursor::new("{\"jsonrpc\":\"2.0\",\"method\":\"ping\"}
".as_bytes().to_vec());
        let mut silent = Vec::new();
        serve(options(), notification, &mut silent).expect("clean shutdown");
        assert!(silent.is_empty(), "an omitted id means no reply");
    }

    #[test]
    fn a_batch_is_answered_with_one_array_of_replies() {
        // MCP revisions before 2025-06-18 permit JSON-RPC batches, and this server still
        // negotiates them, so a client may legitimately send an array in one frame.
        let input = Cursor::new(
            concat!(
                r#"[{"jsonrpc":"2.0","id":1,"method":"ping"},"#,
                r#"{"jsonrpc":"2.0","id":2,"method":"ping"}]"#,
                "
",
            )
            .as_bytes()
            .to_vec(),
        );
        let mut output = Vec::new();
        serve(options(), input, &mut output).expect("clean shutdown");

        let text = String::from_utf8(output).expect("utf-8");
        assert_eq!(text.lines().count(), 1, "a batch answers in one frame: {text}");
        let replies: Vec<Value> = serde_json::from_str(text.trim()).expect("an array");
        assert_eq!(replies.len(), 2);
        assert_eq!(replies[0]["id"], json!(1));
        assert_eq!(replies[1]["id"], json!(2));
    }

    #[test]
    fn a_batch_of_notifications_is_answered_with_silence() {
        let input = Cursor::new(
            "[{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}]
"
                .as_bytes()
                .to_vec(),
        );
        let mut output = Vec::new();
        serve(options(), input, &mut output).expect("clean shutdown");
        assert!(output.is_empty(), "an empty array reply is a protocol violation");
    }

    #[test]
    fn an_empty_batch_is_an_invalid_request() {
        let input = Cursor::new("[]
".as_bytes().to_vec());
        let mut output = Vec::new();
        serve(options(), input, &mut output).expect("clean shutdown");

        let reply: Value =
            serde_json::from_str(String::from_utf8(output).unwrap().trim()).expect("json");
        assert_eq!(reply["error"]["code"], json!(errors::INVALID_REQUEST));
    }

    #[test]
    fn a_bad_member_does_not_cost_the_rest_of_the_batch_its_answer() {
        let input = Cursor::new(
            concat!(
                r#"[{"jsonrpc":"2.0","id":1,"method":"ping"},"#,
                r#"{"nonsense":true},"#,
                r#"{"jsonrpc":"2.0","id":3,"method":"ping"}]"#,
                "
",
            )
            .as_bytes()
            .to_vec(),
        );
        let mut output = Vec::new();
        serve(options(), input, &mut output).expect("clean shutdown");

        let replies: Vec<Value> =
            serde_json::from_str(String::from_utf8(output).unwrap().trim()).expect("an array");
        assert_eq!(replies.len(), 3);
        assert!(replies[0]["result"].is_object());
        assert_eq!(replies[1]["error"]["code"], json!(errors::INVALID_REQUEST));
        assert!(replies[2]["result"].is_object());
    }

    #[test]
    fn resources_list_offers_every_guide_page_and_one_template() {
        let mut session = Session::new(options());

        let listed = session
            .handle(&request("resources/list", json!({})))
            .expect("answered")
            .result
            .unwrap()["resources"]
            .as_array()
            .unwrap()
            .len();
        assert_eq!(listed, crate::guide::PAGES.len());

        let templates = session
            .handle(&request("resources/templates/list", json!({})))
            .expect("answered")
            .result
            .unwrap()["resourceTemplates"]
            .as_array()
            .unwrap()
            .len();
        // One per namespace: gore://guide/{page} and gore://reference/{page}.
        assert_eq!(templates, 2);
    }

    #[test]
    fn a_guide_resource_reads_back_its_page() {
        let mut session = Session::new(options());
        let response = session
            .handle(&request("resources/read", json!({ "uri": "gore://guide/bundles" })))
            .expect("answered");

        let contents = response.result.expect("a result")["contents"].clone();
        assert_eq!(contents[0]["uri"], "gore://guide/bundles");
        assert_eq!(contents[0]["mimeType"], "text/markdown");
        assert!(contents[0]["text"].as_str().unwrap().contains("bundle"));
    }

    #[test]
    fn an_unknown_resource_is_a_protocol_error_that_lists_the_real_pages() {
        let mut session = Session::new(options());
        let response = session
            .handle(&request("resources/read", json!({ "uri": "gore://guide/nope" })))
            .expect("answered");

        let error = response.error.expect("unknown resources are protocol errors");
        assert_eq!(error.code, errors::INVALID_PARAMS);
        assert!(error.message.contains("bundles"), "{}", error.message);
    }

    #[test]
    fn an_unknown_tool_is_a_protocol_error_not_a_tool_error() {
        let mut session = Session::new(options());
        let response = session
            .handle(&request("tools/call", json!({ "name": "gore_nonexistent" })))
            .expect("answered");
        let error = response.error.expect("unknown tools are protocol errors");
        assert_eq!(error.code, errors::INVALID_PARAMS);
        assert!(error.message.contains("gore_nonexistent"));
    }

    #[test]
    fn notifications_produce_no_output_at_all_over_the_transport() {
        let written = exchange("{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n");
        assert!(written.is_empty(), "a notification must not be answered, got {written:?}");
    }

    #[test]
    fn a_full_handshake_round_trips_over_the_transport() {
        let written = exchange(concat!(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-11-25\"}}\n",
            "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n",
            "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\n",
        ));

        assert_eq!(written.len(), 2, "exactly the two requests are answered");
        assert_eq!(written[0]["id"], 1);
        assert_eq!(written[0]["result"]["protocolVersion"], "2025-11-25");
        assert_eq!(written[1]["id"], 2);
        assert!(written[1]["result"]["tools"].is_array());
    }

    #[test]
    fn malformed_input_is_a_parse_error_and_does_not_end_the_session() {
        let written = exchange(concat!(
            "not json\n",
            "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"ping\"}\n",
        ));

        assert_eq!(written[0]["error"]["code"], errors::PARSE_ERROR);
        assert_eq!(written[0]["id"], Value::Null);
        assert_eq!(written[1]["id"], 5, "the session survives a bad frame");
    }

    #[test]
    fn valid_json_that_is_not_a_request_is_an_invalid_request_with_its_id_echoed() {
        let written = exchange("{\"jsonrpc\":\"2.0\",\"id\":8}\n");
        assert_eq!(written[0]["error"]["code"], errors::INVALID_REQUEST);
        assert_eq!(written[0]["id"], 8);
    }

    #[test]
    fn a_closed_input_is_a_clean_shutdown() {
        // The stdio shutdown handshake is "client closes our stdin, then waits for us to exit".
        // This must not be an error, or every normal disconnect looks like a crash.
        let mut output = Vec::new();
        let result = serve(options(), Cursor::new(Vec::new()), &mut output);
        assert!(result.is_ok());
        assert!(output.is_empty());
    }
}
