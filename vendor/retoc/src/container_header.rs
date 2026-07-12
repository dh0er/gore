use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::Seek;
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, SeekFrom, Write},
    marker::PhantomData,
};
use strum::FromRepr;
use tracing::instrument;

use crate::name_map::{EMappedNameType, read_name_batch_parts, write_name_batch_parts};
use crate::{
    FIoContainerId, FPackageId, FSHAHash, ReadExt,
    name_map::{FMappedName, FNameMap},
    ser::*,
};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct FIoContainerHeader {
    pub version: EIoContainerHeaderVersion,
    pub container_id: FIoContainerId,
    packages: StoreEntries,
    optional_segment_package_ids: Vec<FPackageId>,
    optional_segment_store_entries: Vec<u8>,
    redirect_name_map: FNameMap,
    localized_packages: Vec<FIoContainerHeaderLocalizedPackage>,
    package_redirects: Vec<FIoContainerHeaderPackageRedirect>,
    soft_package_references: Option<FIoContainerHeaderSoftPackageReferences>,
    // Legacy UE4 culture map (also known as localized package map) and package redirects (without source package name information)
    legacy_culture_package_map: FCulturePackageMap,
    legacy_package_redirects: Vec<LegacyContainerHeaderPackageRedirect>,
    // HashSet for IDs of the localized packages, since they only need to be added once
    localized_source_package_ids: HashSet<FPackageId>,
    // Package redirect lookup table, from source package ID to the redirected package ID
    package_redirect_lookup: HashMap<FPackageId, FPackageId>,
}
impl Readable for FIoContainerHeader {
    fn de<S: Read>(s: &mut S) -> Result<Self> {
        Self::deserialize(s, None)
    }
}
impl FIoContainerHeader {
    #[instrument(skip_all, name = "FIoContainerHeader")]
    pub fn deserialize<S: Read>(s: &mut S, version_override: Option<EIoContainerHeaderVersion>) -> Result<Self> {
        let signature: u32 = s.de()?;
        let version: EIoContainerHeaderVersion;
        let container_id;
        // if first 4 bytes are not MAGIC then header version must be Initial or earlier
        // if version override >= Initial, then first 4 bytes must be MAGIC, and version must be known
        if version_override.is_some_and(|v| v <= EIoContainerHeaderVersion::Initial) || signature != Self::MAGIC {
            version = version_override.unwrap_or(EIoContainerHeaderVersion::Initial);
            let mut id = [0; 8];
            id[0..4].copy_from_slice(&signature.to_le_bytes());
            id[4..8].copy_from_slice(&s.de::<[u8; 4]>()?);
            container_id = FIoContainerId(u64::from_le_bytes(id));
        } else {
            version = s.de()?;
            if version_override.is_some_and(|requested| requested != version) {
                bail!(
                    "ContainerHeader version override {:?} disagrees with serialized version {:?}",
                    version_override,
                    version
                );
            }
            container_id = s.de()?;
        }

        if version < EIoContainerHeaderVersion::OptionalSegmentPackages {
            let _package_count: u32 = s.de()?;
        }

        let mut new = Self::new(version, container_id);

        if version <= EIoContainerHeaderVersion::Initial {
            let names_buffer: Vec<u8> = s.de()?;
            let _name_hashes_buffer: Vec<u8> = s.de()?;
            let names = read_name_batch_parts(&names_buffer)?;

            // Create local name map for this container. This map should always be empty in legacy UE4 containers
            new.redirect_name_map = FNameMap::create_from_names(EMappedNameType::Container, names);
        }

        new.packages = StoreEntries::deserialize(s, version)?;

        if version > EIoContainerHeaderVersion::Initial {
            if version >= EIoContainerHeaderVersion::OptionalSegmentPackages {
                new.optional_segment_package_ids = s.de()?;
                new.optional_segment_store_entries = s.de()?;
            }

            new.redirect_name_map = FNameMap::deserialize(s, EMappedNameType::Container)?;
            new.localized_packages = s.de()?;
            new.package_redirects = s.de()?;

            // Populate Source Package IDs of localized packages from the list we just read
            new.localized_source_package_ids = new.package_redirects.iter().map(|x| x.source_package_id).collect();

            // Populate package redirects lookup from the package redirect list
            new.package_redirect_lookup.reserve(new.package_redirects.len());
            for redirect_entry in &new.package_redirects {
                new.package_redirect_lookup.insert(redirect_entry.source_package_id, redirect_entry.target_package_id);
            }
        } else {
            new.legacy_culture_package_map = s.de()?;
            new.legacy_package_redirects = s.de()?;

            // Populate package redirects lookup from the legacy package redirect list
            new.package_redirect_lookup.reserve(new.legacy_package_redirects.len());
            for redirect_entry in &new.legacy_package_redirects {
                new.package_redirect_lookup.insert(redirect_entry.source_package_id, redirect_entry.target_package_id);
            }
        }

        if version >= EIoContainerHeaderVersion::SoftPackageReferences {
            if version >= EIoContainerHeaderVersion::SoftPackageReferencesOffset {
                let soft_package_references_serial_info: FIoContainerHeaderSerialInfo = s.de()?;
                if soft_package_references_serial_info.size > 0 {
                    let has_soft_package_references: bool = s.de()?;
                    if has_soft_package_references {
                        new.soft_package_references = Some(s.de()?);
                    }
                }
            } else {
                let has_soft_package_references: bool = s.de()?;
                if has_soft_package_references {
                    new.soft_package_references = Some(s.de()?);
                }
            }
        }

        Ok(new)
    }
    pub fn serialize<S: Write + Seek>(&self, s: &mut S) -> Result<()> {
        if self.version > EIoContainerHeaderVersion::Initial {
            s.ser(&Self::MAGIC)?;
            s.ser(&self.version)?;
        }
        s.ser(&self.container_id)?;

        if self.version < EIoContainerHeaderVersion::OptionalSegmentPackages {
            s.ser(&(self.packages.0.len() as u32))?;
        }

        if self.version <= EIoContainerHeaderVersion::Initial {
            // Serialize container local name map. This map is generally empty in legacy UE4 containers because there are no fields that write to it
            let (names_buffer, name_hashes_buffer) = write_name_batch_parts(&self.redirect_name_map.copy_raw_names())?;
            s.ser(&names_buffer)?;
            s.ser(&name_hashes_buffer)?;
        }

        self.packages.serialize(s, self.version)?;

        if self.version > EIoContainerHeaderVersion::Initial {
            if self.version >= EIoContainerHeaderVersion::OptionalSegmentPackages {
                s.ser(&self.optional_segment_package_ids)?;
                s.ser(&self.optional_segment_store_entries)?;
            }

            self.redirect_name_map.serialize(s)?;
            s.ser(&self.localized_packages)?;
            s.ser(&self.package_redirects)?;
        } else {
            s.ser(&self.legacy_culture_package_map)?;
            s.ser(&self.legacy_package_redirects)?;
        }

        if self.version >= EIoContainerHeaderVersion::SoftPackageReferences {
            if self.version >= EIoContainerHeaderVersion::SoftPackageReferencesOffset {
                let serial_info_offset = s.stream_position()?;
                let mut soft_package_references_serial_info = FIoContainerHeaderSerialInfo::default();
                s.ser(&soft_package_references_serial_info)?;

                soft_package_references_serial_info.offset = s.stream_position()? as i64;
                s.ser(&self.soft_package_references.is_some())?;
                if let Some(soft_package_references) = &self.soft_package_references {
                    s.ser(soft_package_references)?;
                }
                soft_package_references_serial_info.size = s.stream_position()? as i64 - soft_package_references_serial_info.offset;

                let soft_package_references_end_offset = s.stream_position()?;
                s.seek(SeekFrom::Start(serial_info_offset))?;
                s.ser(&soft_package_references_serial_info)?;
                s.seek(SeekFrom::Start(soft_package_references_end_offset))?;
            } else {
                s.ser(&self.soft_package_references.is_some())?;
                if let Some(soft_package_references) = &self.soft_package_references {
                    s.ser(soft_package_references)?;
                }
            }
        }

        Ok(())
    }
}
impl FIoContainerHeader {
    const MAGIC: u32 = 0x496f436e;

    pub fn new(version: EIoContainerHeaderVersion, container_id: FIoContainerId) -> Self {
        Self {
            version,
            container_id,
            packages: StoreEntries::default(),
            optional_segment_package_ids: vec![],
            optional_segment_store_entries: vec![],
            redirect_name_map: FNameMap::default(),
            localized_packages: vec![],
            package_redirects: vec![],
            soft_package_references: None,
            legacy_culture_package_map: FCulturePackageMap::default(),
            legacy_package_redirects: vec![],
            localized_source_package_ids: HashSet::new(),
            package_redirect_lookup: HashMap::new(),
        }
    }

    pub fn add_package(&mut self, package_id: FPackageId, store_entry: StoreEntry) {
        self.packages.0.insert(package_id, store_entry);
    }

    pub fn add_localized_package(&mut self, package_culture: &str, source_package_name: &str, localized_package_id: FPackageId) -> Result<()> {
        let source_package_id = FPackageId::from_name(source_package_name);

        // New style localized packages do not track the localized package IDs, they only track the list of packages that are localized. Actual Package IDs for localized packages
        // are derived in runtime from package names. So we only need to create a single entry in the localized packages for each package
        if self.version > EIoContainerHeaderVersion::Initial {
            if !self.localized_source_package_ids.contains(&source_package_id) {
                let source_package_mapped_name = self.redirect_name_map.store(source_package_name);

                self.localized_source_package_ids.insert(source_package_id);
                self.localized_packages.push(FIoContainerHeaderLocalizedPackage {
                    source_package_id,
                    source_package_name: source_package_mapped_name,
                });
            }
        } else {
            // Old style localized packages. They track individual packages and their localized variants for each culture
            // Key in the culture package map is the culture name, values are mappings of source package ID to localized package ID
            let culture_localized_packages = self.legacy_culture_package_map.0.entry(package_culture.to_string()).or_default();
            culture_localized_packages.push((source_package_id, localized_package_id));
        }
        Ok(())
    }

    pub fn add_package_redirect(&mut self, source_package_name: &str, redirect_package_id: FPackageId) -> Result<()> {
        let source_package_id = FPackageId::from_name(source_package_name);

        // New style redirects track the package name as well as it's package ID
        if self.version > EIoContainerHeaderVersion::Initial {
            let source_package_name = self.redirect_name_map.store(source_package_name);

            self.package_redirects.push(FIoContainerHeaderPackageRedirect {
                source_package_id,
                source_package_name,
                target_package_id: redirect_package_id,
            });
            self.package_redirect_lookup.insert(source_package_id, redirect_package_id);
        } else {
            // Old style redirects only track bare source package ID and redirect package ID
            self.legacy_package_redirects.push(LegacyContainerHeaderPackageRedirect {
                source_package_id,
                target_package_id: redirect_package_id,
            });
            self.package_redirect_lookup.insert(source_package_id, redirect_package_id);
        }
        Ok(())
    }

    pub fn lookup_package_redirect(&self, source_package_id: FPackageId) -> Option<FPackageId> {
        self.package_redirect_lookup.get(&source_package_id).cloned()
    }

    pub fn get_store_entry(&self, package_id: FPackageId) -> Option<StoreEntry> {
        self.packages.get(package_id)
    }
    pub fn package_ids(&self) -> std::iter::Copied<std::collections::btree_map::Keys<'_, FPackageId, StoreEntry>> {
        self.packages.0.keys().copied()
    }
}

const MAX_PREFLIGHT_HEADER_BYTES: usize = 128 * 1024 * 1024;
const MAX_PREFLIGHT_PACKAGES: usize = 500_000;
const MAX_PREFLIGHT_VECTOR_ELEMENTS: usize = 2_000_000;
const MAX_PREFLIGHT_NAMES: usize = 500_000;
const MAX_PREFLIGHT_NAME_BYTES: usize = 64 * 1024 * 1024;
const FNAME_HASH_ALGORITHM_ID_PREFLIGHT: u64 = 0xC164_0000;

/// Validate every allocation-driving field and every indirect range in a raw
/// ContainerHeader before the normal deserializer is allowed to run.
///
/// The regular reader supports several historical layouts and necessarily
/// follows counts and relative array offsets from the file. This scanner is
/// deliberately allocation-light: only the bounded package-id set is retained,
/// while all buffers and array views are checked against the already bounded
/// chunk bytes.
pub fn preflight_container_header(
    data: &[u8],
    version_override: Option<EIoContainerHeaderVersion>,
) -> Result<()> {
    if data.len() > MAX_PREFLIGHT_HEADER_BYTES {
        bail!(
            "ContainerHeader is {} bytes; limit is {MAX_PREFLIGHT_HEADER_BYTES}",
            data.len()
        );
    }
    let mut cursor = HeaderPreflightCursor::new(data);
    let signature = cursor.u32("signature")?;
    let version;
    if version_override.is_some_and(|value| value <= EIoContainerHeaderVersion::Initial)
        || signature != FIoContainerHeader::MAGIC
    {
        version = version_override.unwrap_or(EIoContainerHeaderVersion::Initial);
        cursor.take(4, "legacy container id")?;
    } else {
        let raw_version = cursor.i32("version")?;
        version = EIoContainerHeaderVersion::from_repr(raw_version)
            .with_context(|| format!("invalid ContainerHeader version {raw_version}"))?;
        if version_override.is_some_and(|requested| requested != version) {
            bail!(
                "ContainerHeader version override {:?} disagrees with serialized version {:?}",
                version_override,
                version
            );
        }
        cursor.take(8, "container id")?;
    }

    let advertised_package_count = if version < EIoContainerHeaderVersion::OptionalSegmentPackages {
        let advertised = cursor.u32("legacy package count")? as usize;
        if advertised > MAX_PREFLIGHT_PACKAGES {
            bail!("legacy package count {advertised} exceeds limit {MAX_PREFLIGHT_PACKAGES}");
        }
        Some(advertised)
    } else {
        None
    };

    if version <= EIoContainerHeaderVersion::Initial {
        let names = cursor.length_prefixed_bytes(MAX_PREFLIGHT_NAME_BYTES, "legacy names")?;
        preflight_name_parts(names)?;
        cursor.length_prefixed_bytes(MAX_PREFLIGHT_NAME_BYTES, "legacy name hashes")?;
    }

    let package_count = preflight_store_entries(&mut cursor, version)?;
    if advertised_package_count.is_some_and(|advertised| advertised != package_count) {
        bail!("legacy package count disagrees with package-id array");
    }

    if version > EIoContainerHeaderVersion::Initial {
        if version >= EIoContainerHeaderVersion::OptionalSegmentPackages {
            cursor.vector_bytes(8, MAX_PREFLIGHT_PACKAGES, "optional package ids")?;
            cursor.length_prefixed_bytes(
                MAX_PREFLIGHT_HEADER_BYTES,
                "optional package store entries",
            )?;
        }

        preflight_name_batch(&mut cursor)?;
        cursor.vector_bytes(16, MAX_PREFLIGHT_VECTOR_ELEMENTS, "localized packages")?;
        cursor.vector_bytes(24, MAX_PREFLIGHT_VECTOR_ELEMENTS, "package redirects")?;
    } else {
        let cultures = cursor.count(MAX_PREFLIGHT_VECTOR_ELEMENTS, "culture map")?;
        for _ in 0..cultures {
            cursor.fstring("culture name")?;
            cursor.vector_bytes(16, MAX_PREFLIGHT_VECTOR_ELEMENTS, "culture packages")?;
        }
        cursor.vector_bytes(16, MAX_PREFLIGHT_VECTOR_ELEMENTS, "legacy redirects")?;
    }

    if version >= EIoContainerHeaderVersion::SoftPackageReferences {
        let serial_range = if version >= EIoContainerHeaderVersion::SoftPackageReferencesOffset {
            let offset = cursor.i64("soft references offset")?;
            let size = cursor.i64("soft references size")?;
            if offset < 0 || size < 0 {
                bail!("negative soft-package-reference offset or size");
            }
            let offset = usize::try_from(offset)?;
            let size = usize::try_from(size)?;
            if offset != cursor.position() {
                bail!(
                    "soft-package-reference offset {offset} does not match inline position {}",
                    cursor.position()
                );
            }
            let end = offset
                .checked_add(size)
                .context("soft-package-reference range overflow")?;
            if end > data.len() {
                bail!("soft-package-reference range ends outside ContainerHeader");
            }
            Some((offset, end))
        } else {
            None
        };

        if serial_range.is_none_or(|(start, end)| end > start) {
            let has_references = cursor.bool_u32("soft references present")?;
            if has_references {
                cursor.vector_bytes(8, MAX_PREFLIGHT_VECTOR_ELEMENTS, "soft package ids")?;
                cursor.length_prefixed_bytes(
                    MAX_PREFLIGHT_HEADER_BYTES,
                    "soft package indices",
                )?;
            }
        }
        if let Some((_, expected_end)) = serial_range {
            if cursor.position() != expected_end {
                bail!(
                    "soft-package-reference payload ended at {}, expected {expected_end}",
                    cursor.position()
                );
            }
        }
    }

    let trailing = &data[cursor.position()..];
    if trailing.len() > 15 || trailing.iter().any(|byte| *byte != 0) {
        bail!(
            "ContainerHeader has {} non-padding trailing bytes",
            trailing.len()
        );
    }
    Ok(())
}

fn preflight_store_entries(
    cursor: &mut HeaderPreflightCursor<'_>,
    version: EIoContainerHeaderVersion,
) -> Result<usize> {
    let package_count = cursor.count(MAX_PREFLIGHT_PACKAGES, "package ids")?;
    let package_ids = cursor.take(
        package_count
            .checked_mul(8)
            .context("package-id byte count overflow")?,
        "package ids",
    )?;
    let mut unique = HashSet::new();
    unique
        .try_reserve(package_count)
        .context("reserving bounded package-id set")?;
    for bytes in package_ids.chunks_exact(8) {
        let id = u64::from_le_bytes(bytes.try_into().expect("chunks_exact yields eight bytes"));
        if !unique.insert(id) {
            bail!("duplicate package id {id:#x} in ContainerHeader");
        }
    }

    let store = cursor.length_prefixed_bytes(
        MAX_PREFLIGHT_HEADER_BYTES,
        "package store entry buffer",
    )?;
    let (member_offset, entry_size, imported_view, shader_view) = match version {
        EIoContainerHeaderVersion::PreInitial => (8usize, 16usize, 8usize, None),
        EIoContainerHeaderVersion::Initial => (24, 32, 24, None),
        EIoContainerHeaderVersion::LocalizedPackages
        | EIoContainerHeaderVersion::OptionalSegmentPackages => (8, 24, 8, Some(16)),
        EIoContainerHeaderVersion::NoExportInfo
        | EIoContainerHeaderVersion::SoftPackageReferences
        | EIoContainerHeaderVersion::SoftPackageReferencesOffset => (0, 16, 0, Some(8)),
    };
    let entries_bytes = package_count
        .checked_mul(entry_size)
        .context("package store entry table overflow")?;
    if entries_bytes > store.len() {
        bail!(
            "package store entry table needs {entries_bytes} bytes, buffer has {}",
            store.len()
        );
    }

    let mut aggregate_refs = 0usize;
    for index in 0..package_count {
        let base = index
            .checked_mul(entry_size)
            .context("package store entry offset overflow")?;
        if version < EIoContainerHeaderVersion::NoExportInfo {
            let count_offset = if version == EIoContainerHeaderVersion::Initial {
                base + 8
            } else {
                base
            };
            for (offset, label) in [
                (count_offset, "export count"),
                (count_offset + 4, "export bundle count"),
            ] {
                let count = read_i32_at(store, offset, label)?;
                if !(0..=2_000_000).contains(&count) {
                    bail!("{label} {count} is outside bounded nonnegative range");
                }
            }
        }
        validate_store_array_view(
            store,
            base,
            member_offset,
            imported_view,
            8,
            &mut aggregate_refs,
            "imported packages",
        )?;
        if let Some(shader_view) = shader_view {
            validate_store_array_view(
                store,
                base,
                member_offset + 8,
                shader_view,
                20,
                &mut aggregate_refs,
                "shader map hashes",
            )?;
        }
    }
    Ok(package_count)
}

#[allow(clippy::too_many_arguments)]
fn validate_store_array_view(
    store: &[u8],
    entry_base: usize,
    relative_base: usize,
    view_offset: usize,
    element_size: usize,
    aggregate: &mut usize,
    label: &'static str,
) -> Result<()> {
    let view = entry_base
        .checked_add(view_offset)
        .context("store array view offset overflow")?;
    let count = read_u32_at(store, view, label)? as usize;
    let relative = read_u32_at(store, view + 4, label)? as usize;
    if count > MAX_PREFLIGHT_VECTOR_ELEMENTS {
        bail!("{label} count {count} exceeds limit {MAX_PREFLIGHT_VECTOR_ELEMENTS}");
    }
    *aggregate = aggregate
        .checked_add(count)
        .context("aggregate ContainerHeader reference count overflow")?;
    if *aggregate > MAX_PREFLIGHT_VECTOR_ELEMENTS {
        bail!(
            "aggregate ContainerHeader reference count {} exceeds limit {MAX_PREFLIGHT_VECTOR_ELEMENTS}",
            *aggregate
        );
    }
    if count == 0 {
        return Ok(());
    }
    let start = entry_base
        .checked_add(relative_base)
        .and_then(|value| value.checked_add(relative))
        .context("store array data offset overflow")?;
    let bytes = count
        .checked_mul(element_size)
        .context("store array byte count overflow")?;
    let end = start
        .checked_add(bytes)
        .context("store array end overflow")?;
    if end > store.len() {
        bail!("{label} range {start}..{end} exceeds store buffer {}", store.len());
    }
    Ok(())
}

fn read_u32_at(bytes: &[u8], offset: usize, label: &'static str) -> Result<u32> {
    let end = offset.checked_add(4).context("u32 range overflow")?;
    let raw = bytes
        .get(offset..end)
        .with_context(|| format!("truncated {label} view"))?;
    Ok(u32::from_le_bytes(
        raw.try_into().expect("range was checked to four bytes"),
    ))
}

fn read_i32_at(bytes: &[u8], offset: usize, label: &'static str) -> Result<i32> {
    let end = offset.checked_add(4).context("i32 range overflow")?;
    let raw = bytes
        .get(offset..end)
        .with_context(|| format!("truncated {label}"))?;
    Ok(i32::from_le_bytes(
        raw.try_into().expect("range was checked to four bytes"),
    ))
}

fn preflight_name_parts(bytes: &[u8]) -> Result<()> {
    let mut cursor = HeaderPreflightCursor::new(bytes);
    let mut count = 0usize;
    while cursor.position() < bytes.len() {
        count = count.checked_add(1).context("name count overflow")?;
        if count > MAX_PREFLIGHT_NAMES {
            bail!("legacy name count exceeds limit {MAX_PREFLIGHT_NAMES}");
        }
        let raw = i16::from_be_bytes(
            cursor
                .take(2, "legacy name length")?
                .try_into()
                .expect("two bytes were requested"),
        );
        let decoded = if raw < 0 { i16::MIN - raw } else { raw };
        let units = usize::from(decoded.unsigned_abs());
        if decoded < 0 && cursor.position() & 1 != 0 {
            cursor.take(1, "legacy UTF-16 alignment")?;
        }
        cursor.take(
            units
                .checked_mul(if decoded < 0 { 2 } else { 1 })
                .context("legacy name byte count overflow")?,
            "legacy name bytes",
        )?;
    }
    Ok(())
}

fn preflight_name_batch(cursor: &mut HeaderPreflightCursor<'_>) -> Result<()> {
    let count = cursor.count(MAX_PREFLIGHT_NAMES, "name batch")?;
    if count == 0 {
        return Ok(());
    }
    let advertised_string_bytes = cursor.u32("name batch string bytes")? as usize;
    if advertised_string_bytes > MAX_PREFLIGHT_NAME_BYTES {
        bail!(
            "name batch string bytes {advertised_string_bytes} exceed limit {MAX_PREFLIGHT_NAME_BYTES}"
        );
    }
    let hash_algorithm = cursor.u64("name batch hash algorithm")?;
    if hash_algorithm != FNAME_HASH_ALGORITHM_ID_PREFLIGHT {
        bail!("unsupported FName hash algorithm {hash_algorithm:#x}");
    }
    cursor.take(
        count.checked_mul(8).context("name hash bytes overflow")?,
        "name hashes",
    )?;
    let lengths = cursor.take(
        count
            .checked_mul(2)
            .context("name length table overflow")?,
        "name lengths",
    )?;
    let names_start = cursor.position();
    for raw in lengths.chunks_exact(2) {
        let raw = i16::from_be_bytes(raw.try_into().expect("two-byte name length"));
        let decoded = if raw < 0 { i16::MIN - raw } else { raw };
        let units = usize::from(decoded.unsigned_abs());
        cursor.take(
            units
                .checked_mul(if decoded < 0 { 2 } else { 1 })
                .context("name bytes overflow")?,
            "name bytes",
        )?;
    }
    let actual = cursor.position() - names_start;
    if actual != advertised_string_bytes {
        bail!(
            "name batch advertised {advertised_string_bytes} string bytes, parsed {actual}"
        );
    }
    Ok(())
}

struct HeaderPreflightCursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> HeaderPreflightCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn position(&self) -> usize {
        self.position
    }

    fn take(&mut self, length: usize, label: &'static str) -> Result<&'a [u8]> {
        let end = self
            .position
            .checked_add(length)
            .with_context(|| format!("{label} range overflow"))?;
        let value = self
            .bytes
            .get(self.position..end)
            .with_context(|| format!("truncated {label}"))?;
        self.position = end;
        Ok(value)
    }

    fn u32(&mut self, label: &'static str) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.take(4, label)?
                .try_into()
                .expect("four bytes were requested"),
        ))
    }

    fn i32(&mut self, label: &'static str) -> Result<i32> {
        Ok(i32::from_le_bytes(
            self.take(4, label)?
                .try_into()
                .expect("four bytes were requested"),
        ))
    }

    fn u64(&mut self, label: &'static str) -> Result<u64> {
        Ok(u64::from_le_bytes(
            self.take(8, label)?
                .try_into()
                .expect("eight bytes were requested"),
        ))
    }

    fn i64(&mut self, label: &'static str) -> Result<i64> {
        Ok(i64::from_le_bytes(
            self.take(8, label)?
                .try_into()
                .expect("eight bytes were requested"),
        ))
    }

    fn count(&mut self, limit: usize, label: &'static str) -> Result<usize> {
        let count = self.u32(label)? as usize;
        if count > limit {
            bail!("{label} count {count} exceeds limit {limit}");
        }
        Ok(count)
    }

    fn vector_bytes(
        &mut self,
        element_size: usize,
        limit: usize,
        label: &'static str,
    ) -> Result<&'a [u8]> {
        let count = self.count(limit, label)?;
        self.take(
            count
                .checked_mul(element_size)
                .with_context(|| format!("{label} byte count overflow"))?,
            label,
        )
    }

    fn length_prefixed_bytes(
        &mut self,
        limit: usize,
        label: &'static str,
    ) -> Result<&'a [u8]> {
        let length = self.u32(label)? as usize;
        if length > limit {
            bail!("{label} length {length} exceeds limit {limit}");
        }
        self.take(length, label)
    }

    fn bool_u32(&mut self, label: &'static str) -> Result<bool> {
        match self.u32(label)? {
            0 => Ok(false),
            1 => Ok(true),
            value => bail!("{label} has non-boolean value {value}"),
        }
    }

    fn fstring(&mut self, label: &'static str) -> Result<()> {
        let length = self.i32(label)?;
        let units = if length < 0 {
            length
                .checked_abs()
                .with_context(|| format!("{label} length is i32::MIN"))? as usize
        } else {
            length as usize
        };
        if units > MAX_PREFLIGHT_NAME_BYTES {
            bail!("{label} length {units} exceeds limit {MAX_PREFLIGHT_NAME_BYTES}");
        }
        self.take(
            units
                .checked_mul(if length < 0 { 2 } else { 1 })
                .with_context(|| format!("{label} byte count overflow"))?,
            label,
        )?;
        Ok(())
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, FromRepr, clap::ValueEnum, Serialize, Deserialize)]
#[repr(i32)]
#[clap(rename_all = "verbatim")]
pub enum EIoContainerHeaderVersion {
    PreInitial = -1,
    Initial = 0,
    LocalizedPackages = 1,
    OptionalSegmentPackages = 2,
    NoExportInfo = 3,
    SoftPackageReferences = 4,
    #[default]
    SoftPackageReferencesOffset = 5,
}
impl Readable for EIoContainerHeaderVersion {
    #[instrument(skip_all, name = "EIoContainerHeaderVersion")]
    fn de<S: Read>(s: &mut S) -> Result<Self> {
        let value = s.de()?;
        Self::from_repr(value).with_context(|| format!("invalid EIoContainerHeaderVersion value: {value}"))
    }
}
impl Writeable for EIoContainerHeaderVersion {
    fn ser<S: Write>(&self, s: &mut S) -> Result<()> {
        s.ser(&(*self as u32))
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct FIoContainerHeaderLocalizedPackage {
    source_package_id: FPackageId,
    source_package_name: FMappedName,
}
impl Readable for FIoContainerHeaderLocalizedPackage {
    #[instrument(skip_all, name = "FIoContainerHeaderLocalizedPackage")]
    fn de<S: Read>(s: &mut S) -> Result<Self> {
        Ok(Self {
            source_package_id: s.de()?,
            source_package_name: s.de()?,
        })
    }
}
impl Writeable for FIoContainerHeaderLocalizedPackage {
    fn ser<S: Write>(&self, s: &mut S) -> Result<()> {
        s.ser(&self.source_package_id)?;
        s.ser(&self.source_package_name)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct FIoContainerHeaderPackageRedirect {
    source_package_id: FPackageId,
    target_package_id: FPackageId,
    source_package_name: FMappedName,
}
impl Readable for FIoContainerHeaderPackageRedirect {
    #[instrument(skip_all, name = "FIoContainerHeaderPackageRedirect")]
    fn de<S: Read>(s: &mut S) -> Result<Self> {
        Ok(Self {
            source_package_id: s.de()?,
            target_package_id: s.de()?,
            source_package_name: s.de()?,
        })
    }
}
impl Writeable for FIoContainerHeaderPackageRedirect {
    fn ser<S: Write>(&self, s: &mut S) -> Result<()> {
        s.ser(&self.source_package_id)?;
        s.ser(&self.target_package_id)?;
        s.ser(&self.source_package_name)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct FIoContainerHeaderSoftPackageReferences {
    package_ids: Vec<FPackageId>,
    package_indices: Vec<u8>,
}
impl Readable for FIoContainerHeaderSoftPackageReferences {
    #[instrument(skip_all, name = "FIoContainerHeaderSoftPackageReferences")]
    fn de<S: Read>(s: &mut S) -> Result<Self> {
        Ok(Self { package_ids: s.de()?, package_indices: s.de()? })
    }
}
impl Writeable for FIoContainerHeaderSoftPackageReferences {
    fn ser<S: Write>(&self, s: &mut S) -> Result<()> {
        s.ser(&self.package_ids)?;
        s.ser(&self.package_indices)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Default, Serialize, Deserialize)]
struct FIoContainerHeaderSerialInfo {
    offset: i64,
    size: i64,
}
impl Readable for FIoContainerHeaderSerialInfo {
    #[instrument(skip_all, name = "FIoContainerHeaderSerialInfo")]
    fn de<S: Read>(s: &mut S) -> Result<Self> {
        Ok(Self { offset: s.de()?, size: s.de()? })
    }
}
impl Writeable for FIoContainerHeaderSerialInfo {
    fn ser<S: Write>(&self, s: &mut S) -> Result<()> {
        s.ser(&self.offset)?;
        s.ser(&self.size)?;
        Ok(())
    }
}

// Used for UE4.27 package redirects that do not provide a source package name
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct LegacyContainerHeaderPackageRedirect {
    source_package_id: FPackageId,
    target_package_id: FPackageId,
}
impl Readable for LegacyContainerHeaderPackageRedirect {
    #[instrument(skip_all, name = "LegacyContainerHeaderPackageRedirect")]
    fn de<S: Read>(s: &mut S) -> Result<Self> {
        Ok(Self { source_package_id: s.de()?, target_package_id: s.de()? })
    }
}
impl Writeable for LegacyContainerHeaderPackageRedirect {
    fn ser<S: Write>(&self, s: &mut S) -> Result<()> {
        s.ser(&self.source_package_id)?;
        s.ser(&self.target_package_id)?;
        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoreEntry {
    // version == EIoContainerHeaderVersion::NoExportInfo
    pub export_bundles_size: u64,
    pub load_order: u32,

    // version < EIoContainerHeaderVersion::NoExportInfo
    pub export_count: i32,
    pub export_bundle_count: i32,

    pub imported_packages: Vec<FPackageId>,
    pub shader_map_hashes: Vec<FSHAHash>,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
struct StoreEntries(BTreeMap<FPackageId, StoreEntry>);
impl StoreEntries {
    fn get(&self, package_id: FPackageId) -> Option<StoreEntry> {
        self.0.get(&package_id).cloned()
    }
    #[instrument(skip_all, name = "StoreEntries")]
    fn deserialize<S: Read>(s: &mut S, version: EIoContainerHeaderVersion) -> Result<Self> {
        let package_ids: Vec<FPackageId> = s.de()?;

        let buffer: Vec<u8> = s.de()?;
        let mut cur = Cursor::new(buffer);
        //let mut cur = ser_hex::TraceStream::new("trace_store.json", &mut cur);

        let (member_offset, entry_size) = match version {
            EIoContainerHeaderVersion::PreInitial => (8, 16),
            EIoContainerHeaderVersion::Initial => (24, 32),
            EIoContainerHeaderVersion::LocalizedPackages => (8, 24),
            EIoContainerHeaderVersion::OptionalSegmentPackages => (8, 24),
            EIoContainerHeaderVersion::NoExportInfo => (0, 16),
            EIoContainerHeaderVersion::SoftPackageReferences => (0, 16),
            EIoContainerHeaderVersion::SoftPackageReferencesOffset => (0, 16),
        };

        let entries = read_array(package_ids.len(), &mut cur, |s| FFilePackageStoreEntry::deserialize(s, version))?;

        let entries = entries
            .into_iter()
            .enumerate()
            .map(|(i, entry)| -> Result<StoreEntry> {
                let offset = i * entry_size; // sizeof(FFilePackageStoreEntry)

                let mut new = StoreEntry {
                    export_bundles_size: entry.export_bundles_size,
                    load_order: entry.load_order,

                    export_count: entry.export_count,
                    export_bundle_count: entry.export_bundle_count,

                    ..Default::default()
                };

                let num = entry.imported_packages.array_num as usize;
                new.imported_packages = if num != 0 {
                    let offset = offset + member_offset + entry.imported_packages.offset_to_data_from_this as usize; // offset_of(FFilePackageStoreEntry::imported_packages)
                    cur.seek(SeekFrom::Start(offset as u64))?;
                    cur.de_ctx(num)?
                } else {
                    vec![]
                };

                if version > EIoContainerHeaderVersion::Initial {
                    let num = entry.shader_map_hashes.array_num as usize;
                    new.shader_map_hashes = if num != 0 {
                        let offset = offset + member_offset + entry.shader_map_hashes.offset_to_data_from_this as usize + 8; // offset_of(FFilePackageStoreEntry::shader_map_hashes)
                        cur.seek(SeekFrom::Start(offset as u64))?;
                        cur.de_ctx(num)?
                    } else {
                        vec![]
                    };
                }

                Ok(new)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self(BTreeMap::from_iter(package_ids.into_iter().zip(entries.into_iter()))))
    }
    #[instrument(skip_all, name = "StoreEntries")]
    fn serialize<S: Write>(&self, s: &mut S, version: EIoContainerHeaderVersion) -> Result<()> {
        s.ser(&(self.0.len() as u32))?;
        for package_id in self.0.keys() {
            s.ser(package_id)?;
        }

        let mut buffer: Vec<u8> = vec![];
        let mut cur = Cursor::new(&mut buffer);

        let (member_offset, entry_size) = match version {
            EIoContainerHeaderVersion::PreInitial => (8, 16),
            EIoContainerHeaderVersion::Initial => (24, 32),
            EIoContainerHeaderVersion::LocalizedPackages => (8, 24),
            EIoContainerHeaderVersion::OptionalSegmentPackages => (8, 24),
            EIoContainerHeaderVersion::NoExportInfo => (0, 16),
            EIoContainerHeaderVersion::SoftPackageReferences => (0, 16),
            EIoContainerHeaderVersion::SoftPackageReferencesOffset => (0, 16),
        };

        // calculate end of entries to start writing arrays
        let mut array_offset = self.0.len() * entry_size;

        for entry in self.0.values() {
            let mut ser_entry = FFilePackageStoreEntry {
                export_bundles_size: entry.export_bundles_size,
                load_order: entry.load_order,

                export_count: entry.export_count,
                export_bundle_count: entry.export_bundle_count,

                ..Default::default()
            };

            // save entry to calculate offsets and restore later
            let entry_offset = cur.position() as usize;

            // start writing arrays
            cur.set_position(array_offset as u64);

            if !entry.imported_packages.is_empty() {
                let offset = cur.position() as usize - entry_offset - member_offset;
                ser_entry.imported_packages.offset_to_data_from_this = offset as u32;
                ser_entry.imported_packages.array_num = entry.imported_packages.len() as u32;
                cur.ser_no_length(&entry.imported_packages)?;
            }
            if version > EIoContainerHeaderVersion::Initial && !entry.shader_map_hashes.is_empty() {
                let offset = cur.position() as usize - entry_offset - member_offset - 8;
                ser_entry.shader_map_hashes.offset_to_data_from_this = offset as u32;
                ser_entry.shader_map_hashes.array_num = entry.shader_map_hashes.len() as u32;
                cur.ser_no_length(&entry.shader_map_hashes)?;
            }

            // advance array_offset
            array_offset = cur.position() as usize;

            // reset cursor and write entry
            cur.set_position(entry_offset as u64);
            ser_entry.serialize(&mut cur, version)?;
        }

        s.ser::<Vec<u8>>(&buffer)?;
        Ok(())
    }
}

#[derive(Debug, Default)]
#[repr(C)]
struct TFilePackageStoreEntryCArrayView<T> {
    array_num: u32,
    offset_to_data_from_this: u32,
    _phantom: PhantomData<T>,
}
impl<T> Readable for TFilePackageStoreEntryCArrayView<T> {
    fn de<S: Read>(s: &mut S) -> Result<Self> {
        Ok(Self {
            array_num: s.de()?,
            offset_to_data_from_this: s.de()?,
            _phantom: Default::default(),
        })
    }
}
impl<T> Writeable for TFilePackageStoreEntryCArrayView<T> {
    fn ser<S: Write>(&self, s: &mut S) -> Result<()> {
        s.ser(&self.array_num)?;
        s.ser(&self.offset_to_data_from_this)?;
        Ok(())
    }
}

#[derive(Debug, Default)]
struct FFilePackageStoreEntry {
    // version == EIoContainerHeaderVersion::NoExportInfo
    export_bundles_size: u64,
    load_order: u32,

    // version < EIoContainerHeaderVersion::NoExportInfo
    export_count: i32,
    export_bundle_count: i32,

    imported_packages: TFilePackageStoreEntryCArrayView<FPackageId>,
    shader_map_hashes: TFilePackageStoreEntryCArrayView<FSHAHash>,
}
impl FFilePackageStoreEntry {
    #[instrument(skip_all, name = "FFilePackageStoreEntry")]
    fn deserialize<S: Read>(s: &mut S, version: EIoContainerHeaderVersion) -> Result<Self> {
        let mut entry = Self::default();

        if version == EIoContainerHeaderVersion::Initial {
            entry.export_bundles_size = s.de()?;
        }
        if version < EIoContainerHeaderVersion::NoExportInfo {
            entry.export_count = s.de()?;
            entry.export_bundle_count = s.de()?;
        }
        if version == EIoContainerHeaderVersion::Initial {
            entry.load_order = s.de()?;
            let _pad: u32 = s.de()?;
        }
        entry.imported_packages = s.de()?;
        if version > EIoContainerHeaderVersion::Initial {
            entry.shader_map_hashes = s.de()?;
        };
        Ok(entry)
    }
    #[instrument(skip_all, name = "FFilePackageStoreEntry")]
    fn serialize<S: Write>(&self, s: &mut S, version: EIoContainerHeaderVersion) -> Result<()> {
        if version == EIoContainerHeaderVersion::Initial {
            s.ser(&self.export_bundles_size)?;
        }
        if version < EIoContainerHeaderVersion::NoExportInfo {
            s.ser(&self.export_count)?;
            s.ser(&self.export_bundle_count)?;
        }
        if version == EIoContainerHeaderVersion::Initial {
            s.ser(&self.load_order)?;
            s.ser(&0u32)?;
        }
        s.ser(&self.imported_packages)?;
        if version > EIoContainerHeaderVersion::Initial {
            s.ser(&self.shader_map_hashes)?;
        }
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
struct FCulturePackageMap(BTreeMap<String, Vec<(FPackageId, FPackageId)>>);
impl Readable for FCulturePackageMap {
    fn de<S: Read>(s: &mut S) -> Result<Self> {
        let culture_package_map_len: u32 = s.de()?;
        let mut culture_package_map = BTreeMap::new();
        for _ in 0..culture_package_map_len {
            let key: String = s.de()?;
            let value: Vec<(FPackageId, FPackageId)> = read_array(s.de::<u32>()? as usize, s, |s| Ok((s.de()?, s.de()?)))?;
            culture_package_map.insert(key, value);
        }
        Ok(Self(culture_package_map))
    }
}
impl Writeable for FCulturePackageMap {
    fn ser<S: Write>(&self, s: &mut S) -> Result<()> {
        s.ser(&(self.0.len() as u32))?;
        for (key, value) in &self.0 {
            s.ser(key)?;
            s.ser(&(value.len() as u32))?;
            for (a, b) in value {
                s.ser(a)?;
                s.ser(b)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use fs_err as fs;

    fn test_rw_container_header(data: &[u8], version_override: Option<EIoContainerHeaderVersion>) -> Result<()> {
        let header = FIoContainerHeader::deserialize(&mut Cursor::new(data), version_override)?;

        let mut out_cur = Cursor::new(vec![]);
        header.serialize(&mut out_cur)?;
        out_cur.set_position(0);

        let header2 = FIoContainerHeader::deserialize(&mut out_cur, version_override)?;

        //fs::write("header_in.json", serde_json::to_string(&header)?)?;
        //fs::write("header_out.json", serde_json::to_string(&header2)?)?;
        //fs::write("header_out.bin", out_cur.into_inner())?;

        assert_eq!(header, header2);
        Ok(())
    }

    #[test]
    fn test_container_header_new() -> Result<()> {
        let data = fs::read("tests/UE5.3/ContainerHeader_1.bin")?;
        test_rw_container_header(&data, None)?;
        Ok(())
    }

    #[test]
    fn test_container_header_initial() -> Result<()> {
        let data = fs::read("tests/UE4.27/ContainerHeader_1.bin")?;
        test_rw_container_header(&data, None)?;
        Ok(())
    }

    #[test]
    fn test_container_header_issue7() -> Result<()> {
        let data = fs::read("tests/issues/issue7/header.bin")?;
        test_rw_container_header(&data, Some(EIoContainerHeaderVersion::PreInitial))?;
        Ok(())
    }

    #[test]
    fn test_container_header_issue18() -> Result<()> {
        let data = fs::read("tests/issues/issue18/header.bin")?;
        test_rw_container_header(&data, Some(EIoContainerHeaderVersion::PreInitial))?;
        Ok(())
    }

    #[test]
    fn test_container_header_localized_packages() -> Result<()> {
        let data = fs::read("tests/UE5.0/ContainerHeader_1.bin")?;
        test_rw_container_header(&data, None)?;
        Ok(())
    }
}
