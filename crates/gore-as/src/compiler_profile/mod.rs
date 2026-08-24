//! Version-bound inputs needed to reproduce the embedded G1R AngelScript compiler.
//!
//! This module is deliberately independent from the decompiler's conservative native-symbol
//! hints. A compiler profile needs an exact, fail-closed representation of every serialized bind
//! record before dynamic registration data can be layered on top.

pub mod binds;
pub mod capture;
pub mod embedded_qualification;
pub mod frontend;
pub mod manifest;
pub mod qualification;
pub mod qualification_runner;
pub mod qualification_suite;
pub mod registry;
pub mod standalone_qualification;
