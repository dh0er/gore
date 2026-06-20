use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use gore_as::cache::header::CacheHeader;
use gore_as::cache::scan::scan_strings;

#[derive(Parser)]
#[command(name = "gore-as", about = "AngelScript precompiled-cache tooling")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Parse and print the outer cache header.
    DecodeHeader { file: PathBuf },
    /// Scan length-prefixed type-name strings (decode investigation aid).
    Walk {
        file: PathBuf,
        #[arg(long, default_value_t = 100)]
        max: usize,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::DecodeHeader { file } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let h = CacheHeader::parse(&bytes).context("parsing header")?;
            println!("hash       : {}", hex16(&h.hash));
            println!("magic      : {:#010x}", h.magic);
            println!("type_count : {}", h.type_count);
        }
        Cmd::Walk { file, max } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            for s in scan_strings(&bytes, CacheHeader::SIZE, max) {
                println!("0x{:08x}  len={:<4} {}", s.offset, s.len, s.text);
            }
        }
    }
    Ok(())
}

fn hex16(b: &[u8; 16]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
