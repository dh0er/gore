//! Product-owned standalone-compiler package resolution for FFI compiler routes.
//!
//! Wire callers select only a backend policy. They never provide executable/profile paths or
//! seals. The current product has no embedded, package-authenticated bundle manifest, so the
//! production resolver deliberately reports `bundle_absent`. A later packaging lane may add an
//! available variant only when all sidecar/profile identities originate in an embedded manifest.

use gore_as::compile::CompilerBackendNameV1;
use serde::Deserialize;
use serde_json::{json, Value};

pub(super) const BUNDLE_ABSENT_DETAIL: &str =
    "this GORE build does not contain an authenticated standalone compiler bundle";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CompilerBackendWireV2 {
    Standalone,
    Game,
    StandaloneThenGame,
}

impl CompilerBackendWireV2 {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Standalone => "standalone",
            Self::Game => "game",
            Self::StandaloneThenGame => "standalone_then_game",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StandaloneCompilerPackageStatusV1 {
    BundleAbsent,
}

pub(super) fn resolve_product_standalone_compiler_v1() -> StandaloneCompilerPackageStatusV1 {
    // Do not probe environment variables, caller paths, or files adjacent to the FFI library.
    // None of those locations is package-authenticated. There is no trusted bundle in this build.
    StandaloneCompilerPackageStatusV1::BundleAbsent
}

pub(super) fn backend_evidence(
    requested: CompilerBackendWireV2,
    result_backend: Option<CompilerBackendNameV1>,
    standalone_attempted: bool,
    game_attempted: bool,
    fallback_reason: Option<Value>,
) -> Value {
    json!({
        "requested_mode": requested.as_str(),
        "result_backend": result_backend.map(CompilerBackendNameV1::as_str),
        "standalone_attempted": standalone_attempted,
        "game_attempted": game_attempted,
        "qualified_package": Value::Null,
        "fallback_reason": fallback_reason,
    })
}

pub(super) fn bundle_absent_fallback_reason() -> Value {
    json!({
        "failed_backend": CompilerBackendNameV1::Standalone.as_str(),
        "failure_kind": "unavailable",
        "detail": BUNDLE_ABSENT_DETAIL,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolver_never_derives_package_identity_from_process_state() {
        assert_eq!(
            resolve_product_standalone_compiler_v1(),
            StandaloneCompilerPackageStatusV1::BundleAbsent
        );
    }
}
