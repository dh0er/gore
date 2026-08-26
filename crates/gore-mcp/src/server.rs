//! The MCP session: it turns JSON-RPC frames into MCP semantics.
//!
//! The loop is strictly sequential — one request is served to completion before the next is
//! handled. For this server that is a deliberate simplification rather than an oversight: every
//! tool call runs a child `gore` process, and running several of those concurrently against one
//! game installation is exactly the kind of thing the CLI's own install-mutation guard exists to
//! prevent. The cost is that a long command (`texture index`, `as emit-all`) blocks the session
//! until it finishes or hits its timeout; that is stated in the instructions primer so a client
//! knows what to expect.
//!
//! Sequential does not mean the stream stands still. A call that needs the user's agreement sends a
//! question and blocks on the answer, and anything the client says in the meantime is read off the
//! wire and set aside rather than handled — otherwise a second tool call could start underneath the
//! first. [`TransportPeer`] holds that queue; the loop drains it before reading further.

use std::collections::hash_map::DefaultHasher;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::hash::{Hash, Hasher};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};

use crate::consent::{self, Decision, Needs, Peer, Policy, APPROVAL_FIELD, APPROVAL_REQUEST_FIELD};
use crate::exec::{self, ProcessSpawn, Spawn};
use crate::rpc::{errors, Frame, OutRequest, Request, Response, Transport, MAX_FRAME_BYTES};
use crate::{argv, capabilities, resources, spec, tools};

/// How the server was started. Everything the session needs that is not compile-time constant.
#[derive(Debug, Clone)]
pub struct Options {
    /// The `gore` binary to re-exec for tool calls. Resolved once by the caller so that a moved or
    /// renamed executable fails loudly at startup rather than silently on the first call.
    pub exe: PathBuf,
    /// Version of that binary, reported as `serverInfo.version`.
    pub server_version: String,
    /// Treat commands that modify the game installation or rewrite files in place as already
    /// approved, so they run without asking. Off by default: they are not forbidden, they are
    /// confirmed with the user (see [`crate::consent`]). Turn it on where nobody is watching —
    /// CI, a batch run, an agent with its own approval layer.
    pub allow_write: bool,
    /// The same pre-approval for commands that launch the game executable.
    pub allow_game_launch: bool,
    /// Never put a question to the user; refuse anything that would need one.
    ///
    /// This is the strict posture, for a server exposed to something whose calls nobody reviews.
    /// It cannot be combined usefully with the two flags above, which say the opposite.
    pub never_ask: bool,
    /// Wall-clock cap applied to every command, overriding the per-command defaults. `0` keeps them.
    pub timeout_override_secs: u64,
    /// Cap on captured stdout per command.
    pub max_stdout_bytes: usize,
}

/// 256 KiB: enough for any human-shaped listing, small enough that a runaway scan cannot flood a
/// model's context.
pub const DEFAULT_MAX_STDOUT_BYTES: usize = 256 * 1024;
const APPROVAL_REQUEST_TTL: Duration = Duration::from_secs(15 * 60);
const MAX_PENDING_APPROVALS: usize = 32;

#[derive(Debug)]
struct PendingApproval {
    id: String,
    path: String,
    argv: Vec<OsString>,
    expires_at: Instant,
}

impl PendingApproval {
    fn matches(&self, invocation: &argv::Invocation) -> bool {
        self.path == invocation.path && self.argv == invocation.argv
    }
}

impl Options {
    pub fn new(exe: PathBuf, server_version: impl Into<String>) -> Self {
        Self {
            exe,
            server_version: server_version.into(),
            allow_write: false,
            allow_game_launch: false,
            never_ask: false,
            timeout_override_secs: 0,
            max_stdout_bytes: DEFAULT_MAX_STDOUT_BYTES,
        }
    }

    /// Whether the flags this server was started with already cover a call's requirements.
    ///
    /// Pre-approval, not permission: an uncovered call is not refused, it is put to the user.
    pub fn pre_approves(&self, needs: &Needs) -> bool {
        (!needs.write || self.allow_write) && (!needs.game_launch || self.allow_game_launch)
    }
}

pub struct Session {
    opts: Options,
    spawn: Box<dyn Spawn>,
    protocol_version: &'static str,
    initialized: bool,
    /// Whether the client declared the `elicitation` capability during `initialize`.
    ///
    /// Asking a client that never advertised it is a protocol violation, and in practice it also
    /// hangs: nothing on the other side is listening for the question, so nothing ever answers.
    client_can_elicit: bool,
    /// One-time conversation-approval requests. Each entry is bound to the normalized argv that
    /// was actually refused, not merely to a tool or subcommand name.
    pending_approvals: VecDeque<PendingApproval>,
    next_approval_id: u64,
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
            client_can_elicit: false,
            pending_approvals: VecDeque::new(),
            next_approval_id: 0,
        }
    }

    fn issue_approval_request(&mut self, invocation: &argv::Invocation) -> String {
        self.prune_expired_approvals();
        self.next_approval_id = self.next_approval_id.wrapping_add(1);

        // The id is deliberately opaque. It is not a secret — the refusal gives it to the caller —
        // but a hashed per-session value avoids exposing counters as a protocol clients start to
        // interpret. Authority comes from the server-side argv binding and one-time consumption.
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut first = DefaultHasher::new();
        std::process::id().hash(&mut first);
        now.hash(&mut first);
        self.next_approval_id.hash(&mut first);
        invocation.path.hash(&mut first);
        invocation.argv.hash(&mut first);
        let mut second = DefaultHasher::new();
        first.finish().hash(&mut second);
        invocation.display.hash(&mut second);
        now.rotate_left(41).hash(&mut second);
        let id = format!(
            "gore-consent-{:016x}{:016x}",
            first.finish(),
            second.finish()
        );

        if self.pending_approvals.len() == MAX_PENDING_APPROVALS {
            self.pending_approvals.pop_front();
        }
        self.pending_approvals.push_back(PendingApproval {
            id: id.clone(),
            path: invocation.path.clone(),
            argv: invocation.argv.clone(),
            expires_at: Instant::now() + APPROVAL_REQUEST_TTL,
        });
        id
    }

    fn consume_approval_request(
        &mut self,
        id: &str,
        invocation: &argv::Invocation,
    ) -> Result<(), String> {
        self.prune_expired_approvals();
        let Some(index) = self
            .pending_approvals
            .iter()
            .position(|pending| pending.id == id)
        else {
            return Err(format!(
                "`{APPROVAL_REQUEST_FIELD}` is unknown, expired, or already used. Send this exact \
                 call once without approval fields to obtain a fresh request id."
            ));
        };
        if !self.pending_approvals[index].matches(invocation) {
            return Err(format!(
                "`{APPROVAL_REQUEST_FIELD}` belongs to a different exact command. Do not change \
                 any argument on an approval retry; send this call once without approval fields \
                 to obtain its own request id."
            ));
        }
        self.pending_approvals.remove(index);
        Ok(())
    }

    fn prune_expired_approvals(&mut self) {
        let now = Instant::now();
        self.pending_approvals
            .retain(|pending| pending.expires_at > now);
    }

    /// How this session may treat a call that needs a person to agree.
    ///
    /// The server's own posture wins over the client's ability: someone who started with
    /// `--no-consent-prompts` asked not to be interrupted, and a capable client does not override
    /// that.
    pub fn consent_policy(&self) -> Policy {
        if self.opts.never_ask {
            Policy::NeverAsk
        } else if self.client_can_elicit {
            Policy::Ask
        } else {
            Policy::CannotAsk
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
    /// The only IO is through the two injected seams — `spawn` for child processes, `peer` for
    /// questions put to the user — so a test constructs a `Request`, calls this, and inspects the
    /// `Response` without a pipe or a child process in sight.
    pub fn handle(&mut self, request: &Request, peer: &mut dyn Peer) -> Option<Response> {
        let id = request.response_id();
        // Checked before anything dispatches, because `params_object` cannot tell "no fields" from
        // "fields we could not read": an array or a string flattens to the map an omitted member
        // yields, and the call would then answer success having silently discarded everything the
        // client sent. A notification is still never answered, malformed or not.
        if !request.params_shape_ok() {
            return (!request.is_notification()).then(|| {
                Response::error(
                    id,
                    errors::INVALID_PARAMS,
                    "`params` must be an object; MCP has no request whose parameters are anything else",
                )
            });
        }
        let params = request.params_object();

        match request.method.as_str() {
            // The handshake happens once, and only as a request. Without an id the reply is thrown
            // away, so negotiating from it would leave the two sides disagreeing about what was
            // agreed — including whether this client can be asked anything. A second one after
            // `notifications/initialized` could replace that agreement mid-session.
            "initialize" if request.is_notification() => None,
            // Still allowed before the handshake completes: that is how a client renegotiates a
            // protocol revision this server answered with something it cannot speak.
            "initialize" if self.initialized => Some(Response::error(
                id,
                errors::INVALID_REQUEST,
                "`initialize` may only be sent once, before `notifications/initialized`",
            )),
            "initialize" => Some(Response::ok(id, self.initialize(&params))),

            "notifications/initialized" => {
                // Only a real notification advances the handshake. The same method carrying an id
                // is a malformed request, and taking state from it would be trusting the mistake.
                if request.is_notification() {
                    self.initialized = true;
                }
                notification_reply(request, id)
            }
            // Anything else in the notification namespace (`cancelled`, `progress`, a future
            // addition) is accepted and ignored. Notifications must never be answered, not even
            // with an error, so an unknown one cannot be reported as `METHOD_NOT_FOUND`.
            method if method.starts_with("notifications/") => notification_reply(request, id),

            "ping" => Some(Response::ok(id, json!({}))),

            "tools/list" => Some(Response::ok(
                id,
                json!({ "tools": self.tool_definitions() }),
            )),
            // A tool call with no id is a malformed request, not a fire-and-forget notification.
            // JSON-RPC forbids answering it, so running it would spawn a child, change whatever it
            // changes, and throw the outcome away. There is nothing for a `notifications/cancelled`
            // to name either, so a call that stops to ask could not be withdrawn.
            "tools/call" if request.is_notification() => None,
            "tools/call" => Some(self.call_tool(id, &params, peer)),

            "resources/list" => Some(Response::ok(id, json!({ "resources": self.resources() }))),
            "resources/templates/list" => Some(Response::ok(
                id,
                json!({ "resourceTemplates": self.resource_templates() }),
            )),
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
        // Presence is the whole signal. The capability's value is an options object reserved for
        // future use, and an empty one — which is what every client sends today — means supported.
        // `null` and `false` are the two ways a client spells the opposite, and reading either as a
        // declaration is not a harmless mistake: a question put to a client that cannot answer it
        // does not come back refused, it waits for the round trip to fail.
        self.client_can_elicit = params
            .get("capabilities")
            .and_then(|caps| caps.get("elicitation"))
            .is_some_and(|value| !matches!(value, Value::Null | Value::Bool(false)));
        json!({
            "protocolVersion": self.protocol_version,
            "capabilities": capabilities::capabilities(),
            "serverInfo": capabilities::server_info(&self.opts.server_version),
            // Built after `client_can_elicit` is set above, so the primer describes what will
            // actually happen on this connection rather than what usually happens.
            "instructions": capabilities::instructions(&self.opts, self.consent_policy()),
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
    fn call_tool(
        &mut self,
        id: Value,
        params: &Map<String, Value>,
        peer: &mut dyn Peer,
    ) -> Response {
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

        let (subcommand, args) = match normalize_group_arguments(group, &arguments) {
            Ok(call) => call,
            Err(message) => return Response::ok(id, exec::to_error_result(message)),
        };

        // A claim quoting nobody is not a claim. A missing or null field is simply absent — some
        // clients serialise an omitted optional that way — but anything else present and unusable
        // is reported, because silently ignoring it would run the call as though nothing had been
        // said about consent at all.
        let approval = match arguments.get(APPROVAL_FIELD) {
            None | Some(Value::Null) => None,
            Some(Value::String(words)) if !words.trim().is_empty() => Some(words.clone()),
            Some(_) => {
                return Response::ok(
                    id,
                    exec::to_error_result(format!(
                        "`{APPROVAL_FIELD}` must be the user's own words approving this call, as a \
                         non-empty string. Leave it and `{APPROVAL_REQUEST_FIELD}` out unless they \
                         answered you."
                    )),
                )
            }
        };
        let approval_request_id = match arguments.get(APPROVAL_REQUEST_FIELD) {
            None | Some(Value::Null) => None,
            Some(Value::String(id)) if !id.trim().is_empty() => Some(id.clone()),
            Some(_) => {
                return Response::ok(
                    id,
                    exec::to_error_result(format!(
                        "`{APPROVAL_REQUEST_FIELD}` must be the non-empty opaque id from the \
                         refusal for this exact call."
                    )),
                )
            }
        };
        if approval.is_some() != approval_request_id.is_some() {
            return Response::ok(
                id,
                exec::to_error_result(format!(
                    "`{APPROVAL_REQUEST_FIELD}` and `{APPROVAL_FIELD}` must be supplied together. \
                     First send the protected call without either field; if it is refused, relay \
                     the user's answer with both values exactly as instructed."
                )),
            );
        }

        let invocation = match argv::build(group, &subcommand, &args, &self.opts) {
            Ok(invocation) => invocation,
            Err(error) => return Response::ok(id, exec::to_error_result(error.to_string())),
        };
        let command = group
            .command(&subcommand)
            .expect("argv::build validated the subcommand");

        // Between a complete command line and running it: the one question a person gets to answer.
        // Nothing has been spawned yet, so a "no" here leaves every file exactly as it was.
        let mut asserted = None;
        if let Some(request) = &invocation.consent {
            if self.consent_policy() != Policy::NeverAsk {
                if let (Some(request_id), Some(_)) = (&approval_request_id, &approval) {
                    if let Err(message) = self.consume_approval_request(request_id, &invocation) {
                        return Response::ok(id, exec::to_error_result(message));
                    }
                }
            }
            let decision =
                consent::decide(request, self.consent_policy(), approval.as_deref(), peer);
            if !decision.allows() {
                let approval_request_id = (self.consent_policy() != Policy::NeverAsk)
                    .then(|| self.issue_approval_request(&invocation));
                return Response::ok(
                    id,
                    exec::to_error_result(consent::refusal(
                        request,
                        &decision,
                        approval_request_id.as_deref(),
                    )),
                );
            }
            if let Decision::AllowedByAssertion(words) = decision {
                asserted = Some(words);
            }
        } else if approval.is_some() {
            return Response::ok(
                id,
                exec::to_error_result(format!(
                    "`{APPROVAL_REQUEST_FIELD}` and `{APPROVAL_FIELD}` were supplied, but this \
                     exact call does not need consent. Remove both fields and run it normally."
                )),
            );
        }

        match self.spawn.run(&invocation) {
            Ok(outcome) => {
                let mut result = exec::to_call_result(&invocation, command, &outcome);
                // Appended rather than prepended: the command line stays the first thing read, and
                // no existing reader has to move. It belongs in the result either way — a run that
                // nobody here confirmed should say so where the run itself is recorded.
                if let Some(words) = &asserted {
                    exec::append_note(&mut result, consent::assertion_note(words));
                }
                Response::ok(id, result)
            }
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

/// Normalize the two MCP call shapes without changing the CLI contract.
///
/// Namespace tools keep `subcommand` plus `args`. A tool that selects exactly one command accepts
/// its typed arguments directly, while retaining the old envelope for cached clients. Mixing the
/// two argument locations is refused rather than guessing which value wins.
fn normalize_group_arguments(
    group: &'static spec::GroupSpec,
    arguments: &Map<String, Value>,
) -> Result<(String, Value), String> {
    let is_meta = |key: &str| {
        matches!(
            key,
            "subcommand" | "args" | APPROVAL_FIELD | APPROVAL_REQUEST_FIELD
        )
    };

    if let [command] = group.commands {
        for key in arguments.keys() {
            if !is_meta(key) && command.arg(key).is_none() {
                let accepted = command
                    .args
                    .iter()
                    .map(|arg| arg.name)
                    .collect::<Vec<_>>()
                    .join(", ");
                let accepted = if accepted.is_empty() {
                    "no command arguments".to_owned()
                } else {
                    format!("these command arguments: {accepted}")
                };
                return Err(format!(
                    "`{key}` is not accepted by {}. Pass {accepted} directly at the top level; \
                     the old `subcommand`/`args` envelope is accepted only for compatibility.",
                    group.tool
                ));
            }
        }

        if let Some(subcommand) = arguments.get("subcommand") {
            match subcommand.as_str() {
                Some(given) if given == command.sub => {}
                Some(given) => {
                    return Err(format!(
                        "{} already selects `{}` and has no subcommand `{given}`.",
                        group.tool, command.sub
                    ));
                }
                None => return Err("`subcommand` must be a string when supplied.".into()),
            }
        }

        let mut direct = Map::new();
        for (key, value) in arguments {
            if command.arg(key).is_some() {
                direct.insert(key.clone(), value.clone());
            }
        }
        if arguments.contains_key("args") && !direct.is_empty() {
            return Err(format!(
                "{} received command arguments both directly and inside `args`. Use the direct \
                 typed fields only, or the old envelope only; nothing ran.",
                group.tool
            ));
        }
        let args = arguments
            .get("args")
            .cloned()
            .unwrap_or_else(|| Value::Object(direct));
        return Ok((command.sub.to_owned(), args));
    }

    for key in arguments.keys() {
        if !is_meta(key) {
            return Err(format!(
                "`{key}` is not accepted here. Pass `subcommand`, put the command's own arguments \
                 inside `args`, and use `{APPROVAL_REQUEST_FIELD}` plus `{APPROVAL_FIELD}` only to \
                 relay approval for a previously refused exact call."
            ));
        }
    }
    let Some(subcommand) = arguments.get("subcommand").and_then(Value::as_str) else {
        return Err(format!(
            "`subcommand` is required and must be a string. {} accepts: {}.",
            group.tool,
            group.subcommands().join(", ")
        ));
    };
    Ok((
        subcommand.to_owned(),
        arguments.get("args").cloned().unwrap_or(Value::Null),
    ))
}

/// Comma-separated slugs of one documentation body, for an error that has to say what *is* valid.
fn slugs_of(kind: crate::guide::Kind) -> String {
    crate::guide::pages_of(kind)
        .map(|page| page.slug)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Run the server until the client closes our input.
///
/// A clean end of input is the ordinary stdio shutdown handshake — the client closes our stdin and
/// waits for us to exit — so it returns `Ok(())`. Returning an error there would make every normal
/// disconnect print `error:` and exit non-zero, which some clients report to the user as a crash.
pub fn serve<R: BufRead, W: Write>(opts: Options, reader: R, writer: W) -> io::Result<()> {
    let mut session = Session::new(opts);
    let mut transport = Transport::new(reader, writer);
    // Frames read out of turn while a question was open. Drained before the stream, so the client
    // is still served in the order it spoke.
    let mut deferred: VecDeque<Frame> = VecDeque::new();
    let mut questions_asked: u64 = 0;

    loop {
        let frame = match deferred.pop_front() {
            Some(frame) => frame,
            None => match transport.read_frame()? {
                Some(frame) => frame,
                None => return Ok(()),
            },
        };

        match frame {
            // A batch answers with an array of exactly the replies its members earned. If every
            // member was a notification there is nothing to say, and JSON-RPC 2.0 requires silence
            // rather than an empty array.
            Frame::Batch(members) => {
                let mut replies: Vec<Response> = Vec::new();
                for member in members {
                    let mut peer = TransportPeer {
                        call_id: correlation_id(&member),
                        transport: &mut transport,
                        deferred: &mut deferred,
                        asked: &mut questions_asked,
                    };
                    if let Some(reply) = reply_to(&mut session, member, &mut peer) {
                        replies.push(reply);
                    }
                }
                if !replies.is_empty() {
                    transport.write_message(&replies)?;
                }
            }
            single => {
                let response = {
                    let mut peer = TransportPeer {
                        call_id: correlation_id(&single),
                        transport: &mut transport,
                        deferred: &mut deferred,
                        asked: &mut questions_asked,
                    };
                    reply_to(&mut session, single, &mut peer)
                };
                if let Some(response) = response {
                    transport.write_message(&response)?;
                }
            }
        }
    }
}

/// The request id a frame carries, for matching a cancellation against it.
fn correlation_id(frame: &Frame) -> Value {
    match frame {
        Frame::Message(request) => request.response_id(),
        _ => Value::Null,
    }
}

/// Upper bound on frames set aside while one question is open.
///
/// A client has no reason to send hundreds of messages while it is showing a dialog, so reaching
/// this means something is wrong on the other side. Abandoning the question is the bounded
/// response: the deferred frames are still answered, and the tool call refuses rather than running.
const MAX_DEFERRED_FRAMES: usize = 256;

/// The live connection, seen as somewhere to put a question.
///
/// Reading from the same stream the main loop reads from is what makes this delicate. While an
/// answer is outstanding, whatever else the client sends is set aside rather than handled, because
/// handling it could start a second tool call underneath the first — one game installation, two
/// writers. The loop drains what accumulated as soon as the question is settled.
struct TransportPeer<'a, R: BufRead, W: Write> {
    /// The `tools/call` this question belongs to. A cancellation naming it ends the wait.
    call_id: Value,
    transport: &'a mut Transport<R, W>,
    deferred: &'a mut VecDeque<Frame>,
    asked: &'a mut u64,
}

impl<R: BufRead, W: Write> Peer for TransportPeer<'_, R, W> {
    fn request(&mut self, method: &'static str, params: Value) -> Result<Value, String> {
        *self.asked += 1;
        // Namespaced and counted. JSON-RPC keeps the two directions in separate id spaces, so a
        // collision with a client's own id is harmless in principle — but a client that correlates
        // by id alone would mismatch, and this costs nothing to rule out.
        let id = Value::from(format!("gore-consent-{}", self.asked));

        self.transport
            .write_message(&OutRequest::new(id.clone(), method, params))
            .map_err(|error| format!("the question could not be sent: {error}"))?;

        loop {
            let frame = match self.transport.read_frame() {
                Ok(Some(frame)) => frame,
                Ok(None) => {
                    return Err("the client closed the connection with the question open".into())
                }
                Err(error) => return Err(format!("the connection failed while waiting: {error}")),
            };

            if let Frame::Answer {
                id: answered,
                result,
                error,
                version_ok,
            } = &frame
            {
                if *answered == id {
                    // Failed closed, before the payload is looked at. A peer that answers a "may I
                    // change your game installation?" question in a protocol revision it did not
                    // negotiate is not one whose "run it" this server should act on, and treating
                    // the failure as a refusal costs nothing but a question asked again.
                    if !version_ok {
                        return Err(
                            "the answer declared a JSON-RPC version this server does not \
                                    speak, so it was not read as consent"
                                .into(),
                        );
                    }
                    if let Some(error) = error {
                        return Err(describe_error(error));
                    }
                    return result
                        .clone()
                        .ok_or_else(|| "the answer carried neither a result nor an error".into());
                }
            }

            if cancels(&frame, &self.call_id) {
                // A batch is put back rather than dropped. The cancellation itself earns no reply,
                // but its fellow members may be requests, and an unanswered request leaves the
                // client waiting on something that is never coming.
                if matches!(&frame, Frame::Batch(_)) {
                    self.deferred.push_back(frame);
                }
                return Err("the client cancelled the call with the question open".into());
            }

            if self.deferred.len() >= MAX_DEFERRED_FRAMES {
                return Err(format!(
                    "the client sent more than {MAX_DEFERRED_FRAMES} messages without answering"
                ));
            }
            self.deferred.push_back(frame);
        }
    }
}

/// Whether a frame is the client withdrawing the request we are asking about.
fn cancels(frame: &Frame, call_id: &Value) -> bool {
    match frame {
        Frame::Message(request) => {
            // A null id is matched like any other. `"id": null` is a request rather than a
            // notification — presence is what decides that — so a gated `tools/call` carrying one
            // does open a consent question, and refusing to match its cancellation left the only
            // way out of that wait an answer or EOF. There is no ambiguity to protect against:
            // this loop serves one request at a time, so the call being asked about is the only
            // call a cancellation could be naming.
            request.method == "notifications/cancelled"
                && request
                    .params_object()
                    .get("requestId")
                    .is_some_and(|requested| requested == call_id)
        }
        // Searched member by member, because a client entitled to send batches at all may put the
        // cancellation inside one — and missing it there is the wedge this check exists to prevent:
        // the batch is set aside unread while the question goes on waiting for an answer that has
        // already been withdrawn. The transport never nests batches, so this recurses one level.
        Frame::Batch(members) => members.iter().any(|member| cancels(member, call_id)),
        _ => false,
    }
}

/// A JSON-RPC error object, as one sentence.
fn describe_error(error: &Value) -> String {
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("the client rejected the question");
    match error.get("code").and_then(Value::as_i64) {
        Some(code) => format!("{message} (code {code})"),
        None => message.to_string(),
    }
}

/// Silence for a notification — but only when it really was one.
///
/// A `notifications/*` method sent *with* an id is a request by JSON-RPC's definition, and an
/// unanswered request leaves the caller waiting on something that will never come, or retrying it.
/// The method exists, so `METHOD_NOT_FOUND` would be misleading; what is wrong is the id.
fn notification_reply(request: &Request, id: Value) -> Option<Response> {
    if request.is_notification() {
        return None;
    }
    Some(Response::error(
        id,
        errors::INVALID_REQUEST,
        format!(
            "`{}` is a notification and must not carry an `id`",
            request.method
        ),
    ))
}

/// The reply one frame earns, or `None` when it earns silence.
fn reply_to(session: &mut Session, frame: Frame, peer: &mut dyn Peer) -> Option<Response> {
    match frame {
        Frame::Message(request) => {
            let is_notification = request.is_notification();
            let response = session.handle(&request, peer);
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
        // An answer to a question nobody is waiting for. JSON-RPC has no reply to a reply, so the
        // only correct handling is to drop it — this is where a late answer lands after its call
        // was cancelled, and answering it would put an uncorrelatable frame on the wire.
        Frame::Answer { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn options() -> Options {
        Options::new(PathBuf::from("gore"), "0.1.0")
    }

    /// A peer that must never be reached.
    ///
    /// Most of what this server does needs nobody's permission, and a test that unexpectedly asks
    /// for some is a test whose call was classified wrongly. Panicking names the culprit; a stub
    /// that quietly said yes would let that slip through as a passing test.
    struct NoOneToAsk;

    impl Peer for NoOneToAsk {
        fn request(&mut self, method: &'static str, params: Value) -> Result<Value, String> {
            panic!("this call should not have asked anyone: {method} {params}");
        }
    }

    /// A peer that answers every question the same way, and counts them.
    struct Canned {
        answer: Value,
        asked: usize,
    }

    impl Canned {
        fn allowing() -> Self {
            Self {
                answer: json!({ "action": "accept", "content": { "decision": "run" } }),
                asked: 0,
            }
        }

        fn declining() -> Self {
            Self {
                answer: json!({ "action": "decline" }),
                asked: 0,
            }
        }
    }

    impl Peer for Canned {
        fn request(&mut self, _method: &'static str, _params: Value) -> Result<Value, String> {
            self.asked += 1;
            Ok(self.answer.clone())
        }
    }

    /// `handle` for the tests that are not about consent.
    trait HandleUnasked {
        fn handle_unasked(&mut self, request: &Request) -> Option<Response>;
    }

    impl HandleUnasked for Session {
        fn handle_unasked(&mut self, request: &Request) -> Option<Response> {
            self.handle(request, &mut NoOneToAsk)
        }
    }

    /// Drive `serve` over in-memory pipes and return one parsed response per written line.
    fn exchange(input: &str) -> Vec<Value> {
        let mut output = Vec::new();
        serve(
            options(),
            Cursor::new(input.as_bytes().to_vec()),
            &mut output,
        )
        .expect("serve");
        String::from_utf8(output)
            .expect("utf-8")
            .lines()
            .map(|line| serde_json::from_str(line).expect("each written line is one JSON value"))
            .collect()
    }

    fn request(method: &str, params: Value) -> Request {
        request_with_id(method, params, json!(1))
    }

    /// `id_present` is `#[serde(skip)]` — the transport sets it from the raw object, because
    /// `Option<Value>` cannot tell an omitted `id` from a null one. A helper that deserializes
    /// directly has to set it too, or everything it builds looks like a notification.
    fn request_with_id(method: &str, params: Value, id: Value) -> Request {
        let mut request: Request = serde_json::from_value(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }))
        .expect("request");
        request.id_present = true;
        request
    }

    fn notification(method: &str, params: Value) -> Request {
        serde_json::from_value(json!({ "jsonrpc": "2.0", "method": method, "params": params }))
            .expect("notification")
    }

    #[test]
    fn initialize_reports_a_version_capabilities_identity_and_instructions() {
        let mut session = Session::new(options());
        let response = session
            .handle_unasked(&request(
                "initialize",
                json!({ "protocolVersion": "2025-11-25" }),
            ))
            .expect("initialize is answered");
        let result = response.result.expect("result");

        assert_eq!(result["protocolVersion"], "2025-11-25");
        assert_eq!(result["serverInfo"]["name"], "gore");
        assert_eq!(result["serverInfo"]["version"], "0.1.0");
        assert!(result["capabilities"].get("tools").is_some());
        assert!(
            result["instructions"]
                .as_str()
                .is_some_and(|text| !text.trim().is_empty()),
            "clients load `instructions` automatically; an empty one wastes the slot"
        );
    }

    #[test]
    fn an_unsupported_protocol_version_is_answered_with_ours() {
        let mut session = Session::new(options());
        let response = session
            .handle_unasked(&request(
                "initialize",
                json!({ "protocolVersion": "1900-01-01" }),
            ))
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

        assert!(session.handle_unasked(&notification).is_none());
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
        assert!(session.handle_unasked(&notification).is_none());
    }

    #[test]
    fn ping_answers_with_an_empty_result() {
        let mut session = Session::new(options());
        let response = session
            .handle_unasked(&request("ping", json!({})))
            .expect("ping is answered");
        assert_eq!(response.result.unwrap(), json!({}));
    }

    #[test]
    fn an_unknown_method_is_a_protocol_error() {
        let mut session = Session::new(options());
        let response = session
            .handle_unasked(&request("prompts/list", json!({})))
            .expect("answered");
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
        let response = session
            .handle_unasked(&request("tools/list", json!({})))
            .expect("answered");
        let listed = response.result.unwrap()["tools"]
            .as_array()
            .unwrap()
            .clone();

        // One tool per command group, plus the tools that only exist inside the server.
        assert_eq!(
            listed.len(),
            spec::GROUPS.len() + tools::definitions().len()
        );
        for tool in &listed {
            assert!(tool["name"].as_str().unwrap().starts_with("gore_"));
            assert_eq!(tool["inputSchema"]["type"], "object");
            assert!(tool["annotations"]["readOnlyHint"].is_boolean());
        }

        let names: Vec<&str> = listed
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        for group in spec::GROUPS {
            assert!(
                names.contains(&group.tool),
                "{} is not advertised",
                group.tool
            );
        }
        assert!(names.contains(&tools::guide::NAME));
    }

    #[test]
    fn the_guide_tool_is_reachable_through_tools_call() {
        let (mut session, spawn) = faked(exec::Outcome::success(""));
        let response = session
            .handle_unasked(&request(
                "tools/call",
                json!({
                    "name": "gore_guide",
                    "arguments": { "action": "search", "query": "replace a texture" },
                }),
            ))
            .expect("answered");

        let result = response.result.expect("a result");
        assert_eq!(result["isError"], json!(false));
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("textures"));
        assert!(
            spawn.calls().is_empty(),
            "the guide is embedded; nothing is spawned"
        );
    }

    #[test]
    fn a_tool_call_builds_the_command_line_and_returns_what_the_command_printed() {
        let (mut session, spawn) = faked(exec::Outcome::success("C:/x/gore/config.json\n"));
        let response = session
            .handle_unasked(&request(
                "tools/call",
                json!({ "name": "gore_config", "arguments": { "subcommand": "path" } }),
            ))
            .expect("answered");

        let result = response
            .result
            .expect("a tool call is answered with a result");
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
    fn single_command_tools_accept_typed_arguments_directly() {
        let (mut session, spawn) = faked(exec::Outcome::success("{}\n"));

        let doctor = session
            .handle_unasked(&request(
                "tools/call",
                json!({ "name": "gore_doctor", "arguments": {} }),
            ))
            .expect("answered")
            .result
            .expect("result");
        assert_eq!(doctor["isError"], json!(false));

        let find = session
            .handle_unasked(&request(
                "tools/call",
                json!({
                    "name": "gore_find",
                    "arguments": { "query": ["diego", "dialog"], "max": 3 },
                }),
            ))
            .expect("answered")
            .result
            .expect("result");
        assert_eq!(find["isError"], json!(false));

        let calls = spawn.calls();
        let doctor_argv: Vec<String> = calls[0]
            .argv
            .iter()
            .map(|token| token.to_string_lossy().into_owned())
            .collect();
        assert_eq!(doctor_argv, vec!["doctor", "--json"]);
        let find_argv: Vec<String> = calls[1]
            .argv
            .iter()
            .map(|token| token.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            find_argv,
            vec!["find", "--max", "3", "--json", "--", "diego", "dialog"]
        );
    }

    #[test]
    fn old_single_command_envelopes_remain_accepted_but_mixing_shapes_is_refused() {
        let (mut session, spawn) = faked(exec::Outcome::success("{}\n"));
        let legacy = session
            .handle_unasked(&request(
                "tools/call",
                json!({
                    "name": "gore_find",
                    "arguments": {
                        "subcommand": "find",
                        "args": { "query": ["diego"] },
                    },
                }),
            ))
            .expect("answered")
            .result
            .expect("result");
        assert_eq!(legacy["isError"], json!(false));

        let mixed = session
            .handle_unasked(&request(
                "tools/call",
                json!({
                    "name": "gore_find",
                    "arguments": {
                        "query": ["diego"],
                        "args": { "query": ["viper"] },
                    },
                }),
            ))
            .expect("answered")
            .result
            .expect("result");
        assert_eq!(mixed["isError"], json!(true));
        assert!(mixed["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("both directly and inside `args`"));
        assert_eq!(spawn.calls().len(), 1, "mixed call must not spawn");
    }

    #[test]
    fn dedicated_manager_preflight_is_direct_and_read_only() {
        let (mut session, spawn) = faked(exec::Outcome::success("{}\n"));
        let result = session
            .handle_unasked(&request(
                "tools/call",
                json!({
                    "name": "gore_mgr_preflight",
                    "arguments": { "game": "C:/Game" },
                }),
            ))
            .expect("answered")
            .result
            .expect("result");
        assert_eq!(result["isError"], json!(false));
        assert_eq!(spawn.calls().len(), 1);
        assert!(spawn.calls()[0].consent.is_none());
        let argv: Vec<String> = spawn.calls()[0]
            .argv
            .iter()
            .map(|token| token.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            argv,
            vec!["mgr", "preflight", "--game", "C:/Game", "--json"]
        );
    }

    #[test]
    fn dedicated_bundle_inspection_is_direct_read_only_and_forces_json() {
        let (mut session, spawn) = faked(exec::Outcome::success("{}\n"));
        let result = session
            .handle_unasked(&request(
                "tools/call",
                json!({
                    "name": "gore_mod_inspect",
                    "arguments": { "bundle": "C:/Mods/Diego.zip" },
                }),
            ))
            .expect("answered")
            .result
            .expect("result");
        assert_eq!(result["isError"], json!(false));
        assert_eq!(spawn.calls().len(), 1);
        assert!(spawn.calls()[0].consent.is_none());
        let argv: Vec<String> = spawn.calls()[0]
            .argv
            .iter()
            .map(|token| token.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            argv,
            vec!["mod", "inspect", "--json", "--", "C:/Mods/Diego.zip"]
        );
    }

    #[test]
    fn a_refused_command_is_a_tool_error_and_never_reaches_a_process() {
        let (mut session, spawn) = faked(exec::Outcome::success(""));
        let response = session
            .handle_unasked(&request(
                "tools/call",
                json!({ "name": "gore_project", "arguments": { "subcommand": "deploy-shared" } }),
            ))
            .expect("answered");

        let result = response
            .result
            .expect("a refusal is a result, not a protocol error");
        assert_eq!(result["isError"], json!(true));
        let message = result["content"][0]["text"].as_str().unwrap();
        assert!(message.contains("--allow-write"), "{message}");
        assert!(
            spawn.calls().is_empty(),
            "a refused command must not be spawned"
        );
    }

    #[test]
    fn a_bad_argument_is_a_tool_error_the_model_can_act_on() {
        let (mut session, spawn) = faked(exec::Outcome::success(""));
        let response = session
            .handle_unasked(&request(
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
            .handle_unasked(&request(
                "tools/call",
                json!({ "name": "gore_config", "arguments": { "subcommand": "reset" } }),
            ))
            .expect("answered");

        let message = response.result.unwrap()["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(message.contains("no subcommand `reset`"), "{message}");
        assert!(message.contains("detect"), "{message}");
    }

    #[test]
    fn a_missing_subcommand_says_which_ones_exist() {
        let (mut session, _) = faked(exec::Outcome::success(""));
        let response = session
            .handle_unasked(&request(
                "tools/call",
                json!({ "name": "gore_config", "arguments": {} }),
            ))
            .expect("answered");

        let result = response.result.unwrap();
        assert_eq!(result["isError"], json!(true));
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("`subcommand` is required"));
    }

    #[test]
    fn stray_top_level_arguments_are_rejected_with_a_hint_about_args() {
        let (mut session, _) = faked(exec::Outcome::success(""));
        let response = session
            .handle_unasked(&request(
                "tools/call",
                json!({
                    "name": "gore_config",
                    "arguments": { "subcommand": "path", "key": "game-path" },
                }),
            ))
            .expect("answered");

        let message = response.result.unwrap()["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(message.contains("inside `args`"), "{message}");
    }

    #[test]
    fn a_command_that_exits_non_zero_is_a_tool_error_not_a_protocol_error() {
        let (mut session, _) = faked(exec::Outcome::failure(1, "error: game-path is not set\n"));
        let response = session
            .handle_unasked(&request(
                "tools/call",
                json!({
                    "name": "gore_config",
                    "arguments": { "subcommand": "get", "args": { "key": "game-path" } },
                }),
            ))
            .expect("answered");

        let result = response.result.expect("still a result");
        assert_eq!(result["isError"], json!(true));
        assert!(
            result["content"][1]["text"]
                .as_str()
                .unwrap()
                .contains("exit code 1"),
            "{result}"
        );
    }

    #[test]
    fn a_notification_method_sent_with_an_id_is_answered() {
        // With an id it is a request, whatever the method is called. Staying silent would leave the
        // caller waiting on a reply that is never coming.
        for method in ["notifications/initialized", "notifications/cancelled"] {
            let line = format!(
                "{{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"{method}\"}}
"
            );
            let mut output = Vec::new();
            serve(options(), Cursor::new(line.into_bytes()), &mut output).expect("clean shutdown");

            let text = String::from_utf8(output).expect("utf-8");
            assert!(
                !text.trim().is_empty(),
                "{method} with an id must be answered"
            );
            let reply: Value = serde_json::from_str(text.trim()).expect("json");
            assert_eq!(reply["id"], json!(7));
            assert_eq!(reply["error"]["code"], json!(errors::INVALID_REQUEST));
        }

        // And the handshake is not advanced by a malformed one.
        let mut session = Session::new(options());
        let request = request_with_id("notifications/initialized", json!({}), json!(7));
        assert!(session.handle_unasked(&request).is_some());
        assert!(
            !session.is_initialized(),
            "state must not come from a malformed request"
        );

        // The real notification still advances it, silently.
        assert!(session
            .handle_unasked(&notification("notifications/initialized", json!({})))
            .is_none());
        assert!(session.is_initialized());
    }

    #[test]
    fn a_null_id_is_a_request_not_a_notification() {
        // Only an omitted `id` denotes a notification. Treating `"id": null` as one would run the
        // call — spawning a child, with whatever it does — and then swallow the reply, leaving the
        // client to wait and very likely retry.
        let input = Cursor::new(
            "{\"jsonrpc\":\"2.0\",\"id\":null,\"method\":\"ping\"}
"
            .as_bytes()
            .to_vec(),
        );
        let mut output = Vec::new();
        serve(options(), input, &mut output).expect("clean shutdown");

        let text = String::from_utf8(output).expect("utf-8");
        assert!(
            !text.trim().is_empty(),
            "a null-id request must be answered"
        );
        let reply: Value = serde_json::from_str(text.trim()).expect("json");
        assert!(
            reply["id"].is_null(),
            "the answer echoes the null id: {reply}"
        );
        assert!(reply["result"].is_object());

        // An omitted id is still a notification, and still silent.
        let notification = Cursor::new(
            "{\"jsonrpc\":\"2.0\",\"method\":\"ping\"}
"
            .as_bytes()
            .to_vec(),
        );
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
        assert_eq!(
            text.lines().count(),
            1,
            "a batch answers in one frame: {text}"
        );
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
        assert!(
            output.is_empty(),
            "an empty array reply is a protocol violation"
        );
    }

    #[test]
    fn an_empty_batch_is_an_invalid_request() {
        let input = Cursor::new(
            "[]
"
            .as_bytes()
            .to_vec(),
        );
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
            .handle_unasked(&request("resources/list", json!({})))
            .expect("answered")
            .result
            .unwrap()["resources"]
            .as_array()
            .unwrap()
            .len();
        assert_eq!(listed, crate::guide::PAGES.len());

        let templates = session
            .handle_unasked(&request("resources/templates/list", json!({})))
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
            .handle_unasked(&request(
                "resources/read",
                json!({ "uri": "gore://guide/bundles" }),
            ))
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
            .handle_unasked(&request(
                "resources/read",
                json!({ "uri": "gore://guide/nope" }),
            ))
            .expect("answered");

        let error = response
            .error
            .expect("unknown resources are protocol errors");
        assert_eq!(error.code, errors::INVALID_PARAMS);
        assert!(error.message.contains("bundles"), "{}", error.message);
    }

    #[test]
    fn an_unknown_tool_is_a_protocol_error_not_a_tool_error() {
        let mut session = Session::new(options());
        let response = session
            .handle_unasked(&request(
                "tools/call",
                json!({ "name": "gore_nonexistent" }),
            ))
            .expect("answered");
        let error = response.error.expect("unknown tools are protocol errors");
        assert_eq!(error.code, errors::INVALID_PARAMS);
        assert!(error.message.contains("gore_nonexistent"));
    }

    #[test]
    fn notifications_produce_no_output_at_all_over_the_transport() {
        let written = exchange("{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n");
        assert!(
            written.is_empty(),
            "a notification must not be answered, got {written:?}"
        );
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

    // ----------------------------------------------------------------------------------------- //
    // Consent                                                                                    //
    // ----------------------------------------------------------------------------------------- //

    /// A `tools/call` for a command that overwrites the game's own localisation cache in place.
    fn a_gated_call() -> Request {
        request(
            "tools/call",
            json!({
                "name": "gore_loc",
                "arguments": {
                    "subcommand": "import",
                    "args": { "lcache": "Alkimia.lcache", "edits": "edits.json" },
                },
            }),
        )
    }

    fn initialize_with(session: &mut Session, capabilities: Value) {
        session
            .handle_unasked(&request(
                "initialize",
                json!({ "protocolVersion": "2025-11-25", "capabilities": capabilities }),
            ))
            .expect("initialize is answered");
    }

    #[test]
    fn only_a_client_that_advertised_elicitation_is_ever_asked() {
        // Sending a question to a client that never declared the capability is not merely impolite:
        // nothing over there is listening for it, so the call would block until something times out.
        let mut session = Session::new(options());
        assert_eq!(
            session.consent_policy(),
            Policy::CannotAsk,
            "before initialize"
        );

        initialize_with(&mut session, json!({ "elicitation": {} }));
        assert_eq!(session.consent_policy(), Policy::Ask);

        let mut session = Session::new(options());
        initialize_with(&mut session, json!({ "roots": { "listChanged": true } }));
        assert_eq!(
            session.consent_policy(),
            Policy::CannotAsk,
            "a different capability is not this one"
        );

        let mut session = Session::new(options());
        initialize_with(&mut session, json!({ "elicitation": null }));
        assert_eq!(
            session.consent_policy(),
            Policy::CannotAsk,
            "an explicit null is not a declaration"
        );

        // The other way a client says no. Reading it as yes is the expensive direction: the
        // question goes out, nothing over there answers it, and the call waits on a round trip
        // that fails instead of being refused outright.
        let mut session = Session::new(options());
        initialize_with(&mut session, json!({ "elicitation": false }));
        assert_eq!(
            session.consent_policy(),
            Policy::CannotAsk,
            "`false` is a refusal, not a declaration"
        );
    }

    #[test]
    fn params_that_are_not_an_object_are_refused_rather_than_flattened() {
        // `params_object` turns an array into the same empty map an omitted member yields, so
        // without this the call answers success having discarded everything the client sent. On
        // `initialize` that is the expensive one: the capabilities vanish, and a client that can
        // show a consent dialog is recorded for the whole session as one that cannot.
        let mut session = Session::new(options());
        let malformed = request_with_id("initialize", json!([1, 2, 3]), json!(7));

        let response = session
            .handle_unasked(&malformed)
            .expect("a request is answered");
        let error = response.error.expect("an error, not a result");
        assert_eq!(error.code, errors::INVALID_PARAMS, "{}", error.message);
        assert_eq!(
            session.consent_policy(),
            Policy::CannotAsk,
            "nothing may be negotiated out of a frame that was refused"
        );

        // Both shapes that mean "no fields" still go through.
        for empty in [json!(null), json!({})] {
            let mut session = Session::new(options());
            assert!(
                session
                    .handle_unasked(&request_with_id("ping", empty.clone(), json!(1)))
                    .expect("answered")
                    .error
                    .is_none(),
                "{empty} means no fields, which is allowed"
            );
        }
    }

    #[test]
    fn the_handshake_happens_once_and_only_as_a_request() {
        // A second `initialize` could replace what was agreed — including whether this client can
        // be asked anything — halfway through a session that has already acted on the first answer.
        let mut session = Session::new(options());
        initialize_with(&mut session, json!({ "elicitation": {} }));
        assert_eq!(session.consent_policy(), Policy::Ask);

        assert!(
            session
                .handle_unasked(&notification("notifications/initialized", json!({})))
                .is_none(),
            "a notification must not be answered"
        );

        let second = session
            .handle_unasked(&request(
                "initialize",
                json!({ "protocolVersion": "2025-11-25", "capabilities": {} }),
            ))
            .expect("a repeated initialize is answered, with an error");
        assert!(second.error.is_some(), "{second:?}");
        assert_eq!(
            session.consent_policy(),
            Policy::Ask,
            "the refused handshake must not have taken the empty capabilities with it"
        );
    }

    #[test]
    fn a_handshake_or_a_tool_call_with_no_id_is_not_acted_on() {
        // JSON-RPC forbids answering either, so acting on one means doing the work and throwing the
        // outcome away. For a tool call that is a child process that changed something nobody will
        // ever be told about, and with no id there is nothing a cancellation could name.
        let mut session = Session::new(options());

        let handshake = notification(
            "initialize",
            json!({ "protocolVersion": "2025-11-25", "capabilities": { "elicitation": {} } }),
        );
        assert!(
            session.handle_unasked(&handshake).is_none(),
            "no reply to a notification"
        );
        assert_eq!(
            session.consent_policy(),
            Policy::CannotAsk,
            "nothing may be negotiated from a frame the other side cannot be told the answer to"
        );

        let spawn = std::sync::Arc::new(exec::FakeSpawn::new(exec::Outcome::success("done\n")));
        let mut session = Session::with_spawn(options(), Box::new(std::sync::Arc::clone(&spawn)));
        initialize_with(&mut session, json!({ "elicitation": {} }));
        let call = notification(
            "tools/call",
            json!({ "name": "gore_guide", "arguments": { "action": "list" } }),
        );
        assert!(
            session.handle_unasked(&call).is_none(),
            "no reply to a notification"
        );
        assert!(spawn.calls().is_empty(), "and nothing ran");
    }

    #[test]
    fn the_server_own_posture_outranks_what_the_client_can_do() {
        // --no-consent-prompts is someone saying "do not interrupt me". A capable client does not
        // get to overrule that by advertising a dialog.
        let mut opts = options();
        opts.never_ask = true;
        let mut session = Session::with_spawn(
            opts,
            Box::new(exec::FakeSpawn::new(exec::Outcome::success(""))),
        );
        initialize_with(&mut session, json!({ "elicitation": {} }));
        assert_eq!(session.consent_policy(), Policy::NeverAsk);
    }

    #[test]
    fn a_gated_call_runs_only_after_the_user_agrees() {
        let spawn = std::sync::Arc::new(exec::FakeSpawn::new(exec::Outcome::success("done\n")));
        let mut session = Session::with_spawn(options(), Box::new(std::sync::Arc::clone(&spawn)));
        initialize_with(&mut session, json!({ "elicitation": {} }));

        let mut peer = Canned::allowing();
        let result = session
            .handle(&a_gated_call(), &mut peer)
            .expect("answered")
            .result
            .unwrap();

        assert_eq!(peer.asked, 1, "exactly one question for one call");
        assert_eq!(result["isError"], json!(false));
        assert_eq!(spawn.calls().len(), 1, "the command ran after the yes");
    }

    #[test]
    fn a_declined_call_never_reaches_the_child_process() {
        // The whole promise: saying no leaves every file exactly as it was. Nothing is spawned,
        // so there is nothing to undo.
        let spawn = std::sync::Arc::new(exec::FakeSpawn::new(exec::Outcome::success("done\n")));
        let mut session = Session::with_spawn(options(), Box::new(std::sync::Arc::clone(&spawn)));
        initialize_with(&mut session, json!({ "elicitation": {} }));

        let mut peer = Canned::declining();
        let result = session
            .handle(&a_gated_call(), &mut peer)
            .expect("answered")
            .result
            .unwrap();

        assert_eq!(peer.asked, 1);
        assert!(
            spawn.calls().is_empty(),
            "a refusal must not run the command"
        );
        assert_eq!(
            result["isError"],
            json!(true),
            "the model has to see this as a failure"
        );
        let text = result["content"][0]["text"].as_str().expect("text");
        assert!(text.contains("`decline`"), "{text}");
        assert!(
            !text.contains("the user was asked"),
            "the answer's author is unknowable: {text}"
        );
    }

    #[test]
    fn a_client_that_cannot_be_asked_gets_the_flag_it_would_take_instead() {
        let spawn = std::sync::Arc::new(exec::FakeSpawn::new(exec::Outcome::success("")));
        let mut session = Session::with_spawn(options(), Box::new(std::sync::Arc::clone(&spawn)));
        initialize_with(&mut session, json!({}));

        // `NoOneToAsk` panics if reached, which is the assertion: no capability, no question.
        let result = session
            .handle_unasked(&a_gated_call())
            .expect("answered")
            .result
            .unwrap();

        assert!(spawn.calls().is_empty());
        let text = result["content"][0]["text"].as_str().expect("text");
        assert!(text.contains("gore mcp serve --allow-write"), "{text}");
    }

    #[test]
    fn a_pre_approved_call_is_never_put_to_anyone() {
        // This is the headless posture. `NoOneToAsk` panicking is what proves the dialog is skipped
        // rather than merely auto-answered.
        let mut opts = options();
        opts.allow_write = true;
        let spawn = std::sync::Arc::new(exec::FakeSpawn::new(exec::Outcome::success("done\n")));
        let mut session = Session::with_spawn(opts, Box::new(std::sync::Arc::clone(&spawn)));
        initialize_with(&mut session, json!({ "elicitation": {} }));

        let result = session
            .handle_unasked(&a_gated_call())
            .expect("answered")
            .result
            .unwrap();
        assert_eq!(result["isError"], json!(false));
        assert_eq!(spawn.calls().len(), 1);
    }

    /// The same call, carrying what the caller says the user already answered.
    fn an_approved_call(words: &str, approval_request_id: &str) -> Request {
        request(
            "tools/call",
            json!({
                "name": "gore_loc",
                "arguments": {
                    "subcommand": "import",
                    "args": { "lcache": "Alkimia.lcache", "edits": "edits.json" },
                    "approval_request_id": approval_request_id,
                    "user_approved": words,
                },
            }),
        )
    }

    #[test]
    fn an_asserted_approval_runs_the_call_and_the_result_says_whose_claim_it_was() {
        // `NoOneToAsk` panicking is the assertion that no dialog was put: the claim replaces the
        // question rather than preceding it, which is what makes it useful in a client that answers
        // its own dialogs in four milliseconds.
        let spawn = std::sync::Arc::new(exec::FakeSpawn::new(exec::Outcome::success("done\n")));
        let mut session = Session::with_spawn(options(), Box::new(std::sync::Arc::clone(&spawn)));
        initialize_with(&mut session, json!({}));

        let refused = session
            .handle_unasked(&a_gated_call())
            .expect("initial refusal is answered")
            .result
            .unwrap();
        assert_eq!(refused["isError"], json!(true));
        let request_id = session
            .pending_approvals
            .back()
            .expect("refusal issued an approval request")
            .id
            .clone();
        assert!(
            refused["content"][0]["text"]
                .as_str()
                .expect("refusal text")
                .contains(&request_id),
            "the caller must receive the id it is meant to relay"
        );

        let result = session
            .handle_unasked(&an_approved_call(
                "hör auf jedes mal zu fragen und mach endlich fertig",
                &request_id,
            ))
            .expect("answered")
            .result
            .unwrap();

        assert_eq!(result["isError"], json!(false), "{result}");
        assert_eq!(spawn.calls().len(), 1, "the exact command ran once");
        assert!(
            session.pending_approvals.is_empty(),
            "the request id was consumed"
        );

        let blocks = result["content"].as_array().expect("content");
        let spoken = blocks
            .iter()
            .filter_map(|block| block["text"].as_str())
            .collect::<Vec<_>>();
        let note = spoken
            .iter()
            .find(|text| text.contains("one-time approval request"))
            .unwrap_or_else(|| panic!("no block records the claim: {spoken:?}"));
        assert!(
            note.contains("hör auf jedes mal zu fragen und mach endlich fertig"),
            "{note}"
        );
    }

    #[test]
    fn a_relay_id_is_bound_to_the_exact_invocation_and_works_only_once() {
        let spawn = std::sync::Arc::new(exec::FakeSpawn::new(exec::Outcome::success("done\n")));
        let mut session = Session::with_spawn(options(), Box::new(std::sync::Arc::clone(&spawn)));
        initialize_with(&mut session, json!({}));

        session
            .handle_unasked(&a_gated_call())
            .expect("refused once");
        let request_id = session
            .pending_approvals
            .back()
            .expect("request id")
            .id
            .clone();

        let changed = request(
            "tools/call",
            json!({
                "name": "gore_loc",
                "arguments": {
                    "subcommand": "import",
                    "args": { "lcache": "Other.lcache", "edits": "edits.json" },
                    "approval_request_id": request_id,
                    "user_approved": "ja",
                },
            }),
        );
        let changed_result = session
            .handle_unasked(&changed)
            .expect("answered")
            .result
            .unwrap();
        assert_eq!(changed_result["isError"], json!(true));
        assert!(changed_result["content"][0]["text"]
            .as_str()
            .expect("text")
            .contains("different exact command"));
        assert!(spawn.calls().is_empty(), "a changed retry must not run");

        let exact = an_approved_call("ja", &request_id);
        let exact_result = session
            .handle_unasked(&exact)
            .expect("answered")
            .result
            .unwrap();
        assert_eq!(exact_result["isError"], json!(false));
        assert_eq!(spawn.calls().len(), 1);

        let replay = session
            .handle_unasked(&exact)
            .expect("answered")
            .result
            .unwrap();
        assert_eq!(replay["isError"], json!(true));
        assert!(replay["content"][0]["text"]
            .as_str()
            .expect("text")
            .contains("expired, or already used"));
        assert_eq!(spawn.calls().len(), 1, "reusing the id must not run again");
    }

    #[test]
    fn approval_words_without_the_bound_request_id_are_never_enough() {
        let spawn = std::sync::Arc::new(exec::FakeSpawn::new(exec::Outcome::success("done\n")));
        let mut session = Session::with_spawn(options(), Box::new(std::sync::Arc::clone(&spawn)));
        initialize_with(&mut session, json!({}));

        let call = request(
            "tools/call",
            json!({
                "name": "gore_loc",
                "arguments": {
                    "subcommand": "import",
                    "args": { "lcache": "Alkimia.lcache", "edits": "edits.json" },
                    "user_approved": "ja",
                },
            }),
        );
        let result = session
            .handle_unasked(&call)
            .expect("answered")
            .result
            .unwrap();
        assert_eq!(result["isError"], json!(true));
        let text = result["content"][0]["text"].as_str().expect("text");
        assert!(text.contains("must be supplied together"), "{text}");
        assert!(spawn.calls().is_empty());
    }

    #[test]
    fn an_expired_relay_id_cannot_run_its_former_command() {
        let spawn = std::sync::Arc::new(exec::FakeSpawn::new(exec::Outcome::success("done\n")));
        let mut session = Session::with_spawn(options(), Box::new(std::sync::Arc::clone(&spawn)));
        initialize_with(&mut session, json!({}));

        session
            .handle_unasked(&a_gated_call())
            .expect("refused once");
        let pending = session.pending_approvals.back_mut().expect("request id");
        let request_id = pending.id.clone();
        pending.expires_at = Instant::now() - Duration::from_secs(1);

        let result = session
            .handle_unasked(&an_approved_call("ja", &request_id))
            .expect("answered")
            .result
            .unwrap();
        assert_eq!(result["isError"], json!(true));
        let text = result["content"][0]["text"].as_str().expect("text");
        assert!(text.contains("expired"), "{text}");
        assert!(spawn.calls().is_empty());
    }

    #[test]
    fn an_assertion_does_not_move_a_server_that_was_told_never_to_ask() {
        let mut opts = options();
        opts.never_ask = true;
        let spawn = std::sync::Arc::new(exec::FakeSpawn::new(exec::Outcome::success("done\n")));
        let mut session = Session::with_spawn(opts, Box::new(std::sync::Arc::clone(&spawn)));
        initialize_with(&mut session, json!({ "elicitation": {} }));

        let result = session
            .handle_unasked(&an_approved_call("go ahead", "invented"))
            .expect("answered")
            .result
            .unwrap();

        assert_eq!(result["isError"], json!(true));
        assert!(
            spawn.calls().is_empty(),
            "nothing may run under --no-consent-prompts"
        );
        let text = result["content"][0]["text"].as_str().expect("text");
        assert!(text.contains("--no-consent-prompts"), "{text}");
    }

    #[test]
    fn an_approval_that_is_not_words_is_reported_rather_than_believed() {
        // A number, a boolean or an empty string quotes nobody. Reading any of them as agreement
        // would make the emptiest possible claim the cheapest way past the gate.
        let spawn = std::sync::Arc::new(exec::FakeSpawn::new(exec::Outcome::success("done\n")));
        let mut session = Session::with_spawn(options(), Box::new(std::sync::Arc::clone(&spawn)));
        initialize_with(&mut session, json!({ "elicitation": {} }));

        for bogus in [json!(true), json!(1), json!(""), json!("   ")] {
            let call = request(
                "tools/call",
                json!({
                    "name": "gore_loc",
                    "arguments": {
                        "subcommand": "import",
                        "args": { "lcache": "Alkimia.lcache", "edits": "edits.json" },
                        "user_approved": bogus,
                    },
                }),
            );
            let result = session
                .handle_unasked(&call)
                .expect("answered")
                .result
                .unwrap();

            assert_eq!(
                result["isError"],
                json!(true),
                "{bogus} was accepted: {result}"
            );
            let text = result["content"][0]["text"].as_str().expect("text");
            assert!(text.contains("user_approved"), "{bogus}: {text}");
        }
        assert!(
            spawn.calls().is_empty(),
            "nothing may run on a malformed claim"
        );
    }

    #[test]
    fn a_null_approval_is_read_as_no_approval_rather_than_as_a_mistake() {
        // Clients differ on how they serialise an omitted optional. Reporting `null` as malformed
        // would turn an ordinary question into a confusing error for callers that never set it.
        let spawn = std::sync::Arc::new(exec::FakeSpawn::new(exec::Outcome::success("done\n")));
        let mut session = Session::with_spawn(options(), Box::new(std::sync::Arc::clone(&spawn)));
        initialize_with(&mut session, json!({ "elicitation": {} }));

        let call = request(
            "tools/call",
            json!({
                "name": "gore_loc",
                "arguments": {
                    "subcommand": "import",
                    "args": { "lcache": "Alkimia.lcache", "edits": "edits.json" },
                    "user_approved": null,
                },
            }),
        );
        let mut peer = Canned::declining();
        let result = session
            .handle(&call, &mut peer)
            .expect("answered")
            .result
            .unwrap();

        assert_eq!(peer.asked, 1, "the question is still put");
        assert_eq!(result["isError"], json!(true));
        assert!(spawn.calls().is_empty());
    }

    #[test]
    fn an_ordinary_read_asks_nobody() {
        let spawn = std::sync::Arc::new(exec::FakeSpawn::new(exec::Outcome::success("ok\n")));
        let mut session = Session::with_spawn(options(), Box::new(std::sync::Arc::clone(&spawn)));
        initialize_with(&mut session, json!({ "elicitation": {} }));

        let response = session.handle_unasked(&request(
            "tools/call",
            json!({ "name": "gore_config", "arguments": { "subcommand": "path" } }),
        ));
        assert_eq!(
            response.expect("answered").result.unwrap()["isError"],
            json!(false)
        );
        assert_eq!(spawn.calls().len(), 1);
    }

    #[test]
    fn explicit_strict_standalone_compile_runs_without_any_consent_channel() {
        let spawn = std::sync::Arc::new(exec::FakeSpawn::new(exec::Outcome::success("compiled\n")));
        let mut session = Session::with_spawn(options(), Box::new(std::sync::Arc::clone(&spawn)));
        initialize_with(&mut session, json!({}));

        let response = session
            .handle_unasked(&request(
                "tools/call",
                json!({
                    "name": "gore_as",
                    "arguments": {
                        "subcommand": "compile",
                        "args": {
                            "src": "scripts",
                            "out": "fresh.Cache",
                            "work_dir": "compiler-work",
                            "backend": "standalone",
                            "game": "G"
                        }
                    }
                }),
            ))
            .expect("answered")
            .result
            .unwrap();

        assert_eq!(response["isError"], json!(false), "{response}");
        assert_eq!(spawn.calls().len(), 1);
        assert!(!spawn.calls()[0].may_launch_game);
        assert!(spawn.calls()[0].consent.is_none());
    }

    #[test]
    fn dedicated_standalone_compile_runs_without_any_consent_channel() {
        let spawn = std::sync::Arc::new(exec::FakeSpawn::new(exec::Outcome::success("compiled\n")));
        let mut session = Session::with_spawn(options(), Box::new(std::sync::Arc::clone(&spawn)));
        initialize_with(&mut session, json!({}));

        let response = session
            .handle_unasked(&request(
                "tools/call",
                json!({
                    "name": "gore_as_compile_module",
                    "arguments": {
                        "op": "add",
                        "module": "MyMod.Dialog",
                        "rel_path": "MyMod/Dialog.as",
                        "source": "Dialog.as",
                        "work_dir": "compiler-work",
                        "out": "fresh.Cache",
                        "game": "G"
                    }
                }),
            ))
            .expect("answered")
            .result
            .unwrap();
        assert_eq!(response["isError"], json!(false));
        assert_eq!(spawn.calls().len(), 1);
        assert!(spawn.calls()[0]
            .argv
            .iter()
            .any(|value| value == "standalone"));
    }

    // ----------------------------------------------------------------------------------------- //
    // The transport half of asking                                                               //
    // ----------------------------------------------------------------------------------------- //

    /// Run one `TransportPeer` round trip over scripted input, returning the answer, everything
    /// written to the client, and whatever frames were set aside.
    fn ask_over(input: &str, call_id: Value) -> (Result<Value, String>, Vec<Value>, usize) {
        let mut written = Vec::new();
        let mut transport = Transport::new(Cursor::new(input.as_bytes().to_vec()), &mut written);
        let mut deferred = VecDeque::new();
        let mut asked = 0;

        let outcome = {
            let mut peer = TransportPeer {
                call_id,
                transport: &mut transport,
                deferred: &mut deferred,
                asked: &mut asked,
            };
            peer.request("elicitation/create", json!({ "message": "?" }))
        };

        let sent = String::from_utf8(written)
            .expect("utf-8")
            .lines()
            .map(|line| serde_json::from_str(line).expect("one JSON value per line"))
            .collect();
        (outcome, sent, deferred.len())
    }

    #[test]
    fn a_question_goes_out_as_a_request_and_its_answer_comes_back() {
        let (answer, sent, deferred) = ask_over(
            "{\"jsonrpc\":\"2.0\",\"id\":\"gore-consent-1\",\"result\":{\"action\":\"accept\"}}\n",
            json!(1),
        );

        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0]["method"], "elicitation/create");
        assert_eq!(sent[0]["jsonrpc"], "2.0");
        assert_eq!(sent[0]["id"], "gore-consent-1");
        assert_eq!(answer.expect("an answer")["action"], "accept");
        assert_eq!(deferred, 0);
    }

    #[test]
    fn whatever_the_client_says_meanwhile_is_set_aside_rather_than_handled() {
        // Handling it here would start a second tool call underneath the first — one game
        // installation, two writers — so it waits its turn in the queue the loop drains.
        let (answer, _, deferred) = ask_over(
            concat!(
                "{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"ping\"}\n",
                "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/progress\"}\n",
                "{\"jsonrpc\":\"2.0\",\"id\":\"gore-consent-1\",\"result\":{\"action\":\"decline\"}}\n",
            ),
            json!(1),
        );

        assert_eq!(answer.expect("an answer")["action"], "decline");
        assert_eq!(deferred, 2, "both frames are kept for the loop to answer");
    }

    #[test]
    fn an_answer_to_a_different_question_does_not_settle_this_one() {
        let (answer, _, deferred) = ask_over(
            concat!(
                "{\"jsonrpc\":\"2.0\",\"id\":\"gore-consent-99\",\"result\":{\"action\":\"accept\"}}\n",
                "{\"jsonrpc\":\"2.0\",\"id\":\"gore-consent-1\",\"result\":{\"action\":\"decline\"}}\n",
            ),
            json!(1),
        );

        assert_eq!(
            answer.expect("an answer")["action"],
            "decline",
            "ours is the second one"
        );
        assert_eq!(deferred, 1);
    }

    #[test]
    fn cancelling_the_call_ends_the_wait() {
        // Without this the server sits on a question for a request the client has already given up
        // on, and the session is wedged until somebody closes a pipe.
        let (answer, _, _) = ask_over(
            "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/cancelled\",\"params\":{\"requestId\":1}}\n",
            json!(1),
        );
        match answer {
            Err(detail) => assert!(detail.contains("cancelled"), "{detail}"),
            Ok(value) => panic!("a cancellation is not an answer: {value}"),
        }

        // A cancellation naming a *different* request is somebody else's business.
        let (answer, _, deferred) = ask_over(
            concat!(
                "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/cancelled\",\"params\":{\"requestId\":42}}\n",
                "{\"jsonrpc\":\"2.0\",\"id\":\"gore-consent-1\",\"result\":{\"action\":\"accept\"}}\n",
            ),
            json!(1),
        );
        assert_eq!(answer.expect("an answer")["action"], "accept");
        assert_eq!(deferred, 1);
    }

    #[test]
    fn a_client_that_hangs_up_mid_question_is_a_failure_not_a_yes() {
        let (answer, _, _) = ask_over("", json!(1));
        match answer {
            Err(detail) => assert!(detail.contains("closed"), "{detail}"),
            Ok(value) => panic!("end of input is not agreement: {value}"),
        }
    }

    #[test]
    fn an_error_response_to_the_question_is_reported_with_its_message() {
        let (answer, _, _) = ask_over(
            "{\"jsonrpc\":\"2.0\",\"id\":\"gore-consent-1\",\"error\":{\"code\":-32601,\"message\":\"no elicitation here\"}}\n",
            json!(1),
        );
        match answer {
            Err(detail) => {
                assert!(detail.contains("no elicitation here"), "{detail}");
                assert!(detail.contains("-32601"), "{detail}");
            }
            Ok(value) => panic!("an error is not an answer: {value}"),
        }
    }

    #[test]
    fn a_flood_of_unrelated_frames_ends_the_wait_instead_of_growing_without_bound() {
        let mut input = String::new();
        for id in 0..MAX_DEFERRED_FRAMES + 10 {
            input.push_str(&format!(
                "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"ping\"}}\n"
            ));
        }
        let (answer, _, deferred) = ask_over(&input, json!("call"));

        assert!(answer.is_err(), "the wait has to end somewhere");
        assert!(
            deferred <= MAX_DEFERRED_FRAMES,
            "the queue stayed bounded: {deferred}"
        );
    }

    #[test]
    fn an_answer_nobody_is_waiting_for_is_dropped_rather_than_replied_to() {
        // A late answer arrives here after its call was abandoned. JSON-RPC has no reply to a
        // reply, so anything written back would be a frame the client cannot correlate.
        let written = exchange(concat!(
            "{\"jsonrpc\":\"2.0\",\"id\":\"gore-consent-1\",\"result\":{\"action\":\"accept\"}}\n",
            "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"ping\"}\n",
        ));

        assert_eq!(written.len(), 1, "only the ping earns a reply");
        assert_eq!(written[0]["id"], 3);
    }
}
