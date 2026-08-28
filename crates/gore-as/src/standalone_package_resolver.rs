//! Handle-pinned resolution of the product-owned standalone compiler package.
//!
//! Package paths are never accepted from wire data, environment variables, or the process CWD.
//! A caller supplies only the absolute host-module location; the package root is the fixed
//! app-local `compiler` sibling. That location is not authority: exact embedded catalog bytes are
//! the sole allow-list for the sidecar, profile manifest, protocol, and API identity. Installed
//! game inputs are matched by their parsed compiler compatibility, not by Steam/GOG file hashes.

#[cfg(windows)]
use std::collections::BTreeMap;
use std::fs::File;
#[cfg(windows)]
use std::io::{Read, Seek, SeekFrom};
#[cfg(windows)]
use std::path::Component;
use std::path::{Path, PathBuf};

use serde::Serialize;
#[cfg(windows)]
use sha2::{Digest as _, Sha256};

use crate::compiler_profile::manifest::Sha256Digest;
#[cfg(windows)]
use crate::compiler_profile::manifest::{
    CompilerProfileV1, SealedBlobV1, MAX_COMPILER_PROFILE_JSON_BYTES,
};
use crate::compiler_target::{
    CompilerTargetInputError, CompilerTargetInputPathsV1, ValidatedCompilerTargetInputsV1,
};
#[cfg(windows)]
use crate::standalone_package::PRODUCT_STANDALONE_COMPILER_PACKAGE_ROOT_V1;
use crate::standalone_package::{
    embedded_product_standalone_compiler_catalog_v1, product_standalone_compiler_catalog_sha256_v1,
    ProductStandaloneCompilerCatalogError, ProductStandaloneCompilerCatalogV1,
    ProductStandaloneCompilerTargetV1,
};
use crate::standalone_sidecar::{SidecarExecutableSealV1, ValidatedCompilerProfilePackageV1};

#[cfg(windows)]
const MAX_PROFILE_BLOB_BYTES_V1: u64 = 512 * 1024 * 1024;
#[cfg(windows)]
const MAX_PROFILE_AGGREGATE_BYTES_V1: u64 = 1024 * 1024 * 1024;

/// Receipt-safe identity emitted only after the complete product authority chain passed.
///
/// Fields are private and this type has no deserializer or public constructor. Consequently wire
/// data or a merely self-consistent local profile cannot mint product-qualified evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProductStandaloneCompilerPackageIdentityV1 {
    catalog_sha256: Sha256Digest,
    sidecar_byte_len: u64,
    sidecar_sha256: Sha256Digest,
    compatibility_id: String,
    request_version: u32,
    response_version: u32,
    manifest_byte_len: u64,
    manifest_sha256: Sha256Digest,
    profile_sha256: Sha256Digest,
    target: ProductStandaloneCompilerTargetV1,
}

impl ProductStandaloneCompilerPackageIdentityV1 {
    pub fn catalog_sha256(&self) -> Sha256Digest {
        self.catalog_sha256
    }

    pub fn sidecar_byte_len(&self) -> u64 {
        self.sidecar_byte_len
    }

    pub fn sidecar_sha256(&self) -> Sha256Digest {
        self.sidecar_sha256
    }

    pub fn compatibility_id(&self) -> &str {
        &self.compatibility_id
    }

    pub fn request_version(&self) -> u32 {
        self.request_version
    }

    pub fn response_version(&self) -> u32 {
        self.response_version
    }

    pub fn manifest_byte_len(&self) -> u64 {
        self.manifest_byte_len
    }

    pub fn manifest_sha256(&self) -> Sha256Digest {
        self.manifest_sha256
    }

    pub fn profile_sha256(&self) -> Sha256Digest {
        self.profile_sha256
    }

    pub fn target(&self) -> &ProductStandaloneCompilerTargetV1 {
        &self.target
    }
}

/// Non-forgeable product authority retained while target handles move through compilation.
///
/// This value keeps the sidecar, manifest, every profile blob, and their directory chains pinned.
/// Its identity also records that the resolver successfully constructed the shared target proof,
/// but it deliberately does not claim that EXE/Shipping/Binds handles are still stored here: those
/// handles are transferred to the compile transaction by `into_execution_parts`. Receipt code must
/// use this authority only after the compile API has confirmed its report/restore outcome.
#[derive(Debug)]
pub struct ProductStandaloneCompilerReceiptAuthorityV1 {
    identity: ProductStandaloneCompilerPackageIdentityV1,
    shipping_cache_seal: (u64, Sha256Digest),
    binds_cache_seal: (u64, Sha256Digest),
    sidecar_path: PathBuf,
    profile_manifest_path: PathBuf,
    profile_root: PathBuf,
    sidecar: File,
    profile_package: ValidatedCompilerProfilePackageV1,
    _profile_files: Vec<File>,
    _package_directory_pins: Vec<File>,
}

impl ProductStandaloneCompilerReceiptAuthorityV1 {
    pub fn identity(&self) -> &ProductStandaloneCompilerPackageIdentityV1 {
        &self.identity
    }

    pub fn profile_package(&self) -> &ValidatedCompilerProfilePackageV1 {
        &self.profile_package
    }

    pub(crate) fn shipping_cache_seal(&self) -> (u64, Sha256Digest) {
        self.shipping_cache_seal
    }

    pub(crate) fn binds_cache_seal(&self) -> (u64, Sha256Digest) {
        self.binds_cache_seal
    }

    /// Exact pinned executable handle. Prefer passing this handle to execution over reopening its
    /// path once the sidecar adapter grows a handle-native constructor.
    pub fn sidecar_handle(&self) -> &File {
        &self.sidecar
    }

    /// Location only; never authority. Kept for the existing path-based sidecar adapter.
    pub fn sidecar_path(&self) -> &Path {
        &self.sidecar_path
    }

    /// Location only; the exact bytes are authenticated and pinned by this value.
    pub fn profile_manifest_path(&self) -> &Path {
        &self.profile_manifest_path
    }

    /// Location only; every manifest-referenced blob is separately authenticated and pinned.
    pub fn profile_root(&self) -> &Path {
        &self.profile_root
    }

    pub fn sidecar_seal(&self) -> SidecarExecutableSealV1 {
        SidecarExecutableSealV1 {
            byte_len: self.identity.sidecar_byte_len,
            sha256: self.identity.sidecar_sha256,
        }
    }
}

/// Product-authoritative package before its target handles are transferred to compilation.
///
/// Keep this value whole until the attempt starts. [`Self::into_execution_parts`] is the sole API
/// that moves out the target proof while retaining non-forgeable receipt/package authority.
#[derive(Debug)]
pub struct AvailableProductStandaloneCompilerPackageV1 {
    authority: ProductStandaloneCompilerReceiptAuthorityV1,
    target_inputs: ValidatedCompilerTargetInputsV1,
}

impl AvailableProductStandaloneCompilerPackageV1 {
    pub fn identity(&self) -> &ProductStandaloneCompilerPackageIdentityV1 {
        self.authority.identity()
    }

    pub fn profile_package(&self) -> &ValidatedCompilerProfilePackageV1 {
        self.authority.profile_package()
    }

    pub fn target_inputs(&self) -> &ValidatedCompilerTargetInputsV1 {
        &self.target_inputs
    }

    pub fn sidecar_handle(&self) -> &File {
        self.authority.sidecar_handle()
    }

    pub fn sidecar_path(&self) -> &Path {
        self.authority.sidecar_path()
    }

    pub fn profile_manifest_path(&self) -> &Path {
        self.authority.profile_manifest_path()
    }

    pub fn profile_root(&self) -> &Path {
        self.authority.profile_root()
    }

    pub fn sidecar_seal(&self) -> SidecarExecutableSealV1 {
        self.authority.sidecar_seal()
    }

    pub fn receipt_authority(&self) -> &ProductStandaloneCompilerReceiptAuthorityV1 {
        &self.authority
    }

    /// Construct the sidecar configuration while preserving the resolver's exact compatible
    /// Shipping/Binds target seals. This is the only product path that relaxes the qualification
    /// oracle's byte identity; development configurations remain byte-exact.
    pub fn sidecar_config(
        &self,
        scratch_root: PathBuf,
    ) -> crate::standalone_sidecar::StandaloneSidecarConfigV1 {
        crate::standalone_sidecar::StandaloneSidecarConfigV1::new(
            self.sidecar_path().to_path_buf(),
            self.sidecar_seal(),
            self.profile_manifest_path().to_path_buf(),
            self.profile_root().to_path_buf(),
            scratch_root,
        )
        .with_product_target_inputs(
            self.target_inputs.shipping_cache(),
            self.target_inputs.binds_cache(),
            self.target_inputs.shipping_cache_path(),
            self.target_inputs.binds_cache_path(),
        )
    }

    /// Build the product runner from the package state the resolver already authenticated.
    /// This avoids parsing the same large profile payloads a second time before every compile.
    pub fn sidecar_runner(
        &self,
        scratch_root: PathBuf,
    ) -> Result<
        crate::standalone_sidecar::StandaloneSidecarRunnerV1,
        crate::compiler_backend::CompilerBackendFailureV1,
    > {
        crate::standalone_sidecar::StandaloneSidecarRunnerV1::new_product(
            self.sidecar_config(scratch_root),
            self.profile_package(),
        )
    }

    /// Transfer the exact EXE/Shipping/Binds proof to the compile transaction without dropping the
    /// package handles or the non-forgeable authority needed by Receipt V2 afterwards.
    pub fn into_execution_parts(
        self,
    ) -> (
        ProductStandaloneCompilerReceiptAuthorityV1,
        ValidatedCompilerTargetInputsV1,
    ) {
        (self.authority, self.target_inputs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductStandaloneCompilerPackageUnavailableKindV1 {
    UnsupportedPlatform,
    InvalidEmbeddedCatalog,
    UnsupportedTarget,
    UnsafeHostLocation,
    UnsafePackagePath,
    SidecarAuthentication,
    ProfileManifestAuthentication,
    ProfilePayloadAuthentication,
    QualificationIdentityMismatch,
    TargetInputsUnavailable,
    TargetAuthentication,
    AmbiguousTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProductStandaloneCompilerPackageUnavailableV1 {
    kind: ProductStandaloneCompilerPackageUnavailableKindV1,
    detail: String,
}

impl ProductStandaloneCompilerPackageUnavailableV1 {
    pub fn kind(&self) -> ProductStandaloneCompilerPackageUnavailableKindV1 {
        self.kind
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

/// Honest product state. `BundleAbsent` is intentionally distinct from a corrupt or unavailable
/// catalogued package and remains the production result until a real profile is qualified.
#[derive(Debug)]
pub enum ProductStandaloneCompilerPackageResolutionV1 {
    BundleAbsent,
    Unavailable(ProductStandaloneCompilerPackageUnavailableV1),
    Available(AvailableProductStandaloneCompilerPackageV1),
}

/// Resolve the embedded product bundle beside an absolute host module.
///
/// `host_module_path` supplies location only. Its parent plus the fixed `compiler` component is
/// used; no environment variable, current directory, wire path, or neighboring manifest can add
/// authority. `target` is still constrained to one exact embedded-catalog entry, and all three
/// target artifacts must pass the distribution-neutral [`ValidatedCompilerTargetInputsV1`]
/// compatibility checks.
pub fn resolve_embedded_product_standalone_compiler_package_v1(
    host_module_path: &Path,
    target: &ProductStandaloneCompilerTargetV1,
    target_paths: CompilerTargetInputPathsV1<'_>,
) -> ProductStandaloneCompilerPackageResolutionV1 {
    let catalog = match embedded_product_standalone_compiler_catalog_v1() {
        Ok(None) => return ProductStandaloneCompilerPackageResolutionV1::BundleAbsent,
        Ok(Some(catalog)) => catalog,
        Err(error) => {
            return unavailable(
                ProductStandaloneCompilerPackageUnavailableKindV1::InvalidEmbeddedCatalog,
                error,
            );
        }
    };
    resolve_authoritative_catalog_at_host_v1(
        &catalog,
        product_standalone_compiler_catalog_sha256_v1(
            crate::standalone_package::EMBEDDED_PRODUCT_STANDALONE_COMPILER_CATALOG_JSON_V1,
        ),
        host_module_path,
        target,
        target_paths,
    )
}

/// Resolve a product bundle from physical target inputs without trusting a caller-supplied target
/// tuple.
///
/// Every candidate comes exclusively from the validated embedded catalog. The real
/// EXE/Shipping/Binds handles must identify exactly one candidate. Zero matches are reported as an
/// unsupported target; multiple matches fail closed because Steam/depot metadata cannot be
/// inferred from file paths or wire data.
pub fn resolve_embedded_product_standalone_compiler_package_for_inputs_v1(
    host_module_path: &Path,
    target_paths: CompilerTargetInputPathsV1<'_>,
) -> ProductStandaloneCompilerPackageResolutionV1 {
    let catalog = match embedded_product_standalone_compiler_catalog_v1() {
        Ok(None) => return ProductStandaloneCompilerPackageResolutionV1::BundleAbsent,
        Ok(Some(catalog)) => catalog,
        Err(error) => {
            return unavailable(
                ProductStandaloneCompilerPackageUnavailableKindV1::InvalidEmbeddedCatalog,
                error,
            );
        }
    };
    resolve_authoritative_catalog_for_inputs_v1(
        &catalog,
        product_standalone_compiler_catalog_sha256_v1(
            crate::standalone_package::EMBEDDED_PRODUCT_STANDALONE_COMPILER_CATALOG_JSON_V1,
        ),
        host_module_path,
        target_paths,
    )
}

fn resolve_authoritative_catalog_for_inputs_v1(
    catalog: &ProductStandaloneCompilerCatalogV1,
    catalog_sha256: Sha256Digest,
    host_module_path: &Path,
    target_paths: CompilerTargetInputPathsV1<'_>,
) -> ProductStandaloneCompilerPackageResolutionV1 {
    let mut matched: Option<AvailableProductStandaloneCompilerPackageV1> = None;
    for profile in catalog.profiles() {
        match try_resolve_authoritative_catalog_at_host_v1(
            catalog,
            catalog_sha256,
            host_module_path,
            profile.target(),
            target_paths,
        ) {
            Ok(package) => {
                if let Some(existing) = matched.as_ref() {
                    let left = existing.profile_package().profile();
                    let right = package.profile_package().profile();
                    let same_compiler_api = left.binds == right.binds
                        && left.engine == right.engine
                        && left.unreal_semantics == right.unreal_semantics
                        && left.frontend == right.frontend
                        && left.bytecode == right.bytecode
                        && left.cache_writer == right.cache_writer
                        && left.qualification == right.qualification;
                    if !same_compiler_api {
                        return unavailable(
                            ProductStandaloneCompilerPackageUnavailableKindV1::AmbiguousTarget,
                            "installed game inputs match more than one different compiler API",
                        );
                    }
                    // Multiple distribution/build provenance records may intentionally point at
                    // the same API. Catalog order is canonical, so retaining the first is stable.
                    continue;
                }
                matched = Some(package);
            }
            Err(error)
                if error.kind()
                    == ProductStandaloneCompilerPackageUnavailableKindV1::TargetAuthentication =>
            {
                // Parsed cache/API compatibility did not identify this catalog candidate.
            }
            Err(error) => return ProductStandaloneCompilerPackageResolutionV1::Unavailable(error),
        }
    }
    matched.map_or_else(
        || {
            unavailable(
                ProductStandaloneCompilerPackageUnavailableKindV1::UnsupportedTarget,
                "physical compiler inputs match no embedded target",
            )
        },
        ProductStandaloneCompilerPackageResolutionV1::Available,
    )
}

fn resolve_authoritative_catalog_at_host_v1(
    catalog: &ProductStandaloneCompilerCatalogV1,
    catalog_sha256: Sha256Digest,
    host_module_path: &Path,
    target: &ProductStandaloneCompilerTargetV1,
    target_paths: CompilerTargetInputPathsV1<'_>,
) -> ProductStandaloneCompilerPackageResolutionV1 {
    match try_resolve_authoritative_catalog_at_host_v1(
        catalog,
        catalog_sha256,
        host_module_path,
        target,
        target_paths,
    ) {
        Ok(package) => ProductStandaloneCompilerPackageResolutionV1::Available(package),
        Err(error) => ProductStandaloneCompilerPackageResolutionV1::Unavailable(error),
    }
}

fn try_resolve_authoritative_catalog_at_host_v1(
    catalog: &ProductStandaloneCompilerCatalogV1,
    catalog_sha256: Sha256Digest,
    host_module_path: &Path,
    target: &ProductStandaloneCompilerTargetV1,
    target_paths: CompilerTargetInputPathsV1<'_>,
) -> Result<
    AvailableProductStandaloneCompilerPackageV1,
    ProductStandaloneCompilerPackageUnavailableV1,
> {
    #[cfg(not(windows))]
    {
        let _ = (
            catalog,
            catalog_sha256,
            host_module_path,
            target,
            target_paths,
        );
        return Err(unavailable_value(
            ProductStandaloneCompilerPackageUnavailableKindV1::UnsupportedPlatform,
            "product standalone compiler package resolution requires Windows",
        ));
    }

    #[cfg(windows)]
    {
        let package_root = fixed_package_root(host_module_path)?;
        let entry = catalog.profile_for_target(target).map_err(|error| {
            unavailable_value(
                ProductStandaloneCompilerPackageUnavailableKindV1::UnsupportedTarget,
                error,
            )
        })?;

        let sidecar_path = join_catalog_relative(&package_root, catalog.sidecar().relative_path());
        let profile_manifest_path =
            join_catalog_relative(&package_root, entry.manifest_relative_path());
        let profile_root = profile_manifest_path
            .parent()
            .ok_or_else(|| {
                unavailable_value(
                    ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath,
                    "profile manifest has no package-local parent",
                )
            })?
            .to_path_buf();

        let mut package_directory_pins = pin_absolute_directory_chain(&package_root)?;
        package_directory_pins.extend(pin_absolute_parent_chain(&sidecar_path)?);
        package_directory_pins.extend(pin_absolute_parent_chain(&profile_manifest_path)?);

        let mut sidecar = open_regular_no_follow(&sidecar_path, "standalone compiler sidecar")?;
        let (sidecar_len, sidecar_sha256) = hash_pinned_file(
            &mut sidecar,
            catalog.sidecar().byte_len(),
            ProductStandaloneCompilerPackageUnavailableKindV1::SidecarAuthentication,
            "standalone compiler sidecar",
        )?;
        catalog
            .authenticate_sidecar(sidecar_len, sidecar_sha256)
            .map_err(|error| {
                unavailable_value(
                    ProductStandaloneCompilerPackageUnavailableKindV1::SidecarAuthentication,
                    error,
                )
            })?;

        let mut manifest_file =
            open_regular_no_follow(&profile_manifest_path, "compiler profile manifest")?;
        let manifest_bytes = read_pinned_file(
            &mut manifest_file,
            entry.manifest_byte_len(),
            MAX_COMPILER_PROFILE_JSON_BYTES as u64,
            ProductStandaloneCompilerPackageUnavailableKindV1::ProfileManifestAuthentication,
            "compiler profile manifest",
        )?;
        let authenticated = catalog
            .authenticate_profile_manifest(target, &manifest_bytes)
            .map_err(|error| {
                unavailable_value(
                    ProductStandaloneCompilerPackageUnavailableKindV1::ProfileManifestAuthentication,
                    error,
                )
            })?;
        let authenticated_profile = authenticated.profile();

        let mut profile_files = vec![manifest_file];
        let mut unique_blobs = BTreeMap::<String, &SealedBlobV1>::new();
        for blob in profile_blobs(authenticated_profile) {
            unique_blobs
                .entry(blob.path.to_ascii_lowercase())
                .or_insert(blob);
        }
        let aggregate = unique_blobs.values().try_fold(0u64, |total, blob| {
            if blob.byte_len > MAX_PROFILE_BLOB_BYTES_V1 {
                return Err(unavailable_value(
                    ProductStandaloneCompilerPackageUnavailableKindV1::ProfilePayloadAuthentication,
                    "compiler profile blob exceeds its bounded size",
                ));
            }
            total
                .checked_add(blob.byte_len)
                .filter(|value| *value <= MAX_PROFILE_AGGREGATE_BYTES_V1)
                .ok_or_else(|| {
                    unavailable_value(
                        ProductStandaloneCompilerPackageUnavailableKindV1::ProfilePayloadAuthentication,
                        "compiler profile blobs exceed their aggregate bound",
                    )
                })
        })?;
        let _ = aggregate;
        for blob in unique_blobs.values() {
            let blob_path = join_catalog_relative(&profile_root, &blob.path);
            package_directory_pins.extend(pin_absolute_parent_chain(&blob_path)?);
            let mut file = open_regular_no_follow(&blob_path, "compiler profile blob")?;
            let (byte_len, sha256) = hash_pinned_file(
                &mut file,
                blob.byte_len,
                ProductStandaloneCompilerPackageUnavailableKindV1::ProfilePayloadAuthentication,
                "compiler profile blob",
            )?;
            if byte_len != blob.byte_len || sha256 != blob.sha256 {
                return Err(unavailable_value(
                    ProductStandaloneCompilerPackageUnavailableKindV1::ProfilePayloadAuthentication,
                    "compiler profile blob does not match its authenticated manifest seal",
                ));
            }
            if same_open_file_identity(&file, &sidecar)?
                || same_open_file_identity(&file, &profile_files[0])?
            {
                return Err(unavailable_value(
                    ProductStandaloneCompilerPackageUnavailableKindV1::ProfilePayloadAuthentication,
                    "compiler profile blob aliases a package control artifact",
                ));
            }
            profile_files.push(file);
        }

        // Reopening is safe here: every ancestor, manifest, and referenced blob is held without
        // write/delete sharing until the returned Available handle is dropped.
        let profile_package =
            ValidatedCompilerProfilePackageV1::load(&profile_manifest_path, &profile_root)
                .map_err(|error| {
                    unavailable_value(
                ProductStandaloneCompilerPackageUnavailableKindV1::ProfilePayloadAuthentication,
                error,
            )
                })?;
        let qualified_sidecar = profile_package.standalone_compiler_identity();
        let qualification_reference = catalog.qualification_reference();
        let catalog_protocol = catalog.sidecar().protocol();
        let qualification_protocol = qualification_reference.protocol();
        if qualified_sidecar.byte_len != qualification_reference.byte_len()
            || qualified_sidecar.sha256 != qualification_reference.sha256()
            || qualified_sidecar.request_version != qualification_protocol.request_version()
            || qualified_sidecar.response_version != qualification_protocol.response_version()
        {
            return Err(unavailable_value(
                ProductStandaloneCompilerPackageUnavailableKindV1::QualificationIdentityMismatch,
                "compiler profile does not match the catalogued qualification reference",
            ));
        }
        if profile_package.profile().profile_sha256 != entry.profile_sha256()
            || profile_package.profile().target != *target.target()
            || !profile_package
                .profile()
                .oracle
                .pe_codeview
                .guid
                .eq_ignore_ascii_case(&target.pe_codeview().guid)
            || profile_package.profile().oracle.pe_codeview.age != target.pe_codeview().age
        {
            return Err(unavailable_value(
                ProductStandaloneCompilerPackageUnavailableKindV1::ProfileManifestAuthentication,
                "typed compiler profile drifted from the selected embedded target identity",
            ));
        }

        let target_inputs = ValidatedCompilerTargetInputsV1::load(&profile_package, target_paths)
            .map_err(map_target_input_error)?;
        if target_inputs.profile_sha256() != entry.profile_sha256() {
            return Err(unavailable_value(
                ProductStandaloneCompilerPackageUnavailableKindV1::TargetAuthentication,
                "target proof is bound to a different compiler profile",
            ));
        }

        let identity = ProductStandaloneCompilerPackageIdentityV1 {
            catalog_sha256,
            sidecar_byte_len: sidecar_len,
            sidecar_sha256,
            compatibility_id: catalog.sidecar().compatibility_id().to_owned(),
            request_version: catalog_protocol.request_version(),
            response_version: catalog_protocol.response_version(),
            manifest_byte_len: entry.manifest_byte_len(),
            manifest_sha256: entry.manifest_sha256(),
            profile_sha256: entry.profile_sha256(),
            target: target.clone(),
        };
        let shipping_cache_seal = target_inputs.shipping_cache_seal();
        let binds_cache_seal = target_inputs.binds_cache_seal();
        let authority = ProductStandaloneCompilerReceiptAuthorityV1 {
            identity,
            shipping_cache_seal,
            binds_cache_seal,
            sidecar_path,
            profile_manifest_path,
            profile_root,
            sidecar,
            profile_package,
            _profile_files: profile_files,
            _package_directory_pins: package_directory_pins,
        };
        Ok(AvailableProductStandaloneCompilerPackageV1 {
            authority,
            target_inputs,
        })
    }
}

#[cfg(windows)]
fn profile_blobs(profile: &CompilerProfileV1) -> [&SealedBlobV1; 16] {
    [
        &profile.engine.ordered_engine_properties,
        &profile.engine.registration_trace,
        &profile.engine.post_bind_snapshot,
        &profile.unreal_semantics.reflected_type_graph,
        &profile.frontend.preprocessor_config,
        &profile.frontend.class_generator_config,
        &profile.frontend.compiler_options,
        &profile.bytecode.opcode_table,
        &profile.bytecode.operand_schema,
        &profile.bytecode.codegen_probe_corpus,
        &profile.bytecode.expected_probe_results,
        &profile.cache_writer.serializer_schema,
        &profile.cache_writer.reference_table_order,
        &profile.cache_writer.normalized_oracle_corpus,
        &profile.qualification.diagnostic_parity,
        &profile.qualification.semantic_parity,
    ]
}

#[cfg(windows)]
fn fixed_package_root(
    host_module_path: &Path,
) -> Result<PathBuf, ProductStandaloneCompilerPackageUnavailableV1> {
    if !host_module_path.is_absolute()
        || host_module_path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(unavailable_value(
            ProductStandaloneCompilerPackageUnavailableKindV1::UnsafeHostLocation,
            "host module path must be absolute and lexically normalized",
        ));
    }
    let parent = host_module_path.parent().ok_or_else(|| {
        unavailable_value(
            ProductStandaloneCompilerPackageUnavailableKindV1::UnsafeHostLocation,
            "host module path has no app-local parent",
        )
    })?;
    Ok(parent.join(PRODUCT_STANDALONE_COMPILER_PACKAGE_ROOT_V1))
}

#[cfg(windows)]
fn join_catalog_relative(root: &Path, relative: &str) -> PathBuf {
    relative
        .split('/')
        .fold(root.to_path_buf(), |path, part| path.join(part))
}

#[cfg(windows)]
fn pin_absolute_parent_chain(
    path: &Path,
) -> Result<Vec<File>, ProductStandaloneCompilerPackageUnavailableV1> {
    let parent = path.parent().ok_or_else(|| {
        unavailable_value(
            ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath,
            "package artifact has no parent directory",
        )
    })?;
    pin_absolute_directory_chain(parent)
}

#[cfg(windows)]
fn pin_absolute_directory_chain(
    path: &Path,
) -> Result<Vec<File>, ProductStandaloneCompilerPackageUnavailableV1> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(unavailable_value(
            ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath,
            "package directory must be absolute and lexically normalized",
        ));
    }
    let mut ancestors = path.ancestors().collect::<Vec<_>>();
    ancestors.reverse();
    ancestors
        .into_iter()
        .map(open_directory_pin_no_follow)
        .collect()
}

#[cfg(windows)]
fn open_directory_pin_no_follow(
    path: &Path,
) -> Result<File, ProductStandaloneCompilerPackageUnavailableV1> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ,
        FILE_SHARE_READ,
    };
    let mut options = std::fs::OpenOptions::new();
    options
        .access_mode(FILE_GENERIC_READ)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    let file = options.open(path).map_err(|_| {
        unavailable_value(
            ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath,
            "package directory is unavailable",
        )
    })?;
    let metadata = file.metadata().map_err(|_| {
        unavailable_value(
            ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath,
            "package directory identity could not be inspected",
        )
    })?;
    if !metadata.is_dir() || metadata_is_reparse(&metadata) {
        return Err(unavailable_value(
            ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath,
            "package directory is not a real non-reparse directory",
        ));
    }
    Ok(file)
}

#[cfg(windows)]
fn open_regular_no_follow(
    path: &Path,
    label: &'static str,
) -> Result<File, ProductStandaloneCompilerPackageUnavailableV1> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ};
    let file = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(|_| {
            unavailable_value(
                ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath,
                format!("{label} is unavailable"),
            )
        })?;
    validate_open_regular(&file, label)?;
    Ok(file)
}

#[cfg(windows)]
fn validate_open_regular(
    file: &File,
    label: &'static str,
) -> Result<(), ProductStandaloneCompilerPackageUnavailableV1> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY,
        FILE_ATTRIBUTE_REPARSE_POINT,
    };
    let mut info = unsafe { std::mem::zeroed::<BY_HANDLE_FILE_INFORMATION>() };
    // SAFETY: `file` owns a valid handle and `info` is writable for the call.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0
        || info.dwFileAttributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0
        || info.nNumberOfLinks != 1
    {
        return Err(unavailable_value(
            ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath,
            format!("{label} is not a single regular non-reparse file"),
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn metadata_is_reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
    metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(windows)]
fn read_pinned_file(
    file: &mut File,
    expected_len: u64,
    max_len: u64,
    authentication_kind: ProductStandaloneCompilerPackageUnavailableKindV1,
    label: &'static str,
) -> Result<Vec<u8>, ProductStandaloneCompilerPackageUnavailableV1> {
    if expected_len > max_len {
        return Err(unavailable_value(
            authentication_kind,
            format!("{label} exceeds its bounded size"),
        ));
    }
    let before = file.metadata().map_err(|_| {
        unavailable_value(
            ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath,
            format!("{label} identity could not be inspected"),
        )
    })?;
    if before.len() != expected_len {
        return Err(unavailable_value(
            authentication_kind,
            format!("{label} length does not match the embedded catalog"),
        ));
    }
    let capacity = usize::try_from(expected_len).map_err(|_| {
        unavailable_value(
            authentication_kind,
            format!("{label} length is not addressable"),
        )
    })?;
    file.seek(SeekFrom::Start(0)).map_err(|_| {
        unavailable_value(
            ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath,
            format!("{label} could not be rewound"),
        )
    })?;
    let mut bytes = Vec::with_capacity(capacity);
    file.take(max_len + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| {
            unavailable_value(
                ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath,
                format!("{label} could not be read"),
            )
        })?;
    let after = file.metadata().map_err(|_| {
        unavailable_value(
            ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath,
            format!("{label} identity changed while reading"),
        )
    })?;
    if bytes.len() as u64 != expected_len || after.len() != before.len() {
        return Err(unavailable_value(
            ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath,
            format!("{label} changed while reading"),
        ));
    }
    Ok(bytes)
}

#[cfg(windows)]
fn hash_pinned_file(
    file: &mut File,
    expected_len: u64,
    authentication_kind: ProductStandaloneCompilerPackageUnavailableKindV1,
    label: &'static str,
) -> Result<(u64, Sha256Digest), ProductStandaloneCompilerPackageUnavailableV1> {
    let before = file.metadata().map_err(|_| {
        unavailable_value(
            ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath,
            format!("{label} identity could not be inspected"),
        )
    })?;
    if before.len() != expected_len {
        return Err(unavailable_value(
            authentication_kind,
            format!("{label} length does not match its authenticated seal"),
        ));
    }
    file.seek(SeekFrom::Start(0)).map_err(|_| {
        unavailable_value(
            ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath,
            format!("{label} could not be rewound"),
        )
    })?;
    let mut hash = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|_| {
            unavailable_value(
                ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath,
                format!("{label} could not be read"),
            )
        })?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .filter(|value| *value <= expected_len)
            .ok_or_else(|| {
                unavailable_value(authentication_kind, format!("{label} exceeded its seal"))
            })?;
        hash.update(&buffer[..read]);
    }
    let after = file.metadata().map_err(|_| {
        unavailable_value(
            ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath,
            format!("{label} identity changed while reading"),
        )
    })?;
    if total != expected_len || after.len() != before.len() {
        return Err(unavailable_value(
            ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath,
            format!("{label} changed while reading"),
        ));
    }
    Ok((total, Sha256Digest::from_bytes(hash.finalize().into())))
}

#[cfg(windows)]
fn same_open_file_identity(
    left: &File,
    right: &File,
) -> Result<bool, ProductStandaloneCompilerPackageUnavailableV1> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };
    let inspect = |file: &File| {
        let mut info = unsafe { std::mem::zeroed::<BY_HANDLE_FILE_INFORMATION>() };
        // SAFETY: `file` owns a valid handle and `info` is writable for the call.
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
            return Err(unavailable_value(
                ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath,
                "package file identity could not be inspected",
            ));
        }
        Ok((
            info.dwVolumeSerialNumber,
            info.nFileIndexHigh,
            info.nFileIndexLow,
        ))
    };
    Ok(inspect(left)? == inspect(right)?)
}

fn unavailable(
    kind: ProductStandaloneCompilerPackageUnavailableKindV1,
    error: impl std::fmt::Display,
) -> ProductStandaloneCompilerPackageResolutionV1 {
    ProductStandaloneCompilerPackageResolutionV1::Unavailable(unavailable_value(kind, error))
}

fn unavailable_value(
    kind: ProductStandaloneCompilerPackageUnavailableKindV1,
    error: impl std::fmt::Display,
) -> ProductStandaloneCompilerPackageUnavailableV1 {
    ProductStandaloneCompilerPackageUnavailableV1 {
        kind,
        detail: error.to_string(),
    }
}

fn map_target_input_error(
    error: CompilerTargetInputError,
) -> ProductStandaloneCompilerPackageUnavailableV1 {
    let kind = match error {
        CompilerTargetInputError::UnsupportedPlatform => {
            ProductStandaloneCompilerPackageUnavailableKindV1::UnsupportedPlatform
        }
        CompilerTargetInputError::Mismatch(_)
        | CompilerTargetInputError::InvalidExecutable
        | CompilerTargetInputError::InvalidCodeView => {
            ProductStandaloneCompilerPackageUnavailableKindV1::TargetAuthentication
        }
        CompilerTargetInputError::UnsafePath(_)
        | CompilerTargetInputError::UnsafeFile(_)
        | CompilerTargetInputError::TooLarge(_)
        | CompilerTargetInputError::Changed(_) => {
            ProductStandaloneCompilerPackageUnavailableKindV1::TargetInputsUnavailable
        }
    };
    unavailable_value(kind, error)
}

impl From<ProductStandaloneCompilerCatalogError> for ProductStandaloneCompilerPackageUnavailableV1 {
    fn from(error: ProductStandaloneCompilerCatalogError) -> Self {
        unavailable_value(
            ProductStandaloneCompilerPackageUnavailableKindV1::InvalidEmbeddedCatalog,
            error,
        )
    }
}

impl From<CompilerTargetInputError> for ProductStandaloneCompilerPackageUnavailableV1 {
    fn from(error: CompilerTargetInputError) -> Self {
        map_target_input_error(error)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    #[cfg(windows)]
    use sha1::Sha1;
    #[cfg(windows)]
    use sha2::Sha256;

    #[cfg(windows)]
    use crate::compiler_profile::frontend::{
        ClassGeneratorConfigV1, CompilerOptionsV1, EffectivePreprocessorFlagV1,
        ExternalFrontendHooksV1, NativeSuperKindV1, NativeSuperTypeV1, PreprocessorConfigV1,
        PropertyBlueprintSpecifierV1, PropertyEditSpecifierV1, StaticClassModeV1,
        CLASS_GENERATOR_CONFIG_SCHEMA, COMPILER_OPTIONS_SCHEMA, FRONTEND_SCHEMA_VERSION,
        PREPROCESSOR_CONFIG_SCHEMA,
    };
    #[cfg(windows)]
    use crate::compiler_profile::manifest::{
        BindsProfileV1, BytecodeProfileV1, CacheWriterProfileV1, CompilerOracleV1, EngineProfileV1,
        FileSealV1, FrontendProfileV1, QualificationProfileV1, Sha1Digest,
        UnrealSemanticsProfileV1, COMPILER_PROFILE_SCHEMA, COMPILER_PROFILE_SCHEMA_VERSION,
    };
    use crate::compiler_profile::manifest::{
        CompilerArchitectureV1, CompilerBuildConfigurationV1, CompilerPlatformV1, CompilerTargetV1,
        PeCodeViewV1,
    };
    #[cfg(windows)]
    use crate::compiler_profile::qualification::{
        CompilerProbeCaseV1, CompilerProbeCorpusV1, DiagnosticParityEntryV1,
        DiagnosticParityReportV1, ExpectedProbeResultV1, ExpectedProbeResultsV1, ProbeModeV1,
        ProbeOutcomeV1, ProbeSourceSectionV1, QualifiedSidecarIdentityV1, SemanticParityEntryV1,
        SemanticParityReportV1, DIAGNOSTIC_PARITY_SCHEMA, EXPECTED_RESULTS_SCHEMA,
        PROBE_CORPUS_SCHEMA, QUALIFICATION_SCHEMA_VERSION, QUALIFIED_SIDECAR_REQUEST_VERSION_V2,
        QUALIFIED_SIDECAR_RESPONSE_VERSION_V1, SEMANTIC_PARITY_SCHEMA,
    };
    #[cfg(windows)]
    use crate::compiler_profile::registry::{
        DynamicScriptTypeOperationsV1, EnginePropertySettingV1, EnginePropertyV1,
        FixedTypeOperationsV1, OrderedEnginePropertiesV1, PostBindEntryV1, PostBindResultV1,
        PostBindSnapshotV1, PrimitiveTypeOperationsV1, PrimitiveTypeV1, RegistrationContextV1,
        RegistrationEntryV1, RegistrationTraceV1, TypeOperationsV1, ENGINE_PROPERTIES_SCHEMA,
        POST_BIND_SNAPSHOT_SCHEMA, REGISTRATION_TRACE_SCHEMA,
    };
    #[cfg(windows)]
    use crate::standalone_package::{
        PRODUCT_STANDALONE_COMPILER_CATALOG_SCHEMA_V1,
        PRODUCT_STANDALONE_COMPILER_CATALOG_SCHEMA_VERSION_V1,
        STANDALONE_COMPILER_COMPATIBILITY_ID_V1,
    };
    #[cfg(windows)]
    use crate::standalone_sidecar::{SIDECAR_REQUEST_VERSION_V2, SIDECAR_RESPONSE_VERSION_V1};

    #[cfg(windows)]
    pub(crate) struct SyntheticProductPackageFixtureV1 {
        _temp: tempfile::TempDir,
        app_root: PathBuf,
        package_root: PathBuf,
        host_module: PathBuf,
        sidecar_path: PathBuf,
        manifest_path: PathBuf,
        executable_path: PathBuf,
        shipping_path: PathBuf,
        binds_path: PathBuf,
        target: ProductStandaloneCompilerTargetV1,
        catalog_bytes: Vec<u8>,
        catalog: ProductStandaloneCompilerCatalogV1,
        manifest_bytes: Vec<u8>,
        shipping_bytes: Vec<u8>,
        binds_bytes: Vec<u8>,
    }

    #[cfg(windows)]
    fn synthetic_binds_database(wide_strings: bool) -> Vec<u8> {
        fn push_i32(output: &mut Vec<u8>, value: i32) {
            output.extend_from_slice(&value.to_le_bytes());
        }
        fn push_bool(output: &mut Vec<u8>, value: bool) {
            push_i32(output, i32::from(value));
        }
        fn push_string(output: &mut Vec<u8>, value: &str, wide: bool) {
            if wide {
                let encoded = value.encode_utf16().collect::<Vec<_>>();
                push_i32(output, -(encoded.len() as i32 + 1));
                for unit in encoded {
                    output.extend_from_slice(&unit.to_le_bytes());
                }
                output.extend_from_slice(&0u16.to_le_bytes());
            } else {
                push_i32(output, value.len() as i32 + 1);
                output.extend_from_slice(value.as_bytes());
                output.push(0);
            }
        }
        fn push_property(output: &mut Vec<u8>, declaration: &str, path: &str, wide: bool) {
            push_string(output, declaration, wide);
            push_string(output, path, wide);
            for value in [true, true, false, true, false] {
                push_bool(output, value);
            }
            push_string(output, "Generated", wide);
            push_bool(output, false);
            push_bool(output, false);
        }

        let mut output = Vec::new();
        push_i32(&mut output, 1);
        push_string(&mut output, "FVector", wide_strings);
        push_string(&mut output, "/Script/CoreUObject.Vector", wide_strings);
        push_i32(&mut output, 1);
        push_property(
            &mut output,
            "float X",
            "/Script/CoreUObject.Vector:X",
            wide_strings,
        );

        push_i32(&mut output, 1);
        push_string(&mut output, "UObject", wide_strings);
        push_string(&mut output, "/Script/CoreUObject.Object", wide_strings);
        push_i32(&mut output, 1);
        push_string(&mut output, "FString GetName() const", wide_strings);
        push_string(
            &mut output,
            "/Script/CoreUObject.Object:GetName",
            wide_strings,
        );
        for value in [false, false, false, true, true] {
            push_bool(&mut output, value);
        }
        output.push((-1i8) as u8);
        output.push(2);
        push_string(&mut output, "UObject", wide_strings);
        push_string(&mut output, "GetName", wide_strings);
        push_i32(&mut output, 1);
        push_property(
            &mut output,
            "FName Name",
            "/Script/CoreUObject.Object:Name",
            wide_strings,
        );
        output
    }

    #[cfg(windows)]
    impl SyntheticProductPackageFixtureV1 {
        pub(crate) fn create() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let app_root = temp.path().join("app");
            let package_root = app_root.join(PRODUCT_STANDALONE_COMPILER_PACKAGE_ROOT_V1);
            let profile_root = package_root.join("profiles/build-24539464");
            let sidecar_path = package_root.join("bin/gore-as-standalone-compiler.exe");
            let manifest_path = profile_root.join("compiler-profile.json");
            std::fs::create_dir_all(sidecar_path.parent().unwrap()).unwrap();
            std::fs::create_dir_all(&profile_root).unwrap();
            // The parity reports retain the exact unsigned qualification build. Product bytes
            // model a later reproducible rebuild plus release signing at the same semantic ABI.
            let qualification_sidecar_bytes = b"synthetic-qualified-sidecar";
            let sidecar_bytes = b"synthetic-rebuilt-and-signed-sidecar";
            std::fs::write(&sidecar_path, sidecar_bytes).unwrap();

            let install_root = temp.path().join("install/G1R");
            let executable_path = install_root.join("Gothic1Remake.exe");
            let shipping_path = install_root.join("Content/Script/ScriptCache.bin");
            let binds_path = install_root.join("Content/Script/Binds.Cache");
            std::fs::create_dir_all(shipping_path.parent().unwrap()).unwrap();
            let (executable_bytes, codeview) = synthetic_pe();
            let shipping_bytes = crate::compile::build_full_graph_cache_for_test(
                crate::cache::header::CACHE_MAGIC,
                [1; 16],
                &[("Module", "Module.as")],
            )
            .unwrap();
            let binds_bytes = synthetic_binds_database(false);
            std::fs::write(&executable_path, &executable_bytes).unwrap();
            std::fs::write(&shipping_path, &shipping_bytes).unwrap();
            std::fs::write(&binds_path, &binds_bytes).unwrap();

            let target = ProductStandaloneCompilerTargetV1::try_new(
                CompilerTargetV1 {
                    steam_app_id: 1_297_900,
                    steam_build_id: 24_539_464,
                    depot_id: 1_297_901,
                    depot_manifest_gid: 1_585_071_322_101_748_861,
                    platform: CompilerPlatformV1::Windows,
                    architecture: CompilerArchitectureV1::X86_64,
                    build_configuration: CompilerBuildConfigurationV1::Shipping,
                },
                codeview,
            )
            .unwrap();

            let untyped_bytes = b"untyped-profile-payload";
            let untyped = write_blob(&profile_root, "payload/untyped.bin", untyped_bytes);
            let (properties, trace, snapshot) = registry_payloads();
            let properties_blob = write_blob(
                &profile_root,
                "engine/properties.json",
                &properties.to_json().unwrap(),
            );
            let trace_blob = write_blob(
                &profile_root,
                "engine/registrations.json",
                &trace.to_json().unwrap(),
            );
            let snapshot_blob = write_blob(
                &profile_root,
                "engine/post-bind.json",
                &snapshot.to_json().unwrap(),
            );
            let (preprocessor, class_generator, compiler_options) = frontend_payloads();
            let preprocessor_blob = write_blob(
                &profile_root,
                "frontend/preprocessor.json",
                &preprocessor.to_json().unwrap(),
            );
            let class_generator_blob = write_blob(
                &profile_root,
                "frontend/class-generator.json",
                &class_generator.to_json().unwrap(),
            );
            let compiler_options_blob = write_blob(
                &profile_root,
                "frontend/compiler-options.json",
                &compiler_options.to_json().unwrap(),
            );
            let qualified_sidecar = QualifiedSidecarIdentityV1 {
                byte_len: qualification_sidecar_bytes.len() as u64,
                sha256: sha256(qualification_sidecar_bytes),
                request_version: QUALIFIED_SIDECAR_REQUEST_VERSION_V2,
                response_version: QUALIFIED_SIDECAR_RESPONSE_VERSION_V1,
            };
            let (corpus, expected, diagnostics, semantics) =
                qualification_payloads(qualified_sidecar);
            let corpus_blob = write_blob(
                &profile_root,
                "qualification/corpus.json",
                &corpus.to_json().unwrap(),
            );
            let expected_blob = write_blob(
                &profile_root,
                "qualification/expected.json",
                &expected.to_json().unwrap(),
            );
            let diagnostics_blob = write_blob(
                &profile_root,
                "qualification/diagnostics.json",
                &diagnostics.to_json().unwrap(),
            );
            let semantics_blob = write_blob(
                &profile_root,
                "qualification/semantics.json",
                &semantics.to_json().unwrap(),
            );

            let file_seal = |bytes: &[u8], steam: bool| FileSealV1 {
                byte_len: bytes.len() as u64,
                sha256: sha256(bytes),
                steam_content_sha1: steam
                    .then(|| Sha1Digest::from_bytes(Sha1::digest(bytes).into())),
            };
            let mut profile = CompilerProfileV1 {
                schema: COMPILER_PROFILE_SCHEMA.to_owned(),
                schema_version: COMPILER_PROFILE_SCHEMA_VERSION,
                target: target.target().clone(),
                oracle: CompilerOracleV1 {
                    executable: file_seal(&executable_bytes, true),
                    binds_cache: file_seal(&binds_bytes, true),
                    shipping_cache: file_seal(&shipping_bytes, true),
                    depot_manifest: file_seal(b"synthetic-depot-manifest", false),
                    pe_codeview: target.pe_codeview().clone(),
                },
                binds: BindsProfileV1::from_database(
                    &crate::compiler_profile::binds::BindsDatabase::parse(&binds_bytes).unwrap(),
                ),
                engine: EngineProfileV1 {
                    as_create_version: 23_300,
                    ordered_engine_properties: properties_blob,
                    registration_trace: trace_blob,
                    registration_trace_count: 1,
                    post_bind_snapshot: snapshot_blob,
                },
                unreal_semantics: UnrealSemanticsProfileV1 {
                    reflected_type_graph: untyped.clone(),
                    metadata_schema_version: 1,
                },
                frontend: FrontendProfileV1 {
                    preprocessor_config: preprocessor_blob,
                    class_generator_config: class_generator_blob,
                    compiler_options: compiler_options_blob,
                },
                bytecode: BytecodeProfileV1 {
                    opcode_table_version: "g1r-test-v1".to_owned(),
                    opcode_table: untyped.clone(),
                    operand_schema: untyped.clone(),
                    codegen_probe_corpus: corpus_blob,
                    expected_probe_results: expected_blob,
                },
                cache_writer: CacheWriterProfileV1 {
                    format_version: 1,
                    serializer_schema: untyped.clone(),
                    build_identifier: 0x9e37_7abe,
                    reference_table_order: untyped.clone(),
                    normalized_oracle_corpus: untyped,
                },
                qualification: QualificationProfileV1 {
                    required_probe_suite_version: "product-resolver-test-v1".to_owned(),
                    diagnostic_parity: diagnostics_blob,
                    semantic_parity: semantics_blob,
                    qualified: true,
                },
                profile_sha256: Sha256Digest::from_bytes([0; 32]),
            };
            profile.seal().unwrap();
            let manifest_bytes = serde_json::to_vec(&profile).unwrap();
            std::fs::write(&manifest_path, &manifest_bytes).unwrap();
            let catalog_bytes = serde_json::to_vec_pretty(&serde_json::json!({
                "schema": PRODUCT_STANDALONE_COMPILER_CATALOG_SCHEMA_V1,
                "schema_version": PRODUCT_STANDALONE_COMPILER_CATALOG_SCHEMA_VERSION_V1,
                "sidecar": {
                    "relative_path": "bin/gore-as-standalone-compiler.exe",
                    "byte_len": sidecar_bytes.len(),
                    "sha256": sha256(sidecar_bytes),
                    "compatibility_id": STANDALONE_COMPILER_COMPATIBILITY_ID_V1,
                    "protocol": {
                        "request_version": SIDECAR_REQUEST_VERSION_V2,
                        "response_version": SIDECAR_RESPONSE_VERSION_V1
                    },
                    "static_system_only": true
                },
                "qualification_reference": {
                    "byte_len": qualification_sidecar_bytes.len(),
                    "sha256": sha256(qualification_sidecar_bytes),
                    "compatibility_id": STANDALONE_COMPILER_COMPATIBILITY_ID_V1,
                    "protocol": {
                        "request_version": SIDECAR_REQUEST_VERSION_V2,
                        "response_version": SIDECAR_RESPONSE_VERSION_V1
                    }
                },
                "profiles": [{
                    "manifest_relative_path": "profiles/build-24539464/compiler-profile.json",
                    "manifest_byte_len": manifest_bytes.len(),
                    "manifest_sha256": sha256(&manifest_bytes),
                    "profile_sha256": profile.profile_sha256,
                    "target": target
                }]
            }))
            .unwrap();
            let catalog = ProductStandaloneCompilerCatalogV1::from_json(&catalog_bytes).unwrap();
            let host_module = app_root.join("gore.exe");
            Self {
                _temp: temp,
                app_root,
                package_root,
                host_module,
                sidecar_path,
                manifest_path,
                executable_path,
                shipping_path,
                binds_path,
                target,
                catalog_bytes,
                catalog,
                manifest_bytes,
                shipping_bytes,
                binds_bytes,
            }
        }

        fn resolve(&self) -> ProductStandaloneCompilerPackageResolutionV1 {
            resolve_authoritative_catalog_at_host_v1(
                &self.catalog,
                sha256(&self.catalog_bytes),
                &self.host_module,
                &self.target,
                self.target_paths(),
            )
        }

        fn resolve_for_inputs(&self) -> ProductStandaloneCompilerPackageResolutionV1 {
            resolve_authoritative_catalog_for_inputs_v1(
                &self.catalog,
                sha256(&self.catalog_bytes),
                &self.host_module,
                self.target_paths(),
            )
        }

        fn target_paths(&self) -> CompilerTargetInputPathsV1<'_> {
            CompilerTargetInputPathsV1 {
                executable: &self.executable_path,
                shipping_cache: &self.shipping_path,
                binds_cache: &self.binds_path,
            }
        }

        /// Replace only representation-specific target bytes while preserving compiler
        /// compatibility: Shipping gets a different per-cache GUID and Binds uses wide strings.
        pub(crate) fn install_compatible_target_variants(&self) -> (Vec<u8>, Vec<u8>) {
            let mut shipping = self.shipping_bytes.clone();
            shipping[0] ^= 0x5a; // Per-cache GUID, not compiler compatibility.
            let binds = synthetic_binds_database(true);
            assert_ne!(binds, self.binds_bytes);
            std::fs::write(&self.shipping_path, &shipping).unwrap();
            std::fs::write(&self.binds_path, &binds).unwrap();
            (shipping, binds)
        }

        /// Construct the non-forgeable Receipt V2 authority through the real resolver and model a
        /// compile transaction taking, then releasing, the exact target handles.
        pub(crate) fn receipt_authority(&self) -> ProductStandaloneCompilerReceiptAuthorityV1 {
            let available = match self.resolve() {
                ProductStandaloneCompilerPackageResolutionV1::Available(value) => value,
                other => panic!("expected available synthetic package, got {other:?}"),
            };
            let (authority, target_inputs) = available.into_execution_parts();
            drop(target_inputs);
            authority
        }
    }

    #[cfg(windows)]
    fn write_blob(root: &Path, relative: &str, bytes: &[u8]) -> SealedBlobV1 {
        let path = relative
            .split('/')
            .fold(root.to_path_buf(), |path, part| path.join(part));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
        SealedBlobV1 {
            path: relative.to_owned(),
            byte_len: bytes.len() as u64,
            sha256: sha256(bytes),
        }
    }

    #[cfg(windows)]
    fn registry_payloads() -> (
        OrderedEnginePropertiesV1,
        RegistrationTraceV1,
        PostBindSnapshotV1,
    ) {
        let mut properties = OrderedEnginePropertiesV1 {
            schema: ENGINE_PROPERTIES_SCHEMA.to_owned(),
            schema_version: 1,
            settings: vec![EnginePropertySettingV1 {
                ordinal: 0,
                property: EnginePropertyV1::OptimizeBytecode,
                value: 1,
            }],
            canonical_sha256: Sha256Digest::from_bytes([0; 32]),
        };
        properties.seal().unwrap();
        let primitive_layouts = [
            (PrimitiveTypeV1::Bool, 1, 1),
            (PrimitiveTypeV1::Int8, 1, 1),
            (PrimitiveTypeV1::Int16, 2, 2),
            (PrimitiveTypeV1::Int32, 4, 4),
            (PrimitiveTypeV1::Int64, 8, 8),
            (PrimitiveTypeV1::Uint8, 1, 1),
            (PrimitiveTypeV1::Uint16, 2, 2),
            (PrimitiveTypeV1::Uint32, 4, 4),
            (PrimitiveTypeV1::Uint64, 8, 8),
            (PrimitiveTypeV1::Float32, 4, 4),
            (PrimitiveTypeV1::Float64, 8, 8),
        ];
        let fixed = |value_size, value_alignment| FixedTypeOperationsV1 {
            can_create_property: true,
            never_requires_gc: false,
            requires_property: false,
            can_be_template_subtype: true,
            can_construct: true,
            need_construct: false,
            can_destruct: true,
            need_destruct: false,
            can_copy: true,
            need_copy: false,
            can_compare: true,
            can_hash_value: true,
            value_size,
            value_alignment,
            is_object_pointer: false,
        };
        let mut trace = RegistrationTraceV1 {
            schema: REGISTRATION_TRACE_SCHEMA.to_owned(),
            schema_version: 1,
            host_stubs: vec![],
            primitive_operations: primitive_layouts
                .into_iter()
                .enumerate()
                .map(
                    |(ordinal, (primitive, size, alignment))| PrimitiveTypeOperationsV1 {
                        ordinal: ordinal as u32,
                        primitive,
                        operations: fixed(size, alignment),
                    },
                )
                .collect(),
            dynamic_script_operations: DynamicScriptTypeOperationsV1 {
                delegate: FixedTypeOperationsV1 {
                    need_construct: true,
                    need_destruct: true,
                    need_copy: true,
                    can_hash_value: false,
                    value_size: 16,
                    value_alignment: 8,
                    ..fixed(16, 8)
                },
                multicast_delegate: FixedTypeOperationsV1 {
                    need_construct: true,
                    need_destruct: true,
                    need_copy: true,
                    can_hash_value: false,
                    value_size: 16,
                    value_alignment: 8,
                    ..fixed(16, 8)
                },
            },
            entries: vec![RegistrationEntryV1::Enum {
                ordinal: 0,
                registration_id: 0,
                context: RegistrationContextV1 {
                    namespace: String::new(),
                    config_group: None,
                    access_mask: u32::MAX,
                },
                type_id: 1,
                declaration: "ETest".to_owned(),
                type_operations: TypeOperationsV1::Fixed {
                    operations: fixed(1, 1),
                },
            }],
            canonical_sha256: Sha256Digest::from_bytes([0; 32]),
        };
        trace.seal().unwrap();
        let mut snapshot = PostBindSnapshotV1 {
            schema: POST_BIND_SNAPSHOT_SCHEMA.to_owned(),
            schema_version: 1,
            engine_properties_sha256: properties.canonical_sha256,
            registration_trace_sha256: trace.canonical_sha256,
            entries: vec![PostBindEntryV1 {
                ordinal: 0,
                trace_registration_id: 0,
                result: PostBindResultV1::Enum { engine_type_id: 1 },
            }],
            final_states: vec![],
            canonical_sha256: Sha256Digest::from_bytes([0; 32]),
        };
        snapshot.seal().unwrap();
        (properties, trace, snapshot)
    }

    #[cfg(windows)]
    fn frontend_payloads() -> (
        PreprocessorConfigV1,
        ClassGeneratorConfigV1,
        CompilerOptionsV1,
    ) {
        let mut preprocessor = PreprocessorConfigV1 {
            schema: PREPROCESSOR_CONFIG_SCHEMA.to_owned(),
            schema_version: FRONTEND_SCHEMA_VERSION,
            automatic_imports: true,
            warn_on_manual_import_statements: true,
            use_editor_scripts: false,
            effective_flags: [
                ("COOK_COMMANDLET", false),
                ("EDITOR", false),
                ("EDITORONLY_DATA", false),
                ("RELEASE", true),
                ("TEST", false),
                ("WITH_SERVER_CODE", true),
            ]
            .into_iter()
            .enumerate()
            .map(|(ordinal, (name, value))| EffectivePreprocessorFlagV1 {
                ordinal: ordinal as u32,
                name: name.to_owned(),
                value,
            })
            .collect(),
            default_function_blueprint_callable: true,
            default_property_edit_specifier: PropertyEditSpecifierV1::EditAnywhere,
            default_property_edit_specifier_for_structs: PropertyEditSpecifierV1::EditAnywhere,
            default_property_blueprint_specifier: PropertyBlueprintSpecifierV1::BlueprintReadWrite,
            static_class_mode: StaticClassModeV1::Allowed,
            script_float_is_float64: true,
            angelscript_haze: false,
            enforce_server_rpc_validation: false,
            blueprint_event_argument_specializations: vec!["FName".to_owned(), "int32".to_owned()],
            native_super_types: vec![
                NativeSuperTypeV1 {
                    ordinal: 0,
                    angelscript_type_name: "AActor".to_owned(),
                    unreal_class_path: "/Script/Engine.Actor".to_owned(),
                    property_offset: 0,
                    kind: NativeSuperKindV1::Actor,
                    game_state_subsystem: false,
                    cannot_derive_angelscript: false,
                },
                NativeSuperTypeV1 {
                    ordinal: 1,
                    angelscript_type_name: "UObject".to_owned(),
                    unreal_class_path: "/Script/CoreUObject.Object".to_owned(),
                    property_offset: 0,
                    kind: NativeSuperKindV1::OtherUObject,
                    game_state_subsystem: false,
                    cannot_derive_angelscript: false,
                },
            ],
            fname_comparison_keys: vec![],
            external_hooks: ExternalFrontendHooksV1::unbound(),
            canonical_sha256: Sha256Digest::from_bytes([0; 32]),
        };
        preprocessor.seal().unwrap();
        let mut class_generator = ClassGeneratorConfigV1 {
            schema: CLASS_GENERATOR_CONFIG_SCHEMA.to_owned(),
            schema_version: FRONTEND_SCHEMA_VERSION,
            mark_non_uproperty_properties_as_transient: false,
            canonical_sha256: Sha256Digest::from_bytes([0; 32]),
        };
        class_generator.seal().unwrap();
        let mut compiler_options = CompilerOptionsV1 {
            schema: COMPILER_OPTIONS_SCHEMA.to_owned(),
            schema_version: FRONTEND_SCHEMA_VERSION,
            error_on_incorrect_editor_only_code: true,
            warn_on_divergent_comparison_operator_overloads: true,
            warn_on_implicit_signed_unsigned_conversion: true,
            warn_on_increment_decrement_in_complex_expression: true,
            warn_on_unused_return_value_for_const_methods: true,
            canonical_sha256: Sha256Digest::from_bytes([0; 32]),
        };
        compiler_options.seal().unwrap();
        (preprocessor, class_generator, compiler_options)
    }

    #[cfg(windows)]
    fn qualification_payloads(
        sidecar: QualifiedSidecarIdentityV1,
    ) -> (
        CompilerProbeCorpusV1,
        ExpectedProbeResultsV1,
        DiagnosticParityReportV1,
        SemanticParityReportV1,
    ) {
        let source = "void Test() {}\n";
        let mut corpus = CompilerProbeCorpusV1 {
            schema: PROBE_CORPUS_SCHEMA.to_owned(),
            schema_version: QUALIFICATION_SCHEMA_VERSION,
            suite_id: "product-resolver-test-v1".to_owned(),
            cases: vec![CompilerProbeCaseV1 {
                ordinal: 0,
                case_id: "positive.compile".to_owned(),
                category: "smoke".to_owned(),
                expected_outcome: ProbeOutcomeV1::Accepted,
                mode: ProbeModeV1::CompileOnly,
                sections: vec![ProbeSourceSectionV1 {
                    ordinal: 0,
                    module: "Module".to_owned(),
                    relative_path: "Module.as".to_owned(),
                    source_utf8: source.to_owned(),
                    source_sha256: sha256(source.as_bytes()),
                }],
            }],
            canonical_sha256: Sha256Digest::from_bytes([0; 32]),
        };
        corpus.seal().unwrap();
        let semantic_sha256 = sha256(b"normalized-semantic-result");
        let mut expected = ExpectedProbeResultsV1 {
            schema: EXPECTED_RESULTS_SCHEMA.to_owned(),
            schema_version: QUALIFICATION_SCHEMA_VERSION,
            suite_id: corpus.suite_id.clone(),
            corpus_sha256: corpus.canonical_sha256,
            results: vec![ExpectedProbeResultV1 {
                ordinal: 0,
                case_id: corpus.cases[0].case_id.clone(),
                outcome: ProbeOutcomeV1::Accepted,
                diagnostics: vec![],
                semantic_sha256: Some(semantic_sha256),
            }],
            canonical_sha256: Sha256Digest::from_bytes([0; 32]),
        };
        expected.seal().unwrap();
        let diagnostic_sha256 = expected.results[0].diagnostics_sha256().unwrap();
        let mut diagnostics = DiagnosticParityReportV1 {
            schema: DIAGNOSTIC_PARITY_SCHEMA.to_owned(),
            schema_version: QUALIFICATION_SCHEMA_VERSION,
            suite_id: corpus.suite_id.clone(),
            corpus_sha256: corpus.canonical_sha256,
            expected_results_sha256: expected.canonical_sha256,
            standalone_compiler: sidecar,
            entries: vec![DiagnosticParityEntryV1 {
                ordinal: 0,
                case_id: corpus.cases[0].case_id.clone(),
                expected_sha256: diagnostic_sha256,
                embedded_sha256: diagnostic_sha256,
                standalone_sha256: diagnostic_sha256,
            }],
            canonical_sha256: Sha256Digest::from_bytes([0; 32]),
        };
        diagnostics.seal().unwrap();
        let mut semantics = SemanticParityReportV1 {
            schema: SEMANTIC_PARITY_SCHEMA.to_owned(),
            schema_version: QUALIFICATION_SCHEMA_VERSION,
            suite_id: corpus.suite_id.clone(),
            corpus_sha256: corpus.canonical_sha256,
            expected_results_sha256: expected.canonical_sha256,
            standalone_compiler: sidecar,
            entries: vec![SemanticParityEntryV1 {
                ordinal: 0,
                case_id: corpus.cases[0].case_id.clone(),
                expected_sha256: semantic_sha256,
                embedded_sha256: semantic_sha256,
                standalone_sha256: semantic_sha256,
            }],
            unexplained_differences: vec![],
            qualified: true,
            canonical_sha256: Sha256Digest::from_bytes([0; 32]),
        };
        semantics.seal().unwrap();
        (corpus, expected, diagnostics, semantics)
    }

    #[cfg(windows)]
    fn synthetic_pe() -> (Vec<u8>, PeCodeViewV1) {
        let mut bytes = vec![0u8; 0x500];
        bytes[0..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        bytes[0x84..0x86].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
        bytes[0x94..0x96].copy_from_slice(&0xf0u16.to_le_bytes());
        let optional = 0x98usize;
        bytes[optional..optional + 2].copy_from_slice(&0x20bu16.to_le_bytes());
        bytes[optional + 108..optional + 112].copy_from_slice(&16u32.to_le_bytes());
        bytes[optional + 160..optional + 164].copy_from_slice(&0x1100u32.to_le_bytes());
        bytes[optional + 164..optional + 168].copy_from_slice(&28u32.to_le_bytes());
        let section = optional + 0xf0;
        bytes[section + 8..section + 12].copy_from_slice(&0x200u32.to_le_bytes());
        bytes[section + 12..section + 16].copy_from_slice(&0x1000u32.to_le_bytes());
        bytes[section + 16..section + 20].copy_from_slice(&0x300u32.to_le_bytes());
        bytes[section + 20..section + 24].copy_from_slice(&0x200u32.to_le_bytes());
        let debug = 0x300usize;
        bytes[debug + 12..debug + 16].copy_from_slice(&2u32.to_le_bytes());
        bytes[debug + 16..debug + 20].copy_from_slice(&24u32.to_le_bytes());
        bytes[debug + 24..debug + 28].copy_from_slice(&0x380u32.to_le_bytes());
        let guid = [
            0x67, 0x45, 0x23, 0x01, 0xab, 0x89, 0xef, 0xcd, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
            0xcd, 0xef,
        ];
        bytes[0x380..0x384].copy_from_slice(b"RSDS");
        bytes[0x384..0x394].copy_from_slice(&guid);
        bytes[0x394..0x398].copy_from_slice(&7u32.to_le_bytes());
        (
            bytes,
            PeCodeViewV1 {
                guid: "01234567-89ab-cdef-0123-456789abcdef".to_owned(),
                age: 7,
            },
        )
    }

    #[cfg(windows)]
    fn sha256(bytes: &[u8]) -> Sha256Digest {
        Sha256Digest::from_bytes(Sha256::digest(bytes).into())
    }

    #[test]
    fn production_catalog_resolves_to_explicit_bundle_absent_without_touching_paths() {
        let target = ProductStandaloneCompilerTargetV1::try_new(
            CompilerTargetV1 {
                steam_app_id: 1,
                steam_build_id: 1,
                depot_id: 1,
                depot_manifest_gid: 1,
                platform: CompilerPlatformV1::Windows,
                architecture: CompilerArchitectureV1::X86_64,
                build_configuration: CompilerBuildConfigurationV1::Shipping,
            },
            PeCodeViewV1 {
                guid: "01234567-89ab-cdef-0123-456789abcdef".to_owned(),
                age: 1,
            },
        )
        .unwrap();
        let missing = Path::new("Z:/definitely/not/a/real/package/or/target");
        assert!(matches!(
            resolve_embedded_product_standalone_compiler_package_v1(
                missing,
                &target,
                CompilerTargetInputPathsV1 {
                    executable: missing,
                    shipping_cache: missing,
                    binds_cache: missing,
                },
            ),
            ProductStandaloneCompilerPackageResolutionV1::BundleAbsent
        ));
        assert!(matches!(
            resolve_embedded_product_standalone_compiler_package_for_inputs_v1(
                missing,
                CompilerTargetInputPathsV1 {
                    executable: missing,
                    shipping_cache: missing,
                    binds_cache: missing,
                },
            ),
            ProductStandaloneCompilerPackageResolutionV1::BundleAbsent
        ));
    }

    #[cfg(windows)]
    #[test]
    fn available_package_pins_full_authority_chain_and_refuses_tamper_junction_and_wrong_target() {
        let fixture = SyntheticProductPackageFixtureV1::create();
        let auto_selected = match fixture.resolve_for_inputs() {
            ProductStandaloneCompilerPackageResolutionV1::Available(value) => value,
            other => panic!("expected automatic exact target selection, got {other:?}"),
        };
        assert_eq!(auto_selected.identity().target(), &fixture.target);
        drop(auto_selected);

        let mut wrong_reference_json: serde_json::Value =
            serde_json::from_slice(&fixture.catalog_bytes).unwrap();
        wrong_reference_json["qualification_reference"]["sha256"] =
            serde_json::json!(sha256(b"different-qualified-build"));
        let wrong_reference_bytes = serde_json::to_vec_pretty(&wrong_reference_json).unwrap();
        let wrong_reference_catalog =
            ProductStandaloneCompilerCatalogV1::from_json(&wrong_reference_bytes).unwrap();
        assert!(matches!(
            resolve_authoritative_catalog_at_host_v1(
                &wrong_reference_catalog,
                sha256(&wrong_reference_bytes),
                &fixture.host_module,
                &fixture.target,
                fixture.target_paths(),
            ),
            ProductStandaloneCompilerPackageResolutionV1::Unavailable(ref value)
                if value.kind()
                    == ProductStandaloneCompilerPackageUnavailableKindV1::QualificationIdentityMismatch
        ));

        let available = match fixture.resolve() {
            ProductStandaloneCompilerPackageResolutionV1::Available(value) => value,
            other => panic!("expected available synthetic package, got {other:?}"),
        };
        assert_eq!(
            available.identity().catalog_sha256(),
            sha256(&fixture.catalog_bytes)
        );
        assert_eq!(
            available.identity().profile_sha256(),
            available.profile_package().profile().profile_sha256
        );
        assert_eq!(
            available.identity().compatibility_id(),
            STANDALONE_COMPILER_COMPATIBILITY_ID_V1
        );
        assert_ne!(
            available.identity().sidecar_sha256(),
            available
                .profile_package()
                .standalone_compiler_identity()
                .sha256,
            "a rebuilt/signed sidecar must be accepted at the qualified semantic ABI"
        );
        let product_scratch = fixture._temp.path().join("product-scratch");
        std::fs::create_dir(&product_scratch).unwrap();
        available
            .sidecar_runner(product_scratch)
            .expect("product runner must retain the resolver's compatibility decision");
        assert_eq!(
            fixture.catalog.sidecar().protocol().request_version(),
            SIDECAR_REQUEST_VERSION_V2
        );
        assert_eq!(
            available
                .profile_package()
                .standalone_compiler_identity()
                .request_version,
            QUALIFIED_SIDECAR_REQUEST_VERSION_V2
        );
        assert_eq!(
            available.identity().request_version(),
            SIDECAR_REQUEST_VERSION_V2
        );
        assert_eq!(available.identity().target(), &fixture.target);
        assert_eq!(
            available.target_inputs().shipping_cache(),
            fixture.shipping_bytes
        );
        assert_eq!(available.target_inputs().binds_cache(), fixture.binds_bytes);
        let (authority, target_inputs) = available.into_execution_parts();
        assert_eq!(
            authority.identity().profile_sha256(),
            authority.profile_package().profile().profile_sha256
        );
        assert!(std::fs::write(
            &fixture.sidecar_path,
            b"synthetic-rebuilt-and-signed-sidecar"
        )
        .is_err());
        assert!(std::fs::write(&fixture.manifest_path, &fixture.manifest_bytes).is_err());
        assert!(std::fs::write(
            fixture
                .package_root
                .join("profiles/build-24539464/engine/properties.json"),
            b"replacement",
        )
        .is_err());
        assert!(std::fs::write(&fixture.shipping_path, b"replacement").is_err());
        assert!(std::fs::write(&fixture.binds_path, b"replacement").is_err());
        assert!(std::fs::write(&fixture.executable_path, b"replacement").is_err());
        assert!(std::fs::rename(
            &fixture.package_root,
            fixture.app_root.join("compiler-moved")
        )
        .is_err());
        drop(target_inputs);
        assert!(std::fs::write(&fixture.shipping_path, &fixture.shipping_bytes).is_ok());
        assert!(std::fs::write(
            &fixture.sidecar_path,
            b"synthetic-rebuilt-and-signed-sidecar"
        )
        .is_err());
        drop(authority);

        let original_sidecar = std::fs::read(&fixture.sidecar_path).unwrap();
        let mut tampered_sidecar = original_sidecar.clone();
        tampered_sidecar[0] ^= 1;
        std::fs::write(&fixture.sidecar_path, &tampered_sidecar).unwrap();
        assert!(matches!(
            fixture.resolve(),
            ProductStandaloneCompilerPackageResolutionV1::Unavailable(ref value)
                if value.kind() == ProductStandaloneCompilerPackageUnavailableKindV1::SidecarAuthentication
        ));
        std::fs::write(&fixture.sidecar_path, &original_sidecar).unwrap();

        let mut tampered_manifest = fixture.manifest_bytes.clone();
        tampered_manifest[0] ^= 1;
        std::fs::write(&fixture.manifest_path, &tampered_manifest).unwrap();
        assert!(matches!(
            fixture.resolve(),
            ProductStandaloneCompilerPackageResolutionV1::Unavailable(ref value)
                if value.kind() == ProductStandaloneCompilerPackageUnavailableKindV1::ProfileManifestAuthentication
        ));
        std::fs::write(&fixture.manifest_path, &fixture.manifest_bytes).unwrap();

        let blob_path = fixture
            .package_root
            .join("profiles/build-24539464/engine/properties.json");
        let original_blob = std::fs::read(&blob_path).unwrap();
        let mut tampered_blob = original_blob.clone();
        tampered_blob[0] ^= 1;
        std::fs::write(&blob_path, &tampered_blob).unwrap();
        assert!(matches!(
            fixture.resolve(),
            ProductStandaloneCompilerPackageResolutionV1::Unavailable(ref value)
                if value.kind() == ProductStandaloneCompilerPackageUnavailableKindV1::ProfilePayloadAuthentication
        ));
        std::fs::write(&blob_path, &original_blob).unwrap();

        let original_executable = std::fs::read(&fixture.executable_path).unwrap();
        let mut repackaged_executable = original_executable.clone();
        *repackaged_executable.last_mut().unwrap() ^= 1;
        std::fs::write(&fixture.executable_path, &repackaged_executable).unwrap();
        assert!(matches!(
            fixture.resolve_for_inputs(),
            ProductStandaloneCompilerPackageResolutionV1::Available(_)
        ));
        std::fs::write(&fixture.executable_path, &original_executable).unwrap();

        let wide_binds = synthetic_binds_database(true);
        assert_ne!(wide_binds, fixture.binds_bytes);
        std::fs::write(&fixture.binds_path, &wide_binds).unwrap();
        assert!(matches!(
            fixture.resolve_for_inputs(),
            ProductStandaloneCompilerPackageResolutionV1::Available(_)
        ));
        std::fs::write(&fixture.binds_path, &fixture.binds_bytes).unwrap();

        let original_shipping = std::fs::read(&fixture.shipping_path).unwrap();
        let mut repackaged_shipping = original_shipping.clone();
        repackaged_shipping[0] ^= 1; // Per-cache GUID, not compiler compatibility.
        std::fs::write(&fixture.shipping_path, &repackaged_shipping).unwrap();
        assert!(matches!(
            fixture.resolve_for_inputs(),
            ProductStandaloneCompilerPackageResolutionV1::Available(_)
        ));
        let mut incompatible_shipping = original_shipping.clone();
        incompatible_shipping[16..20].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        std::fs::write(&fixture.shipping_path, &incompatible_shipping).unwrap();
        assert!(matches!(
            fixture.resolve(),
            ProductStandaloneCompilerPackageResolutionV1::Unavailable(ref value)
                if value.kind() == ProductStandaloneCompilerPackageUnavailableKindV1::TargetAuthentication
        ));
        assert!(matches!(
            fixture.resolve_for_inputs(),
            ProductStandaloneCompilerPackageResolutionV1::Unavailable(ref value)
                if value.kind() == ProductStandaloneCompilerPackageUnavailableKindV1::UnsupportedTarget
        ));
        std::fs::write(&fixture.shipping_path, &original_shipping).unwrap();

        let mut wrong_target = fixture.target.target().clone();
        wrong_target.steam_build_id += 1;
        let wrong_target = ProductStandaloneCompilerTargetV1::try_new(
            wrong_target,
            fixture.target.pe_codeview().clone(),
        )
        .unwrap();
        let wrong = resolve_authoritative_catalog_at_host_v1(
            &fixture.catalog,
            sha256(&fixture.catalog_bytes),
            &fixture.host_module,
            &wrong_target,
            CompilerTargetInputPathsV1 {
                executable: &fixture.executable_path,
                shipping_cache: &fixture.shipping_path,
                binds_cache: &fixture.binds_path,
            },
        );
        assert!(matches!(
            wrong,
            ProductStandaloneCompilerPackageResolutionV1::Unavailable(ref value)
                if value.kind() == ProductStandaloneCompilerPackageUnavailableKindV1::UnsupportedTarget
        ));

        let receipt_authority = fixture.receipt_authority();
        assert_eq!(
            receipt_authority.identity().profile_sha256(),
            receipt_authority.profile_package().profile().profile_sha256
        );
        drop(receipt_authority);

        let mut second_profile = CompilerProfileV1::from_json(&fixture.manifest_bytes).unwrap();
        second_profile.target.steam_build_id += 1;
        second_profile.seal().unwrap();
        let second_manifest_path = fixture
            .manifest_path
            .parent()
            .unwrap()
            .join("compiler-profile-ambiguous.json");
        let compatible_manifest_bytes = serde_json::to_vec(&second_profile).unwrap();
        std::fs::write(&second_manifest_path, &compatible_manifest_bytes).unwrap();
        let second_target = ProductStandaloneCompilerTargetV1::try_new(
            second_profile.target.clone(),
            second_profile.oracle.pe_codeview.clone(),
        )
        .unwrap();
        let mut compatible_json: serde_json::Value =
            serde_json::from_slice(&fixture.catalog_bytes).unwrap();
        compatible_json["profiles"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "manifest_relative_path": "profiles/build-24539464/compiler-profile-ambiguous.json",
                "manifest_byte_len": compatible_manifest_bytes.len(),
                "manifest_sha256": sha256(&compatible_manifest_bytes),
                "profile_sha256": second_profile.profile_sha256,
                "target": second_target
            }));
        let compatible_bytes = serde_json::to_vec_pretty(&compatible_json).unwrap();
        let compatible_catalog =
            ProductStandaloneCompilerCatalogV1::from_json(&compatible_bytes).unwrap();
        assert!(matches!(
            resolve_authoritative_catalog_for_inputs_v1(
                &compatible_catalog,
                sha256(&compatible_bytes),
                &fixture.host_module,
                fixture.target_paths(),
            ),
            ProductStandaloneCompilerPackageResolutionV1::Available(_)
        ));

        second_profile.engine.as_create_version += 1;
        second_profile.seal().unwrap();
        let second_manifest_bytes = serde_json::to_vec(&second_profile).unwrap();
        std::fs::write(&second_manifest_path, &second_manifest_bytes).unwrap();
        let mut ambiguous_json: serde_json::Value =
            serde_json::from_slice(&fixture.catalog_bytes).unwrap();
        ambiguous_json["profiles"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "manifest_relative_path": "profiles/build-24539464/compiler-profile-ambiguous.json",
                "manifest_byte_len": second_manifest_bytes.len(),
                "manifest_sha256": sha256(&second_manifest_bytes),
                "profile_sha256": second_profile.profile_sha256,
                "target": second_target
            }));
        let ambiguous_bytes = serde_json::to_vec_pretty(&ambiguous_json).unwrap();
        let ambiguous_catalog =
            ProductStandaloneCompilerCatalogV1::from_json(&ambiguous_bytes).unwrap();
        assert!(matches!(
            resolve_authoritative_catalog_for_inputs_v1(
                &ambiguous_catalog,
                sha256(&ambiguous_bytes),
                &fixture.host_module,
                fixture.target_paths(),
            ),
            ProductStandaloneCompilerPackageResolutionV1::Unavailable(ref value)
                if value.kind() == ProductStandaloneCompilerPackageUnavailableKindV1::AmbiguousTarget
        ));

        let profiles = fixture.package_root.join("profiles");
        let junction_target = fixture._temp.path().join("junction-target");
        std::fs::rename(&profiles, &junction_target).unwrap();
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&profiles)
            .arg(&junction_target)
            .status()
            .unwrap();
        assert!(
            status.success(),
            "creating the junction fixture must succeed"
        );
        assert!(matches!(
            fixture.resolve(),
            ProductStandaloneCompilerPackageResolutionV1::Unavailable(ref value)
                if value.kind() == ProductStandaloneCompilerPackageUnavailableKindV1::UnsafePackagePath
        ));
    }
}
