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
