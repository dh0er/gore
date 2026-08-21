//! AngelScript precompiled-cache decoder for Gothic 1 Remake.

pub mod cache;
pub mod compile;
pub mod compiler_backend;
pub mod compiler_profile;
pub mod diagnostics;
pub mod standalone_sidecar;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_builds() {
        assert_eq!(2 + 2, 4);
    }
}
