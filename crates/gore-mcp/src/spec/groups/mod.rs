//! The command tables, split by area so no single file grows past a few hundred lines.
//!
//! These are pure data. All behaviour lives in [`crate::schema`] and [`crate::argv`].

pub mod core;
pub mod deploy;
pub mod files;
pub mod script;
