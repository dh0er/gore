//! Reconstruction and fail-closed source preparation for the game's dialog trees.
//!
//! A conversation is one AngelScript module: a root topic class naming its participants, a set
//! of topic classes below it, reconstructed class defaults carrying each option's caption and
//! rules, and
//! `Act_Implementation` bodies carrying the spoken lines, effects, and `Subdialog` calls that
//! nest one menu inside another. This module turns that into a [`model::DialogGraph`].
//!
//! Nothing here deploys or launches anything, and nothing here reports what the game would show
//! in a given save state: the tree is what the cache declares. Checkout/check preparation stays
//! offline and writes only through its CLI caller.

pub mod edit;
pub mod extract;
pub mod graph;
pub mod model;

pub use edit::{
    checkout, checkout_many, known_names, read_outline, verify, ChangedBody, ChangedDefault,
    Checkout, ClassOutline, DefaultOutline, EditReport, KnownNames, SourceOutline, Violation,
};
pub use graph::{build, DialogError};
pub use model::{
    Arg, Caption, Check, CheckSource, Conversation, Coverage, DialogGraph, Guard, Rule, RuleKind,
    Setting, Step, StepKind, Topic, TopicFlags, Visibility,
};
