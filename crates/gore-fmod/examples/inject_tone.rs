//! M2 proof: replace one SFX sample with a 440 Hz test tone via multi-FSB PCM injection,
//! validate the rebuilt bank with our own parser, and write it out (no FMOD).
//!
//! Run:  cargo run -p gore-fmod --example inject_tone --release -- [DIR] [BANK] [NAME|INDEX] [--apply]
//! Default: SFX.bank, subsound 0, writes <DIR>\<BANK>.modtest (does NOT touch the game file).
//! With --apply: backs up to <BANK>.gore-bak then overwrites the game file in place.

use gore_fmod::*;

const KEY: &[u8] = b"NGpxstJ42kfNfz4z3CsS";
const DEFAULT_DIR: &str =
    r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Content\FMOD\Desktop";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let apply = args.iter().any(|a| a == "--apply");
    let pos: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let dir = pos.first().map(|s| s.as_str()).unwrap_or(DEFAULT_DIR).to_string();
    let bank = pos.get(1).map(|s| s.as_str()).unwrap_or("SFX.bank").to_string();
    let target_arg = pos.get(2).map(|s| s.to_string());

    let path = format!("{dir}\\{bank}");
    let bytes = std::fs::read(&path).expect("read bank");
    println!("loaded {bank} ({:.1} MB)", bytes.len() as f64 / 1e6);

    // parse FSB5 #0 to resolve names / count
    let entries = parse_bank(&bytes).expect("parse_bank");
    let mut blk = bytes[entries[0].fsb5_offset..entries[0].fsb5_offset + entries[0].fsb5_size].to_vec();
    fsb5_decrypt(&mut blk, KEY);
    let fsb = parse_fsb5(&blk).expect("parse_fsb5");

    // resolve target subsound(s). Default = all menu-button click variants (easy to test
    // in the main menu: every button click should beep).
    let targets: Vec<usize> = match &target_arg {
        None => {
            let t: Vec<usize> = fsb
                .samples
                .iter()
                .enumerate()
                .filter(|(_, x)| x.name.contains("UI_Action_"))
                .map(|(i, _)| i)
                .collect();
            if t.is_empty() { vec![0] } else { t }
        }
        Some(s) => match s.parse::<usize>() {
            Ok(i) => vec![i],
            Err(_) => vec![fsb
                .samples
                .iter()
                .position(|x| x.name == *s)
                .unwrap_or_else(|| panic!("sample name not found: {s}"))],
        },
    };
    println!("replacing {} subsound(s) with a 440Hz tone:", targets.len());
    for &t in &targets {
        println!("  #{t} = \"{}\" ({}Hz {}ch)", fsb.samples[t].name, fsb.samples[t].freq, fsb.samples[t].channels);
    }
    let tone_name = format!("{}_GORETONE", fsb.samples[targets[0]].name);

    // build a 1.0s 440 Hz mono 48k tone (loud, square-ish so it's unmistakable)
    let freq = 48000u32;
    let n = freq as usize;
    let mut pcm = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / freq as f32;
        let s = (2.0 * std::f32::consts::PI * 440.0 * t).sin();
        pcm.push((s * 12000.0) as i16);
    }
    let new_fsb5 = build_fsb5_pcm16(&tone_name, freq, 1, &pcm).expect("build");
    println!("built PCM16 FSB5: {} bytes ({} frames)", new_fsb5.len(), n);

    // inject
    let out = inject_pcm_sample_multi(&bytes, &targets, &new_fsb5, KEY).expect("inject");
    println!("rebuilt bank: {:.1} MB (was {:.1})", out.len() as f64 / 1e6, bytes.len() as f64 / 1e6);

    // ---- validate the rebuilt bank with our own parser ----
    validate(&out, &targets);

    // ---- write ----
    if apply {
        let bak = format!("{path}.gore-bak");
        if !std::path::Path::new(&bak).exists() {
            std::fs::copy(&path, &bak).expect("backup");
            println!("backed up -> {bak}");
        } else {
            println!("backup already exists -> {bak} (left as-is)");
        }
        std::fs::write(&path, &out).expect("write game file");
        println!("APPLIED in place: {path}");
    } else {
        let outp = format!("{path}.modtest");
        std::fs::write(&outp, &out).expect("write");
        println!("wrote {outp}");
        println!("to test: back up {bank}, then rename {bank}.modtest -> {bank}");
    }
}

fn validate(out: &[u8], targets: &[usize]) {
    println!("--- validation ---");
    assert_eq!(&out[0..4], b"RIFF", "RIFF magic");
    assert_eq!(u32_le(out, 0x04) as usize, out.len() - 8, "RIFF size");
    let entries = parse_bank(out).expect("re-parse bank");
    assert_eq!(entries.len(), 2, "should have 2 FSB5 now");
    println!("  FSB5 sub-banks: {} (offsets 0x{:x}, 0x{:x})", entries.len(), entries[0].fsb5_offset, entries[1].fsb5_offset);
    assert_eq!(entries[0].fsb5_offset % 32, 0, "FSB5#0 32-aligned");
    assert_eq!(entries[1].fsb5_offset % 32, 0, "FSB5#1 32-aligned");

    // FSB5 #0 still original Vorbis, intact
    let mut b0 = out[entries[0].fsb5_offset..entries[0].fsb5_offset + entries[0].fsb5_size].to_vec();
    fsb5_decrypt(&mut b0, KEY);
    let f0 = parse_fsb5(&b0).expect("parse fsb5#0");
    println!("  FSB5#0 codec={:?} samples={}", f0.codec, f0.samples.len());
    assert_eq!(f0.codec, Codec::Vorbis);

    // FSB5 #1 = our PCM16 tone
    let mut b1 = out[entries[1].fsb5_offset..entries[1].fsb5_offset + entries[1].fsb5_size].to_vec();
    fsb5_decrypt(&mut b1, KEY);
    let f1 = parse_fsb5(&b1).expect("parse fsb5#1");
    println!("  FSB5#1 codec={:?} samples={} name=\"{}\" {}Hz {}ch frames={}",
        f1.codec, f1.samples.len(), f1.samples[0].name, f1.samples[0].freq, f1.samples[0].channels, f1.samples[0].num_samples);
    assert_eq!(f1.codec, Codec::Pcm16);
    assert_eq!(f1.samples.len(), 1);

    // WAV repoint check: the target's WAV node now (1,0)
    let (list_off, list_size) = {
        let mut off = 0x0C;
        loop {
            let cc = &out[off..off + 4];
            let sz = u32_le(out, off + 4) as usize;
            if cc == b"LIST" { break (off, sz); }
            off += 8 + sz;
        }
    };
    let mut wavs = Vec::new();
    gather_pub(out, list_off + 8 + 4, list_off + 8 + list_size, &mut wavs);
    let repointed = wavs.iter().filter(|(_, sb, ss)| *sb == 1 && *ss == 0).count();
    let still_target = targets
        .iter()
        .any(|&t| wavs.iter().any(|(_, sb, ss)| *sb == 0 && *ss as usize == t));
    println!("  WAV nodes total={} repointed(1,0)={} (expected {}) any-target-still-(0,t)={}",
        wavs.len(), repointed, targets.len(), still_target);
    assert_eq!(repointed, targets.len(), "all target WAVs should now point at (1,0)");
    assert!(!still_target, "no target WAV may still point at FSB5#0");
    println!("VALIDATION PASSED");
}

// local copy of the WAV gatherer (lib's is private)
fn gather_pub(b: &[u8], start: usize, end: usize, wavs: &mut Vec<(usize, i32, i32)>) {
    let mut off = start;
    while off + 8 <= end {
        let cc = &b[off..off + 4];
        let sz = u32_le(b, off + 4) as usize;
        let body = off + 8;
        if !cc.iter().all(|&c| c == 0x20 || (0x21..0x7f).contains(&c)) { break; }
        if cc == b"WAV " && body + 0x1A <= b.len() {
            wavs.push((body, i32::from_le_bytes([b[body+0x12],b[body+0x13],b[body+0x14],b[body+0x15]]),
                            i32::from_le_bytes([b[body+0x16],b[body+0x17],b[body+0x18],b[body+0x19]])));
        }
        if cc == b"LIST" { gather_pub(b, body + 4, (body + sz).min(b.len()), wavs); }
        off = body + sz;
    }
}
