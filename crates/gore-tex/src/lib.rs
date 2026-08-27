//! Texture extraction/replacement for Gothic 1 Remake (UE5 IoStore).
pub mod container;
pub mod decode;
pub mod encode;
pub mod error;
pub mod index;
pub mod installed_package_index;
pub mod item_icons;
pub mod package_index;
pub mod paths;
pub mod texdata;
pub mod vt;

pub use error::{Result, TexError};
