//! Asking a person before running something that cannot be undone.
//!
//! The safety gate in `argv.rs` decides *whether* a call needs a human to agree. This module is how
//! it gets asked: MCP's [elicitation] request, which a server sends mid-call and a client renders as
//! a dialog. Accepted, the command runs; declined, the model gets an ordinary tool error and can
//! pick a different path.
//!
//! That ordering is the whole point. The alternative — the one this replaces — was to refuse
//! outright and name a flag the server would have to be restarted with, which put a client restart
//! between the model and every overwrite. A person who wants to say yes should be able to say yes.
//!
//! Elicitation is a client capability, so a client that does not advertise it is never asked. It
//! gets the old refusal, which is still the honest answer there: nobody on this connection can be
//! reached, and only the user can widen what the server may do.
//!
//! [elicitation]: https://modelcontextprotocol.io/specification/2025-06-18/client/elicitation

use serde_json::{json, Value};

/// The MCP method that puts a question in front of a person.
pub const ELICITATION_METHOD: &str = "elicitation/create";

/// The tool argument through which a caller relays an answer the user gave it directly.
///
/// The second route to consent, and the only one left where a client advertises the capability but
/// answers its own dialogs without showing anybody anything. It is a claim, not a confirmation:
/// see [`Decision::AllowedByAssertion`].
pub const APPROVAL_FIELD: &str = "user_approved";

/// The field the dialog collects, and the two values it may take.
const DECISION_FIELD: &str = "decision";
const RUN: &str = "run";

/// What a call would do that a person should agree to first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Consent {
    /// The command path without arguments, e.g. `texture pack`.
    pub path: String,
    /// Completes "`gore <path>` …": what the call does that earned the question. Written for the
    /// person deciding, so it names files rather than arguments wherever it can.
    pub reason: String,
    /// What the model could do instead. Appended only to the text the *model* reads — a human
    /// deciding whether to allow an overwrite does not need to be told to pick another filename.
    pub remedy: Option<String>,
    /// The command line, exactly as the tool result would show it.
    ///
    /// Shown in the dialog because `reason` cannot always name the file: an in-place rewrite is
    /// identified by an argument that was *omitted*, and which argument holds the input differs per
    /// command. The command line answers "which file?" for every arm at once, without a per-command
    /// table to keep in step — and it is the same string the user could paste into a shell.
    pub command_line: String,
    /// Which pre-approval flags cover this call.
    pub needs: Needs,
}

/// The permissions a call requires, as flags rather than as prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Needs {
    /// Changes the installation, or overwrites something already there.
    pub write: bool,
    /// Starts the game executable.
    pub game_launch: bool,
}

impl Needs {
    /// The `gore mcp serve` flags that would pre-approve this call, most significant first.
    ///
    /// A game launch always writes as well — compiling drives the game to regenerate its cache and
    /// then stages the result — so the launch flag never appears alone. Naming it by itself would
    /// send someone to restart with a flag set that still would not cover the call.
    pub fn flags(&self) -> &'static str {
        match (self.game_launch, self.write) {
            (true, _) => "--allow-game-launch --allow-write",
            (false, true) => "--allow-write",
            (false, false) => "",
        }
    }
}

/// How this server may treat a call that needs consent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// Put the question to a person and wait for the answer.
    Ask,
    /// The client never advertised the `elicitation` capability, so there is nobody to ask.
    CannotAsk,
    /// The server was started with consent prompts turned off.
    NeverAsk,
}

/// The outcome of asking.
///
/// Note what is *not* here: "the user said no". A server sees an answer arrive on a socket; it
/// never sees who produced it. Claude Code running non-interactively answers `decline` itself,
/// within milliseconds and without showing anything, and a message asserting that a person decided
/// would be a plain falsehood — one that also sends the model off to respect a wish nobody made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Run it.
    Allow,
    /// No question was put, because the caller stated the user had already agreed to this call and
    /// quoted them. What the server knows is only that the claim was made — which is why it is a
    /// separate variant, and why the result says whose claim it was.
    AllowedByAssertion(String),
    /// The client answered `decline`. Somebody, or something, said no.
    Declined,
    /// The client answered `cancel`: dismissed with no choice made. Per the specification this is
    /// what a closed dialog produces — and what a client that cannot show one has to send.
    Dismissed,
    /// Nobody could be asked. Carries why, for the message the model reads.
    NotAsked(Policy),
    /// The round trip broke: the client errored, or answered something unreadable.
    Failed(String),
}

/// A channel for sending this server's own requests and reading the client's reply.
///
/// A trait rather than a concrete transport so the session stays testable without a pipe: the real
/// implementation writes to stdout and blocks on stdin, and a test hands over a canned answer. The
/// same seam `Spawn` provides for child processes.
pub trait Peer {
    /// Send `method` and block until the client answers it.
    ///
    /// `Err` is a transport or protocol failure — a broken pipe, an error response, a cancellation.
    /// A user saying "no" is not an error; it comes back as a normal `Ok` result.
    fn request(&mut self, method: &'static str, params: Value) -> Result<Value, String>;
}

impl Decision {
    /// Whether the command may run.
    pub fn allows(&self) -> bool {
        matches!(self, Decision::Allow | Decision::AllowedByAssertion(_))
    }
}

/// Ask, if there is anyone to ask.
///
/// `approval` is what the caller says the user already answered, in the user's own words. It stands
/// in for the question rather than adding to it: where it arrives, no dialog is put, because the
/// cases it exists for are exactly those where a dialog reaches nobody.
pub fn decide(
    consent: &Consent,
    policy: Policy,
    approval: Option<&str>,
    peer: &mut dyn Peer,
) -> Decision {
    // Checked before the claim, not after: `--no-consent-prompts` is the posture for a server put
    // in front of an unattended agent, and the claim would be that agent's own.
    if policy == Policy::NeverAsk {
        return Decision::NotAsked(policy);
    }
    if let Some(words) = approval.map(str::trim).filter(|words| !words.is_empty()) {
        return Decision::AllowedByAssertion(words.to_string());
    }
    match policy {
        Policy::Ask => {}
        other => return Decision::NotAsked(other),
    }
    match peer.request(ELICITATION_METHOD, elicitation_params(consent)) {
        Ok(result) => interpret(&result),
        Err(detail) => Decision::Failed(detail),
    }
}

/// The `elicitation/create` params for one question.
///
/// The schema is a single enum rather than a boolean. Elicitation's schema subset allows either,
/// but a boolean renders as a checkbox that defaults to one of the answers, and a confirmation
/// whose default is "yes" is not a confirmation. Two named choices cannot be answered by accident.
pub fn elicitation_params(consent: &Consent) -> Value {
    json!({
        "message": question(consent),
        "requestedSchema": {
            "type": "object",
            "properties": {
                DECISION_FIELD: {
                    "type": "string",
                    "title": "Run this command?",
                    "description": "Nothing has run yet. Cancelling leaves every file untouched.",
                    "enum": [RUN, "cancel"],
                    "enumNames": ["Run it", "Do not run it"],
                },
            },
            "required": [DECISION_FIELD],
        },
    })
}

/// What the person is shown.
fn question(consent: &Consent) -> String {
    format!(
        "`gore {}` {}.\n\n{}\n\nRun it?",
        consent.path, consent.reason, consent.command_line
    )
}

/// Read the client's reply.
///
/// Only an explicit accept carrying an explicit "run" is agreement. Every other shape — a decline,
/// a dismissal, a missing field, an action this revision does not define — is read as "no", because
/// the failure mode of guessing wrong in the other direction is an overwritten file.
fn interpret(result: &Value) -> Decision {
    let Some(action) = result.get("action").and_then(Value::as_str) else {
        return Decision::Failed("the client's answer had no `action`".into());
    };
    match action {
        "accept" => {
            let chose_run = result
                .get("content")
                .and_then(|content| content.get(DECISION_FIELD))
                .and_then(Value::as_str)
                .is_some_and(|choice| choice == RUN);
            if chose_run {
                Decision::Allow
            } else {
                Decision::Declined
            }
        }
        "decline" => Decision::Declined,
        // Kept apart from `decline` because the specification gives them different meanings —
        // "explicitly refused" against "dismissed without choosing" — and only the second is what a
        // client sends when it never managed to put the question in front of anyone.
        "cancel" => Decision::Dismissed,
        other => Decision::Failed(format!("the client answered with an unknown action `{other}`")),
    }
}

/// What a result says about a call that ran on a claim rather than on an answer this server saw.
///
/// It names the claim as a claim. Whoever reads the transcript afterwards — the user most of all —
/// is entitled to know that nothing here verified it, and to see the words it was made with.
pub fn assertion_note(words: &str) -> String {
    format!(
        "This ran on the assistant's assertion of prior approval, quoted as: \"{words}\". No \
         confirmation reached this server; the claim was not verified."
    )
}

/// The way back from every refusal a retry could still resolve.
///
/// A refusal that only names a flag the model cannot set is a dead end: it either gives up or nags
/// for a restart. There is a cheaper move available to it — put the question to the user in the
/// conversation, where this server cannot reach — and this paragraph is what tells it so.
const ASK_THEM_YOURSELF: &str = "So do not send this call unchanged. Instead ask the user yourself, \
here in the conversation, and show them the command line above. If they agree, send the same call \
again with `user_approved` set to their own words: that runs it without another question, and the \
result records that it ran on your claim rather than on an answer this server saw.";

/// The route that does not depend on anybody believing a claim.
fn last_resort(consent: &Consent) -> String {
    format!(
        "If they would rather not be asked at all, only they can arrange that, by restarting the \
         server with:\n\
         \n    gore mcp serve {}",
        consent.needs.flags(),
    )
}

/// The tool error a non-`Allow` decision produces, written for the model that has to react to it.
///
/// Each variant says something different about what to try next, and that difference matters: a
/// decline means this specific call was considered and rejected, while an unaskable client means
/// the call was never seen by anyone and only the user can change that.
pub fn refusal(consent: &Consent, decision: &Decision) -> String {
    let head = format!("`gore {}` {}", consent.path, consent.reason);
    let remedy = match &consent.remedy {
        Some(remedy) => format!("\n\n{remedy}."),
        None => String::new(),
    };

    match decision {
        // Callers gate on these; producing a refusal for one would be a bug, but a message beats a
        // panic on a path that only ever runs inside a live session.
        Decision::Allow | Decision::AllowedByAssertion(_) => format!("{head}, and it was allowed."),

        Decision::Declined => format!(
            "refused: the confirmation came back \"no\" — the client answered `decline`.\n\n\
             {head}.{remedy}\n\n\
             Nothing ran. Whether a person actually saw the question is not visible from here: a \
             client that cannot show a dialog answers on the user's behalf, in milliseconds.\n\n\
             {ASK_THEM_YOURSELF}\n\n\
             {last_resort}",
            last_resort = last_resort(consent),
        ),

        Decision::Dismissed => format!(
            "refused: the confirmation was dismissed without an answer — the client answered \
             `cancel`.\n\n\
             {head}.{remedy}\n\n\
             Nothing ran. A dismissal means nobody chose: the dialog was closed, or the client \
             could not show one at all.\n\n\
             {ASK_THEM_YOURSELF}\n\n\
             {last_resort}",
            last_resort = last_resort(consent),
        ),

        Decision::NotAsked(Policy::CannotAsk) => format!(
            "refused: {head}, and this MCP client cannot put that question to the user — it did \
             not advertise the `elicitation` capability during initialize.{remedy}\n\n\
             {ASK_THEM_YOURSELF}\n\n\
             {last_resort}",
            last_resort = last_resort(consent),
        ),

        Decision::NotAsked(Policy::NeverAsk) => format!(
            "refused: {head}, and this server was started with --no-consent-prompts, so it does \
             not ask. `user_approved` is refused here too — the flag exists precisely so that an \
             unattended agent cannot talk its own way past this.\n\n\
             Only the user can allow it, by restarting the server with:\n\
             \n    gore mcp serve {flags}{remedy}",
            flags = consent.needs.flags(),
        ),

        // Unreachable by construction, but a wrong message here would be a silent lie about why a
        // command did not run.
        Decision::NotAsked(Policy::Ask) => format!(
            "refused: {head}, and the confirmation was never sent.{remedy}"
        ),

        Decision::Failed(detail) => format!(
            "refused: {head}, and asking the user about it failed: {detail}. Treated as a \
             refusal — nothing ran.{remedy}\n\n\
             {ASK_THEM_YOURSELF}\n\n\
             {last_resort}",
            last_resort = last_resort(consent),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A peer that hands back one canned answer and remembers what it was asked.
    struct StubPeer {
        answer: Result<Value, String>,
        asked: Vec<(&'static str, Value)>,
    }

    impl StubPeer {
        fn answering(answer: Value) -> Self {
            Self { answer: Ok(answer), asked: Vec::new() }
        }

        fn failing(detail: &str) -> Self {
            Self { answer: Err(detail.to_string()), asked: Vec::new() }
        }
    }

    impl Peer for StubPeer {
        fn request(&mut self, method: &'static str, params: Value) -> Result<Value, String> {
            self.asked.push((method, params));
            self.answer.clone()
        }
    }

    fn consent() -> Consent {
        Consent {
            path: "texture pack".into(),
            reason: "`out` already exists at `C:\\mods\\skin.utoc`, and this command overwrites its \
                     output rather than refusing"
                .into(),
            remedy: Some("Choose a path that does not exist yet".into()),
            command_line: "gore.exe texture pack --out 'C:\\mods\\skin.utoc'".into(),
            needs: Needs { write: true, game_launch: false },
        }
    }

    #[test]
    fn an_asserted_approval_runs_the_call_without_putting_a_question() {
        // The case this exists for: a client that advertises `elicitation` but answers the question
        // itself, in milliseconds, without showing anybody anything. Asking again there produces
        // another instant refusal, so the claim has to stand in for the question, not follow it.
        let mut peer = StubPeer::answering(json!({ "action": "decline" }));
        let decision = decide(&consent(), Policy::Ask, Some("yes, overwrite it"), &mut peer);

        assert_eq!(decision, Decision::AllowedByAssertion("yes, overwrite it".into()));
        assert!(decision.allows());
        assert!(peer.asked.is_empty(), "the claim replaces the question rather than adding to it");
    }

    #[test]
    fn an_assertion_carries_no_weight_where_the_server_was_told_not_to_ask() {
        // `--no-consent-prompts` is the posture for a server put in front of an agent nobody is
        // watching. A claim made by that very agent is exactly what it was set against.
        let mut peer = StubPeer::answering(json!({ "action": "accept", "content": { DECISION_FIELD: RUN } }));
        let decision = decide(&consent(), Policy::NeverAsk, Some("go ahead"), &mut peer);

        assert_eq!(decision, Decision::NotAsked(Policy::NeverAsk));
        assert!(!decision.allows());
        assert!(peer.asked.is_empty());
    }

    #[test]
    fn an_empty_assertion_is_not_an_approval() {
        // A caller that sets the field to nothing has quoted nobody. Reading that as agreement would
        // make the emptiest possible claim the cheapest way past the gate.
        for blank in ["", "   ", "\n\t "] {
            let mut peer = StubPeer::answering(json!({ "action": "decline" }));
            let decision = decide(&consent(), Policy::Ask, Some(blank), &mut peer);

            assert_eq!(decision, Decision::Declined, "{blank:?} must not allow the call");
            assert_eq!(peer.asked.len(), 1, "{blank:?} should have fallen through to the question");
        }
    }

    #[test]
    fn a_client_that_cannot_be_asked_still_accepts_an_assertion() {
        // Without this the field would be useless in the posture that needs it most: no dialog can
        // reach the user, so the only route left is the model asking them in the conversation.
        let mut peer = StubPeer::answering(json!({ "action": "decline" }));
        let decision = decide(&consent(), Policy::CannotAsk, Some("ja, mach"), &mut peer);

        assert_eq!(decision, Decision::AllowedByAssertion("ja, mach".into()));
        assert!(peer.asked.is_empty());
    }

    #[test]
    fn the_note_on_an_asserted_run_says_whose_claim_it_was() {
        // The result has to be readable later by someone asking "who agreed to this?". The honest
        // answer is that the server never saw an answer, only a claim — and it quotes the claim.
        let note = assertion_note("ja, überschreib die Datei");

        assert!(note.contains("ja, überschreib die Datei"), "{note}");
        assert!(note.contains("assertion"), "{note}");
        // Same rule as every refusal: this server cannot report that a person decided.
        for claim in ["the user approved", "the user said", "the user agreed"] {
            assert!(!note.contains(claim), "{note}");
        }
    }

    #[test]
    fn a_refusal_that_can_be_retried_names_the_way_back() {
        // Every posture below leaves the model with something to do: ask the user itself, then send
        // the call again carrying their words. Without this the refusal is a dead end and the model
        // either gives up or nags for a flag it cannot set.
        let consent = consent();
        for decision in [
            Decision::Declined,
            Decision::Dismissed,
            Decision::NotAsked(Policy::CannotAsk),
            Decision::Failed("broken pipe".into()),
        ] {
            let text = refusal(&consent, &decision);
            assert!(text.contains("user_approved"), "{decision:?} hides the way back: {text}");
            assert!(text.contains("send the same call again"), "{decision:?}: {text}");
        }

        // Not this one. A server started with --no-consent-prompts refuses claims too, so offering
        // the retry there would send the model into a loop it cannot win.
        let never = refusal(&consent, &Decision::NotAsked(Policy::NeverAsk));
        assert!(!never.contains("send the same call again"), "{never}");
        assert!(never.contains("user_approved"), "it must still say the field will not help: {never}");
    }

    #[test]
    fn an_accepted_run_is_the_only_way_to_allow() {
        let mut peer = StubPeer::answering(json!({
            "action": "accept",
            "content": { DECISION_FIELD: RUN },
        }));
        assert_eq!(decide(&consent(), Policy::Ask, None, &mut peer), Decision::Allow);
        assert_eq!(peer.asked.len(), 1);
        assert_eq!(peer.asked[0].0, ELICITATION_METHOD);
    }

    #[test]
    fn every_other_answer_shape_refuses() {
        // Enumerated rather than sampled: each of these is a way a client can answer, and the cost
        // of reading any one of them as agreement is a file that is already gone.
        let refusals = [
            json!({ "action": "decline" }),
            json!({ "action": "cancel" }),
            // Accepted the dialog, chose the other option.
            json!({ "action": "accept", "content": { DECISION_FIELD: "cancel" } }),
            // Accepted with nothing filled in.
            json!({ "action": "accept" }),
            json!({ "action": "accept", "content": {} }),
            // The field is there but not a string we know.
            json!({ "action": "accept", "content": { DECISION_FIELD: true } }),
            json!({ "action": "accept", "content": { DECISION_FIELD: "RUN" } }),
        ];
        for answer in refusals {
            let mut peer = StubPeer::answering(answer.clone());
            assert_ne!(
                decide(&consent(), Policy::Ask, None, &mut peer),
                Decision::Allow,
                "{answer} must not allow the call"
            );
        }
    }

    #[test]
    fn an_unreadable_answer_is_a_failure_rather_than_a_decline() {
        // The two produce different advice to the model, so collapsing them would send it to ask
        // the user about a decision the user never made.
        let mut peer = StubPeer::answering(json!({ "content": { DECISION_FIELD: RUN } }));
        assert!(matches!(decide(&consent(), Policy::Ask, None, &mut peer), Decision::Failed(_)));

        let mut peer = StubPeer::answering(json!({ "action": "maybe" }));
        assert!(matches!(decide(&consent(), Policy::Ask, None, &mut peer), Decision::Failed(_)));

        let mut peer = StubPeer::failing("broken pipe");
        match decide(&consent(), Policy::Ask, None, &mut peer) {
            Decision::Failed(detail) => assert!(detail.contains("broken pipe")),
            other => panic!("expected a failure, got {other:?}"),
        }
    }

    #[test]
    fn a_client_that_cannot_be_asked_is_never_sent_a_question() {
        for policy in [Policy::CannotAsk, Policy::NeverAsk] {
            let mut peer = StubPeer::answering(json!({ "action": "accept" }));
            assert_eq!(decide(&consent(), policy, None, &mut peer), Decision::NotAsked(policy));
            assert!(peer.asked.is_empty(), "{policy:?} must not reach the wire");
        }
    }

    #[test]
    fn the_question_names_the_command_what_it_would_do_and_the_line_that_does_it() {
        let params = elicitation_params(&consent());
        let message = params["message"].as_str().unwrap();
        assert!(message.contains("gore texture pack"), "{message}");
        assert!(message.contains("C:\\mods\\skin.utoc"), "{message}");
        // The command line is what answers "which file?" for the arms whose reason cannot: an
        // in-place rewrite is identified by an argument that was left out.
        assert!(message.contains("gore.exe texture pack --out"), "{message}");
        // The remedy is advice for the model about picking another path; it is noise in a dialog.
        assert!(!message.contains("does not exist yet"), "{message}");
    }

    #[test]
    fn the_dialog_answers_which_file_even_when_the_reason_cannot() {
        // The in-place arm names the *omitted* argument, so on its own it identifies no file at
        // all. This is the case the command line exists for.
        let in_place = Consent {
            path: "loc import".into(),
            reason: "would overwrite its input in place because `out` was omitted".into(),
            remedy: Some("Pass `out` to write a new file instead".into()),
            command_line: "gore.exe loc import --lcache 'D:\\G1R\\Alkimia.lcache'".into(),
            needs: Needs { write: true, game_launch: false },
        };
        let message = elicitation_params(&in_place)["message"].as_str().unwrap().to_string();
        assert!(message.contains("Alkimia.lcache"), "{message}");
    }

    #[test]
    fn the_requested_schema_stays_inside_the_subset_clients_must_support() {
        // Elicitation restricts schemas to a flat object of primitives. A nested one is not a
        // richer dialog, it is a dialog the client is entitled to fail to render.
        let params = elicitation_params(&consent());
        let schema = &params["requestedSchema"];
        assert_eq!(schema["type"], "object");

        let properties = schema["properties"].as_object().unwrap();
        assert_eq!(properties.len(), 1, "one question, one field");
        for (name, property) in properties {
            let kind = property["type"].as_str().unwrap();
            assert!(
                matches!(kind, "string" | "number" | "integer" | "boolean"),
                "{name} is a {kind}, which is outside the subset"
            );
            assert!(property.get("properties").is_none(), "{name} must not nest");
        }

        // Both options spelled out, and neither is a default that could be submitted untouched.
        let decision = &properties[DECISION_FIELD];
        assert_eq!(decision["enum"], json!([RUN, "cancel"]));
        assert_eq!(decision["enumNames"].as_array().unwrap().len(), 2);
        assert!(decision.get("default").is_none(), "a confirmation must not pre-answer itself");
        assert_eq!(schema["required"], json!([DECISION_FIELD]));
    }

    #[test]
    fn the_launch_flag_never_appears_without_the_write_flag() {
        // A game launch stages its result in the installation, so the launch flag alone would send
        // someone to restart with a flag set that still does not cover the call.
        assert_eq!(Needs { write: true, game_launch: true }.flags(), "--allow-game-launch --allow-write");
        assert_eq!(Needs { write: false, game_launch: true }.flags(), "--allow-game-launch --allow-write");
        assert_eq!(Needs { write: true, game_launch: false }.flags(), "--allow-write");
    }

    #[test]
    fn no_refusal_claims_a_person_decided() {
        // This one is load-bearing. The message used to open "the user was asked about this call
        // and said no", and Claude Code running non-interactively answers `decline` itself within
        // milliseconds without showing anything — so that sentence was simply false, and it sent
        // both the model and the reader off believing a decision had been made that never was.
        //
        // A server sees an answer arrive on a socket. It never sees who produced it, and no wording
        // here may pretend otherwise.
        let consent = consent();
        let claims = [
            "the user was asked",
            "the user said",
            "the user declined",
            "you said",
            "said no",
            "they chose",
        ];

        for decision in [
            Decision::Declined,
            Decision::Dismissed,
            Decision::NotAsked(Policy::CannotAsk),
            Decision::NotAsked(Policy::NeverAsk),
            Decision::Failed("broken pipe".into()),
        ] {
            let text = refusal(&consent, &decision);
            for claim in claims {
                assert!(
                    !text.contains(claim),
                    "{decision:?} asserts {claim:?}, which this server cannot know: {text}"
                );
            }
        }
    }

    #[test]
    fn each_refusal_says_what_to_do_next() {
        let consent = consent();

        // Both answers that came back from a client name the raw action, so the reader can tell an
        // explicit no from a dialog that was never shown, and both offer the way out — because
        // from here the two are indistinguishable.
        let declined = refusal(&consent, &Decision::Declined);
        assert!(declined.contains("`decline`"), "{declined}");
        assert!(declined.contains("ask the user"), "{declined}");
        assert!(declined.contains("gore mcp serve --allow-write"), "{declined}");

        let dismissed = refusal(&consent, &Decision::Dismissed);
        assert!(dismissed.contains("`cancel`"), "{dismissed}");
        assert!(dismissed.contains("gore mcp serve --allow-write"), "{dismissed}");

        for policy in [Policy::CannotAsk, Policy::NeverAsk] {
            let text = refusal(&consent, &Decision::NotAsked(policy));
            assert!(text.contains("gore mcp serve --allow-write"), "{policy:?}: {text}");
        }

        let failed = refusal(&consent, &Decision::Failed("broken pipe".into()));
        assert!(failed.contains("broken pipe"), "{failed}");
        assert!(failed.contains("nothing ran"), "{failed}");
    }

    #[test]
    fn a_dismissal_is_kept_apart_from_an_explicit_no() {
        // The specification gives them different meanings, and only one of them is what a client
        // sends when it could not put the question in front of anybody.
        let mut peer = StubPeer::answering(json!({ "action": "cancel" }));
        assert_eq!(decide(&consent(), Policy::Ask, None, &mut peer), Decision::Dismissed);

        let mut peer = StubPeer::answering(json!({ "action": "decline" }));
        assert_eq!(decide(&consent(), Policy::Ask, None, &mut peer), Decision::Declined);
    }

    #[test]
    fn every_refusal_carries_the_reason_and_the_remedy() {
        let consent = consent();
        for decision in [
            Decision::Declined,
            Decision::Dismissed,
            Decision::NotAsked(Policy::CannotAsk),
            Decision::NotAsked(Policy::NeverAsk),
            Decision::Failed("nope".into()),
        ] {
            let text = refusal(&consent, &decision);
            assert!(text.starts_with("refused:"), "{decision:?}: {text}");
            assert!(text.contains("skin.utoc"), "{decision:?}: {text}");
            assert!(text.contains("does not exist yet"), "{decision:?}: {text}");
        }
    }

    #[test]
    fn a_refusal_reads_as_prose_without_stray_indentation() {
        // Continued string literals in this file are one edit away from folding their indentation
        // into the message the model reads.
        let mut consent = consent();
        consent.remedy = None;
        for decision in [
            Decision::Declined,
            Decision::Dismissed,
            Decision::NotAsked(Policy::CannotAsk),
            Decision::NotAsked(Policy::NeverAsk),
            Decision::NotAsked(Policy::Ask),
            Decision::Failed("nope".into()),
            Decision::Allow,
        ] {
            let text = refusal(&consent, &decision);
            for line in text.lines() {
                // The one deliberate indent is the copy-pasteable command line.
                if line.trim_start().starts_with("gore mcp serve") {
                    continue;
                }
                assert!(!line.starts_with(' '), "{decision:?} indents {line:?}");
                assert!(!line.contains("  "), "{decision:?} double-spaces {line:?}");
            }
            assert!(!text.ends_with(char::is_whitespace), "{decision:?} ends in whitespace: {text}");
        }
    }
}
