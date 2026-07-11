//! Emit ALL modules of a precompiled cache as recompilable `.as` into a directory, mirroring
//! each module's ScriptRelativeFilename. Free-function name collisions across modules are
//! de-collided per-module (AngelScript compiles all loose `.as` into one global scope).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::model::Module;
use super::refs::RefResolver;

pub struct EmitAllStats {
    pub written: usize,
    /// Functions with an editable body actually written to the source tree.
    pub functions: usize,
    /// Raw function/method/constructor records parsed from the cache, including generated
    /// accessors/default initializers and duplicate records intentionally omitted by the emitter.
    pub cache_function_records: usize,
    /// Number of modules containing at least one signature-preserving fallback body.
    pub stubbed: usize,
    /// Exact number of signature-preserving fallback bodies across all modules.
    pub stubbed_functions: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum EmitAllError {
    #[error("io: {0}")]
    Io(String),
}

/// Rename whole-word free occurrences of `name` to `newname` in `src` — occurrences NOT preceded
/// by `.` (so member calls `obj.name(...)` are left alone; only the decl + free calls change).
pub fn rename_free_fn(src: &str, name: &str, newname: &str) -> String {
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
/// module/function totals and how many contain a stubbed (not-fully-recovered) body.
pub fn emit_all_tree(
    mods: &[Module],
    refs: &RefResolver,
    outdir: &Path,
) -> Result<EmitAllStats, EmitAllError> {
    let io = |ctx: &str| {
        let ctx = ctx.to_string();
        move |e: std::io::Error| EmitAllError::Io(format!("{ctx}: {e}"))
    };
    // Resolve the output root up front so a symlinked `outdir` is followed to its real
    // target ONCE here — the per-entry component check below then guards the untrusted
    // cache paths against that real root. (create_dir_all so canonicalize succeeds.)
    std::fs::create_dir_all(outdir).map_err(io(&format!("creating {}", outdir.display())))?;
    let outdir = outdir
        .canonicalize()
        .map_err(io(&format!("resolving {}", outdir.display())))?;
    // Cross-module free-function collisions: AngelScript compiles all loose .as into ONE
    // global scope, so two modules each defining `Foo(<same params>)` (even with different
    // return types) collide as "a function with the same name and parameters already
    // exists". Find such names and rename each per-module — a function's decl and its
    // intra-module free calls live in the same emitted file, so a file-local rename
    // de-collides without breaking resolution (cross-module calls of these don't occur).
    let mut sig_mods: HashMap<String, HashSet<usize>> = HashMap::new();
    for (i, m) in mods.iter().enumerate() {
        for f in &m.functions {
            // generated factory accessors are skipped at emit (not free-emitted) — never
            // rename them, or free CALLS to the native binding would be broken.
            if matches!(
                f.name.as_str(),
                "Spawn" | "Get" | "GetOrCreate" | "Create" | "GetG1R" | "StaticClass"
            ) {
                continue;
            }
            // precise signature (render, not base_name) so only GENUINE same-signature
            // collisions are flagged — a coarse match falsely flags distinct overloads and
            // would rename (and break) functions that are validly called cross-module.
            let ptys: Vec<String> = f.params.iter().map(|p| p.ty.render(refs)).collect();
            sig_mods
                .entry(format!("{}({})", f.name, ptys.join(",")))
                .or_default()
                .insert(i);
        }
    }
    // A name is safe to file-locally rename in a module only if EVERY emittable free
    // function of that name in the module has a colliding signature. If the module also has
    // a NON-colliding same-name overload, the name-based `rename_free_fn` would rewrite that
    // overload's decl + calls too, breaking other modules that call it by its original name.
    // In that mixed case leave the name un-renamed: the genuine collision then surfaces at
    // generate as a "already exists" stub (rare, safe) instead of silently breaking a valid
    // cross-module call to the non-colliding overload.
    let colliding_sigs: HashSet<&str> = sig_mods
        .iter()
        .filter(|(_, modset)| modset.len() > 1)
        .map(|(sig, _)| sig.as_str())
        .collect();
    let mut colliding_in: HashMap<usize, HashSet<String>> = HashMap::new();
    for (i, m) in mods.iter().enumerate() {
        let mut sigs_by_name: HashMap<&str, Vec<String>> = HashMap::new();
        for f in &m.functions {
            if matches!(
                f.name.as_str(),
                "Spawn" | "Get" | "GetOrCreate" | "Create" | "GetG1R" | "StaticClass"
            ) {
                continue;
            }
            let ptys: Vec<String> = f.params.iter().map(|p| p.ty.render(refs)).collect();
            sigs_by_name
                .entry(f.name.as_str())
                .or_default()
                .push(format!("{}({})", f.name, ptys.join(",")));
        }
        for (name, sigs) in sigs_by_name {
            if sigs.iter().any(|s| colliding_sigs.contains(s.as_str()))
                && sigs.iter().all(|s| colliding_sigs.contains(s.as_str()))
            {
                colliding_in.entry(i).or_default().insert(name.to_string());
            }
        }
    }
    let (
        mut written,
        mut functions,
        mut cache_function_records,
        mut stubbed,
        mut stubbed_functions,
    ) = (0usize, 0usize, 0usize, 0usize, 0usize);
    for (mi, m) in mods.iter().enumerate() {
        let mut src = super::emit::emit_module(m, refs);
        functions += super::emit::emitted_body_count(m, refs);
        cache_function_records += m.functions.len()
            + m.classes
                .iter()
                .map(|class| class.methods.len() + class.ctors.len())
                .sum::<usize>();
        if let Some(names) = colliding_in.get(&mi) {
            for name in names {
                src = rename_free_fn(&src, name, &format!("{name}_g{mi}"));
            }
        }
        let module_stubs = src.matches("body not fully recovered — stub [").count();
        if module_stubs != 0 {
            stubbed += 1;
            stubbed_functions += module_stubs;
        }
        let rel = if m.file.is_empty() {
            format!("{}.as", m.name)
        } else {
            m.file.clone()
        };
        let rel = rel.replace('\\', "/");
        // A cache entry's relative filename is untrusted: reject `..`, absolute, or
        // drive-prefixed components so output can never escape `outdir`.
        if std::path::Path::new(&rel).components().any(|c| {
            use std::path::Component::*;
            matches!(c, ParentDir | RootDir | Prefix(_))
        }) {
            eprintln!("skipping {}: unsafe output path {rel:?}", m.name);
            continue;
        }
        // The lexical check above stops `..`/absolute paths, but a pre-existing
        // symlinked component under outdir would still be FOLLOWED by create_dir_all /
        // write, escaping outdir. Reject any existing symlink along outdir -> path.
        let mut cur = outdir.clone();
        let mut symlinked = None;
        for comp in std::path::Path::new(&rel).components() {
            cur.push(comp);
            if std::fs::symlink_metadata(&cur).is_ok_and(|md| md.file_type().is_symlink()) {
                symlinked = Some(cur.clone());
                break;
            }
        }
        if let Some(link) = symlinked {
            eprintln!(
                "skipping {}: symlinked path component {}",
                m.name,
                link.display()
            );
            continue;
        }
        let path = outdir.join(&rel);
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).map_err(io(&format!("creating {}", p.display())))?;
        }
        std::fs::write(&path, src).map_err(io(&format!("writing {}", path.display())))?;
        written += 1;
    }
    Ok(EmitAllStats {
        written,
        functions,
        cache_function_records,
        stubbed,
        stubbed_functions,
    })
}
