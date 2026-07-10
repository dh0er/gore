use crate::{
    EIoChunkType, FPackageId, UEPath, UEPathBuf, align_u64, align_usize,
    chunk_id::FIoChunkIdRaw,
    container_header::{EIoContainerHeaderVersion, FIoContainerHeader, StoreEntry},
};
use crate::compression::{CompressionMethod, compress};
use crate::{EIoStoreTocVersion, FIoChunkHash, FIoChunkId, FIoContainerId, FIoOffsetAndLength, FIoStoreTocCompressedBlockEntry, FIoStoreTocEntryMeta, FIoStoreTocEntryMetaFlags, Toc, ser::*};
use anyhow::{Context, Result};
use fs_err as fs;
use std::io::Cursor;
use std::{
    io::{BufWriter, Seek, Write},
    path::{Path, PathBuf},
};

pub struct IoStoreWriter {
    #[allow(unused)]
    toc_path: PathBuf,
    toc_stream: BufWriter<fs::File>,
    cas_stream: BufWriter<fs::File>,
    toc: Toc,
    container_header: Option<FIoContainerHeader>,
    /// When `true`, `write_chunk` runs the per-block Oodle (Kraken) compression
    /// path (16-aligned offsets, `Compressed` container flag). When `false`
    /// (the default), every block is written RAW with method index 0, producing
    /// an uncompressed container (`container_flags = 8`, no 16-alignment) -- the
    /// pre-compression behaviour that is proven to load in-game. Toggle via
    /// [`IoStoreWriter::set_compress`].
    compress: bool,
}

impl IoStoreWriter {
    pub fn new<P: AsRef<Path>>(toc_path: P, toc_version: EIoStoreTocVersion, container_header_version: Option<EIoContainerHeaderVersion>, mount_point: UEPathBuf) -> Result<Self> {
        let toc_path = toc_path.as_ref().to_path_buf();
        let name = toc_path.file_stem().unwrap().to_string_lossy();
        let toc_stream = BufWriter::new(fs::File::create(&toc_path)?);
        let cas_stream = BufWriter::new(fs::File::create(toc_path.with_extension("ucas"))?);

        let mut toc = Toc::new();
        toc.compression_block_size = 0x10000;
        // Register Oodle (Kraken) as compression method index 1. Index 0 is the
        // implicit "None" (raw) method that UE always recognises and is never
        // listed in the name table. `write_chunk` emits Oodle-compressed blocks
        // with method index 1 (falling back to 0/raw per block when compression
        // does not shrink the block), matching the base game's container which
        // registers exactly one method name, "Oodle".
        toc.compression_methods = vec![CompressionMethod::Oodle];
        toc.version = toc_version;
        toc.container_id = FIoContainerId::from_name(&name);
        toc.directory_index.mount_point = mount_point;
        toc.partition_size = u64::MAX;

        let container_header = container_header_version.map(|v| FIoContainerHeader::new(v, toc.container_id));

        Ok(Self {
            toc_path,
            toc_stream,
            cas_stream,
            toc,
            container_header,
            // Default OFF: write raw/uncompressed blocks, the path proven to
            // load in-game. The compression path is opt-in via
            // `set_compress(true)`; it now mirrors the base game's writer
            // conventions (raw ContainerHeader/shader/script chunks, 1 KiB
            // admission threshold, 16-aligned blocks) -- see `write_chunk`.
            compress: false,
        })
    }
    /// Enable (or disable) per-block Oodle compression for subsequently-written
    /// chunks. Off by default; see the `compress` field. Returns `self` for
    /// builder-style chaining.
    pub fn set_compress(mut self, compress: bool) -> Self {
        self.compress = compress;
        self
    }
    pub fn write_chunk_raw(&mut self, chunk_id_raw: FIoChunkIdRaw, path: Option<&UEPath>, data: &[u8]) -> Result<()> {
        self.write_chunk(FIoChunkId::from_raw(chunk_id_raw, self.toc.version), path, data)
    }
    pub fn write_chunk(&mut self, chunk_id: FIoChunkId, path: Option<&UEPath>, data: &[u8]) -> Result<()> {
        if let Some(path) = path {
            let index = &mut self.toc.directory_index;
            let relative_path = path.strip_prefix(&index.mount_point).with_context(|| format!("mount point {} does not contain path {path}", index.mount_point))?;
            index.add_file(relative_path, self.toc.chunks.len() as u32);
        }

        let mut offset = self.cas_stream.stream_position()?;

        let start_block = self.toc.compression_blocks.len();

        // Method index 1 == Oodle (registered in `new`). Index 0 stays "None" (raw).
        const OODLE_METHOD_INDEX: u8 = 1;
        // Epic's writer never compresses a block whose uncompressed size is at
        // or below 1 KiB: across the base container's 948,727 compressed blocks
        // the minimum uncompressed size is exactly 1025. Stay inside that
        // envelope -- the runtime has never been exercised on a tiny compressed
        // block.
        const MIN_COMPRESSIBLE_BLOCK: usize = 1025;

        // Chunk types the engine reads assuming raw storage. The shipped base
        // container stores EVERY block of these chunk types with method 0 even
        // though the container itself is flagged Compressed (and the 7 MB
        // ContainerHeader would compress heavily). The fatal case is the
        // ContainerHeader: at mount time the runtime fetches
        // align(uncompressed_length, 16) bytes of it in ONE read starting at the
        // chunk's first block offset. The header is the LAST chunk in the .ucas
        // (written by `finalize`), so a compressed -- smaller on disk -- header
        // makes that read run past end-of-file; the read fails and the game
        // silently ignores the entire container. Forcing these chunk types raw
        // reproduces the base game's layout and keeps that mount read in-bounds.
        let force_raw = matches!(
            chunk_id.get_chunk_type(),
            EIoChunkType::ContainerHeader
                | EIoChunkType::ScriptObjects
                | EIoChunkType::ShaderCodeLibrary
                | EIoChunkType::ShaderCode
        );

        let mut hasher = blake3::Hasher::new();
        let mut any_block_compressed = false;
        // The blake3 chunk hash is over the *uncompressed* chunk bytes -- it
        // identifies the logical chunk, independent of how blocks are stored.
        for block in data.chunks(self.toc.compression_block_size as usize) {
            hasher.update(block);
            let uncompressed_size = block.len() as u32;

            if self.compress {
                // OPT-IN compression path. Try Oodle/Kraken (via gore-oodle) for
                // this block, keeping the compressed bytes only when they shrink
                // it (standard IoStore per-block raw fallback to method 0).
                // Blocks of force-raw chunk types and blocks below the 1 KiB
                // admission threshold skip the attempt and are stored raw (but
                // still 16-aligned), matching the base game's writer.
                let try_compress = !force_raw && block.len() >= MIN_COMPRESSIBLE_BLOCK;
                let mut compressed: Vec<u8> = Vec::new();
                if try_compress {
                    compress(CompressionMethod::Oodle, block, &mut compressed)?;
                }

                let (payload, compression_method_index): (&[u8], u8) = if try_compress && compressed.len() < block.len() {
                    any_block_compressed = true;
                    (&compressed, OODLE_METHOD_INDEX)
                } else {
                    (block, 0)
                };

                self.cas_stream.write_all(payload)?;
                let compressed_size = payload.len() as u32;
                // The block entry records the real (unpadded) compressed size and an
                // offset that is 16-aligned (the base game's convention). UE mis-reads
                // every block after the first if offsets are not 16-aligned, so pad the
                // cas stream with zero bytes up to the next 16-byte boundary and advance
                // `offset` to the aligned value. The first block already lands on an
                // aligned offset; padding keeps every subsequent block aligned too.
                // Alignment is GATED to the compressed path: an uncompressed
                // container (below) must match the pre-compression byte layout.
                self.toc.compression_blocks.push(FIoStoreTocCompressedBlockEntry::new(offset, compressed_size, uncompressed_size, compression_method_index));
                let aligned_size = align_usize(payload.len(), 16);
                let pad = aligned_size - payload.len();
                if pad > 0 {
                    self.cas_stream.write_all(&vec![0u8; pad])?;
                }
                offset += aligned_size as u64;
            } else {
                // DEFAULT raw path (pre-compression behaviour). Write the block
                // verbatim with method index 0, no compression, no 16-alignment.
                // The container reports `container_flags = 8` (Indexed only),
                // since no block carries a non-zero method index.
                self.cas_stream.write_all(block)?;
                let block_size = block.len() as u32;
                self.toc.compression_blocks.push(FIoStoreTocCompressedBlockEntry::new(offset, block_size, uncompressed_size, 0));
                offset += block.len() as u64;
            }
        }
        let hash = hasher.finalize();
        // UE's FIoStoreTocEntryMeta::Compressed flag marks chunks that have at
        // least one compressed block; the decode path is driven per-block by the
        // block's method index, but the flag keeps chunk-info/`force_uncompressed`
        // accounting correct and matches what the engine writes.
        let flags = if any_block_compressed { FIoStoreTocEntryMetaFlags::Compressed } else { FIoStoreTocEntryMetaFlags::empty() };
        let meta = FIoStoreTocEntryMeta {
            chunk_hash: FIoChunkHash::from_blake3(hash.as_bytes()),
            flags,
        };

        let offset_and_length = FIoOffsetAndLength::new(start_block as u64 * self.toc.compression_block_size as u64, data.len() as u64);

        self.toc.chunks.push(chunk_id.with_version(self.toc.version));
        self.toc.chunk_offset_lengths.push(offset_and_length);
        self.toc.chunk_metas.push(meta);

        Ok(())
    }

    pub fn write_package_chunk(&mut self, chunk_id: FIoChunkId, path: Option<&UEPath>, data: &[u8], store_entry: &StoreEntry) -> Result<()> {
        let container_header = self.container_header.as_mut().expect("FIoContainerHeader is required to write package chunks");
        container_header.add_package(FPackageId(chunk_id.get_chunk_id()), store_entry.clone());
        self.write_chunk(chunk_id, path, data)
    }
    pub fn add_localized_package(&mut self, package_culture: &str, source_package_name: &str, localized_package_id: FPackageId) -> Result<()> {
        let container_header = self.container_header.as_mut().expect("FIoContainerHeader is required to add localized packages");
        container_header.add_localized_package(package_culture, source_package_name, localized_package_id)
    }
    pub fn add_package_redirect(&mut self, source_package_name: &str, redirect_package_id: FPackageId) -> Result<()> {
        let container_header = self.container_header.as_mut().expect("FIoContainerHeader is required to add package redirects");
        container_header.add_package_redirect(source_package_name, redirect_package_id)
    }
    pub fn container_version(&self) -> EIoStoreTocVersion {
        self.toc.version
    }
    pub fn container_header_version(&self) -> EIoContainerHeaderVersion {
        self.container_header.as_ref().unwrap().version
    }
    pub fn finalize(mut self) -> Result<()> {
        if let Some(container_header) = &self.container_header {
            let mut chunk_buffer = vec![];
            container_header.serialize(&mut Cursor::new(&mut chunk_buffer))?;
            // container header is always aligned for AES for some reason
            chunk_buffer.resize(align_usize(chunk_buffer.len(), 16), 0);

            let chunk_id = FIoChunkId::create(container_header.container_id.0, 0, EIoChunkType::ContainerHeader);
            self.write_chunk(chunk_id, None, &chunk_buffer)?;
        }
        self.toc_stream.ser(&self.toc)?;
        Ok(())
    }
}

/// Read a written `.utoc` back and return its container-level flags plus, for
/// every compressed block (method index != 0), its stored `.ucas` offset. Used
/// by callers (and tests) to assert that a compressed container is flagged
/// `Compressed` and that every compressed block offset is 16-aligned. The
/// per-block accessors are `pub(crate)`, so this lives in retoc where it can
/// reach them.
pub fn dump_compressed_layout<P: AsRef<Path>>(toc_path: P) -> Result<(u8, Vec<u64>)> {
    use crate::ser::ReadExt;
    let bytes = fs::read(toc_path.as_ref())?;
    let toc: Toc = Cursor::new(bytes).de()?;
    let flags = toc.container_flags.bits();
    let compressed_offsets = toc
        .compression_blocks
        .iter()
        .filter(|b| b.get_compression_method_index() != 0)
        .map(|b| b.get_offset())
        .collect();
    Ok((flags, compressed_offsets))
}

/// Assert the mount-time read invariants the GAME enforces on a written
/// container. retoc's own [`Toc::read`] is deliberately lenient -- it fetches
/// each block by its COMPRESSED size -- so a round-trip through our reader
/// cannot catch layout the runtime rejects. The shipping runtime instead:
///
/// 1. reads the ContainerHeader chunk with ONE read of
///    `align(uncompressed_length, 16)` bytes at the chunk's first block file
///    offset; if that span runs past the end of the `.ucas` the read fails and
///    the container is silently ignored (no packages registered, no crash);
/// 2. advances through compressed blocks by `align(compressed_size, 16)`, so a
///    compressed block must sit at a 16-aligned offset and its padded span must
///    be in-bounds;
/// 3. has only ever seen special chunks (ContainerHeader / ScriptObjects /
///    ShaderCodeLibrary / ShaderCode) stored raw, and never a compressed block
///    with uncompressed size <= 1024 (base-container census over 992,548
///    blocks) -- stay inside that envelope.
///
/// The per-block/per-chunk accessors are `pub(crate)`, so this lives in retoc
/// (next to [`dump_compressed_layout`]) where tests and callers can reach it.
pub fn verify_game_mount_invariants<P: AsRef<Path>>(toc_path: P) -> Result<()> {
    use crate::ser::ReadExt;
    use anyhow::ensure;
    let toc_path = toc_path.as_ref();
    let toc: Toc = Cursor::new(fs::read(toc_path)?).de()?;
    let ucas_len = fs::metadata(toc_path.with_extension("ucas"))?.len();
    let block_size = toc.compression_block_size as u64;

    for (ci, chunk) in toc.chunks.iter().enumerate() {
        let ol = &toc.chunk_offset_lengths[ci];
        let chunk_type = chunk.get_chunk_type();
        let force_raw = matches!(
            chunk_type,
            EIoChunkType::ContainerHeader
                | EIoChunkType::ScriptObjects
                | EIoChunkType::ShaderCodeLibrary
                | EIoChunkType::ShaderCode
        );
        let first_block = (ol.get_offset() / block_size) as usize;
        let block_count = (ol.get_length().div_ceil(block_size)) as usize;

        for (bi, block) in toc.compression_blocks[first_block..first_block + block_count].iter().enumerate() {
            let (off, csz, usz) = (block.get_offset(), block.get_compressed_size() as u64, block.get_uncompressed_size() as u64);
            if block.get_compression_method_index() != 0 {
                ensure!(!force_raw, "chunk {ci} ({chunk_type:?}) block {bi}: special chunk type must store every block raw");
                ensure!(usz > 1024, "chunk {ci} block {bi}: compressed block with uncompressed size {usz} <= 1024");
                ensure!(off % 16 == 0, "chunk {ci} block {bi}: compressed block offset {off} not 16-aligned");
                ensure!(csz < usz, "chunk {ci} block {bi}: compressed block did not shrink ({csz} >= {usz})");
                ensure!(off + align_u64(csz, 16) <= ucas_len, "chunk {ci} block {bi}: padded compressed span [{off}, {}) exceeds .ucas size {ucas_len}", off + align_u64(csz, 16));
            } else {
                ensure!(csz == usz, "chunk {ci} block {bi}: raw block sizes differ ({csz} != {usz})");
                ensure!(off + csz <= ucas_len, "chunk {ci} block {bi}: raw span [{off}, {}) exceeds .ucas size {ucas_len}", off + csz);
            }
        }

        // The single-shot mount read of the container header (invariant 1).
        if chunk_type == EIoChunkType::ContainerHeader && block_count > 0 {
            let first_offset = toc.compression_blocks[first_block].get_offset();
            let read_end = first_offset + align_u64(ol.get_length(), 16);
            ensure!(
                read_end <= ucas_len,
                "ContainerHeader chunk {ci}: mount-time read of align({}, 16) bytes at {first_offset} ends at {read_end}, past .ucas size {ucas_len}",
                ol.get_length()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use fs_err as fs;

    /// The compressed write path must keep the game's mount-time reads
    /// in-bounds: ContainerHeader stored raw, no sub-1-KiB compressed blocks,
    /// 16-aligned compressed offsets. Guards against the regression where a
    /// compressed ContainerHeader (the LAST chunk in the .ucas) made the
    /// runtime's single-shot header read run past end-of-file, so the game
    /// silently ignored the whole container.
    #[test]
    fn test_compressed_container_mount_invariants() -> Result<()> {
        let dir = std::env::temp_dir().join(format!("retoc-writer-inv-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir)?;
        let utoc = dir.join("inv.utoc");
        let writer = IoStoreWriter::new(&utoc, EIoStoreTocVersion::PerfectHashWithOverflow, Some(EIoContainerHeaderVersion::OptionalSegmentPackages), "../../..".into())?;
        let mut writer = writer.set_compress(true);

        // Highly compressible block above the 1 KiB admission threshold: compressed.
        writer.write_chunk(FIoChunkId::create(1, 0, EIoChunkType::ExportBundleData), None, &vec![0x42u8; 8192])?;
        // Just as compressible, but at/below the threshold: must stay raw.
        writer.write_chunk(FIoChunkId::create(2, 0, EIoChunkType::ExportBundleData), None, &vec![0x42u8; 512])?;
        writer.finalize()?;

        // Exactly one compressed block (the 8 KiB one; the tiny block and the
        // ContainerHeader stay raw), container flagged Indexed|Compressed.
        let (flags, comp_offsets) = dump_compressed_layout(&utoc)?;
        assert_eq!(flags, 9, "container_flags must be Indexed|Compressed");
        assert_eq!(comp_offsets.len(), 1, "only the 8 KiB block may be compressed");

        verify_game_mount_invariants(&utoc)?;

        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn test_write_container() -> Result<()> {
        fs::create_dir("out").ok();
        let mut writer = IoStoreWriter::new("out/new.utoc", EIoStoreTocVersion::PerfectHashWithOverflow, Some(EIoContainerHeaderVersion::OptionalSegmentPackages), "../../..".into())?;

        let data = fs::read("tests/UE5.3/ScriptObjects.bin")?;
        writer.write_chunk_raw(FIoChunkIdRaw { id: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5] }, Some(UEPath::new("../../../asdf/asdf/dasf/script_objects.bin")), &data)?;
        writer.finalize()?;
        Ok(())
    }
}
