//! AngelScript precompiled-cache decoder for Gothic 1 Remake.
//!
//! See `docs/superpowers/specs/2026-06-20-gore-as-angelscript-decode-design.md`.

pub mod cache;
pub mod compile;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_builds() {
        assert_eq!(2 + 2, 4);
    }
}
