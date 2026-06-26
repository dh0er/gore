//! Texture extraction/replacement for Gothic 1 Remake (UE5 IoStore).
pub mod container;
pub mod decode;
pub mod encode;
pub mod error;
pub mod paths;
pub mod texdata;

pub use error::{Result, TexError};
