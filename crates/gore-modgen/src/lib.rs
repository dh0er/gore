//! Config→Lua mod-generation engine + field-level validation for gore.

pub mod gen;
pub mod validate;

pub use gen::{gen_lua, OverridesConfig};
pub use validate::{validate_config, ValidationError};
