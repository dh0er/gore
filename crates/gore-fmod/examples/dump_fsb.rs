//! Extract one decrypted FSB5 sub-bank from a bank to a standalone .fsb file
//! (for cross-checking with vgmstream etc.).
//! Run: cargo run -p gore-fmod --example dump_fsb --release -- <DIR> <BANK> <FSB_INDEX> <OUT.fsb>
use gore_fmod::*;
const KEY: &[u8] = b"NGpxstJ42kfNfz4z3CsS";
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let dir = &a[0];
    let bank = &a[1];
    let idx: usize = a[2].parse().unwrap();
    let outp = &a[3];
    let b = std::fs::read(format!("{dir}\\{bank}")).unwrap();
    let e = parse_bank(&b).unwrap();
    println!("{} FSB5 sub-banks; extracting #{idx}", e.len());
    let mut blk = b[e[idx].fsb5_offset..e[idx].fsb5_offset + e[idx].fsb5_size].to_vec();
    fsb5_decrypt(&mut blk, KEY);
    let f = parse_fsb5(&blk).unwrap();
    println!("codec={:?} samples={}", f.codec, f.samples.len());
    for s in f.samples.iter().take(3) {
        println!(
            "  \"{}\" {}Hz {}ch frames={} size={} dataoff={}",
            s.name, s.freq, s.channels, s.num_samples, s.size, s.data_offset
        );
    }
    std::fs::write(outp, &blk).unwrap();
    println!("wrote {} ({} bytes)", outp, blk.len());
}
