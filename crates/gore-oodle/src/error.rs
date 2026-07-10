//! Error + effort types for the Kraken codec.

/// Encoder effort. An enum, not a raw int — no invalid level is representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Fastest,
    Fast,
    Default,
    Max,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Input ended before a complete stream/quantum was read.
    Truncated,
    /// Structurally invalid stream (bad codec id, sub-stream size, checksum, …).
    Corrupt(&'static str),
    /// A required output (or internal) buffer size cannot be allocated.
    OutputTooLarge(usize),
    /// Input exceeds an internal encoder limit.
    InputTooLarge(usize),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Truncated => write!(f, "input truncated"),
            Error::Corrupt(why) => write!(f, "corrupt stream: {why}"),
            Error::OutputTooLarge(n) => write!(f, "output size unallocatable: {n} bytes"),
            Error::InputTooLarge(n) => write!(f, "input too large: {n} bytes"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

pub type Result<T> = core::result::Result<T, Error>;
