# gore-as Splice-Repacker (Weg B) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans / subagent-driven-development. Steps use `- [ ]`.

**Goal:** Add a new AngelScript module to the shipped `PrecompiledScript_Shipping.Cache` so the game runs the mod in NORMAL (non-dev) mode — saves intact, no launch flag — by byte-splicing a game-compiled module blob into the 122 MB cache.

**Why this shape:** Empirically established 2026-06-20/21 (see `work/reversing/gore-as/JOURNAL.md` + `findings/container-splice.md`):
- The shipped exe has the AngelScript compiler; `-as-development-mode` compiles loose `.as` from `G1R/Script/` — but dev-mode CANNOT play (New Game dead) and breaks saves. So dev-mode is **compile-only**.
- `-as-generate-precompiled-data` makes the game compile loose `.as` into a mini `PrecompiledScript.Cache` (proven: 314 B for one fn). **The game is our compiler — no offline AngelScript compiler needed.**
- The cache load path has **zero integrity checks**; only gate is the build-constant `BuildIdentifier = 0x9E377ABE` (already matches). Global ref tables are content-addressed by `int64 OldReference`, not dense index → appending a primitive-only module needs **no table edits**.

**Pipeline:** author `.as` → game compiles it to mini-cache `M` (`-as-generate-precompiled-data`) → `gore_as splice C M` inserts `M`'s module before `C`'s tail, bumps `Modules` count → replace `_Shipping` → run normal mode.

**Architecture:** Extend the existing `gore_as` crate (`projects/gore-as/crates/gore_as`). Add a bool=4-correct streaming **container walker** that locates `TAIL_OFF` (end of the last module), and a **splice** command. Reuse `cache::header`.

**Tech Stack:** Rust (existing crate), clap, thiserror, anyhow.

**Spec/decode references:** `work/reversing/gore-as/findings/container-splice.md` (§1 top-level, §2 per-module, §3 byte-mapped Rosetta, §5 splice algorithm, §9 substruct layouts), sample `work/reversing/gore-as/samples/PrecompiledScript.minimal-1fn.Cache`.

---

## Dependencies before coding
- **§9 substruct layouts** (FAngelscriptPrecompiledClass/Enum/GlobalVariable/FunctionImport + UFUNCTION branch) — a background agent is producing these into `container-splice.md`. Needed for the walker to traverse real modules (which have classes/UFUNCTIONs).
- **Richer Rosetta sample** — `work/reversing/gore-as/samples/_gore_richtest.as` (prepared) compiled via the game to byte-verify class/enum/global/param substructs. Generate steps in that file's header.

---

### Task 1: Encoding primitives (bool=4, SIA, TArray) — TDD

**Files:** Create `projects/gore-as/crates/gore_as/src/cache/wire.rs`; Test `tests/wire_test.rs`. Modify `src/cache/mod.rs` (add `pub mod wire;`).

- [ ] **Step 1: Failing test** — a `Cursor` reader over bytes with: `read_u32`, `read_i64`, `read_bool4` (4-byte int32, asserts 0/1), `read_sia` (FStringInArchive: i32 len; if len!=0 read len+1 bytes incl NUL, return string sans NUL), `skip_tarray_i32` (i32 count + count*4 bytes). Test against hand-built byte fixtures.

```rust
// tests/wire_test.rs
use gore_as::cache::wire::Cursor;
#[test]
fn reads_sia_and_bool4() {
    // SIA "ab" = len=3 (incl NUL) + "ab\0"; then bool4=1
    let mut b = Vec::new();
    b.extend_from_slice(&3i32.to_le_bytes()); b.extend_from_slice(b"ab\0");
    b.extend_from_slice(&1i32.to_le_bytes());
    let mut c = Cursor::new(&b);
    assert_eq!(c.read_sia().unwrap(), "ab");
    assert_eq!(c.read_bool4().unwrap(), true);
    assert_eq!(c.pos(), b.len());
}
#[test]
fn empty_sia_is_four_bytes() {
    let b = 0i32.to_le_bytes();
    let mut c = Cursor::new(&b);
    assert_eq!(c.read_sia().unwrap(), "");
    assert_eq!(c.pos(), 4);
}
```

- [ ] **Step 2** run → fail. **Step 3** implement `Cursor` in `wire.rs` (a `{buf, pos}` with the readers; bounds-checked, `thiserror` `WireError`). **Step 4** run → pass. **Step 5** commit `feat(gore-as): wire primitives (bool=4, SIA, TArray)`.

---

### Task 2: Module walker → locate TAIL_OFF — TDD against the Rosetta sample first

**Files:** Create `src/cache/walk_modules.rs` (+ `pub mod`); Test `tests/walk_modules_test.rs`.

Implement `fn module_region_end(bytes: &[u8]) -> Result<usize, WireError>`: parse header (skip 0x14), read `Modules` count, then for each module walk every field per `container-splice.md` §2/§3/§9 (Functions incl. UFUNCTION branch, Classes, Enums, GlobalVariables, FunctionImports, CodeHash, the SIA/TArray tail fields). Return the offset after the last module = `TAIL_OFF`.

- [ ] **Step 1: Failing test** — on the 314 B sample, `module_region_end` returns `0x11e` (286) and the remaining bytes `[0x11e..]` are exactly 28 zero bytes (the 7 empty tail tables), ending at EOF 314.

```rust
// tests/walk_modules_test.rs
use gore_as::cache::walk_modules::module_region_end;
fn sample() -> Vec<u8> {
    std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"),
      "/../../../work/reversing/gore-as/samples/PrecompiledScript.minimal-1fn.Cache")).expect("sample")
}
#[test]
fn finds_tail_in_minimal_sample() {
    let b = sample();
    let tail = module_region_end(&b).unwrap();
    assert_eq!(tail, 0x11e);
    assert!(b[tail..].iter().all(|&x| x == 0), "tail must be the 7 empty tables");
    assert_eq!(b[tail..].len(), 28);
}
```

(If the sample path via `..` is awkward in CI, copy the sample into `tests/fixtures/` in this task's step 0.)

- [ ] **Step 2-4:** implement per §9; run until the minimal-sample test passes.
- [ ] **Step 5: Validate against the REAL cache (the decisive correctness gate).** Add an ignored/opt-in test gated on env `GORE_AS_REAL_CACHE` that runs `module_region_end` on the 122 MB cache and asserts: it consumes exactly 7264 modules AND, continuing to parse the 7 tail tables from `TAIL_OFF`, lands EXACTLY at EOF. If it desyncs, the substruct layout is wrong → fix §9 (use the richer Rosetta). Document the run: `GORE_AS_REAL_CACHE="D:\\...\\PrecompiledScript_Shipping.Cache" cargo test -p gore_as --test walk_modules_test -- --ignored`.
- [ ] **Step 6** commit `feat(gore-as): module walker locates tail offset`.

---

### Task 3: `splice` command — TDD

**Files:** Create `src/cache/splice.rs` (+ `pub mod`); add `Splice` subcommand to `src/bin/gore-as.rs`; Test `tests/splice_test.rs`.

`fn splice(base: &[u8], mini: &[u8]) -> Result<Vec<u8>, SpliceError>`:
1. `base_tail = module_region_end(base)`; `mini_tail = module_region_end(mini)`.
2. Assert `mini[mini_tail..]` is all-zero (mini references no global types) else `SpliceError::MiniHasRefs`.
3. `mod_bytes = mini[0x18..mini_tail]` (the single module entry).
4. `base_count = u32@0x14`; output = `base[0..0x14]` ++ `(base_count+1)` ++ `base[0x18..base_tail]` ++ `mod_bytes` ++ `base[base_tail..]`.

- [ ] **Step 1: Failing test** — splice the minimal sample as `mini` into a synthetic tiny `base` (build a 1-module base by hand or reuse the sample as base too): assert output `Modules`@0x14 == base+1, output parses with `module_region_end` to a tail whose remaining bytes equal base's original tail, and `mod_bytes` appears immediately before the tail.

```rust
// tests/splice_test.rs
use gore_as::cache::{splice::splice, walk_modules::module_region_end};
fn sample() -> Vec<u8> { /* same loader as walk_modules_test */ }
#[test]
fn splice_increments_count_and_preserves_tail() {
    let base = sample(); let mini = sample();
    let out = splice(&base, &mini).unwrap();
    assert_eq!(u32::from_le_bytes(out[0x14..0x18].try_into().unwrap()), 2);
    let tail = module_region_end(&out).unwrap();
    assert!(out[tail..].iter().all(|&x| x == 0));
}
```

- [ ] **Step 2-4** implement; pass. **Step 5** CLI: `gore-as splice <base.Cache> <mini.Cache> -o <out.Cache>` (read both, write output; refuse if mini name collides with an existing module — scan module names). **Step 6** commit `feat(gore-as): cache splice command`.

---

### Task 4: In-game validation (manual, decisive)

- [ ] **Step 1:** Back up `PrecompiledScript_Shipping.Cache` (already have `.goreas-bak`).
- [ ] **Step 2:** Author a real-ish but primitive-only mod `.as` (start with the bake-marker; later a function that calls a known native global). Generate its mini-cache: drop in `G1R/Script/`, Steam launch options `-as-development-mode -as-generate-precompiled-data`, launch (writes `PrecompiledScript.Cache`, exits), then CLEAR launch options.
- [ ] **Step 3:** `gore-as splice "<_Shipping.Cache>" "<PrecompiledScript.Cache>" -o "<_Shipping.Cache.new>"`; replace `_Shipping` with the spliced file (keep the bak).
- [ ] **Step 4:** Remove the loose `.as` and the dev `PrecompiledScript.Cache` from `G1R/Script/` (so normal mode uses only `_Shipping`). Launch the game NORMALLY (no flags).
- [ ] **Step 5:** Verify: (a) game boots, (b) **existing saves still load** (Continue/Load work — confirms splice didn't change save identity), (c) the spliced module is present (callable / observable — TBD how to observe; at minimum no crash + boot proves the loader accepted a 7265-module cache).
- [ ] **Step 6:** Restore `_Shipping` from `.goreas-bak` after the test. Record the result in the journal.

---

## Self-review
- Coverage: walker (§2/§9) → Task 2; splice (§5) → Task 3; load acceptance (§6) → Task 4 in-game; primitives (§0) → Task 1.
- Open risk carried from decode: per-module substruct widths (§9) are HYPOTHESIS until Task 2 Step 5 syncs to EOF on the real cache; the richer Rosetta is the fallback to nail them.
- Out of scope (deferred): editing existing modules; modules that reference NEW game types (case-(a) table merge, §7); Tier-2 UCLASS content. Add as a follow-up plan once the primitive-only splice loads in-game.
