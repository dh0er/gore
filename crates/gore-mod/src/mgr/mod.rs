//! Multi-mod manager engine on top of the single-bundle deploy machinery.
//!
//! The manager keeps a **library** of imported mods (each a normalized dir with a
//! [`model::ModEntryMeta`] sidecar), a **loadout** (which mods are enabled, in order), and
//! composes the enabled set into ONE deployment against the game. Submodules follow that
//! pipeline: [`model`] (cross-tool metadata contract), [`paths`] (shared on-disk layout),
//! [`loadout`] (enable/order state), then [`import`] → [`analyze`] → [`apply`] → [`status`].

pub mod analyze;
pub mod apply;
pub mod import;
pub mod loadout;
pub mod model;
pub mod paths;
pub mod status;

pub use loadout::{Loadout, LoadoutEntry};
pub use model::*;
