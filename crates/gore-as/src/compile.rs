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
    /// Pristine base cache to emit/remap against. When `Some`, these bytes are the base (skip the
    /// disk read) — the FFI passes gore-mod's drift-aware `pristine_script_cache` so the compile
    /// base matches the bytes deploy will splice against. When `None`, fall back to the on-disk
    /// `*.gore-bak`-or-live read (standalone/CLI/offline). NOTE: `game_run_regen` still uses the
    /// LIVE cache for its own backup/restore — only this emit/remap base is overridden.
    pub base_override: Option<Vec<u8>>,
}

#[derive(Debug)]
pub struct CompileOutput {
    pub mini_path: PathBuf,
    pub module_name: String,
}

fn io(ctx: &str) -> impl FnOnce(std::io::Error) -> CompileError {
    let ctx = ctx.to_string();
    move |e| CompileError::Io(format!("{ctx}: {e}"))
}

/// The `G1R` game directory: `game_dir` itself if it already ends in `G1R`, else `game_dir/G1R`.
fn g1r_dir(game_dir: &Path) -> PathBuf {
    if game_dir.file_name().is_some_and(|n| n == "G1R") {
        game_dir.to_path_buf()
    } else {
        game_dir.join("G1R")
    }
}

fn vanilla_cache(game_dir: &Path) -> PathBuf {
    g1r_dir(game_dir).join("Script").join("PrecompiledScript_Shipping.Cache")
}

/// The deploy backup path for a live cache: the live path with `.gore-bak` APPENDED to the full
/// filename (so `…Shipping.Cache` -> `…Shipping.Cache.gore-bak`). Mirrors gore-mod's `bak_path`;
/// built via `OsString::push` (NOT `with_extension`, which would clobber the `.Cache` extension).
fn deploy_bak_path(live: &Path) -> PathBuf {
    let mut s = live.as_os_str().to_os_string();
    s.push(".gore-bak");
    PathBuf::from(s)
}

/// A safe relative path inside the staged tree: non-empty, not absolute, every component a normal
/// name (no `..`, no root/prefix), no control characters — so it can't escape the tree (and, since
/// the same tree is later copied into the game's `Script/`, can't escape that either). Mirrors
/// gore-mod's `is_safe_rel_path`.
fn is_safe_rel_path(p: &str) -> bool {
    use std::path::Component;
    if p.is_empty() || p.chars().any(char::is_control) {
        return false;
    }
    let path = Path::new(p);
    if path.is_absolute() {
        return false;
    }
    let mut any = false;
    for c in path.components() {
        match c {
            Component::Normal(_) => any = true,
            _ => return false,
        }
    }
    any
}

/// `run_regen(game_dir, src_dir) -> regen cache path`. Injected so the orchestration is testable
/// offline; the FFI passes [`game_run_regen`].
pub fn compile_module<R>(opts: &CompileOpts, run_regen: R) -> Result<CompileOutput, CompileError>
where
    R: Fn(&Path, &Path) -> Result<PathBuf, String>,
{
    if opts.op != "add" && opts.op != "edit" {
        return Err(CompileError::Other(format!(
            "invalid script op {:?} for module {:?} (want \"add\" or \"edit\")",
            opts.op, opts.module_name
        )));
    }
    if !opts.as_path.exists() {
        return Err(CompileError::Io(format!("source .as not found: {}", opts.as_path.display())));
    }
    // Validate the untrusted overlay rel_path BEFORE the heavy work: it's joined onto the staged
    // tree (and that tree is later copied into the game's Script/), so an absolute or `..` path
    // could escape both. Reject it up front.
    if !is_safe_rel_path(&opts.rel_path) {
        return Err(CompileError::Other(format!("unsafe script rel_path: {:?}", opts.rel_path)));
    }
    // The PRISTINE base cache to emit/remap against. Prefer the caller-supplied `base_override`
    // (the FFI passes gore-mod's drift-aware `pristine_script_cache`, so the base matches exactly
    // what deploy will splice against, even after a game update made the `*.gore-bak` stale).
    // Without an override, fall back to the on-disk read: if a mod is already deployed, the live
    // cache is the spliced (modded) one and gore-mod's deploy backup `…Cache.gore-bak` holds the
    // true pristine bytes, so prefer the backup when present. `base_path` is the on-disk cache
    // location used only to locate `Binds.Cache` next to it — independent of which bytes `base` holds.
    let live_cache = vanilla_cache(&opts.game_dir);
    let bak = deploy_bak_path(&live_cache);
    let base_path = if bak.exists() { bak } else { live_cache };
    let base = match &opts.base_override {
        Some(bytes) => bytes.clone(),
        None => std::fs::read(&base_path).map_err(io("reading vanilla cache"))?,
    };

    // 1. Emit the vanilla source tree (cache it per cache size under work_dir/tree).
    let tree = opts.work_dir.join("tree");
    let mut refs = RefResolver::build(&base).map_err(|e| CompileError::Other(format!("resolver: {e}")))?;
    let mods = model::parse_modules(&base).map_err(|e| CompileError::Other(format!("parse: {e}")))?;
    refs.set_class_hierarchy(class_hierarchy(&mods));
    // Load native-call arities (Binds.Cache next to the vanilla cache) so emitted source has the
    // right native-call shapes and recompiles — mirrors `AsCmd::EmitAll`. Absent => no fallback.
    if let Some(api) = native_api(&base_path) {
        refs.set_native_api(api);
    }
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

    // For "add", the game names the new module from its ScriptRelativeFilename, which may differ
    // from `opts.module_name`. Resolve the real name = the single module present in the regen but
    // not in the base; fall back to `opts.module_name` if that diff isn't exactly one (the existing
    // extract call then surfaces any mismatch naturally). "edit" keeps `opts.module_name`.
    let target = if opts.op == "add" {
        match (
            crate::cache::walk_modules::module_names(&base),
            crate::cache::walk_modules::module_names(&regen),
        ) {
            (Ok(base_names), Ok(regen_names)) => {
                use std::collections::HashSet;
                let base_set: HashSet<&str> = base_names.iter().map(String::as_str).collect();
                let mut added = regen_names.iter().filter(|n| !base_set.contains(n.as_str()));
                match (added.next(), added.next()) {
                    (Some(only), None) => only.clone(),
                    _ => opts.module_name.clone(),
                }
            }
            _ => opts.module_name.clone(),
        }
    } else {
        opts.module_name.clone()
    };

    // 4. Extract + remap the target module → an empty-tail mini-cache, for BOTH ops. Remapping to
    //    the vanilla base drops the regen cache's full global tail tables (whose pointer/id keys
    //    differ from the base), so deploy's `splice_auto` (add) takes the case-b append path with
    //    no duplicate/stale global rows, and the module's refs resolve against the base. (A
    //    primitive-only add is already empty-tail, so this is a no-op for that path; it only fixes
    //    case-a / native-ref adds.) Deploy still differs by op — gore-mod uses `splice_auto` for
    //    add and `replace_module` for edit — but the mini is now the same minimal shape for both.
    let mini = {
        let out = splice::extract_module(&regen, &target)
            .map_err(|e| CompileError::Other(format!("extract: {e}")))?;
        remap::remap_module_to_base(&out, &base)
            .map_err(|e| CompileError::Other(format!("remap: {e}")))?
            .0
    };

    let mini_path = opts.work_dir.join("module.cache");
    std::fs::write(&mini_path, &mini).map_err(io("writing mini"))?;
    Ok(CompileOutput { mini_path, module_name: target })
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

/// Load native arities from the `GORE_AS_BINDS` env path if set, else a `Binds.Cache` sitting next
/// to `cache_file`, if present. Mirrors `as_cache.rs::load_native_api` / gore-ffi's `as_native_api`
/// so a dev who sets `GORE_AS_BINDS` for the CLI gets the same arities here (no emit/recompile
/// divergence). Quiet by design (library helper — no logging). Absent/unparsable => None.
fn native_api(cache_file: &Path) -> Option<crate::cache::binds::NativeApi> {
    let path = match std::env::var_os("GORE_AS_BINDS") {
        Some(p) => std::path::PathBuf::from(p),
        None => cache_file.parent()?.join("Binds.Cache"),
    };
    if !path.exists() {
        return None;
    }
    crate::cache::binds::NativeApi::load(&path)
}

/// The real game launch. **ASSUMED invocation — confirm against the proven manual run.** Places
/// the loose `.as` tree where the game reads it, launches the shipping exe with
/// `-as-generate-precompiled-data`, waits for the regen cache, and returns its path.
///
/// Compiling NEVER mutates the install (deploy is the only writer): on EVERY exit path — success,
/// timeout, no-regen, or a launch error — this restores `PrecompiledScript_Shipping.Cache` to its
/// pre-call bytes, deletes the `.gore-compile-bak`, and removes every file this call copied into
/// `<G1R>/Script` plus any now-empty directories the copy created. Pre-existing files in `Script/`
/// are left untouched.
pub fn game_run_regen(game_dir: &Path, src_dir: &Path) -> Result<PathBuf, String> {
    let g1r = g1r_dir(game_dir);
    let exe = g1r.join("Binaries").join("Win64").join("G1R-Win64-Shipping.exe");
    if !exe.exists() {
        return Err(format!("game exe not found: {}", exe.display()));
    }
    let script_dir = g1r.join("Script");
    let cache = script_dir.join("PrecompiledScript_Shipping.Cache");

    // Snapshot the live cache so we can restore it, and back it up to disk too.
    let saved_cache = std::fs::read(&cache).map_err(|e| format!("reading live cache: {e}"))?;
    let backup = cache.with_extension("Cache.gore-compile-bak");
    std::fs::write(&backup, &saved_cache).map_err(|e| format!("backing up cache: {e}"))?;

    // Everything that touches the install runs inside this closure; cleanup below runs
    // UNCONDITIONALLY afterwards (Rust has no try/finally). `written` records every destination we
    // copy into Script/ along with its prior bytes: `None` = the file did NOT exist before (delete
    // on cleanup), `Some(bytes)` = it pre-existed and we overwrote it (RESTORE those bytes), so a
    // user's own loose script that collides with the emitted tree is never destroyed.
    let mut written: Vec<(PathBuf, Option<Vec<u8>>)> = Vec::new();
    let regen_out = src_dir.join("regen.cache");
    // Drop any stale regen from a prior/failed/timed-out run in this work dir, so a later
    // `regen_out.exists()` check means THIS run actually produced a fresh cache (no false success).
    let _ = std::fs::remove_file(&regen_out);
    let result = (|| -> Result<PathBuf, String> {
        // Copy the emitted tree into <G1R>/Script so the game compiles it.
        copy_tree(src_dir, &script_dir, &mut written).map_err(|e| format!("staging .as tree: {e}"))?;

        let before = std::fs::metadata(&cache).and_then(|m| m.modified()).ok();
        let status = std::process::Command::new(&exe)
            .arg("-as-generate-precompiled-data")
            .current_dir(&g1r)
            .status()
            .map_err(|e| format!("launching game: {e}"))?;
        let _ = status; // some builds exit non-zero after generating; rely on the cache check below

        // Wait for the cache mtime to advance and its size to stabilize (max ~5 min).
        let mut last_len = 0u64;
        let mut stable = 0;
        for _ in 0..300 {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let Ok(meta) = std::fs::metadata(&cache) else { continue; };
            let advanced = match (before, meta.modified().ok()) {
                (Some(b), Some(n)) => n > b,
                // Unknown/unreadable mtime: can't prove the game regenerated — do NOT treat as
                // advanced (else size-stability alone could accept an unchanged, stale cache).
                _ => false,
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

        if !regen_out.exists() {
            return Err(format!(
                "no regenerated cache produced — confirm the game compiles loose .as under {} with \
                 `-as-generate-precompiled-data` (see plan §unverified)", script_dir.display()
            ));
        }
        Ok(regen_out.clone())
    })();

    // UNCONDITIONAL cleanup: restore the pristine cache, drop the backup, and undo every file we
    // copied into Script/ (deleting ones we created, restoring ones we overwrote) plus any now-empty
    // dirs we created — leaving the install as we found it. Failures here are CAPTURED (not ignored)
    // so a successful compile that left the install dirty is reported as an error below.
    let restore_err = std::fs::write(&cache, &saved_cache).err();
    let _ = std::fs::remove_file(&backup); // best-effort: a leftover backup is harmless
    let cleanup_res = restore_or_remove(&written, &script_dir);

    match result {
        Ok(p) => {
            // The live cache failing to restore is the worst case — the install is left on
            // regenerated bytes — so surface it first, ahead of a stray staged-.as tree.
            if let Some(e) = restore_err {
                return Err(format!(
                    "compiled, but FAILED to restore the live script cache ({e}); the game install \
                     may be left on regenerated bytes — restore PrecompiledScript_Shipping.Cache \
                     from backup"
                ));
            }
            if let Err(e) = cleanup_res {
                return Err(format!(
                    "compiled, but failed to clean staged scripts from the install: {e}"
                ));
            }
            Ok(p)
        }
        // Inner failure: report it, but if cleanup ALSO failed the install is left dirty — that must
        // not be hidden behind a benign-looking compile error, so append the restore/cleanup failure.
        Err(e) => {
            if let Some(re) = restore_err {
                return Err(format!(
                    "{e}; ADDITIONALLY failed to restore the live script cache ({re}) — the install \
                     may be left on regenerated bytes, restore PrecompiledScript_Shipping.Cache from backup"
                ));
            }
            if let Err(ce) = cleanup_res {
                return Err(format!("{e}; additionally failed to clean staged scripts: {ce}"));
            }
            Err(e)
        }
    }
}

/// Options for [`precompile`] — driving the game's own `-as-generate-precompiled-data` step as a
/// standalone compiler that handles all the file juggling (backup, staging, output, restore).
pub struct PrecompileOpts {
    /// Game install root (the folder containing `G1R/`, or the `G1R` dir itself).
    pub game_dir: PathBuf,
    /// Source `.as` tree to stage under `Script/` before compiling. `None` recompiles whatever
    /// `.as` are already installed there.
    pub src: Option<PathBuf>,
    /// Where to write the compiled cache. `Some` writes it there and RESTORES the install to its
    /// pre-call state (the live cache and any staged sources are put back → install untouched).
    /// `None` installs the fresh cache in place under `Script/`.
    pub out: Option<PathBuf>,
    /// When installing in place (`out` is `None`), back up the previous cache to `<cache>.gore-bak`
    /// first — unless one already exists, so the earliest (pristine) backup is preserved.
    pub backup: bool,
}

/// Compile `.as` into a precompiled script cache by driving the game, handling backup, staging,
/// output placement and restore internally. Returns the path of the resulting cache (`out` if set,
/// else the in-place `Script/PrecompiledScript_Shipping.Cache`).
pub fn precompile(opts: &PrecompileOpts) -> Result<PathBuf, String> {
    precompile_with(opts, real_generate)
}

/// The first loose `.as` file found anywhere under `dir` (recursively), or `None`. Used to reject a
/// dirty Script/ before staging a SRC tree, so the game never compiles leftover scripts alongside it.
fn first_loose_script(dir: &Path) -> Option<PathBuf> {
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            if let Some(found) = first_loose_script(&path) {
                return Some(found);
            }
        } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("as")) {
            return Some(path);
        }
    }
    None
}

/// Resolve `out` to an absolute path for the "output must be outside Script/" containment check:
/// relative paths are taken relative to `cwd` (so a relative `-o` can't slip past the check), then
/// canonicalized as far as the path exists — the file itself, else its existing parent joined with
/// the filename, else the lexical absolute path. Extracted so the guard is testable without mutating
/// the process cwd.
fn resolve_out_real(out: &Path, cwd: &Path) -> PathBuf {
    let abs = if out.is_absolute() { out.to_path_buf() } else { cwd.join(out) };
    abs.canonicalize().unwrap_or_else(|_| match (abs.parent(), abs.file_name()) {
        (Some(parent), Some(name)) => {
            parent.canonicalize().map(|p| p.join(name)).unwrap_or_else(|_| abs.clone())
        }
        _ => abs.clone(),
    })
}

/// Testable core of [`precompile`]. `generate(exe, g1r, cache)` must make the game (re)write
/// `cache` and return its new bytes; the real impl [`real_generate`] launches the game and polls,
/// tests inject a stub so the file orchestration can be exercised offline.
fn precompile_with<G>(opts: &PrecompileOpts, generate: G) -> Result<PathBuf, String>
where
    G: FnOnce(&Path, &Path, &Path) -> Result<Vec<u8>, String>,
{
    let g1r = g1r_dir(&opts.game_dir);
    let exe = g1r.join("Binaries").join("Win64").join("G1R-Win64-Shipping.exe");
    if !exe.exists() {
        return Err(format!("game exe not found: {}", exe.display()));
    }
    let script_dir = g1r.join("Script");
    let cache = script_dir.join("PrecompiledScript_Shipping.Cache");

    // Reject a source tree that contains (or IS) the Script destination: `copy_tree` would copy the
    // install into its own subtree, recursing `Script/…/Script` until the path or disk blows up while
    // polluting the live install. (Mirrors deploy_shared's self-copy guard.)
    if let Some(src) = &opts.src {
        let src_real = src.canonicalize().unwrap_or_else(|_| src.clone());
        let dst_real = script_dir.canonicalize().unwrap_or_else(|_| script_dir.clone());
        if dst_real == src_real || dst_real.starts_with(&src_real) {
            return Err(format!(
                "source {} contains the game's Script/ directory ({}); point the source at your \
                 emitted .as tree, not the game root",
                src.display(),
                script_dir.display()
            ));
        }
    }

    // The output must live OUTSIDE the game's Script/ directory. Writing it inside would pollute the
    // install (breaking out-mode's pristine-install contract); worse, if it lands on the live cache
    // or a file staged from SRC, the later restore/cleanup would overwrite or delete the artifact we
    // just wrote while still returning Ok. Reject any output under Script/ — to update the live
    // cache, omit `-o` (in-place mode).
    if let Some(out) = &opts.out {
        let script_real = script_dir.canonicalize().unwrap_or_else(|_| script_dir.clone());
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        if resolve_out_real(out, &cwd).starts_with(&script_real) {
            return Err(format!(
                "output {} is inside the game's Script/ directory ({}); write the compiled cache \
                 elsewhere, or omit -o to install in place",
                out.display(),
                script_dir.display()
            ));
        }
    }

    // When compiling a specific SRC tree, the game must see ONLY that tree. The game compiles EVERY
    // loose `.as` under Script/, so pre-existing loose scripts there (from a prior/interrupted
    // compile or manual staging) would be mixed into the cache alongside SRC. Refuse rather than
    // silently produce a mixed/stale cache; omit the source to compile Script/ as-is (no-src mode).
    if opts.src.is_some() {
        if let Some(stray) = first_loose_script(&script_dir) {
            return Err(format!(
                "the game's Script/ directory ({}) already contains a loose script ({}); the game \
                 would compile it alongside your source. Remove loose .as files there first, or omit \
                 the source argument to compile Script/ as-is",
                script_dir.display(),
                stray.display()
            ));
        }
    }

    // Snapshot the live cache so we can RESTORE it (out-mode) or back it up (in-place).
    let saved = std::fs::read(&cache)
        .map_err(|e| format!("reading live cache {}: {e}", cache.display()))?;

    // In-place backup BEFORE regenerating. Never clobber an existing `.gore-bak`: that is the true
    // pristine (gore-mod's deploy backup, or an earlier compile's), so preserving it keeps a path
    // back to vanilla across repeated compiles.
    if opts.out.is_none() && opts.backup {
        let bak = deploy_bak_path(&cache);
        if !bak.exists() {
            std::fs::write(&bak, &saved).map_err(|e| format!("backing up cache: {e}"))?;
        }
    }

    // Stage the source tree into Script/, recording overwrites so cleanup can restore them.
    let mut staged: Vec<(PathBuf, Option<Vec<u8>>)> = Vec::new();
    let staged_ok = match &opts.src {
        Some(src) => {
            copy_tree(src, &script_dir, &mut staged).map_err(|e| format!("staging src tree: {e}"))
        }
        None => Ok(()),
    };

    // Generate only if staging succeeded.
    let result = staged_ok.and_then(|()| generate(&exe, &g1r, &cache));

    match result {
        Ok(regen) => {
            if let Some(out) = &opts.out {
                // Pristine mode: write the artifact, then restore the install (live cache + staged
                // files) so nothing about the game changes. Attempt EVERY step and aggregate all
                // failures — a write error must not hide a failed rollback, which would leave the
                // install modified without telling the user.
                let write_err = std::fs::write(out, &regen)
                    .map_err(|e| format!("writing output {}: {e}", out.display()))
                    .err();
                let restore_err = std::fs::write(&cache, &saved)
                    .map_err(|e| {
                        format!(
                            "failed to restore the live cache ({e}) — it may be left on regenerated \
                             bytes; restore PrecompiledScript_Shipping.Cache from backup"
                        )
                    })
                    .err();
                let cleanup_err = restore_or_remove(&staged, &script_dir)
                    .map_err(|e| format!("failed to clean staged sources: {e}"))
                    .err();
                let rollback: Vec<String> = restore_err.into_iter().chain(cleanup_err).collect();
                match (write_err, rollback.is_empty()) {
                    (None, true) => Ok(out.clone()),
                    (None, false) => {
                        Err(format!("compiled to {}, but {}", out.display(), rollback.join("; ")))
                    }
                    (Some(w), true) => Err(w),
                    (Some(w), false) => Err(format!("{w}; additionally {}", rollback.join("; "))),
                }
            } else {
                // In-place: the game wrote the fresh cache. Clean up the staged `.as`, but NEVER let
                // cleanup touch the cache itself: if the staged source tree happened to include a
                // file at the cache path, restoring its pre-compile bytes here would silently clobber
                // the freshly compiled cache while still reporting success. Drop that entry first so
                // the new cache survives.
                staged.retain(|(p, _)| p != &cache);
                if let Err(e) = restore_or_remove(&staged, &script_dir) {
                    return Err(format!("compiled in place, but failed to clean staged sources: {e}"));
                }
                Ok(cache.clone())
            }
        }
        // Failure: roll back (restore the possibly-half-written cache, remove staged sources) so the
        // install is left exactly as we found it. If the rollback ITSELF fails (cache or staged files
        // locked/unwritable), surface that alongside the original error — a silent rollback failure
        // could leave the cache or staged `.as` modified without telling the user.
        Err(e) => {
            let restore_err = std::fs::write(&cache, &saved)
                .map_err(|re| format!("restoring live cache: {re}"))
                .err();
            let cleanup_err = restore_or_remove(&staged, &script_dir).err();
            let mut msg = e;
            if let Some(re) = restore_err {
                msg = format!(
                    "{msg}; ADDITIONALLY failed to roll back the install ({re}) — the cache may be \
                     left on regenerated bytes; restore PrecompiledScript_Shipping.Cache from backup"
                );
            }
            if let Some(ce) = cleanup_err {
                msg = format!("{msg}; additionally failed to clean staged sources: {ce}");
            }
            Err(msg)
        }
    }
}

/// Launch the game with `-as-generate-precompiled-data`, wait for `cache` to be (re)written
/// (mtime advances + size stabilizes; ~5 min cap), and return its bytes. Mirrors the wait in
/// [`game_run_regen`].
fn real_generate(exe: &Path, g1r: &Path, cache: &Path) -> Result<Vec<u8>, String> {
    let before = std::fs::metadata(cache).and_then(|m| m.modified()).ok();
    let status = std::process::Command::new(exe)
        .arg("-as-generate-precompiled-data")
        .current_dir(g1r)
        .status()
        .map_err(|e| format!("launching game: {e}"))?;
    // Some shipping builds exit non-zero after generating; the cache check below is authoritative.
    let _ = status;

    let mut last_len = 0u64;
    let mut stable = 0;
    for _ in 0..300 {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let Ok(meta) = std::fs::metadata(cache) else {
            continue;
        };
        let advanced = match (before, meta.modified().ok()) {
            (Some(b), Some(n)) => n > b,
            _ => false,
        };
        if advanced {
            let len = meta.len();
            if len > 0 && len == last_len {
                stable += 1;
            } else {
                stable = 0;
            }
            last_len = len;
            if stable >= 2 {
                return std::fs::read(cache).map_err(|e| format!("reading regen cache: {e}"));
            }
        }
    }
    Err(format!(
        "no regenerated cache produced — confirm the game compiles loose .as under {} with \
         `-as-generate-precompiled-data`",
        cache.parent().unwrap_or(cache).display()
    ))
}

/// Recursively copy `src` into `dst`, recording every destination FILE path written into `out`
/// together with its PRIOR bytes (`None` if it didn't exist, `Some(bytes)` if the copy overwrote a
/// pre-existing file) — so the caller can delete what it created and RESTORE what it overwrote.
/// Directories created are not recorded individually — empty ones are pruned bottom-up by
/// [`restore_or_remove`].
fn copy_tree(src: &Path, dst: &Path, out: &mut Vec<(PathBuf, Option<Vec<u8>>)>) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &to, out)?;
        } else {
            // Capture the pre-existing bytes (if any) BEFORE overwriting, so cleanup can restore a
            // user's own loose script that happens to share this path with the emitted tree.
            let prior = std::fs::read(&to).ok();
            std::fs::copy(entry.path(), &to)?;
            out.push((to, prior));
        }
    }
    Ok(())
}

/// Undo a [`copy_tree`]: for each recorded destination, RESTORE its original bytes if it pre-existed
/// (`Some`) or DELETE it if this copy created it (`None`); then remove any directories that became
/// empty as a result, walking UP toward (but never past) `root`. `root` itself is left in place.
/// A dir still holding a restored pre-existing (or other) file stays non-empty and survives, so
/// pre-existing content is never lost.
///
/// Attempts ALL files (and dirs) even if some fail — cleanup must be maximal — but AGGREGATES any
/// file restore/delete failures into the returned `Err` so a caller can report a polluted install.
/// Directory-prune failures are NOT errors: a dir staying non-empty (e.g. it holds a restored file)
/// is the expected, correct outcome, so empty-dir removal stays best-effort.
fn restore_or_remove(written: &[(PathBuf, Option<Vec<u8>>)], root: &Path) -> Result<(), String> {
    use std::collections::BTreeSet;
    // Restore-or-remove the files first, collecting (not short-circuiting on) failures so every
    // file is attempted before we report.
    let mut errs: Vec<String> = Vec::new();
    for (f, prior) in written {
        match prior {
            Some(bytes) => {
                if let Err(e) = std::fs::write(f, bytes) {
                    errs.push(format!("restore {}: {e}", f.display()));
                }
            }
            None => {
                if let Err(e) = std::fs::remove_file(f) {
                    errs.push(format!("delete {}: {e}", f.display()));
                }
            }
        }
    }
    // Collect candidate parent dirs (deepest first via reverse-sorted full paths), bounded to
    // strict descendants of `root`, then try to remove each empty one bottom-up. Removing a child
    // can empty its parent, so seed parents transitively up to `root`.
    let mut dirs: BTreeSet<PathBuf> = BTreeSet::new();
    for (f, _) in written {
        let mut p = f.parent();
        while let Some(dir) = p {
            if dir == root || !dir.starts_with(root) {
                break;
            }
            dirs.insert(dir.to_path_buf());
            p = dir.parent();
        }
    }
    // Deepest paths sort last; remove in reverse so children go before parents.
    for dir in dirs.iter().rev() {
        // `remove_dir` only succeeds on an empty dir — a restored pre-existing file keeps it alive,
        // so a failure here is expected and NOT aggregated.
        let _ = std::fs::remove_dir(dir);
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g1r_dir_appends_or_keeps() {
        assert_eq!(g1r_dir(Path::new("games/Gothic")), PathBuf::from("games/Gothic/G1R"));
        assert_eq!(g1r_dir(Path::new("games/Gothic/G1R")), PathBuf::from("games/Gothic/G1R"));
    }

    #[test]
    fn precompile_errors_when_exe_missing() {
        // No shipping exe: the guard fires and the generator is NEVER invoked.
        let dir = std::env::temp_dir().join("gore-as-no-exe-xyz");
        let opts = PrecompileOpts { game_dir: dir, src: None, out: None, backup: true };
        let err = precompile_with(&opts, |_, _, _| panic!("must not launch")).unwrap_err();
        assert!(err.contains("game exe not found"), "got: {err}");
    }

    /// A fake install under `base`: a stub shipping exe (so the exists()-guard passes) and a live
    /// cache holding `OLD`. Returns (game_dir, cache_path).
    fn fake_install(base: &Path) -> (PathBuf, PathBuf) {
        let win64 = base.join("G1R").join("Binaries").join("Win64");
        std::fs::create_dir_all(&win64).unwrap();
        std::fs::write(win64.join("G1R-Win64-Shipping.exe"), b"stub").unwrap();
        let script = base.join("G1R").join("Script");
        std::fs::create_dir_all(&script).unwrap();
        let cache = script.join("PrecompiledScript_Shipping.Cache");
        std::fs::write(&cache, b"OLD").unwrap();
        (base.to_path_buf(), cache)
    }

    /// Stub generator: emulate the game overwriting the cache with `NEW`, and return those bytes.
    fn gen_new(_exe: &Path, _g1r: &Path, cache: &Path) -> Result<Vec<u8>, String> {
        std::fs::write(cache, b"NEW").map_err(|e| e.to_string())?;
        Ok(b"NEW".to_vec())
    }

    #[test]
    fn precompile_out_mode_writes_artifact_and_restores_install() {
        let base = std::env::temp_dir().join("gore-as-compile-out");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(src.join("AI")).unwrap();
        std::fs::write(src.join("AI").join("Mod.as"), b"script").unwrap();
        let out = base.join("out.Cache");

        let opts = PrecompileOpts {
            game_dir: game,
            src: Some(src),
            out: Some(out.clone()),
            backup: true,
        };
        let res = precompile_with(&opts, gen_new).unwrap();

        assert_eq!(res, out);
        assert_eq!(std::fs::read(&out).unwrap(), b"NEW", "artifact holds the compiled bytes");
        assert_eq!(std::fs::read(&cache).unwrap(), b"OLD", "live cache restored (install pristine)");
        assert!(
            !cache.parent().unwrap().join("AI").join("Mod.as").exists(),
            "staged .as removed from Script/"
        );
        assert!(!deploy_bak_path(&cache).exists(), "out-mode leaves no .gore-bak");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_in_place_installs_new_cache_and_backs_up() {
        let base = std::env::temp_dir().join("gore-as-compile-inplace");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let opts = PrecompileOpts {
            game_dir: game,
            src: Some(src),
            out: None,
            backup: true,
        };
        let res = precompile_with(&opts, gen_new).unwrap();

        assert_eq!(res, cache);
        assert_eq!(std::fs::read(&cache).unwrap(), b"NEW", "new cache installed in place");
        assert_eq!(
            std::fs::read(deploy_bak_path(&cache)).unwrap(),
            b"OLD",
            "previous cache backed up to .gore-bak"
        );
        assert!(!cache.parent().unwrap().join("Mod.as").exists(), "staged .as cleaned");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_in_place_keeps_new_cache_even_if_src_carries_a_cache_file() {
        // Regression: a staged src tree that happens to include a file at the cache path must NOT
        // cause cleanup to restore the old cache over the freshly compiled one.
        let base = std::env::temp_dir().join("gore-as-compile-srccache");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("PrecompiledScript_Shipping.Cache"), b"SRCCACHE").unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let opts = PrecompileOpts {
            game_dir: game,
            src: Some(src),
            out: None,
            backup: true,
        };
        let res = precompile_with(&opts, gen_new).unwrap();

        assert_eq!(res, cache);
        assert_eq!(
            std::fs::read(&cache).unwrap(),
            b"NEW",
            "freshly compiled cache kept, not clobbered by the staged src cache file"
        );
        assert!(!cache.parent().unwrap().join("Mod.as").exists(), "staged .as cleaned");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_rejects_src_that_contains_the_script_dir() {
        // SRC = the game root (which contains G1R/Script): staging would copy the install into its
        // own subtree. Must be rejected up front, before any staging or generation.
        let base = std::env::temp_dir().join("gore-as-compile-overlap");
        let _ = std::fs::remove_dir_all(&base);
        let (game, _cache) = fake_install(&base);
        let opts = PrecompileOpts {
            game_dir: game.clone(),
            src: Some(game), // the game root contains G1R/Script
            out: None,
            backup: true,
        };
        let err = precompile_with(&opts, |_, _, _| panic!("must not stage or generate")).unwrap_err();
        assert!(err.contains("contains the game's Script"), "got: {err}");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_rolls_back_on_generation_failure() {
        // A generation error rolls the install back: the live cache is restored, staged .as removed,
        // and the original error is surfaced.
        let base = std::env::temp_dir().join("gore-as-compile-genfail");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let opts = PrecompileOpts {
            game_dir: game,
            src: Some(src),
            out: None,
            backup: false,
        };
        // Stub emulates the game partially rewriting the cache, then failing.
        let err = precompile_with(&opts, |_, _, cache| {
            std::fs::write(cache, b"PARTIAL").unwrap();
            Err("boom".to_string())
        })
        .unwrap_err();

        assert!(err.contains("boom"), "surfaces the original error; got: {err}");
        assert_eq!(std::fs::read(&cache).unwrap(), b"OLD", "live cache rolled back");
        assert!(
            !cache.parent().unwrap().join("Mod.as").exists(),
            "staged .as removed on rollback"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_rejects_out_inside_script_dir() {
        // `-o` under Script/ (the live cache, or any path there) is rejected: it would pollute the
        // install and could collide with a staged file / the restore.
        let base = std::env::temp_dir().join("gore-as-compile-outinside");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        for out in [cache.clone(), cache.parent().unwrap().join("MyMod.Cache")] {
            let opts = PrecompileOpts {
                game_dir: game.clone(),
                src: None,
                out: Some(out.clone()),
                backup: false,
            };
            let err = precompile_with(&opts, |_, _, _| panic!("must not generate")).unwrap_err();
            assert!(err.contains("Script/ directory"), "out={:?} got: {err}", out);
        }
        assert_eq!(std::fs::read(&cache).unwrap(), b"OLD", "live cache left untouched");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_out_mode_write_failure_still_restores_install() {
        // If writing the output fails, the install must STILL be rolled back (cache restored, staged
        // removed), and the write error surfaced.
        let base = std::env::temp_dir().join("gore-as-compile-outfail");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();
        // Output under a non-existent directory → std::fs::write fails.
        let out = base.join("nope-dir").join("out.Cache");

        let opts = PrecompileOpts {
            game_dir: game,
            src: Some(src),
            out: Some(out),
            backup: false,
        };
        let err = precompile_with(&opts, gen_new).unwrap_err();

        assert!(err.contains("writing output"), "surfaces the write error; got: {err}");
        assert_eq!(std::fs::read(&cache).unwrap(), b"OLD", "live cache restored despite write failure");
        assert!(
            !cache.parent().unwrap().join("Mod.as").exists(),
            "staged .as removed despite write failure"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn resolve_out_real_makes_relative_paths_absolute_under_cwd() {
        let base = std::env::temp_dir().join("gore-as-resolve-out");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let base_real = base.canonicalize().unwrap();

        // A relative out resolves under cwd, so a Script/-relative path is caught by the guard.
        let rel = resolve_out_real(Path::new("MyMod.Cache"), &base);
        assert!(rel.starts_with(&base_real), "relative out resolved under cwd: {rel:?}");

        // An absolute out elsewhere stays where it is (not under cwd).
        let other = std::env::temp_dir().join("gore-as-resolve-other").join("x.Cache");
        let abs = resolve_out_real(&other, &base);
        assert!(!abs.starts_with(&base_real), "absolute out stays put: {abs:?}");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_rejects_src_when_script_dir_has_loose_scripts() {
        // A pre-existing loose .as in Script/ would be compiled alongside SRC — refuse.
        let base = std::env::temp_dir().join("gore-as-compile-dirty");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        std::fs::write(cache.parent().unwrap().join("Stale.as"), b"stale").unwrap();
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let opts = PrecompileOpts {
            game_dir: game,
            src: Some(src),
            out: None,
            backup: false,
        };
        let err = precompile_with(&opts, |_, _, _| panic!("must not generate")).unwrap_err();
        assert!(err.contains("loose script"), "got: {err}");
        let _ = std::fs::remove_dir_all(&base);
    }

    /// `copy_tree` records every file it writes (with its prior bytes); `restore_or_remove` then
    /// deletes the ones it created and RESTORES the ones it overwrote, plus prunes the now-empty
    /// dirs it created, while leaving the dst root AND non-colliding pre-existing files untouched.
    /// This is the offline guard for the CRITICAL "don't pollute / don't destroy the install"
    /// cleanup invariant.
    #[test]
    fn copy_tree_then_remove_written_leaves_install_clean() {
        let base = std::env::temp_dir().join("gore-as-cleanup-test");
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("src");
        let dst = base.join("dst");
        // Source tree: a top-level file, a nested subdir file, AND a file that COLLIDES with a
        // pre-existing dst file (its original content must be restored on cleanup).
        std::fs::create_dir_all(src.join("AI")).unwrap();
        std::fs::write(src.join("Top.as"), b"top").unwrap();
        std::fs::write(src.join("AI").join("Nested.as"), b"nested").unwrap();
        std::fs::write(src.join("Over.as"), b"new").unwrap();
        // Destination pre-exists and already holds a non-colliding file that must SURVIVE cleanup,
        // plus a colliding file that will be overwritten then RESTORED.
        std::fs::create_dir_all(&dst).unwrap();
        let pre = dst.join("Pre.as");
        std::fs::write(&pre, b"preexisting").unwrap();
        let over = dst.join("Over.as");
        std::fs::write(&over, b"old").unwrap();

        let mut written = Vec::new();
        copy_tree(&src, &dst, &mut written).unwrap();

        // Recorded exactly the three copied files, and they landed on disk.
        assert_eq!(written.len(), 3);
        let top = dst.join("Top.as");
        let nested = dst.join("AI").join("Nested.as");
        assert!(top.exists());
        assert!(nested.exists());
        assert!(written.iter().any(|(p, _)| p == &top));
        assert!(written.iter().any(|(p, _)| p == &nested));
        // The collision was overwritten with the new bytes, and its prior bytes were captured.
        assert_eq!(std::fs::read(&over).unwrap(), b"new");
        assert!(written.iter().any(|(p, prior)| p == &over && prior.as_deref() == Some(b"old")));

        // Cleanup succeeds: the colliding file restores and the copied-only files delete cleanly.
        restore_or_remove(&written, &dst).expect("cleanup should succeed in a writable tmp tree");

        // Copied-only files + the dir the copy created are gone.
        assert!(!top.exists(), "copied top-level file should be removed");
        assert!(!nested.exists(), "copied nested file should be removed");
        assert!(!dst.join("AI").exists(), "now-empty created dir should be pruned");
        // Non-colliding pre-existing file and the dst root itself survive.
        assert!(pre.exists(), "pre-existing file must be left untouched");
        assert!(dst.exists(), "dst root must not be removed");
        // The overwritten pre-existing file is RESTORED to its original bytes (not deleted).
        assert!(over.exists(), "overwritten pre-existing file must be restored, not deleted");
        assert_eq!(std::fs::read(&over).unwrap(), b"old", "restored bytes must be the original");

        let _ = std::fs::remove_dir_all(&base);
    }
}
