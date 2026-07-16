//! List texture assets in a cooked UE5 IoStore container.
//!
//! The container holds *cooked* zen packages; an asset's class (e.g. `Texture2D`)
//! is not stored in chunk metadata but inside each package's zen header. For every
//! package we parse the zen header, walk its export map, and compare each export's
//! `class_index` (a `FPackageObjectIndex`) against a single precomputed
//! script-import index for `Texture2D` -- an exact integer/hash match, with no name
//! lookup or table resolution involved. A package is reported as a texture if any
//! of its exports' `class_index` equals that precomputed Texture2D index
//! (`LightMapTexture2D`/`ShadowMapTexture2D` have their own distinct classes, so
//! this is an exact match on `Texture2D` only).
//!
//! This is the "per-package class resolution" route from the task note. The
//! global script-object *table* lives in the engine's `global.utoc`, not in the
//! game's `G1R-Windows.utoc`, so `load_script_objects()` is unavailable here.
//! Instead we reproduce the exact import-hash UE assigns to a script object:
//! `FPackageObjectIndex::create_script_import("/Script/Engine.Texture2D")`
//! (cityhash64 of the lower-cased, slash-normalised path) and compare each
//! export's `class_index` against it. This is an exact, table-free match.

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Context as _;
use retoc::asset_conversion::{build_legacy, FZenPackageContext};
use retoc::container_header::{EIoContainerHeaderVersion, StoreEntry};
use retoc::iostore;
use retoc::iostore::IoStoreTrait as _;
use retoc::logging::Log;
use retoc::script_objects::FPackageObjectIndex;
use retoc::zen::FZenPackageHeader;
use retoc::{
    Config, EIoChunkType, EIoStoreTocVersion, FIoChunkId, FIoChunkIdRaw, FIoContainerId,
    FPackageId, FSFileWriter, FileWriterTrait, UEPath, UEPathBuf,
};

use crate::error::{Result, TexError};

/// A texture asset discovered in a container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureEntry {
    /// Cooked package path, e.g. `/Game/Characters/Hero/T_Hero_BaseColor`.
    pub asset_path: String,
}

/// Exact verified IoStore chunk consumed while converting one package.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct VerifiedChunkReceipt {
    pub chunk_id: String,
    pub chunk_type: String,
    pub source_utoc: PathBuf,
    /// BLAKE3 of the exact bounded UTOC bytes parsed by Retoc for this winner.
    #[serde(skip_serializing)]
    pub source_utoc_blake3: String,
    pub length: u64,
    pub blake3: String,
    pub toc_hash: String,
    pub toc_hash_bytes: usize,
}

/// Result of snapshot-backed Zen-to-legacy conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedUnpackedAsset {
    pub uasset: PathBuf,
    pub consumed_chunks: Vec<VerifiedChunkReceipt>,
    pub metadata_utocs: Vec<PathBuf>,
    pub opened_utocs: Vec<VerifiedOpenedUtocReceipt>,
}

/// One known optional component emitted while converting a Zen package to the
/// legacy cooked representation used by the bounded DataAsset inspector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum VerifiedLegacySidecarKind {
    Bulk,
    Optional,
    MemoryMapped,
}

/// Path-free bytes for one known optional legacy cooked-package component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedLegacySidecarBytes {
    pub(crate) kind: VerifiedLegacySidecarKind,
    pub(crate) bytes: Vec<u8>,
}

/// Path plus content identity of the exact UTOC bytes parsed for one opened child container.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VerifiedOpenedUtocReceipt {
    pub source_utoc: PathBuf,
    pub source_utoc_blake3: String,
}

/// Snapshot-backed Zen-to-legacy conversion held entirely in memory.
///
/// The source container paths in the native receipts are retained for
/// subsequent native generation checks, but no output path exists and no
/// filesystem write is performed. Callers must keep and revalidate the
/// installation guard that supplied the already-open composite store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedUnpackedAssetBytes {
    pub(crate) uasset: Vec<u8>,
    pub(crate) uexp: Vec<u8>,
    pub(crate) sidecars: Vec<VerifiedLegacySidecarBytes>,
    pub(crate) consumed_chunks: Vec<VerifiedChunkReceipt>,
    pub(crate) metadata_utocs: Vec<VerifiedOpenedUtocReceipt>,
}

/// Which read-only source supplied bytes during a verified primary-IoStore-pair
/// readback.
///
/// The built IoStore pair is always [`Primary`](Self::Primary). The installed game
/// can only be [`Fallback`](Self::Fallback) for script objects and imported
/// package dependencies; target-package chunks are never eligible for fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifiedReadbackSourceRoleV1 {
    Primary,
    Fallback,
}

/// Path-free identity of one UTOC opened for a verified readback.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct VerifiedReadbackSourceSealV1 {
    role: VerifiedReadbackSourceRoleV1,
    utoc_blake3: [u8; 32],
}

impl VerifiedReadbackSourceSealV1 {
    pub fn role(&self) -> VerifiedReadbackSourceRoleV1 {
        self.role
    }

    pub fn utoc_blake3(&self) -> &[u8; 32] {
        &self.utoc_blake3
    }
}

/// Path-free identity of one exact IoStore chunk verified during readback.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct VerifiedReadbackChunkSealV1 {
    source_role: VerifiedReadbackSourceRoleV1,
    source_utoc_blake3: [u8; 32],
    chunk_id: [u8; 12],
    chunk_type: String,
    length: u64,
    blake3: [u8; 32],
    toc_hash: [u8; 32],
    toc_hash_bytes: usize,
}

impl VerifiedReadbackChunkSealV1 {
    pub fn source_role(&self) -> VerifiedReadbackSourceRoleV1 {
        self.source_role
    }

    pub fn source_utoc_blake3(&self) -> &[u8; 32] {
        &self.source_utoc_blake3
    }

    pub fn chunk_id(&self) -> &[u8; 12] {
        &self.chunk_id
    }

    pub fn chunk_type(&self) -> &str {
        &self.chunk_type
    }

    pub fn length(&self) -> u64 {
        self.length
    }

    pub fn blake3(&self) -> &[u8; 32] {
        &self.blake3
    }

    pub fn toc_hash(&self) -> &[u8] {
        &self.toc_hash[..self.toc_hash_bytes]
    }
}

/// One optional component in an in-memory legacy cooked package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifiedReadbackSidecarKindV1 {
    Bulk,
    Optional,
    MemoryMapped,
}

/// Owned, bounded bytes for one optional legacy cooked-package component.
pub struct VerifiedReadbackSidecarV1 {
    kind: VerifiedReadbackSidecarKindV1,
    bytes: Vec<u8>,
}

impl VerifiedReadbackSidecarV1 {
    pub fn kind(&self) -> VerifiedReadbackSidecarKindV1 {
        self.kind
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Write-free, owned legacy bytes reconstructed from one freshly built
/// single-package IoStore pair plus read-only installed-game dependencies.
///
/// Source paths are deliberately absent. The source and chunk getters expose
/// only content seals and the closed primary/fallback role. All byte vectors are
/// bounded by the same limits as the installed-package memory conversion path.
pub struct VerifiedPrimaryAssetReadbackV1 {
    asset_path: String,
    uasset: Vec<u8>,
    uexp: Vec<u8>,
    sidecars: Vec<VerifiedReadbackSidecarV1>,
    source_seals: Vec<VerifiedReadbackSourceSealV1>,
    chunk_seals: Vec<VerifiedReadbackChunkSealV1>,
}

pub type VerifiedPrimaryAssetReadbackPartsV1 = (
    String,
    Vec<u8>,
    Vec<u8>,
    Vec<VerifiedReadbackSidecarV1>,
    Vec<VerifiedReadbackSourceSealV1>,
    Vec<VerifiedReadbackChunkSealV1>,
);

impl std::fmt::Debug for VerifiedPrimaryAssetReadbackV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("VerifiedPrimaryAssetReadbackV1")
            .field("asset_path", &self.asset_path)
            .field("uasset_bytes", &self.uasset.len())
            .field("uexp_bytes", &self.uexp.len())
            .field("sidecar_count", &self.sidecars.len())
            .field("source_seals", &self.source_seals)
            .field("chunk_seals", &self.chunk_seals)
            .finish()
    }
}

impl VerifiedPrimaryAssetReadbackV1 {
    pub fn asset_path(&self) -> &str {
        &self.asset_path
    }

    pub fn uasset(&self) -> &[u8] {
        &self.uasset
    }

    pub fn uexp(&self) -> &[u8] {
        &self.uexp
    }

    pub fn sidecars(&self) -> &[VerifiedReadbackSidecarV1] {
        &self.sidecars
    }

    pub fn sidecar(&self, kind: VerifiedReadbackSidecarKindV1) -> Option<&[u8]> {
        self.sidecars
            .iter()
            .find(|sidecar| sidecar.kind == kind)
            .map(|sidecar| sidecar.bytes.as_slice())
    }

    pub fn source_seals(&self) -> &[VerifiedReadbackSourceSealV1] {
        &self.source_seals
    }

    pub fn chunk_seals(&self) -> &[VerifiedReadbackChunkSealV1] {
        &self.chunk_seals
    }

    pub fn into_parts(self) -> VerifiedPrimaryAssetReadbackPartsV1 {
        (
            self.asset_path,
            self.uasset,
            self.uexp,
            self.sidecars,
            self.source_seals,
            self.chunk_seals,
        )
    }
}

/// Exact, write-free generation probe for one package in a composite store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedAssetGeneration {
    pub consumed_chunks: Vec<VerifiedChunkReceipt>,
    pub metadata_utocs: Vec<PathBuf>,
    pub opened_utocs: Vec<VerifiedOpenedUtocReceipt>,
}

#[derive(Debug, Clone)]
struct CachedChunk {
    bytes: Vec<u8>,
    receipt: VerifiedChunkReceipt,
}

/// Read-through IoStore snapshot. Every first read selects the exact composite
/// winner, verifies its decompressed bytes against the TOC BLAKE3 chunk hash,
/// and caches those bytes. All conversion re-reads then use the immutable cache.
struct VerifiedSnapshotStore<'a> {
    inner: &'a dyn iostore::IoStoreTrait,
    chunks: Mutex<std::collections::HashMap<FIoChunkId, CachedChunk>>,
}

impl<'a> VerifiedSnapshotStore<'a> {
    fn new(inner: &'a dyn iostore::IoStoreTrait) -> Self {
        Self {
            inner,
            chunks: Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn receipts(&self) -> anyhow::Result<Vec<VerifiedChunkReceipt>> {
        let chunks = self
            .chunks
            .lock()
            .map_err(|_| anyhow::anyhow!("verified chunk cache was poisoned"))?;
        let mut receipts: Vec<_> = chunks.values().map(|chunk| chunk.receipt.clone()).collect();
        receipts.sort_by(|left, right| left.chunk_id.cmp(&right.chunk_id));
        Ok(receipts)
    }

    fn prime_container_metadata(&self) -> anyhow::Result<()> {
        let header_ids: Vec<_> = if self.inner.opened_utoc_identity().is_some() {
            self.inner
                .chunks()
                .filter(|chunk| chunk.id().get_chunk_type() == EIoChunkType::ContainerHeader)
                .map(|chunk| chunk.id())
                .collect()
        } else {
            self.inner
                .child_containers()
                .flat_map(|container| container.chunks())
                .filter(|chunk| chunk.id().get_chunk_type() == EIoChunkType::ContainerHeader)
                .map(|chunk| chunk.id())
                .collect()
        };
        if header_ids.is_empty() {
            anyhow::bail!("composite IoStore exposes no ContainerHeader chunks");
        }
        for chunk_id in header_ids {
            self.read_verified(chunk_id)?;
        }
        Ok(())
    }

    fn metadata_utoc_receipts(&self) -> anyhow::Result<Vec<VerifiedOpenedUtocReceipt>> {
        let mut receipts = Vec::new();
        let mut push_identity = |container: &dyn iostore::IoStoreTrait| -> anyhow::Result<()> {
            let (source_utoc, parsed_blake3) = container
                .opened_utoc_identity()
                .ok_or_else(|| anyhow::anyhow!("opened child container has no UTOC identity"))?;
            receipts.push(VerifiedOpenedUtocReceipt {
                source_utoc: source_utoc.to_path_buf(),
                source_utoc_blake3: encode_bytes(parsed_blake3),
            });
            Ok(())
        };
        if self.inner.opened_utoc_identity().is_some() {
            push_identity(self.inner)?;
        } else {
            for container in self.inner.child_containers() {
                push_identity(container)?;
            }
        }
        receipts.sort();
        let before_dedup = receipts.len();
        receipts.dedup();
        if receipts.len() != before_dedup {
            anyhow::bail!("opened child-container UTOC identities are duplicated");
        }
        Ok(receipts)
    }

    fn metadata_utocs(&self) -> anyhow::Result<Vec<PathBuf>> {
        Ok(self
            .metadata_utoc_receipts()?
            .into_iter()
            .map(|receipt| receipt.source_utoc)
            .collect())
    }

    fn read_verified(&self, chunk_id: FIoChunkId) -> anyhow::Result<Vec<u8>> {
        let version = self
            .inner
            .container_file_version()
            .ok_or_else(|| anyhow::anyhow!("container has no TOC version"))?;
        let chunk_id = chunk_id.with_version(version);
        let mut chunks = self
            .chunks
            .lock()
            .map_err(|_| anyhow::anyhow!("verified chunk cache was poisoned"))?;
        if let Some(cached) = chunks.get(&chunk_id).cloned() {
            return Ok(cached.bytes);
        }

        // `chunks()` applies the composite store's same first-winner precedence
        // as `read()`, but retains the concrete source container + TOC hash.
        let info = self
            .inner
            .chunks()
            .find(|info| info.id() == chunk_id)
            .ok_or_else(|| anyhow::anyhow!("{chunk_id:?} not found in composite IoStore"))?;
        let advertised = info.size();
        if advertised > MAX_SNAPSHOT_CHUNK_BYTES {
            anyhow::bail!(
                "IoStore chunk {chunk_id:?} advertises {advertised} bytes; per-chunk snapshot limit is {MAX_SNAPSHOT_CHUNK_BYTES}"
            );
        }
        let cached_total = chunks.values().try_fold(0u64, |total, cached| {
            total
                .checked_add(cached.receipt.length)
                .ok_or_else(|| anyhow::anyhow!("verified chunk cache size overflowed"))
        })?;
        let prospective_total = cached_total
            .checked_add(advertised)
            .ok_or_else(|| anyhow::anyhow!("verified chunk cache size overflowed"))?;
        if prospective_total > MAX_SNAPSHOT_TOTAL_BYTES {
            anyhow::bail!(
                "IoStore snapshot would reach {prospective_total} bytes; aggregate limit is {MAX_SNAPSHOT_TOTAL_BYTES}"
            );
        }
        let bytes = info.read()?;
        if u64::try_from(bytes.len())? != advertised {
            anyhow::bail!(
                "IoStore chunk {chunk_id:?} length mismatch: advertised {advertised}, read {}",
                bytes.len()
            );
        }
        let receipt = verified_chunk_receipt(&info, &bytes)?;
        let cached = CachedChunk { bytes, receipt };
        let result = cached.bytes.clone();
        chunks.insert(chunk_id, cached);
        Ok(result)
    }
}

impl iostore::IoStoreTrait for VerifiedSnapshotStore<'_> {
    fn container_name(&self) -> &str {
        self.inner.container_name()
    }

    fn container_file_version(&self) -> Option<EIoStoreTocVersion> {
        self.inner.container_file_version()
    }

    fn container_header_version(&self) -> Option<EIoContainerHeaderVersion> {
        self.inner.container_header_version()
    }

    fn print_info(&self, depth: usize) {
        self.inner.print_info(depth);
    }

    fn read(&self, chunk_id: FIoChunkId) -> anyhow::Result<Vec<u8>> {
        self.read_verified(chunk_id)
    }

    fn read_raw(&self, chunk_id_raw: FIoChunkIdRaw) -> anyhow::Result<Vec<u8>> {
        let version = self
            .inner
            .container_file_version()
            .ok_or_else(|| anyhow::anyhow!("container has no TOC version"))?;
        self.read_verified(FIoChunkId::from_raw(chunk_id_raw, version))
    }

    fn has_chunk_id(&self, chunk_id: FIoChunkId) -> bool {
        self.inner.has_chunk_id(chunk_id)
    }

    fn has_chunk_id_raw(&self, chunk_id_raw: FIoChunkIdRaw) -> bool {
        self.inner.has_chunk_id_raw(chunk_id_raw)
    }

    fn chunks(&self) -> Box<dyn Iterator<Item = iostore::ChunkInfo<'_>> + Send + '_> {
        self.inner.chunks()
    }

    fn chunks_all(&self) -> Box<dyn Iterator<Item = iostore::ChunkInfo<'_>> + Send + '_> {
        self.inner.chunks_all()
    }

    fn packages(&self) -> Box<dyn Iterator<Item = iostore::PackageInfo<'_>> + Send + '_> {
        self.inner.packages()
    }

    fn packages_all(&self) -> Box<dyn Iterator<Item = iostore::PackageInfo<'_>> + Send + '_> {
        self.inner.packages_all()
    }

    fn child_containers(&self) -> Box<dyn Iterator<Item = &dyn iostore::IoStoreTrait> + '_> {
        self.inner.child_containers()
    }

    fn chunk_path(&self, chunk_id: FIoChunkId) -> Option<String> {
        self.inner.chunk_path(chunk_id)
    }

    fn package_store_entry(&self, package_id: FPackageId) -> Option<StoreEntry> {
        self.inner.package_store_entry(package_id)
    }

    fn lookup_package_redirect(&self, source_package_id: FPackageId) -> Option<FPackageId> {
        self.inner.lookup_package_redirect(source_package_id)
    }
}

/// A closed composite used only by the post-build readback API. Target package
/// data is routed to `primary` exclusively. Every other chunk is primary-first,
/// then falls back to the installed game for script objects/imported packages.
struct PrimaryTargetFallbackStore<'a> {
    primary: &'a dyn iostore::IoStoreTrait,
    fallback: &'a dyn iostore::IoStoreTrait,
    target_package: FPackageId,
    container_version: EIoStoreTocVersion,
    header_version: EIoContainerHeaderVersion,
}

impl<'a> PrimaryTargetFallbackStore<'a> {
    fn new(
        primary: &'a dyn iostore::IoStoreTrait,
        fallback: &'a dyn iostore::IoStoreTrait,
        target_package: FPackageId,
    ) -> anyhow::Result<Self> {
        let container_version = primary
            .container_file_version()
            .ok_or_else(|| anyhow::anyhow!("primary IoStore pair has no TOC version"))?;
        let fallback_container_version = fallback
            .container_file_version()
            .ok_or_else(|| anyhow::anyhow!("game fallback has no TOC version"))?;
        if fallback_container_version != container_version {
            anyhow::bail!("primary IoStore pair and game fallback use different TOC versions");
        }
        let header_version = primary.container_header_version().ok_or_else(|| {
            anyhow::anyhow!("primary IoStore pair has no container-header version")
        })?;
        let fallback_header_version = fallback
            .container_header_version()
            .ok_or_else(|| anyhow::anyhow!("game fallback has no container-header version"))?;
        if fallback_header_version != header_version {
            anyhow::bail!(
                "primary IoStore pair and game fallback use different container-header versions"
            );
        }
        Ok(Self {
            primary,
            fallback,
            target_package,
            container_version,
            header_version,
        })
    }

    fn is_target_package_chunk(&self, chunk_id: FIoChunkId) -> bool {
        chunk_id.get_package_id() == self.target_package
            && matches!(
                chunk_id.get_chunk_type(),
                EIoChunkType::ExportBundleData
                    | EIoChunkType::BulkData
                    | EIoChunkType::OptionalBulkData
                    | EIoChunkType::MemoryMappedBulkData
            )
    }

    fn read_routed(&self, chunk_id: FIoChunkId) -> anyhow::Result<Vec<u8>> {
        let chunk_id = chunk_id.with_version(self.container_version);
        if self.is_target_package_chunk(chunk_id) {
            if !self.primary.has_chunk_id(chunk_id) {
                anyhow::bail!(
                    "target package chunk {chunk_id:?} is absent from the primary IoStore pair; game fallback is forbidden"
                );
            }
            return self.primary.read(chunk_id);
        }
        if self.primary.has_chunk_id(chunk_id) {
            self.primary.read(chunk_id)
        } else {
            self.fallback.read(chunk_id)
        }
    }
}

impl iostore::IoStoreTrait for PrimaryTargetFallbackStore<'_> {
    fn container_name(&self) -> &str {
        "VERIFIED_PRIMARY_TARGET_WITH_GAME_FALLBACK"
    }

    fn container_file_version(&self) -> Option<EIoStoreTocVersion> {
        Some(self.container_version)
    }

    fn container_header_version(&self) -> Option<EIoContainerHeaderVersion> {
        Some(self.header_version)
    }

    fn print_info(&self, depth: usize) {
        self.primary.print_info(depth);
        self.fallback.print_info(depth);
    }

    fn read(&self, chunk_id: FIoChunkId) -> anyhow::Result<Vec<u8>> {
        self.read_routed(chunk_id)
    }

    fn read_raw(&self, chunk_id_raw: FIoChunkIdRaw) -> anyhow::Result<Vec<u8>> {
        self.read_routed(FIoChunkId::from_raw(chunk_id_raw, self.container_version))
    }

    fn has_chunk_id(&self, chunk_id: FIoChunkId) -> bool {
        let chunk_id = chunk_id.with_version(self.container_version);
        if self.is_target_package_chunk(chunk_id) {
            self.primary.has_chunk_id(chunk_id)
        } else {
            self.primary.has_chunk_id(chunk_id) || self.fallback.has_chunk_id(chunk_id)
        }
    }

    fn has_chunk_id_raw(&self, chunk_id_raw: FIoChunkIdRaw) -> bool {
        self.has_chunk_id(FIoChunkId::from_raw(chunk_id_raw, self.container_version))
    }

    fn chunks(&self) -> Box<dyn Iterator<Item = iostore::ChunkInfo<'_>> + Send + '_> {
        let target = self.target_package;
        let mut seen = std::collections::HashSet::new();
        Box::new(
            self.primary
                .chunks()
                .chain(self.fallback.chunks().filter(move |chunk| {
                    chunk.id().get_package_id() != target
                        || !matches!(
                            chunk.id().get_chunk_type(),
                            EIoChunkType::ExportBundleData
                                | EIoChunkType::BulkData
                                | EIoChunkType::OptionalBulkData
                                | EIoChunkType::MemoryMappedBulkData
                        )
                }))
                .filter(move |chunk| seen.insert(chunk.id().get_raw())),
        )
    }

    fn chunks_all(&self) -> Box<dyn Iterator<Item = iostore::ChunkInfo<'_>> + Send + '_> {
        let target = self.target_package;
        let mut seen = std::collections::HashSet::new();
        Box::new(
            self.primary
                .chunks_all()
                .chain(self.fallback.chunks_all().filter(move |chunk| {
                    chunk.id().get_package_id() != target
                        || !matches!(
                            chunk.id().get_chunk_type(),
                            EIoChunkType::ExportBundleData
                                | EIoChunkType::BulkData
                                | EIoChunkType::OptionalBulkData
                                | EIoChunkType::MemoryMappedBulkData
                        )
                }))
                .filter(move |chunk| seen.insert(chunk.id().get_raw())),
        )
    }

    fn packages(&self) -> Box<dyn Iterator<Item = iostore::PackageInfo<'_>> + Send + '_> {
        let target = self.target_package;
        let mut seen = std::collections::HashSet::new();
        Box::new(
            self.primary
                .packages()
                .chain(
                    self.fallback
                        .packages()
                        .filter(move |package| package.id() != target),
                )
                .filter(move |package| seen.insert(package.id())),
        )
    }

    fn packages_all(&self) -> Box<dyn Iterator<Item = iostore::PackageInfo<'_>> + Send + '_> {
        let target = self.target_package;
        let mut seen = std::collections::HashSet::new();
        Box::new(
            self.primary
                .packages_all()
                .chain(
                    self.fallback
                        .packages_all()
                        .filter(move |package| package.id() != target),
                )
                .filter(move |package| seen.insert(package.id())),
        )
    }

    fn child_containers(&self) -> Box<dyn Iterator<Item = &dyn iostore::IoStoreTrait> + '_> {
        Box::new(std::iter::once(self.primary).chain(self.fallback.child_containers()))
    }

    fn chunk_path(&self, chunk_id: FIoChunkId) -> Option<String> {
        let chunk_id = chunk_id.with_version(self.container_version);
        if self.is_target_package_chunk(chunk_id) {
            self.primary.chunk_path(chunk_id)
        } else {
            self.primary
                .chunk_path(chunk_id)
                .or_else(|| self.fallback.chunk_path(chunk_id))
        }
    }

    fn package_store_entry(&self, package_id: FPackageId) -> Option<StoreEntry> {
        if package_id == self.target_package {
            self.primary.package_store_entry(package_id)
        } else {
            self.primary
                .package_store_entry(package_id)
                .or_else(|| self.fallback.package_store_entry(package_id))
        }
    }

    fn lookup_package_redirect(&self, source_package_id: FPackageId) -> Option<FPackageId> {
        if source_package_id == self.target_package {
            self.primary.lookup_package_redirect(source_package_id)
        } else {
            self.primary
                .lookup_package_redirect(source_package_id)
                .or_else(|| self.fallback.lookup_package_redirect(source_package_id))
        }
    }
}

fn encode_bytes(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing into String cannot fail");
    }
    encoded
}

fn verified_chunk_receipt(
    info: &iostore::ChunkInfo<'_>,
    bytes: &[u8],
) -> anyhow::Result<VerifiedChunkReceipt> {
    let chunk_id = info.id();
    let actual = blake3::hash(bytes);
    let expected = &info.hash().0;
    // Older TOCs serialize 20 BLAKE3 bytes and retoc zero-fills the tail;
    // newer TOCs carry all 32.
    let toc_hash_bytes = if expected[20..].iter().any(|byte| *byte != 0) {
        32
    } else {
        20
    };
    if actual.as_bytes()[..toc_hash_bytes] != expected[..toc_hash_bytes] {
        anyhow::bail!(
            "IoStore chunk hash mismatch for {chunk_id:?} from {}",
            info.container().container_path().display()
        );
    }
    Ok(VerifiedChunkReceipt {
        chunk_id: encode_bytes(&chunk_id.get_raw().id),
        chunk_type: format!("{:?}", chunk_id.get_chunk_type()),
        source_utoc: info.container().container_path().to_path_buf(),
        source_utoc_blake3: encode_bytes(info.container().toc_blake3()),
        length: u64::try_from(bytes.len())?,
        blake3: encode_bytes(actual.as_bytes()),
        toc_hash: encode_bytes(&expected[..toc_hash_bytes]),
        toc_hash_bytes,
    })
}

/// Full script-object path of the cooked Texture2D class.
const TEXTURE2D_CLASS_PATH: &str = "/Script/Engine.Texture2D";
const MAX_REPACK_TREE_DEPTH: usize = 128;
const MAX_REPACK_ASSETS: usize = 4096;
const MAX_REPACK_COMPONENT_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_REPACK_BUNDLE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_SNAPSHOT_CHUNK_BYTES: u64 = 512 * 1024 * 1024;
const MAX_SNAPSHOT_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_LEGACY_MEMORY_UASSET_BYTES: usize = 64 * 1024 * 1024;
const MAX_LEGACY_MEMORY_UEXP_BYTES: usize = 256 * 1024 * 1024;
const MAX_LEGACY_MEMORY_PACKAGE_PAIR_BYTES: usize = 320 * 1024 * 1024;
const MAX_LEGACY_MEMORY_SIDECAR_BYTES: usize = 256 * 1024 * 1024;
const MAX_LEGACY_MEMORY_TOTAL_BYTES: usize = 512 * 1024 * 1024;

/// List texture assets in an IoStore container, using `usmap` to resolve types.
///
/// Filters to `UTexture2D`-class exports. `filter` keeps only paths containing the
/// substring.
///
/// Note: `usmap` is accepted for API symmetry with the rest of `gore-tex`; class
/// resolution here is driven by the container's own script-object table (which is
/// exact for the cooked class name) and does not require usmap property parsing.
pub fn list_textures(
    utoc: &Path,
    _usmap: &Path,
    filter: Option<&str>,
) -> Result<Vec<TextureEntry>> {
    // The script-import index UE assigns to the Texture2D class. Computed the same
    // way the cooker does (cityhash of the normalised path) so we can match it
    // without the engine's global script-object table.
    let texture2d_class = FPackageObjectIndex::create_script_import(TEXTURE2D_CLASS_PATH);

    let paths = collect_package_paths(utoc, Some(texture2d_class))?;
    Ok(paths
        .into_iter()
        .filter(|p| filter.is_none_or(|f| p.contains(f)))
        .map(|asset_path| TextureEntry { asset_path })
        .collect())
}

/// List every package asset path in the container at `utoc` (standalone foreign
/// triplets OK: `iostore::open` dispatches a file path to a single-container
/// store). Returns sorted, deduped cooked package paths, e.g.
/// `/Game/Characters/Hero/T_Hero_BaseColor` -- the mod-manager uses this to
/// detect asset overlaps between mods.
pub fn list_packages(utoc: &Path) -> Result<Vec<String>> {
    collect_package_paths(utoc, None)
}

/// Shared per-package scan behind `list_textures`/`list_packages`: parse every
/// package's zen header and collect its asset path, keeping only packages with an
/// export of class `class_filter` when one is given (every package when `None`).
/// Returns sorted, deduped paths.
fn collect_package_paths(
    utoc: &Path,
    class_filter: Option<FPackageObjectIndex>,
) -> Result<Vec<String>> {
    let store = iostore::open(utoc, Arc::new(Config::default()))?;

    let container_version = store
        .container_file_version()
        .ok_or_else(|| anyhow::anyhow!("container has no TOC version"))?;
    let header_version = store
        .container_header_version()
        .ok_or_else(|| anyhow::anyhow!("container has no header version"))?;

    let mut out = Vec::new();

    // Silence the default panic hook for the duration of the loop so the panics we
    // intentionally catch below (one per malformed package) don't spam stderr with
    // backtraces. Restored before returning.
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    for pkg in store.packages() {
        let pkg_id = pkg.id();

        // The per-package work below is panic-safe: a malformed package can not only
        // return `Err` from `read`/`deserialize` but also *panic* deeper in retoc --
        // e.g. `header.package_name()` -> `FNameMap::get` asserts on name kind and
        // indexes `self.names` unchecked, so an out-of-range name index aborts. We
        // wrap the whole body in `catch_unwind` so one bad package is skipped, not
        // fatal to the entire listing. The closure returns `Some(path)` for a
        // matching package, `None` for a non-match or any handled failure; a caught
        // panic is treated exactly like the previous `Err(_) => continue` path.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let chunk_id = FIoChunkId::from_package_id(pkg_id, 0, EIoChunkType::ExportBundleData);

            // Some package entries may not have a readable export-bundle chunk; skip
            // them rather than failing the whole listing.
            let data = match store.read(chunk_id) {
                Ok(d) => d,
                Err(_) => return None,
            };

            let header = match FZenPackageHeader::deserialize(
                &mut Cursor::new(&data),
                store.package_store_entry(pkg_id),
                container_version,
                header_version,
                None,
            ) {
                Ok(h) => h,
                Err(_) => return None,
            };

            // Does any export match the requested class? Compare each export's class
            // import index to the precomputed script-import index (no filter == keep).
            let keep = class_filter.is_none_or(|class| {
                header
                    .export_map
                    .iter()
                    .any(|export| export.class_index == class)
            });

            if !keep {
                return None;
            }

            Some(header.package_name())
        }));

        // Caught panic == skip this package (same as the `Err(_) => continue` arms).
        if let Ok(Some(path)) = result {
            out.push(path);
        }
    }

    std::panic::set_hook(prev_hook);

    out.sort();
    out.dedup();
    Ok(out)
}

/// List the file entry paths of a plain (non-IoStore) V11 `.pak`, sorted and
/// deduped. Paths are as recorded in the pak index (relative to its mount
/// point), e.g. `G1R/Content/UI/Textures/Common/T_HardwareCursor.uasset` -- the
/// mod-manager uses this to inspect foreign pak-only mods.
pub fn list_pak_files(pak: &Path) -> Result<Vec<String>> {
    let mut file = std::io::BufReader::new(std::fs::File::open(pak)?);
    let reader = repak::PakBuilder::new()
        .reader(&mut file)
        .map_err(|e| anyhow::anyhow!("failed to read pak index of {}: {e}", pak.display()))?;
    let mut files = reader.files();
    files.sort();
    files.dedup();
    Ok(files)
}

/// Strictly reopen a freshly produced one-package additive triplet. Unlike the
/// tolerant foreign-mod listing APIs, this fails on any malformed/extra package
/// chunk, verifies every chunk against its TOC BLAKE3 hash, checks the exact
/// package/header/path mapping, and reopens the empty V11 `.pak` sidecar.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct StrictTripletVerification {
    package: String,
    export_path: String,
    chunk_count: usize,
    chunks: Vec<VerifiedChunkReceipt>,
    pak_mount_point: String,
    pak_files: Vec<String>,
    bulk_chunks: usize,
    optional_bulk_chunks: usize,
    memory_mapped_bulk_chunks: usize,
}

impl StrictTripletVerification {
    pub fn package(&self) -> &str {
        &self.package
    }

    pub fn export_path(&self) -> &str {
        &self.export_path
    }

    pub fn chunk_count(&self) -> usize {
        self.chunk_count
    }

    pub fn chunks(&self) -> &[VerifiedChunkReceipt] {
        &self.chunks
    }

    pub fn pak_mount_point(&self) -> &str {
        &self.pak_mount_point
    }

    pub fn pak_files(&self) -> &[String] {
        &self.pak_files
    }

    pub fn bulk_chunks(&self) -> usize {
        self.bulk_chunks
    }

    pub fn optional_bulk_chunks(&self) -> usize {
        self.optional_bulk_chunks
    }

    pub fn memory_mapped_bulk_chunks(&self) -> usize {
        self.memory_mapped_bulk_chunks
    }

    pub fn verify_primary_readback_binding_v1(
        &self,
        readback: &VerifiedPrimaryAssetReadbackV1,
    ) -> Result<()> {
        if self.package != readback.asset_path {
            return Err(anyhow::anyhow!(
                "strict triplet package does not match primary readback asset path"
            )
            .into());
        }
        if self.chunk_count != self.chunks.len() {
            return Err(anyhow::anyhow!("strict triplet chunk count is inconsistent").into());
        }

        let primary_sources: Vec<_> = readback
            .source_seals
            .iter()
            .filter(|source| source.role == VerifiedReadbackSourceRoleV1::Primary)
            .collect();
        let [primary_source] = primary_sources.as_slice() else {
            return Err(anyhow::anyhow!(
                "primary readback must contain exactly one primary UTOC source seal"
            )
            .into());
        };

        let mut strict_chunks = Vec::with_capacity(self.chunks.len());
        let mut strict_chunk_ids = std::collections::HashSet::with_capacity(self.chunks.len());
        for chunk in &self.chunks {
            let chunk_id = decode_fixed_hex(&chunk.chunk_id, "strict triplet chunk id")?;
            if !strict_chunk_ids.insert(chunk_id) {
                return Err(anyhow::anyhow!("strict triplet contains a duplicate chunk id").into());
            }
            let source_utoc_blake3 = decode_fixed_hex(
                &chunk.source_utoc_blake3,
                "strict triplet chunk source UTOC BLAKE3",
            )?;
            if source_utoc_blake3 != primary_source.utoc_blake3 {
                return Err(anyhow::anyhow!(
                    "strict triplet chunk source does not match the primary UTOC seal"
                )
                .into());
            }
            let blake3 = decode_fixed_hex(&chunk.blake3, "strict triplet chunk BLAKE3")?;
            let toc_hash = decode_hex(&chunk.toc_hash, "strict triplet chunk TOC hash")?;
            if toc_hash.len() != chunk.toc_hash_bytes {
                return Err(anyhow::anyhow!(
                    "strict triplet chunk TOC hash length is inconsistent"
                )
                .into());
            }
            strict_chunks.push((chunk_id, blake3, toc_hash, chunk));
        }
        strict_chunks.sort_by_key(|(chunk_id, _, _, _)| *chunk_id);

        if readback.chunk_seals.iter().any(|chunk| {
            chunk.source_role == VerifiedReadbackSourceRoleV1::Fallback
                && strict_chunk_ids.contains(&chunk.chunk_id)
        }) {
            return Err(anyhow::anyhow!(
                "primary readback binds a strict triplet chunk to fallback"
            )
            .into());
        }

        let mut primary_chunks: Vec<_> = readback
            .chunk_seals
            .iter()
            .filter(|chunk| chunk.source_role == VerifiedReadbackSourceRoleV1::Primary)
            .collect();
        primary_chunks.sort_by_key(|chunk| chunk.chunk_id);
        if strict_chunks.len() != primary_chunks.len() {
            return Err(
                anyhow::anyhow!("strict triplet and primary readback chunk sets differ").into(),
            );
        }

        for ((chunk_id, blake3, toc_hash, strict), readback) in
            strict_chunks.iter().zip(primary_chunks)
        {
            if readback.source_utoc_blake3 != primary_source.utoc_blake3
                || readback.chunk_id != *chunk_id
                || readback.chunk_type != strict.chunk_type
                || readback.length != strict.length
                || readback.blake3 != *blake3
                || readback.toc_hash() != toc_hash
            {
                return Err(anyhow::anyhow!(
                    "strict triplet and primary readback chunk seals differ"
                )
                .into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedSidecars {
    pub bulk: bool,
    pub optional_bulk: bool,
    pub memory_mapped_bulk: bool,
}

pub fn verify_single_package_triplet(
    utoc: &Path,
    pak: &Path,
    expected_asset: &str,
    expected_sidecars: ExpectedSidecars,
) -> Result<StrictTripletVerification> {
    let store = iostore::open(utoc, Arc::new(Config::default()))?;
    let container_version = store
        .container_file_version()
        .ok_or_else(|| anyhow::anyhow!("triplet container has no TOC version"))?;
    let header_version = store
        .container_header_version()
        .ok_or_else(|| anyhow::anyhow!("triplet container has no header version"))?;
    let expected_package_id = package_id_from_asset_path(expected_asset);
    let packages: Vec<_> = store.packages().map(|package| package.id()).collect();
    if packages != [expected_package_id] {
        return Err(anyhow::anyhow!(
            "triplet package set mismatch: expected {:016x}, got {:?}",
            expected_package_id.0,
            packages
        )
        .into());
    }

    let mut seen_ids = std::collections::HashSet::new();
    let mut chunks = Vec::new();
    let mut export_data: Option<Vec<u8>> = None;
    let mut export_path: Option<String> = None;
    let mut container_headers = 0usize;
    let mut bulk_chunks = 0usize;
    let mut optional_bulk_chunks = 0usize;
    let mut memory_mapped_bulk_chunks = 0usize;
    let mut chunk_total = 0u64;
    for info in store.chunks_all() {
        let chunk_id = info.id();
        if !seen_ids.insert(chunk_id.get_raw()) {
            return Err(anyhow::anyhow!("duplicate chunk id in triplet: {chunk_id:?}").into());
        }
        let advertised = info.size();
        if advertised > MAX_SNAPSHOT_CHUNK_BYTES {
            return Err(anyhow::anyhow!(
                "triplet chunk {chunk_id:?} is {advertised} bytes; limit is {MAX_SNAPSHOT_CHUNK_BYTES}"
            )
            .into());
        }
        chunk_total = chunk_total
            .checked_add(advertised)
            .ok_or_else(|| anyhow::anyhow!("triplet chunk size overflowed"))?;
        if chunk_total > MAX_SNAPSHOT_TOTAL_BYTES {
            return Err(anyhow::anyhow!(
                "triplet chunks total {chunk_total} bytes; limit is {MAX_SNAPSHOT_TOTAL_BYTES}"
            )
            .into());
        }
        let bytes = info.read()?;
        if u64::try_from(bytes.len()).map_err(|_| anyhow::anyhow!("chunk length overflowed"))?
            != advertised
        {
            return Err(anyhow::anyhow!(
                "triplet chunk {chunk_id:?} length differs from its TOC size"
            )
            .into());
        }
        chunks.push(verified_chunk_receipt(&info, &bytes)?);
        match chunk_id.get_chunk_type() {
            EIoChunkType::ContainerHeader => {
                container_headers += 1;
            }
            EIoChunkType::ExportBundleData => {
                if chunk_id.get_package_id() != expected_package_id || export_data.is_some() {
                    return Err(anyhow::anyhow!(
                        "triplet contains an extra or mismatched export-bundle chunk"
                    )
                    .into());
                }
                export_path = info.path();
                export_data = Some(bytes);
            }
            EIoChunkType::BulkData => {
                if chunk_id.get_package_id() != expected_package_id {
                    return Err(anyhow::anyhow!(
                        "triplet contains bulk data for a different package"
                    )
                    .into());
                }
                bulk_chunks += 1;
            }
            EIoChunkType::OptionalBulkData => {
                if chunk_id.get_package_id() != expected_package_id {
                    return Err(anyhow::anyhow!(
                        "triplet contains optional bulk data for a different package"
                    )
                    .into());
                }
                optional_bulk_chunks += 1;
            }
            EIoChunkType::MemoryMappedBulkData => {
                if chunk_id.get_package_id() != expected_package_id {
                    return Err(anyhow::anyhow!(
                        "triplet contains memory-mapped bulk data for a different package"
                    )
                    .into());
                }
                memory_mapped_bulk_chunks += 1;
            }
            other => {
                return Err(
                    anyhow::anyhow!("unexpected {:?} chunk in one-package triplet", other).into(),
                );
            }
        }
    }
    if container_headers != 1 {
        return Err(anyhow::anyhow!(
            "triplet must contain exactly one ContainerHeader chunk, got {container_headers}"
        )
        .into());
    }
    let expected_counts = [
        usize::from(expected_sidecars.bulk),
        usize::from(expected_sidecars.optional_bulk),
        usize::from(expected_sidecars.memory_mapped_bulk),
    ];
    let actual_counts = [bulk_chunks, optional_bulk_chunks, memory_mapped_bulk_chunks];
    if actual_counts != expected_counts {
        return Err(anyhow::anyhow!(
            "triplet sidecar chunk mismatch: expected {:?}, got {:?}",
            expected_counts,
            actual_counts
        )
        .into());
    }
    let export_data = export_data.ok_or_else(|| {
        anyhow::anyhow!("triplet has no export-bundle chunk for expected package")
    })?;
    let package = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        || -> anyhow::Result<String> {
            let header = FZenPackageHeader::deserialize(
                &mut Cursor::new(&export_data),
                store.package_store_entry(expected_package_id),
                container_version,
                header_version,
                None,
            )?;
            Ok(header.package_name())
        },
    ))
    .map_err(|_| anyhow::anyhow!("triplet package header panicked while reopening"))??;
    if package != expected_asset {
        return Err(anyhow::anyhow!(
            "triplet package name mismatch: expected {expected_asset:?}, got {package:?}"
        )
        .into());
    }
    let expected_export_path = format!(
        "../../../{}.uasset",
        crate::paths::content_mount_rel(expected_asset)
            .ok_or_else(|| anyhow::anyhow!("unsupported package mount: {expected_asset}"))?
    );
    let export_path = export_path
        .ok_or_else(|| anyhow::anyhow!("triplet export chunk has no directory-index path"))?;
    if export_path != expected_export_path {
        return Err(anyhow::anyhow!(
            "triplet mount path mismatch: expected {expected_export_path:?}, got {export_path:?}"
        )
        .into());
    }

    let mut pak_file = std::io::BufReader::new(std::fs::File::open(pak)?);
    let pak_reader = repak::PakBuilder::new()
        .reader(&mut pak_file)
        .map_err(|error| anyhow::anyhow!("failed to reopen pak index: {error}"))?;
    let pak_mount_point = pak_reader.mount_point().to_owned();
    let mut pak_files = pak_reader.files();
    pak_files.sort();
    if pak_mount_point != "../../../" || !pak_files.is_empty() {
        return Err(anyhow::anyhow!(
            "pak sidecar invariant failed: mount={pak_mount_point:?}, files={pak_files:?}"
        )
        .into());
    }
    chunks.sort_by(|left, right| left.chunk_id.cmp(&right.chunk_id));
    Ok(StrictTripletVerification {
        package,
        export_path,
        chunk_count: chunks.len(),
        chunks,
        pak_mount_point,
        pak_files,
        bulk_chunks,
        optional_bulk_chunks,
        memory_mapped_bulk_chunks,
    })
}

/// Unpack a single asset's cooked files (.uasset/.uexp/.ubulk) from the
/// container into `out_dir`. Returns the path to the written `.uasset`.
///
/// The asset is converted from its on-disk *zen* (IoStore) form back to the
/// legacy cooked `.uasset`/`.uexp`/`.ubulk` layout via retoc's
/// `asset_conversion::build_legacy`. That conversion resolves the package's
/// script imports against the engine's *global* script-object table, which for
/// G1R lives in `global.utoc` -- a sibling of the main container, **not** inside
/// `G1R-Windows.utoc`. So we open the whole Paks *directory* (the parent of
/// `utoc`) as a composite store: `IoStoreBackend` then exposes the global
/// script objects through the default `load_script_objects()` while still
/// serving the asset's chunks. (Opening the single `.utoc` file would make
/// `build_legacy` fail to resolve script imports.)
///
/// `usmap` is accepted for API symmetry; the zen->legacy conversion is driven
/// entirely by the package's own header + the global script-object table and
/// does not need property mappings.
pub fn unpack_asset(
    utoc: &Path,
    usmap: &Path,
    asset_path: &str,
    out_dir: &Path,
) -> Result<PathBuf> {
    Ok(unpack_asset_verified(utoc, usmap, asset_path, out_dir)?.uasset)
}

/// Snapshot-backed form of [`unpack_asset`]. In addition to the legacy output,
/// returns every exact IoStore chunk consumed by conversion, including the
/// winning sibling container and verified TOC BLAKE3 identity. Repeated reads
/// during conversion use immutable cached bytes.
pub fn unpack_asset_verified(
    utoc: &Path,
    _usmap: &Path,
    asset_path: &str,
    out_dir: &Path,
) -> Result<VerifiedUnpackedAsset> {
    // Open the directory holding the .utoc so the composite store also picks up
    // `global.utoc` (script objects) -- required for build_legacy to resolve
    // script imports. Fall back to the file itself if it has no parent.
    let store_path = utoc.parent().unwrap_or(utoc);
    let store = iostore::open(store_path, Arc::new(Config::default()))?;
    let (snapshot, package_id) = verified_package_snapshot(store.as_ref(), asset_path)?;

    std::fs::create_dir_all(out_dir)?;
    let leaf = asset_path.rsplit('/').next().unwrap_or(asset_path);
    let uasset = legacy_from_package(&snapshot, package_id, leaf, out_dir)?;
    let opened_utocs = snapshot.metadata_utoc_receipts()?;
    let metadata_utocs = opened_utocs
        .iter()
        .map(|receipt| receipt.source_utoc.clone())
        .collect();
    Ok(VerifiedUnpackedAsset {
        uasset,
        consumed_chunks: snapshot.receipts()?,
        metadata_utocs,
        opened_utocs,
    })
}

/// Convert one exact package through an already-open composite store without
/// creating an output directory or writing any file.
///
/// This seam is intentionally crate-private. The installed-package snapshot is
/// the public authority that owns the path guards, validates the opened child
/// container set, chooses the candidate server-side, and brackets this call
/// with full source revalidation.
pub(crate) fn unpack_asset_from_open_store_verified_to_memory(
    store: &dyn iostore::IoStoreTrait,
    asset_path: &str,
) -> Result<VerifiedUnpackedAssetBytes> {
    let (snapshot, package_id) = verified_package_snapshot(store, asset_path)?;
    let leaf = asset_path.rsplit('/').next().unwrap_or(asset_path);
    let converted = legacy_from_package_to_memory(&snapshot, package_id, leaf)?;
    Ok(VerifiedUnpackedAssetBytes {
        uasset: converted.uasset,
        uexp: converted.uexp,
        sidecars: converted.sidecars,
        consumed_chunks: snapshot.receipts()?,
        metadata_utocs: snapshot.metadata_utoc_receipts()?,
    })
}

#[derive(Debug)]
struct ReadbackSourceAuthority {
    primary: VerifiedOpenedUtocReceipt,
    fallback: Vec<VerifiedOpenedUtocReceipt>,
}

impl ReadbackSourceAuthority {
    fn new(
        primary: Vec<VerifiedOpenedUtocReceipt>,
        fallback: Vec<VerifiedOpenedUtocReceipt>,
    ) -> anyhow::Result<Self> {
        let [primary] = primary.as_slice() else {
            anyhow::bail!(
                "primary IoStore pair must expose exactly one concrete UTOC identity, got {}",
                primary.len()
            );
        };
        if fallback.is_empty() {
            anyhow::bail!("game fallback exposes no concrete UTOC identities");
        }
        if fallback
            .iter()
            .any(|receipt| receipt.source_utoc == primary.source_utoc)
        {
            anyhow::bail!("primary IoStore pair is also present in the game fallback");
        }
        Ok(Self {
            primary: primary.clone(),
            fallback,
        })
    }

    fn role_for_chunk(
        &self,
        chunk: &VerifiedChunkReceipt,
    ) -> anyhow::Result<VerifiedReadbackSourceRoleV1> {
        if chunk.source_utoc == self.primary.source_utoc {
            if chunk.source_utoc_blake3 != self.primary.source_utoc_blake3 {
                anyhow::bail!("primary chunk UTOC seal differs from its opened source seal");
            }
            return Ok(VerifiedReadbackSourceRoleV1::Primary);
        }
        let source = self
            .fallback
            .iter()
            .find(|source| source.source_utoc == chunk.source_utoc)
            .ok_or_else(|| anyhow::anyhow!("verified chunk came from an unauthorized source"))?;
        if chunk.source_utoc_blake3 != source.source_utoc_blake3 {
            anyhow::bail!("fallback chunk UTOC seal differs from its opened source seal");
        }
        Ok(VerifiedReadbackSourceRoleV1::Fallback)
    }

    fn path_free_seals(&self) -> anyhow::Result<Vec<VerifiedReadbackSourceSealV1>> {
        let mut seals = Vec::with_capacity(self.fallback.len() + 1);
        seals.push(VerifiedReadbackSourceSealV1 {
            role: VerifiedReadbackSourceRoleV1::Primary,
            utoc_blake3: decode_fixed_hex(&self.primary.source_utoc_blake3, "primary UTOC BLAKE3")?,
        });
        for source in &self.fallback {
            seals.push(VerifiedReadbackSourceSealV1 {
                role: VerifiedReadbackSourceRoleV1::Fallback,
                utoc_blake3: decode_fixed_hex(&source.source_utoc_blake3, "fallback UTOC BLAKE3")?,
            });
        }
        seals.sort();
        Ok(seals)
    }
}

/// Reopen the IoStore pair from one freshly built single-package output without
/// writing any files, reconstructing bounded legacy cooked bytes in memory.
///
/// `primary_utoc` selects the built `.utoc` (its `.ucas` sibling is opened by
/// Retoc). `game_dir` is the installation root; only its fixed
/// `G1R/Content/Paks` directory is admitted as a read-only fallback. The target
/// package's export and every known bulk sidecar are *primary-only*. Installed
/// game containers may supply ScriptObjects and imported package dependencies,
/// but can never hide a missing or malformed target chunk in the built pair.
///
/// Before returning, all participating UTOCs and every consumed/primary chunk
/// are reopened and reverified. Any source or winner drift fails closed. The
/// returned object owns bounded `.uasset`/`.uexp`/sidecar bytes and exposes only
/// path-free source/chunk content seals. This function does not open the `.pak`
/// sidecar and therefore does not establish complete triplet authority; callers
/// requiring that must separately succeed with [`verify_single_package_triplet`]
/// and bind both results to the same output.
pub fn reopen_primary_asset_with_game_fallback_to_memory_v1(
    primary_utoc: &Path,
    game_dir: &Path,
    asset_path: &str,
) -> Result<VerifiedPrimaryAssetReadbackV1> {
    let game_paks = game_dir.join("G1R/Content/Paks");
    validate_plain_directory_root(&game_paks, "game Paks fallback")?;

    let target_package = package_id_from_asset_path(asset_path);
    let config = Arc::new(Config::default());

    // Every cache-heavy operation is deliberately isolated in a lexical helper.
    // At most one VerifiedSnapshotStore cache (plus the bounded converted output)
    // can therefore be resident at once.
    let (converted, conversion_chunks, all_chunks, source_authority) = {
        let primary = iostore::open(primary_utoc, config.clone())?;
        let fallback = iostore::open(&game_paks, config.clone())?;
        validate_primary_readback_store(primary.as_ref(), asset_path, target_package)?;
        let routed =
            PrimaryTargetFallbackStore::new(primary.as_ref(), fallback.as_ref(), target_package)?;

        let (primary_sources, primary_chunks) = snapshot_complete_store_verified(primary.as_ref())?;
        let (fallback_sources, fallback_metadata_chunks) =
            snapshot_store_metadata_verified(fallback.as_ref())?;
        let source_authority = ReadbackSourceAuthority::new(primary_sources, fallback_sources)?;
        let (converted, conversion_chunks) =
            convert_routed_asset_to_memory_verified(&routed, asset_path)?;
        let all_chunks = merge_readback_chunk_receipts([
            primary_chunks.as_slice(),
            fallback_metadata_chunks.as_slice(),
            conversion_chunks.as_slice(),
        ])?;
        ensure_target_chunks_are_primary(
            &all_chunks,
            &source_authority,
            target_package,
            routed.container_version,
        )?;
        (converted, conversion_chunks, all_chunks, source_authority)
    };

    // Reopen both authorities after conversion, verify their metadata and the
    // complete primary chunk surface again, then replay every conversion read
    // through a fresh routed snapshot. This detects source/winner drift without
    // copying global files beside the generated IoStore pair.
    {
        let recheck_primary = iostore::open(primary_utoc, config.clone())?;
        let recheck_fallback = iostore::open(&game_paks, config)?;
        validate_primary_readback_store(recheck_primary.as_ref(), asset_path, target_package)?;
        let recheck_routed = PrimaryTargetFallbackStore::new(
            recheck_primary.as_ref(),
            recheck_fallback.as_ref(),
            target_package,
        )?;

        let (recheck_primary_sources, recheck_primary_chunks) =
            snapshot_complete_store_verified(recheck_primary.as_ref())?;
        let (recheck_fallback_sources, recheck_fallback_metadata_chunks) =
            snapshot_store_metadata_verified(recheck_fallback.as_ref())?;
        if recheck_primary_sources.as_slice() != std::slice::from_ref(&source_authority.primary)
            || recheck_fallback_sources != source_authority.fallback
        {
            return Err(
                anyhow::anyhow!("IoStore source identities drifted during readback").into(),
            );
        }

        let recheck_conversion_chunks = replay_verified_chunk_reads(
            &recheck_routed,
            recheck_routed.container_version,
            &conversion_chunks,
        )?;
        let recheck_all_chunks = merge_readback_chunk_receipts([
            recheck_primary_chunks.as_slice(),
            recheck_fallback_metadata_chunks.as_slice(),
            recheck_conversion_chunks.as_slice(),
        ])?;
        if recheck_all_chunks != all_chunks {
            return Err(anyhow::anyhow!("IoStore chunk winners drifted during readback").into());
        }
    }

    let mut sidecars = Vec::with_capacity(converted.sidecars.len());
    for sidecar in converted.sidecars {
        let kind = match sidecar.kind {
            VerifiedLegacySidecarKind::Bulk => VerifiedReadbackSidecarKindV1::Bulk,
            VerifiedLegacySidecarKind::Optional => VerifiedReadbackSidecarKindV1::Optional,
            VerifiedLegacySidecarKind::MemoryMapped => VerifiedReadbackSidecarKindV1::MemoryMapped,
        };
        sidecars.push(VerifiedReadbackSidecarV1 {
            kind,
            bytes: sidecar.bytes,
        });
    }
    sidecars.sort_by_key(|sidecar| sidecar.kind);

    let source_seals = source_authority.path_free_seals()?;
    let mut chunk_seals = Vec::with_capacity(all_chunks.len());
    for chunk in &all_chunks {
        chunk_seals.push(path_free_chunk_seal(chunk, &source_authority)?);
    }
    chunk_seals.sort_by(|left, right| {
        left.source_role
            .cmp(&right.source_role)
            .then(left.source_utoc_blake3.cmp(&right.source_utoc_blake3))
            .then(left.chunk_id.cmp(&right.chunk_id))
    });

    Ok(VerifiedPrimaryAssetReadbackV1 {
        asset_path: asset_path.to_owned(),
        uasset: converted.uasset,
        uexp: converted.uexp,
        sidecars,
        source_seals,
        chunk_seals,
    })
}

fn snapshot_complete_store_verified(
    store: &dyn iostore::IoStoreTrait,
) -> Result<(Vec<VerifiedOpenedUtocReceipt>, Vec<VerifiedChunkReceipt>)> {
    let snapshot = VerifiedSnapshotStore::new(store);
    snapshot.prime_container_metadata()?;
    for chunk_id in store.chunks_all().map(|chunk| chunk.id()) {
        snapshot.read_verified(chunk_id)?;
    }
    Ok((snapshot.metadata_utoc_receipts()?, snapshot.receipts()?))
}

fn snapshot_store_metadata_verified(
    store: &dyn iostore::IoStoreTrait,
) -> Result<(Vec<VerifiedOpenedUtocReceipt>, Vec<VerifiedChunkReceipt>)> {
    let snapshot = VerifiedSnapshotStore::new(store);
    snapshot.prime_container_metadata()?;
    Ok((snapshot.metadata_utoc_receipts()?, snapshot.receipts()?))
}

fn convert_routed_asset_to_memory_verified(
    store: &dyn iostore::IoStoreTrait,
    asset_path: &str,
) -> Result<(LegacyMemoryOutput, Vec<VerifiedChunkReceipt>)> {
    let (snapshot, package_id) = verified_package_snapshot(store, asset_path)?;
    let leaf = asset_path.rsplit('/').next().unwrap_or(asset_path);
    let converted = legacy_from_package_to_memory(&snapshot, package_id, leaf)?;
    Ok((converted, snapshot.receipts()?))
}

fn replay_verified_chunk_reads(
    store: &dyn iostore::IoStoreTrait,
    container_version: EIoStoreTocVersion,
    required: &[VerifiedChunkReceipt],
) -> Result<Vec<VerifiedChunkReceipt>> {
    let snapshot = VerifiedSnapshotStore::new(store);
    snapshot.prime_container_metadata()?;
    for chunk in required {
        let raw = chunk
            .chunk_id
            .parse::<FIoChunkIdRaw>()
            .with_context(|| format!("invalid internal chunk receipt {}", chunk.chunk_id))?;
        snapshot.read_verified(FIoChunkId::from_raw(raw, container_version))?;
    }
    Ok(snapshot.receipts()?)
}

fn validate_primary_readback_store(
    primary: &dyn iostore::IoStoreTrait,
    asset_path: &str,
    target_package: FPackageId,
) -> Result<()> {
    if primary.opened_utoc_identity().is_none() || primary.child_containers().next().is_some() {
        return Err(anyhow::anyhow!(
            "primary readback source must be one concrete UTOC, not a composite"
        )
        .into());
    }
    let packages: Vec<_> = primary.packages().map(|package| package.id()).collect();
    if packages != [target_package] {
        return Err(anyhow::anyhow!(
            "primary IoStore pair package set does not match the expected single package"
        )
        .into());
    }

    let mut seen = std::collections::HashSet::new();
    let mut container_headers = 0usize;
    let mut export_bundles = 0usize;
    let mut bulk = 0usize;
    let mut optional = 0usize;
    let mut memory_mapped = 0usize;
    for chunk in primary.chunks_all() {
        let chunk_id = chunk.id();
        if !seen.insert(chunk_id.get_raw()) {
            return Err(
                anyhow::anyhow!("primary IoStore pair contains a duplicate chunk id").into(),
            );
        }
        match chunk_id.get_chunk_type() {
            EIoChunkType::ContainerHeader => container_headers += 1,
            EIoChunkType::ExportBundleData => {
                if chunk_id.get_package_id() != target_package {
                    return Err(anyhow::anyhow!(
                        "primary IoStore pair export belongs to a different package"
                    )
                    .into());
                }
                export_bundles += 1;
            }
            EIoChunkType::BulkData => {
                if chunk_id.get_package_id() != target_package {
                    return Err(anyhow::anyhow!(
                        "primary IoStore pair bulk data belongs to a different package"
                    )
                    .into());
                }
                bulk += 1;
            }
            EIoChunkType::OptionalBulkData => {
                if chunk_id.get_package_id() != target_package {
                    return Err(anyhow::anyhow!(
                        "primary IoStore pair optional bulk data belongs to a different package"
                    )
                    .into());
                }
                optional += 1;
            }
            EIoChunkType::MemoryMappedBulkData => {
                if chunk_id.get_package_id() != target_package {
                    return Err(anyhow::anyhow!(
                        "primary IoStore pair memory-mapped data belongs to a different package"
                    )
                    .into());
                }
                memory_mapped += 1;
            }
            other => {
                return Err(anyhow::anyhow!(
                    "unexpected {other:?} chunk in primary single-package IoStore pair"
                )
                .into())
            }
        }
    }
    if container_headers != 1
        || export_bundles != 1
        || bulk > 1
        || optional > 1
        || memory_mapped > 1
    {
        return Err(anyhow::anyhow!(
            "primary IoStore pair has an invalid single-package chunk cardinality"
        )
        .into());
    }
    if primary.package_store_entry(target_package).is_none() {
        return Err(
            anyhow::anyhow!("primary IoStore pair has no target package store entry").into(),
        );
    }
    let export_chunk =
        FIoChunkId::from_package_id(target_package, 0, EIoChunkType::ExportBundleData);
    let expected_export_path = format!(
        "../../../{}.uasset",
        crate::paths::content_mount_rel(asset_path)
            .ok_or_else(|| anyhow::anyhow!("unsupported package mount: {asset_path}"))?
    );
    if primary.chunk_path(export_chunk).as_deref() != Some(expected_export_path.as_str()) {
        return Err(anyhow::anyhow!("primary IoStore pair target mount path mismatch").into());
    }
    Ok(())
}

fn merge_readback_chunk_receipts<const N: usize>(
    groups: [&[VerifiedChunkReceipt]; N],
) -> anyhow::Result<Vec<VerifiedChunkReceipt>> {
    let mut merged = std::collections::BTreeMap::<(PathBuf, String), VerifiedChunkReceipt>::new();
    for chunk in groups.into_iter().flatten() {
        let key = (chunk.source_utoc.clone(), chunk.chunk_id.clone());
        if let Some(existing) = merged.get(&key) {
            if existing != chunk {
                anyhow::bail!("conflicting seals for one IoStore source chunk");
            }
        } else {
            merged.insert(key, chunk.clone());
        }
    }
    Ok(merged.into_values().collect())
}

fn ensure_target_chunks_are_primary(
    chunks: &[VerifiedChunkReceipt],
    sources: &ReadbackSourceAuthority,
    target_package: FPackageId,
    container_version: EIoStoreTocVersion,
) -> anyhow::Result<()> {
    let mut target_exports = 0usize;
    for chunk in chunks {
        let raw = chunk
            .chunk_id
            .parse::<FIoChunkIdRaw>()
            .with_context(|| format!("invalid internal chunk receipt {}", chunk.chunk_id))?;
        let chunk_id = FIoChunkId::from_raw(raw, container_version);
        if chunk_id.get_package_id() == target_package
            && matches!(
                chunk_id.get_chunk_type(),
                EIoChunkType::ExportBundleData
                    | EIoChunkType::BulkData
                    | EIoChunkType::OptionalBulkData
                    | EIoChunkType::MemoryMappedBulkData
            )
        {
            if sources.role_for_chunk(chunk)? != VerifiedReadbackSourceRoleV1::Primary {
                anyhow::bail!("target package chunk was supplied by the game fallback");
            }
            target_exports +=
                usize::from(chunk_id.get_chunk_type() == EIoChunkType::ExportBundleData);
        }
    }
    if target_exports != 1 {
        anyhow::bail!("verified readback did not consume exactly one primary target export");
    }
    Ok(())
}

fn path_free_chunk_seal(
    chunk: &VerifiedChunkReceipt,
    sources: &ReadbackSourceAuthority,
) -> anyhow::Result<VerifiedReadbackChunkSealV1> {
    if !matches!(chunk.toc_hash_bytes, 20 | 32) {
        anyhow::bail!("internal IoStore receipt has an unsupported TOC hash length");
    }
    let mut toc_hash = [0u8; 32];
    let decoded_toc_hash = decode_hex(&chunk.toc_hash, "chunk TOC hash")?;
    if decoded_toc_hash.len() != chunk.toc_hash_bytes {
        anyhow::bail!("internal IoStore receipt TOC hash length mismatch");
    }
    toc_hash[..decoded_toc_hash.len()].copy_from_slice(&decoded_toc_hash);
    Ok(VerifiedReadbackChunkSealV1 {
        source_role: sources.role_for_chunk(chunk)?,
        source_utoc_blake3: decode_fixed_hex(
            &chunk.source_utoc_blake3,
            "chunk source UTOC BLAKE3",
        )?,
        chunk_id: decode_fixed_hex(&chunk.chunk_id, "chunk id")?,
        chunk_type: chunk.chunk_type.clone(),
        length: chunk.length,
        blake3: decode_fixed_hex(&chunk.blake3, "chunk BLAKE3")?,
        toc_hash,
        toc_hash_bytes: chunk.toc_hash_bytes,
    })
}

fn decode_hex(encoded: &str, label: &str) -> anyhow::Result<Vec<u8>> {
    fn nibble(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }

    if !encoded.len().is_multiple_of(2) {
        anyhow::bail!("{label} has an odd hexadecimal length");
    }
    let mut decoded = Vec::with_capacity(encoded.len() / 2);
    for pair in encoded.as_bytes().chunks_exact(2) {
        let high = nibble(pair[0]).ok_or_else(|| anyhow::anyhow!("{label} is not hexadecimal"))?;
        let low = nibble(pair[1]).ok_or_else(|| anyhow::anyhow!("{label} is not hexadecimal"))?;
        decoded.push(high << 4 | low);
    }
    Ok(decoded)
}

fn decode_fixed_hex<const N: usize>(encoded: &str, label: &str) -> anyhow::Result<[u8; N]> {
    let decoded = decode_hex(encoded, label)?;
    decoded
        .try_into()
        .map_err(|_| anyhow::anyhow!("{label} has the wrong byte length"))
}

fn verified_package_snapshot<'a>(
    store: &'a dyn iostore::IoStoreTrait,
    asset_path: &str,
) -> Result<(VerifiedSnapshotStore<'a>, FPackageId)> {
    let snapshot = VerifiedSnapshotStore::new(store);
    snapshot.prime_container_metadata()?;

    let container_version = snapshot
        .container_file_version()
        .ok_or_else(|| anyhow::anyhow!("container has no TOC version"))?;
    let header_version = snapshot
        .container_header_version()
        .ok_or_else(|| anyhow::anyhow!("container has no header version"))?;

    // UE package IDs are the lower-cased UTF-16 CityHash of the virtual package
    // path. The complete deserialized package name is still compared below so a
    // missing package or hash collision fails closed before conversion.
    let package_id = package_id_from_asset_path(asset_path);
    let chunk_id = FIoChunkId::from_package_id(package_id, 0, EIoChunkType::ExportBundleData);
    if !snapshot.has_chunk_id(chunk_id) {
        return Err(TexError::AssetNotFound(asset_path.into()));
    }
    let verified_name = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        || -> anyhow::Result<String> {
            let data = snapshot.read(chunk_id)?;
            let header = FZenPackageHeader::deserialize(
                &mut Cursor::new(&data),
                snapshot.package_store_entry(package_id),
                container_version,
                header_version,
                None,
            )?;
            Ok(header.package_name())
        },
    ))
    .map_err(|_| anyhow::anyhow!("resolved package header panicked while validating its name"))??;
    if verified_name != asset_path {
        return Err(TexError::AssetNotFound(asset_path.into()));
    }
    Ok((snapshot, package_id))
}

/// Bind one virtual package to the concrete winning package/bulk chunks and all
/// ContainerHeader chunks used by the current composite store. Unlike
/// [`unpack_asset_verified`], this performs no filesystem output and does not
/// deserialize global script objects; callers can therefore compare generation
/// provenance before creating an output staging directory.
pub fn probe_asset_generation_verified(
    utoc: &Path,
    asset_path: &str,
) -> Result<VerifiedAssetGeneration> {
    probe_asset_generation_inner(utoc, asset_path, None)
}

/// As [`probe_asset_generation_verified`], but unions an allowed prior dependency set with every
/// current target export/bulk/optional/memory-mapped chunk. Required IDs can reproduce imported
/// package headers consumed by legacy conversion, but can never narrow away a live target chunk.
pub fn probe_asset_generation_for_chunks_verified(
    utoc: &Path,
    asset_path: &str,
    required_chunk_ids: &[String],
) -> Result<VerifiedAssetGeneration> {
    probe_asset_generation_inner(utoc, asset_path, Some(required_chunk_ids))
}

fn probe_asset_generation_inner(
    utoc: &Path,
    asset_path: &str,
    required_chunk_ids: Option<&[String]>,
) -> Result<VerifiedAssetGeneration> {
    let store_path = utoc.parent().unwrap_or(utoc);
    let store = iostore::open(store_path, Arc::new(Config::default()))?;
    let snapshot = VerifiedSnapshotStore::new(store.as_ref());
    snapshot.prime_container_metadata()?;

    let container_version = snapshot
        .container_file_version()
        .ok_or_else(|| anyhow::anyhow!("container has no TOC version"))?;
    let header_version = snapshot
        .container_header_version()
        .ok_or_else(|| anyhow::anyhow!("container has no header version"))?;
    let package_id = package_id_from_asset_path(asset_path);
    let export_chunk = FIoChunkId::from_package_id(package_id, 0, EIoChunkType::ExportBundleData);
    if !snapshot.has_chunk_id(export_chunk) {
        return Err(TexError::AssetNotFound(asset_path.into()));
    }

    let verified_name = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        || -> anyhow::Result<String> {
            let data = snapshot.read(export_chunk)?;
            let header = FZenPackageHeader::deserialize(
                &mut Cursor::new(&data),
                snapshot.package_store_entry(package_id),
                container_version,
                header_version,
                None,
            )?;
            Ok(header.package_name())
        },
    ))
    .map_err(|_| anyhow::anyhow!("resolved package header panicked while probing generation"))??;
    if verified_name != asset_path {
        return Err(TexError::AssetNotFound(asset_path.into()));
    }

    let package_chunks = select_generation_probe_chunks(
        snapshot.chunks().map(|chunk| chunk.id()),
        package_id,
        container_version,
        required_chunk_ids,
    )?;
    for chunk_id in package_chunks {
        snapshot.read_verified(chunk_id)?;
    }

    let opened_utocs = snapshot.metadata_utoc_receipts()?;
    let metadata_utocs = opened_utocs
        .iter()
        .map(|receipt| receipt.source_utoc.clone())
        .collect();
    Ok(VerifiedAssetGeneration {
        consumed_chunks: snapshot.receipts()?,
        metadata_utocs,
        opened_utocs,
    })
}

/// Select the complete live target package surface plus any prior conversion dependencies.
///
/// A caller-provided required set may widen the probe to dependency package chunks, but can never
/// narrow away a currently present target bulk/optional/memory-mapped sidecar. ContainerHeader
/// chunks are already read by `prime_container_metadata` and are therefore omitted here.
fn select_generation_probe_chunks(
    live_chunks: impl IntoIterator<Item = FIoChunkId>,
    package_id: FPackageId,
    container_version: EIoStoreTocVersion,
    required_chunk_ids: Option<&[String]>,
) -> anyhow::Result<Vec<FIoChunkId>> {
    let mut selected = std::collections::BTreeMap::<[u8; 12], FIoChunkId>::new();
    for id in live_chunks.into_iter().filter(|id| {
        id.get_package_id() == package_id
            && matches!(
                id.get_chunk_type(),
                EIoChunkType::ExportBundleData
                    | EIoChunkType::BulkData
                    | EIoChunkType::OptionalBulkData
                    | EIoChunkType::MemoryMappedBulkData
            )
    }) {
        selected.insert(id.get_raw().id, id);
    }
    if let Some(required) = required_chunk_ids {
        for encoded in required {
            let raw = encoded
                .parse::<FIoChunkIdRaw>()
                .with_context(|| format!("invalid provenance chunk id {encoded:?}"))?;
            let id = FIoChunkId::from_raw(raw, container_version);
            match id.get_chunk_type() {
                EIoChunkType::ContainerHeader => {}
                EIoChunkType::ExportBundleData
                | EIoChunkType::BulkData
                | EIoChunkType::OptionalBulkData
                | EIoChunkType::MemoryMappedBulkData => {
                    selected.insert(id.get_raw().id, id);
                }
                other => {
                    anyhow::bail!("unsupported provenance chunk type {other:?} for {encoded}")
                }
            }
        }
    }
    Ok(selected.into_values().collect())
}

fn package_id_from_asset_path(asset_path: &str) -> FPackageId {
    FPackageId(FIoContainerId::from_name(asset_path).0)
}

/// Return whether a canonical 12-byte raw IoStore chunk id belongs to the
/// package identified by `asset_path`. The package id is the little-endian
/// first eight bytes; this comparison is independent of TOC chunk-type version.
pub fn chunk_id_matches_asset_path(chunk_id: &str, asset_path: &str) -> bool {
    fn nibble(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }

    if chunk_id.len() != 24 || !chunk_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return false;
    }
    let expected = package_id_from_asset_path(asset_path).0.to_le_bytes();
    chunk_id.as_bytes()[..16]
        .chunks_exact(2)
        .zip(expected)
        .all(|(pair, expected)| {
            let high = nibble(pair[0]);
            let low = nibble(pair[1]);
            high.zip(low)
                .is_some_and(|(high, low)| (high << 4 | low) == expected)
        })
}

/// Like `unpack_asset` but takes an already-known package id (from the texture
/// index) and caller-supplied output leaf directly. This bypasses
/// `unpack_asset`'s exact virtual-path/header-name verification, so public callers
/// should prefer that API unless they own a sealed index. Opens the parent Paks
/// directory so global script objects resolve.
pub fn unpack_asset_by_id(
    utoc: &Path,
    _usmap: &Path,
    package_id: u64,
    leaf: &str,
    out_dir: &Path,
) -> Result<PathBuf> {
    let store_path = utoc.parent().unwrap_or(utoc);
    let store = iostore::open(store_path, Arc::new(Config::default()))?;
    std::fs::create_dir_all(out_dir)?;
    legacy_from_package(store.as_ref(), FPackageId(package_id), leaf, out_dir)
}

/// Shared zen->legacy conversion tail. Given an already-resolved `FPackageId` and an
/// open (composite) store, builds the legacy cooked `.uasset`/`.uexp`/`.ubulk` into
/// `out_dir` (named after `leaf`) and returns the `.uasset` path.
///
/// `build_legacy` writes paths *relative* to the FSFileWriter's root dir, so we name
/// the output after `leaf` and root the writer at `out_dir`: the
/// .uasset/.uexp/.ubulk land directly in out_dir sharing the same stem (so
/// `with_extension(..)` finds siblings).
fn legacy_from_package(
    store: &dyn iostore::IoStoreTrait,
    package_id: FPackageId,
    leaf: &str,
    out_dir: &Path,
) -> Result<PathBuf> {
    let out_rel = format!("{leaf}.uasset");

    let log = Log::no_log();
    // No Verse script-cell store: the ordinary cooked assets handled here do not
    // carry Verse cell imports; script objects still resolve through global.utoc.
    let context = FZenPackageContext::create(store, None, &log, None);
    let writer = FSFileWriter::new(out_dir);

    build_legacy(&context, package_id, UEPath::new(&out_rel), &writer)?;

    let uasset = out_dir.join(format!("{leaf}.uasset"));
    Ok(uasset)
}

#[derive(Debug, Clone, Copy)]
struct LegacyMemoryLimits {
    max_uasset_bytes: usize,
    max_uexp_bytes: usize,
    max_pair_bytes: usize,
    max_sidecar_bytes: usize,
    max_total_bytes: usize,
}

impl Default for LegacyMemoryLimits {
    fn default() -> Self {
        Self {
            max_uasset_bytes: MAX_LEGACY_MEMORY_UASSET_BYTES,
            max_uexp_bytes: MAX_LEGACY_MEMORY_UEXP_BYTES,
            max_pair_bytes: MAX_LEGACY_MEMORY_PACKAGE_PAIR_BYTES,
            max_sidecar_bytes: MAX_LEGACY_MEMORY_SIDECAR_BYTES,
            max_total_bytes: MAX_LEGACY_MEMORY_TOTAL_BYTES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LegacyMemoryComponent {
    Uasset,
    Uexp,
    Bulk,
    Optional,
    MemoryMapped,
}

impl LegacyMemoryComponent {
    fn sidecar_kind(self) -> Option<VerifiedLegacySidecarKind> {
        match self {
            Self::Uasset | Self::Uexp => None,
            Self::Bulk => Some(VerifiedLegacySidecarKind::Bulk),
            Self::Optional => Some(VerifiedLegacySidecarKind::Optional),
            Self::MemoryMapped => Some(VerifiedLegacySidecarKind::MemoryMapped),
        }
    }
}

#[derive(Default)]
struct LegacyMemoryWriterState {
    files: std::collections::BTreeMap<LegacyMemoryComponent, Vec<u8>>,
    total_bytes: usize,
}

struct BoundedLegacyMemoryWriter {
    leaf: String,
    limits: LegacyMemoryLimits,
    state: Mutex<LegacyMemoryWriterState>,
}

struct LegacyMemoryOutput {
    uasset: Vec<u8>,
    uexp: Vec<u8>,
    sidecars: Vec<VerifiedLegacySidecarBytes>,
}

impl BoundedLegacyMemoryWriter {
    fn new(leaf: &str, limits: LegacyMemoryLimits) -> anyhow::Result<Self> {
        if leaf.is_empty()
            || leaf.len() > 255
            || !leaf
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            anyhow::bail!("legacy package output leaf is noncanonical");
        }
        if limits.max_uasset_bytes == 0
            || limits.max_uexp_bytes == 0
            || limits.max_pair_bytes == 0
            || limits.max_sidecar_bytes == 0
            || limits.max_total_bytes == 0
            || limits.max_pair_bytes
                > limits
                    .max_uasset_bytes
                    .checked_add(limits.max_uexp_bytes)
                    .ok_or_else(|| anyhow::anyhow!("legacy package memory limits overflowed"))?
            || limits.max_pair_bytes > limits.max_total_bytes
            || limits.max_sidecar_bytes > limits.max_total_bytes
        {
            anyhow::bail!("legacy package memory limits are invalid");
        }
        Ok(Self {
            leaf: leaf.to_owned(),
            limits,
            state: Mutex::new(LegacyMemoryWriterState::default()),
        })
    }

    fn component_for_path(&self, path: &str) -> Option<LegacyMemoryComponent> {
        if path == format!("{}.uasset", self.leaf) {
            Some(LegacyMemoryComponent::Uasset)
        } else if path == format!("{}.uexp", self.leaf) {
            Some(LegacyMemoryComponent::Uexp)
        } else if path == format!("{}.ubulk", self.leaf) {
            Some(LegacyMemoryComponent::Bulk)
        } else if path == format!("{}.uptnl", self.leaf) {
            Some(LegacyMemoryComponent::Optional)
        } else if path == format!("{}.m.ubulk", self.leaf) {
            Some(LegacyMemoryComponent::MemoryMapped)
        } else {
            None
        }
    }

    fn finish(self) -> anyhow::Result<LegacyMemoryOutput> {
        let mut state = self
            .state
            .into_inner()
            .map_err(|_| anyhow::anyhow!("legacy package memory writer was poisoned"))?;
        let uasset = state
            .files
            .remove(&LegacyMemoryComponent::Uasset)
            .filter(|bytes| !bytes.is_empty())
            .ok_or_else(|| anyhow::anyhow!("legacy conversion emitted no non-empty uasset"))?;
        let uexp = state
            .files
            .remove(&LegacyMemoryComponent::Uexp)
            .filter(|bytes| !bytes.is_empty())
            .ok_or_else(|| anyhow::anyhow!("legacy conversion emitted no non-empty uexp"))?;
        let pair_bytes = uasset
            .len()
            .checked_add(uexp.len())
            .ok_or_else(|| anyhow::anyhow!("legacy package pair length overflowed"))?;
        if pair_bytes > self.limits.max_pair_bytes {
            anyhow::bail!("legacy package pair exceeds its memory budget");
        }

        let mut sidecars = Vec::with_capacity(state.files.len());
        for (component, bytes) in state.files {
            let kind = component.sidecar_kind().ok_or_else(|| {
                anyhow::anyhow!("legacy conversion emitted duplicate core output")
            })?;
            sidecars.push(VerifiedLegacySidecarBytes { kind, bytes });
        }
        Ok(LegacyMemoryOutput {
            uasset,
            uexp,
            sidecars,
        })
    }
}

impl FileWriterTrait for BoundedLegacyMemoryWriter {
    fn write_file(&self, path: String, _allow_compress: bool, data: Vec<u8>) -> anyhow::Result<()> {
        let component = self.component_for_path(&path).ok_or_else(|| {
            anyhow::anyhow!("legacy conversion emitted an unexpected output name")
        })?;
        let component_limit = match component {
            LegacyMemoryComponent::Uasset => self.limits.max_uasset_bytes,
            LegacyMemoryComponent::Uexp => self.limits.max_uexp_bytes,
            LegacyMemoryComponent::Bulk
            | LegacyMemoryComponent::Optional
            | LegacyMemoryComponent::MemoryMapped => self.limits.max_sidecar_bytes,
        };
        if data.len() > component_limit {
            anyhow::bail!("legacy conversion output exceeds its component memory budget");
        }

        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("legacy package memory writer was poisoned"))?;
        if state.files.contains_key(&component) {
            anyhow::bail!("legacy conversion emitted a duplicate output component");
        }
        let next_total = state
            .total_bytes
            .checked_add(data.len())
            .ok_or_else(|| anyhow::anyhow!("legacy package aggregate length overflowed"))?;
        if next_total > self.limits.max_total_bytes {
            anyhow::bail!("legacy conversion output exceeds its aggregate memory budget");
        }
        let prospective_pair = match component {
            LegacyMemoryComponent::Uasset => data.len().checked_add(
                state
                    .files
                    .get(&LegacyMemoryComponent::Uexp)
                    .map_or(0, Vec::len),
            ),
            LegacyMemoryComponent::Uexp => data.len().checked_add(
                state
                    .files
                    .get(&LegacyMemoryComponent::Uasset)
                    .map_or(0, Vec::len),
            ),
            _ => Some(0),
        }
        .ok_or_else(|| anyhow::anyhow!("legacy package pair length overflowed"))?;
        if prospective_pair > self.limits.max_pair_bytes {
            anyhow::bail!("legacy package pair exceeds its memory budget");
        }
        state.files.insert(component, data);
        state.total_bytes = next_total;
        Ok(())
    }
}

fn legacy_from_package_to_memory(
    store: &dyn iostore::IoStoreTrait,
    package_id: FPackageId,
    leaf: &str,
) -> Result<LegacyMemoryOutput> {
    let out_rel = format!("{leaf}.uasset");
    let log = Log::no_log();
    let context = FZenPackageContext::create(store, None, &log, None);
    let writer = BoundedLegacyMemoryWriter::new(leaf, LegacyMemoryLimits::default())?;
    let build_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        build_legacy(&context, package_id, UEPath::new(&out_rel), &writer)
    }))
    .map_err(|_| anyhow::anyhow!("legacy package conversion panicked"))?;
    build_result?;
    Ok(writer.finish()?)
}

/// Pack a directory of edited legacy cooked files (laid out under their mount
/// path, e.g. `cooked_dir/G1R/Content/UI/Textures/Common/T_HardwareCursor.uasset`)
/// into a Zen triplet `out_dir/<name>.{utoc,ucas,pak}`, UE5.4. Returns the 3 paths
/// `[utoc, ucas, pak]`.
///
/// `compress` is opt-in. With `compress == false` (the default at every call
/// site) chunks are written UNCOMPRESSED (method 0, `container_flags = 8`) --
/// valid and game-loadable (UE mounts uncompressed IoStore fine), and proven to
/// work in-game. With `compress == true` the writer Oodle-compresses `.ucas`
/// blocks (16-aligned, `container_flags = Indexed|Compressed = 9`); the
/// compression code is wired and framing-fixed but the game currently ignores
/// our compressed containers (unresolved Oodle framing/encoder issue), so it is
/// off by default.
///
/// This re-implements retoc's `to-zen` orchestration (`action_to_zen`, which was
/// CLI-only) on top of the vendored lib's `pub` building blocks:
///   1. open the game's Paks *directory* as a composite source store so its
///      *global* script objects (in `global.utoc`) resolve -- exactly mirroring
///      `unpack_asset`; `build_zen_asset` needs them to resolve each package's
///      script imports;
///   2. for every `.uasset` (with a sibling `.uexp`) found under `cooked_dir`,
///      read the legacy cooked bytes into an `FSerializedAssetBundle`;
///   3. `build_zen_asset(...)` (UE5_4: `NoExportInfo` header, `OnDemandMetaData`
///      toc, `PropertyTagCompleteTypeName` pkg version) with mount point
///      `../../../` and asset path `../../../<relative-cooked-path>`;
///   4. `ConvertedZenAssetBundle::write` into the `IoStoreWriter`, then `finalize`
///      (which serialises the TOC + container-header chunk);
///   5. emit the empty `.pak` stub the game needs to detect/mount the container.
///
/// `game_dir` *is* required: a plain texture's package still references the
/// Texture2D script class, whose `FPackageObjectIndex` must resolve against the
/// global script-object table -- which lives in the game's `global.utoc`, not in
/// the cooked input. Passing `None` for `script_objects` produces a container the
/// game rejects (unresolved script imports). Verified: script objects ARE needed.
///
/// Security contract: `cooked_dir` and `out_dir` must be caller-owned directories
/// without concurrent writers. Both roots and cooked-tree entries reject
/// symlinks/reparse points; a missing output root is created one plain directory
/// component at a time. All component reads are bounded/no-follow. `name` is
/// restricted to a safe filename atom, and every triplet destination must be
/// absent; this function never intentionally replaces an existing output.
/// Multi-file publication is not atomic here, so untrusted/public CLI callers
/// should wrap it in a fresh private staging directory and atomically promote that
/// directory after verification.
pub fn repack_to_zen(
    cooked_dir: &Path,
    name: &str,
    out_dir: &Path,
    game_dir: &Path,
    compress: bool,
) -> Result<[PathBuf; 3]> {
    Ok(repack_to_zen_verified(cooked_dir, name, out_dir, game_dir, compress)?.triplet)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedRepackedTriplet {
    pub triplet: [PathBuf; 3],
    pub source_chunks: Vec<VerifiedChunkReceipt>,
    pub metadata_utocs: Vec<PathBuf>,
}

/// Source-snapshot-reporting form of [`repack_to_zen`]. Script-object chunks
/// are TOC-hash verified before conversion and returned with every source TOC
/// whose parsed metadata participated in composite-store resolution.
pub fn repack_to_zen_verified(
    cooked_dir: &Path,
    name: &str,
    out_dir: &Path,
    game_dir: &Path,
    compress: bool,
) -> Result<VerifiedRepackedTriplet> {
    use retoc::iostore_writer::IoStoreWriter;
    use retoc::legacy_asset::FSerializedAssetBundle;
    use retoc::version::EngineVersion;
    use retoc::zen_asset_conversion::build_zen_asset;
    use retoc::{build_verse_cell_store, UEPath, UEPathBuf};

    let ver = EngineVersion::UE5_4;
    let toc_version = ver.toc_version();
    let header_version = ver.container_header_version();
    let pkg_file_version = ver.package_file_version();
    let mount_point = UEPath::new("../../../");

    validate_repack_name(name)?;
    validate_plain_directory_root(cooked_dir, "cooked input")?;
    let planned = [
        out_dir.join(format!("{name}.utoc")),
        out_dir.join(format!("{name}.ucas")),
        out_dir.join(format!("{name}.pak")),
    ];
    for path in &planned {
        match std::fs::symlink_metadata(path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
            Ok(_) => {
                return Err(anyhow::anyhow!(
                    "triplet output already exists; refusing to replace {}",
                    path.display()
                )
                .into())
            }
        }
    }

    // 1. Open the game's Paks directory as a composite source store so the global
    //    script objects (in `global.utoc`) are available -- same rationale as
    //    `unpack_asset`. `build_zen_asset` resolves each package's script imports
    //    against these; without them the container's imports are unresolved and the
    //    game refuses to load it.
    let paks_dir = game_dir.join("G1R/Content/Paks");
    let store = iostore::open(&paks_dir, Arc::new(Config::default()))?;
    let source_snapshot = VerifiedSnapshotStore::new(store.as_ref());
    source_snapshot.prime_container_metadata()?;
    let script_objects = Some(Arc::new(source_snapshot.load_script_objects()?));

    // No Verse cells in plain cooked textures; an empty store mirrors the CLI's
    // `Some(script_cell_store)` arg (the CLI always passes a constructed store).
    let script_cells = Some(build_verse_cell_store(&Vec::new()));

    // 2. Collect every `.uasset` (with a sibling `.uexp`) under `cooked_dir`, as a
    //    path relative to `cooked_dir` (becomes the cooked/pak path inside the
    //    mount, e.g. `G1R/Content/UI/Textures/Common/T_HardwareCursor.uasset`).
    let mut asset_rels: Vec<PathBuf> = Vec::new();
    collect_uassets(cooked_dir, cooked_dir, 0, &mut asset_rels)?;
    if asset_rels.is_empty() {
        return Err(TexError::AssetNotFound(format!(
            "no .uasset (with sibling .uexp) found under {}",
            cooked_dir.display()
        )));
    }

    // 3-4. Open the writer and convert+write each asset.
    ensure_plain_directory_tree(out_dir, "triplet output")?;
    let utoc_path = planned[0].clone();
    let writer = IoStoreWriter::new(
        &utoc_path,
        toc_version,
        Some(header_version),
        UEPathBuf::from(mount_point),
    )?;
    // Compression is opt-in. Default (`compress == false`) writes raw blocks
    // (`container_flags = 8`) -- the proven-in-game uncompressed path. When
    // `compress == true` the writer Oodle-compresses blocks and the container is
    // flagged `Indexed|Compressed` (9).
    let mut writer = writer.set_compress(compress);

    let log = Log::no_log();
    for rel in &asset_rels {
        let abs = cooked_dir.join(rel);
        // The path handed to `build_zen_asset` is the mount-relative cooked path
        // (forward-slash, UE-style), prefixed with the `../../../` mount point.
        let rel_ue = path_to_ue(rel);
        let asset_ue_path = mount_point.join(&rel_ue);

        let bundle: FSerializedAssetBundle = read_legacy_bundle_bounded(&abs)?;

        let mut converted = build_zen_asset(
            bundle,
            &std::collections::HashMap::new(), // no referenced shader maps for a plain texture
            &asset_ue_path,
            Some(pkg_file_version),
            header_version,
            false, // allow_fixup: UE4-only external-arc fixup; false for UE5_4 (NoExportInfo)
            script_objects.clone(),
            script_cells.clone(),
            &log,
        )?;

        // NoExportInfo > Initial, so no import fix-up pass is needed: write directly.
        converted.write(&mut writer)?;
    }

    // 5. Serialise the TOC + container-header chunk.
    writer.finalize()?;

    // The game needs an (even empty) `.pak` sidecar to detect and mount the
    // IoStore container -- mirrors retoc's `action_to_zen`.
    let pak_path = planned[2].clone();
    {
        use std::io::BufWriter;
        let mut pak_file = BufWriter::new(std::fs::File::create(&pak_path)?);
        repak::PakBuilder::new()
            .writer(
                &mut pak_file,
                repak::Version::V11,
                mount_point.to_string(),
                None,
            )
            .write_index()
            .map_err(|e| anyhow::anyhow!("failed to write empty .pak index: {e}"))?;
    }

    Ok(VerifiedRepackedTriplet {
        triplet: [utoc_path, planned[1].clone(), pak_path],
        source_chunks: source_snapshot.receipts()?,
        metadata_utocs: source_snapshot.metadata_utocs()?,
    })
}

fn validate_repack_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 96
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        || is_windows_reserved_name(name)
    {
        return Err(anyhow::anyhow!(
            "container name must be a non-reserved 1..=96 ASCII filename atom using letters, digits, '_' or '-'"
        )
        .into());
    }
    Ok(())
}

fn is_windows_reserved_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || upper
            .strip_prefix("COM")
            .or_else(|| upper.strip_prefix("LPT"))
            .is_some_and(|suffix| suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'1'..=b'9'))
}

fn validate_plain_directory_root(path: &Path, label: &str) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        anyhow::anyhow!(
            "{label} directory {} is unavailable: {error}",
            path.display()
        )
    })?;
    if metadata_is_reparse(&metadata) || !metadata.is_dir() {
        return Err(anyhow::anyhow!(
            "{label} root must be a plain directory, not a symlink/reparse point: {}",
            path.display()
        )
        .into());
    }
    Ok(())
}

/// Create a directory tree without `create_dir_all` following an existing
/// symlink/junction component. Concurrent mutation is outside the public API's
/// caller-owned-directory contract; every component is nevertheless rechecked
/// immediately after creation or discovery.
fn ensure_plain_directory_tree(path: &Path, label: &str) -> Result<()> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut current = PathBuf::new();
    for component in absolute.components() {
        use std::path::Component;
        match component {
            // A Windows drive/UNC prefix is not a complete filesystem path until
            // its following RootDir component has been appended (probing `C:`
            // itself can fail with ERROR_INVALID_FUNCTION).
            Component::Prefix(_) => {
                current.push(component.as_os_str());
                continue;
            }
            Component::RootDir | Component::Normal(_) => {
                current.push(component.as_os_str());
            }
            Component::CurDir => continue,
            Component::ParentDir => {
                return Err(anyhow::anyhow!(
                    "{label} directory may not contain '..': {}",
                    path.display()
                )
                .into());
            }
        }
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata_is_reparse(&metadata) || !metadata.is_dir() {
                    return Err(anyhow::anyhow!(
                        "{label} path component must be a plain directory: {}",
                        current.display()
                    )
                    .into());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(&current)?;
                validate_plain_directory_root(&current, label)?;
            }
            Err(error) => return Err(error.into()),
        }
    }
    validate_plain_directory_root(&absolute, label)
}

/// The game's IoStore override folder: containers dropped here are mounted on top
/// of the base game (additive override; later-mounting wins).
fn mods_dir(game_dir: &Path) -> PathBuf {
    game_dir.join("G1R/Content/Paks/~mods")
}

/// On-disk record of a deployed container, written next to the triplet as
/// `<name>.gore-deploy.json`. Lists the absolute paths of every file we copied in
/// so `undeploy` can remove exactly what it added (and nothing else).
#[derive(serde::Serialize, serde::Deserialize)]
struct DeployRecord {
    name: String,
    files: Vec<PathBuf>,
}

/// Copy a Zen triplet (`[utoc, ucas, pak]`) into the game's `~mods` override folder
/// and write a JSON deploy record listing the copied file paths. Returns the path to
/// the record (`<mods>/<name>.gore-deploy.json`).
///
/// Non-destructive: this is an *additive* override -- the game mounts the `~mods`
/// container on top of the base game, so nothing in the base install is modified or
/// backed up. `undeploy` reverses it by deleting exactly the files this recorded.
pub fn deploy(triplet: &[PathBuf; 3], game_dir: &Path, name: &str) -> Result<PathBuf> {
    let mods = mods_dir(game_dir);
    std::fs::create_dir_all(&mods)?;
    // Canonicalize the mods dir so the deploy record holds ABSOLUTE paths even when
    // `game_dir` is relative (e.g. `--game .`). Otherwise a later `undeploy --game
    // <absolute>` run from a different cwd would resolve the recorded relative paths
    // against the wrong directory and fail to remove the mounted triplet. Falls back
    // to the un-canonicalized path if canonicalize fails (dir was just created, so it
    // should succeed).
    let mods = std::fs::canonicalize(&mods).unwrap_or(mods);

    // Crash-safety + rollback: write the deploy RECORD first, then copy the triplet
    // files. The record journals the intended destinations, so if the process is
    // killed or power is lost mid-copy, a record always exists for `undeploy` (or a
    // later redeploy) to remove the partial triplet — the copied .utoc/.ucas/.pak
    // never linger mounted with nothing to find them. Each on-disk mutation (the
    // record, then each triplet file) snapshots its PRIOR bytes before being
    // overwritten; on any RETURNED error `cleanup` restores those bytes (`Some`) or
    // removes a genuinely-new file (`None`), so a failed (re)deploy leaves the
    // previous state intact. An existing-but-unreadable file aborts before any
    // write rather than risk a rollback deleting it as if it were fresh.
    let mut written: Vec<(PathBuf, Option<Vec<u8>>)> = Vec::with_capacity(4);
    let cleanup = |written: &[(PathBuf, Option<Vec<u8>>)]| {
        for (f, prior) in written.iter().rev() {
            match prior {
                Some(bytes) => {
                    let _ = std::fs::write(f, bytes);
                }
                None => {
                    let _ = std::fs::remove_file(f);
                }
            }
        }
    };
    // Snapshot prior bytes of a path we're about to overwrite. NotFound -> None
    // (fresh add). Any other error -> abort (we can't safely roll it back).
    let snapshot = |path: &Path| -> Result<Option<Vec<u8>>> {
        match std::fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    };

    // Resolve destination paths up front — the record must list them before any
    // file is copied.
    let mut dsts: Vec<PathBuf> = Vec::with_capacity(3);
    for src in triplet {
        let leaf = src.file_name().ok_or_else(|| {
            TexError::AssetNotFound(format!("triplet path has no file name: {}", src.display()))
        })?;
        dsts.push(mods.join(leaf));
    }

    let record_path = mods.join(format!("{name}.gore-deploy.json"));
    let record = DeployRecord {
        name: name.to_string(),
        files: dsts.clone(),
    };
    let json = serde_json::to_string_pretty(&record)
        .map_err(|e| TexError::Retoc(anyhow::anyhow!("serialising deploy record: {e}")))?;

    // 1. Write the record FIRST (atomically: temp sibling + rename, so an existing
    //    record is never left truncated). Register its prior bytes for rollback.
    let prior_record = snapshot(&record_path)?;
    written.push((record_path.clone(), prior_record));
    let tmp_record = mods.join(format!("{name}.gore-deploy.json.tmp"));
    if let Err(e) = std::fs::write(&tmp_record, &json) {
        let _ = std::fs::remove_file(&tmp_record);
        cleanup(&written);
        return Err(e.into());
    }
    if let Err(e) = std::fs::rename(&tmp_record, &record_path) {
        let _ = std::fs::remove_file(&tmp_record);
        cleanup(&written);
        return Err(e.into());
    }

    // 2. Copy each triplet file, snapshotting its prior bytes before the copy
    //    (std::fs::copy creates/truncates the dst first, so a mid-copy failure
    //    leaves a partial file the rollback must restore or remove).
    for (src, dst) in triplet.iter().zip(dsts.iter()) {
        let prior = match snapshot(dst) {
            Ok(p) => p,
            Err(e) => {
                cleanup(&written);
                return Err(e);
            }
        };
        written.push((dst.clone(), prior));
        if let Err(e) = std::fs::copy(src, dst) {
            cleanup(&written);
            return Err(e.into());
        }
    }

    Ok(record_path)
}

/// Read `<mods>/<name>.gore-deploy.json` and delete every file it lists plus the
/// record itself. Individually-missing files are tolerated (reported to stderr) so a
/// partially-cleaned deploy can still be finished. Errors if the record is absent.
pub fn undeploy(game_dir: &Path, name: &str) -> Result<()> {
    let mods = mods_dir(game_dir);
    let record_path = mods.join(format!("{name}.gore-deploy.json"));
    if !record_path.exists() {
        return Err(TexError::DeployRecordNotFound(record_path));
    }

    let json = std::fs::read_to_string(&record_path)?;
    let record: DeployRecord = serde_json::from_str(&json).map_err(|e| {
        TexError::Retoc(anyhow::anyhow!(
            "parsing deploy record {}: {e}",
            record_path.display()
        ))
    })?;

    for f in &record.files {
        match std::fs::remove_file(f) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("warning: deployed file already gone: {}", f.display());
            }
            Err(e) => return Err(e.into()),
        }
    }

    std::fs::remove_file(&record_path)?;
    Ok(())
}

/// Recursively collect `.uasset` files (that have a sibling `.uexp`) under `dir`,
/// pushing each as a path relative to `root`.
fn collect_uassets(root: &Path, dir: &Path, depth: usize, out: &mut Vec<PathBuf>) -> Result<()> {
    if depth > MAX_REPACK_TREE_DEPTH {
        return Err(
            anyhow::anyhow!("cooked tree exceeds depth limit {MAX_REPACK_TREE_DEPTH}").into(),
        );
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)?;
        if metadata_is_reparse(&metadata) {
            return Err(anyhow::anyhow!(
                "symbolic-link or reparse entry refused in cooked tree: {}",
                path.display()
            )
            .into());
        }
        if metadata.is_dir() {
            collect_uassets(root, &path, depth + 1, out)?;
        } else if path.extension().is_some_and(|e| e == "uasset") {
            let uexp = path.with_extension("uexp");
            let uexp_metadata = std::fs::symlink_metadata(&uexp).map_err(|error| {
                anyhow::anyhow!(
                    "required sibling {} is unavailable: {error}",
                    uexp.display()
                )
            })?;
            if metadata_is_reparse(&uexp_metadata) || !uexp_metadata.is_file() {
                return Err(anyhow::anyhow!(
                    "required sibling is not a plain file: {}",
                    uexp.display()
                )
                .into());
            }
            if let Ok(rel) = path.strip_prefix(root) {
                out.push(rel.to_path_buf());
                if out.len() > MAX_REPACK_ASSETS {
                    return Err(anyhow::anyhow!(
                        "cooked tree exceeds asset count limit {MAX_REPACK_ASSETS}"
                    )
                    .into());
                }
            }
        } else if !metadata.is_file() {
            return Err(anyhow::anyhow!(
                "non-regular cooked tree entry refused: {}",
                path.display()
            )
            .into());
        }
    }
    Ok(())
}

fn read_legacy_bundle_bounded(
    uasset: &Path,
) -> Result<retoc::legacy_asset::FSerializedAssetBundle> {
    use retoc::legacy_asset::FSerializedAssetBundle;

    let paths = [
        uasset.to_path_buf(),
        uasset.with_extension("uexp"),
        uasset.with_extension("ubulk"),
        uasset.with_extension("uptnl"),
        with_double_ext(uasset, "m.ubulk"),
    ];
    let required = [true, true, false, false, false];
    let mut lengths = [None; 5];
    let mut total = 0u64;
    for (index, path) in paths.iter().enumerate() {
        match std::fs::symlink_metadata(path) {
            Ok(metadata) => {
                if metadata_is_reparse(&metadata) || !metadata.is_file() {
                    return Err(anyhow::anyhow!(
                        "legacy component is not a plain file: {}",
                        path.display()
                    )
                    .into());
                }
                let length = metadata.len();
                if length > MAX_REPACK_COMPONENT_BYTES {
                    return Err(anyhow::anyhow!(
                        "legacy component {} is {length} bytes; limit is {MAX_REPACK_COMPONENT_BYTES}",
                        path.display()
                    )
                    .into());
                }
                total = total
                    .checked_add(length)
                    .ok_or_else(|| anyhow::anyhow!("legacy bundle size overflowed"))?;
                if total > MAX_REPACK_BUNDLE_BYTES {
                    return Err(anyhow::anyhow!(
                        "legacy bundle is {total} bytes; limit is {MAX_REPACK_BUNDLE_BYTES}"
                    )
                    .into());
                }
                lengths[index] = Some(length);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && !required[index] => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(FSerializedAssetBundle {
        asset_file_buffer: read_component_exact(&paths[0], lengths[0].unwrap())?,
        exports_file_buffer: read_component_exact(&paths[1], lengths[1].unwrap())?,
        bulk_data_buffer: lengths[2]
            .map(|length| read_component_exact(&paths[2], length))
            .transpose()?,
        optional_bulk_data_buffer: lengths[3]
            .map(|length| read_component_exact(&paths[3], length))
            .transpose()?,
        memory_mapped_bulk_data_buffer: lengths[4]
            .map(|length| read_component_exact(&paths[4], length))
            .transpose()?,
    })
}

fn read_component_exact(path: &Path, expected_length: u64) -> Result<Vec<u8>> {
    let allocation = usize::try_from(expected_length)
        .map_err(|_| anyhow::anyhow!("component length does not fit memory"))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(allocation)
        .map_err(|error| anyhow::anyhow!("reserving component buffer failed: {error}"))?;
    let mut file = open_regular_no_follow(path)?;
    let opened_metadata = file.metadata()?;
    if metadata_is_reparse(&opened_metadata)
        || !opened_metadata.is_file()
        || opened_metadata.len() != expected_length
    {
        return Err(
            anyhow::anyhow!("legacy component changed before open: {}", path.display()).into(),
        );
    }
    (&mut file)
        .take(expected_length.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len())
        .map_err(|_| anyhow::anyhow!("component length does not fit u64"))?
        != expected_length
    {
        return Err(anyhow::anyhow!(
            "legacy component changed length while reading: {}",
            path.display()
        )
        .into());
    }
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata_is_reparse(&metadata) || !metadata.is_file() || metadata.len() != expected_length {
        return Err(
            anyhow::anyhow!("legacy component changed while reading: {}", path.display()).into(),
        );
    }
    Ok(bytes)
}

#[cfg(windows)]
fn open_regular_no_follow(path: &Path) -> Result<std::fs::File> {
    use std::os::windows::fs::OpenOptionsExt as _;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    Ok(std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?)
}

#[cfg(unix)]
fn open_regular_no_follow(path: &Path) -> Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt as _;
    Ok(std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?)
}

#[cfg(not(any(windows, unix)))]
fn open_regular_no_follow(path: &Path) -> Result<std::fs::File> {
    Ok(std::fs::File::open(path)?)
}

fn metadata_is_reparse(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Replace a path's final extension with a compound one (e.g. `T_X.uasset` ->
/// `T_X.m.ubulk`).
fn with_double_ext(path: &Path, compound_ext: &str) -> PathBuf {
    let stem = path
        .file_stem()
        .map(|s| s.to_os_string())
        .unwrap_or_default();
    let mut name = stem;
    name.push(".");
    name.push(compound_ext);
    path.with_file_name(name)
}

/// Convert an OS relative path to a forward-slash UE path string.
fn path_to_ue(rel: &Path) -> UEPathBuf {
    let s = rel.to_string_lossy().replace('\\', "/");
    UEPathBuf::from(s)
}

impl PartialOrd for TextureEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for TextureEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.asset_path.cmp(&other.asset_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;
    use std::path::PathBuf;

    fn raw_chunk_hex(id: FIoChunkId) -> String {
        id.get_raw().id.iter().fold(String::new(), |mut hex, byte| {
            write!(&mut hex, "{byte:02x}").unwrap();
            hex
        })
    }

    fn tiny_legacy_memory_limits() -> LegacyMemoryLimits {
        LegacyMemoryLimits {
            max_uasset_bytes: 4,
            max_uexp_bytes: 5,
            max_pair_bytes: 7,
            max_sidecar_bytes: 3,
            max_total_bytes: 10,
        }
    }

    #[test]
    fn bounded_legacy_memory_writer_accepts_only_the_exact_flat_component_set() {
        let writer =
            BoundedLegacyMemoryWriter::new("DA_Test", tiny_legacy_memory_limits()).unwrap();
        writer
            .write_file("DA_Test.uexp".to_owned(), false, vec![2; 4])
            .unwrap();
        writer
            .write_file("DA_Test.ubulk".to_owned(), false, vec![3; 3])
            .unwrap();
        writer
            .write_file("DA_Test.uasset".to_owned(), false, vec![1; 3])
            .unwrap();
        let output = writer.finish().unwrap();
        assert_eq!(output.uasset, vec![1; 3]);
        assert_eq!(output.uexp, vec![2; 4]);
        assert_eq!(
            output.sidecars,
            vec![VerifiedLegacySidecarBytes {
                kind: VerifiedLegacySidecarKind::Bulk,
                bytes: vec![3; 3],
            }]
        );

        for invalid in [
            "../DA_Test.uasset",
            "nested/DA_Test.uasset",
            "DA_Test.UASSET",
            "Other.uasset",
            "DA_Test.txt",
        ] {
            let writer =
                BoundedLegacyMemoryWriter::new("DA_Test", tiny_legacy_memory_limits()).unwrap();
            assert!(writer
                .write_file(invalid.to_owned(), false, vec![1])
                .is_err());
        }
        assert!(BoundedLegacyMemoryWriter::new("../bad", tiny_legacy_memory_limits()).is_err());
    }

    #[test]
    fn bounded_legacy_memory_writer_fails_closed_on_duplicate_and_every_budget() {
        let duplicate =
            BoundedLegacyMemoryWriter::new("DA_Test", tiny_legacy_memory_limits()).unwrap();
        duplicate
            .write_file("DA_Test.uasset".to_owned(), false, vec![1])
            .unwrap();
        assert!(duplicate
            .write_file("DA_Test.uasset".to_owned(), false, vec![1])
            .is_err());

        let component =
            BoundedLegacyMemoryWriter::new("DA_Test", tiny_legacy_memory_limits()).unwrap();
        assert!(component
            .write_file("DA_Test.uasset".to_owned(), false, vec![0; 5])
            .is_err());
        assert!(component
            .write_file("DA_Test.uexp".to_owned(), false, vec![0; 6])
            .is_err());

        let sidecar =
            BoundedLegacyMemoryWriter::new("DA_Test", tiny_legacy_memory_limits()).unwrap();
        assert!(sidecar
            .write_file("DA_Test.ubulk".to_owned(), false, vec![0; 4])
            .is_err());

        let pair = BoundedLegacyMemoryWriter::new("DA_Test", tiny_legacy_memory_limits()).unwrap();
        pair.write_file("DA_Test.uasset".to_owned(), false, vec![0; 4])
            .unwrap();
        assert!(pair
            .write_file("DA_Test.uexp".to_owned(), false, vec![0; 4])
            .is_err());

        let aggregate =
            BoundedLegacyMemoryWriter::new("DA_Test", tiny_legacy_memory_limits()).unwrap();
        aggregate
            .write_file("DA_Test.uasset".to_owned(), false, vec![0; 3])
            .unwrap();
        aggregate
            .write_file("DA_Test.uexp".to_owned(), false, vec![0; 4])
            .unwrap();
        aggregate
            .write_file("DA_Test.ubulk".to_owned(), false, vec![0; 3])
            .unwrap();
        assert!(aggregate
            .write_file("DA_Test.uptnl".to_owned(), false, vec![0; 1])
            .is_err());

        let missing =
            BoundedLegacyMemoryWriter::new("DA_Test", tiny_legacy_memory_limits()).unwrap();
        missing
            .write_file("DA_Test.uasset".to_owned(), false, vec![1])
            .unwrap();
        assert!(missing.finish().is_err());
    }

    #[test]
    fn primary_target_router_never_uses_fallback_target_chunks() {
        use retoc::iostore_writer::IoStoreWriter;
        use retoc::version::EngineVersion;

        let base = unique_tmp("primary-target-router");
        let primary_utoc = base.join("built").join("Built_P.utoc");
        let fallback_dir = base.join("game-paks");
        std::fs::create_dir_all(primary_utoc.parent().unwrap()).unwrap();
        std::fs::create_dir_all(&fallback_dir).unwrap();

        let version = EngineVersion::UE5_4;
        let target = FPackageId(0x0102_0304_0506_0708);
        let dependency = FPackageId(0x1112_1314_1516_1718);
        let target_export = FIoChunkId::from_package_id(target, 0, EIoChunkType::ExportBundleData);
        let target_optional =
            FIoChunkId::from_package_id(target, 0, EIoChunkType::OptionalBulkData);
        let dependency_export =
            FIoChunkId::from_package_id(dependency, 0, EIoChunkType::ExportBundleData);
        let script_objects = FIoChunkId::create(0, 0, EIoChunkType::ScriptObjects);

        let mut primary_writer = IoStoreWriter::new(
            &primary_utoc,
            version.toc_version(),
            Some(version.container_header_version()),
            UEPathBuf::from("../../../"),
        )
        .unwrap();
        primary_writer
            .write_package_chunk(
                target_export,
                Some(UEPath::new("../../../G1R/Content/Target.uasset")),
                b"primary-target",
                &StoreEntry::default(),
            )
            .unwrap();
        primary_writer.finalize().unwrap();

        let fallback_utoc = fallback_dir.join("global.utoc");
        let mut fallback_writer = IoStoreWriter::new(
            &fallback_utoc,
            version.toc_version(),
            Some(version.container_header_version()),
            UEPathBuf::from("../../../"),
        )
        .unwrap();
        fallback_writer
            .write_package_chunk(
                target_export,
                Some(UEPath::new("../../../G1R/Content/Target.uasset")),
                b"fallback-target",
                &StoreEntry::default(),
            )
            .unwrap();
        fallback_writer
            .write_chunk(target_optional, None, b"fallback-target-optional")
            .unwrap();
        fallback_writer
            .write_package_chunk(
                dependency_export,
                Some(UEPath::new("../../../G1R/Content/Dependency.uasset")),
                b"fallback-dependency",
                &StoreEntry::default(),
            )
            .unwrap();
        fallback_writer
            .write_chunk(script_objects, None, b"fallback-script-objects")
            .unwrap();
        fallback_writer.finalize().unwrap();

        let primary = iostore::open(&primary_utoc, Arc::new(Config::default())).unwrap();
        let fallback = iostore::open(&fallback_dir, Arc::new(Config::default())).unwrap();
        let routed =
            PrimaryTargetFallbackStore::new(primary.as_ref(), fallback.as_ref(), target).unwrap();

        assert_eq!(routed.read(target_export).unwrap(), b"primary-target");
        assert!(
            !routed.has_chunk_id(target_optional),
            "fallback must not make a missing target sidecar appear present"
        );
        assert!(routed.read(target_optional).is_err());
        assert_eq!(
            routed.read(dependency_export).unwrap(),
            b"fallback-dependency"
        );
        assert_eq!(
            routed.read(script_objects).unwrap(),
            b"fallback-script-objects"
        );

        let visible_target_chunks: Vec<_> = routed
            .chunks()
            .filter(|chunk| {
                chunk.id().get_package_id() == target
                    && matches!(
                        chunk.id().get_chunk_type(),
                        EIoChunkType::ExportBundleData
                            | EIoChunkType::BulkData
                            | EIoChunkType::OptionalBulkData
                            | EIoChunkType::MemoryMappedBulkData
                    )
            })
            .collect();
        assert_eq!(visible_target_chunks.len(), 1);
        assert_eq!(
            visible_target_chunks[0].id(),
            target_export.with_version(version.toc_version())
        );
        assert_eq!(visible_target_chunks[0].read().unwrap(), b"primary-target");

        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn path_free_readback_seals_reject_unknown_or_drifted_sources() {
        let primary_path = PathBuf::from("secret-primary.utoc");
        let fallback_path = PathBuf::from("secret-fallback.utoc");
        let primary_hash = "11".repeat(32);
        let fallback_hash = "22".repeat(32);
        let authority = ReadbackSourceAuthority::new(
            vec![VerifiedOpenedUtocReceipt {
                source_utoc: primary_path.clone(),
                source_utoc_blake3: primary_hash.clone(),
            }],
            vec![VerifiedOpenedUtocReceipt {
                source_utoc: fallback_path.clone(),
                source_utoc_blake3: fallback_hash.clone(),
            }],
        )
        .unwrap();

        let source_seals = authority.path_free_seals().unwrap();
        assert_eq!(source_seals.len(), 2);
        assert_eq!(
            source_seals[0].role(),
            VerifiedReadbackSourceRoleV1::Primary
        );
        assert_eq!(source_seals[0].utoc_blake3(), &[0x11; 32]);
        assert_eq!(
            source_seals[1].role(),
            VerifiedReadbackSourceRoleV1::Fallback
        );
        assert_eq!(source_seals[1].utoc_blake3(), &[0x22; 32]);

        let receipt = VerifiedChunkReceipt {
            chunk_id: "01".repeat(12),
            chunk_type: "ExportBundleData".to_owned(),
            source_utoc: primary_path,
            source_utoc_blake3: primary_hash,
            length: 3,
            blake3: "33".repeat(32),
            toc_hash: "44".repeat(20),
            toc_hash_bytes: 20,
        };
        let seal = path_free_chunk_seal(&receipt, &authority).unwrap();
        assert_eq!(seal.source_role(), VerifiedReadbackSourceRoleV1::Primary);
        assert_eq!(seal.chunk_id(), &[1; 12]);
        assert_eq!(seal.length(), 3);
        assert_eq!(seal.blake3(), &[0x33; 32]);
        assert_eq!(seal.toc_hash(), &[0x44; 20]);

        let mut drifted = receipt.clone();
        drifted.source_utoc_blake3 = "55".repeat(32);
        assert!(path_free_chunk_seal(&drifted, &authority).is_err());
        let mut unknown = receipt;
        unknown.source_utoc = PathBuf::from("unknown.utoc");
        assert!(path_free_chunk_seal(&unknown, &authority).is_err());
    }

    fn binding_chunk(
        id: u8,
        chunk_type: &str,
        length: u64,
        blake3: u8,
        toc_hash: u8,
        toc_hash_bytes: usize,
    ) -> (VerifiedChunkReceipt, VerifiedReadbackChunkSealV1) {
        let mut sealed_toc_hash = [0; 32];
        sealed_toc_hash[..toc_hash_bytes].fill(toc_hash);
        (
            VerifiedChunkReceipt {
                chunk_id: format!("{id:02x}").repeat(12),
                chunk_type: chunk_type.to_owned(),
                source_utoc: PathBuf::from("private-primary.utoc"),
                source_utoc_blake3: "11".repeat(32),
                length,
                blake3: format!("{blake3:02x}").repeat(32),
                toc_hash: format!("{toc_hash:02x}").repeat(toc_hash_bytes),
                toc_hash_bytes,
            },
            VerifiedReadbackChunkSealV1 {
                source_role: VerifiedReadbackSourceRoleV1::Primary,
                source_utoc_blake3: [0x11; 32],
                chunk_id: [id; 12],
                chunk_type: chunk_type.to_owned(),
                length,
                blake3: [blake3; 32],
                toc_hash: sealed_toc_hash,
                toc_hash_bytes,
            },
        )
    }

    fn matching_primary_binding() -> (StrictTripletVerification, VerifiedPrimaryAssetReadbackV1) {
        let (export_receipt, export_seal) = binding_chunk(2, "ExportBundleData", 3, 0x22, 0x33, 20);
        let (header_receipt, header_seal) = binding_chunk(1, "ContainerHeader", 4, 0x44, 0x55, 32);
        (
            StrictTripletVerification {
                package: "/Game/Test/DA_Binding".to_owned(),
                export_path: "../../../G1R/Content/Test/DA_Binding.uasset".to_owned(),
                chunk_count: 2,
                chunks: vec![export_receipt, header_receipt],
                pak_mount_point: "../../../".to_owned(),
                pak_files: Vec::new(),
                bulk_chunks: 0,
                optional_bulk_chunks: 0,
                memory_mapped_bulk_chunks: 0,
            },
            VerifiedPrimaryAssetReadbackV1 {
                asset_path: "/Game/Test/DA_Binding".to_owned(),
                uasset: vec![1, 2, 3],
                uexp: vec![4, 5],
                sidecars: vec![VerifiedReadbackSidecarV1 {
                    kind: VerifiedReadbackSidecarKindV1::Bulk,
                    bytes: vec![6, 7],
                }],
                source_seals: vec![
                    VerifiedReadbackSourceSealV1 {
                        role: VerifiedReadbackSourceRoleV1::Primary,
                        utoc_blake3: [0x11; 32],
                    },
                    VerifiedReadbackSourceSealV1 {
                        role: VerifiedReadbackSourceRoleV1::Fallback,
                        utoc_blake3: [0x77; 32],
                    },
                ],
                chunk_seals: vec![
                    export_seal,
                    header_seal,
                    VerifiedReadbackChunkSealV1 {
                        source_role: VerifiedReadbackSourceRoleV1::Fallback,
                        source_utoc_blake3: [0x77; 32],
                        chunk_id: [9; 12],
                        chunk_type: "ScriptObjects".to_owned(),
                        length: 1,
                        blake3: [0x88; 32],
                        toc_hash: [0x99; 32],
                        toc_hash_bytes: 20,
                    },
                ],
            },
        )
    }

    fn assert_primary_binding_rejected(
        mutate: impl FnOnce(&mut StrictTripletVerification, &mut VerifiedPrimaryAssetReadbackV1),
    ) {
        let (mut strict, mut readback) = matching_primary_binding();
        mutate(&mut strict, &mut readback);
        assert!(strict
            .verify_primary_readback_binding_v1(&readback)
            .is_err());
    }

    #[test]
    fn strict_triplet_binding_accepts_only_the_exact_primary_chunk_set() {
        let (strict, readback) = matching_primary_binding();
        strict
            .verify_primary_readback_binding_v1(&readback)
            .unwrap();
        assert_eq!(strict.package(), "/Game/Test/DA_Binding");
        assert_eq!(
            strict.export_path(),
            "../../../G1R/Content/Test/DA_Binding.uasset"
        );
        assert_eq!(strict.chunk_count(), 2);
        assert_eq!(strict.chunks().len(), 2);
        assert_eq!(strict.pak_mount_point(), "../../../");
        assert!(strict.pak_files().is_empty());
        assert_eq!(strict.bulk_chunks(), 0);
        assert_eq!(strict.optional_bulk_chunks(), 0);
        assert_eq!(strict.memory_mapped_bulk_chunks(), 0);
    }

    #[test]
    fn strict_triplet_binding_rejects_every_authority_or_chunk_drift() {
        assert_primary_binding_rejected(|strict, _| strict.package.push_str("_Other"));
        assert_primary_binding_rejected(|_, readback| readback.source_seals.clear());
        assert_primary_binding_rejected(|_, readback| {
            readback.source_seals.push(VerifiedReadbackSourceSealV1 {
                role: VerifiedReadbackSourceRoleV1::Primary,
                utoc_blake3: [0x11; 32],
            });
        });
        assert_primary_binding_rejected(|strict, _| {
            strict.chunks[0].source_utoc_blake3 = "12".repeat(32);
        });
        assert_primary_binding_rejected(|strict, _| strict.chunk_count += 1);
        assert_primary_binding_rejected(|_, readback| readback.chunk_seals[0].chunk_id = [3; 12]);
        assert_primary_binding_rejected(|_, readback| {
            readback.chunk_seals[0].chunk_type = "BulkData".to_owned();
        });
        assert_primary_binding_rejected(|_, readback| readback.chunk_seals[0].length += 1);
        assert_primary_binding_rejected(|_, readback| readback.chunk_seals[0].blake3[0] ^= 1);
        assert_primary_binding_rejected(|_, readback| readback.chunk_seals[0].toc_hash[0] ^= 1);
        assert_primary_binding_rejected(|_, readback| {
            readback.chunk_seals[0].toc_hash_bytes -= 1;
        });
        assert_primary_binding_rejected(|_, readback| {
            readback.chunk_seals[0].source_utoc_blake3[0] ^= 1;
        });
        assert_primary_binding_rejected(|_, readback| {
            let mut fallback_target = readback.chunk_seals[0].clone();
            fallback_target.source_role = VerifiedReadbackSourceRoleV1::Fallback;
            fallback_target.source_utoc_blake3 = [0x77; 32];
            readback.chunk_seals.push(fallback_target);
        });
    }

    #[test]
    fn primary_readback_into_parts_moves_every_owned_allocation() {
        let (_, readback) = matching_primary_binding();
        let asset_path_ptr = readback.asset_path.as_ptr();
        let uasset_ptr = readback.uasset.as_ptr();
        let uexp_ptr = readback.uexp.as_ptr();
        let sidecars_ptr = readback.sidecars.as_ptr();
        let source_seals_ptr = readback.source_seals.as_ptr();
        let chunk_seals_ptr = readback.chunk_seals.as_ptr();

        let (asset_path, uasset, uexp, sidecars, source_seals, chunk_seals) = readback.into_parts();
        assert_eq!(asset_path.as_ptr(), asset_path_ptr);
        assert_eq!(uasset.as_ptr(), uasset_ptr);
        assert_eq!(uexp.as_ptr(), uexp_ptr);
        assert_eq!(sidecars.as_ptr(), sidecars_ptr);
        assert_eq!(source_seals.as_ptr(), source_seals_ptr);
        assert_eq!(chunk_seals.as_ptr(), chunk_seals_ptr);
        assert_eq!(asset_path, "/Game/Test/DA_Binding");
        assert_eq!(uasset, [1, 2, 3]);
        assert_eq!(uexp, [4, 5]);
        assert_eq!(sidecars[0].bytes(), [6, 7]);
    }

    #[test]
    fn generation_probe_required_chunks_cannot_hide_live_target_optional_bulk() {
        let version = EIoStoreTocVersion::ReplaceIoChunkHashWithIoHash;
        let target = FPackageId(0x0102_0304_0506_0708);
        let dependency = FPackageId(0x1112_1314_1516_1718);
        let target_export = FIoChunkId::from_package_id(target, 0, EIoChunkType::ExportBundleData)
            .with_version(version);
        let target_optional =
            FIoChunkId::from_package_id(target, 0, EIoChunkType::OptionalBulkData)
                .with_version(version);
        let dependency_export =
            FIoChunkId::from_package_id(dependency, 0, EIoChunkType::ExportBundleData)
                .with_version(version);
        let metadata = FIoChunkId::from_package_id(target, 0, EIoChunkType::ContainerHeader)
            .with_version(version);
        let required = vec![
            raw_chunk_hex(target_export),
            raw_chunk_hex(dependency_export),
            raw_chunk_hex(metadata),
        ];

        let selected = select_generation_probe_chunks(
            [target_export, target_optional, dependency_export],
            target,
            version,
            Some(&required),
        )
        .unwrap();
        let selected: std::collections::BTreeSet<_> =
            selected.into_iter().map(|id| id.get_raw().id).collect();

        assert!(selected.contains(&target_export.get_raw().id));
        assert!(selected.contains(&target_optional.get_raw().id));
        assert!(selected.contains(&dependency_export.get_raw().id));
        assert!(!selected.contains(&metadata.get_raw().id));
        assert_eq!(selected.len(), 3);
    }

    fn minimal_raw_toc() -> Vec<u8> {
        let mut bytes = vec![0u8; 0x90];
        bytes[..16].copy_from_slice(b"-==--==--==--==-");
        bytes[16] = 6;
        bytes[20..24].copy_from_slice(&0x90u32.to_le_bytes());
        bytes[44..48].copy_from_slice(&0x1_0000u32.to_le_bytes());
        bytes[52..56].copy_from_slice(&1u32.to_le_bytes());
        bytes[88..96].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes
    }

    #[test]
    fn malicious_utoc_counts_fail_without_panic_or_large_allocation() {
        let base = unique_tmp("malicious-utoc-count");
        let mut huge_entries = minimal_raw_toc();
        huge_entries[24..28].copy_from_slice(&u32::MAX.to_le_bytes());
        let mut bad_fixed_header = minimal_raw_toc();
        bad_fixed_header[20..24].copy_from_slice(&u32::MAX.to_le_bytes());
        let mut huge_directory_count = minimal_raw_toc();
        huge_directory_count[48..52].copy_from_slice(&8u32.to_le_bytes());
        huge_directory_count.extend_from_slice(&0i32.to_le_bytes());
        huge_directory_count.extend_from_slice(&u32::MAX.to_le_bytes());
        let mut invalid_chunk_type = minimal_raw_toc();
        invalid_chunk_type[24..28].copy_from_slice(&1u32.to_le_bytes());
        let mut chunk_id = [0u8; 12];
        chunk_id[11] = 0xff;
        invalid_chunk_type.extend_from_slice(&chunk_id);
        invalid_chunk_type.extend_from_slice(&[0u8; 10]);
        invalid_chunk_type.extend_from_slice(&[0u8; 33]);

        for (index, bytes) in [
            huge_entries,
            bad_fixed_header,
            huge_directory_count,
            invalid_chunk_type,
        ]
        .into_iter()
        .enumerate()
        {
            let utoc = base.join(format!("bad-{index}.utoc"));
            std::fs::write(&utoc, bytes).unwrap();
            std::fs::write(utoc.with_extension("ucas"), []).unwrap();
            let result =
                std::panic::catch_unwind(|| iostore::open(&utoc, Arc::new(Config::default())));
            assert!(result.is_ok(), "bounded IoStore open must not panic");
            assert!(result.unwrap().is_err());
        }
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn malicious_container_header_count_fails_before_deserialization() {
        use retoc::iostore_writer::IoStoreWriter;
        use retoc::version::EngineVersion;

        let base = unique_tmp("malicious-container-header");
        let utoc = base.join("header.utoc");
        let version = EngineVersion::UE5_4;
        IoStoreWriter::new(
            &utoc,
            version.toc_version(),
            Some(version.container_header_version()),
            UEPathBuf::from("../../../"),
        )
        .unwrap()
        .finalize()
        .unwrap();
        let ucas = utoc.with_extension("ucas");
        let mut bytes = std::fs::read(&ucas).unwrap();
        assert!(bytes.len() >= 20);
        bytes[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        std::fs::write(&ucas, bytes).unwrap();

        let result = std::panic::catch_unwind(|| iostore::open(&utoc, Arc::new(Config::default())));
        assert!(
            result.is_ok(),
            "bounded ContainerHeader open must not panic"
        );
        assert!(result.unwrap().is_err());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn malicious_container_header_indirect_array_fails_before_allocation() {
        use retoc::iostore_writer::IoStoreWriter;
        use retoc::version::EngineVersion;

        let base = unique_tmp("malicious-container-indirect");
        let utoc = base.join("header.utoc");
        let version = EngineVersion::UE5_4;
        let mut writer = IoStoreWriter::new(
            &utoc,
            version.toc_version(),
            Some(version.container_header_version()),
            UEPathBuf::from("../../../"),
        )
        .unwrap();
        let package = FPackageId(0x1234);
        writer
            .write_package_chunk(
                FIoChunkId::from_package_id(package, 0, EIoChunkType::ExportBundleData),
                None,
                &[0],
                &StoreEntry::default(),
            )
            .unwrap();
        writer.finalize().unwrap();

        let ucas = utoc.with_extension("ucas");
        let mut bytes = std::fs::read(&ucas).unwrap();
        let magic = 0x496f_436eu32.to_le_bytes();
        let header_start = bytes
            .windows(magic.len())
            .position(|window| window == magic)
            .expect("writer emitted a ContainerHeader signature");
        let imported_count = header_start + 40;
        bytes[imported_count..imported_count + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        std::fs::write(&ucas, bytes).unwrap();

        let result = std::panic::catch_unwind(|| iostore::open(&utoc, Arc::new(Config::default())));
        assert!(result.is_ok(), "bounded indirect-array scan must not panic");
        assert!(result.unwrap().is_err());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn duplicate_sibling_container_header_ids_fail_closed() {
        use retoc::iostore_writer::IoStoreWriter;
        use retoc::version::EngineVersion;

        let base = unique_tmp("duplicate-container-header");
        let first = base.join("first.utoc");
        let version = EngineVersion::UE5_4;
        IoStoreWriter::new(
            &first,
            version.toc_version(),
            Some(version.container_header_version()),
            UEPathBuf::from("../../../"),
        )
        .unwrap()
        .finalize()
        .unwrap();
        let second = base.join("second.utoc");
        std::fs::copy(&first, &second).unwrap();
        std::fs::copy(first.with_extension("ucas"), second.with_extension("ucas")).unwrap();
        let error = match iostore::open(&base, Arc::new(Config::default())) {
            Ok(_) => panic!("duplicate sibling headers were unexpectedly accepted"),
            Err(error) => error,
        };
        assert!(
            error.to_string().contains("duplicate ContainerHeader"),
            "unexpected error: {error:#}"
        );
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn legal_zero_length_iostore_chunk_reads_empty_without_panic() {
        use retoc::iostore_writer::IoStoreWriter;
        use retoc::version::EngineVersion;

        let base = unique_tmp("zero-length-chunk");
        let utoc = base.join("empty.utoc");
        let version = EngineVersion::UE5_4;
        let package = FPackageId(0x1234_5678_9abc_def0);
        let chunk_id = FIoChunkId::from_package_id(package, 0, EIoChunkType::BulkData);
        let mut writer = IoStoreWriter::new(
            &utoc,
            version.toc_version(),
            Some(version.container_header_version()),
            UEPathBuf::from("../../../"),
        )
        .unwrap();
        writer.write_chunk(chunk_id, None, &[]).unwrap();
        writer.finalize().unwrap();

        let store = iostore::open(&utoc, Arc::new(Config::default())).unwrap();
        let info = store
            .chunks()
            .find(|chunk| chunk.id().get_chunk_type() == EIoChunkType::BulkData)
            .expect("writer emitted the empty bulk chunk");
        assert_eq!(info.size(), 0);
        let read = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| info.read()));
        assert!(read.is_ok(), "zero-length Toc::read must not panic");
        assert!(read.unwrap().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn method_zero_block_size_mismatch_fails_before_reader_consumption() {
        use retoc::iostore_writer::IoStoreWriter;
        use retoc::version::EngineVersion;

        let base = unique_tmp("method-zero-size-mismatch");
        let utoc = base.join("mismatch.utoc");
        let version = EngineVersion::UE5_4;
        let mut writer = IoStoreWriter::new(
            &utoc,
            version.toc_version(),
            Some(version.container_header_version()),
            UEPathBuf::from("../../../"),
        )
        .unwrap();
        writer
            .write_chunk(
                FIoChunkId::from_package_id(
                    FPackageId(0x1020_3040_5060_7080),
                    0,
                    EIoChunkType::BulkData,
                ),
                None,
                &[1, 2, 3, 4],
            )
            .unwrap();
        writer.finalize().unwrap();

        let mut toc = std::fs::read(&utoc).unwrap();
        let entry_count = u32::from_le_bytes(toc[24..28].try_into().unwrap()) as usize;
        let perfect_hash_count = u32::from_le_bytes(toc[84..88].try_into().unwrap()) as usize;
        let overflow_count = u32::from_le_bytes(toc[96..100].try_into().unwrap()) as usize;
        let first_block =
            0x90 + entry_count * (12 + 10) + (perfect_hash_count + overflow_count) * 4;
        assert_eq!(&toc[first_block + 5..first_block + 8], &[4, 0, 0]);
        assert_eq!(&toc[first_block + 8..first_block + 11], &[4, 0, 0]);
        toc[first_block + 5..first_block + 8].copy_from_slice(&[3, 0, 0]);
        std::fs::write(&utoc, toc).unwrap();

        let error = match iostore::open(&utoc, Arc::new(Config::default())) {
            Ok(_) => panic!("method-0 size mismatch was unexpectedly accepted"),
            Err(error) => error,
        };
        assert!(
            format!("{error:#}").contains("method-0"),
            "unexpected error: {error:#}"
        );
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn repack_name_rejects_traversal_and_windows_devices() {
        for invalid in ["", "../escape", "has.dot", "CON", "com1", "LPT9"] {
            assert!(
                validate_repack_name(invalid).is_err(),
                "unexpectedly accepted {invalid:?}"
            );
        }
        validate_repack_name("zzz_GoreWolfProof_P-2").unwrap();
    }

    #[test]
    fn output_directory_tree_is_created_plain_and_rejects_files() {
        let base = unique_tmp("plain-output-tree");
        let nested = base.join("one").join("two");
        ensure_plain_directory_tree(&nested, "test output").unwrap();
        validate_plain_directory_root(&nested, "test output").unwrap();

        let file = base.join("not-a-directory");
        std::fs::write(&file, b"sentinel").unwrap();
        assert!(ensure_plain_directory_tree(&file, "test output").is_err());
        assert_eq!(std::fs::read(&file).unwrap(), b"sentinel");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn oversized_legacy_component_fails_before_allocation() {
        let base = unique_tmp("oversized-component");
        let uasset = base.join("TooLarge.uasset");
        std::fs::File::create(&uasset)
            .unwrap()
            .set_len(MAX_REPACK_COMPONENT_BYTES + 1)
            .unwrap();
        let error = match read_legacy_bundle_bounded(&uasset) {
            Ok(_) => panic!("oversized component was unexpectedly accepted"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("limit"), "unexpected error: {error}");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[cfg(unix)]
    #[test]
    fn cooked_root_symlink_is_rejected() {
        use std::os::unix::fs::symlink;

        let base = unique_tmp("root-symlink");
        let target = base.join("target");
        std::fs::create_dir(&target).unwrap();
        let link = base.join("link");
        symlink(&target, &link).unwrap();
        assert!(validate_plain_directory_root(&link, "cooked input").is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[cfg(windows)]
    #[test]
    fn cooked_root_reparse_point_is_rejected_when_symlinks_are_available() {
        use std::os::windows::fs::symlink_dir;

        let base = unique_tmp("root-symlink");
        let target = base.join("target");
        std::fs::create_dir(&target).unwrap();
        let link = base.join("link");
        if symlink_dir(&target, &link).is_err() {
            let _ = std::fs::remove_dir_all(&base);
            return;
        }
        assert!(validate_plain_directory_root(&link, "cooked input").is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn package_path_hash_matches_known_wolf_iostore_id() {
        let id = package_id_from_asset_path(
            "/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps",
        );
        assert_eq!(id.0, 0xc974_a39e_a173_e101);
        assert_eq!(
            package_id_from_asset_path(
                "/game/blueprints/trackingsystem/footstepspresets/da_wolffootsteps"
            ),
            id
        );
    }

    #[test]
    #[ignore = "requires a local Gothic 1 Remake installation"]
    fn real_wolf_generation_probe_passes_bounded_preflight() {
        let game = std::env::var_os("GORE_REAL_GAME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake"));
        let utoc = crate::paths::main_container(&game).unwrap();
        let generation = probe_asset_generation_verified(
            &utoc,
            "/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps",
        )
        .unwrap();
        assert!(generation
            .consumed_chunks
            .iter()
            .any(|chunk| chunk.chunk_type == "ExportBundleData"));
        assert!(generation
            .consumed_chunks
            .iter()
            .any(|chunk| chunk.chunk_type == "ContainerHeader"));
    }

    fn game_dir() -> Option<PathBuf> {
        let p = PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake");
        p.exists().then_some(p)
    }

    /// A unique throwaway dir under the system temp dir (no `tempfile` dep).
    fn unique_tmp(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "gore-tex-test-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// Build a fake triplet `[utoc, ucas, pak]` of small files in `dir`.
    fn fake_triplet(dir: &Path, stem: &str) -> [PathBuf; 3] {
        let exts = ["utoc", "ucas", "pak"];
        let mut out: Vec<PathBuf> = Vec::new();
        for ext in exts {
            let p = dir.join(format!("{stem}.{ext}"));
            std::fs::write(&p, format!("{stem}.{ext} contents").as_bytes()).unwrap();
            out.push(p);
        }
        [out[0].clone(), out[1].clone(), out[2].clone()]
    }

    /// [5] Deploying with a NON-canonical / relative-style `game_dir` must still
    /// record ABSOLUTE paths, so an `undeploy` invoked with a differently-spelled
    /// (absolute) `game_dir` from another cwd resolves them correctly. We pass a
    /// game dir containing a `.` component (the same non-canonical shape `--game .`
    /// produces) and assert every recorded file path is absolute, then undeploy via
    /// the plain absolute dir and confirm the recorded files are gone.
    #[test]
    fn deploy_records_absolute_paths_for_relative_game_dir() {
        let base = unique_tmp("relgame");
        let src_dir = base.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let triplet = fake_triplet(&src_dir, "zzz_mod_tex_P");

        // Non-canonical game dir: `<base>/./.` — `canonicalize` in `deploy` must
        // collapse this so the record holds absolute, canonical paths rather than a
        // path carrying the `.` components.
        let noncanon_game = base.join(".").join(".");
        let record_path = deploy(&triplet, &noncanon_game, "zzz_mod_tex_P").unwrap();
        let json = std::fs::read_to_string(&record_path).unwrap();
        let record: DeployRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record.files.len(), 3);
        for f in &record.files {
            assert!(f.is_absolute(), "record path not absolute: {}", f.display());
            assert!(f.exists(), "record path missing: {}", f.display());
            assert!(
                !f.components().any(|c| c == std::path::Component::CurDir),
                "record path not canonical (has '.'): {}",
                f.display()
            );
        }

        // Undeploy via the plain absolute base (different spelling) still finds and
        // removes exactly the recorded files + the record.
        undeploy(&base, "zzz_mod_tex_P").unwrap();
        for f in &record.files {
            assert!(!f.exists(), "undeploy left file: {}", f.display());
        }
        assert!(!record_path.exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    /// [4] If a triplet copy fails partway, no files from this `deploy` call may be
    /// left in `~mods` (and no record is written) — otherwise a partial IoStore set
    /// mounts on next launch with nothing for undeploy to remove. We force failure
    /// by giving a triplet whose 2nd entry's source does not exist.
    #[test]
    fn deploy_rolls_back_partial_on_copy_failure() {
        let base = unique_tmp("partial");
        let src_dir = base.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        // First src exists; second does NOT -> copy of #2 fails after #1 copied.
        let s0 = src_dir.join("zzz_mod_tex_P.utoc");
        std::fs::write(&s0, b"utoc").unwrap();
        let s1 = src_dir.join("zzz_mod_tex_P.ucas"); // intentionally NOT created
        let s2 = src_dir.join("zzz_mod_tex_P.pak");
        std::fs::write(&s2, b"pak").unwrap();
        let triplet = [s0, s1, s2];

        let err = deploy(&triplet, &base, "zzz_mod_tex_P");
        assert!(err.is_err(), "expected deploy to fail on missing src");

        // The first file's copy succeeded then was rolled back: ~mods must hold
        // neither the copied file nor a deploy record.
        let mods = mods_dir(&base);
        if mods.exists() {
            let leftovers: Vec<_> = std::fs::read_dir(&mods)
                .unwrap()
                .filter_map(|e| e.ok().map(|e| e.file_name()))
                .collect();
            assert!(
                leftovers.is_empty(),
                "partial deploy left files in ~mods: {leftovers:?}"
            );
        }
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Redeploying an already-deployed name overwrites the live `~mods` triplet.
    /// If a later copy fails, rollback must RESTORE the previous triplet's bytes,
    /// not delete them (deletion would wipe the working deployment). Deploy v1,
    /// then attempt v2 (same leaf names, 2nd src missing) and assert the first
    /// destination still holds v1's bytes and the old record survives.
    #[test]
    fn deploy_redeploy_failure_restores_existing_triplet() {
        let base = unique_tmp("redeploy");
        let src1 = base.join("src1");
        std::fs::create_dir_all(&src1).unwrap();
        let v1 = fake_triplet(&src1, "zzz_mod_tex_P");
        deploy(&v1, &base, "zzz_mod_tex_P").unwrap();
        let mods = std::fs::canonicalize(mods_dir(&base)).unwrap();
        let dst_utoc = mods.join("zzz_mod_tex_P.utoc");
        let v1_utoc = std::fs::read(&dst_utoc).unwrap();

        // v2: same leaf names so it targets the same destinations; 2nd src missing
        // so the copy fails AFTER the first destination was overwritten.
        let src2 = base.join("src2");
        std::fs::create_dir_all(&src2).unwrap();
        let s0 = src2.join("zzz_mod_tex_P.utoc");
        std::fs::write(&s0, b"V2 NEW UTOC BYTES").unwrap();
        let s1 = src2.join("zzz_mod_tex_P.ucas"); // intentionally NOT created
        let s2 = src2.join("zzz_mod_tex_P.pak");
        std::fs::write(&s2, b"v2 pak").unwrap();
        let v2 = [s0, s1, s2];

        let err = deploy(&v2, &base, "zzz_mod_tex_P");
        assert!(err.is_err(), "expected redeploy to fail on missing src");

        assert!(
            dst_utoc.exists(),
            "redeploy failure deleted the existing triplet"
        );
        assert_eq!(
            std::fs::read(&dst_utoc).unwrap(),
            v1_utoc,
            "existing triplet bytes were not restored on rollback"
        );
        assert!(
            mods.join("zzz_mod_tex_P.gore-deploy.json").exists(),
            "old deploy record was removed"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    #[ignore = "slow: full container scan; run with --ignored"]
    fn lists_textures_from_real_container() {
        let Some(g) = game_dir() else {
            eprintln!("skip: game not installed");
            return;
        };
        let utoc = crate::paths::main_container(&g).unwrap();
        let usmap = crate::paths::usmap(&g).unwrap();

        let all = list_textures(&utoc, &usmap, None).unwrap();
        eprintln!("total textures: {}", all.len());
        for e in all.iter().take(20) {
            eprintln!("  {}", e.asset_path);
        }
        assert!(all.len() > 100, "expected many textures, got {}", all.len());

        let filtered = list_textures(&utoc, &usmap, Some("Hero")).unwrap();
        eprintln!("filtered (Hero): {}", filtered.len());
        for e in filtered.iter().take(20) {
            eprintln!("  {}", e.asset_path);
        }
        assert!(filtered.len() <= all.len());
        assert!(filtered.iter().all(|e| e.asset_path.contains("Hero")));
    }

    #[test]
    #[ignore = "slow: full container scan; run with --ignored"]
    fn unpacks_one_texture_asset() {
        let Some(g) = game_dir() else {
            eprintln!("skip: game not installed");
            return;
        };
        let utoc = crate::paths::main_container(&g).unwrap();
        let usmap = crate::paths::usmap(&g).unwrap();

        // Take the first "T_" texture the container actually contains, so the
        // test stays valid even if a specific path is renamed by a game patch.
        let textures = list_textures(&utoc, &usmap, Some("T_")).unwrap();
        let asset = textures
            .first()
            .map(|e| e.asset_path.clone())
            .expect("expected at least one T_ texture in the container");
        eprintln!("unpacking asset: {asset}");

        let tmp = std::env::temp_dir().join("gore-tex-unpack-test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let uasset = unpack_asset(&utoc, &usmap, &asset, &tmp).unwrap();
        assert!(uasset.exists());
        assert!(std::fs::metadata(&uasset).unwrap().len() > 0);

        let uexp = uasset.with_extension("uexp");
        let ubulk = uasset.with_extension("ubulk");
        eprintln!(
            "unpacked: {:?} ({} bytes); siblings: uexp={} ({} bytes) ubulk={} ({} bytes)",
            uasset,
            std::fs::metadata(&uasset).unwrap().len(),
            uexp.exists(),
            uexp.exists()
                .then(|| std::fs::metadata(&uexp).unwrap().len())
                .unwrap_or(0),
            ubulk.exists(),
            ubulk
                .exists()
                .then(|| std::fs::metadata(&ubulk).unwrap().len())
                .unwrap_or(0),
        );
    }

    /// `list_pak_files` over a tiny plain V11 pak built with the same repak writer
    /// API `repack_to_zen` uses. Entries are written out of order to prove the
    /// returned list is sorted.
    #[test]
    fn list_pak_files_reads_v11_pak() {
        let dir = unique_tmp("listpak");
        let pak_path = dir.join("tiny.pak");
        {
            use std::io::BufWriter;
            let mut pak_file = BufWriter::new(std::fs::File::create(&pak_path).unwrap());
            let mut w = repak::PakBuilder::new().writer(
                &mut pak_file,
                repak::Version::V11,
                "../../../".to_string(),
                None,
            );
            w.write_file("G1R/Content/B.txt", false, b"bee").unwrap();
            w.write_file("G1R/Content/A.txt", false, b"aye").unwrap();
            w.write_index().unwrap();
        }

        let files = list_pak_files(&pak_path).unwrap();
        assert_eq!(
            files,
            vec![
                "G1R/Content/A.txt".to_string(),
                "G1R/Content/B.txt".to_string()
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A nonexistent container path must surface as an error (io-ish TexError),
    /// not a panic or an empty listing.
    #[test]
    fn list_packages_missing_file_errors() {
        let dir = unique_tmp("nopkg");
        let missing = dir.join("does_not_exist.utoc");
        let err = list_packages(&missing).unwrap_err();
        assert!(
            matches!(err, TexError::Retoc(_) | TexError::Io(_)),
            "expected io-ish error, got: {err:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `list_packages` against the real main container: every package (not just
    /// textures) is listed, so the result must be much larger than the texture
    /// listing and include `/Game/` paths.
    #[test]
    #[ignore = "slow: full container scan; run with --ignored"]
    fn list_packages_main_container_nonempty() {
        let Some(g) = game_dir() else {
            eprintln!("skip: game not installed");
            return;
        };
        let utoc = crate::paths::main_container(&g).unwrap();

        let all = list_packages(&utoc).unwrap();
        eprintln!("total packages: {}", all.len());
        for p in all.iter().take(20) {
            eprintln!("  {p}");
        }
        assert!(
            all.len() > 1000,
            "expected many packages, got {}",
            all.len()
        );
        assert!(
            all.iter().any(|p| p.starts_with("/Game/")),
            "expected at least one /Game/ package path"
        );
        // Sorted + deduped contract.
        assert!(
            all.windows(2).all(|w| w[0] < w[1]),
            "paths not sorted/deduped"
        );
    }

    /// The real-container test above needs the game installed; this fast test pins
    /// the panic-safety contract our per-package loop relies on, with no I/O.
    ///
    /// A malformed package can panic deep in retoc (e.g. `FNameMap::get`'s
    /// `assert_eq!`/unchecked index, see module docs). Constructing such a package
    /// for a unit test would require crafting a full on-disk IoStore container with
    /// a deliberately corrupt zen header -- too expensive to be worthwhile. Instead
    /// we verify the exact mechanism the loop uses: a panic inside the per-package
    /// closure is caught and turned into "skip" (`None`), the surviving packages are
    /// still collected, and the panic hook is restored afterwards.
    #[test]
    fn panicking_package_is_skipped_not_fatal() {
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        let mut out: Vec<u32> = Vec::new();
        // Package 1 -> ok, package 2 -> panics (stand-in for FNameMap::get aborting),
        // package 3 -> ok. A non-panic-safe loop would die on package 2.
        for pkg in [1u32, 2, 3] {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if pkg == 2 {
                    let names: Vec<&str> = Vec::new();
                    // Unchecked out-of-range index, mirroring `self.names[idx]`.
                    return Some(names[5].len() as u32);
                }
                Some(pkg)
            }));
            if let Ok(Some(v)) = result {
                out.push(v);
            }
        }

        std::panic::set_hook(prev_hook);

        // Bad package skipped; good packages survived.
        assert_eq!(out, vec![1, 3]);
    }

    /// deploy/undeploy against a fake game dir (no real container needed): deploy
    /// copies the triplet + writes the record into `~mods`; undeploy removes all 4
    /// and leaves `~mods` empty.
    #[test]
    fn deploy_then_undeploy_roundtrip() {
        let base =
            std::env::temp_dir().join(format!("gore-tex-deploy-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let game = base.join("game");
        let src = base.join("src");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::create_dir_all(&src).unwrap();

        let name = "zzz_X_P";
        let triplet = [
            src.join(format!("{name}.utoc")),
            src.join(format!("{name}.ucas")),
            src.join(format!("{name}.pak")),
        ];
        for p in &triplet {
            std::fs::write(p, b"dummy").unwrap();
        }

        let mods = game.join("G1R/Content/Paks/~mods");

        // deploy: 3 triplet files + the record exist under ~mods. `deploy`
        // canonicalizes the mods dir (so records hold absolute paths even for a
        // relative `--game`), so compare canonicalized paths rather than the exact
        // spelling.
        let record = deploy(&triplet, &game, name).unwrap();
        assert_eq!(
            std::fs::canonicalize(&record).unwrap(),
            std::fs::canonicalize(mods.join(format!("{name}.gore-deploy.json"))).unwrap()
        );
        for ext in ["utoc", "ucas", "pak"] {
            assert!(
                mods.join(format!("{name}.{ext}")).exists(),
                "missing deployed .{ext}"
            );
        }
        assert!(record.exists(), "missing deploy record");

        // undeploy: all 4 gone, ~mods is empty.
        undeploy(&game, name).unwrap();
        for ext in ["utoc", "ucas", "pak"] {
            assert!(
                !mods.join(format!("{name}.{ext}")).exists(),
                ".{ext} not removed"
            );
        }
        assert!(!record.exists(), "record not removed");
        assert_eq!(
            std::fs::read_dir(&mods).unwrap().count(),
            0,
            "~mods should be empty after undeploy"
        );

        // undeploy again -> record-missing error.
        let err = undeploy(&game, name).unwrap_err();
        assert!(matches!(err, TexError::DeployRecordNotFound(_)));

        let _ = std::fs::remove_dir_all(&base);
    }

    /// The to-zen write-path oracle: unpack an UNCHANGED asset, repack the cooked
    /// files into a fresh Zen triplet, then read the asset back OUT of that triplet
    /// and confirm it decodes to the SAME pixels. Proves `repack_to_zen` (legacy ->
    /// zen conversion + FBulkDataMapEntry regeneration + the IoStore writer) yields
    /// a valid, game-readable container.
    #[test]
    #[ignore = "slow: unpack + repack against real container"]
    fn repack_unchanged_roundtrips_to_same_pixels() {
        let g = std::path::PathBuf::from(r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake");
        if !g.exists() {
            eprintln!("skip: game absent");
            return;
        }
        let utoc = crate::paths::main_container(&g).unwrap();
        let usmap = crate::paths::usmap(&g).unwrap();
        let asset = "/Game/UI/Textures/Common/T_HardwareCursor"; // small inline texture

        // 1. unpack original + record its decoded pixels
        let tmp = std::env::temp_dir().join("gore-tex-repack-rt");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let cooked = tmp.join("G1R/Content/UI/Textures/Common");
        std::fs::create_dir_all(&cooked).unwrap();
        let uasset = unpack_asset(&utoc, &usmap, asset, &cooked).unwrap();
        let orig = crate::decode::parse(
            &std::fs::read(&uasset).unwrap(),
            &std::fs::read(uasset.with_extension("uexp")).unwrap(),
            &std::fs::read(uasset.with_extension("ubulk")).unwrap_or_default(),
            &std::fs::read(&usmap).unwrap(),
        )
        .unwrap();
        let orig_px = crate::decode::to_rgba8(&orig).unwrap();

        // 2. repack the (unchanged) cooked dir -> triplet
        let out = tmp.join("out");
        std::fs::create_dir_all(&out).unwrap();
        // DEFAULT (compress = false): the proven-in-game uncompressed write path.
        let triplet = repack_to_zen(&tmp, "RepackRoundTrip_P", &out, &g, false).unwrap();
        for p in &triplet {
            assert!(
                p.exists() && std::fs::metadata(p).unwrap().len() > 0,
                "triplet member missing/empty: {}",
                p.display()
            );
        }
        eprintln!(
            "triplet sizes: utoc={} ucas={} pak={}",
            std::fs::metadata(&triplet[0]).unwrap().len(),
            std::fs::metadata(&triplet[1]).unwrap().len(),
            std::fs::metadata(&triplet[2]).unwrap().len(),
        );

        // 2b. Re-dump the regenerated TOC and prove the DEFAULT path is the
        //     uncompressed one: container_flags == 8 (Indexed only, no Compressed
        //     bit) and NO block carries a non-zero compression method. Reuses
        //     retoc's real Toc reader. (The compress=true path -- flags==9 +
        //     16-aligned compressed offsets -- is covered by the gated
        //     `upscale_streamed_water_2x_roundtrips_through_zen` test.)
        let (flags, comp_offsets) =
            retoc::iostore_writer::dump_compressed_layout(&triplet[0]).unwrap();
        eprintln!(
            "container_flags={flags} (expect 8); {} compressed blocks",
            comp_offsets.len()
        );
        assert_eq!(
            flags, 8,
            "container_flags must be Indexed only (8) for the uncompressed default"
        );
        assert!(
            comp_offsets.is_empty(),
            "uncompressed default must have no compressed blocks (method != 0)"
        );
        eprintln!("OK: container_flags=8 (uncompressed), no compressed blocks");

        // 3. Read the asset back without copying `global.*` beside the generated
        //    triplet and without writing reconstructed legacy files. The built
        //    triplet is authoritative for every target chunk; the installed game
        //    is a read-only fallback only for ScriptObjects/dependencies.
        let rb_package =
            reopen_primary_asset_with_game_fallback_to_memory_v1(&triplet[0], &g, asset).unwrap();
        assert_eq!(rb_package.asset_path(), asset);
        assert!(rb_package
            .chunk_seals()
            .iter()
            .any(|seal| seal.chunk_type() == "ExportBundleData"
                && seal.source_role() == VerifiedReadbackSourceRoleV1::Primary));
        assert!(rb_package
            .chunk_seals()
            .iter()
            .any(|seal| seal.chunk_type() == "ScriptObjects"
                && seal.source_role() == VerifiedReadbackSourceRoleV1::Fallback));
        for ext in ["utoc", "ucas", "pak"] {
            assert!(
                !out.join(format!("global.{ext}")).exists(),
                "readback copied global.{ext} into the build output"
            );
        }
        let rb = crate::decode::parse(
            rb_package.uasset(),
            rb_package.uexp(),
            rb_package
                .sidecar(VerifiedReadbackSidecarKindV1::Bulk)
                .unwrap_or_default(),
            &std::fs::read(&usmap).unwrap(),
        )
        .unwrap();
        let rb_px = crate::decode::to_rgba8(&rb).unwrap();

        // The essential assertion: same pixels in == same pixels out.
        assert_eq!(orig.width, rb.width, "width changed");
        assert_eq!(orig.height, rb.height, "height changed");
        assert_eq!(orig_px.len(), rb_px.len(), "pixel count changed");
        assert!(
            orig_px == rb_px,
            "decoded pixels differ after repack round-trip"
        );
        eprintln!(
            "OK: {}x{} px identical after repack round-trip",
            orig.width, orig.height
        );
    }
}
