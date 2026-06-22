# gore-as AngelScript Decompiler (edit existing scripts) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans / subagent-driven-development. Steps use `- [ ]`. Multi-session; build stage by stage, validate each against the known-source Rosetta before moving on.

**Goal:** Read existing compiled AngelScript back to editable form so a modder can CHANGE existing behavior (not only add). End workflow: **decompile a module → edit `.as` → game recompiles it (`-as-generate-precompiled-data`) → `gore_as` REPLACES that module in the 122 MB cache**. Run normal mode.

**Why now / context:** We can already ADD modules (case-a/case-b splice, Tier-2 proven: a spliced `.as` class is a live `/Script/Angelscript.*` UClass). Editing existing logic is the missing piece. The cache stores compiled bytecode (`TArray<int32> ByteCode` per function, decoded by the walker), not source — so editing needs decompilation.

**Architecture:** Extend the `gore_as` crate. New modules: `cache::isa` (the AngelScript 23300 opcode table), `cache::disasm` (bytecode → opcode listing), later `decompile::*` (opcodes → readable `.as`). Plus a `splice::replace_module` (swap a module's bytes in place). Validate everything against the **richtest Rosetta** whose `.as` source we wrote (so we know the expected decompilation).

**Tech Stack:** Rust (existing crate). Reference: `WillGordon9999/UNREANGEL@main` AngelScript core (`as_bytecode.*`, `as_restore.cpp`, `as_compiler.cpp`).

**Honest scope note:** A *full, clean* decompiler (bytecode → original-quality `.as`) is large and lossy — AngelScript is a stack VM; control-flow + expression recovery is real work and output won't byte-match the original source. Stages D1–D2 are high-value and achievable on their own (read + targeted edits). D3 (full decompile) is the stretch; partial/annotated output is still useful.

---

## Stage D1 — Disassembler (bytecode → asBC opcode listing)

Prereq: `cache::isa` (opcode table) — being produced by a background agent into
`work/reversing/gore-as/findings/bytecode-isa.md` + `src/cache/isa.rs`.

### Task D1.1: Wire in `isa` + a bytecode-stream reader
**Files:** Modify `src/cache/mod.rs` (add `pub mod isa; pub mod disasm;`); Create `src/cache/disasm.rs`; Test `tests/disasm_test.rs`.

- [ ] **Step 1:** Add `pub mod isa;` and `pub mod disasm;` to `cache/mod.rs` (after the isa.rs file from the agent is present; `cargo build -p gore_as` must compile).
- [ ] **Step 2 (failing test):** In `disasm_test.rs`, decode the richtest sample's `GoreTestClass::method1` ByteCode and assert the disassembly is non-empty and ends with a `RET`-family opcode. (Get the function's ByteCode bytes by extending the walker to RETURN per-function ByteCode ranges — see Task D1.2.)
- [ ] **Step 3:** Implement `disasm.rs`: `fn disassemble(bytecode: &[i32]) -> Vec<Instr>` where `Instr { offset_dw: usize, op: &'static OpInfo, operands: Operands }`. Walk the dword stream: opcode = low byte of `bytecode[i]`; look up `op_info`; consume `op.size_dwords` dwords; decode operands per `op.fmt` (W=bits16..32 of the opcode dword; DW=next dword; QW=next two dwords; rW=16-bit var slot). Provide a `Display` that prints `0xNNNN  MNEMONIC operands`.
- [ ] **Step 4:** Run the test; iterate until method1 disassembles cleanly (no unknown opcode, consumes the whole array). method1 source = `int method1(int a, float b){ return a + field1; }` → expect param/member loads, an integer ADD, and a return. Eyeball the listing matches.
- [ ] **Step 5:** Commit `feat(gore-as): bytecode disassembler (asBC listing)`.

### Task D1.2: Expose per-function ByteCode from the walker
**Files:** Modify `src/cache/walk_modules.rs`; Test in `disasm_test.rs`.

- [ ] Add a variant of the walker (or a `walk_collect` returning a structure) that records, per module/function, the `(name, bytecode_dword_range)` so the disassembler can fetch a function's `&[i32]`. Keep the fast `module_region_end` for splicing. TDD: assert richtest yields `GoreFreeFn`, `GoreTestClass` (module funcs) + `method1`, ctor (class funcs) with non-empty ByteCode for method1.

### Task D1.3: Resolve references in the listing
**Files:** Modify `src/cache/disasm.rs`; Test `tests/disasm_test.rs`.

- [ ] For opcodes carrying a TYPE id / FUNCTION id / GLOBAL ptr / STRING const / jump offset (per `bytecode-isa.md`), resolve to names via the tail tables (`tables.rs`: TypeIdReferenceToPointer→TypeReferences→Name, FunctionIdReferenceToPointer→FunctionReferences→Name) and annotate the listing (e.g. `CALL <func: opAdd>` , `ADDSi <member: field1>`). Validate against richtest method1 (`a + field1`, return int) and the ctor.

---

## Stage D2 — Module-replace splice (swap a module in place)

**Files:** `src/cache/splice.rs` (add `replace_module`); Test `tests/splice_test.rs`.

- [ ] `fn replace_module(base: &[u8], new_mini: &[u8], target_name: &str) -> Result<Vec<u8>>`: find `target_name`'s module byte range in `base` (extend the walker to return per-module ranges), replace it with `new_mini`'s module bytes, keep the `Modules` count the SAME, and merge `new_mini`'s tail-table entries (case-a merge, dedup engine ids) while leaving the replaced module's now-orphaned old table entries (harmless, name-resolved) OR pruning them (stretch). Validate: replace a module in a synthetic base, re-walk to same count, tail tables parse to EOF.
- [ ] This enables the edit workflow WITHOUT a full decompiler if the user hand-edits via override; with the decompiler it closes the loop.

---

## Stage D3 — Decompiler (opcodes → readable `.as`) — STRETCH, research-heavy

Build incrementally; partial output is useful. Validate every step against richtest (known source).

- [ ] **D3.1 Basic blocks + control-flow graph:** split the instruction list at jump targets/branches; recover the CFG.
- [ ] **D3.2 Stack-based expression reconstruction:** simulate the asBC value stack to rebuild expressions (loads/consts/arith/calls → `a + field1`). AngelScript is register/stack hybrid (var slots `rW`); map var slots to locals/params using the function's variable info.
- [ ] **D3.3 Control-flow structuring:** CFG → `if/else`, `while`, `for`, `return` (pattern-match the common shapes the AS compiler emits).
- [ ] **D3.4 Types/signatures:** function signature + locals from the function record (params/return DataTypes, ObjVariableTypes) + resolved names.
- [ ] **D3.5 Emit `.as`:** print a compilable-ish module. Round-trip test: decompile richtest → feed back through the game's `-as-generate-precompiled-data` → compare structure (won't byte-match; check it compiles + behaves).

---

## Validation strategy (all stages)
- **Golden source = richtest** (`samples/_gore_richtest.as` ↔ `samples/PrecompiledScript.richtest.Cache`): we KNOW the source, so disasm/decompile output can be judged. Also the 314 B minimal (one `void(){}`).
- For real-cache spot checks: decompile a small, recognizable real module and sanity-check.

## Out of scope / deferred
- Byte-perfect source recovery (impossible/lossy — comments, names of locals, formatting are gone).
- A bytecode ASSEMBLER (edit at opcode level then re-emit bytecode) — only if recompile-via-game proves insufficient. The chosen edit path is decompile→edit source→game-recompile→replace, reusing the game as the compiler (as in the splice pipeline).
