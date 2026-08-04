//! M2a de-risk: walk the FEV/BNKI chunk tree of a bank, report whether samples are
//! referenced via WAV (WaveformResource — repointable) or STBL (SoundTable — not),
//! count WAV nodes, show their (SoundBankIndex, SubsoundIndex), dump SNDH body.
//!
//! Run: cargo run -p gore-fmod --example tree --release -- [FMOD_DESKTOP_DIR]

use gore_fmod::*;

const DEFAULT_DIR: &str =
    r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Content\FMOD\Desktop";

#[derive(Default)]
struct Stats {
    wav_count: usize,
    wav_samples: Vec<(i32, i32)>, // (SoundBankIndex, SubsoundIndex)
    has_stbl: bool,
    stbl_bank_index: Option<i32>,
    has_wavs: bool,
    sndh_dump: Option<String>,
    lines: usize,
}

fn ascii4(b: &[u8], o: usize) -> String {
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

fn i32_le(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn walk(b: &[u8], start: usize, end: usize, depth: usize, st: &mut Stats) {
    let mut off = start;
    while off + 8 <= end {
        let fourcc = ascii4(b, off);
        let size = u32_le(b, off + 4) as usize;
        let body = off + 8;
        let bodyend = (body + size).min(b.len());
        // sanity: fourcc must be printable-ish
        if !fourcc.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
            break;
        }

        let show = st.lines < 60;
        if show {
            println!("{}{} size={}", "  ".repeat(depth), fourcc, size);
            st.lines += 1;
        }

        match fourcc.as_str() {
            "LIST" => {
                let ltype = ascii4(b, body);
                if ltype == "WAVS" {
                    st.has_wavs = true;
                }
                if show {
                    println!("{}  (list-type {})", "  ".repeat(depth), ltype);
                }
                walk(b, body + 4, bodyend, depth + 1, st);
            }
            "WAV " => {
                st.wav_count += 1;
                if body + 0x1A <= b.len() {
                    let sb = i32_le(b, body + 0x12);
                    let ss = i32_le(b, body + 0x16);
                    if st.wav_samples.len() < 12 {
                        st.wav_samples.push((sb, ss));
                    }
                }
            }
            "STBL" => {
                st.has_stbl = true;
                if body + 8 <= b.len() {
                    st.stbl_bank_index = Some(i32_le(b, body + 4));
                }
            }
            "SNDH" => {
                let n = 16.min(bodyend - body);
                st.sndh_dump = Some(
                    (0..n)
                        .map(|i| format!("{:02x}", b[body + i]))
                        .collect::<Vec<_>>()
                        .join(" "),
                );
            }
            _ => {}
        }
        off = bodyend;
    }
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_DIR.to_string());
    for name in ["SFX.bank", "Music.bank", "VO.bank", "CINEMATICS.bank"] {
        let path = format!("{dir}\\{name}");
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                println!("\n##### {name}: READ ERROR {e}");
                continue;
            }
        };
        println!("\n##### {name} ({:.1} MB) #####", bytes.len() as f64 / 1e6);

        // locate LIST/PROJ/BNKI, walk sub-chunks (reuse the top-level walk)
        let mut st = Stats::default();
        walk(&bytes, 0x0C, bytes.len(), 0, &mut st);

        println!("  --- summary ---");
        println!("  WAVS list present : {}", st.has_wavs);
        println!("  WAV  nodes        : {}", st.wav_count);
        println!(
            "  STBL SoundTable   : {}{}",
            st.has_stbl,
            st.stbl_bank_index
                .map(|i| format!(" (SoundbankIndex={i})"))
                .unwrap_or_default()
        );
        if !st.wav_samples.is_empty() {
            println!(
                "  first WAV refs (SoundBankIndex, SubsoundIndex): {:?}",
                st.wav_samples
            );
        }
        if let Some(d) = &st.sndh_dump {
            println!("  SNDH body[0..16]  : {d}");
        }
        println!(
            "  => REPOINTABLE via WAV : {}",
            if st.wav_count > 0 {
                "YES"
            } else {
                "NO (STBL-only or other)"
            }
        );
    }
}
