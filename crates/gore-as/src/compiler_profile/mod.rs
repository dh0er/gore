//! Version-bound inputs needed to reproduce the embedded G1R AngelScript compiler.
//!
//! This module is deliberately independent from the decompiler's conservative native-symbol
//! hints. A compiler profile needs an exact, fail-closed representation of every serialized bind
//! record before dynamic registration data can be layered on top.

pub mod binds;
pub mod manifest;
