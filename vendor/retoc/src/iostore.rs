use std::{
    collections::HashSet,
    ffi::OsStr,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, bail};
use fs_err as fs;

use crate::{
    Config, EIoChunkType, EIoStoreTocVersion, FIoChunkHash, FIoChunkId, FPackageId, Toc,
    chunk_id::FIoChunkIdRaw,
    container_header::{
        EIoContainerHeaderVersion, FIoContainerHeader, StoreEntry,
        preflight_container_header,
    },
    file_pool::FilePool,
    script_objects::ZenScriptObjects,
    ser::*,
};

macro_rules! indent_println {
    ($indent:expr, $($arg:tt)*) => {
        println!("{:width$}{}", "", format!($($arg)*), width = 2 * $indent);
    }
}

struct UniqueIterator<I, T> {
    inner: I,
    encountered: HashSet<T>,
}

impl<I, T> UniqueIterator<I, T> {
    fn new(inner: I) -> Self {
        Self { inner, encountered: HashSet::new() }
    }
}

impl<I: Iterator> Iterator for UniqueIterator<I, I::Item>
where
    I::Item: std::hash::Hash + std::cmp::Eq + Copy,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = self.inner.next();
            if let Some(next) = next {
                if self.encountered.insert(next) {
                    return Some(next);
                }
            } else {
                return None;
            }
        }
    }
}

pub fn open<P: AsRef<Path>>(path: P, config: Arc<Config>) -> Result<Box<dyn IoStoreTrait>> {
    Ok(if path.as_ref().is_dir() { Box::new(IoStoreBackend::open(path, config)?) } else { Box::new(IoStoreContainer::open(path, config)?) })
}

/// Return an object that can be sorted by to achieve container priority.
/// Higher priority should Cmp higher
fn sort_container_name(full_name: &str) -> (bool, u32, &str) {
    let mut base_name = full_name;

    let mut chunk_version = 0;
    if let Some(name) = base_name.strip_suffix("_P") {
        base_name = name;
        chunk_version = 1;
        if let Some((name, version)) = base_name.rsplit_once("_")
            && let Ok(version) = version.parse::<u32>()
        {
            base_name = name;
            chunk_version = version + 2;
        }
    }

    // special case global to always sort highest
    (full_name == "global", chunk_version, base_name)
}

fn ensure_unique_container_header_ids(
    ids: impl IntoIterator<Item = FIoChunkId>,
) -> Result<()> {
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            bail!(
                "duplicate ContainerHeader chunk id {:#x} across sibling containers",
                id.get_chunk_id()
            );
        }
    }
    Ok(())
}

const MAX_COMPOSITE_CONTAINERS: usize = 256;
const MAX_COMPOSITE_TOC_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_COMPOSITE_METADATA_BYTES: u64 = 128 * 1024 * 1024;
const MAX_TOC_BYTES: u64 = 256 * 1024 * 1024;
const MAX_TOC_ENTRIES: usize = 500_000;
const MAX_COMPRESSION_BLOCKS: usize = 1_200_000;
const MAX_COMPRESSION_METHODS: usize = 64;
const MAX_DIRECTORY_INDEX_BYTES: usize = 128 * 1024 * 1024;
const MAX_DIRECTORY_ENTRIES: usize = 250_000;
const MAX_DIRECTORY_DEPTH: usize = 256;
const MAX_CONTAINER_HEADER_CHUNK_BYTES: u64 = 128 * 1024 * 1024;
const MAX_ADVERTISED_CHUNK_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_UCAS_BYTES: u64 = 128 * 1024 * 1024 * 1024;

#[derive(Debug)]
struct RawTocHeader {
    version: u8,
    entry_count: usize,
    compression_block_count: usize,
    compression_method_count: usize,
    compression_method_name_length: usize,
    compression_block_size: u64,
    directory_index_size: usize,
    container_flags: u8,
    perfect_hash_count: usize,
    overflow_count: usize,
}

fn checked_product(left: usize, right: usize, label: &'static str) -> Result<usize> {
    left.checked_mul(right).with_context(|| format!("{label} size overflow"))
}

fn checked_advance(position: &mut usize, amount: usize, total: usize, label: &'static str) -> Result<std::ops::Range<usize>> {
    let start = *position;
    let end = start.checked_add(amount).with_context(|| format!("{label} range overflow"))?;
    if end > total {
        bail!("truncated {label}: range {start}..{end}, file size {total}");
    }
    *position = end;
    Ok(start..end)
}

fn raw_u32(bytes: &[u8], offset: usize, label: &'static str) -> Result<u32> {
    let end = offset.checked_add(4).context("u32 range overflow")?;
    let value = bytes.get(offset..end).with_context(|| format!("truncated {label}"))?;
    Ok(u32::from_le_bytes(value.try_into().expect("four-byte range")))
}

fn raw_u64(bytes: &[u8], offset: usize, label: &'static str) -> Result<u64> {
    let end = offset.checked_add(8).context("u64 range overflow")?;
    let value = bytes.get(offset..end).with_context(|| format!("truncated {label}"))?;
    Ok(u64::from_le_bytes(value.try_into().expect("eight-byte range")))
}

fn parse_raw_toc_header(bytes: &[u8]) -> Result<RawTocHeader> {
    if bytes.len() < 0x90 {
        bail!("TOC is {} bytes; fixed header requires 144", bytes.len());
    }
    if bytes[..16] != *b"-==--==--==--==-" {
        bail!("unrecognized TOC magic, this is a .utoc file?");
    }
    let version = bytes[16];
    if !(1..=8).contains(&version) {
        bail!("unsupported TOC version {version}");
    }
    let header_size = raw_u32(bytes, 20, "TOC header size")?;
    if header_size != 0x90 {
        bail!("invalid TOC header size {header_size:#x}; expected 0x90");
    }
    let entry_count = raw_u32(bytes, 24, "TOC entry count")? as usize;
    let compression_block_count = raw_u32(bytes, 28, "compression block count")? as usize;
    let compression_block_entry_size = raw_u32(bytes, 32, "compression block entry size")?;
    let compression_method_count = raw_u32(bytes, 36, "compression method count")? as usize;
    let compression_method_name_length = raw_u32(bytes, 40, "compression method name length")? as usize;
    let compression_block_size = raw_u32(bytes, 44, "compression block size")? as u64;
    let directory_index_size = raw_u32(bytes, 48, "directory index size")? as usize;
    let partition_count = raw_u32(bytes, 52, "partition count")?;
    let container_flags = bytes[80];
    let perfect_hash_count = raw_u32(bytes, 84, "perfect hash count")? as usize;
    let partition_size = raw_u64(bytes, 88, "partition size")?;
    let overflow_count = raw_u32(bytes, 96, "perfect hash overflow count")? as usize;

    if entry_count > MAX_TOC_ENTRIES {
        bail!("TOC entry count {entry_count} exceeds limit {MAX_TOC_ENTRIES}");
    }
    if compression_block_count > MAX_COMPRESSION_BLOCKS {
        bail!(
            "compression block count {compression_block_count} exceeds limit {MAX_COMPRESSION_BLOCKS}"
        );
    }
    if compression_block_entry_size != 12 {
        bail!("compression block entry size {compression_block_entry_size} is not 12");
    }
    if compression_method_count > MAX_COMPRESSION_METHODS {
        bail!(
            "compression method count {compression_method_count} exceeds limit {MAX_COMPRESSION_METHODS}"
        );
    }
    if compression_method_count > 0 && !(1..=64).contains(&compression_method_name_length) {
        bail!("invalid compression method name length {compression_method_name_length}");
    }
    if compression_block_size == 0
        || !compression_block_size.is_power_of_two()
        || compression_block_size > 16 * 1024 * 1024
    {
        bail!("invalid compression block size {compression_block_size}");
    }
    if directory_index_size > MAX_DIRECTORY_INDEX_BYTES {
        bail!(
            "directory index size {directory_index_size} exceeds limit {MAX_DIRECTORY_INDEX_BYTES}"
        );
    }
    if partition_count != 1 {
        bail!("unsupported partition count {partition_count}; bounded reader requires one UCAS");
    }
    if partition_size == 0 {
        bail!("partition size must be nonzero");
    }
    if container_flags & !0x0f != 0 {
        bail!("invalid IoStore container flags {container_flags:#x}");
    }
    if perfect_hash_count > entry_count || overflow_count > entry_count {
        bail!("perfect-hash counts exceed TOC entry count");
    }
    if (version < 4 && (perfect_hash_count != 0 || overflow_count != 0))
        || (version == 4 && overflow_count != 0)
    {
        bail!("perfect-hash counts are not valid for TOC version {version}");
    }
    Ok(RawTocHeader {
        version,
        entry_count,
        compression_block_count,
        compression_method_count,
        compression_method_name_length,
        compression_block_size,
        directory_index_size,
        container_flags,
        perfect_hash_count,
        overflow_count,
    })
}

fn raw_chunk_offset_length(bytes: &[u8]) -> (u64, u64) {
    let offset = u64::from_be_bytes([0, 0, 0, bytes[0], bytes[1], bytes[2], bytes[3], bytes[4]]);
    let length = u64::from_be_bytes([0, 0, 0, bytes[5], bytes[6], bytes[7], bytes[8], bytes[9]]);
    (offset, length)
}

fn raw_block(bytes: &[u8]) -> (u64, u64, u64, u8) {
    let offset = u64::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], 0, 0, 0]);
    let compressed = u32::from_le_bytes([bytes[5], bytes[6], bytes[7], 0]) as u64;
    let uncompressed = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], 0]) as u64;
    (offset, compressed, uncompressed, bytes[11])
}

fn preflight_toc_bytes(bytes: &[u8], ucas_len: u64) -> Result<u64> {
    let header = parse_raw_toc_header(bytes)?;
    let mut position = 0x90usize;
    let chunk_ids_range = checked_advance(
        &mut position,
        checked_product(header.entry_count, 12, "chunk id table")?,
        bytes.len(),
        "chunk id table",
    )?;
    let offsets_range = checked_advance(
        &mut position,
        checked_product(header.entry_count, 10, "chunk offset table")?,
        bytes.len(),
        "chunk offset table",
    )?;
    checked_advance(
        &mut position,
        checked_product(header.perfect_hash_count, 4, "perfect hash table")?,
        bytes.len(),
        "perfect hash table",
    )?;
    checked_advance(
        &mut position,
        checked_product(header.overflow_count, 4, "perfect hash overflow table")?,
        bytes.len(),
        "perfect hash overflow table",
    )?;
    let blocks_range = checked_advance(
        &mut position,
        checked_product(header.compression_block_count, 12, "compression block table")?,
        bytes.len(),
        "compression block table",
    )?;
    checked_advance(
        &mut position,
        checked_product(
            header.compression_method_count,
            header.compression_method_name_length,
            "compression method table",
        )?,
        bytes.len(),
        "compression method table",
    )?;

    if header.container_flags & 0x04 != 0 {
        let signature_size = raw_u32(bytes, position, "signature size")? as usize;
        checked_advance(&mut position, 4, bytes.len(), "signature size")?;
        if signature_size > 16 * 1024 * 1024 {
            bail!("signature size {signature_size} exceeds 16 MiB limit");
        }
        checked_advance(&mut position, signature_size, bytes.len(), "TOC signature")?;
        checked_advance(&mut position, signature_size, bytes.len(), "block signature")?;
        checked_advance(
            &mut position,
            checked_product(header.compression_block_count, 20, "chunk block signatures")?,
            bytes.len(),
            "chunk block signatures",
        )?;
    }
    let directory_range = checked_advance(
        &mut position,
        header.directory_index_size,
        bytes.len(),
        "directory index",
    )?;
    let meta_size = if header.version >= 8 { 24 } else { 33 };
    checked_advance(
        &mut position,
        checked_product(header.entry_count, meta_size, "chunk metadata")?,
        bytes.len(),
        "chunk metadata",
    )?;
    if position != bytes.len() {
        bail!("TOC has {} unparsed trailing bytes", bytes.len() - position);
    }

    if !directory_range.is_empty() {
        preflight_directory_index(&bytes[directory_range], header.entry_count)?;
    }

    let chunk_ids = &bytes[chunk_ids_range];
    let chunk_offsets = &bytes[offsets_range];
    let blocks = &bytes[blocks_range];
    let is_new_chunk_ids = header.version > 4;
    let max_chunk_type = if is_new_chunk_ids { 13 } else { 12 };
    let header_type = if is_new_chunk_ids { 6 } else { 10 };
    let mut unique_chunk_ids = HashSet::new();
    unique_chunk_ids
        .try_reserve(header.entry_count)
        .context("reserving bounded chunk-id set")?;
    let mut header_count = 0usize;
    let mut header_bytes = 0u64;

    for block in blocks.chunks_exact(12) {
        let (offset, compressed, uncompressed, method) = raw_block(block);
        if compressed == 0 || uncompressed == 0 {
            bail!("compression block has zero compressed or uncompressed size");
        }
        if uncompressed > header.compression_block_size {
            bail!(
                "compression block advertises {uncompressed} uncompressed bytes; block size is {}",
                header.compression_block_size
            );
        }
        if method as usize > header.compression_method_count {
            bail!("compression block method index {method} exceeds method table");
        }
        if method == 0 && compressed != uncompressed {
            bail!(
                "uncompressed method-0 block advertises compressed size {compressed} but reader consumes {uncompressed} bytes"
            );
        }
        let reader_bytes = if method == 0 { uncompressed } else { compressed };
        let on_disk = if header.container_flags & 0x02 != 0 {
            reader_bytes.checked_add(15).context("encrypted block alignment overflow")? & !15
        } else {
            reader_bytes
        };
        let end = offset.checked_add(on_disk).context("UCAS block range overflow")?;
        if end > ucas_len {
            bail!("compression block range {offset}..{end} exceeds UCAS length {ucas_len}");
        }
    }

    for (id, raw_range) in chunk_ids.chunks_exact(12).zip(chunk_offsets.chunks_exact(10)) {
        if id[11] > max_chunk_type {
            bail!("invalid chunk type {} for TOC version {}", id[11], header.version);
        }
        if !unique_chunk_ids.insert(id) {
            bail!("duplicate chunk id {}", hex::encode(id));
        }
        let (offset, length) = raw_chunk_offset_length(raw_range);
        if length > MAX_ADVERTISED_CHUNK_BYTES || usize::try_from(length).is_err() {
            bail!("chunk {} advertises {length} bytes; limit is {MAX_ADVERTISED_CHUNK_BYTES}", hex::encode(id));
        }
        if id[11] == header_type {
            header_count += 1;
            if header_count > 1 {
                bail!("container exposes more than one ContainerHeader chunk");
            }
            if length == 0 || length > MAX_CONTAINER_HEADER_CHUNK_BYTES {
                bail!("ContainerHeader chunk length {length} is outside 1..={MAX_CONTAINER_HEADER_CHUNK_BYTES}");
            }
            header_bytes = header_bytes
                .checked_add(length)
                .context("ContainerHeader metadata size overflow")?;
        }
        if length == 0 {
            continue;
        }
        let end = offset.checked_add(length).context("chunk virtual range overflow")?;
        let first = usize::try_from(offset / header.compression_block_size)?;
        let last = usize::try_from(
            end.checked_add(header.compression_block_size - 1)
                .context("chunk block alignment overflow")?
                / header.compression_block_size
                - 1,
        )?;
        if first > last || last >= header.compression_block_count {
            bail!("chunk {} references compression blocks {first}..={last} outside table", hex::encode(id));
        }
        let mut uncompressed_total = 0u64;
        for block in blocks[first * 12..(last + 1) * 12].chunks_exact(12) {
            uncompressed_total = uncompressed_total
                .checked_add(raw_block(block).2)
                .context("chunk block size sum overflow")?;
        }
        let allocation = length.checked_add(15).context("chunk allocation alignment overflow")? & !15;
        if uncompressed_total < length || uncompressed_total > allocation {
            bail!(
                "chunk {} block payload {uncompressed_total} is incompatible with advertised length {length}",
                hex::encode(id)
            );
        }
    }
    u64::try_from(bytes.len())?
        .checked_add(header_bytes)
        .context("container metadata size overflow")
}

fn preflight_directory_index(bytes: &[u8], toc_entry_count: usize) -> Result<()> {
    let mut cursor = RawDirectoryCursor::new(bytes);
    cursor.fstring(16 * 1024, "directory mount point")?;
    let directory_count = cursor.count(MAX_DIRECTORY_ENTRIES, "directory entries")?;
    let directory_bytes = cursor.take(
        checked_product(directory_count, 16, "directory entries")?,
        "directory entries",
    )?;
    let file_count = cursor.count(MAX_DIRECTORY_ENTRIES, "file entries")?;
    let file_bytes = cursor.take(
        checked_product(file_count, 12, "file entries")?,
        "file entries",
    )?;
    let string_count = cursor.count(MAX_DIRECTORY_ENTRIES, "directory strings")?;
    for _ in 0..string_count {
        cursor.fstring(16 * 1024, "directory string")?;
    }
    if cursor.position != bytes.len() {
        bail!("directory index has {} trailing bytes", bytes.len() - cursor.position);
    }
    if directory_count == 0 && file_count != 0 {
        bail!("directory index has files but no root directory");
    }

    let valid_optional = |value: u32, bound: usize| value == u32::MAX || (value as usize) < bound;
    for entry in directory_bytes.chunks_exact(16) {
        let name = u32::from_le_bytes(entry[0..4].try_into().expect("four bytes"));
        let child = u32::from_le_bytes(entry[4..8].try_into().expect("four bytes"));
        let sibling = u32::from_le_bytes(entry[8..12].try_into().expect("four bytes"));
        let file = u32::from_le_bytes(entry[12..16].try_into().expect("four bytes"));
        if !valid_optional(name, string_count)
            || !valid_optional(child, directory_count)
            || !valid_optional(sibling, directory_count)
            || !valid_optional(file, file_count)
        {
            bail!("directory entry contains an out-of-range index");
        }
    }
    for entry in file_bytes.chunks_exact(12) {
        let name = u32::from_le_bytes(entry[0..4].try_into().expect("four bytes"));
        let next = u32::from_le_bytes(entry[4..8].try_into().expect("four bytes"));
        let user_data = u32::from_le_bytes(entry[8..12].try_into().expect("four bytes"));
        if name as usize >= string_count
            || !valid_optional(next, file_count)
            || user_data as usize >= toc_entry_count
        {
            bail!("file entry contains an out-of-range index");
        }
    }

    if directory_count != 0 {
        validate_directory_graph(directory_bytes, file_bytes, directory_count, file_count)?;
    }
    Ok(())
}

fn validate_directory_graph(
    directories: &[u8],
    files: &[u8],
    directory_count: usize,
    file_count: usize,
) -> Result<()> {
    let mut seen_directories = vec![false; directory_count];
    let mut seen_files = vec![false; file_count];
    let mut stack = vec![(0usize, 0usize)];
    while let Some((index, depth)) = stack.pop() {
        if depth > MAX_DIRECTORY_DEPTH {
            bail!("directory graph depth exceeds {MAX_DIRECTORY_DEPTH}");
        }
        if seen_directories[index] {
            bail!("directory graph contains a cycle or duplicate link at {index}");
        }
        seen_directories[index] = true;
        let entry = &directories[index * 16..index * 16 + 16];
        let mut file = u32::from_le_bytes(entry[12..16].try_into().expect("four bytes"));
        while file != u32::MAX {
            let file_index = file as usize;
            if seen_files[file_index] {
                bail!("file-entry chain contains a cycle or duplicate link at {file_index}");
            }
            seen_files[file_index] = true;
            let file_entry = &files[file_index * 12..file_index * 12 + 12];
            file = u32::from_le_bytes(file_entry[4..8].try_into().expect("four bytes"));
        }

        let mut child = u32::from_le_bytes(entry[4..8].try_into().expect("four bytes"));
        let mut sibling_guard = HashSet::new();
        while child != u32::MAX {
            if !sibling_guard.insert(child) {
                bail!("directory sibling chain contains a cycle at {child}");
            }
            let child_index = child as usize;
            stack.push((child_index, depth + 1));
            let child_entry = &directories[child_index * 16..child_index * 16 + 16];
            child = u32::from_le_bytes(child_entry[8..12].try_into().expect("four bytes"));
        }
    }
    Ok(())
}

struct RawDirectoryCursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> RawDirectoryCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take(&mut self, length: usize, label: &'static str) -> Result<&'a [u8]> {
        let range = checked_advance(&mut self.position, length, self.bytes.len(), label)?;
        Ok(&self.bytes[range])
    }

    fn u32(&mut self, label: &'static str) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4, label)?.try_into().expect("four bytes")))
    }

    fn i32(&mut self, label: &'static str) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4, label)?.try_into().expect("four bytes")))
    }

    fn count(&mut self, limit: usize, label: &'static str) -> Result<usize> {
        let count = self.u32(label)? as usize;
        if count > limit {
            bail!("{label} count {count} exceeds limit {limit}");
        }
        Ok(count)
    }

    fn fstring(&mut self, limit: usize, label: &'static str) -> Result<()> {
        let length = self.i32(label)?;
        let units = if length < 0 {
            length.checked_abs().with_context(|| format!("{label} length is i32::MIN"))? as usize
        } else {
            length as usize
        };
        let byte_count = units
            .checked_mul(if length < 0 { 2 } else { 1 })
            .with_context(|| format!("{label} size overflow"))?;
        if byte_count > limit {
            bail!("{label} byte length {byte_count} exceeds limit {limit}");
        }
        self.take(byte_count, label)?;
        Ok(())
    }
}

pub trait IoStoreTrait: Send + Sync {
    fn container_name(&self) -> &str;
    fn container_file_version(&self) -> Option<EIoStoreTocVersion>;
    fn container_header_version(&self) -> Option<EIoContainerHeaderVersion>;
    fn print_info(&self, depth: usize);

    fn read(&self, chunk_id: FIoChunkId) -> Result<Vec<u8>>;
    fn read_raw(&self, chunk_id_raw: FIoChunkIdRaw) -> Result<Vec<u8>>;
    fn has_chunk_id(&self, chunk_id: FIoChunkId) -> bool;
    fn has_chunk_id_raw(&self, chunk_id_raw: FIoChunkIdRaw) -> bool;
    fn chunks(&self) -> Box<dyn Iterator<Item = ChunkInfo<'_>> + Send + '_>;
    fn chunks_all(&self) -> Box<dyn Iterator<Item = ChunkInfo<'_>> + Send + '_>;
    fn packages(&self) -> Box<dyn Iterator<Item = PackageInfo<'_>> + Send + '_>;
    fn packages_all(&self) -> Box<dyn Iterator<Item = PackageInfo<'_>> + Send + '_>;
    fn child_containers(&self) -> Box<dyn Iterator<Item = &dyn IoStoreTrait> + '_>;
    /// Get absolute path (including mount point) if it has one
    fn chunk_path(&self, chunk_id: FIoChunkId) -> Option<String>;
    fn package_store_entry(&self, package_id: FPackageId) -> Option<StoreEntry>;
    fn lookup_package_redirect(&self, source_package_id: FPackageId) -> Option<FPackageId>;

    fn load_script_objects(&self) -> Result<ZenScriptObjects> {
        if self.container_file_version().unwrap() > EIoStoreTocVersion::PerfectHash {
            let script_objects_data = self.read(FIoChunkId::create(0, 0, EIoChunkType::ScriptObjects))?;
            ZenScriptObjects::deserialize_new(&mut Cursor::new(script_objects_data))
        } else {
            let script_objects_data = self.read(FIoChunkId::create(0, 0, EIoChunkType::LoaderInitialLoadMeta))?;
            let names = self.read(FIoChunkId::create(0, 0, EIoChunkType::LoaderGlobalNames))?;
            ZenScriptObjects::deserialize_old(&mut Cursor::new(script_objects_data), &names)
        }
    }
}

#[derive(Clone, Copy)]
pub struct ChunkInfo<'a> {
    id: FIoChunkId,
    container: &'a IoStoreContainer,
    size: u64,
}
impl ChunkInfo<'_> {
    pub fn id(&self) -> FIoChunkId {
        self.id
    }
    pub fn container(&self) -> &IoStoreContainer {
        self.container
    }
    pub fn size(&self) -> u64 {
        self.size
    }
    pub fn path(&self) -> Option<String> {
        self.container.chunk_path(self.id)
    }
    fn toc_index(&self) -> u32 {
        *self.container.toc.chunk_id_map.get(&self.id).unwrap()
    }
    pub fn hash(&self) -> &FIoChunkHash {
        &self.container.toc.chunk_metas[self.toc_index() as usize].chunk_hash
    }
    pub fn read(&self) -> Result<Vec<u8>> {
        self.container.read(self.id)
    }
}
impl std::cmp::Eq for ChunkInfo<'_> {}
impl std::cmp::PartialEq for ChunkInfo<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}
impl std::hash::Hash for ChunkInfo<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

#[derive(Clone, Copy)]
pub struct PackageInfo<'a> {
    id: FPackageId,
    container: &'a IoStoreContainer,
}
impl PackageInfo<'_> {
    pub fn id(&self) -> FPackageId {
        self.id
    }
    pub fn container(&self) -> &IoStoreContainer {
        self.container
    }
}
impl std::cmp::Eq for PackageInfo<'_> {}
impl std::cmp::PartialEq for PackageInfo<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}
impl std::hash::Hash for PackageInfo<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

struct IoStoreBackend {
    containers: Vec<Box<dyn IoStoreTrait>>,
}
impl IoStoreBackend {
    #[allow(unused)]
    pub fn new() -> Result<Self> {
        Ok(Self { containers: vec![] })
    }
    pub fn open<P: AsRef<Path>>(dir: P, config: Arc<Config>) -> Result<Self> {
        let mut containers: Vec<Box<dyn IoStoreTrait>> = vec![];
        let mut aggregate_toc_bytes = 0u64;
        let mut aggregate_metadata_bytes = 0u64;
        for entry in fs::read_dir(dir.as_ref())? {
            let entry = entry?;
            let path = entry.path();
            if path.extension() == Some(OsStr::new("utoc")) {
                if containers.len() >= MAX_COMPOSITE_CONTAINERS {
                    bail!(
                        "IoStore directory contains more than {MAX_COMPOSITE_CONTAINERS} containers"
                    );
                }
                let metadata = fs::symlink_metadata(&path)?;
                if !metadata.is_file() || metadata.file_type().is_symlink() {
                    bail!("IoStore TOC is not a plain file: {}", path.display());
                }
                aggregate_toc_bytes = aggregate_toc_bytes
                    .checked_add(metadata.len())
                    .context("aggregate IoStore TOC size overflow")?;
                if aggregate_toc_bytes > MAX_COMPOSITE_TOC_BYTES {
                    bail!(
                        "aggregate IoStore TOC size {aggregate_toc_bytes} exceeds limit {MAX_COMPOSITE_TOC_BYTES}"
                    );
                }
                let container = IoStoreContainer::open(path, config.clone())?;
                aggregate_metadata_bytes = aggregate_metadata_bytes
                    .checked_add(container.metadata_bytes)
                    .context("aggregate IoStore metadata size overflow")?;
                if aggregate_metadata_bytes > MAX_COMPOSITE_METADATA_BYTES {
                    bail!(
                        "aggregate IoStore metadata size {aggregate_metadata_bytes} exceeds limit {MAX_COMPOSITE_METADATA_BYTES}"
                    );
                }
                containers.push(Box::new(container));
            }
        }

        // Header chunks key the package metadata for each child container. A
        // duplicate would make the composite winner implicit and could bind a
        // receipt to different metadata after a sibling appears or is renamed.
        ensure_unique_container_header_ids(containers.iter().flat_map(|container| {
            container
                .chunks()
                .filter(|chunk| chunk.id().get_chunk_type() == EIoChunkType::ContainerHeader)
                .map(|chunk| chunk.id())
        }))?;
        // Validate that all containers are of the same version
        let mut previous_container_version: Option<EIoStoreTocVersion> = None;
        let mut previous_container_name: String = String::new();
        let mut previous_header_container_version: Option<EIoContainerHeaderVersion> = None;
        let mut previous_header_container_name: String = String::new();

        for container in &containers {
            let this_container_version = container.container_file_version().unwrap();
            let this_container_name = container.container_name().to_string();

            // Check that container Table Of Contents version matches the previous container
            if previous_container_version.is_none() {
                previous_container_name = this_container_name.clone();
                previous_container_version = Some(this_container_version);
            }
            if this_container_version != previous_container_version.unwrap() {
                bail!(
                    "Cannot create composite container for containers of different versions: Container {} and {} have different versions {:?} and {:?}",
                    previous_container_name,
                    this_container_name,
                    previous_container_version.unwrap(),
                    this_container_version
                );
            }

            // Check that container header version matches the previous container
            if let Some(this_container_header_version) = container.container_header_version() {
                if previous_header_container_version.is_none() {
                    previous_header_container_name = this_container_name.clone();
                    previous_header_container_version = Some(this_container_header_version);
                }
                if this_container_header_version != previous_header_container_version.unwrap() {
                    bail!(
                        "Cannot create composite container for containers of different header versions: Container {} and {} have different versions {:?} and {:?}",
                        previous_header_container_name,
                        this_container_name,
                        previous_header_container_version.unwrap(),
                        this_container_header_version
                    );
                }
            }
        }

        containers.sort_by(|a, b| sort_container_name(b.container_name()).cmp(&sort_container_name(a.container_name())));
        Ok(Self { containers })
    }
}
impl IoStoreTrait for IoStoreBackend {
    fn container_name(&self) -> &str {
        "VIRTUAL"
    }
    fn container_file_version(&self) -> Option<EIoStoreTocVersion> {
        self.containers.first().and_then(|x| x.container_file_version())
    }
    fn container_header_version(&self) -> Option<EIoContainerHeaderVersion> {
        // Some containers might not have a container header, so take the first container with a header
        self.containers.iter().find_map(|x| x.container_header_version())
    }
    fn print_info(&self, mut depth: usize) {
        indent_println!(depth, "{}", self.container_name());
        depth += 1;

        if self.child_containers().count() != 0 {
            indent_println!(depth, "child containers ({}):", self.containers.len());
            for container in self.child_containers() {
                container.print_info(depth + 1);
            }
        }
    }
    fn read(&self, mut chunk_id: FIoChunkId) -> Result<Vec<u8>> {
        if let Some(version) = self.container_file_version() {
            chunk_id = chunk_id.with_version(version);
        }
        self.containers.iter().find(|c| c.has_chunk_id(chunk_id)).with_context(|| format!("{chunk_id:?} not found in any containers"))?.read(chunk_id)
    }
    fn read_raw(&self, chunk_id_raw: FIoChunkIdRaw) -> Result<Vec<u8>> {
        self.containers.iter().find(|c| c.has_chunk_id_raw(chunk_id_raw)).with_context(|| format!("{chunk_id_raw:?} not found in any containers"))?.read_raw(chunk_id_raw)
    }
    fn has_chunk_id(&self, chunk_id: FIoChunkId) -> bool {
        self.containers.iter().any(|c| c.has_chunk_id(chunk_id))
    }
    fn has_chunk_id_raw(&self, chunk_id_raw: FIoChunkIdRaw) -> bool {
        self.containers.iter().any(|c| c.has_chunk_id_raw(chunk_id_raw))
    }
    fn chunks(&self) -> Box<dyn Iterator<Item = ChunkInfo<'_>> + Send + '_> {
        Box::new(UniqueIterator::new(self.chunks_all()))
    }
    fn chunks_all(&self) -> Box<dyn Iterator<Item = ChunkInfo<'_>> + Send + '_> {
        Box::new(self.containers.iter().flat_map(|c| c.chunks_all()))
    }
    fn packages(&self) -> Box<dyn Iterator<Item = PackageInfo<'_>> + Send + '_> {
        Box::new(self.containers.iter().flat_map(|c| c.packages()))
    }
    fn packages_all(&self) -> Box<dyn Iterator<Item = PackageInfo<'_>> + Send + '_> {
        Box::new(UniqueIterator::new(self.containers.iter().flat_map(|c| c.packages())))
    }
    fn child_containers(&self) -> Box<dyn Iterator<Item = &dyn IoStoreTrait> + '_> {
        Box::new(self.containers.iter().map(Box::as_ref))
    }
    fn chunk_path(&self, chunk_id: FIoChunkId) -> Option<String> {
        self.containers.iter().find_map(|c| c.chunk_path(chunk_id))
    }
    fn package_store_entry(&self, package_id: FPackageId) -> Option<StoreEntry> {
        self.containers.iter().find_map(|c| c.package_store_entry(package_id))
    }
    fn lookup_package_redirect(&self, source_package_id: FPackageId) -> Option<FPackageId> {
        self.containers.iter().find_map(|c| c.lookup_package_redirect(source_package_id))
    }
}

pub struct IoStoreContainer {
    name: String,
    #[allow(unused)]
    path: PathBuf,
    toc: Toc,
    cas: FilePool,
    metadata_bytes: u64,

    container_header: Option<FIoContainerHeader>,
}
impl IoStoreContainer {
    pub fn open<P: AsRef<Path>>(toc_path: P, config: Arc<Config>) -> Result<Self> {
        let path = toc_path.as_ref().to_path_buf();
        let toc_metadata = fs::symlink_metadata(&path)?;
        if !toc_metadata.is_file() || toc_metadata.file_type().is_symlink() {
            bail!("IoStore TOC is not a plain file: {}", path.display());
        }
        if toc_metadata.len() > MAX_TOC_BYTES {
            bail!(
                "IoStore TOC is {} bytes; limit is {MAX_TOC_BYTES}",
                toc_metadata.len()
            );
        }
        let ucas_path = path.with_extension("ucas");
        let ucas_metadata = fs::symlink_metadata(&ucas_path)?;
        if !ucas_metadata.is_file() || ucas_metadata.file_type().is_symlink() {
            bail!("IoStore UCAS is not a plain file: {}", ucas_path.display());
        }
        if ucas_metadata.len() > MAX_UCAS_BYTES {
            bail!(
                "IoStore UCAS is {} bytes; limit is {MAX_UCAS_BYTES}",
                ucas_metadata.len()
            );
        }

        let toc_len = usize::try_from(toc_metadata.len())?;
        let mut toc_bytes = Vec::new();
        toc_bytes
            .try_reserve_exact(toc_len)
            .context("reserving bounded TOC buffer")?;
        let toc_file = fs::File::open(&path)?;
        toc_file
            .take(toc_metadata.len().saturating_add(1))
            .read_to_end(&mut toc_bytes)?;
        if toc_bytes.len() != toc_len {
            bail!(
                "IoStore TOC changed while reading: expected {toc_len} bytes, got {}",
                toc_bytes.len()
            );
        }
        let metadata_bytes = preflight_toc_bytes(&toc_bytes, ucas_metadata.len())
            .with_context(|| format!("preflighting {}", path.display()))?;
        let toc: Toc = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Cursor::new(&toc_bytes).de_ctx(config.clone())
        }))
        .map_err(|_| anyhow::anyhow!("TOC parser panicked after bounded preflight"))??;
        let cas = FilePool::new(&ucas_path, rayon::max_num_threads())?;

        let mut container = Self {
            name: path.file_stem().context("failed to get container name")?.to_string_lossy().into(),
            path,
            toc,
            cas,
            metadata_bytes,

            container_header: None,
        };

        // TODO avoid linear search for header
        // TODO populate header lazily?
        let header_chunk = container.chunks().find(|info| info.id().get_chunk_type() == EIoChunkType::ContainerHeader);
        if let Some(header_chunk) = header_chunk {
            let chunk_id = header_chunk.id();
            let data = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                container.read(chunk_id)
            }))
            .map_err(|_| anyhow::anyhow!("ContainerHeader chunk read panicked after bounded TOC preflight"))??;
            preflight_container_header(&data, config.container_header_version_override)
                .with_context(|| format!("preflighting ContainerHeader {chunk_id:?}"))?;
            let header = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                FIoContainerHeader::deserialize(
                    &mut std::io::Cursor::new(&data),
                    config.container_header_version_override,
                )
            }))
            .map_err(|_| anyhow::anyhow!("ContainerHeader parser panicked after bounded preflight"))??;
            container.container_header = Some(header);
        }

        Ok(container)
    }
    #[allow(unused)]
    pub fn container_path(&self) -> &Path {
        self.path.as_ref()
    }
}
impl IoStoreTrait for IoStoreContainer {
    fn container_name(&self) -> &str {
        &self.name
    }
    fn container_file_version(&self) -> Option<EIoStoreTocVersion> {
        Some(self.toc.version)
    }
    fn container_header_version(&self) -> Option<EIoContainerHeaderVersion> {
        self.container_header.as_ref().map(|x| x.version)
    }
    fn print_info(&self, mut depth: usize) {
        indent_println!(depth, "{}", self.container_name());
        depth += 1;

        indent_println!(depth, "container_id: {:x?}", self.toc.container_id);
        indent_println!(depth, "container_flags: {:?}", self.toc.container_flags);
        indent_println!(depth, "version: {:?}", self.toc.version);
        let mount_point = &self.toc.directory_index.mount_point;
        if !mount_point.as_str().is_empty() {
            indent_println!(depth, "mount_point: {}", mount_point);
        }
        indent_println!(depth, "chunks: {}", self.toc.chunks.len());
        indent_println!(depth, "packages: {}", self.packages().count());
        // assumes header has already been parsed
        indent_println!(depth, "container_header_version: {:?}", self.container_header.as_ref().map(|h| h.version));
        indent_println!(depth, "compression_methods: {:?}", self.toc.compression_methods);
    }
    fn read(&self, chunk_id: FIoChunkId) -> Result<Vec<u8>> {
        let chunk_id = chunk_id.with_version(self.toc.version);
        let index = *self.toc.chunk_id_map.get(&chunk_id).with_context(|| format!("container {:?} does not contain {:?}", self.name, chunk_id))?;
        let mut file_lock = self.cas.acquire()?;
        self.toc.read(&mut file_lock.file(), index).with_context(|| format!("Failed to read chunk {chunk_id:?}"))
    }
    fn read_raw(&self, chunk_id_raw: FIoChunkIdRaw) -> Result<Vec<u8>> {
        self.read(FIoChunkId::from_raw(chunk_id_raw, self.toc.version))
    }
    fn has_chunk_id(&self, chunk_id: FIoChunkId) -> bool {
        self.toc.chunk_id_map.contains_key(&chunk_id.with_version(self.toc.version))
    }
    fn has_chunk_id_raw(&self, chunk_id_raw: FIoChunkIdRaw) -> bool {
        self.has_chunk_id(FIoChunkId::from_raw(chunk_id_raw, self.toc.version))
    }
    fn chunks(&self) -> Box<dyn Iterator<Item = ChunkInfo<'_>> + Send + '_> {
        // chunks should already be unique in individual containers
        self.chunks_all()
    }
    fn chunks_all(&self) -> Box<dyn Iterator<Item = ChunkInfo<'_>> + Send + '_> {
        Box::new(self.toc.chunks.iter().zip(&self.toc.chunk_offset_lengths).map(|(&id, offset_and_length)| ChunkInfo {
            id,
            container: self,
            size: offset_and_length.get_length(),
        }))
    }
    fn packages(&self) -> Box<dyn Iterator<Item = PackageInfo<'_>> + Send + '_> {
        // packages should already be unique in individual containers
        self.packages_all()
    }
    fn packages_all(&self) -> Box<dyn Iterator<Item = PackageInfo<'_>> + Send + '_> {
        Box::new(self.container_header.iter().flat_map(|header| header.package_ids()).map(|id| PackageInfo { id, container: self }))
    }
    fn child_containers(&self) -> Box<dyn Iterator<Item = &dyn IoStoreTrait> + '_> {
        Box::new(std::iter::empty())
    }
    fn chunk_path(&self, chunk_id: FIoChunkId) -> Option<String> {
        self.toc.file_name(chunk_id)
    }
    fn package_store_entry(&self, package_id: FPackageId) -> Option<StoreEntry> {
        self.container_header.as_ref().and_then(|header| header.get_store_entry(package_id))
    }
    fn lookup_package_redirect(&self, source_package_id: FPackageId) -> Option<FPackageId> {
        self.container_header.as_ref().and_then(|header| header.lookup_package_redirect(source_package_id))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sort_container() {
        let mut containers = [
            "pakchunk0-Windows",
            "pakchunk10-Windows",
            "pakchunk11-Windows",
            "pakchunk12-Windows",
            "pakchunk13-Windows",
            "pakchunk14-Windows",
            "pakchunk15-Windows",
            "global",
            "pakchunk16-Windows",
            "pakchunk17-Windows",
            "pakchunk1optional-Windows",
            "pakchunk1-Windows",
            "pakchunk8-Windows_1_P",
            "pakchunk3-Windows",
            "pakchunk8-Windows_P",
            "pakchunk6-Windows",
            "pakchunk0optional-Windows",
            "pakchunk9-Windows",
            "pakchunk8-Windows_0_P",
            "pakchunk4-Windows",
            "pakchunk2-Windows",
            "pakchunk5-Windows",
            "pakchunk7-Windows",
        ];
        containers.sort_by(|a, b| sort_container_name(b).cmp(&sort_container_name(a)));
        //for container in containers {
        //eprintln!("{:?}", sort_container_name(container));
        //}
        assert_eq!(
            containers,
            [
                "global",
                "pakchunk8-Windows_1_P",
                "pakchunk8-Windows_0_P",
                "pakchunk8-Windows_P",
                "pakchunk9-Windows",
                "pakchunk7-Windows",
                "pakchunk6-Windows",
                "pakchunk5-Windows",
                "pakchunk4-Windows",
                "pakchunk3-Windows",
                "pakchunk2-Windows",
                "pakchunk1optional-Windows",
                "pakchunk17-Windows",
                "pakchunk16-Windows",
                "pakchunk15-Windows",
                "pakchunk14-Windows",
                "pakchunk13-Windows",
                "pakchunk12-Windows",
                "pakchunk11-Windows",
                "pakchunk10-Windows",
                "pakchunk1-Windows",
                "pakchunk0optional-Windows",
                "pakchunk0-Windows",
            ]
        );
    }
}
