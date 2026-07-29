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

use super::message::Request;

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
    let id = value.get("id").cloned().unwrap_or(Value::Null);
    match serde_json::from_value::<Request>(value) {
        Ok(request) if request.version_ok() => Frame::Message(Box::new(request)),
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
