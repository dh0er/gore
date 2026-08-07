use std::path::PathBuf;

use crate::SourceFormat;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("voice archive source I/O failed at {path}: {source}")]
    SourceIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("voice extraction output I/O failed at {path}: {source}")]
    OutputIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("invalid archive payload for {path:?}: {source}")]
    ArchiveData {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("resource limit exceeded for {kind}: {actual} > {limit}")]
    LimitExceeded {
        kind: &'static str,
        actual: u64,
        limit: u64,
    },
    #[error("entry not found: {query:?}")]
    NotFound { query: String },
    #[error("entry selector {query:?} is ambiguous: {candidates:?}")]
    Ambiguous {
        query: String,
        candidates: Vec<String>,
    },
    #[error("unsafe archive path {path:?}: {reason}")]
    UnsafePath { path: String, reason: &'static str },
    #[error("unsafe voice archive source {path}: {reason}")]
    UnsafeSource { path: PathBuf, reason: &'static str },
    #[error("unsafe voice extraction output {path}: {reason}")]
    UnsafeOutput { path: PathBuf, reason: &'static str },
    #[error("archive contains an encrypted entry that cannot be processed safely: {0:?}")]
    EncryptedEntry(String),
    #[error("archive contains a symbolic-link entry that cannot be extracted safely: {0:?}")]
    SymlinkEntry(String),
    #[error("input and output refer to the same path: {0}")]
    InputOutputSame(PathBuf),
    #[error("output already exists (refusing to overwrite it): {0}")]
    OutputExists(PathBuf),
    #[error("archive entry already exists: {0:?}")]
    EntryAlreadyExists(String),
    #[error("edit batch is empty")]
    EmptyEditBatch,
    #[error("conflicting edits target the same case-insensitive path: {first:?} and {second:?}")]
    ConflictingEdits { first: String, second: String },
    #[error("voice archive edits require an .ogg entry, got {0:?}")]
    NotOggPath(String),
    #[error("unsupported compression method for replacement entry {path:?}: {method:?}")]
    UnsupportedCompression {
        path: String,
        method: zip::CompressionMethod,
    },
    #[error("archive changed after it was indexed")]
    ArchiveChanged,
    #[error("output verification failed: {0}")]
    Verification(String),
    #[error(transparent)]
    InvalidOgg(#[from] OggError),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OggError {
    #[error("Ogg stream is empty")]
    Empty,
    #[error("truncated Ogg page at byte {offset}")]
    Truncated { offset: usize },
    /// The payload is not an Ogg at all, as opposed to an Ogg that goes wrong partway through.
    ///
    /// Kept apart from [`Self::Capture`] because the two failures need different answers. A byte
    /// offset helps someone whose stream started as an Ogg; at byte zero it told a voice actor
    /// holding a WAV only that a term she had never met was missing.
    #[error(
        "{} — voice archives hold Ogg/Vorbis, the codec of every recording the game ships (mono, \
         48 kHz). Convert it first: '{}'",
        .format.describe(),
        .format.ffmpeg_command()
    )]
    NotOgg { format: SourceFormat },
    #[error("invalid Ogg capture pattern at byte {offset}")]
    Capture { offset: usize },
    #[error("unsupported Ogg page version {version} at byte {offset}")]
    Version { offset: usize, version: u8 },
    #[error("invalid Ogg header flags 0x{flags:02x} at byte {offset}")]
    HeaderFlags { offset: usize, flags: u8 },
    #[error("Ogg checksum mismatch at byte {offset}")]
    Checksum { offset: usize },
    #[error("logical stream {serial} does not begin with a valid BOS page")]
    MissingBos { serial: u32 },
    #[error("logical stream {serial} contains an unexpected second BOS page")]
    UnexpectedBos { serial: u32 },
    #[error("logical stream {serial} has unexpected page sequence {actual}, expected {expected}")]
    Sequence {
        serial: u32,
        actual: u32,
        expected: u32,
    },
    #[error("logical stream {serial} has an invalid continuation flag")]
    Continuation { serial: u32 },
    #[error("logical stream {serial} contains data after EOS")]
    AfterEos { serial: u32 },
    #[error("logical stream {serial} ends without an EOS page")]
    MissingEos { serial: u32 },
    #[error("logical stream {serial} ends with an incomplete packet")]
    IncompletePacket { serial: u32 },
    #[error("Ogg identification packet is malformed: {0}")]
    Identification(&'static str),
    #[error("recognized audio logical stream {serial} is incomplete or malformed: {reason}")]
    AudioStructure { serial: u32, reason: &'static str },
    #[error("multiple recognized audio logical streams are not supported")]
    MultipleAudioStreams,
    #[error("Ogg resource limit exceeded for {kind}: {actual} > {limit}")]
    LimitExceeded {
        kind: &'static str,
        actual: usize,
        limit: usize,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
