//! Shared domain model for gore-tools modding utilities (gore-cli, gore-mod).
//!
//! Holds the UE reflection model (parsed from UE4SS SDK dumps), the item/npc/
//! knowledge catalog model + prefix→category mapping, the config→Lua mod
//! generation engine, and field-level validation. It deliberately does NOT
//! contain save-payload parsing — that stays in `goresave_core`.

pub mod catalog;
pub mod gen;
pub mod model;
pub mod parser;
pub mod validate;

pub use catalog::{category_for_id, item_category_from_id, CatalogJsonEntry, ItemCategory};
pub use model::ReflectionModel;
pub use gen::{gen_lua, OverridesConfig};
pub use validate::{validate_config, ValidationError};
