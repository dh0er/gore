//! Canonical, fail-closed observation of the complete precompiled-cache model.
//!
//! This module is intentionally independent of the source emitter and the historical
//! byte-difference oracle. It never treats unresolved runtime ids as comparable raw numbers and
//! never applies namespace-drift or alignment heuristics. Every serialized reference accepted by
//! the observer is either a documented null/primitive sentinel or resolves through the complete
//! seven-table reference graph.

mod model;
mod observe;

use thiserror::Error;

pub use observe::{
    observe_whole_cache_semantics_v1, CanonicalInvokeReturnV1, CanonicalInvokeValueV1,
    ObservedCacheModuleIdentityV1, ObservedCachePropertyIdentityV1,
    WholeCacheSemanticObservationV1,
};

/// Fail-closed errors returned by [`observe_whole_cache_semantics_v1`].
#[derive(Debug, Error)]
pub enum SemanticObserverError {
    #[error(transparent)]
    Wire(#[from] crate::cache::wire::WireError),
    #[error("resource limit exceeded for {resource}: {actual} > {limit}")]
    ResourceLimit {
        resource: &'static str,
        actual: usize,
        limit: usize,
    },
    #[error("allocation failed while decoding {resource}")]
    AllocationFailed { resource: &'static str },
    #[error("trailing cache bytes at offset {offset}: {remaining} bytes remain")]
    TrailingBytes { offset: usize, remaining: usize },
    #[error("duplicate {kind} key {key}")]
    DuplicateKey { kind: &'static str, key: String },
    #[error("ambiguous semantic identity in {kind}")]
    AmbiguousIdentity { kind: &'static str },
    #[error("unresolved {kind} reference {value:#x} in {context}")]
    UnresolvedReference {
        context: String,
        kind: &'static str,
        value: i64,
    },
    #[error(
        "legacy ByteCodeReferences is nonempty in {context}; no portable meaning is specified"
    )]
    UnsupportedByteCodeReferences { context: String },
    #[error("invalid bytecode in {context}: {detail}")]
    InvalidBytecode { context: String, detail: String },
    #[error("invalid cache structure in {context}: {detail}")]
    InvalidStructure {
        context: &'static str,
        detail: &'static str,
    },
    #[error("invalid invocation observation: {0}")]
    InvalidInvoke(String),
}

#[cfg(test)]
pub(crate) mod tests;
