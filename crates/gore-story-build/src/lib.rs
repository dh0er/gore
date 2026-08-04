//! Deterministic, non-publishing source inspection for revision-3 story content.
//!
//! The crate accepts only native revision-3 project state and deliberately stops before deployment
//! or runtime claims.

pub mod revision3_npc;
pub mod revision3_quest;

#[cfg(test)]
mod revision3_npc_tests;

#[cfg(test)]
mod revision3_quest_tests;
