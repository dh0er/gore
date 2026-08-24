//! AngelScript precompiled-cache decoder for Gothic 1 Remake.

pub mod cache;
pub mod compile;
pub mod compiler_backend;
pub mod compiler_profile;
pub mod compiler_target;
pub mod diagnostics;
pub mod full_graph_plan;
pub mod generation_receipt;
pub mod generation_receipt_v2;
pub mod standalone_package;
pub mod standalone_package_resolver;
pub mod standalone_sidecar;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_builds() {
        assert_eq!(2 + 2, 4);
    }
}
