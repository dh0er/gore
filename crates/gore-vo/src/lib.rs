//! Safe primitives for inspecting, extracting, and rewriting G1R voice ZIP archives.
//!
//! Rewrites are deliberately copy-on-write: an input archive is never modified in place.
//! A candidate can be composed and verified entirely in memory, or written beside the requested
//! output path, reopened and verified, then atomically published without overwriting an existing
//! file.
//!
//! Source paths are opened without following links and copied, with a size bound and SHA-256 seal,
//! into a private temporary snapshot before any ZIP metadata or payload is parsed. Later extraction
//! and rewrite calls repeat that disk-backed snapshot and require the exact stored seal, preventing
//! in-place source mutation from changing bytes after validation. This deliberately costs one full
//! bounded source read per snapshot and uses no archive-sized memory buffer.

mod archive;
mod error;
mod limits;
mod ogg;

pub use archive::{
    validate_archive_entry_path, validate_output_root_ancestors, ArchiveEdit, ArchiveEntry,
    ArchiveIndex, ArchiveSeal, EditAction, RewriteReport, WriteReport,
};
pub use error::{Error, OggError, Result};
pub use limits::Limits;
pub use ogg::{validate_ogg, OggCodec, OggInfo};
