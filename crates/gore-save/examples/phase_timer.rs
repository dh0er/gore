//! Scratch research driver: decompose the cost of one save write into phases and
//! test whether the chunk codec parallelises. Read-only; writes nothing. Not shipped.

use std::time::Instant;

const PACKAGE_FILE_TAG: u32 = 0x9E2A83C1;
const COMPRESSED_HEADER_V2: u32 = 0x2222_2222;

struct Chunk {
    off: usize,
    csize: usize,
    usize_: usize,
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: phase_timer <save.sav>");
    let threads: usize = std::env::args()
        .nth(2)
        .and_then(|v| v.parse().ok())
        .unwrap_or(std::thread::available_parallelism().map(|v| v.get()).unwrap_or(8));
    let data = std::fs::read(&path).expect("read save");

    // Find the compressed-stream header by scanning for the package tag.
    let tag = PACKAGE_FILE_TAG.to_le_bytes();
    let tag_at = (0..data.len() - 4)
        .find(|&i| data[i..i + 4] == tag)
        .expect("PACKAGE_FILE_TAG not found");
    let mut p = tag_at + 4;
    let hdr_version = u32::from_le_bytes(data[p..p + 4].try_into().unwrap());
    assert_eq!(hdr_version, COMPRESSED_HEADER_V2, "only v2 headers here");
    p += 4;
    let max_chunk = u64::from_le_bytes(data[p..p + 8].try_into().unwrap()) as usize;
    p += 8;
    let _alg = data[p];
    p += 1;
    let sum_c = u64::from_le_bytes(data[p..p + 8].try_into().unwrap()) as usize;
    p += 8;
    let sum_u = u64::from_le_bytes(data[p..p + 8].try_into().unwrap()) as usize;
    p += 8;
    let count = sum_u.div_ceil(max_chunk);
    let mut table = Vec::with_capacity(count);
    for _ in 0..count {
        let c = u64::from_le_bytes(data[p..p + 8].try_into().unwrap()) as usize;
        p += 8;
        let u = u64::from_le_bytes(data[p..p + 8].try_into().unwrap()) as usize;
        p += 8;
        table.push((c, u));
    }
    let mut cursor = p;
    let chunks: Vec<Chunk> = table
        .iter()
        .map(|&(c, u)| {
            let ch = Chunk { off: cursor, csize: c, usize_: u };
            cursor += c;
            ch
        })
        .collect();

    println!("file            : {} bytes", data.len());
    println!("chunks          : {count}  (max chunk {max_chunk} bytes)");
    println!("compressed total: {sum_c} bytes");
    println!("plaintext total : {sum_u} bytes ({:.1} MB, ratio {:.1}:1)", sum_u as f64 / 1e6, sum_u as f64 / sum_c as f64);
    println!();

    // --- serial decompress ---
    let t = Instant::now();
    let plain: Vec<Vec<u8>> = chunks
        .iter()
        .map(|c| gore_oodle::decompress(&data[c.off..c.off + c.csize], c.usize_).expect("decode"))
        .collect();
    let serial_dec = t.elapsed();
    println!("decompress all  serial   : {serial_dec:?}  ({:.0} MB/s)", sum_u as f64 / 1e6 / serial_dec.as_secs_f64());

    // --- parallel decompress ---
    let t = Instant::now();
    let par_plain = par_map(&chunks, threads, |c| {
        gore_oodle::decompress(&data[c.off..c.off + c.csize], c.usize_).expect("decode")
    });
    let par_dec = t.elapsed();
    println!("decompress all  {threads:>2} threads: {par_dec:?}  ({:.0} MB/s, {:.1}x)", sum_u as f64 / 1e6 / par_dec.as_secs_f64(), serial_dec.as_secs_f64() / par_dec.as_secs_f64());
    assert_eq!(plain, par_plain, "parallel decode must match serial");
    println!();

    // --- compress at each level, serial and parallel ---
    for (name, level) in [
        ("Fastest", gore_oodle::Level::Fastest),
        ("Fast   ", gore_oodle::Level::Fast),
        ("Default", gore_oodle::Level::Default),
    ] {
        let t = Instant::now();
        let out: Vec<Vec<u8>> = plain.iter().map(|c| gore_oodle::compress(c, level).expect("encode")).collect();
        let ser = t.elapsed();
        let total: usize = out.iter().map(|c| c.len()).sum();

        let t = Instant::now();
        let out_par = par_map(&plain, threads, |c| gore_oodle::compress(c, level).expect("encode"));
        let par = t.elapsed();
        assert_eq!(out, out_par, "parallel encode must match serial");

        let identical = out
            .iter()
            .zip(chunks.iter())
            .filter(|(re, c)| re.as_slice() == &data[c.off..c.off + c.csize])
            .count();
        println!("    -> {identical}/{count} re-encoded chunks are BYTE-IDENTICAL to what is on disk");
        println!(
            "compress {name} serial: {ser:>10.3?} ({:>4.0} MB/s) | {threads:>2} threads: {par:>10.3?} ({:>4.0} MB/s, {:.1}x) | size {} ({:+.1}% vs disk)",
            sum_u as f64 / 1e6 / ser.as_secs_f64(),
            sum_u as f64 / 1e6 / par.as_secs_f64(),
            ser.as_secs_f64() / par.as_secs_f64(),
            total,
            (total as f64 / sum_c as f64 - 1.0) * 100.0
        );
    }
    println!();

    // --- how many chunks does a typical in-place edit actually dirty? ---
    let flat_len: usize = plain.iter().map(|c| c.len()).sum();
    println!("a single in-place byte patch dirties 1 of {count} chunks ({:.2}% of {flat_len} plaintext bytes)", 100.0 / count as f64);

    // --- cost of re-encoding only one chunk ---
    let t = Instant::now();
    let _ = gore_oodle::compress(&plain[count / 2], gore_oodle::Level::Default).expect("encode");
    println!("re-encode ONE 128 KB chunk (Default): {:?}", t.elapsed());
}

fn par_map<T: Sync, R: Send>(items: &[T], threads: usize, f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let n = items.len();
    let mut out: Vec<Option<R>> = (0..n).map(|_| None).collect();
    let chunk = n.div_ceil(threads);
    std::thread::scope(|scope| {
        let f = &f;
        for (items_part, out_part) in items.chunks(chunk).zip(out.chunks_mut(chunk)) {
            scope.spawn(move || {
                for (item, slot) in items_part.iter().zip(out_part.iter_mut()) {
                    *slot = Some(f(item));
                }
            });
        }
    });
    out.into_iter().map(|v| v.unwrap()).collect()
}
