# mod-studio AngelScript Tab — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Scripts tab to mod-studio that stages AngelScript `.as` mods, compiles them via the game, and splices the bytecode into the precompiled-script cache at Build/Deploy.

**Architecture:** Mirror the existing Audio/Texture feature-tabs. A new Flutter `lib/scripts/` domain holds staged `ScriptMod`s; an explicit **Compile** button turns each `.as` into a cached 1-module mini-cache (via a new FFI `script_compile` that drives the game's `-as-generate-precompiled-data`); Build/Deploy carries the mini-caches in the bundle and `gore-mod` splices (add) / replaces (edit) them into `PrecompiledScript_Shipping.Cache` in-place, with a `*.gore-bak` backup. All cache surgery reuses the existing `gore-as` crate.

**Tech Stack:** Rust (`gore-as`, `gore-mod`, `gore-ffi`), Flutter + Riverpod (`apps/mod-studio`), serde JSON over the dart:ffi bridge.

**Spec:** [`docs/superpowers/specs/2026-06-30-mod-studio-angelscript-tab-design.md`](../specs/2026-06-30-mod-studio-angelscript-tab-design.md)

---

## Toolchain & test commands

- **Rust** (from repo root): `cargo test -p gore-mod`, `cargo test -p gore-ffi`, `cargo test -p gore-as`. Build all: `cargo build`.
- **Flutter** (from `apps/mod-studio`): `flutter test` (the repo pins the SDK via `.fvmrc`; if `fvm` is installed, prefix `fvm `, e.g. `fvm flutter test`). Static check: `flutter analyze`.
- Some tests need the real 122 MB game cache / a running game; those are marked `#[ignore]` (Rust) or `@Tags(['game'])` (Dart) and are **not** part of the normal suite. Each task says which.

## ⚠️ One unverified area — the game compile invocation (Task 11)

Everything except the **game launch** inside `run_regen` is fully specified and testable offline. The exact exe arguments, the directory the game reads loose `.as` from, and where it writes the regen cache live in the gitignored `work/reversing/gore-as` findings and are **assumed** in this plan (`-as-generate-precompiled-data`, loose `.as` under `G1R/Script/`, regen written back to `PrecompiledScript_Shipping.Cache`). `run_regen` **preflights and errors** if the regen cache isn't produced, so a wrong assumption is a localized, surfaced fix — not a silent failure. **Before implementing Task 11, confirm these three facts against the proven manual run.** Tasks 1–10 and 12 do not depend on them.

---

## File structure

**Rust — modify:**
- `crates/gore-mod/Cargo.toml` — add `gore-as` dep.
- `crates/gore-mod/src/lib.rs` — `ScriptModule`, `ScriptEntry`, `BuildSpec.scripts`, `Component::AngelScriptPatch`, `GamePaths.script_cache`, `build_bundle` + `prepare` arms.
- `crates/gore-ffi/Cargo.toml` — add `gore-as` dep.
- `crates/gore-ffi/src/lib.rs` — `script_list_modules`, `script_emit_module`, `script_compile` commands + dispatch.
- `crates/gore/src/cmd/as_cache.rs` — `AsCmd::EmitAll` delegates to the new lib fn.

**Rust — create:**
- `crates/gore-as/src/cache/emit_all.rs` — `emit_all_tree` (lifted from the CLI) + `rename_free_fn`.
- `crates/gore-as/src/compile.rs` — `compile_module` orchestration (offline glue + injectable `run_regen`).

**Flutter — create:**
- `lib/scripts/domain/script_mods_notifier.dart` — `ScriptMod`, state, notifier, provider.
- `lib/scripts/domain/script_modules_provider.dart` — vanilla-module list FutureProvider.
- `lib/scripts/ui/script_tab.dart` — the tab UI.
- `test/scripts/script_mods_notifier_test.dart`, `test/scripts/project_scripts_test.dart`, `test/scripts/script_tab_test.dart`.

**Flutter — modify:**
- `lib/project/project_model.dart`, `lib/project/project_io.dart`, `lib/project/project_controller.dart`.
- `lib/core/mod_ffi.dart` — three wrappers + `ScriptModuleInfo`.
- `lib/home_page.dart`, `lib/editor/ui/overrides_panel.dart`, `lib/export/ui/build_deploy_dialog.dart`.

---

## Task 1: gore-mod — script types + build_bundle

**Files:**
- Modify: `crates/gore-mod/Cargo.toml`
- Modify: `crates/gore-mod/src/lib.rs` (types near line 56–92; `build_bundle` near 154–167; `GamePaths` 234–272; existing test literals 1362/1397/1414)
- Test: `crates/gore-mod/src/lib.rs` (`#[cfg(test)]` module)

- [ ] **Step 1: Add the gore-as dependency**

In `crates/gore-mod/Cargo.toml`, under `[dependencies]` after the `gore-tex` line:

```toml
gore-as = { path = "../gore-as" }
```

- [ ] **Step 2: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `crates/gore-mod/src/lib.rs`:

```rust
#[test]
fn build_emits_angelscript_patch() {
    let dir = std::env::temp_dir().join("gore-mod-as-build");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mini = dir.join("mod.cache");
    std::fs::write(&mini, b"MINI-CACHE-BYTES").unwrap();
    let spec = BuildSpec {
        meta: ModMeta { name: "AsMod".into(), version: String::new(), author: String::new() },
        delay_ms: 0,
        overrides: vec![],
        loc_edits: Default::default(),
        audio: vec![],
        texture: vec![],
        scripts: vec![ScriptModule {
            op: "add".into(),
            module_name: "MyMod".into(),
            mini_cache: mini.display().to_string(),
        }],
    };
    let bundle = build_bundle(&spec).unwrap();
    assert!(bundle.files.contains_key("scripts/manifest.json"));
    assert!(bundle.files.contains_key("scripts/0_MyMod.cache"));
    assert_eq!(bundle.files["scripts/0_MyMod.cache"], b"MINI-CACHE-BYTES");
    assert!(matches!(bundle.manifest.components.last(),
        Some(Component::AngelScriptPatch { path }) if path == "scripts"));
    // manifest round-trips to the typed entry
    let m: Vec<ScriptEntry> =
        serde_json::from_slice(&bundle.files["scripts/manifest.json"]).unwrap();
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].op, "add");
    assert_eq!(m[0].module, "MyMod");
    assert_eq!(m[0].mini, "scripts/0_MyMod.cache");
}
```

- [ ] **Step 3: Run the test to verify it fails to compile**

Run: `cargo test -p gore-mod build_emits_angelscript_patch`
Expected: FAIL — `ScriptModule`, `ScriptEntry`, `BuildSpec.scripts`, `Component::AngelScriptPatch` don't exist; the other `BuildSpec` literals are now missing the `scripts` field.

- [ ] **Step 4: Add the types**

In `crates/gore-mod/src/lib.rs`, after the `TextureReplacement` struct (line 61):

```rust
/// One AngelScript module mod: splice (`op = "add"`) or replace (`op = "edit"`) the compiled
/// 1-module mini-cache at `mini_cache` into the precompiled-script cache at deploy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptModule {
    pub op: String,          // "add" | "edit"
    pub module_name: String, // the Modules TMap key (used for "edit"/replace)
    pub mini_cache: String,  // path to the compiled 1-module mini-cache on disk
}

/// One entry in a bundle's `scripts/manifest.json`: `mini` is a bundle-relative path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptEntry {
    pub op: String,
    pub module: String,
    pub mini: String,
}
```

Add the field to `BuildSpec` (after the `texture` field, line 77):

```rust
    #[serde(default)]
    pub scripts: Vec<ScriptModule>,
```

Add the variant to `Component` (after `TexturePatch`, line 91):

```rust
    /// AngelScript mini-caches at `path` (manifest.json + `*.cache`); deploy splices/replaces
    /// them into `PrecompiledScript_Shipping.Cache` in place, with a `*.gore-bak` backup.
    AngelScriptPatch { path: String },
```

- [ ] **Step 5: Add the build_bundle arm**

In `build_bundle`, after the textures block (after line 167, before `let manifest = ...`):

```rust
    // scripts → manifest + compiled mini-caches (spliced/replaced at deploy)
    if !spec.scripts.is_empty() {
        let mut entries: Vec<ScriptEntry> = Vec::new();
        for (i, s) in spec.scripts.iter().enumerate() {
            if s.op != "add" && s.op != "edit" {
                return Err(ModError::Other(format!(
                    "invalid script op {:?} for module {:?} (want \"add\" or \"edit\")",
                    s.op, s.module_name
                )));
            }
            let mini = std::fs::read(&s.mini_cache)
                .map_err(io(&format!("reading mini-cache {}", s.mini_cache)))?;
            let mini_rel = format!("scripts/{i}_{}.cache", sanitize(&s.module_name));
            files.insert(mini_rel.clone(), mini);
            entries.push(ScriptEntry { op: s.op.clone(), module: s.module_name.clone(), mini: mini_rel });
        }
        files.insert("scripts/manifest.json".into(), serde_json::to_vec_pretty(&entries)?);
        components.push(Component::AngelScriptPatch { path: "scripts".into() });
    }
```

- [ ] **Step 6: Add the GamePaths field**

In `GamePaths` (line 234) add a field:

```rust
    pub script_cache: PathBuf,
```

In `resolve_game_paths`, inside the returned `GamePaths { ... }` (line 267):

```rust
        script_cache: g1r.join("Script").join("PrecompiledScript_Shipping.Cache"),
```

- [ ] **Step 7: Fix the existing BuildSpec literals**

Run: `cargo build -p gore-mod 2>&1 | head -40` to find every `BuildSpec { ... }` missing `scripts`. There are three in the test module (the `build_bundle_overrides_loc_audio`, `empty_name_rejected`, and `build_emits_texture_patch` tests). Add `scripts: vec![],` after the `texture: ...,` line in each. Also run `git grep -n "BuildSpec {" crates/ apps/` and add `scripts: vec![]` to any other struct literal (deserialized specs are unaffected thanks to `#[serde(default)]`).

- [ ] **Step 8: Run the tests**

Run: `cargo test -p gore-mod`
Expected: PASS — `build_emits_angelscript_patch` plus all pre-existing tests.

- [ ] **Step 9: Commit**

```bash
git add crates/gore-mod/Cargo.toml crates/gore-mod/src/lib.rs
git commit -m "feat(gore-mod): script module build spec + bundle component"
```

---

## Task 2: gore-mod — prepare splices scripts into the cache

**Files:**
- Modify: `crates/gore-mod/src/lib.rs` (`prepare`, match block at line 669–851)
- Test: `crates/gore-mod/src/lib.rs` (`#[cfg(test)]`)

- [ ] **Step 1: Write the failing tests**

Add to the test module:

```rust
/// prepare() must reject a manifest whose op is neither add nor edit, naming the module.
#[test]
fn prepare_rejects_bad_script_op() {
    let dir = std::env::temp_dir().join("gore-mod-as-prep-badop");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("scripts")).unwrap();
    std::fs::write(dir.join("scripts/0_M.cache"), b"x").unwrap();
    let entries = vec![ScriptEntry { op: "nope".into(), module: "M".into(), mini: "scripts/0_M.cache".into() }];
    std::fs::write(dir.join("scripts/manifest.json"), serde_json::to_vec(&entries).unwrap()).unwrap();
    let manifest = ModManifest {
        format: 1,
        mod_meta: ModMeta { name: "M".into(), version: String::new(), author: String::new() },
        components: vec![Component::AngelScriptPatch { path: "scripts".into() }],
    };
    // A game dir whose script cache file exists (content irrelevant — op is rejected first).
    let game = dir.join("game");
    let script_dir = game.join("G1R/Script");
    std::fs::create_dir_all(&script_dir).unwrap();
    std::fs::write(script_dir.join("PrecompiledScript_Shipping.Cache"), b"base").unwrap();
    let gp = resolve_game_paths(&game);
    let err = prepare(&dir, &manifest, &gp, None).unwrap_err();
    assert!(err.to_string().contains("invalid script op"), "got: {err}");
}

/// prepare() must error clearly when the game has no script cache.
#[test]
fn prepare_errors_when_no_script_cache() {
    let dir = std::env::temp_dir().join("gore-mod-as-prep-nocache");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("scripts")).unwrap();
    std::fs::write(dir.join("scripts/0_M.cache"), b"x").unwrap();
    let entries = vec![ScriptEntry { op: "add".into(), module: "M".into(), mini: "scripts/0_M.cache".into() }];
    std::fs::write(dir.join("scripts/manifest.json"), serde_json::to_vec(&entries).unwrap()).unwrap();
    let manifest = ModManifest {
        format: 1,
        mod_meta: ModMeta { name: "M".into(), version: String::new(), author: String::new() },
        components: vec![Component::AngelScriptPatch { path: "scripts".into() }],
    };
    let gp = resolve_game_paths(&dir.join("empty-game"));
    let err = prepare(&dir, &manifest, &gp, None).unwrap_err();
    assert!(err.to_string().contains("script cache not found"), "got: {err}");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p gore-mod prepare_`
Expected: FAIL — `prepare` has no `AngelScriptPatch` arm (non-exhaustive match won't compile).

- [ ] **Step 3: Add the prepare arm**

In `prepare`, inside the `match comp { ... }` (after the `Component::TexturePatch` arm closes at line 849):

```rust
            Component::AngelScriptPatch { path } => {
                if !is_safe_rel_path(path) {
                    return Err(ModError::Other(format!("unsafe script patch path: {path:?}")));
                }
                let entries: Vec<ScriptEntry> = serde_json::from_slice(
                    &std::fs::read(bundle_dir.join(path).join("manifest.json"))
                        .map_err(io("reading script manifest"))?,
                )?;
                let cache_path = gp.script_cache.clone();
                if !cache_path.exists() {
                    return Err(ModError::Other(format!(
                        "script cache not found: {} — is the game path correct?",
                        cache_path.display()
                    )));
                }
                let (pristine, drifted) = read_pristine(&cache_path, prev)?;
                if drifted {
                    plan.refresh_baks.push(cache_path.clone());
                }
                let mut running = pristine;
                for e in &entries {
                    if !is_safe_rel_path(&e.mini) {
                        return Err(ModError::Other(format!("unsafe mini path: {:?}", e.mini)));
                    }
                    let mini = std::fs::read(bundle_dir.join(&e.mini))
                        .map_err(io("reading mini-cache"))?;
                    running = match e.op.as_str() {
                        "add" => gore_as::cache::splice::splice_auto(&running, &mini)
                            .map_err(|err| ModError::Other(format!("splice {}: {err}", e.module)))?,
                        "edit" => gore_as::cache::splice::replace_module(&running, &mini, &e.module)
                            .map_err(|err| ModError::Other(format!("replace {}: {err}", e.module)))?,
                        other => return Err(ModError::Other(format!(
                            "invalid script op {other:?} for module {:?}", e.module
                        ))),
                    };
                }
                plan.writes.push((cache_path, running));
            }
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p gore-mod`
Expected: PASS.

- [ ] **Step 5: Add an `#[ignore]` end-to-end test (real cache)**

This documents the full add/edit path; it is skipped unless run explicitly with a real game cache via `GORE_TEST_GAME`.

```rust
/// Full splice against a real game install. Run with:
///   GORE_TEST_GAME="C:/.../Gothic 1 Remake"  cargo test -p gore-mod -- --ignored real_script_deploy
#[test]
#[ignore]
fn real_script_deploy_add_roundtrips() {
    let Ok(game) = std::env::var("GORE_TEST_GAME") else { return; };
    let game = std::path::PathBuf::from(game);
    let gp = resolve_game_paths(&game);
    assert!(gp.script_cache.exists(), "no script cache at {}", gp.script_cache.display());
    // A mini-cache produced by `gore as extract`/`script_compile` (Task 11). Provide its path:
    let Ok(mini) = std::env::var("GORE_TEST_MINI") else {
        eprintln!("set GORE_TEST_MINI to a 1-module mini-cache to run the splice");
        return;
    };
    let dir = std::env::temp_dir().join("gore-mod-as-real");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let spec = BuildSpec {
        meta: ModMeta { name: "RealAsMod".into(), version: String::new(), author: String::new() },
        delay_ms: 0, overrides: vec![], loc_edits: Default::default(), audio: vec![], texture: vec![],
        scripts: vec![ScriptModule { op: "add".into(), module_name: "ignored_for_add".into(), mini_cache: mini }],
    };
    let bundle = build_bundle(&spec).unwrap();
    write_bundle(&dir, &bundle).unwrap();
    let manifest = bundle.manifest;
    let before = module_count(&std::fs::read(&gp.script_cache).unwrap());
    let plan = prepare(&dir, &manifest, &gp, None).unwrap();
    let (_, spliced) = plan.writes.last().unwrap();
    assert_eq!(module_count(spliced), before + 1, "splice should add exactly one module");
}
```

Add `use gore_as::cache::walk_modules::module_count;` to the test module if not already imported (import it inside the test or at the top of the `tests` mod).

- [ ] **Step 6: Commit**

```bash
git add crates/gore-mod/src/lib.rs
git commit -m "feat(gore-mod): splice/replace script modules in prepare()"
```

---

## Task 3: gore-ffi — list + emit script commands

**Files:**
- Modify: `crates/gore-ffi/Cargo.toml`
- Modify: `crates/gore-ffi/src/lib.rs` (dispatch 81–96; new fns; tests)

- [ ] **Step 1: Add the dependency**

In `crates/gore-ffi/Cargo.toml` under `[dependencies]` after `gore-tex`:

```toml
gore-as = { path = "../gore-as" }
```

- [ ] **Step 2: Write the failing tests**

Add to the `#[cfg(test)] mod tests` in `crates/gore-ffi/src/lib.rs`:

```rust
#[test]
fn script_list_modules_requires_cache() {
    let v: Value = serde_json::from_str(&execute_json(
        r#"{"command":"script_list_modules","payload":{}}"#,
    )).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["code"], "BAD_REQUEST");
}

#[test]
fn script_emit_module_requires_args() {
    let v: Value = serde_json::from_str(&execute_json(
        r#"{"command":"script_emit_module","payload":{"cache":"x"}}"#,
    )).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["code"], "BAD_REQUEST");
}
```

- [ ] **Step 3: Run to verify failure**

Run: `cargo test -p gore-ffi script_`
Expected: FAIL — both return `UNKNOWN_COMMAND` (not registered).

- [ ] **Step 4: Register the commands**

In `dispatch` (line 93, after `"texture_extract" => texture_extract(payload),`):

```rust
        "script_list_modules" => script_list_modules(payload),
        "script_emit_module" => script_emit_module(payload),
```

- [ ] **Step 5: Implement the two functions**

Add near the other command fns in `crates/gore-ffi/src/lib.rs`. The class-hierarchy/native-API setup mirrors `gore/src/cmd/as_cache.rs`:

```rust
/// Build the class name -> super name map so emitted source gets subclass casts right.
fn as_class_hierarchy(mods: &[gore_as::cache::model::Module]) -> std::collections::HashMap<String, String> {
    let mut h = std::collections::HashMap::new();
    for m in mods {
        for c in &m.classes {
            let sup = c.super_class.clone().filter(|s| !s.is_empty()).unwrap_or_default();
            h.insert(c.name.clone(), sup);
        }
    }
    h
}

/// Load native arities from a `Binds.Cache` sitting next to `cache_file`, if present.
fn as_native_api(cache_file: &std::path::Path) -> Option<gore_as::cache::binds::NativeApi> {
    let path = cache_file.parent()?.join("Binds.Cache");
    if !path.exists() { return None; }
    gore_as::cache::binds::NativeApi::load(&path)
}

/// `{cache}` → `{ok, modules:[{name, file}]}` — list modules in a precompiled cache.
fn script_list_modules(payload: Value) -> Value {
    let Some(cache) = payload.get("cache").and_then(Value::as_str) else {
        return err("BAD_REQUEST", "missing 'cache'");
    };
    let bytes = match std::fs::read(cache) {
        Ok(b) => b,
        Err(e) => return err("IO", format!("reading cache: {e}")),
    };
    let mods = match gore_as::cache::model::parse_modules(&bytes) {
        Ok(m) => m,
        Err(e) => return err("PARSE", format!("parsing cache: {e}")),
    };
    let modules: Vec<Value> = mods.iter()
        .map(|m| json!({"name": m.name, "file": m.file}))
        .collect();
    json!({"ok": true, "modules": modules})
}

/// `{cache, module}` → `{ok, source}` — emit recompilable .as for one module.
fn script_emit_module(payload: Value) -> Value {
    let (Some(cache), Some(module)) = (
        payload.get("cache").and_then(Value::as_str),
        payload.get("module").and_then(Value::as_str),
    ) else {
        return err("BAD_REQUEST", "missing 'cache' or 'module'");
    };
    let bytes = match std::fs::read(cache) {
        Ok(b) => b,
        Err(e) => return err("IO", format!("reading cache: {e}")),
    };
    let mut refs = match gore_as::cache::refs::RefResolver::build(&bytes) {
        Ok(r) => r,
        Err(e) => return err("RESOLVER", format!("{e}")),
    };
    let mods = match gore_as::cache::model::parse_modules(&bytes) {
        Ok(m) => m,
        Err(e) => return err("PARSE", format!("{e}")),
    };
    refs.set_class_hierarchy(as_class_hierarchy(&mods));
    if let Some(api) = as_native_api(std::path::Path::new(cache)) {
        refs.set_native_api(api);
    }
    let Some(m) = mods.iter().find(|m| m.name == module) else {
        return err("NOT_FOUND", format!("module not found: {module}"));
    };
    let source = gore_as::cache::emit::emit_module(m, &refs);
    json!({"ok": true, "source": source})
}
```

- [ ] **Step 6: Run the tests**

Run: `cargo test -p gore-ffi`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/gore-ffi/Cargo.toml crates/gore-ffi/src/lib.rs
git commit -m "feat(gore-ffi): script_list_modules + script_emit_module"
```

---

## Task 4: gore-as — `emit_all_tree` library function

Move the full-tree `.as` emission (with cross-module free-function de-collision) out of the CLI into the library so the compile orchestration (Task 11) can reuse it.

**Files:**
- Create: `crates/gore-as/src/cache/emit_all.rs`
- Modify: `crates/gore-as/src/cache/mod.rs` (add `pub mod emit_all;`) — if `gore-as` declares cache submodules in `src/lib.rs` instead, add it there.
- Modify: `crates/gore/src/cmd/as_cache.rs` (replace `AsCmd::EmitAll` body + delete the now-moved `rename_free_fn`)

- [ ] **Step 1: Confirm where cache submodules are declared**

Run: `git grep -n "pub mod splice\|pub mod model\|mod emit" crates/gore-as/src`
Use the file that lists the other `cache::*` modules; add `pub mod emit_all;` there.

- [ ] **Step 2: Create `emit_all.rs`**

Create `crates/gore-as/src/cache/emit_all.rs`. Move `rename_free_fn` (currently `crates/gore/src/cmd/as_cache.rs:9-27`) verbatim, and lift the body of `AsCmd::EmitAll` (`as_cache.rs:203-318`) into this function. Keep the logic identical — only change `println!/eprintln!` reporting into the returned struct, and take parsed inputs as parameters:

```rust
//! Emit ALL modules of a precompiled cache as recompilable `.as` into a directory, mirroring
//! each module's ScriptRelativeFilename. Free-function name collisions across modules are
//! de-collided per-module (AngelScript compiles all loose `.as` into one global scope).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::model::Module;
use super::refs::RefResolver;

pub struct EmitAllStats {
    pub written: usize,
    pub stubbed: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum EmitAllError {
    #[error("io: {0}")]
    Io(String),
}

/// Whole-word free rename (decl + free calls), skipping member calls (`obj.name`) and scope
/// (`A::name`). Identical to the original CLI helper.
pub fn rename_free_fn(src: &str, name: &str, newname: &str) -> String {
    // ... move the exact body from as_cache.rs:9-27 here unchanged ...
    let (b, nb) = (src.as_bytes(), name.as_bytes());
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hit = b[i..].starts_with(nb)
            && (i == 0 || (!word(b[i - 1]) && b[i - 1] != b'.' && b[i - 1] != b':'))
            && (i + nb.len() >= b.len() || !word(b[i + nb.len()]));
        if hit {
            out.extend_from_slice(newname.as_bytes());
            i += nb.len();
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| src.to_string())
}

/// Emit every module in `mods` to `outdir`, using `refs` for type/native resolution. Returns
/// how many files were written and how many contain a stubbed (not-fully-recovered) function.
pub fn emit_all_tree(mods: &[Module], refs: &RefResolver, outdir: &Path) -> Result<EmitAllStats, EmitAllError> {
    let io = |ctx: &str| move |e: std::io::Error| EmitAllError::Io(format!("{ctx}: {e}"));
    std::fs::create_dir_all(outdir).map_err(io("creating outdir"))?;
    let outdir = outdir.canonicalize().map_err(io("resolving outdir"))?;
    // ... lift the collision-detection + per-module emit + safe-path-write loop from
    //     as_cache.rs:216-317 VERBATIM, substituting `outdir` (already canonicalized) and
    //     accumulating `written`/`stubbed`. Replace the trailing eprintln! with the return. ...
    let mut sig_mods: HashMap<String, HashSet<usize>> = HashMap::new();
    // (exact body moved from the CLI — see as_cache.rs:222-317)
    // ...
    let (written, stubbed) = (0usize, 0usize); // replaced by the moved loop's counters
    Ok(EmitAllStats { written, stubbed })
}
```

> Engineer note: this is a mechanical lift. Copy lines 216–317 of the original `as_cache.rs` into the body where indicated, renaming the local `outdir` shadow to use the parameter, and return `EmitAllStats` instead of printing. Do not change behavior.

- [ ] **Step 3: Add a unit test for the moved pure helper**

Create `crates/gore-as/tests/emit_all_test.rs`:

```rust
use gore_as::cache::emit_all::rename_free_fn;

#[test]
fn renames_decl_and_free_calls_only() {
    let src = "void Foo(){} void bar(){ Foo(); obj.Foo(); A::Foo(); }";
    let out = rename_free_fn(src, "Foo", "Foo_g3");
    assert!(out.contains("void Foo_g3(){}"));
    assert!(out.contains("Foo_g3();"));      // free call renamed
    assert!(out.contains("obj.Foo();"));     // member call untouched
    assert!(out.contains("A::Foo();"));      // scoped call untouched
}
```

- [ ] **Step 4: Run to verify it fails**

Run: `cargo test -p gore-as renames_decl_and_free_calls_only`
Expected: FAIL until `emit_all` module is declared and compiles.

- [ ] **Step 5: Wire the CLI to delegate**

In `crates/gore/src/cmd/as_cache.rs`: delete the top-level `rename_free_fn` (now in the lib), and replace the `AsCmd::EmitAll { file, outdir } => { ... }` arm body with: read bytes, build `refs` + class hierarchy + native api (the arm already does this), `parse_modules`, then:

```rust
            let stats = gore_as::cache::emit_all::emit_all_tree(&mods, &refs, &outdir)
                .with_context(|| format!("emitting to {}", outdir.display()))?;
            eprintln!("emitted {} modules to {} ({} contain a stubbed function)",
                stats.written, outdir.display(), stats.stubbed);
```

- [ ] **Step 6: Run the suites**

Run: `cargo test -p gore-as && cargo build -p gore`
Expected: PASS / clean build.

- [ ] **Step 7: Commit**

```bash
git add crates/gore-as/src/cache/emit_all.rs crates/gore-as/src/cache/mod.rs crates/gore-as/tests/emit_all_test.rs crates/gore/src/cmd/as_cache.rs
git commit -m "refactor(gore-as): lift emit_all_tree into the library"
```

---

## Task 5: Flutter — ScriptMod domain

**Files:**
- Create: `apps/mod-studio/lib/scripts/domain/script_mods_notifier.dart`
- Test: `apps/mod-studio/test/scripts/script_mods_notifier_test.dart`

- [ ] **Step 1: Write the failing test**

Create `apps/mod-studio/test/scripts/script_mods_notifier_test.dart`:

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/scripts/domain/script_mods_notifier.dart';

void main() {
  test('ScriptMod json round-trips', () {
    const m = ScriptMod(
      op: ScriptOp.edit,
      moduleName: 'AI.AIItemScoring',
      relPath: 'AI/AIItemScoring.as',
      asPath: '/tmp/AIItemScoring.as',
      miniPath: '/tmp/mini.cache',
    );
    final j = m.toJson();
    final back = ScriptMod.fromJson(j);
    expect(back.op, ScriptOp.edit);
    expect(back.moduleName, 'AI.AIItemScoring');
    expect(back.relPath, 'AI/AIItemScoring.as');
    expect(back.asPath, '/tmp/AIItemScoring.as');
    expect(back.miniPath, '/tmp/mini.cache');
    expect(back.compiled, isTrue);
  });

  test('notifier set/remove/count/clear/load', () {
    final n = ScriptModsNotifier();
    expect(n.state.count, 0);
    n.setMod(const ScriptMod(op: ScriptOp.add, moduleName: 'M1', relPath: 'M1.as', asPath: 'a'));
    n.setMod(const ScriptMod(op: ScriptOp.add, moduleName: 'M2', relPath: 'M2.as', asPath: 'b'));
    expect(n.state.count, 2);
    n.remove('M1');
    expect(n.state.count, 1);
    expect(n.state.entries.single.moduleName, 'M2');
    n.loadAll([const ScriptMod(op: ScriptOp.edit, moduleName: 'M3', relPath: 'M3.as', asPath: 'c')]);
    expect(n.state.count, 1);
    expect(n.state.entries.single.op, ScriptOp.edit);
    n.clearAll();
    expect(n.state.count, 0);
  });
}
```

> Note: `gore_mod` is the Dart package name (see `pubspec.yaml`'s `name:`). Confirm with `grep '^name:' apps/mod-studio/pubspec.yaml` and use that import prefix throughout.

- [ ] **Step 2: Run to verify failure**

Run (from `apps/mod-studio`): `flutter test test/scripts/script_mods_notifier_test.dart`
Expected: FAIL — file/types don't exist.

- [ ] **Step 3: Implement the domain**

Create `apps/mod-studio/lib/scripts/domain/script_mods_notifier.dart`:

```dart
import 'package:flutter_riverpod/legacy.dart';

/// Whether a staged script mod adds a brand-new module or edits an existing one.
enum ScriptOp { add, edit }

ScriptOp scriptOpFromString(String s) => s == 'edit' ? ScriptOp.edit : ScriptOp.add;
String scriptOpToString(ScriptOp o) => o == ScriptOp.edit ? 'edit' : 'add';

/// One staged AngelScript mod: compile [asPath] into [miniPath] (a 1-module mini-cache), then
/// splice (add) / replace (edit) module [moduleName] into the precompiled cache at deploy.
class ScriptMod {
  const ScriptMod({
    required this.op,
    required this.moduleName,
    required this.relPath,
    required this.asPath,
    this.miniPath = '',
  });

  final ScriptOp op;
  final String moduleName; // Modules TMap key
  final String relPath;    // ScriptRelativeFilename, e.g. AI/Foo.as
  final String asPath;     // .as source on disk (embedded in the .goremod)
  final String miniPath;   // compiled mini-cache on disk ('' until compiled)

  String get key => moduleName;
  bool get compiled => miniPath.isNotEmpty;

  Map<String, Object?> toJson() => {
        'op': scriptOpToString(op),
        'module': moduleName,
        'rel_path': relPath,
        'as_path': asPath,
        'mini_path': miniPath,
      };

  factory ScriptMod.fromJson(Map<String, Object?> j) => ScriptMod(
        op: scriptOpFromString((j['op'] as String?) ?? 'add'),
        moduleName: j['module'] as String,
        relPath: (j['rel_path'] as String?) ?? '',
        asPath: j['as_path'] as String,
        miniPath: (j['mini_path'] as String?) ?? '',
      );

  ScriptMod withAsPath(String path) =>
      ScriptMod(op: op, moduleName: moduleName, relPath: relPath, asPath: path, miniPath: miniPath);
  ScriptMod withMiniPath(String path) =>
      ScriptMod(op: op, moduleName: moduleName, relPath: relPath, asPath: asPath, miniPath: path);
}

class ScriptModsState {
  const ScriptModsState({this.items = const {}});
  final Map<String, ScriptMod> items;
  int get count => items.length;
  List<ScriptMod> get entries => items.values.toList()
    ..sort((a, b) => a.moduleName.compareTo(b.moduleName));
  ScriptModsState copyWith({Map<String, ScriptMod>? items}) =>
      ScriptModsState(items: items ?? this.items);
}

class ScriptModsNotifier extends StateNotifier<ScriptModsState> {
  ScriptModsNotifier() : super(const ScriptModsState());
  void setMod(ScriptMod m) {
    final items = Map<String, ScriptMod>.from(state.items);
    items[m.key] = m;
    state = state.copyWith(items: items);
  }
  void remove(String key) {
    if (!state.items.containsKey(key)) return;
    final items = Map<String, ScriptMod>.from(state.items)..remove(key);
    state = state.copyWith(items: items);
  }
  void clearAll() {
    if (state.items.isEmpty) return;
    state = const ScriptModsState();
  }
  void loadAll(List<ScriptMod> list) {
    state = ScriptModsState(items: {for (final m in list) m.key: m});
  }
}

final scriptModsProvider =
    StateNotifierProvider<ScriptModsNotifier, ScriptModsState>((ref) => ScriptModsNotifier());
```

- [ ] **Step 4: Run the test**

Run: `flutter test test/scripts/script_mods_notifier_test.dart`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/mod-studio/lib/scripts/domain/script_mods_notifier.dart apps/mod-studio/test/scripts/script_mods_notifier_test.dart
git commit -m "feat(mod-studio): ScriptMod domain + notifier"
```

---

## Task 6: Flutter — project model carries scripts

**Files:**
- Modify: `apps/mod-studio/lib/project/project_model.dart`
- Test: `apps/mod-studio/test/scripts/project_scripts_test.dart`

- [ ] **Step 1: Write the failing test**

Create `apps/mod-studio/test/scripts/project_scripts_test.dart`:

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/project_model.dart';
import 'package:gore_mod/scripts/domain/script_mods_notifier.dart';

void main() {
  test('ModProject round-trips scripts and emits build spec', () {
    final p = ModProject(
      name: 'M',
      scripts: const [
        ScriptMod(op: ScriptOp.add, moduleName: 'New', relPath: 'New.as', asPath: '/a/New.as', miniPath: '/a/new.cache'),
        ScriptMod(op: ScriptOp.edit, moduleName: 'AI.Foo', relPath: 'AI/Foo.as', asPath: '/a/Foo.as', miniPath: '/a/foo.cache'),
      ],
    );
    final back = ModProject.fromJson(p.toJson());
    expect(back.scripts.length, 2);
    expect(back.scripts.first.moduleName, 'New');

    final spec = p.toBuildSpec();
    final scripts = spec['scripts'] as List;
    expect(scripts.length, 2);
    expect((scripts.first as Map)['op'], 'add');
    expect((scripts.first as Map)['module_name'], 'New');
    expect((scripts.first as Map)['mini_cache'], '/a/new.cache');
  });
}
```

- [ ] **Step 2: Run to verify failure**

Run: `flutter test test/scripts/project_scripts_test.dart`
Expected: FAIL — `ModProject` has no `scripts`.

- [ ] **Step 3: Add scripts to the model**

In `apps/mod-studio/lib/project/project_model.dart`:

Import at the top (after the textures import, line 3):

```dart
import '../scripts/domain/script_mods_notifier.dart';
```

Add a constructor param (after `this.textures = const [],` line 16) and field (after `final List<TextureReplacement> textures;` line 26):

```dart
    this.scripts = const [],
```
```dart
  final List<ScriptMod> scripts;
```

Extend `copyWith` (lines 28–41) to thread scripts:

```dart
  ModProject copyWith({
    List<AudioReplacement>? audio,
    List<TextureReplacement>? textures,
    List<ScriptMod>? scripts,
  }) =>
      ModProject(
        name: name,
        version: version,
        author: author,
        delayMs: delayMs,
        overrides: overrides,
        locEdits: locEdits,
        audio: audio ?? this.audio,
        textures: textures ?? this.textures,
        scripts: scripts ?? this.scripts,
      );
```

In `toJson` (after the `'textures': ...` line 53):

```dart
        'scripts': [for (final s in scripts) s.toJson()],
```

In `fromJson` (after the `textures: [...]` block, line 75):

```dart
      scripts: [
        for (final s in (j['scripts'] as List? ?? const []))
          ScriptMod.fromJson((s as Map).cast<String, Object?>())
      ],
```

In `toBuildSpec` (after the `'texture': ...` line 86):

```dart
        'scripts': [
          for (final s in scripts)
            {'op': scriptOpToString(s.op), 'module_name': s.moduleName, 'mini_cache': s.miniPath}
        ],
```

- [ ] **Step 4: Run the test**

Run: `flutter test test/scripts/project_scripts_test.dart`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/mod-studio/lib/project/project_model.dart apps/mod-studio/test/scripts/project_scripts_test.dart
git commit -m "feat(mod-studio): ModProject carries script mods"
```

---

## Task 7: Flutter — project IO + controller plumbing

**Files:**
- Modify: `apps/mod-studio/lib/project/project_io.dart`
- Modify: `apps/mod-studio/lib/project/project_controller.dart`
- Test: append to `apps/mod-studio/test/scripts/project_scripts_test.dart`

- [ ] **Step 1: Write the failing test**

Append to `apps/mod-studio/test/scripts/project_scripts_test.dart` (add the imports `dart:io`, `package:path/path.dart as p`, and `package:gore_mod/project/project_io.dart`):

```dart
  test('saveProject/loadProject embeds and restores script .as + mini', () async {
    final tmp = await Directory.systemTemp.createTemp('goremod_scripts_test_');
    final asFile = File(p.join(tmp.path, 'New.as'))..writeAsStringSync('void Foo(){}');
    final miniFile = File(p.join(tmp.path, 'new.cache'))..writeAsBytesSync([1, 2, 3]);
    final project = ModProject(
      name: 'M',
      scripts: [
        ScriptMod(op: ScriptOp.add, moduleName: 'New', relPath: 'New.as',
            asPath: asFile.path, miniPath: miniFile.path),
      ],
    );
    final out = p.join(tmp.path, 'm.goremod');
    await saveProject(project, out);
    final loaded = await loadProject(out);
    expect(loaded.scripts.length, 1);
    final s = loaded.scripts.single;
    expect(File(s.asPath).readAsStringSync(), 'void Foo(){}');
    expect(File(s.miniPath).readAsBytesSync(), [1, 2, 3]);
  });
```

- [ ] **Step 2: Run to verify failure**

Run: `flutter test test/scripts/project_scripts_test.dart`
Expected: FAIL — scripts aren't embedded/restored.

- [ ] **Step 3: Embed scripts in saveProject**

In `apps/mod-studio/lib/project/project_io.dart`:

Import (after the textures import, line 8):

```dart
import '../scripts/domain/script_mods_notifier.dart';
```

In `saveProject`, after the textures embedding loop (after line 36, before `final embedded = ...`):

```dart
  final embeddedScripts = <ScriptMod>[];
  var sidx = 0;
  for (final s in project.scripts) {
    final asBytes = await File(s.asPath).readAsBytes();
    final asRel = 'assets/scripts/${sidx}_${p.basename(s.asPath)}';
    archive.addFile(ArchiveFile(asRel, asBytes.length, asBytes));
    var rebuilt = s.withAsPath(asRel);
    // Embed the compiled mini-cache too, if this mod has been compiled.
    if (s.miniPath.isNotEmpty) {
      final miniBytes = await File(s.miniPath).readAsBytes();
      final miniRel = 'assets/scripts_cache/${sidx}_${p.basename(s.miniPath)}';
      archive.addFile(ArchiveFile(miniRel, miniBytes.length, miniBytes));
      rebuilt = rebuilt.withMiniPath(miniRel);
    }
    sidx++;
    embeddedScripts.add(rebuilt);
  }
```

Update the `embedded` copyWith (line 38–39) to include scripts:

```dart
  final embedded =
      project.copyWith(audio: embeddedAudio, textures: embeddedTextures, scripts: embeddedScripts);
```

- [ ] **Step 4: Restore scripts in loadProject**

In `loadProject`, after the textures extraction loop (after line 117, before `return project.copyWith(...)`):

```dart
  final extractedScripts = <ScriptMod>[];
  for (final s in project.scripts) {
    // Untrusted embedded paths: only extract entries under assets/ with no '..' / absolute path,
    // resolving strictly inside the temp dir. Same guard as audio/textures.
    String? extract(String rel) {
      final segs = rel.split('/');
      final safe = rel.startsWith('assets/') && !p.isAbsolute(rel) && !segs.contains('..');
      if (!safe) return null;
      final f = archive.findFile(rel);
      final out = p.joinAll([tmp.path, ...segs]);
      if (f == null || !p.isWithin(tmp.path, out)) return null;
      Directory(p.dirname(out)).createSync(recursive: true);
      File(out).writeAsBytesSync(f.content as List<int>);
      return out;
    }

    final asOut = extract(s.asPath);
    if (asOut == null) continue; // unsafe/missing source: drop the mod
    var rebuilt = s.withAsPath(asOut);
    if (s.miniPath.isNotEmpty) {
      final miniOut = extract(s.miniPath);
      // A missing/unsafe mini just means "not compiled" — keep the source, clear the mini.
      rebuilt = rebuilt.withMiniPath(miniOut ?? '');
    }
    extractedScripts.add(rebuilt);
  }
```

Update the final return (line 119):

```dart
  return project.copyWith(
      audio: extractedAudio, textures: extractedTextures, scripts: extractedScripts);
```

- [ ] **Step 5: Wire the controller**

In `apps/mod-studio/lib/project/project_controller.dart`:

Import (after the textures import, line 10):

```dart
import '../scripts/domain/script_mods_notifier.dart';
```

In `projectIsDirty` (line 25–29) add a clause:

```dart
    ref.watch(scriptModsProvider).count > 0 ||
```

In `hasUnsavedChanges`'s null-baseline branch (line 50–53) add:

```dart
        ref.read(scriptModsProvider).count > 0 ||
```

In `gatherProject` (line 61–70) add:

```dart
    scripts: ref.read(scriptModsProvider).entries,
```

In `applyProject` (after line 85) add:

```dart
  ref.read(scriptModsProvider.notifier).loadAll(project.scripts);
```

In `newProject` (after line 97) add:

```dart
  ref.read(scriptModsProvider.notifier).clearAll();
```

- [ ] **Step 6: Run the tests**

Run: `flutter test test/scripts/` then `flutter analyze`
Expected: PASS / no new analyzer issues.

- [ ] **Step 7: Commit**

```bash
git add apps/mod-studio/lib/project/project_io.dart apps/mod-studio/lib/project/project_controller.dart apps/mod-studio/test/scripts/project_scripts_test.dart
git commit -m "feat(mod-studio): embed + restore script mods in .goremod"
```

---

## Task 8: Flutter — FFI wrappers

**Files:**
- Modify: `apps/mod-studio/lib/core/mod_ffi.dart`
- Test: `apps/mod-studio/test/scripts/script_ffi_test.dart`

- [ ] **Step 1: Write the failing test**

Create `apps/mod-studio/test/scripts/script_ffi_test.dart`:

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';

void main() {
  test('ScriptModuleInfo.fromJson', () {
    final m = ScriptModuleInfo.fromJson({'name': 'AI.Foo', 'file': 'AI/Foo.as'});
    expect(m.name, 'AI.Foo');
    expect(m.file, 'AI/Foo.as');
  });
}
```

- [ ] **Step 2: Run to verify failure**

Run: `flutter test test/scripts/script_ffi_test.dart`
Expected: FAIL — `ScriptModuleInfo` undefined.

- [ ] **Step 3: Add the wrappers + model**

In `apps/mod-studio/lib/core/mod_ffi.dart`, add methods inside `class ModFfi` (after `findGameExe`, line 73):

```dart
  /// List modules in a precompiled script cache: [{name, file}].
  Future<List<ScriptModuleInfo>> scriptListModules(String cache) async {
    final r = await _call('script_list_modules', {'cache': cache});
    final list = (r['modules'] as List?) ?? const [];
    return list
        .whereType<Map>()
        .map((m) => ScriptModuleInfo.fromJson(m.cast<String, Object?>()))
        .toList();
  }

  /// Emit recompilable .as source for one module.
  Future<String> scriptEmitModule(String cache, String module) async {
    final r = await _call('script_emit_module', {'cache': cache, 'module': module});
    return r['source'] as String;
  }

  /// Compile a staged .as into a 1-module mini-cache via the game; returns {mini_path, module}.
  Future<Map<String, Object?>> scriptCompile({
    required String gameDir,
    required String op,
    required String moduleName,
    required String relPath,
    required String asPath,
    required String workDir,
  }) =>
      _call('script_compile', {
        'game_dir': gameDir,
        'op': op,
        'module_name': moduleName,
        'rel_path': relPath,
        'as_path': asPath,
        'work_dir': workDir,
      });
```

Add the model class after `AudioSampleInfo` (end of file):

```dart
class ScriptModuleInfo {
  ScriptModuleInfo({required this.name, required this.file});
  final String name;
  final String file;
  factory ScriptModuleInfo.fromJson(Map<String, Object?> j) =>
      ScriptModuleInfo(name: j['name'] as String, file: (j['file'] as String?) ?? '');
}
```

- [ ] **Step 4: Run the test + analyze**

Run: `flutter test test/scripts/script_ffi_test.dart && flutter analyze`
Expected: PASS / clean.

- [ ] **Step 5: Commit**

```bash
git add apps/mod-studio/lib/core/mod_ffi.dart apps/mod-studio/test/scripts/script_ffi_test.dart
git commit -m "feat(mod-studio): script FFI wrappers"
```

---

## Task 9: Flutter — vanilla modules provider

**Files:**
- Create: `apps/mod-studio/lib/scripts/domain/script_modules_provider.dart`

- [ ] **Step 1: Implement the provider**

Create `apps/mod-studio/lib/scripts/domain/script_modules_provider.dart`. It loads the vanilla module list for the edit picker. The game's script cache path is `<gameRoot>/G1R/Script/PrecompiledScript_Shipping.Cache`. Resolve the game root from the configured exe via the existing `game_paths.dart` helper (confirm its name with `grep gameRootFromExe apps/mod-studio/lib/app/game_paths.dart`).

```dart
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;

import '../../app/game_paths.dart';
import '../../core/mod_ffi.dart';
import '../../core/providers.dart';

/// The vanilla precompiled-script cache path for the configured game, or null if no game is set.
String? scriptCachePath(WidgetRef ref) {
  final root = gameRootFromExe(ref.watch(gameExePathProvider));
  if (root == null) return null;
  return p.join(root, 'G1R', 'Script', 'PrecompiledScript_Shipping.Cache');
}

/// Lists vanilla modules for the "edit existing" picker. Empty list if no game / cache.
final scriptModulesProvider = FutureProvider.autoDispose<List<ScriptModuleInfo>>((ref) async {
  final root = gameRootFromExe(ref.watch(gameExePathProvider));
  if (root == null) return const [];
  final cache = p.join(root, 'G1R', 'Script', 'PrecompiledScript_Shipping.Cache');
  final ffi = ModFfi(ref.read(coreServiceProvider));
  return ffi.scriptListModules(cache);
});
```

> Confirm `gameExePathProvider` lives in `app/game_paths.dart` and `coreServiceProvider` in `core/providers.dart` (both are imported the same way by `home_page.dart`). Adjust imports if needed.

- [ ] **Step 2: Analyze**

Run: `flutter analyze lib/scripts/domain/script_modules_provider.dart`
Expected: no issues.

- [ ] **Step 3: Commit**

```bash
git add apps/mod-studio/lib/scripts/domain/script_modules_provider.dart
git commit -m "feat(mod-studio): vanilla script modules provider"
```

---

## Task 10: Flutter — Scripts tab UI

**Files:**
- Create: `apps/mod-studio/lib/scripts/ui/script_tab.dart`
- Test: `apps/mod-studio/test/scripts/script_tab_test.dart`

- [ ] **Step 1: Write the failing widget test**

Create `apps/mod-studio/test/scripts/script_tab_test.dart`:

```dart
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gore_mod/scripts/domain/script_mods_notifier.dart';
import 'package:gore_mod/scripts/ui/script_tab.dart';

void main() {
  testWidgets('shows staged script mods', (tester) async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    container.read(scriptModsProvider.notifier).setMod(
      const ScriptMod(op: ScriptOp.add, moduleName: 'MyNewModule', relPath: 'MyNewModule.as', asPath: '/x/MyNewModule.as'),
    );
    await tester.pumpWidget(UncontrolledProviderScope(
      container: container,
      child: const MaterialApp(home: Scaffold(body: ScriptTab())),
    ));
    expect(find.text('MyNewModule'), findsOneWidget);
    // An uncompiled mod surfaces a "not compiled" affordance.
    expect(find.textContaining('compile', findRichText: true), findsWidgets);
  });
}
```

- [ ] **Step 2: Run to verify failure**

Run: `flutter test test/scripts/script_tab_test.dart`
Expected: FAIL — `ScriptTab` doesn't exist.

- [ ] **Step 3: Implement the tab**

Create `apps/mod-studio/lib/scripts/ui/script_tab.dart`. Structure mirrors the other tabs: left = staged list + Add/Edit actions; right = selected-mod detail with Compile. File picker via `file_selector`. Compile/emit go through `ModFfi`. Build dirs come from `Directory.systemTemp`.

```dart
import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;

import '../../app/game_paths.dart';
import '../../core/mod_ffi.dart';
import '../../core/providers.dart';
import '../domain/script_mods_notifier.dart';
import '../domain/script_modules_provider.dart';

final _selectedModuleProvider = StateProvider<String?>((ref) => null);

class ScriptTab extends ConsumerWidget {
  const ScriptTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(scriptModsProvider);
    final selectedKey = ref.watch(_selectedModuleProvider);
    final selected = selectedKey == null ? null : state.items[selectedKey];
    final scheme = Theme.of(context).colorScheme;

    return Row(
      children: [
        SizedBox(
          width: 360,
          child: _StagedList(state: state, selectedKey: selectedKey),
        ),
        const VerticalDivider(width: 1),
        Expanded(
          child: selected == null
              ? Center(child: Text('Select or add a script mod',
                  style: TextStyle(color: scheme.onSurfaceVariant)))
              : _ModDetail(mod: selected),
        ),
      ],
    );
  }
}

class _StagedList extends ConsumerWidget {
  const _StagedList({required this.state, required this.selectedKey});
  final ScriptModsState state;
  final String? selectedKey;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final scheme = Theme.of(context).colorScheme;
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(8),
          child: Row(
            children: [
              Expanded(
                child: OutlinedButton.icon(
                  icon: const Icon(Icons.add, size: 18),
                  label: const Text('Add new'),
                  onPressed: () => _addNew(context, ref),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: OutlinedButton.icon(
                  icon: const Icon(Icons.edit_outlined, size: 18),
                  label: const Text('Edit existing'),
                  onPressed: () => _editExisting(context, ref),
                ),
              ),
            ],
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: state.count == 0
              ? Center(child: Text('No script mods staged',
                  style: TextStyle(color: scheme.onSurfaceVariant)))
              : ListView(
                  children: [
                    for (final m in state.entries)
                      ListTile(
                        selected: m.key == selectedKey,
                        leading: Icon(m.op == ScriptOp.add ? Icons.add_box_outlined : Icons.edit_note_outlined),
                        title: Text(m.moduleName, maxLines: 1, overflow: TextOverflow.ellipsis),
                        subtitle: Text(
                          m.compiled ? 'compiled' : 'not compiled — press Compile',
                          style: TextStyle(
                            color: m.compiled ? scheme.primary : scheme.error, fontSize: 12),
                        ),
                        trailing: IconButton(
                          icon: const Icon(Icons.remove_circle_outline, size: 18),
                          onPressed: () => ref.read(scriptModsProvider.notifier).remove(m.key),
                        ),
                        onTap: () => ref.read(_selectedModuleProvider.notifier).state = m.key,
                      ),
                  ],
                ),
        ),
      ],
    );
  }

  Future<void> _addNew(BuildContext context, WidgetRef ref) async {
    final file = await openFile(acceptedTypeGroups: const [
      XTypeGroup(label: 'AngelScript', extensions: ['as']),
    ]);
    if (file == null) return;
    // Derive module name + rel path from the filename; the game confirms the real module name
    // when the mod is compiled (Task 11 result updates moduleName if needed).
    final base = p.basename(file.path);
    final name = p.basenameWithoutExtension(file.path);
    final mod = ScriptMod(op: ScriptOp.add, moduleName: name, relPath: base, asPath: file.path);
    ref.read(scriptModsProvider.notifier).setMod(mod);
    ref.read(_selectedModuleProvider.notifier).state = mod.key;
  }

  Future<void> _editExisting(BuildContext context, WidgetRef ref) async {
    final modules = await ref.read(scriptModulesProvider.future);
    if (!context.mounted) return;
    if (modules.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('No vanilla modules — set the game path in Settings.')));
      return;
    }
    final picked = await showDialog<ScriptModuleInfo>(
      context: context,
      builder: (ctx) => _ModulePicker(modules: modules),
    );
    if (picked == null) return;
    // Pre-fill the editable .as by emitting the vanilla module to a temp file.
    final cache = scriptCachePath(ref);
    String asPath = '';
    if (cache != null) {
      try {
        final src = await ModFfi(ref.read(coreServiceProvider)).scriptEmitModule(cache, picked.name);
        final dir = await Directory.systemTemp.createTemp('goremod_emit_');
        final f = File(p.join(dir.path, p.basename(picked.file.isEmpty ? '${picked.name}.as' : picked.file)));
        await f.create(recursive: true);
        await f.writeAsString(src);
        asPath = f.path;
      } catch (_) {/* leave asPath empty; user can pick a file in the detail pane */}
    }
    final mod = ScriptMod(
      op: ScriptOp.edit, moduleName: picked.name,
      relPath: picked.file.isEmpty ? '${picked.name}.as' : picked.file, asPath: asPath);
    ref.read(scriptModsProvider.notifier).setMod(mod);
    ref.read(_selectedModuleProvider.notifier).state = mod.key;
  }
}

class _ModulePicker extends StatefulWidget {
  const _ModulePicker({required this.modules});
  final List<ScriptModuleInfo> modules;
  @override
  State<_ModulePicker> createState() => _ModulePickerState();
}

class _ModulePickerState extends State<_ModulePicker> {
  String _q = '';
  @override
  Widget build(BuildContext context) {
    final filtered = widget.modules
        .where((m) => m.name.toLowerCase().contains(_q.toLowerCase()))
        .take(200)
        .toList();
    return AlertDialog(
      title: const Text('Pick a module to edit'),
      content: SizedBox(
        width: 480,
        height: 420,
        child: Column(
          children: [
            TextField(
              decoration: const InputDecoration(hintText: 'Search modules', isDense: true),
              onChanged: (v) => setState(() => _q = v),
            ),
            const SizedBox(height: 8),
            Expanded(
              child: ListView(
                children: [
                  for (final m in filtered)
                    ListTile(
                      dense: true,
                      title: Text(m.name, maxLines: 1, overflow: TextOverflow.ellipsis),
                      onTap: () => Navigator.pop(context, m),
                    ),
                ],
              ),
            ),
          ],
        ),
      ),
      actions: [TextButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel'))],
    );
  }
}

class _ModDetail extends ConsumerStatefulWidget {
  const _ModDetail({required this.mod});
  final ScriptMod mod;
  @override
  ConsumerState<_ModDetail> createState() => _ModDetailState();
}

class _ModDetailState extends ConsumerState<_ModDetail> {
  bool _busy = false;
  String? _status;
  bool _error = false;

  @override
  Widget build(BuildContext context) {
    final mod = widget.mod;
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(mod.moduleName, style: Theme.of(context).textTheme.titleMedium),
          Text(mod.op == ScriptOp.add ? 'New module' : 'Edit existing module',
              style: TextStyle(color: scheme.onSurfaceVariant)),
          const SizedBox(height: 12),
          _kv('Module', mod.moduleName),
          _kv('Path', mod.relPath),
          _kv('Source', mod.asPath.isEmpty ? '(none — pick a .as)' : p.basename(mod.asPath)),
          _kv('Compiled', mod.compiled ? p.basename(mod.miniPath) : 'no'),
          const SizedBox(height: 12),
          Wrap(
            spacing: 8,
            children: [
              OutlinedButton.icon(
                icon: const Icon(Icons.file_open_outlined, size: 18),
                label: const Text('Choose .as'),
                onPressed: _busy ? null : _pickSource,
              ),
              FilledButton.icon(
                icon: const Icon(Icons.build_outlined, size: 18),
                label: const Text('Compile'),
                onPressed: (_busy || mod.asPath.isEmpty) ? null : _compile,
              ),
            ],
          ),
          if (_busy) const Padding(
            padding: EdgeInsets.symmetric(vertical: 8), child: LinearProgressIndicator()),
          if (_status != null)
            Padding(
              padding: const EdgeInsets.only(top: 8),
              child: Text(_status!,
                  style: TextStyle(color: _error ? scheme.error : scheme.onSurfaceVariant)),
            ),
        ],
      ),
    );
  }

  Widget _kv(String k, String v) => Padding(
        padding: const EdgeInsets.symmetric(vertical: 2),
        child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
          SizedBox(width: 90, child: Text(k, style: const TextStyle(fontWeight: FontWeight.w600))),
          Expanded(child: Text(v, style: const TextStyle(fontFamily: 'Consolas', fontSize: 12))),
        ]),
      );

  Future<void> _pickSource() async {
    final file = await openFile(acceptedTypeGroups: const [
      XTypeGroup(label: 'AngelScript', extensions: ['as']),
    ]);
    if (file == null) return;
    // Changing the source invalidates any prior compile.
    ref.read(scriptModsProvider.notifier)
        .setMod(widget.mod.withAsPath(file.path).withMiniPath(''));
  }

  Future<void> _compile() async {
    final gameRoot = gameRootFromExe(ref.read(gameExePathProvider));
    if (gameRoot == null) {
      setState(() { _error = true; _status = 'Set the game path in Settings to compile.'; });
      return;
    }
    setState(() { _busy = true; _error = false; _status = 'Compiling via game…'; });
    try {
      final work = await Directory.systemTemp.createTemp('goremod_as_compile_');
      final r = await ModFfi(ref.read(coreServiceProvider)).scriptCompile(
        gameDir: gameRoot,
        op: scriptOpToString(widget.mod.op),
        moduleName: widget.mod.moduleName,
        relPath: widget.mod.relPath,
        asPath: widget.mod.asPath,
        workDir: work.path,
      );
      final mini = r['mini_path'] as String;
      final resolvedName = (r['module'] as String?) ?? widget.mod.moduleName;
      // The compile may resolve the real module name (esp. for "add"); update + re-key.
      final updated = ScriptMod(
        op: widget.mod.op, moduleName: resolvedName, relPath: widget.mod.relPath,
        asPath: widget.mod.asPath, miniPath: mini);
      final notifier = ref.read(scriptModsProvider.notifier);
      if (resolvedName != widget.mod.moduleName) notifier.remove(widget.mod.key);
      notifier.setMod(updated);
      ref.read(_selectedModuleProvider.notifier).state = updated.key;
      if (mounted) setState(() => _status = 'Compiled ✓');
    } catch (e) {
      if (mounted) setState(() { _error = true; _status = '$e'; });
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }
}
```

- [ ] **Step 4: Run the widget test**

Run: `flutter test test/scripts/script_tab_test.dart && flutter analyze lib/scripts`
Expected: PASS / clean.

- [ ] **Step 5: Commit**

```bash
git add apps/mod-studio/lib/scripts/ui/script_tab.dart apps/mod-studio/test/scripts/script_tab_test.dart
git commit -m "feat(mod-studio): Scripts tab UI"
```

---

## Task 11: gore-as + gore-ffi — compile orchestration (game-gated)

> **Confirm first:** the three game-launch facts in the "One unverified area" callout. `run_regen` bakes the assumed invocation and self-verifies; correct its constants if the real run differs.

**Files:**
- Create: `crates/gore-as/src/compile.rs`
- Modify: `crates/gore-as/src/lib.rs` (add `pub mod compile;`)
- Modify: `crates/gore-ffi/src/lib.rs` (dispatch + `script_compile`)
- Test: `crates/gore-as/tests/compile_test.rs`

- [ ] **Step 1: Write the failing test (offline, injected regen)**

The orchestration is testable without the game by injecting a `run_regen` that returns a prepared cache. Create `crates/gore-as/tests/compile_test.rs`:

```rust
use std::path::{Path, PathBuf};
use gore_as::compile::{compile_module, CompileOpts, CompileError};

// A fake regen that just copies a fixture "regen" cache into place and returns it.
fn fake_regen_ok(fixture: PathBuf) -> impl Fn(&Path, &Path) -> Result<PathBuf, String> {
    move |_game_dir: &Path, _src_dir: &Path| Ok(fixture.clone())
}

#[test]
fn compile_errors_when_source_missing() {
    let tmp = std::env::temp_dir().join("gore-as-compile-missing");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let opts = CompileOpts {
        game_dir: tmp.clone(),
        op: "add".into(),
        module_name: "M".into(),
        rel_path: "M.as".into(),
        as_path: tmp.join("does-not-exist.as"),
        work_dir: tmp.clone(),
    };
    let err = compile_module(&opts, fake_regen_ok(tmp.join("regen.cache"))).unwrap_err();
    assert!(matches!(err, CompileError::Io(_)));
}
```

> A full success test needs a valid regen-cache fixture and the vanilla base; that path is exercised by the `#[ignore]` game test (Step 6) and by `gore-as`'s existing extract/remap tests. This test pins the offline error wiring.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p gore-as compile_errors_when_source_missing`
Expected: FAIL — `gore_as::compile` doesn't exist.

- [ ] **Step 3: Implement the orchestration**

Create `crates/gore-as/src/compile.rs`. The offline glue is fully specified; the single game call is the injected `run_regen` (the FFI passes the real one from Step 5).

```rust
//! Compile a staged `.as` into a 1-module mini-cache by driving the game's precompiled-data
//! generation, then extracting (add) / extract-remapping (edit) the target module.

use std::path::{Path, PathBuf};

use crate::cache::{emit_all, model, refs::RefResolver, remap, splice};

#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("io: {0}")]
    Io(String),
    #[error("regen: {0}")]
    Regen(String),
    #[error("the game did not produce a usable regen cache at {0}")]
    NoRegen(String),
    #[error("module {0:?} not found in the regen cache")]
    ModuleMissing(String),
    #[error("{0}")]
    Other(String),
}

pub struct CompileOpts {
    pub game_dir: PathBuf,
    pub op: String, // "add" | "edit"
    pub module_name: String,
    pub rel_path: String,
    pub as_path: PathBuf,
    pub work_dir: PathBuf,
}

pub struct CompileOutput {
    pub mini_path: PathBuf,
    pub module_name: String,
}

fn io(ctx: &str) -> impl FnOnce(std::io::Error) -> CompileError + '_ {
    move |e| CompileError::Io(format!("{ctx}: {e}"))
}

fn vanilla_cache(game_dir: &Path) -> PathBuf {
    let g1r = if game_dir.file_name().is_some_and(|n| n == "G1R") {
        game_dir.to_path_buf()
    } else {
        game_dir.join("G1R")
    };
    g1r.join("Script").join("PrecompiledScript_Shipping.Cache")
}

/// `run_regen(game_dir, src_dir) -> regen cache path`. Injected so the orchestration is testable
/// offline; the FFI passes [`game_run_regen`].
pub fn compile_module<R>(opts: &CompileOpts, run_regen: R) -> Result<CompileOutput, CompileError>
where
    R: Fn(&Path, &Path) -> Result<PathBuf, String>,
{
    if !opts.as_path.exists() {
        return Err(CompileError::Io(format!("source .as not found: {}", opts.as_path.display())));
    }
    let base_path = vanilla_cache(&opts.game_dir);
    let base = std::fs::read(&base_path).map_err(io("reading vanilla cache"))?;

    // 1. Emit the vanilla source tree (cache it per cache size under work_dir/tree).
    let tree = opts.work_dir.join("tree");
    let mut refs = RefResolver::build(&base).map_err(|e| CompileError::Other(format!("resolver: {e}")))?;
    let mods = model::parse_modules(&base).map_err(|e| CompileError::Other(format!("parse: {e}")))?;
    refs.set_class_hierarchy(class_hierarchy(&mods));
    emit_all::emit_all_tree(&mods, &refs, &tree)
        .map_err(|e| CompileError::Other(format!("emit tree: {e}")))?;

    // 2. Overlay the user's .as at its rel path.
    let dst = tree.join(&opts.rel_path);
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(io("mkdir overlay"))?;
    }
    std::fs::copy(&opts.as_path, &dst).map_err(io("overlay .as"))?;

    // 3. Drive the game to regenerate the precompiled cache from `tree`.
    let regen_path = run_regen(&opts.game_dir, &tree).map_err(CompileError::Regen)?;
    if !regen_path.exists() {
        return Err(CompileError::NoRegen(regen_path.display().to_string()));
    }
    let regen = std::fs::read(&regen_path).map_err(io("reading regen cache"))?;

    // 4. Extract (add) / extract+remap (edit) the target module → mini-cache.
    let mini = match opts.op.as_str() {
        "edit" => {
            let mut out = splice::extract_module(&regen, &opts.module_name)
                .map_err(|e| CompileError::Other(format!("extract: {e}")))?;
            let (remapped, _counts) = remap::remap_module_to_base(&out, &base)
                .map_err(|e| CompileError::Other(format!("remap: {e}")))?;
            out = remapped;
            out
        }
        _ => splice::extract_module(&regen, &opts.module_name)
            .map_err(|e| CompileError::Other(format!("extract: {e}")))?,
    };

    let mini_path = opts.work_dir.join("module.cache");
    std::fs::write(&mini_path, &mini).map_err(io("writing mini"))?;
    Ok(CompileOutput { mini_path, module_name: opts.module_name.clone() })
}

fn class_hierarchy(mods: &[model::Module]) -> std::collections::HashMap<String, String> {
    let mut h = std::collections::HashMap::new();
    for m in mods {
        for c in &m.classes {
            let sup = c.super_class.clone().filter(|s| !s.is_empty()).unwrap_or_default();
            h.insert(c.name.clone(), sup);
        }
    }
    h
}

/// The real game launch. **ASSUMED invocation — confirm against the proven manual run.** Places
/// the loose `.as` tree where the game reads it, launches the shipping exe with
/// `-as-generate-precompiled-data`, waits for the regen cache, and returns its path. Restores the
/// live cache so compiling never mutates the install (deploy is the only writer).
pub fn game_run_regen(game_dir: &Path, src_dir: &Path) -> Result<PathBuf, String> {
    use std::time::Duration;
    let g1r = if game_dir.file_name().is_some_and(|n| n == "G1R") {
        game_dir.to_path_buf()
    } else {
        game_dir.join("G1R")
    };
    let exe = g1r.join("Binaries").join("Win64").join("G1R-Win64-Shipping.exe");
    if !exe.exists() {
        return Err(format!("game exe not found: {}", exe.display()));
    }
    let script_dir = g1r.join("Script");
    let cache = script_dir.join("PrecompiledScript_Shipping.Cache");

    // Snapshot the live cache + any loose .as we are about to overlay, so we can restore.
    let saved_cache = std::fs::read(&cache).map_err(|e| format!("reading live cache: {e}"))?;
    let backup = cache.with_extension("Cache.gore-compile-bak");
    std::fs::write(&backup, &saved_cache).map_err(|e| format!("backing up cache: {e}"))?;

    // Copy the emitted tree into <G1R>/Script so the game compiles it.
    copy_tree(src_dir, &script_dir).map_err(|e| format!("staging .as tree: {e}"))?;

    let before = std::fs::metadata(&cache).and_then(|m| m.modified()).ok();
    let status = std::process::Command::new(&exe)
        .arg("-as-generate-precompiled-data")
        .current_dir(&g1r)
        .status()
        .map_err(|e| format!("launching game: {e}"))?;
    let _ = status; // some builds exit non-zero after generating; rely on the cache check below

    // Wait for the cache mtime to advance and its size to stabilize (max ~5 min).
    let regen_out = src_dir.join("regen.cache");
    let mut last_len = 0u64;
    let mut stable = 0;
    for _ in 0..300 {
        std::thread::sleep(Duration::from_secs(1));
        let Ok(meta) = std::fs::metadata(&cache) else { continue; };
        let advanced = match (before, meta.modified().ok()) {
            (Some(b), Some(n)) => n > b,
            _ => true,
        };
        if advanced {
            let len = meta.len();
            if len > 0 && len == last_len { stable += 1; } else { stable = 0; }
            last_len = len;
            if stable >= 2 {
                std::fs::copy(&cache, &regen_out).map_err(|e| format!("copying regen: {e}"))?;
                break;
            }
        }
    }

    // Restore the pristine live cache no matter what.
    let _ = std::fs::write(&cache, &saved_cache);
    let _ = std::fs::remove_file(&backup);

    if !regen_out.exists() {
        return Err(format!(
            "no regenerated cache produced — confirm the game compiles loose .as under {} with \
             `-as-generate-precompiled-data` (see plan §unverified)", script_dir.display()
        ));
    }
    Ok(regen_out)
}

fn copy_tree(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), &to)?;
        }
    }
    Ok(())
}
```

Add to `crates/gore-as/src/lib.rs`:

```rust
pub mod compile;
```

- [ ] **Step 4: Run the offline test**

Run: `cargo test -p gore-as compile_errors_when_source_missing`
Expected: PASS.

- [ ] **Step 5: Wire the FFI command**

In `crates/gore-ffi/src/lib.rs` dispatch (after `"script_emit_module" => ...`):

```rust
        "script_compile" => script_compile(payload),
```

Add the function:

```rust
/// `{game_dir, op, module_name, rel_path, as_path, work_dir}` → `{ok, mini_path, module}`.
fn script_compile(payload: Value) -> Value {
    let g = |k: &str| payload.get(k).and_then(Value::as_str).map(str::to_string);
    let (Some(game_dir), Some(op), Some(module_name), Some(rel_path), Some(as_path), Some(work_dir)) =
        (g("game_dir"), g("op"), g("module_name"), g("rel_path"), g("as_path"), g("work_dir"))
    else {
        return err("BAD_REQUEST", "missing one of game_dir/op/module_name/rel_path/as_path/work_dir");
    };
    let opts = gore_as::compile::CompileOpts {
        game_dir: PathBuf::from(game_dir),
        op,
        module_name,
        rel_path,
        as_path: PathBuf::from(as_path),
        work_dir: PathBuf::from(work_dir),
    };
    match gore_as::compile::compile_module(&opts, gore_as::compile::game_run_regen) {
        Ok(out) => json!({"ok": true, "mini_path": out.mini_path.display().to_string(), "module": out.module_name}),
        Err(e) => err("COMPILE_FAILED", e.to_string()),
    }
}
```

- [ ] **Step 6: Add the game-gated integration test**

Create `crates/gore-as/tests/compile_game_test.rs`:

```rust
// Real compile via the installed game. Run with:
//   GORE_TEST_GAME="C:/.../Gothic 1 Remake" cargo test -p gore-as -- --ignored real_compile_add
#[test]
#[ignore]
fn real_compile_add() {
    let Ok(game) = std::env::var("GORE_TEST_GAME") else { return; };
    let game = std::path::PathBuf::from(game);
    let work = std::env::temp_dir().join("gore-as-real-compile");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    // A trivial primitive-only module.
    let as_path = work.join("GoreHello.as");
    std::fs::write(&as_path, "int GoreHello(){ return 42; }").unwrap();
    let opts = gore_as::compile::CompileOpts {
        game_dir: game,
        op: "add".into(),
        module_name: "GoreHello".into(),
        rel_path: "GoreHello.as".into(),
        as_path,
        work_dir: work.clone(),
    };
    let out = gore_as::compile::compile_module(&opts, gore_as::compile::game_run_regen).unwrap();
    assert!(out.mini_path.exists());
    let mini = std::fs::read(&out.mini_path).unwrap();
    assert_eq!(gore_as::cache::walk_modules::module_count(&mini), 1);
}
```

- [ ] **Step 7: Run the suites**

Run: `cargo test -p gore-as && cargo test -p gore-ffi && cargo build`
Expected: PASS (the `#[ignore]` game tests are skipped).

- [ ] **Step 8: Commit**

```bash
git add crates/gore-as/src/compile.rs crates/gore-as/src/lib.rs crates/gore-as/tests/compile_test.rs crates/gore-as/tests/compile_game_test.rs crates/gore-ffi/src/lib.rs
git commit -m "feat: AngelScript compile-via-game orchestration + script_compile FFI"
```

---

## Task 12: Flutter — hub wiring (tab, changes panel, build dialog)

**Files:**
- Modify: `apps/mod-studio/lib/home_page.dart`
- Modify: `apps/mod-studio/lib/editor/ui/overrides_panel.dart`
- Modify: `apps/mod-studio/lib/export/ui/build_deploy_dialog.dart`
- Test: `apps/mod-studio/test/scripts/build_dialog_scripts_test.dart`

- [ ] **Step 1: Add the tab to home_page**

In `apps/mod-studio/lib/home_page.dart`:

Imports (after the textures imports, line 29):

```dart
import 'scripts/domain/script_mods_notifier.dart';
import 'scripts/ui/script_tab.dart';
```

Dirty flag (line 171–174) — add a clause:

```dart
        ref.watch(scriptModsProvider).count > 0 ||
```

`DefaultTabController(length: 6` → `length: 7` (line 244).

Add a `Tab` to the `TabBar.tabs` list after the Textures tab (after line 270):

```dart
                        const Tab(
                          icon: Icon(Icons.code),
                          text: 'AngelScript',
                        ),
```

Add the view to `TabBarView.children` after `const TextureTab(),` (line 348):

```dart
                  // AngelScript: stage .as mods, compile, splice.
                  const ScriptTab(),
```

- [ ] **Step 2: Add script rows to the Changes panel**

In `apps/mod-studio/lib/editor/ui/overrides_panel.dart`:

Import (after the textures import, line 7):

```dart
import '../../scripts/domain/script_mods_notifier.dart';
```

In `build` (after line 27) add:

```dart
    final scriptState   = ref.watch(scriptModsProvider);
    final scripts       = ref.read(scriptModsProvider.notifier);
```

After `final textureEntries = ...` (line 40):

```dart
    final scriptEntries = scriptState.entries;
```

Update `total` (line 42):

```dart
    final total   = overridesState.count + locState.entryCount + audioState.count + textureState.count + scriptState.count;
```

Add to the clear-all `onPressed` (after `textures.clearAll();` line 68):

```dart
                    scripts.clearAll();
```

Add a section to the `ListView` (after the textures block, line 106):

```dart
                    if (scriptEntries.isNotEmpty) ...[
                      const _SectionHeader('AngelScript'),
                      for (final entry in scriptEntries)
                        _ScriptRow(entry: entry, notifier: scripts),
                    ],
```

Add the row widget at the end of the file (after `_TextureRow`):

```dart
class _ScriptRow extends StatelessWidget {
  const _ScriptRow({required this.entry, required this.notifier});

  final ScriptMod entry;
  final ScriptModsNotifier notifier;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '${entry.op == ScriptOp.add ? 'add' : 'edit'}  ·  ${entry.moduleName}',
                  style: const TextStyle(fontFamily: 'Consolas', fontSize: 12),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                Text(
                  entry.compiled ? 'compiled' : 'not compiled',
                  style: TextStyle(
                    fontSize: 12,
                    color: entry.compiled ? scheme.primary : scheme.error,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ],
            ),
          ),
          IconButton(
            icon: const Icon(Icons.remove_circle_outline, size: 18),
            tooltip: AppLocalizations.of(context).removeOverride,
            onPressed: () => notifier.remove(entry.key),
          ),
        ],
      ),
    );
  }
}
```

- [ ] **Step 3: Write the failing build-dialog test**

Create `apps/mod-studio/test/scripts/build_dialog_scripts_test.dart`:

```dart
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gore_mod/export/ui/build_deploy_dialog.dart';
import 'package:gore_mod/scripts/domain/script_mods_notifier.dart';

void main() {
  testWidgets('build dialog counts script mods', (tester) async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    container.read(scriptModsProvider.notifier).setMod(
      const ScriptMod(op: ScriptOp.add, moduleName: 'M', relPath: 'M.as', asPath: 'a', miniPath: 'm'),
    );
    await tester.pumpWidget(UncontrolledProviderScope(
      container: container,
      child: const MaterialApp(home: Scaffold(body: BuildDeployDialog())),
    ));
    expect(find.textContaining('1 script'), findsOneWidget);
  });
}
```

- [ ] **Step 4: Run to verify failure**

Run: `flutter test test/scripts/build_dialog_scripts_test.dart`
Expected: FAIL — no script line / count.

- [ ] **Step 5: Update the build dialog**

In `apps/mod-studio/lib/export/ui/build_deploy_dialog.dart`:

Import (after the textures import, line 15):

```dart
import '../../scripts/domain/script_mods_notifier.dart';
```

In `build` (after line 97):

```dart
    final scripts = ref.watch(scriptModsProvider).count;
```

Update `hasContent` (line 100):

```dart
    final hasContent = overrides + locEdits + audio + textures + scripts > 0;
```

Add a contents line (after line 134):

```dart
            Text('• $scripts script mod(s)'),
```

Optionally, warn if any staged script isn't compiled (add after the contents lines, before `const SizedBox(height: 12)` at line 135):

```dart
            if (ref.watch(scriptModsProvider).entries.any((s) => !s.compiled))
              Text(
                'Some script mods are not compiled — compile them in the AngelScript tab first.',
                style: TextStyle(color: theme.colorScheme.error),
              ),
```

- [ ] **Step 6: Run the tests + analyze + full suite**

Run: `flutter test && flutter analyze`
Expected: PASS / clean.

- [ ] **Step 7: Commit**

```bash
git add apps/mod-studio/lib/home_page.dart apps/mod-studio/lib/editor/ui/overrides_panel.dart apps/mod-studio/lib/export/ui/build_deploy_dialog.dart apps/mod-studio/test/scripts/build_dialog_scripts_test.dart
git commit -m "feat(mod-studio): wire AngelScript tab into home, changes, build dialog"
```

---

## Final verification

- [ ] **Rust:** `cargo test` (workspace) — all green; `cargo build` clean.
- [ ] **Flutter:** from `apps/mod-studio`, `flutter analyze` clean and `flutter test` green.
- [ ] **Manual (game required):** set the game path in Settings → AngelScript tab → Add new, pick a trivial primitive-only `.as` → Compile (confirm Task 11 game-launch facts here) → Build/Deploy → launch game and observe → Undeploy restores the vanilla cache from `*.gore-bak`.
- [ ] Update the spec's "open detail" once the real `run_regen` invocation is confirmed.
