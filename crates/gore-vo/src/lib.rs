//! Safe primitives for inspecting, extracting, and rewriting G1R voice ZIP archives.
//!
//! Rewrites are deliberately copy-on-write: an input archive is never modified in place.
//! A candidate can be composed and verified entirely in memory, or written beside the requested
//! output path, reopened and verified, then atomically published without overwriting an existing
//! file.

mod archive;
mod error;
mod limits;
mod ogg;

pub use archive::{
    validate_archive_entry_path, ArchiveEdit, ArchiveEntry, ArchiveIndex, EditAction,
    RewriteReport, WriteReport,
};
pub use error::{Error, OggError, Result};
pub use limits::Limits;
pub use ogg::{validate_ogg, OggCodec, OggInfo};
