//! Shared domain model for gore-tools modding utilities (gore-cli, gore-mod).
//!
//! This crate is the forward home for reflection/catalog logic consumed by the
//! modding front-ends. It deliberately does NOT contain save-payload parsing —
//! that stays in `goresave_core`. Today it holds the item catalog model and the
//! prefix→category mapping (ported from the gore-save Flutter `item_categories`
//! logic); the reflection model + config→Lua generation engine land here as
//! gore-cli is built (see docs `gore-cli-design.md`).

pub mod catalog;

pub use catalog::{item_category_from_id, CatalogEntry, ItemCategory};
