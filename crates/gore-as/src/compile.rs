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
    if opts.op != "add" && opts.op != "edit" {
        return Err(CompileError::Other(format!(
            "invalid script op {:?} for module {:?} (want \"add\" or \"edit\")",
            opts.op, opts.module_name
        )));
    }
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

    // 4. Extract (add) / extract+remap (edit) the target module → mini-cache.
    let mini = match opts.op.as_str() {
        "edit" => {
            let out = splice::extract_module(&regen, &opts.module_name)
                .map_err(|e| CompileError::Other(format!("extract: {e}")))?;
            let (remapped, _counts) = remap::remap_module_to_base(&out, &base)
                .map_err(|e| CompileError::Other(format!("remap: {e}")))?;
            remapped
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

/// Load native arities from a `Binds.Cache` sitting next to `cache_file`, if present. Mirrors
/// `as_cache.rs::load_native_api` / gore-ffi's `as_native_api`. Absent/unparsable => None.
fn native_api(cache_file: &Path) -> Option<crate::cache::binds::NativeApi> {
    let path = cache_file.parent()?.join("Binds.Cache");
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

    // Snapshot the live cache so we can restore it, and back it up to disk too.
    let saved_cache = std::fs::read(&cache).map_err(|e| format!("reading live cache: {e}"))?;
    let backup = cache.with_extension("Cache.gore-compile-bak");
    std::fs::write(&backup, &saved_cache).map_err(|e| format!("backing up cache: {e}"))?;

    // Everything that touches the install runs inside this closure; cleanup below runs
    // UNCONDITIONALLY afterwards (Rust has no try/finally). `written` records every file we copy
    // into Script/ so cleanup removes exactly those (even on a partial-copy or launch error).
    let mut written: Vec<PathBuf> = Vec::new();
    let regen_out = src_dir.join("regen.cache");
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

        if !regen_out.exists() {
            return Err(format!(
                "no regenerated cache produced — confirm the game compiles loose .as under {} with \
                 `-as-generate-precompiled-data` (see plan §unverified)", script_dir.display()
            ));
        }
        Ok(regen_out.clone())
    })();

    // UNCONDITIONAL cleanup: restore the pristine cache, drop the backup, and remove every file we
    // copied into Script/ plus any now-empty dirs we created — leaving the install as we found it.
    let _ = std::fs::write(&cache, &saved_cache);
    let _ = std::fs::remove_file(&backup);
    remove_written(&written, &script_dir);

    result
}

/// Recursively copy `src` into `dst`, recording every destination FILE path written into `out`
/// (so the caller can delete exactly what it staged). Directories created are not recorded
/// individually — empty ones are pruned bottom-up by [`remove_written`].
fn copy_tree(src: &Path, dst: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &to, out)?;
        } else {
            std::fs::copy(entry.path(), &to)?;
            out.push(to);
        }
    }
    Ok(())
}

/// Delete every file in `written`, then remove any directories that became empty as a result,
/// walking UP toward (but never past) `root`. `root` itself is left in place. Best-effort: a
/// non-empty dir (held a pre-existing file) is left alone, so pre-existing content survives.
fn remove_written(written: &[PathBuf], root: &Path) {
    use std::collections::BTreeSet;
    // Remove the files first.
    for f in written {
        let _ = std::fs::remove_file(f);
    }
    // Collect candidate parent dirs (deepest first via reverse-sorted full paths), bounded to
    // strict descendants of `root`, then try to remove each empty one bottom-up. Removing a child
    // can empty its parent, so seed parents transitively up to `root`.
    let mut dirs: BTreeSet<PathBuf> = BTreeSet::new();
    for f in written {
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
        // `remove_dir` only succeeds on an empty dir — pre-existing siblings keep it alive.
        let _ = std::fs::remove_dir(dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `copy_tree` records every file it writes; `remove_written` then deletes exactly those plus
    /// the now-empty dirs it created, while leaving the dst root AND pre-existing files untouched.
    /// This is the offline guard for the CRITICAL "don't pollute the install" cleanup invariant.
    #[test]
    fn copy_tree_then_remove_written_leaves_install_clean() {
        let base = std::env::temp_dir().join("gore-as-cleanup-test");
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("src");
        let dst = base.join("dst");
        // Source tree: a top-level file + a nested subdir file.
        std::fs::create_dir_all(src.join("AI")).unwrap();
        std::fs::write(src.join("Top.as"), b"top").unwrap();
        std::fs::write(src.join("AI").join("Nested.as"), b"nested").unwrap();
        // Destination pre-exists and already holds a file that must SURVIVE cleanup.
        std::fs::create_dir_all(&dst).unwrap();
        let pre = dst.join("Pre.as");
        std::fs::write(&pre, b"preexisting").unwrap();

        let mut written = Vec::new();
        copy_tree(&src, &dst, &mut written).unwrap();

        // Recorded exactly the two copied files, and they landed on disk.
        assert_eq!(written.len(), 2);
        let top = dst.join("Top.as");
        let nested = dst.join("AI").join("Nested.as");
        assert!(top.exists());
        assert!(nested.exists());
        assert!(written.contains(&top));
        assert!(written.contains(&nested));

        remove_written(&written, &dst);

        // Copied files + the dir the copy created are gone.
        assert!(!top.exists(), "copied top-level file should be removed");
        assert!(!nested.exists(), "copied nested file should be removed");
        assert!(!dst.join("AI").exists(), "now-empty created dir should be pruned");
        // Pre-existing file and the dst root itself survive.
        assert!(pre.exists(), "pre-existing file must be left untouched");
        assert!(dst.exists(), "dst root must not be removed");

        let _ = std::fs::remove_dir_all(&base);
    }
}
