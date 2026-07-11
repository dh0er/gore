//! AngelScript precompiled-cache decoder for Gothic 1 Remake.

pub mod cache;
pub mod compile;
pub mod diagnostics;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_builds() {
        assert_eq!(2 + 2, 4);
    }
}
