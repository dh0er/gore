/// Resource limits applied before and while processing archives and Ogg streams.
///
/// Defaults are intentionally generous enough for a large voice archive while still placing
/// finite bounds on every attacker-controlled count or allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_archive_bytes: u64,
    /// Maximum serialized central-directory bytes accepted before the ZIP parser is invoked.
    pub max_central_directory_bytes: u64,
    pub max_entries: usize,
    pub max_path_bytes: usize,
    pub max_entry_uncompressed_bytes: u64,
    pub max_total_uncompressed_bytes: u64,
    pub max_compression_ratio: u64,
    pub max_ogg_bytes: usize,
    pub max_ogg_pages: usize,
    pub max_ogg_page_body_bytes: usize,
    pub max_ogg_packet_bytes: usize,
    /// Maximum bytes in any Vorbis/Opus identification, comment, or setup header.
    pub max_ogg_codec_header_bytes: usize,
    /// Maximum completed audio packets in one recognized logical stream.
    pub max_ogg_audio_packets: usize,
    /// Maximum decoded Vorbis frames per channel (one hour at 48 kHz by default).
    pub max_ogg_decoded_samples_per_channel: usize,
    /// Maximum declared entries summed across every Vorbis setup codebook. Lewton allocates a
    /// codeword-length slot for every declared entry, including sparse/unused entries.
    pub max_vorbis_codebook_entries: usize,
    /// Conservative upper bound for Lewton's transient Huffman trie work, computed as one root per
    /// codebook plus the sum of every active codeword length.
    pub max_vorbis_huffman_tree_nodes: usize,
    /// Maximum materialized Vorbis VQ scalars summed across every lookup-bearing codebook.
    pub max_vorbis_vq_scalars: usize,
    /// Conservative ceiling for APIs that return a complete rewritten archive in RAM.
    pub max_in_memory_archive_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_archive_bytes: 16 * 1024 * 1024 * 1024,
            max_central_directory_bytes: 512 * 1024 * 1024,
            max_entries: 250_000,
            max_path_bytes: 1_024,
            max_entry_uncompressed_bytes: 512 * 1024 * 1024,
            max_total_uncompressed_bytes: 64 * 1024 * 1024 * 1024,
            max_compression_ratio: 1_000,
            max_ogg_bytes: 64 * 1024 * 1024,
            max_ogg_pages: 500_000,
            max_ogg_page_body_bytes: 255 * 255,
            max_ogg_packet_bytes: 16 * 1024 * 1024,
            max_ogg_codec_header_bytes: 1024 * 1024,
            max_ogg_audio_packets: 1_000_000,
            max_ogg_decoded_samples_per_channel: 48_000 * 60 * 60,
            max_vorbis_codebook_entries: 256 * 1024,
            max_vorbis_huffman_tree_nodes: 1024 * 1024,
            max_vorbis_vq_scalars: 4 * 1024 * 1024,
            max_in_memory_archive_bytes: 512 * 1024 * 1024,
        }
    }
}
