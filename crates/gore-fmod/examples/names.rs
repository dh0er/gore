//! List sample names (optionally filtered) of a bank's FSB5 #0, with their subsound index.
//! Run: cargo run -p gore-fmod --example names --release -- [DIR] [BANK] [FILTER]
use gore_fmod::*;
const KEY: &[u8] = b"NGpxstJ42kfNfz4z3CsS";
const DEFAULT_DIR: &str =
    r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Content\FMOD\Desktop";
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let dir = a.first().map(|s| s.as_str()).unwrap_or(DEFAULT_DIR);
    let bank = a.get(1).map(|s| s.as_str()).unwrap_or("SFX.bank");
    let filter = a.get(2).map(|s| s.to_lowercase());
    let b = std::fs::read(format!("{dir}\\{bank}")).unwrap();
    let e = parse_bank(&b).unwrap();
    let mut blk = b[e[0].fsb5_offset..e[0].fsb5_offset + e[0].fsb5_size].to_vec();
    fsb5_decrypt(&mut blk, KEY);
    let f = parse_fsb5(&blk).unwrap();
    let mut shown = 0;
    for (i, s) in f.samples.iter().enumerate() {
        if let Some(flt) = &filter {
            if !s.name.to_lowercase().contains(flt) {
                continue;
            }
        }
        println!("#{i:<5} {} ({}Hz {}ch)", s.name, s.freq, s.channels);
        shown += 1;
        if shown >= 60 {
            println!("… (capped at 60)");
            break;
        }
    }
    println!("total {} samples, {} shown", f.samples.len(), shown);
}
