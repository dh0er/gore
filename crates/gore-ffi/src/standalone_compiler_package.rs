//! Product-owned standalone-compiler package resolution for FFI compiler routes.
//!
//! Wire callers select only a backend policy. They never provide executable/profile paths or
//! seals. Package authority comes exclusively from gore-as' embedded catalog; the selected game
//! files are used only as physical target evidence. The FFI module path supplies package location
//! and never adds identity.

use std::path::{Path, PathBuf};

use gore_as::compile::CompilerBackendNameV1;
use gore_as::compiler_target::CompilerTargetInputPathsV1;
use gore_as::standalone_package_resolver::{
    resolve_embedded_product_standalone_compiler_package_for_inputs_v1,
    AvailableProductStandaloneCompilerPackageV1, ProductStandaloneCompilerPackageIdentityV1,
    ProductStandaloneCompilerPackageResolutionV1, ProductStandaloneCompilerPackageUnavailableV1,
};
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

#[derive(Debug)]
pub(super) enum ResolvedProductStandaloneCompilerV1 {
    BundleAbsent,
    Unavailable(ProductStandaloneCompilerPackageUnavailableV1),
    Available(ResolvedProductPackageV1),
}

/// An authenticated package together with the pristine source its Shipping target was validated
/// against, so the check after the pin can prove the pinned bytes are the selected original and
/// not merely whatever the pristine selection returns afterwards.
#[derive(Debug)]
pub(super) struct ResolvedProductPackageV1 {
    pub(super) package: AvailableProductStandaloneCompilerPackageV1,
    pub(super) pristine_source: Option<gore_mod::PristineScriptCacheSource>,
}

/// Resolve one product-authenticated compiler package for the exact selected installation.
///
/// The catalog is the only identity authority. `game_dir` contributes fixed EXE/Shipping/Binds
/// locations, and `current_ffi_module_path` contributes only the app-relative package root.
pub(super) fn resolve_product_standalone_compiler_for_game_v1(
    game_dir: &Path,
) -> Result<ResolvedProductStandaloneCompilerV1, String> {
    let host_module = current_ffi_module_path()?;
    let paths = compiler_target_paths_for_game_v1(game_dir);
    Ok(
        match resolve_embedded_product_standalone_compiler_package_for_inputs_v1(
            &host_module,
            CompilerTargetInputPathsV1 {
                executable: &paths.executable,
                shipping_cache: &paths.shipping_cache,
                binds_cache: &paths.binds_cache,
            },
        ) {
            ProductStandaloneCompilerPackageResolutionV1::BundleAbsent => {
                ResolvedProductStandaloneCompilerV1::BundleAbsent
            }
            ProductStandaloneCompilerPackageResolutionV1::Unavailable(reason) => {
                ResolvedProductStandaloneCompilerV1::Unavailable(reason)
            }
            ProductStandaloneCompilerPackageResolutionV1::Available(package) => {
                ResolvedProductStandaloneCompilerV1::Available(ResolvedProductPackageV1 {
                    package,
                    pristine_source: paths.pristine_source,
                })
            }
        },
    )
}

/// The physical target files of the selected installation.
///
/// The Shipping cache is the deployment-aware pristine source: while a script mod is installed
/// that is the deployment's `*.gore-bak`, so the installed mod stays in place and the compiler
/// still validates its target against the original the deployment preserved. When no source can
/// be selected (no cache, an interrupted deployment awaiting recovery) the live cache is named
/// instead, and the authoritative pristine read every route performs after pinning the target
/// reports the real problem under its own code.
pub(super) struct CompilerTargetPathsV1 {
    pub(super) executable: PathBuf,
    pub(super) shipping_cache: PathBuf,
    pub(super) binds_cache: PathBuf,
    pub(super) pristine_source: Option<gore_mod::PristineScriptCacheSource>,
}

pub(super) fn compiler_target_paths_for_game_v1(game_dir: &Path) -> CompilerTargetPathsV1 {
    let install_root = gore_mod::semantic_install_root(game_dir);
    let g1r = install_root.join("G1R");
    let executable = g1r
        .join("Binaries")
        .join("Win64")
        .join("G1R-Win64-Shipping.exe");
    let live_shipping_cache = g1r.join("Script").join("PrecompiledScript_Shipping.Cache");
    let binds_cache = g1r.join("Script").join("Binds.Cache");
    let pristine_source = gore_mod::pristine_script_cache_source(game_dir).ok();
    let shipping_cache = pristine_source
        .as_ref()
        .map(|source| source.path.clone())
        .unwrap_or(live_shipping_cache);
    CompilerTargetPathsV1 {
        executable,
        shipping_cache,
        binds_cache,
        pristine_source,
    }
}

pub(super) fn backend_evidence(
    requested: CompilerBackendWireV2,
    result_backend: Option<CompilerBackendNameV1>,
    standalone_attempted: bool,
    game_attempted: bool,
    fallback_reason: Option<Value>,
) -> Value {
    backend_evidence_with_package(
        requested,
        result_backend,
        standalone_attempted,
        game_attempted,
        None,
        fallback_reason,
    )
}

pub(super) fn backend_evidence_with_package(
    requested: CompilerBackendWireV2,
    result_backend: Option<CompilerBackendNameV1>,
    standalone_attempted: bool,
    game_attempted: bool,
    qualified_package: Option<&ProductStandaloneCompilerPackageIdentityV1>,
    fallback_reason: Option<Value>,
) -> Value {
    json!({
        "requested_mode": requested.as_str(),
        "result_backend": result_backend.map(CompilerBackendNameV1::as_str),
        "standalone_attempted": standalone_attempted,
        "game_attempted": game_attempted,
        "qualified_package": qualified_package,
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

pub(super) fn package_unavailable_fallback_reason(
    reason: &ProductStandaloneCompilerPackageUnavailableV1,
) -> Value {
    json!({
        "failed_backend": CompilerBackendNameV1::Standalone.as_str(),
        "failure_kind": "unavailable",
        "detail": format!("{:?}: {}", reason.kind(), reason.detail()),
    })
}

#[cfg(windows)]
fn current_ffi_module_path() -> Result<PathBuf, String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt as _;

    use windows_sys::Win32::Foundation::HMODULE;
    use windows_sys::Win32::System::LibraryLoader::{
        GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
        GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    };

    let mut module: HMODULE = std::ptr::null_mut();
    // SAFETY: FROM_ADDRESS interprets the second argument as an address inside the requested
    // module. The function item is permanently resident while this call executes, and
    // UNCHANGED_REFCOUNT avoids manufacturing a loader reference that would need releasing.
    let ok = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            current_ffi_module_path as *const () as *const u16,
            &mut module,
        )
    };
    if ok == 0 || module.is_null() {
        return Err(format!(
            "resolving the loaded gore-ffi module handle: {}",
            std::io::Error::last_os_error()
        ));
    }

    let mut capacity = 512usize;
    loop {
        let mut buffer = vec![0u16; capacity];
        // SAFETY: `module` was resolved above and the writable buffer has exactly `capacity`
        // UTF-16 elements.
        let len = unsafe { GetModuleFileNameW(module, buffer.as_mut_ptr(), capacity as u32) };
        if len == 0 {
            return Err(format!(
                "resolving the loaded gore-ffi module path: {}",
                std::io::Error::last_os_error()
            ));
        }
        if (len as usize) < capacity - 1 {
            buffer.truncate(len as usize);
            return Ok(PathBuf::from(OsString::from_wide(&buffer)));
        }
        capacity = capacity
            .checked_mul(2)
            .filter(|next| *next <= 32 * 1024)
            .ok_or_else(|| "loaded gore-ffi module path exceeds 32 KiB".to_owned())?;
    }
}

#[cfg(not(windows))]
fn current_ffi_module_path() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|error| format!("resolving the FFI host executable: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    /// Writes the on-disk state a script deployment leaves behind: the modded live cache, the
    /// pristine `*.gore-bak`, and a deploy record that authenticates both.
    fn install_script_mod_record(game: &Path, pristine: &[u8], deployed: &[u8]) -> PathBuf {
        let live = game.join("G1R/Script/PrecompiledScript_Shipping.Cache");
        let backup = PathBuf::from(format!("{}.gore-bak", live.display()));
        std::fs::write(&backup, pristine).unwrap();
        std::fs::write(&live, deployed).unwrap();
        let recorded_live = std::fs::canonicalize(&live).unwrap();
        let recorded_backup = std::fs::canonicalize(&backup).unwrap();
        let identity = |bytes: &[u8]| format!("sha256:{:x}", Sha256::digest(bytes));
        let mut record = gore_mod::DeployRecord {
            mod_name: "fixture".to_owned(),
            backups: vec![(
                recorded_live.display().to_string(),
                recorded_backup.display().to_string(),
                true,
            )],
            ..Default::default()
        };
        record
            .deployed_hashes
            .insert(recorded_live.display().to_string(), identity(deployed));
        record
            .backup_hashes
            .insert(recorded_backup.display().to_string(), identity(pristine));
        std::fs::write(
            game.join("gore-mod.deployed.json"),
            serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
        backup
    }

    /// The Shipping target follows the deployment-aware pristine source: the live cache while
    /// nothing is deployed, the deployment's backup while a script mod is installed, and the live
    /// cache again when the source cannot be selected at all (the authoritative pristine read
    /// after the pin then reports the real problem).
    #[test]
    fn target_paths_follow_the_deployment_aware_pristine_source() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("game");
        let script = game.join("G1R/Script");
        std::fs::create_dir_all(&script).unwrap();
        let live = script.join("PrecompiledScript_Shipping.Cache");

        let missing = compiler_target_paths_for_game_v1(&game);
        assert_eq!(missing.shipping_cache, live);
        assert!(missing.pristine_source.is_none());

        std::fs::write(&live, b"pristine").unwrap();
        let untouched = compiler_target_paths_for_game_v1(&game);
        assert_eq!(untouched.shipping_cache, live);
        assert!(!untouched.pristine_source.unwrap().from_backup);

        let backup = install_script_mod_record(&game, b"pristine", b"deployed");
        let installed = compiler_target_paths_for_game_v1(&game);
        assert_eq!(installed.shipping_cache, backup);
        assert!(installed.pristine_source.unwrap().from_backup);
        assert_eq!(
            installed.executable,
            game.join("G1R/Binaries/Win64/G1R-Win64-Shipping.exe")
        );
        assert_eq!(installed.binds_cache, script.join("Binds.Cache"));
    }

    #[test]
    fn empty_product_catalog_resolves_before_untrusted_target_paths() {
        assert!(matches!(
            resolve_product_standalone_compiler_for_game_v1(Path::new(
                "C:/definitely-not-a-game-install"
            )),
            Ok(ResolvedProductStandaloneCompilerV1::BundleAbsent)
        ));
    }
}
