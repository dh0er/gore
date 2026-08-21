//! Offline decoder for the dormant, version-pinned G1R runtime capture stream.
//!
//! The native helper does not install hooks or inject itself.  A separately authorized bridge
//! must call it at the pinned observation points.  This module rejects partial, unsealed,
//! differently-versioned, pointer-bearing, or out-of-order streams before they can become a
//! compiler profile input.

mod decode;
mod model;

pub use decode::{decode_capture_v1, CaptureDecodeError};
pub use model::*;
