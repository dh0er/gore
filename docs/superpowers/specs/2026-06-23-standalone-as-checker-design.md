# Standalone AngelScript compile-checker for gore-as

**Status:** design (approved-pending review)
**Date:** 2026-06-23
**Scope:** Phase 1 — an offline compile-CHECKER. Replaces the game-launch loop used to
validate the decompiler's emitted `.as`. NOT a cache generator (that is a later phase).

## 1. Goal

Today, validating that the decompiler's emitted `.as` tree compiles requires launching
`G1R-Win64-Shipping.exe` with `-as-development-mode -as-generate-precompiled-data`, injecting a
DLL to scrape the compile diagnostics, and killing the process — ~5 minutes per iteration, not
CI-able, needs the game installed.

Replace that with `gore as check <dir-of-.as>`: embed the **real** AngelScript compiler (the
Hazelight UnrealEngine-Angelscript fork, the exact one the game uses), register the native API
surface offline, compile the `.as` tree, and emit diagnostics in the **same format** the game
loop produces. No game process, runs in CI, sub-second-to-seconds per run.

Non-goal (Phase 1): producing a game-loadable `PrecompiledScript_Shipping.Cache`. That needs
bytecode serialization + exact type-IDs and is deferred (see §10).

## 2. Key findings that make this feasible

(From the reverse-engineering research pass — recorded so the plan rests on evidence.)

- **The fork compiler is decoupled from live UE reflection.** It is lightly-patched vanilla
  AngelScript 2.33 (`asCBuilder`/`asCCompiler`/`asCScriptEngine`). Type resolution goes entirely
  through the engine's own registration tables (`GetObjectMethodDescriptions`,
  `FindMatchingFuncdef`, `asCObjectType`/`asCDataType`). UE reflection is consulted only at
  *registration time* (`BindScriptTypes()`), never inside `Build()`. The "UClass" special-cases
  in the compiler key on the *registered type's name string*, not a live UObject lookup. So a
  standalone build that registers the same surface compiles identically — pure vanilla-AS usage:
  `RegisterObjectType/Method/Property` → `AddScriptSection` → `Build` → message callback.
- **Compiler source is not shipped** (statically linked into the exe). Public mirror
  `WillGordon9999/UNREANGEL` is UE 5.4.x, `ANGELSCRIPT_VERSION 23300` ("2.33.0 WIP") — matches.
  AS core lives under `Angelscript/Source/AngelscriptCode/Public/source/angelscript/`.
- **Diagnostics already match.** The engine routes all compile errors through
  `asIScriptEngine::SetMessageCallback` → `asSMessageInfo { section, row, col, type, message }`
  = file:line:col + message, exactly what the game loop captures.
- **Binds.Cache covers ~99% of the registration surface** (11,189 types, 16,371 full signatures,
  properties+types, global functions, namespaces, templates). Property offsets and type-IDs are
  **not needed** for type-checking (dummy values are fine). Two gaps require a supplement:
  native **enum enumerators+values** (36 native enums) and native **base-class edges**; plus the
  **GameplayTag namespace** (synthesized at runtime from project tag data).
- **The emitted code uses fork-only syntax** — `UFUNCTION()`/`UPROPERTY()` (empty-arg only),
  `n"..."` FName literal, `Cast<T>()`, `TSubclassOf<>`/`TArray<>`/`TMap<>`, `super()`,
  `Type::StaticClass()`, `nullptr`, bare `U*`/`A*` references (no `@`). Stock AngelScript rejects
  most of these → the vendored compiler must be the fork (or vanilla + these patches).
- **FFI pattern already exists in-repo.** `crates/gore-oodle` links a large C++ codebase (`ooz`)
  via the `cc` crate + a hand-written `extern "C"` shim, no bindgen/cxx. Mirror it exactly.
- **The checker never executes scripts** — only `Build()`. Native methods can be registered with
  `asCALL_GENERIC` / dummy function pointers, avoiding the MASM calling-convention layer entirely.

## 3. Architecture

```
gore as check <dir-of-.as> [--binds Binds.Cache] [--ue4ss <dump>] [--format capture|native]
  │
  1. RegistrationSurface  (Rust)
  │     ├─ binds.rs  (EXTENDED: expose full decl strings + namespace, not just arity)
  │     │     → object types (ref/value), methods, properties, global fns, namespaces, templates
  │     ├─ supplements (UE4SS dump): native enum values, native base-class edges, gameplay tags
  │     └─ ue-as prelude: template types (TArray/TSubclassOf/TMap…), native value-type operators
  │
  2. C++ AngelScript engine  (vendored fork, built via cc + extern "C" shim)
  │     create engine (same engine properties as the game: floatIsFloat64, automaticImports…)
  │     → register the surface (asCALL_GENERIC / declaration-only)
  │     → module->AddScriptSection(relpath, source) for every .as
  │     → module->Build()
  │     → SetMessageCallback collects asSMessageInfo into a vector
  │
  3. Diagnostics  (Rust)
        drain messages → group by section → emit "=== file.as ===\n<error>…"
        (byte-compatible with work/reversing/gore-as/ashook/as_errors_capture.txt)
```

### Components

- **`crates/gore-as/vendor/angelscript/`** — vendored fork AS core (front-end + engine tables;
  no VM/JIT/callfunc-asm needed for a checker). Acquisition: §4.
- **`crates/gore-as/csrc/as_shim.cpp`** — `extern "C"` shim over the engine. ~15 functions
  (create/destroy, register_type/method/property/global/funcdef/enum_value, start_module,
  add_section, build, message_count/section/row/col/type/text). Mirrors `gore-oodle`'s
  `ooz_shim.cpp`.
- **`crates/gore-as/build.rs`** — `cc::Build` compiling the vendor `.cpp` + the shim (C++17,
  MSVC, warning suppressions like gore-oodle). New `[build-dependencies] cc`.
- **`crates/gore-as/src/check/engine.rs`** — safe Rust wrapper: `unsafe extern "C"` block + an
  `Engine` newtype with `Drop`, paralleling `gore-oodle/src/lib.rs`.
- **`crates/gore-as/src/check/registration.rs`** — builds the `RegistrationSurface` from
  `NativeApi` + supplements + prelude and drives the shim's register_* calls.
- **`crates/gore-as/src/check/supplements.rs`** — parses the UE4SS dump for native enum
  values + base-class edges; loads the gameplay-tag list.
- **`crates/gore-as/src/cache/binds.rs`** (extend) — retain & expose the full method/property
  **decl strings** and the function **namespace** (currently parsed then discarded after arity).
- **`crates/gore/src/cmd/as_cache.rs`** — new `AsCmd::Check` variant + `run` arm, reusing the
  existing `load_native_api()` helper.

## 4. Compiler source acquisition (try in order)

User decision: "whatever works best, try multiple if needed." Strategy:

1. **Primary — vendor from the public mirror** `WillGordon9999/UNREANGEL`: clone, extract the AS
   core (`Public/source/angelscript/` + `angelscript.h` + `as_config.h`), drop into
   `crates/gore-as/vendor/angelscript/`. It is AS 23300, matching the build.
2. **If the mirror diverges** from the shipped fork (it lacks the `0x9e377abe` container magic;
   the ABI is Hazelight-modified) such that the checker mis-reports vs. the oracle (§7): port
   the needed fork patches onto **vanilla AngelScript 2.33** (local samples already exist under
   `work/reversing/gore-as/_src/`) — the fork patches we need are the *parser/semantic*
   extensions (UFUNCTION/UPROPERTY tokens, `n"..."` literal, `Cast<>`, template handling, bare
   object refs, `nullptr`). We do NOT need the fork's UE-binding or precompiled-data writer.
3. **Last resort — exact Hazelight fork** (Epic-org-gated): request access/checkout from the
   user only if 1 and 2 cannot be tuned to match the oracle.

The **validation oracle (§7) decides** which source is "good enough" — we do not guess.

## 5. Registration pipeline details

- **Order:** register all object *types* (names, ref/value flag) first, then methods /
  properties / behaviours / global functions, then compile. Order within each phase is free;
  forward references resolve at `Build()`.
- **Ref vs value:** `F*` → value type (`asOBJ_VALUE | asOBJ_POD`, dummy size); `U*`/`A*` →
  reference type (`asOBJ_REF | asOBJ_NOCOUNT`) so they are used as handles.
- **Dummy offsets / IDs:** `RegisterObjectProperty(obj, "int A", 0)` — the checker consults only
  the decl for name+type; offset/type-ID never compared to the shipped cache. Safe.
- **No execution:** register methods/globals with `asCALL_GENERIC` and a single shared no-op
  generic stub (or null where the fork permits declaration-only). Avoids the MASM thunks.
- **Namespaces:** `SetDefaultNamespace(ns)` around global-function / mixin registration; `ns`
  comes from the Binds.Cache function-field slot (the extended parser exposes it).
- **Templates:** pre-register `TArray<T>`, `TSubclassOf<T>`, `TMap<K,V>`, `TPair<…>` as
  `asOBJ_TEMPLATE` in the prelude; subtype resolved from the decl string.
- **Operators:** a small hand-written UE-AS prelude registers the standard native value-type
  operators (`FVector` arithmetic, `==`, indexers) the dump doesn't carry.
- **Supplements:** enums via `RegisterEnum`+`RegisterEnumValue`; native base edges so inherited
  members resolve; the `GameplayTag` namespace populated with the project tag list.

## 6. Diagnostics output

Default `--format capture`: reproduce the game-loop capture exactly —
```
=== <ScriptRelativeFilename>.as ===
<message line>
<message line>
```
grouped by section (the `.as` relative path used as the AddScriptSection name). This is a
drop-in for `work/reversing/gore-as/ashook/as_errors_capture.txt`, so the existing
`gen_stublist.py` consumes it unchanged. (If `gen_stublist.py` needs the `Compiling <ret>
Name(...)` context lines, the checker synthesizes them per function, or we adapt the script —
decided during implementation.) `--format native` emits raw `section (row, col) : Error : text`.

## 7. Validation strategy — the oracle

We already have **ground truth**: the real game-compile diagnostics for the full 7264-module
emitted tree (the captured `as_errors_capture.txt` from the game loop, and the converged
stublist). The checker is correct when, run on the **same emitted tree**, it reports the **same
set of failing functions/files** the game does (~630 failures in the current decompiler output).

Procedure:
1. Emit the full tree (`gore as emit-all`, no stublist).
2. Run `gore as check` on it; diff its diagnostics against the game capture.
3. Triage divergences: **false errors** (checker rejects what the game accepts) = a registration
   gap (missing enum/base-edge/operator/namespace) → fix the surface. **Missing errors** (checker
   accepts what the game rejects) = the checker is too lax (e.g. wrong engine property) → fix.
4. Iterate until the diff is empty (or within a documented, understood residual).

This makes correctness **measurable**, not a matter of judgement, and it is how we decide whether
the mirror source (§4.1) is faithful or we must fall back (§4.2).

## 8. Risks & mitigations

| Risk | Mitigation |
|------|------------|
| Mirror source diverges from shipped fork → mis-reports | The oracle (§7) catches it; fall back to vanilla + patches (§4.2). |
| Registration surface incomplete → false "unknown symbol" errors | Oracle-driven gap-fixing; supplements from UE4SS dump; operator/template prelude. |
| `GameplayTag::Tag` / `Type::StaticClass()` resolution (needs the reflected DB) | StaticClass() auto-resolves once the type is registered; GameplayTag namespace populated from the project tag list (UE4SS / `DefaultGameplayTags.ini`). |
| Fork requires non-null fn pointer at registration even if never called | Register with `asCALL_GENERIC` + one shared no-op generic function. Confirm against the fork early. |
| C++ build under cargo/MSVC | Proven by gore-oodle (same toolchain, `cc`, C++17, `/wd…`). |
| Engine-property mismatch (e.g. `floatIsFloat64`) changes diagnostics | Replicate the game's `asCreateScriptEngine` init flags exactly (known: floatIsFloat64=true, automaticImports=1). |

## 9. Milestones

1. **Vendor + build** the fork AS core standalone (cc + minimal shim that just creates an engine
   and compiles a trivial `.as`) — proves the toolchain.
2. **binds.rs extension** — expose full decls + namespaces; unit-tested against real Binds.Cache.
3. **Registration surface** from Binds.Cache + UE-AS prelude; register, compile one small module.
4. **Supplements** — UE4SS enum/base-edge/gameplay-tag loader.
5. **`gore as check`** command + capture-format output.
6. **Oracle tuning** — run on the full tree, drive the diff to empty.

## 10. Future (out of scope here)

A cache GENERATOR reuses this phase wholesale: same engine + registration, then
`asIScriptModule::SaveByteCode` produces a real mini-cache module, handed to the **already-working**
`crates/gore-as/src/cache/splice.rs` (`replace_module`/`splice_case_a`) to write into the 122 MB
container. The hard remaining piece there is exact type-ID matching, deferred.

## 11. Testing

- Unit: binds.rs decl/namespace extraction (against real Binds.Cache, gated on file presence).
- Unit: supplement parsing (enum values, base edges) against the UE4SS dump.
- Integration: `gore as check` on a tiny hand-written `.as` set (known good + known bad) →
  expected diagnostics.
- System: the §7 oracle diff on the full emitted tree (the acceptance gate).
