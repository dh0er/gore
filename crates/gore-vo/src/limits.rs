/// Resource limits applied before and while processing archives and Ogg streams.
///
/// Defaults are intentionally generous enough for a large voice archive while still placing
/// finite bounds on every attacker-controlled count or allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_archive_bytes: u64,
    pub max_entries: usize,
    pub max_path_bytes: usize,
    pub max_entry_uncompressed_bytes: u64,
    pub max_total_uncompressed_bytes: u64,
    pub max_compression_ratio: u64,
    pub max_ogg_bytes: usize,
    pub max_ogg_pages: usize,
    pub max_ogg_page_body_bytes: usize,
    pub max_ogg_packet_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_archive_bytes: 16 * 1024 * 1024 * 1024,
            max_entries: 250_000,
            max_path_bytes: 1_024,
            max_entry_uncompressed_bytes: 512 * 1024 * 1024,
            max_total_uncompressed_bytes: 64 * 1024 * 1024 * 1024,
            max_compression_ratio: 1_000,
            max_ogg_bytes: 64 * 1024 * 1024,
            max_ogg_pages: 500_000,
            max_ogg_page_body_bytes: 255 * 255,
            max_ogg_packet_bytes: 16 * 1024 * 1024,
        }
    }
}
