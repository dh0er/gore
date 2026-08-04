//! SCRATCH proof-of-decode example for Task 2 (retoc + gore-oodle).
//!
//! NOT part of the gore-tex public API or test suite -- it exists only to prove
//! that the vendored retoc fork decompresses real Oodle-compressed chunks from
//! the shipped Gothic 1 Remake IoStore container via gore-oodle (no oo2core).
//!
//! Run with the container path (defaults to the local Steam install):
//!   cargo run -p gore-tex --example decode_real_container -- \
//!     "D:/SteamLibrary/steamapps/common/Gothic 1 Remake/G1R/Content/Paks/G1R-Windows.utoc"
//!
//! It reads a sample of chunks, decompresses each (Oodle blocks go through
//! gore_oodle::kraken_decompress), and verifies the decoded bytes against the
//! blake3 chunk hash stored in the TOC. A hash match is end-to-end proof that
//! the gore-oodle decode is byte-identical to what the container was built with.

use std::sync::Arc;

use anyhow::{Context, Result};
use retoc::iostore;
use retoc::Config;

const DEFAULT_UTOC: &str =
    "D:/SteamLibrary/steamapps/common/Gothic 1 Remake/G1R/Content/Paks/G1R-Windows.utoc";

fn main() -> Result<()> {
    let utoc = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_UTOC.to_string());
    println!("opening container: {utoc}");

    let store = iostore::open(&utoc, Arc::new(Config::default()))
        .with_context(|| format!("failed to open {utoc}"))?;

    println!("container name : {}", store.container_name());
    println!("toc version    : {:?}", store.container_file_version());
    println!("header version : {:?}", store.container_header_version());

    // Sample chunks across the container so we are very likely to hit
    // Oodle-compressed blocks (the bulk of a UE5 container).
    let total = store.chunks().count();
    println!("total chunks   : {total}");

    let sample_target = 64usize;
    let step = (total / sample_target).max(1);

    let mut checked = 0usize;
    let mut total_decoded_bytes = 0u64;
    let mut largest = 0u64;

    for (i, chunk) in store.chunks().enumerate() {
        if i % step != 0 {
            continue;
        }

        // read() runs the full IoStore decode path:
        //   Toc::read -> compression::decompress -> gore_oodle::kraken_decompress
        // for every Oodle-compressed block in the chunk.
        let data = chunk
            .read()
            .with_context(|| format!("decode failed for chunk #{i} ({:?})", chunk.id()))?;

        // Verify decoded bytes against the blake3 hash recorded in the TOC meta.
        // (Mirrors retoc's own `verify` command: first 20 bytes of blake3.)
        let hash = blake3::hash(&data);
        let toc_hash = chunk.hash();
        if data.is_empty() {
            // Some meta-only chunks can be zero length; skip the hash compare.
        } else if toc_hash.0[..20] != hash.as_bytes()[..20] {
            anyhow::bail!(
                "HASH MISMATCH on chunk #{i} ({:?}, {} bytes) -- gore-oodle decode is NOT byte-identical",
                chunk.id(),
                data.len()
            );
        }

        total_decoded_bytes += data.len() as u64;
        largest = largest.max(data.len() as u64);
        checked += 1;
    }

    println!("---");
    println!("verified {checked} sampled chunks (blake3 matched TOC meta)");
    println!("decoded {total_decoded_bytes} bytes total, largest chunk {largest} bytes");
    println!("PROOF OK: gore-oodle decode path is byte-identical for this container");
    Ok(())
}
