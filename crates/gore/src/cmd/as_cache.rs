use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Subcommand;

use gore_as::cache::header::CacheHeader;
use gore_as::cache::scan::scan_strings;
use gore_as::cache::splice::splice_auto;
use gore_as::cache::walk_modules::{module_count, module_region_end};

#[derive(Subcommand)]
pub enum AsCmd {
    /// Parse and print the outer cache header.
    DecodeHeader { file: PathBuf },
    /// Scan length-prefixed type-name strings (decode investigation aid).
    Walk {
        file: PathBuf,
        #[arg(long, default_value_t = 100)]
        max: usize,
    },
    /// Print module count + TAIL_OFF (the splice insertion point) for a cache.
    Info { file: PathBuf },
    /// Decompile functions whose name contains <needle> to structured AngelScript.
    Decompile {
        file: PathBuf,
        /// Substring filter on `module.Class::func` (default: all).
        #[arg(default_value = "")]
        needle: String,
        /// Max functions to print.
        #[arg(long, default_value_t = 20)]
        max: usize,
    },
    /// Emit ALL modules as recompilable .as into <outdir>, mirroring ScriptRelativeFilename.
    EmitAll {
        file: PathBuf,
        outdir: PathBuf,
    },
    /// Emit recompilable .as for modules whose name contains <needle>.
    Emit {
        file: PathBuf,
        #[arg(default_value = "")]
        needle: String,
        #[arg(long, default_value_t = 5)]
        max: usize,
    },
    /// Dump StaticNames tail-table entries (the `n"..."` FName-literal pool indexed by
    /// `__STATIC_NAME(Id)`). With no indices: count + first 10 entries.
    StaticNames {
        file: PathBuf,
        /// Specific indices to print.
        indices: Vec<i64>,
    },
    /// Disassemble functions whose name contains <needle> to an asBC listing.
    Disasm {
        file: PathBuf,
        #[arg(default_value = "")]
        needle: String,
        #[arg(long, default_value_t = 20)]
        max: usize,
    },
    /// Replace an existing module (by name) in a base cache with a mini-cache's module.
    Replace {
        base: PathBuf,
        mini: PathBuf,
        target: String,
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Splice a primitive-only mini-cache module into a base cache.
    Splice {
        /// Base cache (e.g. PrecompiledScript_Shipping.Cache).
        base: PathBuf,
        /// Mini-cache from -as-generate-precompiled-data (one primitive-only module).
        mini: PathBuf,
        /// Output path for the spliced cache.
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Extract one module into a standalone 1-module mini-cache (module + full tail tables).
    /// Lets a dependency-heavy edited module be pulled from a full-tree regen and Replace'd
    /// into the vanilla base.
    Extract {
        /// Source cache (e.g. a full-tree regen).
        cache: PathBuf,
        /// Module name (the Modules TMap key) to extract.
        module: String,
        /// Output path for the 1-module mini-cache.
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Extract one module from a regen cache AND remap its bytecode refs to a base (vanilla)
    /// cache's keys, emitting a 1-module mini with EMPTY tail tables. The result can be
    /// Replace'd into the base without growing the cache (no duplicate global tables). This is
    /// the key step for editing EXISTING modules. See work/reversing/gore-as/specs/ref-remap.md.
    ExtractRemap {
        /// Regen cache (full-tree -as-generate-precompiled-data output) containing the edit.
        regen_cache: PathBuf,
        /// Module name (the Modules TMap key) to extract + remap.
        module: String,
        /// Base (vanilla) cache whose keys the module's refs are rewritten to.
        base_cache: PathBuf,
        /// Output path for the remapped 1-module mini-cache (empty tail tables).
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Semantic byte-faithfulness oracle: diff a VANILLA cache against a REGEN (re-compilation of
    /// our decompiled source) per function, after normalizing away build-noise (ref keys N1, jump
    /// absolutes N3, constant encodings N4; opt-in slot-renumber N2). Classifies each aligned
    /// function IDENTICAL / BENIGN-DIFF / SEMANTIC-DIFF. See specs/semantic-oracle.md.
    Bytediff {
        /// Vanilla reference cache (e.g. samples/cache_A.Cache).
        vanilla: PathBuf,
        /// Regen cache (re-compilation of our decompiled .as tree).
        regen: PathBuf,
        /// Only diff modules whose name contains this substring.
        #[arg(long)]
        module: Option<String>,
        /// Only diff functions whose display name (module.Class::func) contains this substring.
        #[arg(long)]
        func: Option<String>,
        /// Filter output to a verdict: identical|benign|semantic (repeatable).
        #[arg(long = "verdict")]
        verdicts: Vec<String>,
        /// List which normalizers fired for BENIGN-DIFF functions (default: summary only).
        #[arg(long)]
        show_benign: bool,
        /// Instruction window (±N) around each SEMANTIC divergence.
        #[arg(long, default_value_t = 6)]
        context: usize,
        /// Enable the OPT-IN N2 slot-renumber normalizer (default OFF; see spec §3.2).
        #[arg(long = "norm-slots")]
        norm_slots: bool,
        /// Disable the N5 `FScopeCycleCounter` RAII profiler-scope strip (default ON; §B.2).
        #[arg(long = "no-norm-scope")]
        no_norm_scope: bool,
        /// Disable the N6 dominated boolean-cascade re-guard fold (default ON; §B.1).
        #[arg(long = "no-norm-reguard")]
        no_norm_reguard: bool,
        /// Write a machine-readable JSON scoreboard (per-verdict counts + alignment loss) here.
        #[arg(long)]
        json: Option<PathBuf>,
        /// Exit non-zero if any SEMANTIC-DIFF is found (CI gate).
        #[arg(long)]
        fail_on_semantic: bool,
    },
}

/// Build the script-class hierarchy (class name -> super class name) from parsed modules.
/// Locate and load the native API arities from Binds.Cache: `GORE_AS_BINDS` env if set, else a
/// `Binds.Cache` sitting next to the input cache file. Absent/unparsable => None (no fallback).
fn load_native_api(cache_file: &std::path::Path) -> Option<gore_as::cache::binds::NativeApi> {
    let path = match std::env::var_os("GORE_AS_BINDS") {
        Some(p) => PathBuf::from(p),
        None => cache_file.parent()?.join("Binds.Cache"),
    };
    if !path.exists() {
        return None;
    }
    let api = gore_as::cache::binds::NativeApi::load(&path);
    match &api {
        Some(_) => eprintln!("loaded native arities from {}", path.display()),
        None => eprintln!("warning: failed to parse {}", path.display()),
    }
    api
}

fn class_hierarchy(mods: &[gore_as::cache::model::Module]) -> std::collections::HashMap<String, String> {
    let mut h = std::collections::HashMap::new();
    for m in mods {
        for c in &m.classes {
            // Record EVERY script class so `is_script_class` recognizes it; a root class with
            // no super maps to "" (is_subclass stops there). Omitting no-super classes made
            // them look like engine types, skipping script-class casts/subclass checks.
            let super_name = c.super_class.clone().filter(|s| !s.is_empty()).unwrap_or_default();
            h.insert(c.name.clone(), super_name);
        }
    }
    h
}

/// All script-class METHOD names from the parsed modules (batch-24b shadow-gate input): a
/// class method shadows a same-named free global inside class scope even when the method is
/// never referenced by any bytecode (so T3 alone can miss it).
fn class_method_names(mods: &[gore_as::cache::model::Module]) -> Vec<String> {
    mods.iter()
        .flat_map(|m| m.classes.iter())
        .flat_map(|c| c.methods.iter())
        .map(|f| f.name.clone())
        .collect()
}

/// Per-class field-type maps (class -> field -> composed type name) from parsed modules, so the
/// emitter can resolve INHERITED member types across module boundaries (batch-21 Class B).
fn class_fields(
    mods: &[gore_as::cache::model::Module],
    refs: &gore_as::cache::refs::RefResolver,
) -> HashMap<String, HashMap<String, String>> {
    let mut out: HashMap<String, HashMap<String, String>> = HashMap::new();
    for m in mods {
        for c in &m.classes {
            let fields: HashMap<String, String> =
                c.fields.iter().map(|f| (f.name.clone(), f.ty.base_name(refs))).collect();
            out.insert(c.name.clone(), fields);
        }
    }
    out
}

pub fn run(cmd: AsCmd) -> Result<()> {
    match cmd {
        AsCmd::DecodeHeader { file } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let h = CacheHeader::parse(&bytes).context("parsing header")?;
            println!("hash       : {}", hex16(&h.hash));
            println!("magic      : {:#010x}", h.magic);
            println!("type_count : {}", h.type_count);
        }
        AsCmd::Walk { file, max } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            for s in scan_strings(&bytes, CacheHeader::SIZE, max) {
                println!("0x{:08x}  len={:<4} {}", s.offset, s.len, s.text);
            }
        }
        AsCmd::Info { file } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let tail = module_region_end(&bytes).context("walking modules")?;
            println!("modules  : {}", module_count(&bytes));
            println!("tail_off : {:#x}", tail);
            println!("eof      : {:#x}", bytes.len());
            println!("tail_len : {} bytes (global ref tables)", bytes.len() - tail);
        }
        AsCmd::Decompile { file, needle, max } => {
            let bytes = std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let mut refs = gore_as::cache::refs::RefResolver::build(&bytes).context("resolver")?;
            // Mirror `emit`/`emit-all`: load the class hierarchy and native arity table so
            // decompile output matches emitted source (subclass casts, native-call trimming).
            let mods = gore_as::cache::model::parse_modules(&bytes).context("parse modules")?;
            refs.set_class_hierarchy(class_hierarchy(&mods));
            let cf = class_fields(&mods, &refs);
            refs.set_class_fields(cf);
            refs.add_method_names(class_method_names(&mods));
            if let Some(api) = load_native_api(&file) {
                refs.set_native_api(api);
            }
            let funcs = gore_as::cache::walk_modules::collect_function_bytecodes(&bytes).context("walk")?;
            let mut n = 0;
            for f in funcs.iter().filter(|f| f.func.contains(&needle)) {
                if n >= max {
                    break;
                }
                println!("{}", gore_as::cache::structure::decompile(f, &refs));
                n += 1;
            }
            eprintln!("({n} function(s))");
        }
        AsCmd::EmitAll { file, outdir } => {
            let bytes = std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let mut refs = gore_as::cache::refs::RefResolver::build(&bytes).context("resolver")?;
            let mods = gore_as::cache::model::parse_modules(&bytes).context("parse modules")?;
            refs.set_class_hierarchy(class_hierarchy(&mods));
            let cf = class_fields(&mods, &refs);
            refs.set_class_fields(cf);
            refs.add_method_names(class_method_names(&mods));
            if let Some(api) = load_native_api(&file) {
                refs.set_native_api(api);
            }
            // batch-25f: publish the collision renames as an ID-based map in the resolver so
            // CALL/CALLINTF render sites in EVERY module resolve the renamed leaf. emit_all_tree
            // (below) recomputes the SAME colliding set for the per-file TEXT rename of decls; both
            // use the identical `_g{mi}` scheme so decls and call sites cannot disagree. The
            // security-hardened output (outdir canonicalize + per-entry symlink/`..` guards) now
            // lives inside emit_all_tree.
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
                    if matches!(f.name.as_str(),
                        "Spawn" | "Get" | "GetOrCreate" | "Create" | "GetG1R" | "StaticClass") {
                        continue;
                    }
                    // precise signature (render, not base_name) so only GENUINE same-signature
                    // collisions are flagged — a coarse match falsely flags distinct overloads and
                    // would rename (and break) functions that are validly called cross-module.
                    let ptys: Vec<String> = f.params.iter().map(|p| p.ty.render(&refs)).collect();
                    sig_mods.entry(format!("{}({})", f.name, ptys.join(","))).or_default().insert(i);
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
                    if matches!(f.name.as_str(),
                        "Spawn" | "Get" | "GetOrCreate" | "Create" | "GetG1R" | "StaticClass") {
                        continue;
                    }
                    let ptys: Vec<String> = f.params.iter().map(|p| p.ty.render(&refs)).collect();
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
            // batch-25f: publish the collision renames as an ID-based map in the resolver so
            // CALL/CALLINTF render sites in EVERY module resolve the renamed leaf. The TEXT
            // pass below still rewrites the DECLARING module (declaration line; already-
            // renamed `Name_g<mi>(` call sites don't match its word boundary, `_` is a word
            // char, so the two passes never double-rename). Suffixes come from the same
            // `colliding_in` set with the same `_g{mi}` scheme — decls and call sites cannot
            // disagree.
            let mut rename_map: HashMap<String, HashMap<String, String>> = HashMap::new();
            for (mi, names) in &colliding_in {
                let e = rename_map.entry(mods[*mi].name.clone()).or_default();
                for name in names {
                    e.insert(name.clone(), format!("{name}_g{mi}"));
                }
            }
            refs.set_free_fn_renames(&rename_map);
            let stats = gore_as::cache::emit_all::emit_all_tree(&mods, &refs, &outdir)
                .with_context(|| format!("emitting to {}", outdir.display()))?;
            eprintln!("emitted {} modules to {} ({} contain a stubbed function)",
                stats.written, outdir.display(), stats.stubbed);
        }
        AsCmd::Emit { file, needle, max } => {
            let bytes = std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let mut refs = gore_as::cache::refs::RefResolver::build(&bytes).context("resolver")?;
            let mods = gore_as::cache::model::parse_modules(&bytes).context("parse modules")?;
            refs.set_class_hierarchy(class_hierarchy(&mods));
            let cf = class_fields(&mods, &refs);
            refs.set_class_fields(cf);
            refs.add_method_names(class_method_names(&mods));
            if let Some(api) = load_native_api(&file) {
                refs.set_native_api(api);
            }
            let mut n = 0;
            for m in mods.iter().filter(|m| m.name.contains(&needle)) {
                if n >= max {
                    break;
                }
                println!("{}", gore_as::cache::emit::emit_module(m, &refs));
                n += 1;
            }
            eprintln!("({n} module(s))");
        }
        AsCmd::StaticNames { file, indices } => {
            let bytes = std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let refs = gore_as::cache::refs::RefResolver::build(&bytes).context("resolver")?;
            println!("StaticNames count: {}", refs.static_name_count());
            let show: Vec<i64> = if indices.is_empty() { (0..10).collect() } else { indices };
            for i in show {
                match refs.static_name(i) {
                    Some(n) => println!("  [{i}] {n:?}"),
                    None => println!("  [{i}] <out of range>"),
                }
            }
        }
        AsCmd::Disasm { file, needle, max } => {
            let bytes = std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let funcs = gore_as::cache::walk_modules::collect_function_bytecodes(&bytes).context("walk")?;
            let mut n = 0;
            for f in funcs.iter().filter(|f| f.func.contains(&needle)) {
                if n >= max {
                    break;
                }
                match gore_as::cache::disasm::disassemble(&f.bytecode) {
                    Ok(ins) => println!("// {}\n{}", f.func, gore_as::cache::disasm::listing(&ins)),
                    Err(e) => println!("// {} — {e}", f.func),
                }
                n += 1;
            }
            eprintln!("({n} function(s))");
        }
        AsCmd::Replace { base, mini, target, out } => {
            let base_b = std::fs::read(&base).with_context(|| format!("reading {}", base.display()))?;
            let mini_b = std::fs::read(&mini).with_context(|| format!("reading {}", mini.display()))?;
            let n = module_count(&base_b);
            let res = gore_as::cache::splice::replace_module(&base_b, &mini_b, &target).context("replace")?;
            std::fs::write(&out, &res).with_context(|| format!("writing {}", out.display()))?;
            println!(
                "replaced {:?}: {} modules (unchanged) ; {} -> {} bytes ; wrote {}",
                target, n, base_b.len(), res.len(), out.display()
            );
        }
        AsCmd::Splice { base, mini, out } => {
            let base_b =
                std::fs::read(&base).with_context(|| format!("reading {}", base.display()))?;
            let mini_b =
                std::fs::read(&mini).with_context(|| format!("reading {}", mini.display()))?;
            let before = module_count(&base_b);
            let spliced = splice_auto(&base_b, &mini_b).context("splicing")?;
            std::fs::write(&out, &spliced).with_context(|| format!("writing {}", out.display()))?;
            println!(
                "spliced: {} modules -> {} ; {} -> {} bytes ; wrote {}",
                before,
                module_count(&spliced),
                base_b.len(),
                spliced.len(),
                out.display()
            );
        }
        AsCmd::Extract { cache, module, out } => {
            let b = std::fs::read(&cache).with_context(|| format!("reading {}", cache.display()))?;
            let n = module_count(&b);
            let mini = gore_as::cache::splice::extract_module(&b, &module).context("extract")?;
            std::fs::write(&out, &mini).with_context(|| format!("writing {}", out.display()))?;
            println!(
                "extracted {:?} from {} modules -> 1-module mini ; {} bytes ; wrote {}",
                module, n, mini.len(), out.display()
            );
        }
        AsCmd::ExtractRemap { regen_cache, module, base_cache, out } => {
            let regen_b = std::fs::read(&regen_cache)
                .with_context(|| format!("reading {}", regen_cache.display()))?;
            let base_b = std::fs::read(&base_cache)
                .with_context(|| format!("reading {}", base_cache.display()))?;
            let n = module_count(&regen_b);
            let mini = gore_as::cache::splice::extract_module(&regen_b, &module)
                .context("extract")?;
            let (remapped, counts) =
                gore_as::cache::remap::remap_module_to_base(&mini, &base_b)
                    .context("remap")?;
            std::fs::write(&out, &remapped)
                .with_context(|| format!("writing {}", out.display()))?;
            println!(
                "extract-remap {:?} from {} modules -> remapped 1-module mini ; {} bytes ; wrote {}",
                module, n, remapped.len(), out.display()
            );
            println!(
                "refs remapped: {} total (bytecode: global={} func_ptr={} type_ptr={} func_id={} type_id={} ; embedded: type_ptr={} func_id={})",
                counts.total(),
                counts.global_ptr, counts.func_ptr, counts.type_ptr, counts.func_id, counts.type_id,
                counts.embed_type_ptr, counts.embed_func_id
            );
        }
        AsCmd::Bytediff {
            vanilla,
            regen,
            module,
            func,
            verdicts,
            show_benign,
            context,
            norm_slots,
            no_norm_scope,
            no_norm_reguard,
            json,
            fail_on_semantic,
        } => {
            use gore_as::cache::bytediff::{self, Filters, NormOpts, Verdict};
            let v_bytes = std::fs::read(&vanilla)
                .with_context(|| format!("reading vanilla {}", vanilla.display()))?;
            let r_bytes = std::fs::read(&regen)
                .with_context(|| format!("reading regen {}", regen.display()))?;

            let mut opts = NormOpts::default();
            opts.n2_slots = norm_slots;
            opts.n5_scope = !no_norm_scope;
            opts.n6_reguard = !no_norm_reguard;
            let filters = Filters { module: module.clone(), func: func.clone() };

            let report = bytediff::run(&v_bytes, &r_bytes, &opts, &filters, context)
                .context("bytediff")?;

            // Verdict filter for per-function output (empty = show all).
            let want = |v: Verdict| -> bool {
                if verdicts.is_empty() {
                    return true;
                }
                verdicts.iter().any(|s| match s.as_str() {
                    "identical" => v == Verdict::Identical,
                    "benign" => v == Verdict::Benign,
                    "semantic" => v == Verdict::Semantic,
                    _ => false,
                })
            };

            // Per-function lines. SEMANTIC always prints its window; BENIGN prints its fired
            // normalizers only under --show-benign; IDENTICAL prints a one-liner when explicitly
            // requested via --verdict identical (otherwise summarized to keep 162k-fn runs sane).
            let show_identical_lines = verdicts.iter().any(|s| s == "identical");
            for d in &report.diffs {
                if !want(d.verdict) {
                    continue;
                }
                match d.verdict {
                    Verdict::Identical => {
                        if show_identical_lines {
                            println!("{}  IDENTICAL  (v={} ops, r={} ops)", d.name, d.v_ops, d.r_ops);
                        }
                    }
                    Verdict::Benign => {
                        let labels = d.fired.labels();
                        if show_benign {
                            println!(
                                "{}  BENIGN-DIFF  [{}]  (v={} ops, r={} ops)",
                                d.name, labels.join(" "), d.v_ops, d.r_ops
                            );
                        }
                    }
                    Verdict::Semantic => {
                        println!(
                            "{}  SEMANTIC-DIFF  (v={} ops, r={} ops)",
                            d.name, d.v_ops, d.r_ops
                        );
                        if let Some(h) = &d.hint {
                            println!("    hint: {h}");
                        }
                        if let Some(w) = &d.window {
                            print!("{w}");
                        }
                    }
                }
            }

            // Alignment loss (always reported — a dropped/added symbol is a severe defect).
            for m in &report.only_in_vanilla_modules {
                println!("ONLY-IN-VANILLA module: {m}");
            }
            for m in &report.only_in_regen_modules {
                println!("ONLY-IN-REGEN module: {m}");
            }
            if func.is_none() {
                for f in &report.only_in_vanilla_funcs {
                    println!("ONLY-IN-VANILLA func: {f}");
                }
                for f in &report.only_in_regen_funcs {
                    println!("ONLY-IN-REGEN func: {f}");
                }
            }

            // Summary scoreboard.
            let n_ident = report.count(Verdict::Identical);
            let n_benign = report.count(Verdict::Benign);
            let n_sem = report.count(Verdict::Semantic);
            let aligned = report.diffs.len();
            let b1 = if aligned > 0 {
                100.0 * (n_ident + n_benign) as f64 / aligned as f64
            } else {
                100.0
            };
            eprintln!("---- bytediff scoreboard ----");
            eprintln!("aligned functions : {aligned}");
            eprintln!("  IDENTICAL       : {n_ident}");
            eprintln!("  BENIGN-DIFF     : {n_benign}");
            eprintln!("  SEMANTIC-DIFF   : {n_sem}");
            eprintln!(
                "alignment loss    : {} module(s) only-in-vanilla, {} only-in-regen, {} func(s) only-in-vanilla, {} only-in-regen",
                report.only_in_vanilla_modules.len(),
                report.only_in_regen_modules.len(),
                report.only_in_vanilla_funcs.len(),
                report.only_in_regen_funcs.len()
            );
            eprintln!("B1 byte-faithful  : {b1:.2}%  (IDENTICAL+BENIGN / aligned)");
            // Per-normalizer fire counts across BENIGN functions.
            let (mut c1, mut c2, mut c3, mut c4) = (0usize, 0usize, 0usize, 0usize);
            let (mut c5, mut c6) = (0usize, 0usize);
            for d in &report.diffs {
                if d.verdict == Verdict::Benign {
                    c1 += d.fired.n1_refs as usize;
                    c2 += d.fired.n2_slots as usize;
                    c3 += d.fired.n3_jumps as usize;
                    c4 += d.fired.n4_consts as usize;
                    c5 += d.fired.n5_scope as usize;
                    c6 += d.fired.n6_reguard as usize;
                }
            }
            eprintln!(
                "normalizer fires  : N1:refs={c1} N2:slots={c2} N3:jumps={c3} N4:consts={c4} N5:scope={c5} N6:reguard={c6}"
            );

            if let Some(jpath) = &json {
                let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
                let mut sem_list = String::from("[");
                let mut first = true;
                for d in &report.diffs {
                    if d.verdict == Verdict::Semantic {
                        if !first {
                            sem_list.push(',');
                        }
                        first = false;
                        let hint = d.hint.as_deref().unwrap_or("");
                        sem_list.push_str(&format!(
                            "{{\"name\":\"{}\",\"v_ops\":{},\"r_ops\":{},\"hint\":\"{}\"}}",
                            esc(&d.name), d.v_ops, d.r_ops, esc(hint)
                        ));
                    }
                }
                sem_list.push(']');
                let json_out = format!(
                    "{{\n  \"aligned\": {aligned},\n  \"identical\": {n_ident},\n  \"benign\": {n_benign},\n  \"semantic\": {n_sem},\n  \"b1_byte_faithful_pct\": {b1:.4},\n  \"only_in_vanilla_modules\": {},\n  \"only_in_regen_modules\": {},\n  \"only_in_vanilla_funcs\": {},\n  \"only_in_regen_funcs\": {},\n  \"normalizer_fires\": {{\"n1_refs\": {c1}, \"n2_slots\": {c2}, \"n3_jumps\": {c3}, \"n4_consts\": {c4}, \"n5_scope\": {c5}, \"n6_reguard\": {c6}}},\n  \"semantic_list\": {sem_list}\n}}\n",
                    report.only_in_vanilla_modules.len(),
                    report.only_in_regen_modules.len(),
                    report.only_in_vanilla_funcs.len(),
                    report.only_in_regen_funcs.len(),
                );
                std::fs::write(jpath, &json_out)
                    .with_context(|| format!("writing json {}", jpath.display()))?;
                eprintln!("wrote JSON scoreboard to {}", jpath.display());
            }

            if fail_on_semantic && report.any_semantic() {
                anyhow::bail!("{n_sem} SEMANTIC-DIFF function(s) found (--fail-on-semantic)");
            }
        }
    }
    Ok(())
}

fn hex16(b: &[u8; 16]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
