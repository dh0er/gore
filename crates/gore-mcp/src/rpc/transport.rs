//! Newline-delimited framing for the MCP stdio transport.
//!
//! The stdio transport carries one JSON-RPC message per line. That works because `serde_json`
//! escapes every control character, so a serialized value can never contain a literal newline —
//! one message is always exactly one physical line.
//!
//! `Transport` is generic over `BufRead`/`Write` rather than reaching for `std::io::stdin()`
//! directly. That is not merely for testability: it is the structural guarantee that this crate
//! can never write anything to the real stdout except protocol messages. The only place a true
//! stdout handle exists is the thin `gore mcp serve` wrapper in the CLI crate.

use std::io::{self, BufRead, Read, Write};

use serde::Serialize;
use serde_json::Value;

use super::message::{Request, JSONRPC_VERSION};

/// Upper bound on a single incoming line.
///
/// Nothing an MCP client legitimately sends comes close; the cap exists so a peer that never emits
/// a newline cannot make us buffer without limit.
pub const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;

/// Chunk size used when discarding the tail of an oversized line.
const DRAIN_CHUNK_BYTES: u64 = 64 * 1024;

/// One unit read off the wire.
#[derive(Debug)]
pub enum Frame {
    /// A well-formed JSON-RPC request or notification.
    Message(Box<Request>),
    /// Valid JSON, but not a JSON-RPC request object. The id is echoed back when we could find one.
    Invalid { id: Value, reason: String },
    /// Not valid JSON.
    Malformed { reason: String },
    /// Longer than [`MAX_FRAME_BYTES`]; the remainder of the line was discarded.
    Oversized,
    /// A JSON-RPC batch: an array of requests and notifications in one frame.
    ///
    /// MCP revisions before `2025-06-18` permit these, and this server still negotiates those
    /// revisions, so a client is entitled to send one. Elements are parsed individually — a batch
    /// with one malformed member still answers the rest.
    Batch(Vec<Frame>),
    /// The client answering a request *this server* sent, correlated by `id`.
    ///
    /// Exactly one of `result` / `error` is populated by a well-behaved peer; both are carried as
    /// they arrived so the waiter decides what a malformed pair means. An answer is never replied
    /// to — JSON-RPC has no response-to-a-response — so an unsolicited one is simply dropped.
    ///
    /// `version_ok` is false when the frame carried a `jsonrpc` member that is not `"2.0"`. The
    /// verdict rides along rather than turning the frame into an `Invalid` one, because rejecting
    /// it there is exactly what would put a reply to a reply on the wire. The waiter fails its
    /// question closed instead — and the only thing ever waiting on an answer is a consent
    /// question, where failing closed is a refusal.
    Answer { id: Value, result: Option<Value>, error: Option<Value>, version_ok: bool },
}

pub struct Transport<R: BufRead, W: Write> {
    reader: R,
    writer: W,
    buf: Vec<u8>,
}

impl<R: BufRead, W: Write> Transport<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self { reader, writer, buf: Vec::with_capacity(8 * 1024) }
    }

    /// Read the next frame.
    ///
    /// Returns `Ok(None)` at clean end of input, which for stdio is the ordinary shutdown signal:
    /// the client closes our stdin and waits for us to exit. Blank lines are skipped rather than
    /// reported, since some clients pad the stream.
    pub fn read_frame(&mut self) -> io::Result<Option<Frame>> {
        loop {
            self.buf.clear();
            // Read one byte past the cap so that hitting it is distinguishable from a line that is
            // exactly MAX_FRAME_BYTES long.
            let read = {
                let mut limited = (&mut self.reader).take(MAX_FRAME_BYTES as u64 + 1);
                limited.read_until(b'\n', &mut self.buf)?
            };
            if read == 0 {
                return Ok(None);
            }

            if self.buf.len() > MAX_FRAME_BYTES {
                // We stopped mid-line. Throw away the rest so the next read starts on a boundary.
                if self.buf.last() != Some(&b'\n') {
                    self.drain_to_newline()?;
                }
                return Ok(Some(Frame::Oversized));
            }

            let line = trim_frame(&self.buf);
            if line.is_empty() {
                continue;
            }
            return Ok(Some(parse_frame(line)));
        }
    }

    /// Write one message followed by a newline, then flush.
    ///
    /// Flushing every message is mandatory, not defensive: the peer is blocked waiting on our
    /// stdout, and a buffered response is an apparent hang.
    pub fn write_message(&mut self, value: &impl Serialize) -> io::Result<()> {
        let mut line = serde_json::to_vec(value).map_err(io::Error::other)?;
        line.push(b'\n');
        self.writer.write_all(&line)?;
        self.writer.flush()
    }

    fn drain_to_newline(&mut self) -> io::Result<()> {
        let mut scratch = Vec::with_capacity(DRAIN_CHUNK_BYTES as usize);
        loop {
            scratch.clear();
            let read = {
                let mut limited = (&mut self.reader).take(DRAIN_CHUNK_BYTES);
                limited.read_until(b'\n', &mut scratch)?
            };
            if read == 0 || scratch.last() == Some(&b'\n') {
                return Ok(());
            }
        }
    }
}

fn trim_frame(raw: &[u8]) -> &[u8] {
    let mut end = raw.len();
    while end > 0 && (raw[end - 1] == b'\n' || raw[end - 1] == b'\r') {
        end -= 1;
    }
    let mut start = 0;
    while start < end && (raw[start] == b' ' || raw[start] == b'\t') {
        start += 1;
    }
    &raw[start..end]
}

/// Two-stage parse so that "not JSON" and "not a JSON-RPC request" map to different error codes,
/// and so that a structurally invalid request can still have its id echoed back.
fn parse_frame(line: &[u8]) -> Frame {
    let value: Value = match serde_json::from_slice(line) {
        Ok(value) => value,
        Err(error) => return Frame::Malformed { reason: error.to_string() },
    };
    if let Value::Array(items) = value {
        if items.is_empty() {
            // JSON-RPC 2.0 calls an empty batch an invalid request rather than an empty answer.
            return Frame::Invalid {
                id: Value::Null,
                reason: "an empty batch is not a request".into(),
            };
        }
        // One level only: an array nested inside a batch is not a request, and `parse_object`
        // reports it as such instead of recursing.
        return Frame::Batch(items.into_iter().map(parse_object).collect());
    }
    parse_object(value)
}

fn parse_object(value: Value) -> Frame {
    let id = value.get("id").cloned().unwrap_or(Value::Null);
    // Read before deserializing: `Option<Value>` collapses an omitted `id` and `"id": null` into
    // the same `None`, and only the first of those is a notification.
    let id_present = value.get("id").is_some();
    // An object, array or boolean id is not a shape JSON-RPC allows. Rejecting it here, before the
    // session sees it, is what keeps a bad id from running the call and then answering with
    // something the client cannot correlate — and therefore may retry.
    if let Some(given) = value.get("id") {
        if !Request::id_shape_ok(given) {
            return Frame::Invalid {
                id: Value::Null,
                reason: "`id` must be a string, a number, or null".into(),
            };
        }
    }
    // No `method`, but a `result` or an `error`: the client is answering something we asked, not
    // sending a broken request. Checked before the `Request` parse, which would reject it for the
    // missing `method` and report it as invalid — and replying with an error to a response is a
    // frame the peer cannot correlate to anything.
    //
    // A wrong `jsonrpc` cannot make this an `Invalid` frame — that earns an error *reply*, and a
    // reply to a reply is a frame the peer cannot correlate to anything. It must not be waved
    // through either: the one thing ever waiting on an answer is a consent question, and a peer
    // that changes protocol version from frame to frame is not one whose "run it" this server
    // should act on. So the verdict travels with the frame and the waiter decides.
    if value.get("method").is_none() {
        let result = value.get("result").cloned();
        let error = value.get("error").cloned();
        if result.is_some() || error.is_some() {
            let version_ok = value
                .get("jsonrpc")
                .map_or(true, |version| version.as_str() == Some(JSONRPC_VERSION));
            return Frame::Answer { id, result, error, version_ok };
        }
    }
    match serde_json::from_value::<Request>(value) {
        Ok(mut request) if request.version_ok() => {
            request.id_present = id_present;
            Frame::Message(Box::new(request))
        }
        Ok(_) => Frame::Invalid { id, reason: "`jsonrpc` must be \"2.0\"".into() },
        Err(error) => Frame::Invalid { id, reason: error.to_string() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn transport(input: &str) -> Transport<Cursor<Vec<u8>>, Vec<u8>> {
        Transport::new(Cursor::new(input.as_bytes().to_vec()), Vec::new())
    }

    #[test]
    fn an_id_that_is_not_a_string_number_or_null_is_an_invalid_request() {
        // JSON-RPC allows only those three. Dispatching an object id would run the call and answer
        // with something the client cannot correlate — and may therefore send again.
        for id in ["{}", "[]", "true", "[1,2]"] {
            let line = format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"ping\"}}");
            match parse_frame(line.as_bytes()) {
                Frame::Invalid { reason, .. } => {
                    assert!(reason.contains("`id`"), "{id}: {reason}")
                }
                other => panic!("{id} should be invalid, got {other:?}"),
            }
        }

        for id in ["1", "\"abc\"", "null"] {
            let line = format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"ping\"}}");
            assert!(
                matches!(parse_frame(line.as_bytes()), Frame::Message(_)),
                "{id} is a valid id"
            );
        }
    }

    #[test]
    fn reads_one_message_per_line() {
        let mut t = transport("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n");

        match t.read_frame().unwrap() {
            Some(Frame::Message(request)) => assert_eq!(request.method, "ping"),
            other => panic!("expected a message, got {other:?}"),
        }
        match t.read_frame().unwrap() {
            Some(Frame::Message(request)) => assert!(request.is_notification()),
            other => panic!("expected a notification, got {other:?}"),
        }
        assert!(t.read_frame().unwrap().is_none());
    }

    #[test]
    fn blank_and_whitespace_lines_are_skipped() {
        let mut t = transport("\n\n   \n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n");
        match t.read_frame().unwrap() {
            Some(Frame::Message(request)) => assert_eq!(request.method, "ping"),
            other => panic!("expected a message, got {other:?}"),
        }
    }

    #[test]
    fn a_line_without_a_trailing_newline_still_parses() {
        let mut t = transport("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}");
        assert!(matches!(t.read_frame().unwrap(), Some(Frame::Message(_))));
        assert!(t.read_frame().unwrap().is_none());
    }

    #[test]
    fn carriage_returns_are_tolerated() {
        let mut t = transport("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\r\n");
        assert!(matches!(t.read_frame().unwrap(), Some(Frame::Message(_))));
    }

    #[test]
    fn non_json_is_malformed_but_valid_json_that_is_not_a_request_is_invalid() {
        let mut t = transport("not json at all\n{\"jsonrpc\":\"2.0\",\"id\":9}\n");
        assert!(matches!(t.read_frame().unwrap(), Some(Frame::Malformed { .. })));
        match t.read_frame().unwrap() {
            // No `method`, so it is not a request — but the id is still recoverable.
            Some(Frame::Invalid { id, .. }) => assert_eq!(id, Value::from(9)),
            other => panic!("expected an invalid request, got {other:?}"),
        }
    }

    #[test]
    fn a_frame_with_no_method_but_a_result_or_an_error_is_an_answer() {
        // This is how the client replies to an `elicitation/create` we sent. Reported as an invalid
        // request it would earn an error reply, and a reply to a reply is a frame the peer has
        // nothing to match it against.
        let ok = "{\"jsonrpc\":\"2.0\",\"id\":\"gore-consent-1\",\"result\":{\"action\":\"accept\"}}";
        match parse_frame(ok.as_bytes()) {
            Frame::Answer { id, result, error, version_ok } => {
                assert_eq!(id, Value::from("gore-consent-1"));
                assert_eq!(result.unwrap()["action"], "accept");
                assert!(error.is_none());
                assert!(version_ok);
            }
            other => panic!("expected an answer, got {other:?}"),
        }

        let failed = "{\"jsonrpc\":\"2.0\",\"id\":4,\"error\":{\"code\":-32601,\"message\":\"no\"}}";
        match parse_frame(failed.as_bytes()) {
            Frame::Answer { id, result, error, version_ok } => {
                assert_eq!(id, Value::from(4));
                assert!(result.is_none());
                assert_eq!(error.unwrap()["code"], -32601);
                assert!(version_ok);
            }
            other => panic!("expected an answer, got {other:?}"),
        }
    }

    #[test]
    fn an_answer_carries_the_verdict_on_its_own_protocol_version() {
        // It stays an answer rather than becoming `Invalid`, because rejecting it there would put
        // an error *reply* to a reply on the wire. The verdict travels instead, and the one place
        // that waits on an answer — the consent question — fails closed on it.
        let wrong = "{\"jsonrpc\":\"1.0\",\"id\":\"gore-consent-1\",\"result\":{\"action\":\"accept\"}}";
        match parse_frame(wrong.as_bytes()) {
            Frame::Answer { version_ok, result, .. } => {
                assert!(!version_ok, "1.0 is not a version this server speaks");
                assert!(result.is_some(), "and the payload still arrives, for the waiter to refuse");
            }
            other => panic!("expected an answer, got {other:?}"),
        }

        // Omitted is tolerated for the same reason a request may omit it: rejecting on pedantry
        // costs interoperability, and only a *wrong* value says the peer speaks something else.
        let absent = "{\"id\":\"gore-consent-1\",\"result\":{\"action\":\"accept\"}}";
        match parse_frame(absent.as_bytes()) {
            Frame::Answer { version_ok, .. } => assert!(version_ok),
            other => panic!("expected an answer, got {other:?}"),
        }
    }

    #[test]
    fn a_method_makes_it_a_request_even_alongside_a_result() {
        // `method` is what distinguishes the two directions. A frame carrying both is malformed
        // either way, but reading it as a request is the one that cannot swallow a real call.
        let line = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\",\"result\":{}}";
        match parse_frame(line.as_bytes()) {
            Frame::Message(request) => assert_eq!(request.method, "ping"),
            other => panic!("expected a request, got {other:?}"),
        }
    }

    #[test]
    fn a_wrong_jsonrpc_version_is_invalid() {
        let mut t = transport("{\"jsonrpc\":\"1.0\",\"id\":3,\"method\":\"ping\"}\n");
        match t.read_frame().unwrap() {
            Some(Frame::Invalid { id, .. }) => assert_eq!(id, Value::from(3)),
            other => panic!("expected an invalid request, got {other:?}"),
        }
    }

    #[test]
    fn an_oversized_line_is_reported_without_buffering_it_and_the_next_line_still_parses() {
        let mut input = String::from("{\"a\":\"");
        input.push_str(&"x".repeat(MAX_FRAME_BYTES + 1024));
        input.push_str("\"}\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n");

        let mut t = transport(&input);
        assert!(matches!(t.read_frame().unwrap(), Some(Frame::Oversized)));
        match t.read_frame().unwrap() {
            Some(Frame::Message(request)) => assert_eq!(request.method, "ping"),
            other => panic!("expected recovery on the next line, got {other:?}"),
        }
    }

    #[test]
    fn a_written_message_is_exactly_one_line_even_with_embedded_newlines() {
        let mut t = transport("");
        t.write_message(&serde_json::json!({ "text": "first\nsecond" })).unwrap();
        let written = String::from_utf8(t.writer.clone()).unwrap();

        assert!(written.ends_with('\n'));
        assert_eq!(written.matches('\n').count(), 1, "embedded newlines must be escaped: {written:?}");
    }
}
