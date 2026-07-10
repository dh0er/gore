//! M0 decrypt spike: decrypt + parse the game's FMOD banks, report structure,
//! codec, sample counts, and a PCM-rebuild bloat estimate. Pure Rust, no FMOD.
//!
//! Run: cargo run -p gore-fmod --example spike --release -- [FMOD_DESKTOP_DIR]

use gore_fmod::*;

const KEY: &[u8] = b"NGpxstJ42kfNfz4z3CsS";
const DEFAULT_DIR: &str =
    r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Content\FMOD\Desktop";

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| DEFAULT_DIR.to_string());
    let banks = [
        "Master.bank",
        "Music.bank",
        "SFX.bank",
        "CINEMATICS.bank",
        "VO.bank",
    ];

    for name in banks {
        let path = format!("{dir}\\{name}");
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                println!("\n=== {name} === READ ERROR: {e}");
                continue;
            }
        };
        println!("\n=== {name} ({:.1} MB) ===", bytes.len() as f64 / 1e6);

        let entries = match parse_bank(&bytes) {
            Ok(e) => e,
            Err(e) => {
                println!("  parse_bank FAILED: {e}");
                continue;
            }
        };
        println!("  embedded FSB5 sub-banks: {}", entries.len());

        let mut total_enc = 0u64;
        let mut total_pcm = 0u64;
        let mut first_roundtrip_checked = false;

        for (bi, ent) in entries.iter().enumerate() {
            // determine size: explicit, else to next entry / EOF
            let size = if ent.fsb5_size != 0 {
                ent.fsb5_size
            } else if bi + 1 < entries.len() {
                entries[bi + 1].fsb5_offset - ent.fsb5_offset
            } else {
                bytes.len() - ent.fsb5_offset
            };
            if ent.fsb5_offset + size > bytes.len() {
                println!("  [FSB5 {bi}] offset/size out of range");
                continue;
            }
            let mut block = bytes[ent.fsb5_offset..ent.fsb5_offset + size].to_vec();

            // round-trip cipher check on the first block
            if !first_roundtrip_checked {
                let mut rt = block.clone();
                fsb5_decrypt(&mut rt, KEY);
                fsb5_encrypt(&mut rt, KEY);
                println!(
                    "  cipher round-trip (encrypt∘decrypt == original): {}",
                    if rt == block { "OK" } else { "MISMATCH" }
                );
                first_roundtrip_checked = true;
            }

            fsb5_decrypt(&mut block, KEY);
            let magic_ok = &block[0..4] == b"FSB5";
            let fsb = match parse_fsb5(&block) {
                Ok(f) => f,
                Err(e) => {
                    println!(
                        "  [FSB5 {bi}] @0x{:x} size={} decrypt magic_ok={} parse FAILED: {e}",
                        ent.fsb5_offset, size, magic_ok
                    );
                    continue;
                }
            };

            // bloat: PCM16 size = num_samples * channels * 2
            let pcm: u64 = fsb
                .samples
                .iter()
                .map(|s| s.num_samples as u64 * s.channels.max(1) as u64 * 2)
                .sum();
            total_enc += size as u64;
            total_pcm += pcm;

            println!(
                "  [FSB5 {bi}] codec={:?} v{} samples={} encSize={:.2}MB pcm16Est={:.2}MB",
                fsb.codec,
                fsb.version,
                fsb.samples.len(),
                size as f64 / 1e6,
                pcm as f64 / 1e6,
            );
            for s in fsb.samples.iter().take(4) {
                println!(
                    "        \"{}\"  {}Hz {}ch  {} frames  enc={}B{}",
                    s.name,
                    s.freq,
                    s.channels,
                    s.num_samples,
                    s.size,
                    s.vorbis_crc32
                        .map(|c| format!(" vorbisCRC=0x{c:08x}"))
                        .unwrap_or_default()
                );
            }
            if fsb.samples.len() > 4 {
                println!("        … +{} more", fsb.samples.len() - 4);
            }
        }

        if total_pcm > 0 {
            println!(
                "  TOTAL: enc={:.1}MB  pcm16-rebuild-est={:.1}MB  (bloat ×{:.1})",
                total_enc as f64 / 1e6,
                total_pcm as f64 / 1e6,
                total_pcm as f64 / total_enc.max(1) as f64
            );
        }
    }
}
