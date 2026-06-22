# gore-as — AngelScript decode + runtime script injection

**Date:** 2026-06-20
**Status:** Design approved (pending written-spec review)
**Project:** `projects/gore-as` (new monorepo sibling)

## Goal

Author new AngelScript and run it in the shipping Gothic 1 Remake via a self-contained
offline toolchain. The shipped executable has the AngelScript **bytecode loader** but not
the **compiler**, so we generate bytecode ourselves and inject it at runtime through UE4SS.

Two capability tiers, both in scope, sequenced:

- **Tier 1 (build first):** Inject plain AngelScript functions/classes that call
  already-registered native APIs and manipulate any `UObject` at runtime. Ceiling ≈ the
  existing UE4SS-Lua + CDO runtime-modding path, but written in the game's own language
  with type-safe access to its real API. This is the unavoidable foundation for Tier 2.
- **Tier 2 (committed, follows Tier 1):** Add new **UE-reflected script classes** the
  engine's own systems treat as first-class content (new item / quest / dialog the
  merchant lists, quest tracker, and save system recognize). Requires reproducing
  Hazelight's container metadata + class-generation hooks. Higher risk; feasibility
  confirmed during decode (M1) before heavy investment.

## Target facts (grounded)

- Engine: UE5 + **Hazelight UnrealEngine-Angelscript** fork. Build root in shipped strings:
  `D:\P4J\Gothic1Remake\G1R\Plugins\Angelscript\Source\AngelscriptCode\` (their internal
  Perforce — fork + game C++ module are **not public**).
- Shipped exe: `G1R\Binaries\Win64\G1R-Win64-Shipping.exe`. UE4SS pre-installed in
  `G1R\Binaries\Win64\ue4ss\` (dwmapi.dll loader).
- Scripts: `G1R\Script\PrecompiledScript_Shipping.Cache`, **122,877,404 bytes**.
  - Bytes 0x00–0x0F: 16-byte validation hash header (`d54f0ffb 10c1054b 99f11446 a43ed5dc`).
  - Then length-prefixed (length includes NUL) type-name table: e.g. `AI.AIItemScoring`
    (namespaced AS class), `UGothicAIItemActionScoringEntry` (UE native class).
  - Sibling: `G1R\Script\Binds.Cache` (~5.9 MB) — native binding data, likely needed for
    full type resolution.
- SDK reference (already produced via UE4SS GenerateSDK): `ue4ss\CXXHeaderDump\`, 1163 .hpp;
  `Angelscript.hpp` (~4.4 MB) = all AS classes; `G1R.hpp` = native C++ module.
- Delivery decided: **runtime injection via UE4SS** (do NOT repack the shipped file). The
  16-byte header therefore matters for *understanding/round-trip*, not for delivery.

## Why approach A (offline compiler)

Rejected alternatives:
- **Hazelight editor build** — would give guaranteed-correct bytecode and free `shared`/
  cross-module handling, but Gothic's specific fork + game C++ module are not public;
  reconstructing a bootable project from the SDK dump alone is likely infeasible.
- **Hand-written bytecode assembler** — total control, no compiler dependency, but brutal
  beyond trivial functions; no type checking. Unfit for "add new classes."

Approach A is the only path that is both self-contained and scales to real new classes.

**Key enabler:** AngelScript `SaveByteCode`/`LoadByteCode` stores type and function
references by **name + signature**, re-linked against the live engine at load (asCReader).
So the offline compiler must match the game's **names/signatures** (which the SDK dump
provides) — not its internal registration order or pointer layout.

## Components

1. **Decoder** (Rust, reuses `gore-core` UE-property parser where useful)
   Parse `.Cache`: header → Hazelight `FAngelscriptPrecompiledData` container → module
   table, type-name table, per-module raw AS bytecode + metadata. Emit JSON + bytecode
   disassembly. Pins the AngelScript core version, documents the hash scheme, and maps the
   container metadata that Tier 2 will need to reproduce.

2. **Round-trip validator**
   Re-serialize an unmodified module byte-identical (or loader-accepted). Go/no-go gate
   proving we truly understand the format.

3. **Registration codegen**
   Parse the SDK dump (`Angelscript.hpp` + `G1R.hpp`, cross-checked against `Binds.Cache`)
   → generate native-type **stub** registrations (signatures only; no real implementation
   needed for compilation) for the offline engine.

4. **Offline compiler**
   Host app linking a **version-matched** `libangelscript` + the generated stubs. Compiles
   user `.as`, calls `SaveByteCode` → bytecode blob.

5. **Runtime injector** (UE4SS — C++ mod or Lua + native shim)
   Locate the live `asIScriptEngine*` via the `FAngelscriptManager` singleton (UE4SS
   reflection or signature scan), create a module, `LoadByteCode(blob)`, register/bind so
   the game can reach it, invoke entry points (timer/keybind/hook trigger).

## Milestones (ordered by risk reduction)

- **M0 — Spike.** Compile a trivial `.as` with a stock version-matched `libangelscript`,
  produce *any* valid bytecode blob. Cheap; unblocks M2.
- **M1 — Decode.** Readable dump of the 122 MB cache; pin AS version, hash scheme, module
  layout, **and assess Tier-2 container metadata reproducibility**. Also delivers a
  "what scripts exist" inventory. *Decision point for Tier 2 depth.*
- **M2 — Injection PoC.** Inject a trivial plain-AS function at runtime via UE4SS, call it,
  observe output in-game. Proves the hardest unknown (engine pointer + `LoadByteCode`
  acceptance) early.
- **M3 — Round-trip.** Re-encode one shipped module byte-identical. Format-mastery gate.
- **M4 — Offline compiler (Tier 1 complete).** Stub-registration codegen + compile real
  `.as` referencing native APIs → working injected Tier-1 class/function.
- **M5 — Tier 2 (committed).** New UE-reflected script class the game instantiates as real
  content. Scope/approach finalized from M1 findings. Requires reproducing container
  metadata + Hazelight class-generation hooks.

## Testing

- **Decode:** golden test vs the real cache — parse clean; round-trip byte-identical on
  sampled modules.
- **Compiler:** compile → `LoadByteCode` in our own version-matched engine → assert it
  links and executes offline (no game needed).
- **Injection:** manual in-game verification — call the injected function, observe a log
  line or visible effect.
- **Tier 2:** in-game — new content appears in the relevant system (e.g. a new item buyable
  at a merchant / present in inventory) and survives a save/load round-trip.

## Risks & open questions (resolve during M1)

- **AS core version pinning** — bytecode format is version-locked. Must identify the exact
  Hazelight fork commit / AngelScript WIP version and match `libangelscript` to it. A
  mismatch makes `LoadByteCode` reject the blob.
- **16-byte hash scheme** — content hash (recompute) vs interface/version hash (copy). Not
  blocking for runtime inject, but needed for round-trip and any future repack.
- **Container metadata (Tier 2)** — how Hazelight maps AS classes → UClasses, property
  bindings, GAS attributes. Reproducibility here is the Tier-2 gate.
- **Cross-module / `shared` access** — whether new modules can reference existing AS
  gameplay classes (item/NPC/quest base classes) or only engine-registered native types.
  Determines how self-contained injected scripts must be.
- **Locating `asIScriptEngine*` at runtime** — via `FAngelscriptManager` singleton through
  UE4SS reflection, or AOB signature scan if not reflected.
- **Native signature completeness** — stubs must cover every signature referenced by
  compiled scripts; missing entries fail compilation. `Binds.Cache` may be required to fill
  gaps the headers miss.

## Documentation (standing rule)

All reversing findings are logged continuously to `work/reversing/gore-as/` —
`JOURNAL.md` (chronological, append-only, CONFIRMED/HYPOTHESIS/DEAD-END tags) plus
`findings/<topic>.md` per investigation thread. This folder is **local scratch
(gitignored)**; distilled *confirmed* facts graduate to the tracked
`projects/gore-as/FORMAT.md`. Parallel agents each own one `findings/<topic>.md`
(no write conflicts) and append a one-line summary to `JOURNAL.md`.

## Placement

New project `projects/gore-as`, sibling to `gore-{cli,core,dump,mod,save}`.
- Rust crates for decoder / round-trip / compiler-host; reuse `gore-core` parser.
- UE4SS injector as a separate native/Lua artifact under the same project.
- Any GUI folds into `gore-mod` later (shared DNA), not built now (YAGNI).
- Follows the monorepo per-project build (`build.py`) + prefixed-tag release convention.
