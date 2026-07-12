//! Cache-bound inspection of native GameplayTag-to-float32 map defaults.
//!
//! This is the only public promotion boundary for the cache-only reference scanner. It first
//! proves that the supplied bytes belong to the same sealed profile, then derives every site
//! again from those bytes and admits only exact, sealed native field declarations.

use std::ops::Range;

use sha2::{Digest, Sha256};
use thiserror::Error;

use super::default_ancestry::DefaultNativeAncestry;
use super::default_fingerprint::{
    combined_default_cache_fingerprint, DEFAULT_CACHE_FINGERPRINT_FORMAT,
};
use super::default_tag_map::{
    reference_proven_tag_map_sites, ReferenceProvenTagMapSite, TagMapReferenceReport,
};
use super::header::{CacheHeader, HeaderError};

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
}

/// One cache-bound tag-map site whose native declaring field has a sealed schema proof.
/// All fields are private; callers can inspect but cannot fabricate a proof site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeTagMapSite {
    function: String,
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
}

impl NativeTagMapSite {
    pub fn function(&self) -> &str {
        &self.function
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
}

/// Opaque cache-bound report of exact tag-map sites with sealed native field schemas.
///
/// This is inspection output, not mutation authority. A future patch path must rebuild the proof
/// from its input cache and internally verify the complete raw-cache SHA-256; it must never trust
/// a caller-supplied or previously retained report as authorization to write bytes.
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

    let references = reference_proven_tag_map_sites(cache)
        .map_err(|error| NativeTagMapInspectError::ReferenceScan(error.to_string()))?;
    let (sites, stats) = filter_native_sites(&references, ancestry);
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

fn filter_native_sites(
    references: &TagMapReferenceReport,
    ancestry: &DefaultNativeAncestry,
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
        sites.push(promote_site(site, field_schema_proof_id));
    }
    stats.native_field_proven_windows = sites.len();
    (sites, stats)
}

fn promote_site(
    site: &ReferenceProvenTagMapSite,
    field_schema_proof_id: &'static str,
) -> NativeTagMapSite {
    NativeTagMapSite {
        function: site.function.clone(),
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::default_tag_map::{
        ExactGlobalReference, ExactTypeIdentity, RawTagMapWindow, TagMapReferenceStats,
    };

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
            &[("UWeaponDefinition", None)],
            &[("UWeaponDefinition", "m_DamageBase")],
        );
        let references = TagMapReferenceReport {
            sites: vec![
                reference_site("UWeaponDefinition", "", "m_DamageBase"),
                reference_site("UWeaponDefinition", "Foreign", "m_DamageBase"),
                reference_site("UWeaponDefinition", "", "m_damageBase"),
            ],
            stats: TagMapReferenceStats {
                reference_proven_windows: 3,
                ..TagMapReferenceStats::default()
            },
        };

        let (sites, stats) = filter_native_sites(&references, &ancestry);
        assert_eq!(sites.len(), 1);
        assert_eq!(stats.native_field_proven_windows(), 1);
        assert_eq!(stats.non_native_owner_identities(), 1);
        assert_eq!(stats.unsealed_native_fields(), 1);
        assert_eq!(sites[0].owner(), "UWeaponDefinition");
        assert_eq!(sites[0].owner_module(), "");
        assert_eq!(sites[0].field(), "m_DamageBase");
        assert_eq!(sites[0].tag_namespace(), "GameplayTag");
        assert_eq!(sites[0].expected(), 10.0f32.to_le_bytes());
    }

    #[test]
    fn configured_same_profile_rejects_a_different_cache() {
        let Some(path) = std::env::var_os("GORE_AS_DEFAULT_CACHE") else {
            eprintln!("skip: set GORE_AS_DEFAULT_CACHE");
            return;
        };
        let cache = std::fs::read(path).expect("read configured Shipping cache");
        let ancestry = DefaultNativeAncestry::from_test_edges_and_maps(&[], &[]);
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
}
