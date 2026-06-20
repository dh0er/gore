# gore-as Cache Decode Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the `gore_as` crate and decode the *verifiable* outer structure of `PrecompiledScript_Shipping.Cache`, plus a tolerant string-scanner harness to map the type-name layout for follow-up RE.

**Architecture:** New Rust crate `projects/gore-as/crates/gore_as` (lib + `gore-as` bin), added to the root workspace. A `cache::header` module parses the 24-byte outer header (16-byte hash + magic + type count) with golden tests against a committed 8 KB fixture. A `cache::scan` module provides a resyncing length-prefixed-string scanner used to map the type table during decode. The bin exposes `decode-header` and `walk`. A `FORMAT.md` records confirmed format facts and the pinned AngelScript core version.

**Tech Stack:** Rust (edition 2021), clap, thiserror, anyhow; dev: assert_cmd, predicates, pretty_assertions. Matches existing `gore_core` / `gore_cli` conventions.

**Scope note:** This is the *foundation* slice of the Tier-1 work in `docs/superpowers/specs/2026-06-20-gore-as-angelscript-decode-design.md`. It covers M1's verifiable outer-header decode + investigation harness and starts M0's version pinning. Deeper work (full container/record parse, bytecode disassembly, round-trip, offline compiler, UE4SS injector) is intentionally deferred to follow-up plans listed at the end — those require format facts this plan's harness will surface, so writing "exact code" for them now would be guesswork.

**Verified facts this plan builds on (from the real 122 MB file):**
- Bytes `0x00..0x10`: 16-byte hash header = `d54f0ffb10c1054b99f11446a43ed5dc`.
- `u32 @ 0x10` (LE) = `0x9e377abe` (magic).
- `u32 @ 0x14` (LE) = `7264` (type/entry count).
- `0x18` onward: structured per-type records beginning with a u32-length-prefixed name (`AI.AIItemScoring`), interleaved with size fields and a second name (`UGothicAIItemActionScoringEntry`, len 31, preceded by a `0x5a`=90 size field). Full record layout is NOT yet known — that is what the `walk` harness + follow-up plan resolve.

---

### Task 1: Scaffold `gore_as` crate and register in workspace

**Files:**
- Create: `projects/gore-as/crates/gore_as/Cargo.toml`
- Create: `projects/gore-as/crates/gore_as/src/lib.rs`
- Modify: `Cargo.toml` (root workspace `members`)

- [ ] **Step 1: Create the crate manifest**

Create `projects/gore-as/crates/gore_as/Cargo.toml`:

```toml
[package]
name = "gore_as"
version = "0.1.0"
edition = "2021"
license = "MIT"
description = "AngelScript precompiled-cache decoder + tooling for Gothic 1 Remake (gore-tools)."

[lib]
crate-type = ["rlib"]

[[bin]]
name = "gore-as"
path = "src/bin/gore-as.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
thiserror = "2"
anyhow = "1"

[dev-dependencies]
assert_cmd = "2"
predicates = "2"
pretty_assertions = "1"
```

- [ ] **Step 2: Create a minimal lib with a smoke test**

Create `projects/gore-as/crates/gore_as/src/lib.rs`:

```rust
//! AngelScript precompiled-cache decoder for Gothic 1 Remake.
//!
//! See `docs/superpowers/specs/2026-06-20-gore-as-angelscript-decode-design.md`.

pub mod cache;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_builds() {
        assert_eq!(2 + 2, 4);
    }
}
```

- [ ] **Step 3: Create the `cache` module stub**

Create `projects/gore-as/crates/gore_as/src/cache/mod.rs`:

```rust
//! Parsing of `PrecompiledScript_Shipping.Cache`.

pub mod header;
pub mod scan;
```

Create empty placeholders so the crate compiles before later tasks fill them:

`projects/gore-as/crates/gore_as/src/cache/header.rs`:

```rust
// Filled in Task 3.
```

`projects/gore-as/crates/gore_as/src/cache/scan.rs`:

```rust
// Filled in Task 5.
```

- [ ] **Step 4: Register the crate in the root workspace**

In root `Cargo.toml`, add the member to the `members` array so it reads:

```toml
[workspace]
members = [
    "projects/gore-save/crates/goresave_core",
    "projects/gore-save/crates/goresave_oodle",
    "projects/gore-core/crates/gore_core",
    "projects/gore-cli/crates/gore_cli",
    "projects/gore-as/crates/gore_as",
]
resolver = "2"
```

- [ ] **Step 5: Create the bin entrypoint stub**

Create `projects/gore-as/crates/gore_as/src/bin/gore-as.rs` (real CLI lands in Task 4; stub keeps the `[[bin]]` target valid):

```rust
fn main() {
    println!("gore-as: use a subcommand (decode-header, walk). See Task 4.");
}
```

- [ ] **Step 6: Build the crate**

Run: `cargo build -p gore_as`
Expected: compiles clean (warnings about unused modules are fine).

- [ ] **Step 7: Run the smoke test**

Run: `cargo test -p gore_as`
Expected: PASS (`crate_builds`).

- [ ] **Step 8: Commit**

```bash
git add projects/gore-as/crates/gore_as Cargo.toml Cargo.lock
git commit -m "feat(gore-as): scaffold gore_as crate in workspace"
```

---

### Task 2: Add the 8 KB header fixture

**Files:**
- Create: `projects/gore-as/crates/gore_as/tests/fixtures/cache_head_8k.bin`
- Create: `projects/gore-as/crates/gore_as/tests/fixtures/README.md`

- [ ] **Step 1: Extract the first 8192 bytes of the real cache**

The full 122 MB cache is NOT committed. Extract a small header slice from your local game install (path per the spec). Run from the repo root in Git Bash:

```bash
mkdir -p projects/gore-as/crates/gore_as/tests/fixtures
head -c 8192 "/d/SteamLibrary/steamapps/common/Gothic 1 Remake/G1R/Script/PrecompiledScript_Shipping.Cache" \
  > projects/gore-as/crates/gore_as/tests/fixtures/cache_head_8k.bin
```

- [ ] **Step 2: Verify the fixture size and first bytes**

Run:

```bash
wc -c < projects/gore-as/crates/gore_as/tests/fixtures/cache_head_8k.bin
xxd -l 24 projects/gore-as/crates/gore_as/tests/fixtures/cache_head_8k.bin
```

Expected: size `8192`; first 16 bytes `d54f 0ffb 10c1 054b 99f1 1446 a43e d5dc`, then `be7a 379e 601c 0000`.

- [ ] **Step 3: Document the fixture provenance**

Create `projects/gore-as/crates/gore_as/tests/fixtures/README.md`:

```markdown
# Test fixtures

`cache_head_8k.bin` — first 8192 bytes of `PrecompiledScript_Shipping.Cache`
(Gothic 1 Remake, Steam appid 1297900). A header-only slice used for hermetic
decode tests. Not the full script payload. Regenerate with:

    head -c 8192 "<game>/G1R/Script/PrecompiledScript_Shipping.Cache" > cache_head_8k.bin
```

- [ ] **Step 4: Commit**

```bash
git add projects/gore-as/crates/gore_as/tests/fixtures
git commit -m "test(gore-as): add 8KB cache header fixture"
```

---

### Task 3: Decode the outer cache header (TDD)

**Files:**
- Modify: `projects/gore-as/crates/gore_as/src/cache/header.rs`
- Create: `projects/gore-as/crates/gore_as/tests/header_test.rs`

- [ ] **Step 1: Write the failing test**

Create `projects/gore-as/crates/gore_as/tests/header_test.rs`:

```rust
use gore_as::cache::header::{CacheHeader, HeaderError, CACHE_MAGIC};

fn fixture() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/cache_head_8k.bin"
    ))
    .expect("fixture present")
}

#[test]
fn parses_outer_header() {
    let bytes = fixture();
    let h = CacheHeader::parse(&bytes).expect("header parses");
    assert_eq!(
        h.hash,
        [
            0xd5, 0x4f, 0x0f, 0xfb, 0x10, 0xc1, 0x05, 0x4b, 0x99, 0xf1, 0x14, 0x46, 0xa4, 0x3e,
            0xd5, 0xdc
        ]
    );
    assert_eq!(h.magic, CACHE_MAGIC);
    assert_eq!(h.magic, 0x9e37_7abe);
    assert_eq!(h.type_count, 7264);
}

#[test]
fn rejects_short_input() {
    let err = CacheHeader::parse(&[0u8; 10]).unwrap_err();
    assert!(matches!(err, HeaderError::TooShort { .. }));
}

#[test]
fn rejects_bad_magic() {
    let mut bytes = vec![0u8; CacheHeader::SIZE];
    // valid length, wrong magic at 0x10
    bytes[16..20].copy_from_slice(&0xdead_beef_u32.to_le_bytes());
    let err = CacheHeader::parse(&bytes).unwrap_err();
    assert!(matches!(err, HeaderError::BadMagic { .. }));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p gore_as --test header_test`
Expected: FAIL to compile (`CacheHeader` / `HeaderError` / `CACHE_MAGIC` not found).

- [ ] **Step 3: Implement the header parser**

Replace the contents of `projects/gore-as/crates/gore_as/src/cache/header.rs`:

```rust
//! Outer header of `PrecompiledScript_Shipping.Cache`.
//!
//! Layout (little-endian):
//! - `0x00..0x10`  16-byte validation hash
//! - `0x10..0x14`  u32 magic = `0x9e377abe`
//! - `0x14..0x18`  u32 type/entry count
//! - `0x18..`      per-type records (see `cache::scan` / follow-up decode)

use thiserror::Error;

/// Magic at offset `0x10` of the precompiled cache.
pub const CACHE_MAGIC: u32 = 0x9e37_7abe;

/// Parsed outer header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheHeader {
    /// 16-byte validation hash (`0x00..0x10`).
    pub hash: [u8; 16],
    /// Magic word; must equal [`CACHE_MAGIC`].
    pub magic: u32,
    /// Number of type/entry records that follow the header.
    pub type_count: u32,
}

/// Errors from [`CacheHeader::parse`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum HeaderError {
    #[error("cache too short: need {need} bytes, got {got}")]
    TooShort { need: usize, got: usize },
    #[error("bad cache magic: got {got:#010x}, expected {expected:#010x}")]
    BadMagic { got: u32, expected: u32 },
}

impl CacheHeader {
    /// Byte length of the outer header.
    pub const SIZE: usize = 24;

    /// Parse the outer header from the start of the cache bytes.
    pub fn parse(bytes: &[u8]) -> Result<Self, HeaderError> {
        if bytes.len() < Self::SIZE {
            return Err(HeaderError::TooShort {
                need: Self::SIZE,
                got: bytes.len(),
            });
        }
        let mut hash = [0u8; 16];
        hash.copy_from_slice(&bytes[0..16]);
        let magic = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        let type_count = u32::from_le_bytes(bytes[20..24].try_into().unwrap());
        if magic != CACHE_MAGIC {
            return Err(HeaderError::BadMagic {
                got: magic,
                expected: CACHE_MAGIC,
            });
        }
        Ok(CacheHeader {
            hash,
            magic,
            type_count,
        })
    }
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p gore_as --test header_test`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add projects/gore-as/crates/gore_as/src/cache/header.rs projects/gore-as/crates/gore_as/tests/header_test.rs
git commit -m "feat(gore-as): decode outer cache header"
```

---

### Task 4: `decode-header` CLI subcommand (TDD)

**Files:**
- Modify: `projects/gore-as/crates/gore_as/src/bin/gore-as.rs`
- Create: `projects/gore-as/crates/gore_as/tests/cli_test.rs`

- [ ] **Step 1: Write the failing CLI test**

Create `projects/gore-as/crates/gore_as/tests/cli_test.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;

fn fixture_path() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/cache_head_8k.bin").to_string()
}

#[test]
fn decode_header_prints_values() {
    Command::cargo_bin("gore-as")
        .unwrap()
        .args(["decode-header", &fixture_path()])
        .assert()
        .success()
        .stdout(contains("magic      : 0x9e377abe"))
        .stdout(contains("type_count : 7264"))
        .stdout(contains("d54f0ffb10c1054b99f11446a43ed5dc"));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p gore_as --test cli_test`
Expected: FAIL — the stub bin prints the placeholder, not the header values.

- [ ] **Step 3: Implement the CLI**

Replace `projects/gore-as/crates/gore_as/src/bin/gore-as.rs`:

```rust
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
```

Note: this references `scan_strings` (Task 5). Until Task 5 lands, comment out the `Walk` arm body and the `use ... scan_strings;` line, or do Task 5 first. Recommended order: Task 5 before building the bin. If building now, temporarily replace the `Walk` arm with `Cmd::Walk { .. } => unimplemented!("Task 5"),` and drop the scan import.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p gore_as --test cli_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add projects/gore-as/crates/gore_as/src/bin/gore-as.rs projects/gore-as/crates/gore_as/tests/cli_test.rs
git commit -m "feat(gore-as): decode-header CLI subcommand"
```

---

### Task 5: Type-name scanner harness (TDD)

**Files:**
- Modify: `projects/gore-as/crates/gore_as/src/cache/scan.rs`
- Create: `projects/gore-as/crates/gore_as/tests/scan_test.rs`

- [ ] **Step 1: Write the failing test**

Create `projects/gore-as/crates/gore_as/tests/scan_test.rs`:

```rust
use gore_as::cache::header::CacheHeader;
use gore_as::cache::scan::scan_strings;

fn fixture() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/cache_head_8k.bin"
    ))
    .expect("fixture present")
}

#[test]
fn finds_known_type_names() {
    let bytes = fixture();
    let found = scan_strings(&bytes, CacheHeader::SIZE, 50);
    let texts: Vec<&str> = found.iter().map(|s| s.text.as_str()).collect();
    assert!(texts.contains(&"AI.AIItemScoring"), "got {texts:?}");
    assert!(
        texts.contains(&"UGothicAIItemActionScoringEntry"),
        "got {texts:?}"
    );
}

#[test]
fn first_hit_is_at_header_end() {
    let bytes = fixture();
    let found = scan_strings(&bytes, CacheHeader::SIZE, 1);
    assert_eq!(found[0].offset, 0x18);
    assert_eq!(found[0].text, "AI.AIItemScoring");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p gore_as --test scan_test`
Expected: FAIL to compile (`scan_strings` / `ScannedString` not found).

- [ ] **Step 3: Implement the scanner**

Replace `projects/gore-as/crates/gore_as/src/cache/scan.rs`:

```rust
//! Tolerant length-prefixed string scanner.
//!
//! The per-type record layout after the header is not yet fully reverse-
//! engineered. This scanner walks `u32`-length-prefixed ASCII names and resyncs
//! by advancing one byte when a candidate is not a plausible name. It is an
//! investigation aid for mapping the type table, NOT a format-accurate parser.

/// A name found by [`scan_strings`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedString {
    /// Byte offset of the `u32` length prefix.
    pub offset: usize,
    /// The length prefix value (bytes of the name, may include a trailing NUL).
    pub len: u32,
    /// Decoded text with any trailing NUL stripped.
    pub text: String,
}

/// Scan up to `max` length-prefixed ASCII names starting at `start`.
pub fn scan_strings(bytes: &[u8], start: usize, max: usize) -> Vec<ScannedString> {
    let mut out = Vec::new();
    let mut o = start;
    while o + 4 <= bytes.len() && out.len() < max {
        let len = u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
        if (1..=256).contains(&len) && o + 4 + len as usize <= bytes.len() {
            let raw = &bytes[o + 4..o + 4 + len as usize];
            if is_plausible_name(raw) {
                let text = String::from_utf8_lossy(raw)
                    .trim_end_matches('\0')
                    .to_string();
                out.push(ScannedString {
                    offset: o,
                    len,
                    text,
                });
                o += 4 + len as usize;
                continue;
            }
        }
        o += 1;
    }
    out
}

fn is_plausible_name(raw: &[u8]) -> bool {
    let body = raw.strip_suffix(b"\0").unwrap_or(raw);
    if body.is_empty() {
        return false;
    }
    body.iter()
        .all(|&c| c == b'.' || c == b'_' || c == b':' || c.is_ascii_alphanumeric())
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p gore_as --test scan_test`
Expected: PASS (2 tests).

- [ ] **Step 5: Verify the full bin + tests build together**

Run: `cargo test -p gore_as`
Expected: all tests PASS (smoke, header x3, cli, scan x2). If you stubbed the `Walk` arm in Task 4, restore the real `scan_strings` body now.

- [ ] **Step 6: Commit**

```bash
git add projects/gore-as/crates/gore_as/src/cache/scan.rs projects/gore-as/crates/gore_as/tests/scan_test.rs projects/gore-as/crates/gore_as/src/bin/gore-as.rs
git commit -m "feat(gore-as): tolerant type-name scanner + walk subcommand"
```

---

### Task 6: Run `walk` on the full cache and capture the type-name map

**Files:**
- Create: `projects/gore-as/FORMAT.md`

- [ ] **Step 1: Run the scanner over the full 122 MB cache**

Run (adjust the path to your install):

```bash
cargo run -p gore_as --release --bin gore-as -- walk \
  "/d/SteamLibrary/steamapps/common/Gothic 1 Remake/G1R/Script/PrecompiledScript_Shipping.Cache" \
  --max 200 | head -60
```

Expected: a list of `0x........ len=NN <Name>` lines beginning at `0x00000018` with `AI.AIItemScoring`, followed by many namespaced AS class names (`AI.*`, item/NPC/quest types) and `U...` native class names.

- [ ] **Step 2: Record confirmed format facts**

Create `projects/gore-as/FORMAT.md` capturing what is now known. Fill the observation bullets from the Step 1 output (replace the example lines with real ones you see):

```markdown
# PrecompiledScript_Shipping.Cache — format notes

Source: Gothic 1 Remake (Steam appid 1297900), Hazelight UnrealEngine-Angelscript fork.
Build root in shipped exe strings: `D:\P4J\Gothic1Remake\G1R\Plugins\Angelscript\...`.

## Outer header (confirmed)
| Offset | Type      | Value (sample)                      | Meaning            |
|--------|-----------|-------------------------------------|--------------------|
| 0x00   | u8[16]    | d54f0ffb10c1054b99f11446a43ed5dc    | validation hash    |
| 0x10   | u32 LE    | 0x9e377abe                          | magic              |
| 0x14   | u32 LE    | 7264                                | type/entry count   |
| 0x18   | records   | —                                   | per-type records   |

## Per-type record (PARTIAL — to resolve in follow-up)
Observed at 0x18: u32 len=17, "AI.AIItemScoring\0"; then a second name field;
then a size field (e.g. 0x5a=90) preceding "UGothicAIItemActionScoringEntry" (len 31).
Open: exact record field order, what the size fields bound, where per-module
AngelScript bytecode begins. (Resolved by the container-parse follow-up plan.)

## Sibling file
`Binds.Cache` (~5.9 MB) — native binding data; likely needed for full type resolution.
```

- [ ] **Step 3: Commit**

```bash
git add projects/gore-as/FORMAT.md
git commit -m "docs(gore-as): record confirmed cache format facts"
```

---

### Task 7: Pin the AngelScript core version (investigation)

**Files:**
- Modify: `projects/gore-as/FORMAT.md`

This task has no unit test — it is discovery whose deliverable is a recorded, sourced finding. Pin the exact AngelScript core version so a follow-up plan can build a byte-compatible `libangelscript`.

- [ ] **Step 1: Determine the UE engine version**

The shipping exe mangles plain version strings. Try, in order, and record whichever yields a version:

```bash
# a) file/product version of the exe
powershell -Command "(Get-Item 'D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe').VersionInfo | Format-List ProductVersion,FileVersion"
# b) any embedded engine tag
strings -n 5 "D:/SteamLibrary/steamapps/common/Gothic 1 Remake/G1R/Binaries/Win64/G1R-Win64-Shipping.exe" | grep -iE 'UE5|Unreal Engine 5|Release-5\.' | sort -u | head
# c) UE4SS log often prints the engine version on launch
cat "D:/SteamLibrary/steamapps/common/Gothic 1 Remake/G1R/Binaries/Win64/ue4ss/UE4SS.log" 2>/dev/null | grep -iE 'engine version|5\.[0-9]' | head
```

Expected: a UE5 minor version (e.g. `5.3` / `5.4`).

- [ ] **Step 2: Map UE version → Hazelight fork → AngelScript core version**

The Hazelight fork pins AngelScript per engine branch. From `github.com/Hazelight/UnrealEngine-Angelscript`, find the branch/tag matching the UE version from Step 1, then read the bundled AngelScript core version from `Plugins/Angelscript/Source/ThirdParty/AngelScript/.../as_config.h` (`#define ANGELSCRIPT_VERSION`). Record the fork commit/branch and the `ANGELSCRIPT_VERSION` integer.

- [ ] **Step 3: Cross-check against the cache magic**

Note in `FORMAT.md` whether the cache magic `0x9e377abe` corresponds to a Hazelight precompiled-data version constant (search the fork source for `9e377abe` / the precompiled-data serializer version). This disambiguates the container version from the AngelScript bytecode version.

- [ ] **Step 4: Record the finding**

Append to `projects/gore-as/FORMAT.md`:

```markdown
## Versions (pinned)
- UE engine version: <from Step 1, with source>
- Hazelight fork branch/commit: <from Step 2>
- AngelScript core ANGELSCRIPT_VERSION: <int from as_config.h>
- Container/precompiled-data version: <magic 0x9e377abe meaning, from Step 3>
```

- [ ] **Step 5: Commit**

```bash
git add projects/gore-as/FORMAT.md
git commit -m "docs(gore-as): pin AngelScript + engine versions"
```

---

## Self-review

- **Spec coverage (this slice):** M1 verifiable outer-header decode → Tasks 3–4; "what scripts exist" inventory harness → Tasks 5–6; M0 version pinning → Task 7. Crate placement (`projects/gore-as`, reuse conventions) → Tasks 1–2. Deeper spec items (full container parse, disasm, round-trip M3, offline compiler M4, injector M2, Tier 2 M5) are explicitly deferred below — not silent gaps.
- **No placeholders:** every code step shows complete, compilable code; investigation steps (Task 7) give exact commands + a concrete deliverable template, not "figure it out".
- **Type consistency:** `CacheHeader { hash, magic, type_count }`, `CacheHeader::SIZE` (24), `CACHE_MAGIC` (`0x9e377abe`), `HeaderError::{TooShort,BadMagic}`, `scan_strings(bytes,start,max) -> Vec<ScannedString>`, `ScannedString { offset, len, text }` — names used identically in lib, bin, and tests.

## Follow-up plans (not in this plan — written after the harness surfaces facts)

1. **Container parse + bytecode disassembly (completes M1).** Full per-type record layout, per-module raw AngelScript bytecode extraction, opcode disassembler. Needs Task 6/7 output.
2. **Round-trip validator (M3).** Re-serialize an unmodified module byte-identical; nail the 16-byte hash scheme.
3. **Injection PoC (M2).** UE4SS mod: locate `asIScriptEngine*` via `FAngelscriptManager`, `LoadByteCode` a trivial blob, invoke it in-game. Parallelizable — depends only on M0 version pin, not on full decode.
4. **Offline compiler (M4, completes Tier 1).** Version-matched `libangelscript` host + SDK-dump stub-registration codegen → compile `.as` → blob.
5. **Tier 2.** New UE-reflected script class via reproduced container metadata + class-gen hooks.
