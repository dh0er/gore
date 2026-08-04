//! Probe top-level RIFF chunks + how the FSB5 is wrapped (SND chunk?).
use gore_fmod::*;

const DEFAULT_DIR: &str =
    r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Content\FMOD\Desktop";

fn a4(b: &[u8], o: usize) -> String {
    (0..4)
        .map(|i| {
            let c = b[o + i];
            if (32..127).contains(&c) {
                c as char
            } else {
                '.'
            }
        })
        .collect()
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_DIR.to_string());
    let name = std::env::args().nth(2).unwrap_or_else(|| "SFX.bank".into());
    let b = std::fs::read(format!("{dir}\\{name}")).unwrap();
    println!(
        "{name} len={} riff_size@0x04={} (len-8={})",
        b.len(),
        u32_le(&b, 0x04),
        b.len() - 8
    );

    // top-level chunk walk
    let mut off = 0x0C;
    println!("--- top-level chunks ---");
    while off + 8 <= b.len() {
        let cc = a4(&b, off);
        let sz = u32_le(&b, off + 4) as usize;
        println!(
            "  {cc:6} off=0x{off:x} size={sz} body=0x{:x} next=0x{:x}",
            off + 8,
            off + 8 + sz
        );
        if !cc.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
            println!("  (stop: garbage)");
            break;
        }
        off += 8 + sz;
    }

    // SNDH entries + what's at the referenced FSB5 offset
    let entries = parse_bank(&b).unwrap();
    println!("--- SNDH entries ---");
    for (i, e) in entries.iter().enumerate() {
        let pre_off = e.fsb5_offset.saturating_sub(8);
        println!(
            "  [{i}] fsb5_offset=0x{:x} size={}  bytes_before=[{}] hex_before={:02x?}",
            e.fsb5_offset,
            e.fsb5_size,
            a4(&b, pre_off),
            &b[pre_off..e.fsb5_offset]
        );
    }
}
