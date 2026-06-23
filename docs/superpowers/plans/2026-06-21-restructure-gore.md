# Restructure to flat `gore` layout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restructure the monorepo from the nested `projects/<x>/crates/<y>` layout into a flat `crates/* + apps/*` layout under a single repo/brand `gore`, split the `gore_core` grab-bag into focused library crates, and fold the standalone `gore-as` binary into one user-facing `gore` CLI.

**Architecture:** Monolith at the UX layer (one `gore` binary over many focused libs), GUIs stay as separate Flutter apps that reuse the same Rust crates via `dart:ffi`. The restructure is mechanical and behavior-preserving: every step keeps the workspace green (`cargo build`, `cargo test`, `flutter analyze`). This is a refactor, so the "test" at each step is the existing suite staying green, not new test code.

**Tech Stack:** Rust workspace (cargo), Flutter (Windows desktop), Inno Setup installers, WinSparkle auto-update, Python `build.py` orchestrator, GitHub Actions CI.

**Base branch:** Fork the restructure branch from `origin/chore/monorepo-restructure` (the canonical integrated tip, 282 commits ahead of `main` as of 2026-06-22; it is a strict superset of all in-flight gore-as/gore-mod/loc work). Do NOT base it on `main` (= goresave v0.3.0 only) or on any single feature branch. Quiesce other open feature branches first — this restructure renames/moves every path, so any branch not merged beforehand becomes a rename-vs-edit conflict.

---

## Backward-compatibility invariants (DO NOT BREAK)

The user's instruction is: **no unnecessary backward compatibility — only what is required.** The shipped product is `goresave` v0.3.0 (save editor) and nothing else. The *only* compat invariants that must survive the restructure:

1. **Inno AppId `{{C7A35D8E-4B61-4E0D-9C0A-2F8B5D1E6A43}`** in the goresave installer. Changing it makes shipped users install side-by-side instead of upgrading. KEEP verbatim.
2. **WinSparkle DSA keypair** + embedded `dsa_pub.pem`. KEEP — new releases must verify against the key shipped in v0.3.0.
3. **Appcast feed `https://github.com/dh0er/goresave/releases/latest/download/appcast-windows.xml`.** Shipped apps poll this. After the GitHub repo rename `goresave → gore`, GitHub auto-redirects the old URL, so the feed keeps resolving. The next goresave release must still publish with `make_latest=true` and attach `appcast-windows.xml`.
4. **C FFI symbol names** `goresave_execute` / `goresave_free` (save editor) and `gore_core_execute` / `gore_core_free` (mod app). These are private in-process ABIs; renaming them is pure churn with zero benefit, so we keep the symbol names even though the crates around them are renamed. Only the *DLL filenames* change (they follow the crate name), and the Dart loaders are updated to match.

Everything else — crate names, directory layout, the unreleased `gore-cli`/`gore-mod`/`gore-as`, internal paths — is renamed cleanly with **no shim crates and no alias re-exports**.

---

## Versioning model (decided)

**Per-product independent semver — NOT a single unified version.** The monorepo ships three distinct user-facing deliverables on independent cadences, so each carries its own version and its own release tag:

| Product | Version lives in | Tag prefix | Notes |
|---------|------------------|------------|-------|
| save-editor (Flutter app) | `apps/save-editor/pubspec.yaml` | `gore-save-v*` | shipped at 0.3.0; auto-update users — must stay monotonic, no fake bumps |
| mod-studio (Flutter app) | `apps/mod-studio/pubspec.yaml` | `gore-mod-v*` | not yet released |
| gore CLI (`gore` crate) | `crates/gore/Cargo.toml` | `gore-cli-v*` | the only Rust crate with a meaningful published version |

Rationale: save-editor has live auto-update users at 0.3.0 — a unified version would force version jumps and spurious update prompts whenever the CLI or mod app changes, and would force lockstep releases against three genuinely different cadences (CLI churns with RE work; mod-studio is unreleased; save-editor is stable). "One tool = one version" applies to the `gore` binary *internally*, not across three separate products. The tag stays `gore-cli-v*` (not `gore-v*`) because `gore-v1.0` reads like a whole-repo version — clarity over cosmetics.

**Internal library crates carry no independent version.** `gore-reflect`, `gore-catalog`, `gore-loc`, `gore-modgen`, `gore-ffi`, `gore-oodle`, `gore-as`, and `gore-save` (the Rust cdylib crates) are never published to crates.io — their version is cosmetic. They inherit a single `[workspace.package]` version so we never hand-bump them. Note: the **app** versions for save-editor/mod-studio live in their pubspec, so the `gore-save`/`gore-ffi` *crate* versions are purely internal. Only the `gore` binary crate keeps its own explicit version.

---

## Target layout

```
gore/                          (repo, was dh0er/goresave)
├─ Cargo.toml                  flat workspace: members = ["crates/*"]
├─ build.py                    orchestrator (PROJECTS map rewritten)
├─ crates/
│  ├─ gore/                    THE binary — package `gore`, bin `gore` (was gore_cli + folds gore-as cmds)
│  ├─ gore-reflect/            UE reflection model + dump parse   (was gore_core: model, parser)
│  ├─ gore-catalog/            item/npc/knowledge catalogs        (was gore_core: catalog, catalog/pipeline)
│  ├─ gore-loc/                localization + game-dir discovery  (was gore_core: loc, loc_store, discover, paths)
│  ├─ gore-modgen/             overrides.toml → Lua + validation  (was gore_core: gen, validate)
│  ├─ gore-ffi/                cdylib dart:ffi bridge for mod-studio (was gore_core: ffi)
│  ├─ gore-save/               GSAV parse/edit + its own cdylib   (was goresave_core)
│  ├─ gore-oodle/              oodle codec                        (was goresave_oodle)
│  └─ gore-as/                 AngelScript cache lib (no bin)
├─ apps/
│  ├─ save-editor/             Flutter (was projects/gore-save/app)
│  └─ mod-studio/              Flutter (was projects/gore-mod/app)
├─ lua/                        shared UE4SS Lua SDK (was projects/gore-lua)
├─ dump-mod/                   generated UE4SS dump mod (was projects/gore-dump)
├─ scripts/                    appcast.py etc. (unchanged location)
├─ docs/
└─ dist/                       build output: dist/<project>/...
```

### Crate dependency DAG after the split

```
gore-reflect   (no internal deps)
gore-catalog   (no internal deps)
gore-loc       (no internal deps)
gore-oodle     (no internal deps)
gore-modgen    → gore-reflect
gore-ffi       → gore-modgen, gore-reflect, gore-loc       [cdylib]
gore-save      → gore-loc, gore-oodle                      [cdylib + rlib]
gore-as        (no internal deps)
gore (bin)     → gore-reflect, gore-catalog, gore-loc, gore-modgen, gore-as
```

### `gore_core` module → new crate mapping (for rewriting `use gore_core::…`)

| Old path | New path |
|----------|----------|
| `gore_core::model::*` | `gore_reflect::model::*` |
| `gore_core::parser::*` | `gore_reflect::parser::*` |
| `gore_core::catalog::*` (incl. `pipeline`, `parse_catalog`) | `gore_catalog::*` |
| `gore_core::discover::*` | `gore_loc::discover::*` |
| `gore_core::paths::*` | `gore_loc::paths::*` |
| `gore_core::loc::*` | `gore_loc::loc::*` |
| `gore_core::loc_store::*` | `gore_loc::loc_store::*` |
| `gore_core::gen::*` (incl. `lua_escape`) | `gore_modgen::gen::*` |
| `gore_core::validate::*` | `gore_modgen::validate::*` |

---

## Phase 0 — Baseline & safety net

### Task 0: Record a green baseline

**Files:** none (verification only)

- [ ] **Step 1: Confirm the whole workspace builds**

Run: `cargo build --workspace`
Expected: finishes with `Finished` and no errors.

- [ ] **Step 2: Confirm the whole test suite passes**

Run: `cargo test --workspace`
Expected: all tests pass. Record the count (e.g. "N passed") — every later phase must end at the same green state.

- [ ] **Step 3: Confirm both Flutter apps analyze clean**

Run:
```bash
cd projects/gore-save/app && flutter pub get && flutter analyze
cd projects/gore-mod/app && flutter pub get && flutter analyze
```
Expected: `No issues found!` for both.

- [ ] **Step 4: Snapshot the FFI symbols (so renames can be verified later)**

Run: `grep -rn "no_mangle" projects/gore-save/crates/goresave_core/src projects/gore-core/crates/gore_core/src/ffi.rs`
Expected: shows `goresave_execute`, `goresave_free`, `gore_core_execute`, `gore_core_free`. These symbol names must remain unchanged through the whole plan.

No commit (baseline only).

---

## Phase 1 — Flatten directories (no crate renames yet)

Goal: move every crate/app/asset to its final directory while keeping crate **names** unchanged, so only path references change. Workspace stays green.

### Task 1: Flatten the Rust crates into `crates/`

**Files:**
- Move: `projects/gore-core/crates/gore_core` → `crates/gore_core`
- Move: `projects/gore-cli/crates/gore_cli` → `crates/gore_cli`
- Move: `projects/gore-as/crates/gore_as` → `crates/gore_as`
- Move: `projects/gore-save/crates/goresave_core` → `crates/goresave_core`
- Move: `projects/gore-save/crates/goresave_oodle` → `crates/goresave_oodle`
- Modify: `Cargo.toml` (workspace members), and every crate `Cargo.toml` `path = "../../../…"` dependency

- [ ] **Step 1: Move the crate directories**

```bash
git mv projects/gore-core/crates/gore_core crates/gore_core
git mv projects/gore-cli/crates/gore_cli crates/gore_cli
git mv projects/gore-as/crates/gore_as crates/gore_as
git mv projects/gore-save/crates/goresave_core crates/goresave_core
git mv projects/gore-save/crates/goresave_oodle crates/goresave_oodle
```

- [ ] **Step 2: Rewrite the workspace members in `Cargo.toml`**

Replace the `members` array with:
```toml
[workspace]
members = ["crates/*"]
resolver = "2"
```

- [ ] **Step 3: Fix path dependencies inside each crate's `Cargo.toml`**

The path deps now resolve as siblings under `crates/`. Apply these exact replacements:

- `crates/gore_cli/Cargo.toml`: `gore_core = { path = "../../../gore-core/crates/gore_core" }` → `gore_core = { path = "../gore_core" }`
- `crates/goresave_core/Cargo.toml`:
  - `gore_core = { path = "../../../gore-core/crates/gore_core" }` → `gore_core = { path = "../gore_core" }`
  - `goresave_oodle = { path = "../goresave_oodle" }` → unchanged (already a sibling)

- [ ] **Step 4: Build to verify the move**

Run: `cargo build --workspace`
Expected: `Finished` with no errors.

- [ ] **Step 5: Test to verify the move**

Run: `cargo test --workspace`
Expected: same pass count as the Task 0 baseline.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor: flatten rust crates into top-level crates/"
```

### Task 2: Flatten the Flutter apps into `apps/`

**Files:**
- Move: `projects/gore-save/app` → `apps/save-editor`
- Move: `projects/gore-mod/app` → `apps/mod-studio`
- Move: `projects/gore-save/CHANGELOG.md` → `apps/save-editor/CHANGELOG.md`
- Move: `projects/gore-mod/CHANGELOG.md` → `apps/mod-studio/CHANGELOG.md`
- Move: `projects/gore-save/installer` → `apps/save-editor/installer`
- Move: `projects/gore-mod/installer` → `apps/mod-studio/installer`
- Move: `projects/gore-save/fixtures`, `projects/gore-save/integration_test`, `projects/gore-save/tools` → under `apps/save-editor/`

- [ ] **Step 1: Move the app trees and their siblings**

```bash
git mv projects/gore-save/app apps/save-editor
git mv projects/gore-mod/app apps/mod-studio
git mv projects/gore-save/CHANGELOG.md apps/save-editor/CHANGELOG.md
git mv projects/gore-mod/CHANGELOG.md apps/mod-studio/CHANGELOG.md
git mv projects/gore-save/installer apps/save-editor/installer
git mv projects/gore-mod/installer apps/mod-studio/installer
git mv projects/gore-save/fixtures apps/save-editor/fixtures
git mv projects/gore-save/integration_test apps/save-editor/integration_test
git mv projects/gore-save/tools apps/save-editor/tools
```

- [ ] **Step 2: Fix the FFI DLL search paths in the Dart loaders**

The Dart loaders walk `../../target/...` relative to the app dir. The app moved from `projects/gore-save/app` (depth 3 under repo) to `apps/save-editor` (depth 2), so one `..` level drops. Edit `apps/save-editor/lib/features/editor/domain/core_service.dart`: in the candidate list, change every `p.join(cwd, '..', '..', '..', 'target', ...)` to `p.join(cwd, '..', '..', 'target', ...)` and every `p.join(cwd, '..', '..', 'target', ...)` to `p.join(cwd, '..', 'target', ...)`. Keep the bare `goresave_core.dll` (executable-dir) candidate unchanged.

Apply the equivalent depth fix in `apps/mod-studio/lib/core/core_service.dart`.

- [ ] **Step 3: Clean stale CMake caches (known restructure gotcha)**

Run:
```bash
cd apps/save-editor && flutter clean && flutter pub get
cd apps/mod-studio && flutter clean && flutter pub get
```
Expected: clean re-fetch, no errors. (Stale `build/windows` CMakeCache from a path change otherwise breaks the native build.)

- [ ] **Step 4: Analyze both apps**

Run: `cd apps/save-editor && flutter analyze` then `cd apps/mod-studio && flutter analyze`
Expected: `No issues found!` for both.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor: flatten flutter apps into apps/ (save-editor, mod-studio)"
```

### Task 3: Flatten the remaining project assets

**Files:**
- Move: `projects/gore-lua` → `lua`
- Move: `projects/gore-dump` → `dump-mod`
- Move: `projects/gore-core/pipeline` → `crates/gore_core/pipeline` (or `docs/pipelines` if it is docs — inspect first)
- Move (straggler docs/changelogs into their crate dirs so later renames carry them):
  - `projects/gore-cli/CHANGELOG.md` → `crates/gore_cli/CHANGELOG.md`
  - `projects/gore-as/FORMAT.md`, `projects/gore-as/DECOMPILER_STATUS.md` → `crates/gore_as/`
  - `projects/gore-save/README.md`, `projects/gore-save/test.py` → `apps/save-editor/`
- Delete (superseded by the root `build.py`): `projects/gore-save/build.py`, `projects/gore-mod/build.py`
- Modify: `crates/gore_cli/src/cmd/deploy_shared.rs`, `crates/gore_cli/src/cmd/scaffold.rs` (repo-relative `projects/gore-lua` path literals)
- Delete: now-empty `projects/` subtrees

- [ ] **Step 1: Inspect `projects/gore-core/pipeline` to decide its home**

Run: `ls projects/gore-core/pipeline`
If it contains scripts that belong with the catalog code, it will move with the catalog crate later; for now move it to `crates/gore_core/pipeline`. If it is docs/spec, move to `docs/pipelines`.

- [ ] **Step 2: Move lua + dump-mod + pipeline + straggler files**

```bash
git mv projects/gore-lua lua
git mv projects/gore-dump dump-mod
git mv projects/gore-core/pipeline crates/gore_core/pipeline   # or docs/pipelines per Step 1
git mv projects/gore-cli/CHANGELOG.md crates/gore_cli/CHANGELOG.md
git mv projects/gore-as/FORMAT.md crates/gore_as/FORMAT.md
git mv projects/gore-as/DECOMPILER_STATUS.md crates/gore_as/DECOMPILER_STATUS.md
git mv projects/gore-save/README.md apps/save-editor/README.md
git mv projects/gore-save/test.py apps/save-editor/test.py
git rm projects/gore-save/build.py projects/gore-mod/build.py   # superseded by root build.py
```

- [ ] **Step 3: Fix the `deploy-shared` / `scaffold` repo-relative path literals**

`DeployShared` no longer has a `default_value`; the default `src` is resolved at runtime by `resolve_default_src()` which searches paths relative to the executable, including a repo-dev fallback that hardcodes `projects/gore-lua/shared`. Edit `crates/gore_cli/src/cmd/deploy_shared.rs`: change the ancestor-walk candidate `anc.join("projects").join("gore-lua").join("shared")` to `anc.join("lua").join("shared")`, and update the error-message hint `pass --src <path-to-gore-lua/shared>` text to match `lua/shared`. Then in `crates/gore_cli/src/cmd/scaffold.rs`, update the generated-comment reference `see projects/gore-lua/README.md` to `see lua/README.md`.
(The `exe_dir.join("gore-lua").join("shared")` packaged-layout candidate is about the shipped layout next to the exe, set by `build.py` — leave it; revisit in Task 10 if the packaged folder name changes.)

- [ ] **Step 4: Remove the now-empty `projects/` tree**

Run: `find projects -type f` — confirm nothing remains, then remove the empty dirs (`git status` should show no tracked files left under `projects/`). Expect `projects/` to disappear entirely.

- [ ] **Step 5: Build + test + analyze**

Run: `cargo build --workspace && cargo test --workspace`
Expected: green, same count.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor: move gore-lua->lua, gore-dump->dump-mod, drop projects/ tree"
```

### Task 3b: Add `[workspace.package]` so internal libs share one version

**Files:**
- Modify: `Cargo.toml` (add `[workspace.package]`)
- Modify: every internal lib crate's `Cargo.toml` to inherit (deferred for crates created later in Task 8 — their snippets already use the inherited form)

This implements the versioning decision: internal libs never carry an independent version; only the `gore` binary crate keeps an explicit one.

- [ ] **Step 1: Add the shared package metadata to the root `Cargo.toml`**

Append to `Cargo.toml`:
```toml
[workspace.package]
version = "0.0.0"        # cosmetic: internal libs are never published
edition = "2021"
license = "MIT"
```

- [ ] **Step 2: Make the existing internal lib crates inherit**

For each of `crates/gore_core`, `crates/goresave_core`, `crates/goresave_oodle`, `crates/gore_as` (current names — they are renamed/split later, but inherit now so the form is consistent), replace the literal `version = "…"`, `edition = "2021"`, and `license = "MIT"` lines in `[package]` with:
```toml
version.workspace = true
edition.workspace = true
license.workspace = true
```
Do **NOT** change `crates/gore_cli/Cargo.toml` — the `gore` binary keeps its own explicit `version` (the CLI product version, tag `gore-cli-v*`).

- [ ] **Step 3: Build + test**

Run: `cargo build --workspace && cargo test --workspace`
Expected: green. (`cargo` resolves `version.workspace = true` against `[workspace.package]`.)

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "build: share one workspace version across internal libs; gore keeps its own"
```

---

## Phase 2 — Rename crates (clean break, no aliases)

Each rename is: edit `Cargo.toml` `[package] name`, rename the dir, repoint dependents' `path` + the `use` prefix, verify green, commit. One crate per task.

### Task 4: Rename `goresave_oodle` → `gore-oodle`

**Files:**
- Modify: `crates/goresave_oodle/Cargo.toml`
- Move: `crates/goresave_oodle` → `crates/gore-oodle`
- Modify: `crates/goresave_core/Cargo.toml`, and any `use goresave_oodle` in `crates/goresave_core/src`

- [ ] **Step 1: Rename the package and dir**

In `crates/goresave_oodle/Cargo.toml` set `name = "gore-oodle"`. Then:
```bash
git mv crates/goresave_oodle crates/gore-oodle
```

- [ ] **Step 2: Repoint the dependent**

In `crates/goresave_core/Cargo.toml`: `goresave_oodle = { path = "../goresave_oodle" }` → `gore-oodle = { path = "../gore-oodle" }`.

Rewrite imports: replace every `goresave_oodle` with `gore_oodle` in `crates/goresave_core/src/`.
Run: `grep -rln "goresave_oodle" crates/goresave_core/src` then apply the replacement in each hit.

- [ ] **Step 3: Build + test**

Run: `cargo build --workspace && cargo test --workspace`
Expected: green.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "refactor: rename crate goresave_oodle -> gore-oodle"
```

### Task 5: Rename `goresave_core` → `gore-save`

**Files:**
- Modify: `crates/goresave_core/Cargo.toml` (`name = "gore-save"`; keep `crate-type` and the `#[no_mangle]` symbols untouched)
- Move: `crates/goresave_core` → `crates/gore-save`
- Modify: `apps/save-editor/lib/features/editor/domain/core_service.dart` (DLL filename)
- Modify: `build.py` (`core_dll` key — handled in Phase 4, note here)

- [ ] **Step 1: Rename package + dir**

In `crates/goresave_core/Cargo.toml` set `name = "gore-save"`. Leave `[lib] crate-type` as-is. Then:
```bash
git mv crates/goresave_core crates/gore-save
```

- [ ] **Step 2: Update the DLL filename in the save-editor Dart loader**

The cdylib output name follows the crate: `goresave_core.dll` → `gore_save.dll`. In `apps/save-editor/lib/features/editor/domain/core_service.dart`, replace every literal `goresave_core.dll` with `gore_save.dll`. Do NOT change the looked-up symbol names `goresave_execute`/`goresave_free` (invariant #4).

- [ ] **Step 3: Build the crate + test**

Run: `cargo build -p gore-save && cargo test --workspace`
Expected: green; confirm `target/debug/gore_save.dll` exists (`ls target/debug/gore_save.dll`).

- [ ] **Step 4: Verify the FFI symbol survived the rename**

Run: `grep -rn "no_mangle" crates/gore-save/src` (and the symbol names below it).
Expected: still `goresave_execute` / `goresave_free` (unchanged).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor: rename crate goresave_core -> gore-save (dll gore_save.dll; C symbols unchanged)"
```

### Task 6: Rename `gore_cli` → `gore` (the binary)

**Files:**
- Modify: `crates/gore_cli/Cargo.toml` (`name = "gore"`, set `[[bin]] name = "gore"`)
- Move: `crates/gore_cli` → `crates/gore`
- Modify: `crates/gore/src/main.rs` (`#[command(name = "gore-cli"…)]` → `name = "gore"`)

- [ ] **Step 1: Rename package, declare the bin, rename dir**

In `crates/gore_cli/Cargo.toml`: set `name = "gore"` and add (or adjust) the bin target:
```toml
[[bin]]
name = "gore"
path = "src/main.rs"
```
Then:
```bash
git mv crates/gore_cli crates/gore
```

- [ ] **Step 2: Update the clap command name**

In `crates/gore/src/main.rs`, change `#[command(name = "gore-cli", about = "Gothic 1 Remake mod tooling CLI", version)]` to `#[command(name = "gore", about = "Gothic Remake mod tooling CLI", version)]`.

- [ ] **Step 3: Build + test + run**

Run: `cargo build -p gore && cargo test -p gore && ./target/debug/gore --help`
Expected: `--help` prints `gore` as the program name with all existing subcommands.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "refactor: rename crate gore_cli -> gore (bin: gore.exe)"
```

### Task 7: Rename `gore_as` dir → `crates/gore-as`, drop its standalone bin

The `gore_as` package name already fits. Remove its `[[bin]]` so its commands can be folded into `gore` in Phase 3 (lib-only crate).

**Files:**
- Move: `crates/gore_as` → `crates/gore-as`
- Modify: `crates/gore-as/Cargo.toml` (rename package to `gore-as`, remove `[[bin]]`, drop now-unused bin-only deps if any)
- Delete: `crates/gore-as/src/bin/gore-as.rs` (logic re-homed into `gore` in Phase 3)

- [ ] **Step 1: Rename dir + package**

```bash
git mv crates/gore_as crates/gore-as
```
In `crates/gore-as/Cargo.toml`: set `name = "gore-as"`. Remove the `[[bin]] name = "gore-as"` block. Keep `[lib] crate-type = ["rlib"]`.

- [ ] **Step 2: Preserve the bin logic for Phase 3, then delete the bin file**

Copy `crates/gore-as/src/bin/gore-as.rs` to a scratch reference (`work/gore-as-bin.rs.bak`) — its `Cmd` enum + match arms become the `gore as …` subcommand in Phase 3. Then:
```bash
git rm crates/gore-as/src/bin/gore-as.rs
```

- [ ] **Step 3: Build (lib only) + test**

Run: `cargo build -p gore-as && cargo test -p gore-as`
Expected: green; no binary produced for gore-as.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "refactor: gore_as -> gore-as, lib-only (bin folded into gore in next phase)"
```

---

## Phase 3 — Split `gore_core` and unify the binary

### Task 8: Split `gore_core` into `gore-reflect`, `gore-catalog`, `gore-loc`, `gore-modgen`, `gore-ffi`

This is the highest-risk task. Work crate-by-crate, building after each new crate compiles, then repoint consumers last. Use the module→crate table from the header.

**Files:**
- Create: `crates/gore-reflect/{Cargo.toml,src/lib.rs}` + moved `model.rs`, `parser.rs`
- Create: `crates/gore-catalog/{Cargo.toml,src/lib.rs}` + moved `catalog.rs`, `catalog/pipeline.rs`
- Create: `crates/gore-loc/{Cargo.toml,src/lib.rs}` + moved `loc.rs`, `loc_store.rs`, `discover.rs`, `paths.rs`
- Create: `crates/gore-modgen/{Cargo.toml,src/lib.rs}` + moved `gen.rs`, `validate.rs`
- Create: `crates/gore-ffi/{Cargo.toml,src/lib.rs}` + moved `ffi.rs`
- Delete: `crates/gore_core` (whole)
- Modify: `crates/gore/src/cmd/*.rs`, `crates/gore-save/src/lib.rs` (repoint `use gore_core::…`)

- [ ] **Step 1: Create `gore-reflect` (model + parser)**

```bash
mkdir -p crates/gore-reflect/src
git mv crates/gore_core/src/model.rs crates/gore-reflect/src/model.rs
git mv crates/gore_core/src/parser.rs crates/gore-reflect/src/parser.rs
```
Write `crates/gore-reflect/Cargo.toml`:
```toml
[package]
name = "gore-reflect"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "UE reflection model + UE4SS SDK dump parser for gore-tools."

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"

[dev-dependencies]
pretty_assertions = "1"
```
Write `crates/gore-reflect/src/lib.rs`:
```rust
pub mod model;
pub mod parser;

pub use model::ReflectionModel;
```
`parser.rs` already uses `crate::model::…` — same crate, no change needed.

- [ ] **Step 2: Build the new crate in isolation**

Run: `cargo build -p gore-reflect`
Expected: compiles. (It is not yet a workspace member if `members = ["crates/*"]` already globs it — confirm with `cargo metadata --no-deps --format-version 1 | grep gore-reflect`.)

- [ ] **Step 3: Create `gore-catalog` (catalog + pipeline)**

```bash
mkdir -p crates/gore-catalog/src/catalog
git mv crates/gore_core/src/catalog.rs crates/gore-catalog/src/catalog.rs
git mv crates/gore_core/src/catalog/pipeline.rs crates/gore-catalog/src/catalog/pipeline.rs
```
Note: `catalog.rs` declares `mod pipeline;` referring to `catalog/pipeline.rs`. To keep that working, make `catalog` the crate root content. Write `crates/gore-catalog/src/lib.rs`:
```rust
#[path = "catalog.rs"]
mod catalog_impl;
pub use catalog_impl::*;
pub mod pipeline {
    pub use crate::catalog_impl::pipeline::*;
}
```
Simpler alternative (preferred): rename `catalog.rs` to `lib.rs` and keep the `pipeline` submodule:
```bash
git mv crates/gore-catalog/src/catalog.rs crates/gore-catalog/src/lib.rs
mkdir -p crates/gore-catalog/src/pipeline
# move pipeline.rs next to lib.rs as a child module dir
git mv crates/gore-catalog/src/catalog/pipeline.rs crates/gore-catalog/src/pipeline.rs
rmdir crates/gore-catalog/src/catalog
```
Ensure `lib.rs` keeps `pub mod pipeline;` and any `pub use` that consumers rely on (`parse_catalog`, `category_for_id`, `item_category_from_id`, `CatalogJsonEntry`, `ItemCategory`).
Write `crates/gore-catalog/Cargo.toml`:
```toml
[package]
name = "gore-catalog"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Item/NPC/knowledge catalog model + generation pipelines for gore-tools."

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
pretty_assertions = "1"
```

- [ ] **Step 4: Build `gore-catalog`**

Run: `cargo build -p gore-catalog`
Expected: compiles. If `pipeline.rs` referenced `super::` or `crate::catalog::…`, fix to `crate::…`.

- [ ] **Step 5: Create `gore-loc` (loc + loc_store + discover + paths)**

```bash
mkdir -p crates/gore-loc/src
git mv crates/gore_core/src/loc.rs crates/gore-loc/src/loc.rs
git mv crates/gore_core/src/loc_store.rs crates/gore-loc/src/loc_store.rs
git mv crates/gore_core/src/discover.rs crates/gore-loc/src/discover.rs
git mv crates/gore_core/src/paths.rs crates/gore-loc/src/paths.rs
```
Write `crates/gore-loc/src/lib.rs`:
```rust
pub mod discover;
pub mod loc;
pub mod loc_store;
pub mod paths;
```
Write `crates/gore-loc/Cargo.toml`:
```toml
[package]
name = "gore-loc"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "AlkimiaLocalization .lcache crypto + game-dir discovery + shared paths for gore-tools."

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
aes = "0.8"

[target.'cfg(windows)'.dependencies]
winreg = "0.52"

[dev-dependencies]
pretty_assertions = "1"
```
`loc_store.rs` uses `crate::{discover, loc::Lcache, paths}` — same crate, no change.

- [ ] **Step 6: Build `gore-loc`**

Run: `cargo build -p gore-loc`
Expected: compiles.

- [ ] **Step 7: Create `gore-modgen` (gen + validate, depends on gore-reflect)**

```bash
mkdir -p crates/gore-modgen/src
git mv crates/gore_core/src/gen.rs crates/gore-modgen/src/gen.rs
git mv crates/gore_core/src/validate.rs crates/gore-modgen/src/validate.rs
```
Write `crates/gore-modgen/src/lib.rs`:
```rust
pub mod gen;
pub mod validate;

pub use gen::{gen_lua, OverridesConfig};
pub use validate::{validate_config, ValidationError};
```
Write `crates/gore-modgen/Cargo.toml`:
```toml
[package]
name = "gore-modgen"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Declarative overrides.toml -> UE4SS Lua mod generation + field validation."

[dependencies]
gore-reflect = { path = "../gore-reflect" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
thiserror = "2"

[dev-dependencies]
pretty_assertions = "1"
```
In `validate.rs`, change `use crate::{gen::{…}, model::{PropType, ReflectionModel}}` to `use crate::gen::{OverrideValue, OverridesConfig};` plus `use gore_reflect::model::{PropType, ReflectionModel};`.

- [ ] **Step 8: Build `gore-modgen`**

Run: `cargo build -p gore-modgen`
Expected: compiles.

- [ ] **Step 9: Create `gore-ffi` (the cdylib, depends on modgen + reflect + loc)**

```bash
mkdir -p crates/gore-ffi/src
git mv crates/gore_core/src/ffi.rs crates/gore-ffi/src/lib.rs
```
In `crates/gore-ffi/src/lib.rs`, rewrite the imports at the top:
- `use crate::gen::{gen_lua, OverridesConfig};` → `use gore_modgen::gen::{gen_lua, OverridesConfig};`
- `use crate::model::ReflectionModel;` → `use gore_reflect::model::ReflectionModel;`
- `use crate::validate::validate_config;` → `use gore_modgen::validate::validate_config;`
- `use crate::{loc_store, paths};` → `use gore_loc::{loc_store, paths};`
- the inline `crate::loc_store::LocStoreError::NotFound` → `gore_loc::loc_store::LocStoreError::NotFound`
Keep the `#[no_mangle] gore_core_execute` / `gore_core_free` symbol names (invariant #4).
Write `crates/gore-ffi/Cargo.toml`:
```toml
[package]
name = "gore-ffi"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "C ABI / dart:ffi bridge for the gore mod-studio app."

[lib]
crate-type = ["cdylib"]

[dependencies]
gore-reflect = { path = "../gore-reflect" }
gore-modgen = { path = "../gore-modgen" }
gore-loc = { path = "../gore-loc" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 10: Build `gore-ffi`**

Run: `cargo build -p gore-ffi`
Expected: compiles; confirm `ls target/debug/gore_ffi.dll`.

- [ ] **Step 11: Delete the empty `gore_core` crate**

Confirm `crates/gore_core/src` has only `lib.rs` left, then:
```bash
git rm -r crates/gore_core
```

- [ ] **Step 12: Repoint the `gore` binary's imports**

In `crates/gore/Cargo.toml`, replace `gore_core = { path = "../gore_core" }` with:
```toml
gore-reflect = { path = "../gore-reflect" }
gore-catalog = { path = "../gore-catalog" }
gore-loc = { path = "../gore-loc" }
gore-modgen = { path = "../gore-modgen" }
```
Then rewrite imports across `crates/gore/src/cmd/*.rs` per the module→crate table:
- `catalog.rs`: `use gore_core::catalog::pipeline;` → `use gore_catalog::pipeline;`
- `dump.rs`: `use gore_core::{ model::…, parser::… }` → `use gore_reflect::{ model::…, parser::… }`
- `dump_mod.rs`: `use gore_core::{catalog::parse_catalog, gen::lua_escape};` → `use gore_catalog::parse_catalog;` + `use gore_modgen::gen::lua_escape;`
- `gen.rs`: `use gore_core::{ gen::…, validate::…, model::… }` → `use gore_modgen::{gen::…, validate::…};` + `use gore_reflect::model::…` as needed
- `gui_model.rs`: `use gore_core::{catalog::parse_catalog, model::{PropType, ReflectionModel}};` → `use gore_catalog::parse_catalog;` + `use gore_reflect::model::{PropType, ReflectionModel};`; also `gore_core::model::Property` → `gore_reflect::model::Property` (two more hits at lines ~92, ~208)
- `loc.rs`: `use gore_core::loc::Lcache;` → `use gore_loc::loc::Lcache;`; `use gore_core::{loc_store, paths};` → `use gore_loc::{loc_store, paths};`
- `stubs.rs`: `use gore_core::model::{PropType, ReflectionModel};` → `use gore_reflect::model::{PropType, ReflectionModel};`
- `sync.rs`: `use gore_core::catalog::parse_catalog;` → `use gore_catalog::parse_catalog;`

Run `grep -rn "gore_core" crates/gore/src` afterward — expect zero hits.

- [ ] **Step 13: Repoint `gore-save`'s imports**

In `crates/gore-save/Cargo.toml`, replace `gore_core = { path = "../gore_core" }` with `gore-loc = { path = "../gore-loc" }`.
In `crates/gore-save/src/lib.rs`, replace `gore_core::loc_store` → `gore_loc::loc_store` and `gore_core::paths` → `gore_loc::paths` (all hits around lines 499–530). Run `grep -rn "gore_core" crates/gore-save/src` — expect zero hits.

- [ ] **Step 14: Point the mod-studio app at the new DLL name**

The mod cdylib moved from `gore_core` to `gore-ffi`, so its DLL is `gore_ffi.dll`. In `apps/mod-studio/lib/core/core_service.dart`, replace every literal `gore_core.dll` with `gore_ffi.dll`. Keep the symbol lookups `gore_core_execute`/`gore_core_free` unchanged (invariant #4).

- [ ] **Step 15: Build + test the whole workspace**

Run: `cargo build --workspace && cargo test --workspace`
Expected: green, same pass count as the Task 0 baseline. Confirm `grep -rn "gore_core" crates/` returns nothing.

- [ ] **Step 16: Analyze the mod app**

Run: `cd apps/mod-studio && flutter analyze`
Expected: `No issues found!`

- [ ] **Step 17: Commit**

```bash
git add -A
git commit -m "refactor: split gore_core into gore-reflect/-catalog/-loc/-modgen/-ffi"
```

### Task 9: Fold `gore-as` commands into the `gore` binary as `gore as …`

**Files:**
- Modify: `crates/gore/Cargo.toml` (add `gore-as = { path = "../gore-as" }`, add `anyhow` if not present)
- Create: `crates/gore/src/cmd/as_cache.rs` (the AngelScript subcommand group, from the saved `work/gore-as-bin.rs.bak`)
- Modify: `crates/gore/src/cmd/mod.rs`, `crates/gore/src/main.rs` (register `As` subcommand)

- [ ] **Step 1: Add the dependency**

In `crates/gore/Cargo.toml` `[dependencies]` add:
```toml
gore-as = { path = "../gore-as" }
anyhow = "1"
```
(`anyhow` is already present per the current manifest; confirm.)

- [ ] **Step 2: Create the `as` subcommand module**

Create `crates/gore/src/cmd/as_cache.rs` with an `AsCmd` enum mirroring the old `gore-as` `Cmd` enum (decode-header, walk, info, decompile, emit, emit-all, disasm, replace, splice) and a `run(cmd: AsCmd) -> anyhow::Result<()>` that contains the match arms copied verbatim from `work/gore-as-bin.rs.bak` (they call into `gore_as::cache::…`). Example shape:
```rust
use std::path::PathBuf;
use anyhow::{Context, Result};
use clap::Subcommand;
use gore_as::cache::header::CacheHeader;
// … remaining gore_as::cache::* imports from the old bin …

#[derive(Subcommand)]
pub enum AsCmd {
    /// Parse and print the outer cache header.
    DecodeHeader { file: PathBuf },
    // … the rest, verbatim from the old gore-as bin …
}

pub fn run(cmd: AsCmd) -> Result<()> {
    match cmd {
        AsCmd::DecodeHeader { file } => { /* old match arm body */ }
        // … the rest …
    }
    Ok(())
}
```

- [ ] **Step 3: Register the module + subcommand**

In `crates/gore/src/cmd/mod.rs` add `pub mod as_cache;`.
In `crates/gore/src/main.rs`, add a variant to the `Commands` enum:
```rust
    /// AngelScript precompiled-cache tooling (decode/emit/splice/decompile).
    As {
        #[command(subcommand)]
        cmd: cmd::as_cache::AsCmd,
    },
```
and a match arm in `main()`: `Commands::As { cmd } => cmd::as_cache::run(cmd),`.
Note: `main()` currently returns `()` via per-arm `Result` handling; the `as_cache::run` returns `anyhow::Result<()>` — map it the same way the other arms' errors are printed (the existing `if let Err(e) = result` block already handles a `Result`; ensure the arm's type unifies — wrap with `.map_err(Into::into)` if the existing arms use a different error type).

- [ ] **Step 4: Build + run the folded command**

Run: `cargo build -p gore && ./target/debug/gore as --help`
Expected: lists the AngelScript subcommands. Spot-check one: `./target/debug/gore as decode-header --help`.

- [ ] **Step 5: Test**

Run: `cargo test --workspace`
Expected: green.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(gore): fold gore-as into the gore binary as 'gore as <cmd>'"
```

---

## Phase 4 — build.py, CI, scripts

### Task 10: Rewrite the `build.py` PROJECTS map for the new layout

**Files:**
- Modify: `build.py` (PROJECTS dict, RELEASE_ORDER, any hardcoded `projects/…` paths)

- [ ] **Step 1: Replace the PROJECTS dict**

Rewrite the `PROJECTS` dict (locate it via `grep -n "PROJECTS: dict" build.py`) to the new layout. The releasable products are unchanged (save editor, mod studio, the CLI), only paths/names move. **Drop the `gore-as` and `gore-core` entries entirely**: `gore-core` is split into libs and `gore-as` becomes a lib folded into the `gore` binary — neither is a standalone build/release target anymore. The internal libs need no PROJECTS entry (they build via `cargo build --workspace`). Also update `RELEASE_ORDER` to `["gore", "gore-save", "gore-mod"]` (was `["gore-cli", "gore-save", "gore-mod"]`).
```python
PROJECTS: dict[str, dict] = {
    "gore-save": {                       # save editor (Flutter, WinSparkle)
        "kind": "flutter",
        "dir": "apps/save-editor",
        "pubspec": "pubspec.yaml",
        "tag_prefix": "gore-save",
        "changelog": "CHANGELOG.md",
        "installer": "installer/setup.iss",
        "installer_name": "GoresaveSetup",   # keep filename: shipped feed expects it
        "exe": "goresave.exe",               # keep: CMake BINARY_NAME (AppId-tied upgrade)
        "core_dll": "gore_save",             # was goresave_core; dll now gore_save.dll
        "dist_zip": "goresave-{version}-windows-x64",
        "releasable": True,
    },
    "gore-mod": {                        # mod studio (Flutter, WinSparkle)
        "kind": "flutter",
        "dir": "apps/mod-studio",
        "pubspec": "pubspec.yaml",
        "tag_prefix": "gore-mod",
        "changelog": "CHANGELOG.md",
        "installer": "installer/setup.iss",
        "installer_name": "GoreModSetup",
        "exe": "gore_mod.exe",
        "core_dll": "gore_ffi",              # was gore_core; dll now gore_ffi.dll
        "dist_zip": "gore-mod-{version}-windows-x64",
        "releasable": True,
    },
    "gore": {                            # the unified CLI (was gore-cli)
        "kind": "rust-bin",
        "dir": "crates/gore",
        "manifest": "Cargo.toml",
        "crate": "gore",
        "bin": "gore",                       # produces gore.exe
        "tag_prefix": "gore-cli",            # KEEP tag prefix: release.yml + habit
        "changelog": "CHANGELOG.md",
        "dist_zip": "gore-{version}-windows-x64",
        "releasable": True,
    },
}
```
Notes baked in: the goresave `installer_name`, `exe`, and AppId stay (invariants #1). The CLI keeps `tag_prefix = "gore-cli"` so the existing release.yml trigger and your tagging habit keep working; only the binary/dir renamed. If you prefer the tag `gore-v*`, change it here AND in release.yml together (Task 11).

- [ ] **Step 2: Fix remaining hardcoded paths in build.py**

Run: `grep -n "projects/" build.py`
For each hit, repoint to the new path (e.g. dist dirs, the `gore-save`-specific branch in the flutter recipe). Standardize dist output to `dist/<project>/`.
The CLI manifest is now `crates/gore/Cargo.toml` (manifest key `"Cargo.toml"`, dir `crates/gore`) and its changelog `crates/gore/CHANGELOG.md` — already in place from Task 3 (CHANGELOG → `crates/gore_cli/`) + Task 6 (dir rename `gore_cli → gore` carried it). Verify with `ls crates/gore/CHANGELOG.md`.

- [ ] **Step 3: Smoke-test the orchestrator (no publish)**

Run: `python build.py gore build`
Expected: builds `gore.exe`. Then `python build.py gore-save build` (builds the DLL + Flutter release). Expected: succeeds, produces `gore_save.dll`.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "build: repoint build.py PROJECTS map to flat crates/ + apps/ layout"
```

### Task 11: Update the Release + CI workflows

**Files:**
- Modify: `.github/workflows/release.yml` (all `projects/gore-save/…` and `projects/gore-mod/…` paths)
- Modify: `.github/workflows/ci.yml` (any path filters / build paths)

- [ ] **Step 1: Repoint release.yml paths**

`release.yml` has three jobs (gore-save, gore-mod, gore-cli), each with its own paths. Replace, per job:
- **gore-save job:** `projects/gore-save/app/pubspec.yaml` → `apps/save-editor/pubspec.yaml`; `projects/gore-save/CHANGELOG.md` → `apps/save-editor/CHANGELOG.md`; `projects/gore-save/dist/…` → the dist path build.py now emits (`dist/gore-save/…` if standardized in Task 10; otherwise keep consistent with build.py).
- **gore-mod job:** the analogous `projects/gore-mod/app/pubspec.yaml` → `apps/mod-studio/pubspec.yaml`; `projects/gore-mod/CHANGELOG.md` → `apps/mod-studio/CHANGELOG.md`; `projects/gore-mod/dist/…` → new dist path.
- **gore-cli job:** `projects/gore-cli/crates/gore_cli/Cargo.toml` → `crates/gore/Cargo.toml`; `projects/gore-cli/CHANGELOG.md` → `crates/gore/CHANGELOG.md`; `projects/gore-cli/dist/…` → new dist path. The job's tag trigger stays `gore-cli-v*` and `make_latest=false` (only gore-save publishes `latest`). The produced binary is now `gore.exe` (was `gore-cli.exe`) — update any artifact/exe references in this job.
Keep the gore-save appcast flags (`--title goresave`, `--release-tag gore-save-v…`, `make_latest: "true"`) unchanged — invariants #2/#3.

- [ ] **Step 2: Repoint ci.yml**

Run: `grep -n "projects/" .github/workflows/ci.yml`
Repoint each path. If ci.yml uses path-filter triggers, update the globs to `crates/**`, `apps/**`, `lua/**`.

- [ ] **Step 3: Lint the workflow YAML**

Run: `python -c "import yaml,sys; [yaml.safe_load(open(f)) for f in ['.github/workflows/release.yml','.github/workflows/ci.yml']]; print('yaml ok')"`
Expected: `yaml ok`.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "ci: repoint release + ci workflows to flat layout (feed/AppId/DSA unchanged)"
```

---

## Phase 5 — Repo rename + branding + docs

### Task 12: Rewrite the README and root docs for the new structure

**Files:**
- Modify: `README.md` (it still calls gore-cli/gore-mod "planned" and omits gore-as/gore-lua)

- [ ] **Step 1: Rewrite README.md**

Replace the Projects table and Layout section to reflect: repo `gore`, one `gore` CLI (with `gore as` folded in), the `crates/*` library set, the two `apps/*`, `lua/`, `dump-mod/`. Document the build commands (`cargo build`, `python build.py <project> …`). State the three axes (save-edit / mod-authoring / script-RE) and that the modding front-ends emit the same UE4SS Lua CDO-override artifact.

- [ ] **Step 2: Verify no stale path references remain in docs**

Run: `grep -rn "projects/\|goresave_core\|gore_core\|gore-cli" README.md docs/ | grep -v "docs/superpowers/plans"`
Expected: only intentional historical mentions (e.g. tag prefix `gore-cli-v`) remain; fix the rest.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "docs: rewrite README for flat gore layout"
```

### Task 13: Rename the GitHub repo and update embedded URLs

**Files:**
- Modify: `apps/save-editor/lib/features/app/ui/about_dialog.dart` (`_githubUrl`)
- Modify: `apps/save-editor/lib/features/app/domain/desktop_updater.dart` (`_appcastUrl` — optional, see note)
- Manual: GitHub repo rename (gh/web)

- [ ] **Step 1: Rename the GitHub repo**

Run: `gh repo rename gore --repo dh0er/goresave`
Then update the local remote: `git remote set-url origin git@github.com:dh0er/gore.git`.
Note: GitHub permanently redirects `dh0er/goresave/*` → `dh0er/gore/*`, so shipped v0.3.0 apps polling the old appcast URL keep working (invariant #3).

- [ ] **Step 2: Update the about-dialog URL**

In `apps/save-editor/lib/features/app/ui/about_dialog.dart`, change `_githubUrl` from `https://github.com/dh0er/goresave` to `https://github.com/dh0er/gore`.

- [ ] **Step 3: Update the appcast feed URL (forward-looking)**

In `apps/save-editor/lib/features/app/domain/desktop_updater.dart`, change `_appcastUrl` host path `dh0er/goresave` → `dh0er/gore`. This only affects *future* builds; shipped users keep using the redirected old URL. The feed asset name `appcast-windows.xml` and `releases/latest` path stay (invariant #3).

- [ ] **Step 4: Analyze + commit**

Run: `cd apps/save-editor && flutter analyze`
Expected: `No issues found!`
```bash
git add -A
git commit -m "chore: point embedded URLs at renamed gore repo (old URL redirects)"
```

---

## Phase 6 — Final verification

### Task 14: Full green sweep

**Files:** none (verification only)

- [ ] **Step 1: Clean build of the whole workspace**

Run: `cargo build --workspace && cargo test --workspace`
Expected: green; pass count matches the Task 0 baseline.

- [ ] **Step 2: Confirm the unified binary surface**

Run: `./target/debug/gore --help` then `./target/debug/gore as --help`
Expected: all original `gore-cli` subcommands plus the `as` group are present.

- [ ] **Step 3: Both apps analyze + the DLLs load**

Run: `cd apps/save-editor && flutter analyze` and `cd apps/mod-studio && flutter analyze`
Expected: `No issues found!` for both. Confirm `target/debug/gore_save.dll` and `target/debug/gore_ffi.dll` exist.

- [ ] **Step 4: No stale identifiers anywhere**

Run: `grep -rn "gore_core\|goresave_core\|goresave_oodle\|gore_cli\|projects/" --include=*.rs --include=*.toml --include=*.dart --include=*.py --include=*.yml . | grep -v target | grep -v "docs/superpowers/plans"`
Expected: zero hits except the deliberately preserved C symbols (`goresave_execute`, `gore_core_execute`) and the `gore-cli` tag prefix.

- [ ] **Step 5: Verify the preserved invariants are intact**

Run: `grep -n "C7A35D8E-4B61-4E0D-9C0A-2F8B5D1E6A43" apps/save-editor/installer/setup.iss` (AppId present) and `grep -rn "goresave_execute\|gore_core_execute" crates/` (symbols present) and `grep -n "appcast-windows.xml" apps/save-editor/lib/features/app/domain/desktop_updater.dart`.
Expected: all three present.

- [ ] **Step 6: Final commit (if any verification fixups were needed)**

```bash
git add -A
git commit -m "chore: final verification fixups for gore restructure"
```

---

## Self-review notes

- **Spec coverage:** flatten (Phase 1), workspace version model (Task 3b), rename crates (Phase 2), split gore_core (Task 8), unify binary incl. gore-as fold (Task 9), build.py (Task 10), CI (Task 11), README (Task 12), repo rename (Task 13), final sweep (Task 14). All elements of the agreed design are covered.
- **Versioning:** per-product independent semver (3 tags), internal libs inherit one `[workspace.package]` version, `gore` binary keeps its own — decided in the header, implemented in Task 3b, and the new-crate snippets in Task 8 use the inherited form.
- **Invariants:** AppId, DSA key, appcast feed, and C FFI symbol names are explicitly preserved and re-verified in Task 14 Step 5.
- **Type consistency:** the module→crate table is applied identically in Task 8 Steps 12–13; consumer import rewrites are enumerated per file.
- **Open verification during execution:** Task 8 Step 3 offers two ways to keep the `catalog`/`pipeline` module shape — pick the "rename catalog.rs to lib.rs" path unless `pipeline.rs` has `super::` references that make the `#[path]` form simpler; either way `cargo build -p gore-catalog` is the gate.
