//! Cache-bound inspection of native GameplayTag-to-float32 map defaults.
//!
//! The public inspection boundary promotes cache-only reference evidence only after sealed cache,
//! field-schema, target-class, and target-to-owner ancestry proof. The public patch boundary never
//! accepts that report as authority: it rebuilds the proof before and after one copy-on-write CAS.

use std::collections::HashMap;
use std::ops::Range;

use sha2::{Digest, Sha256};
use thiserror::Error;

use super::default_ancestry::DefaultNativeAncestry;
use super::default_class_hierarchy::{
    DefaultClassAncestryProof, DefaultClassHierarchy, DefaultClassIdentity,
};
use super::default_fingerprint::{
    combined_default_cache_fingerprint, DEFAULT_CACHE_FINGERPRINT_FORMAT,
};
use super::default_tag_map::{
    reference_proven_tag_map_sites, ReferenceProvenTagMapSite, TagMapReferenceReport,
};
use super::header::{CacheHeader, HeaderError};
use super::model::parse_modules;

pub const NATIVE_TAG_MAP_SELECTOR_FORMAT: &str = "gore-as-native-tag-map-selector-v1";
pub const NATIVE_TAG_MAP_SELECTOR_KIND: &str = "gameplay-tag-float32-map-entry";
pub const NATIVE_TAG_MAP_VALUE_TYPE: &str = "float32";

/// Strict semantic identity for one generated native GameplayTag-map entry.
///
/// Every field is selector input. Offsets, raw pointers, function names, and current bytes are
/// deliberately excluded. Callers may construct requests, but only a complete proof rebuilt from
/// the supplied cache can authorize a match.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NativeTagMapSelector {
    pub format: String,
    pub kind: String,
    pub module: String,
    pub class: String,
    pub field_owner: String,
    pub field: String,
    pub tag_module: String,
    pub tag_namespace: String,
    pub tag: String,
    pub tag_is_string: bool,
    pub value_type: String,
    pub map_proof_id: String,
    pub ancestry_profile: String,
}

#[derive(Debug, Error)]
pub enum NativeTagMapInspectError {
    #[error(transparent)]
    Header(#[from] HeaderError),
    #[error("failed to compute the combined default-cache fingerprint: {0}")]
    Fingerprint(String),
    #[error("cache does not match the sealed native-ancestry profile")]
    UnsupportedCache,
    #[error("failed to scan exact tag-map references: {0}")]
    ReferenceScan(String),
    #[error("failed to parse script module model: {0}")]
    ModuleModel(String),
    #[error("script class identity/hierarchy proof failed: {0}")]
    ClassHierarchy(String),
}

#[derive(Debug, Error)]
pub enum NativeTagMapPatchError {
    #[error(transparent)]
    Inspect(#[from] NativeTagMapInspectError),
    #[error("native tag-map selector was not found")]
    SelectorNotFound,
    #[error("native tag-map selector matched {matches} proven sites")]
    SelectorAmbiguous { matches: usize },
    #[error("expected operand has {got} bytes; selector requires 4")]
    ExpectedWidth { got: usize },
    #[error("replacement operand has {got} bytes; selector requires 4")]
    ReplacementWidth { got: usize },
    #[error("expected operand drifted: expected {expected}, got {actual}")]
    CasMismatch { expected: String, actual: String },
    #[error("expected and replacement operands are identical")]
    NoChange,
    #[error("native tag-map patch postcondition failed: {0}")]
    Postcondition(String),
}

/// One cache-bound tag-map site whose native declaring field has a sealed schema proof.
/// All fields are private; callers can inspect but cannot fabricate a proof site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeTagMapSite {
    selector: NativeTagMapSelector,
    function: String,
    context_sha256: String,
    operand_range: Range<usize>,
    owner: String,
    owner_module: String,
    owner_namespace: String,
    field: String,
    tag_name: String,
    tag_module: String,
    tag_namespace: String,
    tag_is_string: bool,
    expected: [u8; 4],
    field_schema_proof_id: &'static str,
    ancestry_proof: DefaultClassAncestryProof,
}

impl NativeTagMapSite {
    pub fn selector(&self) -> &NativeTagMapSelector {
        &self.selector
    }

    pub fn target_module(&self) -> &str {
        &self.selector.module
    }

    pub fn target_class(&self) -> &str {
        &self.selector.class
    }

    pub fn function(&self) -> &str {
        &self.function
    }

    /// SHA-256 provenance for the six-instruction raw reference window.
    ///
    /// This is diagnostic output only and is deliberately not part of the semantic selector.
    pub fn context_sha256(&self) -> &str {
        &self.context_sha256
    }

    pub fn operand_range(&self) -> Range<usize> {
        self.operand_range.clone()
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn owner_module(&self) -> &str {
        &self.owner_module
    }

    pub fn owner_namespace(&self) -> &str {
        &self.owner_namespace
    }

    pub fn field(&self) -> &str {
        &self.field
    }

    pub fn tag_name(&self) -> &str {
        &self.tag_name
    }

    pub fn tag_module(&self) -> &str {
        &self.tag_module
    }

    pub fn tag_namespace(&self) -> &str {
        &self.tag_namespace
    }

    pub fn tag_is_string(&self) -> bool {
        self.tag_is_string
    }

    pub fn expected(&self) -> [u8; 4] {
        self.expected
    }

    pub fn field_schema_proof_id(&self) -> &'static str {
        self.field_schema_proof_id
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NativeTagMapStats {
    init_functions: usize,
    branched_init_functions: usize,
    raw_windows: usize,
    reference_proven_windows: usize,
    native_field_proven_windows: usize,
    missing_owner_types: usize,
    missing_properties: usize,
    property_owner_mismatches: usize,
    missing_tag_globals: usize,
    non_gameplay_tag_globals: usize,
    missing_callees: usize,
    non_exact_tmap_add_callees: usize,
    non_native_owner_identities: usize,
    unsealed_native_fields: usize,
    missing_target_classes: usize,
    target_ancestry_mismatches: usize,
    ambiguous_selectors: usize,
}

impl NativeTagMapStats {
    pub fn init_functions(&self) -> usize {
        self.init_functions
    }

    pub fn branched_init_functions(&self) -> usize {
        self.branched_init_functions
    }

    pub fn raw_windows(&self) -> usize {
        self.raw_windows
    }

    pub fn reference_proven_windows(&self) -> usize {
        self.reference_proven_windows
    }

    pub fn native_field_proven_windows(&self) -> usize {
        self.native_field_proven_windows
    }

    pub fn missing_owner_types(&self) -> usize {
        self.missing_owner_types
    }

    pub fn missing_properties(&self) -> usize {
        self.missing_properties
    }

    pub fn property_owner_mismatches(&self) -> usize {
        self.property_owner_mismatches
    }

    pub fn missing_tag_globals(&self) -> usize {
        self.missing_tag_globals
    }

    pub fn non_gameplay_tag_globals(&self) -> usize {
        self.non_gameplay_tag_globals
    }

    pub fn missing_callees(&self) -> usize {
        self.missing_callees
    }

    pub fn non_exact_tmap_add_callees(&self) -> usize {
        self.non_exact_tmap_add_callees
    }

    pub fn non_native_owner_identities(&self) -> usize {
        self.non_native_owner_identities
    }

    pub fn unsealed_native_fields(&self) -> usize {
        self.unsealed_native_fields
    }

    pub fn missing_target_classes(&self) -> usize {
        self.missing_target_classes
    }

    pub fn target_ancestry_mismatches(&self) -> usize {
        self.target_ancestry_mismatches
    }

    pub fn ambiguous_selectors(&self) -> usize {
        self.ambiguous_selectors
    }
}

/// Opaque cache-bound report of exact tag-map sites with sealed native field schemas.
///
/// This is inspection output, not mutation authority. `patch_native_tag_map` rebuilds the proof
/// from its input cache and verifies the complete cache/profile evidence internally; it never
/// trusts a caller-supplied or previously retained report as authorization to write bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeTagMapReport {
    cache_len: usize,
    cache_guid: [u8; 16],
    raw_cache_sha256: [u8; 32],
    fingerprint_format: &'static str,
    fingerprint_sha256: [u8; 32],
    scalar_operand_count: usize,
    tag_operand_count: usize,
    ancestry_profile_id: &'static str,
    map_proof_id: &'static str,
    stats: NativeTagMapStats,
    sites: Vec<NativeTagMapSite>,
}

impl NativeTagMapReport {
    pub fn cache_len(&self) -> usize {
        self.cache_len
    }

    pub fn cache_guid(&self) -> [u8; 16] {
        self.cache_guid
    }

    pub fn raw_cache_sha256(&self) -> [u8; 32] {
        self.raw_cache_sha256
    }

    pub fn fingerprint_sha256(&self) -> [u8; 32] {
        self.fingerprint_sha256
    }

    pub fn fingerprint_format(&self) -> &'static str {
        self.fingerprint_format
    }

    pub fn scalar_operand_count(&self) -> usize {
        self.scalar_operand_count
    }

    pub fn tag_operand_count(&self) -> usize {
        self.tag_operand_count
    }

    pub fn ancestry_profile_id(&self) -> &'static str {
        self.ancestry_profile_id
    }

    pub fn map_proof_id(&self) -> &'static str {
        self.map_proof_id
    }

    pub fn site_count(&self) -> usize {
        self.sites.len()
    }

    pub fn stats(&self) -> &NativeTagMapStats {
        &self.stats
    }

    pub fn sites(&self) -> &[NativeTagMapSite] {
        &self.sites
    }
}

/// Copy-on-write result of one proven four-byte GameplayTag-map patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeTagMapPatch {
    bytes: Vec<u8>,
    before: NativeTagMapSite,
    after: NativeTagMapSite,
}

impl NativeTagMapPatch {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn before(&self) -> &NativeTagMapSite {
        &self.before
    }

    pub fn after(&self) -> &NativeTagMapSite {
        &self.after
    }
}

/// Inspect native GameplayTag-to-float32 map defaults in one exact, sealed cache build.
///
/// Membership is checked before the independently derived reference report is promoted. No API
/// exists to combine a profile with a caller-supplied individual reference site.
pub fn inspect_native_tag_maps(
    cache: &[u8],
    ancestry: &DefaultNativeAncestry,
) -> Result<NativeTagMapReport, NativeTagMapInspectError> {
    let guid = CacheHeader::parse(cache)?.hash;
    let fingerprint = combined_default_cache_fingerprint(cache)
        .map_err(|error| NativeTagMapInspectError::Fingerprint(error.to_string()))?;
    if !ancestry.supports_cache(&guid, &fingerprint) {
        return Err(NativeTagMapInspectError::UnsupportedCache);
    }

    let modules = parse_modules(cache)
        .map_err(|error| NativeTagMapInspectError::ModuleModel(error.to_string()))?;
    let hierarchy = DefaultClassHierarchy::build(&modules, Some(ancestry.clone()))
        .map_err(|error| NativeTagMapInspectError::ClassHierarchy(error.to_string()))?;

    let references = reference_proven_tag_map_sites(cache)
        .map_err(|error| NativeTagMapInspectError::ReferenceScan(error.to_string()))?;
    let (sites, stats) = filter_native_sites(&references, ancestry, &hierarchy);
    Ok(NativeTagMapReport {
        cache_len: cache.len(),
        cache_guid: guid,
        raw_cache_sha256: Sha256::digest(cache).into(),
        fingerprint_format: DEFAULT_CACHE_FINGERPRINT_FORMAT,
        fingerprint_sha256: fingerprint.sha256,
        scalar_operand_count: fingerprint.scalar_operand_count,
        tag_operand_count: fingerprint.tag_operand_count,
        ancestry_profile_id: ancestry.profile_id(),
        map_proof_id: ancestry.gameplay_tag_float32_map_proof_id(),
        stats,
        sites,
    })
}

/// Apply one semantic native GameplayTag-map float32 patch to a cloned cache buffer.
///
/// The supplied report/site is never accepted as authority: this function rebuilds the complete
/// opaque inspection proof from `cache`, resolves exactly one semantic selector, performs two
/// four-byte CAS checks, and rebuilds the proof again from the output bytes.
pub fn patch_native_tag_map(
    cache: &[u8],
    ancestry: &DefaultNativeAncestry,
    selector: &NativeTagMapSelector,
    expected: &[u8],
    replacement: &[u8],
) -> Result<NativeTagMapPatch, NativeTagMapPatchError> {
    if expected.len() != 4 {
        return Err(NativeTagMapPatchError::ExpectedWidth {
            got: expected.len(),
        });
    }
    if replacement.len() != 4 {
        return Err(NativeTagMapPatchError::ReplacementWidth {
            got: replacement.len(),
        });
    }

    let before_report = inspect_native_tag_maps(cache, ancestry)?;
    let before = unique_selector_site(&before_report.sites, selector)?.clone();
    let range = validate_operand_request(cache, &before, expected, replacement)?;

    let mut output = cache.to_vec();
    output[range.clone()].copy_from_slice(replacement);
    verify_only_range_changed(cache, &output, range.clone())?;

    let after_report = inspect_native_tag_maps(&output, ancestry)?;
    verify_fingerprint_invariant(&before_report, &after_report)?;
    let after = unique_selector_site(&after_report.sites, selector)?.clone();
    if after.operand_range != range || after.expected.as_slice() != replacement {
        return Err(NativeTagMapPatchError::Postcondition(
            "selector did not rediscover the replacement at the original operand".into(),
        ));
    }

    Ok(NativeTagMapPatch {
        bytes: output,
        before,
        after,
    })
}

fn validate_operand_request(
    cache: &[u8],
    before: &NativeTagMapSite,
    expected: &[u8],
    replacement: &[u8],
) -> Result<Range<usize>, NativeTagMapPatchError> {
    if before.expected.as_slice() != expected {
        return Err(NativeTagMapPatchError::CasMismatch {
            expected: encode_hex(expected),
            actual: encode_hex(&before.expected),
        });
    }
    let range = before.operand_range.clone();
    let actual = cache.get(range.clone()).ok_or_else(|| {
        NativeTagMapPatchError::Postcondition("proven input operand range is invalid".into())
    })?;
    if actual != expected {
        return Err(NativeTagMapPatchError::CasMismatch {
            expected: encode_hex(expected),
            actual: encode_hex(actual),
        });
    }
    if expected == replacement {
        return Err(NativeTagMapPatchError::NoChange);
    }
    Ok(range)
}

fn unique_selector_site<'a>(
    sites: &'a [NativeTagMapSite],
    selector: &NativeTagMapSelector,
) -> Result<&'a NativeTagMapSite, NativeTagMapPatchError> {
    let mut matches = sites.iter().filter(|site| &site.selector == selector);
    let Some(site) = matches.next() else {
        return Err(NativeTagMapPatchError::SelectorNotFound);
    };
    if matches.next().is_some() {
        let count = sites
            .iter()
            .filter(|candidate| &candidate.selector == selector)
            .count();
        return Err(NativeTagMapPatchError::SelectorAmbiguous { matches: count });
    }
    Ok(site)
}

fn verify_only_range_changed(
    before: &[u8],
    after: &[u8],
    changed: Range<usize>,
) -> Result<(), NativeTagMapPatchError> {
    if before.len() != after.len() {
        return Err(NativeTagMapPatchError::Postcondition(format!(
            "cache length changed from {} to {}",
            before.len(),
            after.len()
        )));
    }
    if changed.end > before.len() || changed.start > changed.end {
        return Err(NativeTagMapPatchError::Postcondition(
            "changed operand range is outside the cache".into(),
        ));
    }
    if before[..changed.start] != after[..changed.start]
        || before[changed.end..] != after[changed.end..]
    {
        return Err(NativeTagMapPatchError::Postcondition(
            "bytes outside the proven operand range changed".into(),
        ));
    }
    Ok(())
}

fn verify_fingerprint_invariant(
    before: &NativeTagMapReport,
    after: &NativeTagMapReport,
) -> Result<(), NativeTagMapPatchError> {
    if before.cache_len != after.cache_len
        || before.cache_guid != after.cache_guid
        || before.fingerprint_format != after.fingerprint_format
        || before.fingerprint_sha256 != after.fingerprint_sha256
        || before.scalar_operand_count != after.scalar_operand_count
        || before.tag_operand_count != after.tag_operand_count
        || before.ancestry_profile_id != after.ancestry_profile_id
        || before.map_proof_id != after.map_proof_id
    {
        return Err(NativeTagMapPatchError::Postcondition(
            "combined cache/profile evidence changed after operand replacement".into(),
        ));
    }
    Ok(())
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn filter_native_sites(
    references: &TagMapReferenceReport,
    ancestry: &DefaultNativeAncestry,
    hierarchy: &DefaultClassHierarchy,
) -> (Vec<NativeTagMapSite>, NativeTagMapStats) {
    let source = &references.stats;
    let mut stats = NativeTagMapStats {
        init_functions: source.init_functions,
        branched_init_functions: source.branched_init_functions,
        raw_windows: source.raw_windows,
        reference_proven_windows: source.reference_proven_windows,
        missing_owner_types: source.missing_owner_types,
        missing_properties: source.missing_properties,
        property_owner_mismatches: source.property_owner_mismatches,
        missing_tag_globals: source.missing_tag_globals,
        non_gameplay_tag_globals: source.non_gameplay_tag_globals,
        missing_callees: source.missing_callees,
        non_exact_tmap_add_callees: source.non_exact_tmap_add_callees,
        ..NativeTagMapStats::default()
    };
    let mut sites = Vec::new();

    for site in &references.sites {
        if !site.field_owner.module.is_empty() || !site.field_owner.namespace.is_empty() {
            stats.non_native_owner_identities += 1;
            continue;
        }
        let Some(field_schema_proof_id) =
            ancestry.proves_gameplay_tag_float32_map(&site.field_owner.name, &site.field)
        else {
            stats.unsealed_native_fields += 1;
            continue;
        };
        let Some(target) = hierarchy.initializer_identity(&site.function) else {
            stats.missing_target_classes += 1;
            continue;
        };
        let Some(ancestry_proof) = hierarchy.proves_ancestry(&target.class, &site.field_owner.name)
        else {
            stats.target_ancestry_mismatches += 1;
            continue;
        };
        sites.push(promote_site(
            site,
            target,
            ancestry_proof,
            ancestry,
            field_schema_proof_id,
        ));
    }
    stats.native_field_proven_windows = sites.len();
    let mut selector_counts = HashMap::new();
    for site in &sites {
        *selector_counts.entry(&site.selector).or_insert(0usize) += 1;
    }
    stats.ambiguous_selectors = selector_counts
        .values()
        .filter(|count| **count != 1)
        .count();
    (sites, stats)
}

fn promote_site(
    site: &ReferenceProvenTagMapSite,
    target: &DefaultClassIdentity,
    ancestry_proof: DefaultClassAncestryProof,
    ancestry: &DefaultNativeAncestry,
    field_schema_proof_id: &'static str,
) -> NativeTagMapSite {
    NativeTagMapSite {
        selector: NativeTagMapSelector {
            format: NATIVE_TAG_MAP_SELECTOR_FORMAT.into(),
            kind: NATIVE_TAG_MAP_SELECTOR_KIND.into(),
            module: target.module.clone(),
            class: target.class.clone(),
            field_owner: site.field_owner.name.clone(),
            field: site.field.clone(),
            tag_module: site.tag.module.clone(),
            tag_namespace: site.tag.namespace.clone(),
            tag: site.tag.name.clone(),
            tag_is_string: site.tag.is_string,
            value_type: NATIVE_TAG_MAP_VALUE_TYPE.into(),
            map_proof_id: field_schema_proof_id.into(),
            ancestry_profile: ancestry.profile_id().into(),
        },
        function: site.function.clone(),
        context_sha256: site.raw.context_sha256.clone(),
        operand_range: site.operand_range.clone(),
        owner: site.field_owner.name.clone(),
        owner_module: site.field_owner.module.clone(),
        owner_namespace: site.field_owner.namespace.clone(),
        field: site.field.clone(),
        tag_name: site.tag.name.clone(),
        tag_module: site.tag.module.clone(),
        tag_namespace: site.tag.namespace.clone(),
        tag_is_string: site.tag.is_string,
        expected: site.raw.expected,
        field_schema_proof_id,
        ancestry_proof,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::default_tag_map::{
        ExactGlobalReference, ExactTypeIdentity, RawTagMapWindow, TagMapReferenceStats,
    };
    use crate::cache::model::{Class, Func, Module};

    fn module_with_initializer(class: &str, super_class: &str) -> Module {
        Module {
            name: "Items".into(),
            file: "Items.as".into(),
            functions: Vec::new(),
            classes: vec![Class {
                name: class.into(),
                namespace: String::new(),
                super_class: Some(super_class.into()),
                fields: Vec::new(),
                methods: vec![Func {
                    name: "__InitDefaults".into(),
                    namespace: String::new(),
                    param_defaults: Vec::new(),
                    ret: crate::cache::types::DataType {
                        token: 0x52,
                        ..Default::default()
                    },
                    params: Vec::new(),
                    bytecode: Vec::new(),
                    variable_space: 0,
                    obj_locals: Vec::new(),
                    is_ufunction: false,
                    traits: 0x20,
                }],
                ctors: Vec::new(),
                flags: 0,
            }],
            enums: Vec::new(),
            globals: Vec::new(),
        }
    }

    fn reference_site(owner: &str, module: &str, field: &str) -> ReferenceProvenTagMapSite {
        ReferenceProvenTagMapSite {
            function: "Items.UFixture::__InitDefaults".into(),
            raw: RawTagMapWindow {
                instruction_index: 0,
                instruction_offset_dw: 0,
                operand_offset_dw: 1,
                value_slot: 7,
                expected: 10.0f32.to_le_bytes(),
                owner_type_id: 1,
                member_offset: 16,
                tag_global_ptr: 2,
                callee_func_ptr: 3,
                context_sha256: "fixture".into(),
            },
            operand_range: 4..8,
            field_owner: ExactTypeIdentity {
                name: owner.into(),
                module: module.into(),
                namespace: String::new(),
            },
            field: field.into(),
            tag: ExactGlobalReference {
                name: "Item_Damage_Physical_Edge".into(),
                module: String::new(),
                namespace: "GameplayTag".into(),
                is_string: false,
            },
        }
    }

    #[test]
    fn filter_requires_exact_profile_field_and_bare_native_owner_identity() {
        let ancestry = DefaultNativeAncestry::from_test_edges_and_maps(
            &[("UWeaponDefinition", None), ("UOtherOwner", None)],
            &[
                ("UWeaponDefinition", "m_DamageBase"),
                ("UOtherOwner", "m_DamageBase"),
            ],
        );
        let hierarchy = DefaultClassHierarchy::build(
            &[module_with_initializer("UFixture", "UWeaponDefinition")],
            Some(ancestry.clone()),
        )
        .unwrap();
        let mut missing_target = reference_site("UWeaponDefinition", "", "m_DamageBase");
        missing_target.function = "Items.UMissing::__InitDefaults".into();
        let references = TagMapReferenceReport {
            sites: vec![
                reference_site("UWeaponDefinition", "", "m_DamageBase"),
                reference_site("UWeaponDefinition", "Foreign", "m_DamageBase"),
                reference_site("UWeaponDefinition", "", "m_damageBase"),
                missing_target,
                reference_site("UOtherOwner", "", "m_DamageBase"),
            ],
            stats: TagMapReferenceStats {
                reference_proven_windows: 5,
                ..TagMapReferenceStats::default()
            },
        };

        let (sites, stats) = filter_native_sites(&references, &ancestry, &hierarchy);
        assert_eq!(sites.len(), 1);
        assert_eq!(stats.native_field_proven_windows(), 1);
        assert_eq!(stats.non_native_owner_identities(), 1);
        assert_eq!(stats.unsealed_native_fields(), 1);
        assert_eq!(stats.missing_target_classes(), 1);
        assert_eq!(stats.target_ancestry_mismatches(), 1);
        assert_eq!(stats.ambiguous_selectors(), 0);
        assert_eq!(sites[0].target_module(), "Items");
        assert_eq!(sites[0].target_class(), "UFixture");
        assert_eq!(sites[0].context_sha256(), "fixture");
        assert_eq!(sites[0].owner(), "UWeaponDefinition");
        assert_eq!(sites[0].owner_module(), "");
        assert_eq!(sites[0].field(), "m_DamageBase");
        assert_eq!(sites[0].tag_namespace(), "GameplayTag");
        assert_eq!(sites[0].expected(), 10.0f32.to_le_bytes());
        assert_eq!(sites[0].selector().format, NATIVE_TAG_MAP_SELECTOR_FORMAT);
        assert_eq!(sites[0].selector().kind, NATIVE_TAG_MAP_SELECTOR_KIND);
        assert_eq!(sites[0].selector().value_type, NATIVE_TAG_MAP_VALUE_TYPE);
        assert_eq!(
            sites[0].selector().map_proof_id,
            ancestry.gameplay_tag_float32_map_proof_id()
        );

        let selector = sites[0].selector().clone();
        assert!(matches!(
            unique_selector_site(&[sites[0].clone(), sites[0].clone()], &selector),
            Err(NativeTagMapPatchError::SelectorAmbiguous { matches: 2 })
        ));
        let mut wrong_target = selector.clone();
        wrong_target.class = "UOtherTarget".into();
        assert!(matches!(
            unique_selector_site(&sites, &wrong_target),
            Err(NativeTagMapPatchError::SelectorNotFound)
        ));
        let mut forged_proof = selector.clone();
        forged_proof.map_proof_id = "sha256:forged".into();
        assert!(matches!(
            unique_selector_site(&sites, &forged_proof),
            Err(NativeTagMapPatchError::SelectorNotFound)
        ));

        let mut cache = vec![0u8; 12];
        cache[4..8].copy_from_slice(&10.0f32.to_le_bytes());
        let wrong = 9.0f32.to_le_bytes();
        assert!(matches!(
            validate_operand_request(&cache, &sites[0], &wrong, &wrong),
            Err(NativeTagMapPatchError::CasMismatch { .. })
        ));
        assert!(matches!(
            validate_operand_request(
                &cache,
                &sites[0],
                &10.0f32.to_le_bytes(),
                &10.0f32.to_le_bytes()
            ),
            Err(NativeTagMapPatchError::NoChange)
        ));
        assert_eq!(
            validate_operand_request(
                &cache,
                &sites[0],
                &10.0f32.to_le_bytes(),
                &11.0f32.to_le_bytes()
            )
            .unwrap(),
            4..8
        );
        let mut outside_changed = cache.clone();
        outside_changed[0] ^= 1;
        assert!(matches!(
            verify_only_range_changed(&cache, &outside_changed, 4..8),
            Err(NativeTagMapPatchError::Postcondition(_))
        ));
        assert!(matches!(
            patch_native_tag_map(&[], &ancestry, &selector, &[0; 3], &[0; 4]),
            Err(NativeTagMapPatchError::ExpectedWidth { got: 3 })
        ));
        assert!(matches!(
            patch_native_tag_map(&[], &ancestry, &selector, &[0; 4], &[0; 3]),
            Err(NativeTagMapPatchError::ReplacementWidth { got: 3 })
        ));
    }

    #[test]
    fn configured_same_profile_rejects_a_different_cache() {
        let Some(path) = std::env::var_os("GORE_AS_DEFAULT_CACHE") else {
            eprintln!("skip: set GORE_AS_DEFAULT_CACHE");
            return;
        };
        let Some(usmap_path) = std::env::var_os("GORE_AS_DEFAULT_USMAP") else {
            eprintln!("skip: set GORE_AS_DEFAULT_USMAP");
            return;
        };
        let cache_path = std::path::PathBuf::from(path);
        let cache = std::fs::read(&cache_path).expect("read configured Shipping cache");
        let binds = crate::cache::binds::NativeApi::load(
            &cache_path
                .parent()
                .expect("Script directory")
                .join("Binds.Cache"),
        )
        .expect("load sealed sibling Binds.Cache");
        let usmap = std::fs::read(usmap_path).expect("read configured USMAP");
        let schemas = gore_asset::SchemaDb::from_usmap(&usmap).expect("parse configured USMAP");
        let ancestry = DefaultNativeAncestry::from_schema_db(&binds, &cache, &schemas)
            .expect("derive configured sealed ancestry profile");
        let guid = CacheHeader::parse(&cache).expect("production header").hash;
        let fingerprint = combined_default_cache_fingerprint(&cache)
            .expect("production combined default fingerprint");
        assert!(ancestry.supports_cache(&guid, &fingerprint));

        let references = reference_proven_tag_map_sites(&cache).expect("production tag-map sites");
        let context_offset = references.sites[0]
            .operand_range
            .start
            .checked_sub(1)
            .expect("SetV4 word precedes its immediate");
        assert!(references
            .sites
            .iter()
            .all(|site| !site.operand_range.contains(&context_offset)));
        let mut wrong_cache = cache;
        wrong_cache[context_offset] ^= 0x80;
        assert_eq!(
            CacheHeader::parse(&wrong_cache)
                .expect("context change keeps cache header parseable")
                .hash,
            guid
        );
        let wrong_fingerprint = combined_default_cache_fingerprint(&wrong_cache)
            .expect("context change keeps cache structurally fingerprintable");
        assert!(
            wrong_fingerprint.sha256 != fingerprint.sha256
                || wrong_fingerprint.tag_operand_count != fingerprint.tag_operand_count
        );
        assert!(matches!(
            inspect_native_tag_maps(&wrong_cache, &ancestry),
            Err(NativeTagMapInspectError::UnsupportedCache)
        ));
    }

    #[test]
    fn configured_sword_damage_entry_patches_10_to_11_copy_on_write() {
        let Some(cache_path) = std::env::var_os("GORE_AS_DEFAULT_CACHE") else {
            eprintln!("skip: set GORE_AS_DEFAULT_CACHE");
            return;
        };
        let Some(usmap_path) = std::env::var_os("GORE_AS_DEFAULT_USMAP") else {
            eprintln!("skip: set GORE_AS_DEFAULT_USMAP");
            return;
        };
        let cache_path = std::path::PathBuf::from(cache_path);
        let cache = std::fs::read(&cache_path).expect("read configured Shipping cache");
        let pristine = cache.clone();
        let binds = crate::cache::binds::NativeApi::load(
            &cache_path
                .parent()
                .expect("Script directory")
                .join("Binds.Cache"),
        )
        .expect("load sibling Binds.Cache");
        let usmap = std::fs::read(usmap_path).expect("read configured USMAP");
        let schemas = gore_asset::SchemaDb::from_usmap(&usmap).expect("parse configured USMAP");
        let ancestry = DefaultNativeAncestry::from_schema_db(&binds, &cache, &schemas)
            .expect("build sealed production ancestry");

        let before_report =
            inspect_native_tag_maps(&cache, &ancestry).expect("inspect production tag maps");
        let sword: Vec<_> = before_report
            .sites()
            .iter()
            .filter(|site| {
                site.selector.class == "UItMw_1H_Sword_Old_01"
                    && site.selector.field_owner == "UWeaponDefinition"
                    && site.selector.field == "m_DamageBase"
                    && site.selector.tag == "Item_Damage_Physical_Edge"
                    && site.expected == 10.0f32.to_le_bytes()
            })
            .collect();
        assert_eq!(sword.len(), 1);
        assert!(matches!(
            sword[0].ancestry_proof,
            DefaultClassAncestryProof::Native(profile)
                if profile == ancestry.profile_id()
        ));
        let selector = sword[0].selector().clone();
        let range = sword[0].operand_range();
        let expected = 10.0f32.to_le_bytes();
        let replacement = 11.0f32.to_le_bytes();

        let patched = patch_native_tag_map(&cache, &ancestry, &selector, &expected, &replacement)
            .expect("patch Sword m_DamageBase tag entry");
        assert_eq!(cache, pristine, "input cache must remain untouched");
        assert_eq!(patched.bytes().len(), cache.len());
        assert_eq!(&patched.bytes()[range.clone()], replacement.as_slice());
        assert_eq!(&patched.bytes()[..range.start], &cache[..range.start]);
        assert_eq!(&patched.bytes()[range.end..], &cache[range.end..]);
        assert_eq!(patched.before().selector(), &selector);
        assert_eq!(patched.before().expected(), expected);
        assert_eq!(patched.after().selector(), &selector);
        assert_eq!(patched.after().expected(), replacement);
        assert_eq!(patched.before().operand_range(), range);
        assert_eq!(patched.after().operand_range(), range);
        let changed: Vec<_> = cache
            .iter()
            .zip(patched.bytes())
            .enumerate()
            .filter_map(|(offset, (before, after))| (before != after).then_some(offset))
            .collect();
        assert!(!changed.is_empty());
        assert!(changed.iter().all(|offset| range.contains(offset)));

        let reconstructed =
            DefaultNativeAncestry::from_schema_db(&binds, patched.bytes(), &schemas)
                .expect("reconstruct sealed ancestry from patched cache");
        let after_report = inspect_native_tag_maps(patched.bytes(), &reconstructed)
            .expect("rebuild opaque report from patched bytes and reconstructed ancestry");
        assert_eq!(
            after_report.fingerprint_sha256(),
            before_report.fingerprint_sha256()
        );
        assert_eq!(
            after_report.scalar_operand_count(),
            before_report.scalar_operand_count()
        );
        assert_eq!(
            after_report.tag_operand_count(),
            before_report.tag_operand_count()
        );
        assert_ne!(
            after_report.raw_cache_sha256(),
            before_report.raw_cache_sha256()
        );
        let rediscovered: Vec<_> = after_report
            .sites()
            .iter()
            .filter(|site| site.selector() == &selector)
            .collect();
        assert_eq!(rediscovered.len(), 1);
        assert_eq!(rediscovered[0].operand_range(), range);
        assert_eq!(rediscovered[0].expected(), replacement);

        assert!(matches!(
            patch_native_tag_map(
                patched.bytes(),
                &reconstructed,
                &selector,
                &expected,
                &replacement
            ),
            Err(NativeTagMapPatchError::CasMismatch { .. })
        ));
    }
}
