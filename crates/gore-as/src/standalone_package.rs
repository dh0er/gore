//! Product-owned authority contract for a packaged standalone AngelScript compiler.
//!
//! Parsing a catalog proves only that its syntax and internal invariants are valid. Product
//! authority comes from catalog bytes embedded in the GORE binary. An optional on-disk copy is
//! informational and must match those bytes exactly before it may be used. The production catalog
//! intentionally remains absent until a real compiler profile has passed oracle qualification.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::compiler_profile::manifest::{
    CompilerProfileError, CompilerProfileV1, CompilerTargetV1, PeCodeViewV1, Sha256Digest,
    MAX_COMPILER_PROFILE_JSON_BYTES,
};
use crate::standalone_sidecar::{
    SIDECAR_REQUEST_VERSION_V1, SIDECAR_REQUEST_VERSION_V2, SIDECAR_RESPONSE_VERSION_V1,
};

pub const PRODUCT_STANDALONE_COMPILER_CATALOG_SCHEMA_V1: &str =
    "gore.as.product-standalone-compiler-catalog";
pub const PRODUCT_STANDALONE_COMPILER_CATALOG_SCHEMA_VERSION_V1: u32 = 1;
pub const PRODUCT_STANDALONE_COMPILER_PACKAGE_ROOT_V1: &str = "compiler";
pub const STANDALONE_COMPILER_COMPATIBILITY_ID_V1: &str = "gore-as-standalone-semantic-v1";
pub const MAX_PRODUCT_STANDALONE_COMPILER_CATALOG_JSON_BYTES_V1: usize = 256 * 1024;
pub const MAX_PRODUCT_STANDALONE_COMPILER_PROFILES_V1: usize = 64;
pub const MAX_PRODUCT_STANDALONE_COMPILER_RELATIVE_PATH_BYTES_V1: usize = 512;
pub const MAX_PRODUCT_STANDALONE_COMPILER_PATH_COMPONENT_BYTES_V1: usize = 128;
pub const MAX_STANDALONE_COMPILER_COMPATIBILITY_ID_BYTES_V1: usize = 128;
pub const MAX_PRODUCT_STANDALONE_COMPILER_SIDECAR_BYTES_V1: u64 = 256 * 1024 * 1024;
pub const EMBEDDED_PRODUCT_STANDALONE_COMPILER_CATALOG_BUILD_SHA256_HEX_V1: &str =
    env!("GORE_EMBEDDED_STANDALONE_COMPILER_CATALOG_SHA256");
pub const EMBEDDED_PRODUCT_STANDALONE_COMPILER_CATALOG_BUILD_MARKER_PREFIX_V1: &str =
    "GORE_AS_EMBEDDED_COMPILER_CATALOG_SHA256=";
pub const EMBEDDED_PRODUCT_STANDALONE_COMPILER_CATALOG_BUILD_MARKER_V1: &str = concat!(
    "GORE_AS_EMBEDDED_COMPILER_CATALOG_SHA256=",
    env!("GORE_EMBEDDED_STANDALONE_COMPILER_CATALOG_SHA256")
);

/// Sole production package authority.
///
/// Empty means that this build contains no product-qualified standalone compiler bundle. It is not
/// valid catalog JSON and must be mapped to `bundle_absent`, never to an invalid external package.
pub const EMBEDDED_PRODUCT_STANDALONE_COMPILER_CATALOG_JSON_V1: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/product-standalone-compiler-catalog.json"
));

/// Build-time identity embedded alongside the exact catalog bytes.
///
/// Release tooling verifies the marker in every product host after linking. Runtime catalog
/// resolution independently hashes the included bytes against this digest before parsing them.
pub fn embedded_product_standalone_compiler_catalog_build_identity_v1(
) -> Result<(&'static str, Sha256Digest), ProductStandaloneCompilerCatalogError> {
    let encoded = EMBEDDED_PRODUCT_STANDALONE_COMPILER_CATALOG_BUILD_MARKER_V1
        .strip_prefix(EMBEDDED_PRODUCT_STANDALONE_COMPILER_CATALOG_BUILD_MARKER_PREFIX_V1)
        .ok_or(ProductStandaloneCompilerCatalogError::EmbeddedBuildIdentityInvalid)?;
    let digest = Sha256Digest::from_hex(encoded)
        .map_err(|_| ProductStandaloneCompilerCatalogError::EmbeddedBuildIdentityInvalid)?;
    Ok((
        EMBEDDED_PRODUCT_STANDALONE_COMPILER_CATALOG_BUILD_MARKER_V1,
        digest,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductStandaloneCompilerProtocolV1 {
    request_version: u32,
    response_version: u32,
}

impl ProductStandaloneCompilerProtocolV1 {
    pub fn request_version(self) -> u32 {
        self.request_version
    }

    pub fn response_version(self) -> u32 {
        self.response_version
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductStandaloneCompilerSidecarV1 {
    relative_path: String,
    byte_len: u64,
    sha256: Sha256Digest,
    compatibility_id: String,
    protocol: ProductStandaloneCompilerProtocolV1,
    /// Release verification must prove that the PE imports only statically named system DLLs.
    /// Runtime catalog validation refuses any weaker package policy.
    static_system_only: bool,
}

impl ProductStandaloneCompilerSidecarV1 {
    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }

    pub fn byte_len(&self) -> u64 {
        self.byte_len
    }

    pub fn sha256(&self) -> Sha256Digest {
        self.sha256
    }

    pub fn compatibility_id(&self) -> &str {
        &self.compatibility_id
    }

    pub fn protocol(&self) -> ProductStandaloneCompilerProtocolV1 {
        self.protocol
    }

    pub fn static_system_only(&self) -> bool {
        self.static_system_only
    }
}

/// Historical standalone compiler used to produce the sealed parity reports.
///
/// This is deliberately distinct from [`ProductStandaloneCompilerSidecarV1`]: reproducible
/// rebuilding and release signing may change executable bytes without changing compiler
/// semantics. Product execution accepts that rebuilt sidecar only when both records declare the
/// same semantic compatibility and wire protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductStandaloneCompilerQualificationReferenceV1 {
    byte_len: u64,
    sha256: Sha256Digest,
    compatibility_id: String,
    protocol: ProductStandaloneCompilerProtocolV1,
}

impl ProductStandaloneCompilerQualificationReferenceV1 {
    pub fn byte_len(&self) -> u64 {
        self.byte_len
    }

    pub fn sha256(&self) -> Sha256Digest {
        self.sha256
    }

    pub fn compatibility_id(&self) -> &str {
        &self.compatibility_id
    }

    pub fn protocol(&self) -> ProductStandaloneCompilerProtocolV1 {
        self.protocol
    }
}

/// Exact game/compiler identity used to select one qualified profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductStandaloneCompilerTargetV1 {
    target: CompilerTargetV1,
    pe_codeview: PeCodeViewV1,
}

impl ProductStandaloneCompilerTargetV1 {
    pub fn try_new(
        target: CompilerTargetV1,
        pe_codeview: PeCodeViewV1,
    ) -> Result<Self, ProductStandaloneCompilerCatalogError> {
        let value = Self {
            target,
            pe_codeview,
        };
        validate_target(&value)?;
        Ok(value)
    }

    pub fn target(&self) -> &CompilerTargetV1 {
        &self.target
    }

    pub fn pe_codeview(&self) -> &PeCodeViewV1 {
        &self.pe_codeview
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductStandaloneCompilerProfileV1 {
    manifest_relative_path: String,
    manifest_byte_len: u64,
    manifest_sha256: Sha256Digest,
    profile_sha256: Sha256Digest,
    target: ProductStandaloneCompilerTargetV1,
}

impl ProductStandaloneCompilerProfileV1 {
    pub fn manifest_relative_path(&self) -> &str {
        &self.manifest_relative_path
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

/// Immutable catalog parsed from authority-bearing embedded bytes.
///
/// Fields stay private so validation cannot be bypassed by mutating a successfully parsed value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductStandaloneCompilerCatalogV1 {
    schema: String,
    schema_version: u32,
    sidecar: ProductStandaloneCompilerSidecarV1,
    qualification_reference: ProductStandaloneCompilerQualificationReferenceV1,
    profiles: Vec<ProductStandaloneCompilerProfileV1>,
}

impl ProductStandaloneCompilerCatalogV1 {
    pub fn from_json(bytes: &[u8]) -> Result<Self, ProductStandaloneCompilerCatalogError> {
        if bytes.len() > MAX_PRODUCT_STANDALONE_COMPILER_CATALOG_JSON_BYTES_V1 {
            return Err(ProductStandaloneCompilerCatalogError::JsonTooLarge {
                actual: bytes.len(),
                max: MAX_PRODUCT_STANDALONE_COMPILER_CATALOG_JSON_BYTES_V1,
            });
        }
        let catalog: Self = serde_json::from_slice(bytes)?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn to_json(&self) -> Result<Vec<u8>, ProductStandaloneCompilerCatalogError> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self)?;
        if bytes.len() > MAX_PRODUCT_STANDALONE_COMPILER_CATALOG_JSON_BYTES_V1 {
            return Err(ProductStandaloneCompilerCatalogError::JsonTooLarge {
                actual: bytes.len(),
                max: MAX_PRODUCT_STANDALONE_COMPILER_CATALOG_JSON_BYTES_V1,
            });
        }
        Ok(bytes)
    }

    pub fn schema(&self) -> &str {
        &self.schema
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn sidecar(&self) -> &ProductStandaloneCompilerSidecarV1 {
        &self.sidecar
    }

    pub fn qualification_reference(&self) -> &ProductStandaloneCompilerQualificationReferenceV1 {
        &self.qualification_reference
    }

    pub fn profiles(&self) -> &[ProductStandaloneCompilerProfileV1] {
        &self.profiles
    }

    /// Select exactly one profile from an already validated catalog.
    pub fn profile_for_target(
        &self,
        target: &ProductStandaloneCompilerTargetV1,
    ) -> Result<&ProductStandaloneCompilerProfileV1, ProductStandaloneCompilerCatalogError> {
        self.profiles
            .iter()
            .find(|profile| target_equal(&profile.target, target))
            .ok_or(ProductStandaloneCompilerCatalogError::UnsupportedTarget)
    }

    /// Authenticate a sidecar seal measured from a no-follow, handle-pinned file.
    pub fn authenticate_sidecar(
        &self,
        actual_byte_len: u64,
        actual_sha256: Sha256Digest,
    ) -> Result<&ProductStandaloneCompilerSidecarV1, ProductStandaloneCompilerCatalogError> {
        if actual_byte_len != self.sidecar.byte_len || actual_sha256 != self.sidecar.sha256 {
            return Err(
                ProductStandaloneCompilerCatalogError::SidecarAuthenticationMismatch {
                    expected_byte_len: self.sidecar.byte_len,
                    actual_byte_len,
                    expected_sha256: self.sidecar.sha256,
                    actual_sha256,
                },
            );
        }
        Ok(&self.sidecar)
    }

    /// Authenticate and parse the exact qualified profile manifest selected by the target tuple.
    pub fn authenticate_profile_manifest(
        &self,
        target: &ProductStandaloneCompilerTargetV1,
        manifest_bytes: &[u8],
    ) -> Result<
        AuthenticatedProductStandaloneCompilerProfileV1<'_>,
        ProductStandaloneCompilerCatalogError,
    > {
        let entry = self.profile_for_target(target)?;
        let actual_byte_len = manifest_bytes.len() as u64;
        if actual_byte_len != entry.manifest_byte_len {
            return Err(
                ProductStandaloneCompilerCatalogError::ProfileManifestLengthMismatch {
                    expected_byte_len: entry.manifest_byte_len,
                    actual_byte_len,
                },
            );
        }
        let actual_sha256 = sha256(manifest_bytes);
        if actual_sha256 != entry.manifest_sha256 {
            return Err(
                ProductStandaloneCompilerCatalogError::ProfileManifestAuthenticationMismatch {
                    expected_byte_len: entry.manifest_byte_len,
                    actual_byte_len,
                    expected_sha256: entry.manifest_sha256,
                    actual_sha256,
                },
            );
        }

        let profile = CompilerProfileV1::from_json(manifest_bytes)?;
        if profile.profile_sha256 != entry.profile_sha256
            || profile.target != entry.target.target
            || !codeview_equal(&profile.oracle.pe_codeview, &entry.target.pe_codeview)
        {
            return Err(ProductStandaloneCompilerCatalogError::ProfileIdentityMismatch);
        }
        Ok(AuthenticatedProductStandaloneCompilerProfileV1 { entry, profile })
    }

    fn validate(&self) -> Result<(), ProductStandaloneCompilerCatalogError> {
        if self.schema != PRODUCT_STANDALONE_COMPILER_CATALOG_SCHEMA_V1 {
            return Err(ProductStandaloneCompilerCatalogError::UnsupportedSchema(
                self.schema.clone(),
            ));
        }
        if self.schema_version != PRODUCT_STANDALONE_COMPILER_CATALOG_SCHEMA_VERSION_V1 {
            return Err(
                ProductStandaloneCompilerCatalogError::UnsupportedSchemaVersion(
                    self.schema_version,
                ),
            );
        }
        if self.profiles.is_empty() {
            return Err(ProductStandaloneCompilerCatalogError::ProfilesEmpty);
        }
        if self.profiles.len() > MAX_PRODUCT_STANDALONE_COMPILER_PROFILES_V1 {
            return Err(ProductStandaloneCompilerCatalogError::TooManyProfiles {
                actual: self.profiles.len(),
                max: MAX_PRODUCT_STANDALONE_COMPILER_PROFILES_V1,
            });
        }

        validate_relative_package_path("sidecar.relative_path", &self.sidecar.relative_path)?;
        if !self
            .sidecar
            .relative_path
            .to_ascii_lowercase()
            .ends_with(".exe")
        {
            return Err(ProductStandaloneCompilerCatalogError::InvalidPathKind {
                field: "sidecar.relative_path",
                required_suffix: ".exe",
            });
        }
        validate_nonzero_seal(
            "sidecar",
            self.sidecar.byte_len,
            self.sidecar.sha256,
            MAX_PRODUCT_STANDALONE_COMPILER_SIDECAR_BYTES_V1,
        )?;
        validate_standalone_compiler_compatibility_id_v1(
            "sidecar.compatibility_id",
            &self.sidecar.compatibility_id,
        )?;
        if !matches!(
            self.sidecar.protocol.request_version,
            SIDECAR_REQUEST_VERSION_V1 | SIDECAR_REQUEST_VERSION_V2
        ) || self.sidecar.protocol.response_version != SIDECAR_RESPONSE_VERSION_V1
        {
            return Err(ProductStandaloneCompilerCatalogError::UnsupportedProtocol {
                request_version: self.sidecar.protocol.request_version,
                response_version: self.sidecar.protocol.response_version,
            });
        }
        if !self.sidecar.static_system_only {
            return Err(ProductStandaloneCompilerCatalogError::StaticSystemOnlyRequired);
        }

        validate_nonzero_seal(
            "qualification_reference",
            self.qualification_reference.byte_len,
            self.qualification_reference.sha256,
            MAX_PRODUCT_STANDALONE_COMPILER_SIDECAR_BYTES_V1,
        )?;
        validate_standalone_compiler_compatibility_id_v1(
            "qualification_reference.compatibility_id",
            &self.qualification_reference.compatibility_id,
        )?;
        let qualification_protocol = self.qualification_reference.protocol;
        if !matches!(
            qualification_protocol.request_version,
            SIDECAR_REQUEST_VERSION_V1 | SIDECAR_REQUEST_VERSION_V2
        ) || qualification_protocol.response_version != SIDECAR_RESPONSE_VERSION_V1
        {
            return Err(ProductStandaloneCompilerCatalogError::UnsupportedProtocol {
                request_version: qualification_protocol.request_version,
                response_version: qualification_protocol.response_version,
            });
        }
        if self.sidecar.compatibility_id != self.qualification_reference.compatibility_id {
            return Err(ProductStandaloneCompilerCatalogError::QualificationCompatibilityMismatch);
        }
        if self.sidecar.protocol != qualification_protocol {
            return Err(ProductStandaloneCompilerCatalogError::QualificationProtocolMismatch);
        }

        let mut paths = BTreeSet::<String>::new();
        insert_unique_path(&mut paths, &self.sidecar.relative_path)?;
        let mut targets = BTreeSet::new();
        for (index, profile) in self.profiles.iter().enumerate() {
            validate_relative_package_path(
                "profiles[].manifest_relative_path",
                &profile.manifest_relative_path,
            )?;
            if !profile
                .manifest_relative_path
                .to_ascii_lowercase()
                .ends_with(".json")
            {
                return Err(ProductStandaloneCompilerCatalogError::InvalidPathKind {
                    field: "profiles[].manifest_relative_path",
                    required_suffix: ".json",
                });
            }
            insert_unique_path(&mut paths, &profile.manifest_relative_path)?;
            validate_nonzero_seal(
                "profiles[].manifest",
                profile.manifest_byte_len,
                profile.manifest_sha256,
                MAX_COMPILER_PROFILE_JSON_BYTES as u64,
            )?;
            if profile.profile_sha256 == zero_sha256() {
                return Err(ProductStandaloneCompilerCatalogError::ZeroDigest {
                    field: "profiles[].profile_sha256",
                });
            }
            validate_target(&profile.target)?;
            if !targets.insert(target_key(&profile.target)) {
                return Err(ProductStandaloneCompilerCatalogError::DuplicateTarget);
            }
            if index > 0
                && compare_profile_entries(&self.profiles[index - 1], profile) != Ordering::Less
            {
                return Err(ProductStandaloneCompilerCatalogError::ProfilesNotSorted);
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct AuthenticatedProductStandaloneCompilerProfileV1<'a> {
    entry: &'a ProductStandaloneCompilerProfileV1,
    profile: CompilerProfileV1,
}

impl<'a> AuthenticatedProductStandaloneCompilerProfileV1<'a> {
    pub fn entry(&self) -> &'a ProductStandaloneCompilerProfileV1 {
        self.entry
    }

    pub fn profile(&self) -> &CompilerProfileV1 {
        &self.profile
    }

    pub fn into_profile(self) -> CompilerProfileV1 {
        self.profile
    }
}

/// Resolve the catalog compiled into this product build.
///
/// `Ok(None)` is the intentional `bundle_absent` state. An embedded non-empty but invalid catalog
/// is an internal packaging error and is never downgraded to absence.
pub fn embedded_product_standalone_compiler_catalog_v1(
) -> Result<Option<ProductStandaloneCompilerCatalogV1>, ProductStandaloneCompilerCatalogError> {
    let (_, expected_sha256) = embedded_product_standalone_compiler_catalog_build_identity_v1()?;
    let actual_sha256 = sha256(EMBEDDED_PRODUCT_STANDALONE_COMPILER_CATALOG_JSON_V1);
    if actual_sha256 != expected_sha256 {
        return Err(
            ProductStandaloneCompilerCatalogError::EmbeddedCatalogBuildSealMismatch {
                expected_sha256,
                actual_sha256,
            },
        );
    }
    if EMBEDDED_PRODUCT_STANDALONE_COMPILER_CATALOG_JSON_V1.is_empty() {
        return Ok(None);
    }
    ProductStandaloneCompilerCatalogV1::from_json(
        EMBEDDED_PRODUCT_STANDALONE_COMPILER_CATALOG_JSON_V1,
    )
    .map(Some)
}

/// Authenticate an optional on-disk audit copy against exact trusted embedded bytes.
///
/// JSON-equivalent reserialization is intentionally rejected: the embedded bytes are the trust
/// root, and accepting alternate bytes would make catalog evidence ambiguous.
pub fn authenticate_product_standalone_compiler_catalog_copy_v1(
    trusted_embedded_bytes: &[u8],
    candidate_bytes: &[u8],
) -> Result<ProductStandaloneCompilerCatalogV1, ProductStandaloneCompilerCatalogError> {
    let catalog = ProductStandaloneCompilerCatalogV1::from_json(trusted_embedded_bytes)?;
    if candidate_bytes.len() > MAX_PRODUCT_STANDALONE_COMPILER_CATALOG_JSON_BYTES_V1 {
        return Err(ProductStandaloneCompilerCatalogError::JsonTooLarge {
            actual: candidate_bytes.len(),
            max: MAX_PRODUCT_STANDALONE_COMPILER_CATALOG_JSON_BYTES_V1,
        });
    }
    if trusted_embedded_bytes != candidate_bytes {
        return Err(
            ProductStandaloneCompilerCatalogError::CatalogAuthenticationMismatch {
                expected_sha256: sha256(trusted_embedded_bytes),
                actual_sha256: sha256(candidate_bytes),
            },
        );
    }
    Ok(catalog)
}

pub fn product_standalone_compiler_catalog_sha256_v1(bytes: &[u8]) -> Sha256Digest {
    sha256(bytes)
}

fn validate_nonzero_seal(
    field: &'static str,
    byte_len: u64,
    digest: Sha256Digest,
    max: u64,
) -> Result<(), ProductStandaloneCompilerCatalogError> {
    if byte_len == 0 || byte_len > max {
        return Err(ProductStandaloneCompilerCatalogError::InvalidFileLength {
            field,
            actual: byte_len,
            max,
        });
    }
    if digest == zero_sha256() {
        return Err(ProductStandaloneCompilerCatalogError::ZeroDigest { field });
    }
    Ok(())
}

fn validate_target(
    target: &ProductStandaloneCompilerTargetV1,
) -> Result<(), ProductStandaloneCompilerCatalogError> {
    if target.target.steam_app_id == 0
        || target.target.steam_build_id == 0
        || target.target.depot_id == 0
        || target.target.depot_manifest_gid == 0
    {
        return Err(ProductStandaloneCompilerCatalogError::IncompleteTarget);
    }
    if target.pe_codeview.age == 0 || !valid_codeview_guid(&target.pe_codeview.guid) {
        return Err(ProductStandaloneCompilerCatalogError::InvalidCodeView);
    }
    Ok(())
}

fn valid_codeview_guid(value: &str) -> bool {
    if value.len() != 36 {
        return false;
    }
    value.bytes().enumerate().all(|(index, byte)| {
        if matches!(index, 8 | 13 | 18 | 23) {
            byte == b'-'
        } else {
            byte.is_ascii_hexdigit()
        }
    })
}

fn validate_relative_package_path(
    field: &'static str,
    path: &str,
) -> Result<(), ProductStandaloneCompilerCatalogError> {
    let components_safe = path.split('/').all(|component| {
        let device_stem = component
            .split_once('.')
            .map_or(component, |(stem, _)| stem)
            .to_ascii_lowercase();
        let reserved_device = matches!(
            device_stem.as_str(),
            "con"
                | "prn"
                | "aux"
                | "nul"
                | "com1"
                | "com2"
                | "com3"
                | "com4"
                | "com5"
                | "com6"
                | "com7"
                | "com8"
                | "com9"
                | "lpt1"
                | "lpt2"
                | "lpt3"
                | "lpt4"
                | "lpt5"
                | "lpt6"
                | "lpt7"
                | "lpt8"
                | "lpt9"
        );
        !component.is_empty()
            && component != "."
            && component != ".."
            && component.len() <= MAX_PRODUCT_STANDALONE_COMPILER_PATH_COMPONENT_BYTES_V1
            && !component.ends_with('.')
            && !component.ends_with(' ')
            && !reserved_device
            && component
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    });
    if path.is_empty()
        || path.len() > MAX_PRODUCT_STANDALONE_COMPILER_RELATIVE_PATH_BYTES_V1
        || !path.is_ascii()
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains(':')
        || path.contains('\0')
        || !components_safe
    {
        return Err(ProductStandaloneCompilerCatalogError::UnsafeRelativePath {
            field,
            path: path.to_owned(),
        });
    }
    Ok(())
}

fn insert_unique_path(
    paths: &mut BTreeSet<String>,
    path: &str,
) -> Result<(), ProductStandaloneCompilerCatalogError> {
    let folded = path.to_ascii_lowercase();
    if paths
        .iter()
        .any(|existing| path_alias_or_ancestor(existing, &folded))
    {
        return Err(
            ProductStandaloneCompilerCatalogError::DuplicateOrAliasedPath {
                path: path.to_owned(),
            },
        );
    }
    paths.insert(folded);
    Ok(())
}

fn path_alias_or_ancestor(left: &str, right: &str) -> bool {
    left == right
        || right
            .strip_prefix(left)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || left
            .strip_prefix(right)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

type TargetKey = (u32, u64, u32, u64, String, u32);

fn target_key(target: &ProductStandaloneCompilerTargetV1) -> TargetKey {
    (
        target.target.steam_app_id,
        target.target.steam_build_id,
        target.target.depot_id,
        target.target.depot_manifest_gid,
        target.pe_codeview.guid.to_ascii_lowercase(),
        target.pe_codeview.age,
    )
}

fn compare_profile_entries(
    left: &ProductStandaloneCompilerProfileV1,
    right: &ProductStandaloneCompilerProfileV1,
) -> Ordering {
    target_key(&left.target)
        .cmp(&target_key(&right.target))
        .then_with(|| {
            left.manifest_relative_path
                .to_ascii_lowercase()
                .cmp(&right.manifest_relative_path.to_ascii_lowercase())
        })
        .then_with(|| {
            left.manifest_relative_path
                .cmp(&right.manifest_relative_path)
        })
}

fn target_equal(
    left: &ProductStandaloneCompilerTargetV1,
    right: &ProductStandaloneCompilerTargetV1,
) -> bool {
    left.target == right.target && codeview_equal(&left.pe_codeview, &right.pe_codeview)
}

fn codeview_equal(left: &PeCodeViewV1, right: &PeCodeViewV1) -> bool {
    left.age == right.age && left.guid.eq_ignore_ascii_case(&right.guid)
}

fn sha256(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::from_bytes(Sha256::digest(bytes).into())
}

fn zero_sha256() -> Sha256Digest {
    Sha256Digest::from_bytes([0; 32])
}

pub(crate) fn validate_standalone_compiler_compatibility_id_v1(
    field: &'static str,
    value: &str,
) -> Result<(), ProductStandaloneCompilerCatalogError> {
    let version = value
        .strip_prefix("gore-as-standalone-semantic-v")
        .filter(|version| {
            !version.is_empty()
                && version.len() <= MAX_STANDALONE_COMPILER_COMPATIBILITY_ID_BYTES_V1
                && version.bytes().all(|byte| byte.is_ascii_digit())
                && !version.starts_with('0')
        });
    if version.is_none() || value.len() > MAX_STANDALONE_COMPILER_COMPATIBILITY_ID_BYTES_V1 {
        return Err(
            ProductStandaloneCompilerCatalogError::InvalidCompatibilityId {
                field,
                value: value.to_owned(),
            },
        );
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum ProductStandaloneCompilerCatalogError {
    #[error("embedded standalone compiler catalog build identity is invalid")]
    EmbeddedBuildIdentityInvalid,
    #[error(
        "embedded standalone compiler catalog differs from its build seal (expected \
         {expected_sha256}, got {actual_sha256})"
    )]
    EmbeddedCatalogBuildSealMismatch {
        expected_sha256: Sha256Digest,
        actual_sha256: Sha256Digest,
    },
    #[error("standalone compiler catalog JSON is {actual} bytes; maximum is {max}")]
    JsonTooLarge { actual: usize, max: usize },
    #[error("invalid standalone compiler catalog JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("standalone compiler catalog schema {0:?} is unsupported")]
    UnsupportedSchema(String),
    #[error("standalone compiler catalog schema version {0} is unsupported")]
    UnsupportedSchemaVersion(u32),
    #[error("standalone compiler catalog must contain at least one qualified profile")]
    ProfilesEmpty,
    #[error("standalone compiler catalog contains {actual} profiles; maximum is {max}")]
    TooManyProfiles { actual: usize, max: usize },
    #[error("{field} has unsafe package-relative path {path:?}")]
    UnsafeRelativePath { field: &'static str, path: String },
    #[error("{field} must end with {required_suffix}")]
    InvalidPathKind {
        field: &'static str,
        required_suffix: &'static str,
    },
    #[error("package path {path:?} duplicates, aliases, or is an ancestor of another file path")]
    DuplicateOrAliasedPath { path: String },
    #[error("{field} length {actual} is outside the nonzero maximum {max}")]
    InvalidFileLength {
        field: &'static str,
        actual: u64,
        max: u64,
    },
    #[error("{field} uses a zero SHA-256 digest")]
    ZeroDigest { field: &'static str },
    #[error(
        "sidecar protocol request={request_version}, response={response_version} is unsupported"
    )]
    UnsupportedProtocol {
        request_version: u32,
        response_version: u32,
    },
    #[error("sidecar package policy must require static_system_only")]
    StaticSystemOnlyRequired,
    #[error("{field} contains invalid standalone compiler compatibility id {value:?}")]
    InvalidCompatibilityId { field: &'static str, value: String },
    #[error(
        "shipped sidecar and qualification reference use different compiler compatibility ids"
    )]
    QualificationCompatibilityMismatch,
    #[error("shipped sidecar and qualification reference use different wire protocols")]
    QualificationProtocolMismatch,
    #[error("compiler target tuple contains a zero Steam/depot identity")]
    IncompleteTarget,
    #[error("compiler target tuple contains an invalid PE CodeView GUID or age")]
    InvalidCodeView,
    #[error("qualified profiles must be in strict canonical target/path order")]
    ProfilesNotSorted,
    #[error("standalone compiler catalog contains a duplicate target tuple")]
    DuplicateTarget,
    #[error("standalone compiler catalog has no profile for the exact target tuple")]
    UnsupportedTarget,
    #[error(
        "sidecar authentication mismatch (expected {expected_byte_len} bytes/{expected_sha256}, \
         got {actual_byte_len} bytes/{actual_sha256})"
    )]
    SidecarAuthenticationMismatch {
        expected_byte_len: u64,
        actual_byte_len: u64,
        expected_sha256: Sha256Digest,
        actual_sha256: Sha256Digest,
    },
    #[error(
        "profile manifest authentication mismatch (expected {expected_byte_len} \
         bytes/{expected_sha256}, got {actual_byte_len} bytes/{actual_sha256})"
    )]
    ProfileManifestAuthenticationMismatch {
        expected_byte_len: u64,
        actual_byte_len: u64,
        expected_sha256: Sha256Digest,
        actual_sha256: Sha256Digest,
    },
    #[error(
        "profile manifest length mismatch (expected {expected_byte_len} bytes, got \
         {actual_byte_len})"
    )]
    ProfileManifestLengthMismatch {
        expected_byte_len: u64,
        actual_byte_len: u64,
    },
    #[error("authenticated profile manifest does not match its catalog target/profile identity")]
    ProfileIdentityMismatch,
    #[error("qualified profile manifest is invalid: {0}")]
    CompilerProfile(#[from] CompilerProfileError),
    #[error(
        "external catalog copy does not match embedded authority (expected {expected_sha256}, \
         got {actual_sha256})"
    )]
    CatalogAuthenticationMismatch {
        expected_sha256: Sha256Digest,
        actual_sha256: Sha256Digest,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler_profile::manifest::{
        BindsProfileV1, BytecodeProfileV1, CacheWriterProfileV1, CompilerArchitectureV1,
        CompilerBuildConfigurationV1, CompilerOracleV1, CompilerPlatformV1, EngineProfileV1,
        FileSealV1, FrontendProfileV1, QualificationProfileV1, SealedBlobV1, Sha1Digest,
        UnrealSemanticsProfileV1, COMPILER_PROFILE_SCHEMA, COMPILER_PROFILE_SCHEMA_VERSION,
    };

    fn digest(byte: u8) -> Sha256Digest {
        Sha256Digest::from_bytes([byte; 32])
    }

    fn target(build_id: u64, guid: &str) -> ProductStandaloneCompilerTargetV1 {
        ProductStandaloneCompilerTargetV1 {
            target: CompilerTargetV1 {
                steam_app_id: 1_297_900,
                steam_build_id: build_id,
                depot_id: 1_297_901,
                depot_manifest_gid: 1_585_071_322_101_748_861,
                platform: CompilerPlatformV1::Windows,
                architecture: CompilerArchitectureV1::X86_64,
                build_configuration: CompilerBuildConfigurationV1::Shipping,
            },
            pe_codeview: PeCodeViewV1 {
                guid: guid.to_owned(),
                age: 1,
            },
        }
    }

    fn profile_entry(
        path: &str,
        manifest_bytes: &[u8],
        profile_sha256: Sha256Digest,
        target: ProductStandaloneCompilerTargetV1,
    ) -> ProductStandaloneCompilerProfileV1 {
        ProductStandaloneCompilerProfileV1 {
            manifest_relative_path: path.to_owned(),
            manifest_byte_len: manifest_bytes.len() as u64,
            manifest_sha256: sha256(manifest_bytes),
            profile_sha256,
            target,
        }
    }

    fn catalog(
        profiles: Vec<ProductStandaloneCompilerProfileV1>,
    ) -> ProductStandaloneCompilerCatalogV1 {
        ProductStandaloneCompilerCatalogV1 {
            schema: PRODUCT_STANDALONE_COMPILER_CATALOG_SCHEMA_V1.to_owned(),
            schema_version: PRODUCT_STANDALONE_COMPILER_CATALOG_SCHEMA_VERSION_V1,
            sidecar: ProductStandaloneCompilerSidecarV1 {
                relative_path: "bin/gore-as-standalone-compiler.exe".to_owned(),
                byte_len: 4,
                sha256: sha256(b"tool"),
                compatibility_id: STANDALONE_COMPILER_COMPATIBILITY_ID_V1.to_owned(),
                protocol: ProductStandaloneCompilerProtocolV1 {
                    request_version: SIDECAR_REQUEST_VERSION_V1,
                    response_version: SIDECAR_RESPONSE_VERSION_V1,
                },
                static_system_only: true,
            },
            qualification_reference: ProductStandaloneCompilerQualificationReferenceV1 {
                byte_len: 4,
                sha256: sha256(b"tool"),
                compatibility_id: STANDALONE_COMPILER_COMPATIBILITY_ID_V1.to_owned(),
                protocol: ProductStandaloneCompilerProtocolV1 {
                    request_version: SIDECAR_REQUEST_VERSION_V1,
                    response_version: SIDECAR_RESPONSE_VERSION_V1,
                },
            },
            profiles,
        }
    }

    fn blob(path: &str, byte: u8) -> SealedBlobV1 {
        SealedBlobV1 {
            path: path.to_owned(),
            byte_len: u64::from(byte) + 1,
            sha256: digest(byte),
        }
    }

    fn file(byte: u8, steam_sha: bool) -> FileSealV1 {
        FileSealV1 {
            byte_len: u64::from(byte) + 1,
            sha256: digest(byte),
            steam_content_sha1: steam_sha.then(|| Sha1Digest::from_bytes([byte; 20])),
        }
    }

    fn qualified_profile(target: &ProductStandaloneCompilerTargetV1) -> CompilerProfileV1 {
        let mut profile = CompilerProfileV1 {
            schema: COMPILER_PROFILE_SCHEMA.to_owned(),
            schema_version: COMPILER_PROFILE_SCHEMA_VERSION,
            target: target.target.clone(),
            oracle: CompilerOracleV1 {
                executable: file(1, true),
                binds_cache: file(2, true),
                shipping_cache: file(3, true),
                depot_manifest: file(4, false),
                pe_codeview: target.pe_codeview.clone(),
            },
            binds: BindsProfileV1 {
                wire_schema_version: 1,
                struct_count: 1,
                class_count: 1,
                method_count: 1,
                struct_property_count: 1,
                class_property_count: 1,
                canonical_database_sha256: digest(5),
            },
            engine: EngineProfileV1 {
                as_create_version: 1,
                ordered_engine_properties: blob("engine/properties.json", 6),
                registration_trace: blob("engine/trace.json", 7),
                registration_trace_count: 1,
                post_bind_snapshot: blob("engine/post-bind.json", 8),
            },
            unreal_semantics: UnrealSemanticsProfileV1 {
                reflected_type_graph: blob("unreal/types.json", 9),
                metadata_schema_version: 1,
            },
            frontend: FrontendProfileV1 {
                preprocessor_config: blob("frontend/preprocessor.json", 10),
                class_generator_config: blob("frontend/classes.json", 11),
                compiler_options: blob("frontend/options.json", 12),
            },
            bytecode: BytecodeProfileV1 {
                opcode_table_version: "v1".to_owned(),
                opcode_table: blob("bytecode/opcodes.json", 13),
                operand_schema: blob("bytecode/operands.json", 14),
                codegen_probe_corpus: blob("qualification/corpus.json", 15),
                expected_probe_results: blob("qualification/results.json", 16),
            },
            cache_writer: CacheWriterProfileV1 {
                format_version: 1,
                serializer_schema: blob("cache/serializer.json", 17),
                build_identifier: 1,
                reference_table_order: blob("cache/references.json", 18),
                normalized_oracle_corpus: blob("cache/oracle.json", 19),
            },
            qualification: QualificationProfileV1 {
                required_probe_suite_version: "v1".to_owned(),
                diagnostic_parity: blob("qualification/diagnostics.json", 20),
                semantic_parity: blob("qualification/semantics.json", 21),
                qualified: true,
            },
            profile_sha256: digest(0),
        };
        profile.seal().unwrap();
        profile
    }

    fn one_profile_catalog_json() -> Vec<u8> {
        let selected = target(24_539_464, "01234567-89ab-cdef-0123-456789abcdef");
        let profile = qualified_profile(&selected);
        let manifest = serde_json::to_vec(&profile).unwrap();
        catalog(vec![profile_entry(
            "profiles/build-24539464/compiler-profile.json",
            &manifest,
            profile.profile_sha256,
            selected,
        )])
        .to_json()
        .unwrap()
    }

    #[test]
    fn production_catalog_is_intentionally_bundle_absent() {
        assert!(EMBEDDED_PRODUCT_STANDALONE_COMPILER_CATALOG_JSON_V1.is_empty());
        assert!(embedded_product_standalone_compiler_catalog_v1()
            .unwrap()
            .is_none());
    }

    #[test]
    fn valid_synthetic_catalog_round_trips_and_authenticates_exact_seals() {
        let json = one_profile_catalog_json();
        let parsed = ProductStandaloneCompilerCatalogV1::from_json(&json).unwrap();
        parsed.authenticate_sidecar(4, sha256(b"tool")).unwrap();

        let mut full_graph: serde_json::Value = serde_json::from_slice(&json).unwrap();
        full_graph["sidecar"]["protocol"]["request_version"] =
            serde_json::json!(SIDECAR_REQUEST_VERSION_V2);
        full_graph["qualification_reference"]["protocol"]["request_version"] =
            serde_json::json!(SIDECAR_REQUEST_VERSION_V2);
        let full_graph = ProductStandaloneCompilerCatalogV1::from_json(
            &serde_json::to_vec(&full_graph).unwrap(),
        )
        .unwrap();
        assert_eq!(
            full_graph.sidecar().protocol().request_version(),
            SIDECAR_REQUEST_VERSION_V2
        );

        let mut incompatible: serde_json::Value = serde_json::from_slice(&json).unwrap();
        incompatible["qualification_reference"]["compatibility_id"] =
            serde_json::json!("gore-as-standalone-semantic-v2");
        assert!(matches!(
            ProductStandaloneCompilerCatalogV1::from_json(
                &serde_json::to_vec(&incompatible).unwrap()
            ),
            Err(ProductStandaloneCompilerCatalogError::QualificationCompatibilityMismatch)
        ));

        let mut protocol_mismatch: serde_json::Value = serde_json::from_slice(&json).unwrap();
        protocol_mismatch["qualification_reference"]["protocol"]["request_version"] =
            serde_json::json!(SIDECAR_REQUEST_VERSION_V2);
        assert!(matches!(
            ProductStandaloneCompilerCatalogV1::from_json(
                &serde_json::to_vec(&protocol_mismatch).unwrap()
            ),
            Err(ProductStandaloneCompilerCatalogError::QualificationProtocolMismatch)
        ));

        let query_target = target(24_539_464, "01234567-89AB-CDEF-0123-456789ABCDEF");
        let manifest_target = target(24_539_464, "01234567-89ab-cdef-0123-456789abcdef");
        let expected_profile = qualified_profile(&manifest_target);
        let manifest = serde_json::to_vec(&expected_profile).unwrap();
        let authenticated = parsed
            .authenticate_profile_manifest(&query_target, &manifest)
            .unwrap();
        assert_eq!(
            authenticated.profile().profile_sha256,
            expected_profile.profile_sha256
        );
    }

    #[test]
    fn parser_rejects_unknown_fields_and_unbounded_input() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&one_profile_catalog_json()).unwrap();
        value["unexpected"] = serde_json::json!(true);
        let error =
            ProductStandaloneCompilerCatalogV1::from_json(&serde_json::to_vec(&value).unwrap())
                .unwrap_err();
        assert!(matches!(
            error,
            ProductStandaloneCompilerCatalogError::Json(_)
        ));

        let oversized = vec![b' '; MAX_PRODUCT_STANDALONE_COMPILER_CATALOG_JSON_BYTES_V1 + 1];
        assert!(matches!(
            ProductStandaloneCompilerCatalogV1::from_json(&oversized),
            Err(ProductStandaloneCompilerCatalogError::JsonTooLarge { .. })
        ));
    }

    #[test]
    fn parser_rejects_unsafe_aliasing_unsorted_and_weak_policy() {
        let manifest = b"profile";
        let first_target = target(1, "01234567-89ab-cdef-0123-456789abcdef");
        let second_target = target(2, "11234567-89ab-cdef-0123-456789abcdef");

        let mut unsafe_path = catalog(vec![profile_entry(
            "../compiler-profile.json",
            manifest,
            digest(1),
            first_target.clone(),
        )]);
        assert!(matches!(
            unsafe_path.validate(),
            Err(ProductStandaloneCompilerCatalogError::UnsafeRelativePath { .. })
        ));
        unsafe_path.profiles[0].manifest_relative_path =
            "BIN/GORE-AS-STANDALONE-COMPILER.EXE".to_owned();
        assert!(matches!(
            unsafe_path.validate(),
            Err(ProductStandaloneCompilerCatalogError::InvalidPathKind { .. })
                | Err(ProductStandaloneCompilerCatalogError::DuplicateOrAliasedPath { .. })
        ));

        let unsorted = catalog(vec![
            profile_entry("profiles/b.json", manifest, digest(2), second_target),
            profile_entry("profiles/a.json", manifest, digest(1), first_target),
        ]);
        assert!(matches!(
            unsorted.validate(),
            Err(ProductStandaloneCompilerCatalogError::ProfilesNotSorted)
        ));

        let mut weak =
            ProductStandaloneCompilerCatalogV1::from_json(&one_profile_catalog_json()).unwrap();
        weak.sidecar.static_system_only = false;
        assert!(matches!(
            weak.validate(),
            Err(ProductStandaloneCompilerCatalogError::StaticSystemOnlyRequired)
        ));
    }

    #[test]
    fn exact_catalog_sidecar_and_profile_authentication_reject_drift() {
        let trusted = one_profile_catalog_json();
        let mut candidate = trusted.clone();
        candidate.push(b' ');
        assert!(matches!(
            authenticate_product_standalone_compiler_catalog_copy_v1(&trusted, &candidate),
            Err(ProductStandaloneCompilerCatalogError::CatalogAuthenticationMismatch { .. })
        ));

        let parsed = ProductStandaloneCompilerCatalogV1::from_json(&trusted).unwrap();
        assert!(matches!(
            parsed.authenticate_sidecar(4, sha256(b"toom")),
            Err(ProductStandaloneCompilerCatalogError::SidecarAuthenticationMismatch { .. })
        ));

        let selected = target(24_539_464, "01234567-89ab-cdef-0123-456789abcdef");
        let mut manifest = serde_json::to_vec(&qualified_profile(&selected)).unwrap();
        manifest[0] ^= 1;
        assert!(matches!(
            parsed.authenticate_profile_manifest(&selected, &manifest),
            Err(
                ProductStandaloneCompilerCatalogError::ProfileManifestAuthenticationMismatch { .. }
            )
        ));

        manifest.push(b' ');
        assert!(matches!(
            parsed.authenticate_profile_manifest(&selected, &manifest),
            Err(ProductStandaloneCompilerCatalogError::ProfileManifestLengthMismatch { .. })
        ));
    }

    #[test]
    fn parser_rejects_duplicate_target_and_casefold_path_aliases() {
        let manifest = b"profile";
        let selected = target(24_539_464, "01234567-89ab-cdef-0123-456789abcdef");
        let duplicate_target = catalog(vec![
            profile_entry("profiles/a.json", manifest, digest(1), selected.clone()),
            profile_entry("profiles/b.json", manifest, digest(2), selected.clone()),
        ]);
        assert!(matches!(
            duplicate_target.validate(),
            Err(ProductStandaloneCompilerCatalogError::DuplicateTarget)
        ));

        let alias = catalog(vec![
            profile_entry("profiles/a.json", manifest, digest(1), selected.clone()),
            profile_entry(
                "PROFILES/A.JSON",
                manifest,
                digest(2),
                target(24_539_465, "11234567-89ab-cdef-0123-456789abcdef"),
            ),
        ]);
        assert!(matches!(
            alias.validate(),
            Err(ProductStandaloneCompilerCatalogError::DuplicateOrAliasedPath { .. })
        ));
    }
}
