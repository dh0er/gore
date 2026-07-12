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
    EmitAll { file: PathBuf, outdir: PathBuf },
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
    /// Offline-check whether the optional diagnostics hook has one safe AOB match. Does not launch
    /// the game or change the installation.
    DiagnosticsCheck {
        /// Exact executable to scan (supports non-Steam/custom layouts).
        #[arg(long, conflicts_with = "game")]
        exe: Option<PathBuf>,
        /// Game install root. Falls back to configured path, then Steam auto-detect.
        #[arg(long)]
        game: Option<PathBuf>,
    },
    /// Compile AngelScript into a precompiled cache by driving the game's own
    /// `-as-generate-precompiled-data` flag. With no SRC, recompiles the loose `.as` already under
    /// `<game>/G1R/Script/`; with SRC, stages that tree first. With `-o`, writes the cache there and
    /// leaves the install untouched; without `-o`, installs the fresh cache in place (backing the
    /// previous one up to `*.gore-bak`). All the backup / stage / restore file handling is internal.
    Compile {
        /// Source `.as` tree (a directory) to compile. Omit to recompile the loose `.as` already
        /// installed under `<game>/G1R/Script/`.
        src: Option<PathBuf>,
        /// Write the compiled cache here and leave the game install untouched. Omit to install the
        /// fresh cache in place under `Script/`.
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// Game install root (the folder containing `G1R/`). Falls back to the configured game
        /// path, then Steam auto-detect.
        #[arg(long)]
        game: Option<PathBuf>,
        /// When installing in place, do NOT back up the previous cache.
        #[arg(long)]
        no_backup: bool,
        /// Disable the optional runtime compiler-diagnostic hook and use the normal generator.
        #[arg(long, conflicts_with = "diagnostics_hook")]
        no_diagnostics: bool,
        /// Explicit `gore-as-diagnostics-hook.dll`; otherwise use environment, sibling, then the
        /// integrity-checked embedded helper.
        #[arg(long, value_name = "DLL")]
        diagnostics_hook: Option<PathBuf>,
        /// Delay between game launch and diagnostics injection (loader warm-up).
        #[arg(
            long,
            default_value_t = 2000,
            value_name = "MS",
            value_parser = clap::value_parser!(u64).range(0..=30_000)
        )]
        diagnostics_inject_delay_ms: u64,
    },
    /// Compile one authored module into a deployable 1-module mini-cache. This wraps the complete
    /// Studio pipeline: emit the pristine source tree, overlay one `.as` file, drive the game
    /// compiler, extract the resulting module, and remap it back to the pristine cache.
    CompileModule {
        /// `add` for a new module or `edit` for an existing module.
        #[arg(long, value_parser = ["add", "edit"])]
        op: String,
        /// Expected module name. For `add`, the compiler-detected module name is reported and used.
        #[arg(long)]
        module: String,
        /// Safe path of the authored file relative to the game's `Script/` tree.
        #[arg(long)]
        rel_path: String,
        /// Authored `.as` source file to overlay.
        #[arg(long)]
        source: PathBuf,
        /// Persistent compiler workspace used for the emitted tree and intermediate regen cache.
        #[arg(long)]
        work_dir: PathBuf,
        /// Explicitly retain minimal rows for classes/functions/names absent from the pristine
        /// cache. Normally used with `--op add`; strict remapping remains the default.
        #[arg(long)]
        allow_new_symbols: bool,
        /// Output path for the remapped 1-module mini-cache.
        #[arg(short, long)]
        out: PathBuf,
        /// Game install root. Falls back to configured path, then Steam auto-detect.
        #[arg(long)]
        game: Option<PathBuf>,
        /// Disable the optional runtime compiler-diagnostic hook and use the normal generator.
        #[arg(long, conflicts_with = "diagnostics_hook")]
        no_diagnostics: bool,
        /// Explicit `gore-as-diagnostics-hook.dll`; otherwise use environment, sibling, then the
        /// integrity-checked embedded helper.
        #[arg(long, value_name = "DLL")]
        diagnostics_hook: Option<PathBuf>,
        /// Delay between game launch and diagnostics injection (loader warm-up).
        #[arg(
            long,
            default_value_t = 2000,
            value_name = "MS",
            value_parser = clap::value_parser!(u64).range(0..=30_000)
        )]
        diagnostics_inject_delay_ms: u64,
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
    /// cache's keys, normally emitting a 1-module mini with EMPTY tail tables. With
    /// --allow-new-symbols, the mini instead carries only the genuinely-new rows required by the
    /// module. The result can be Replace'd into the base without copying the regen's full tables.
    ExtractRemap {
        /// Regen cache (full-tree -as-generate-precompiled-data output) containing the edit.
        regen_cache: PathBuf,
        /// Module name (the Modules TMap key) to extract + remap.
        module: String,
        /// Base (vanilla) cache whose keys the module's refs are rewritten to.
        base_cache: PathBuf,
        /// Explicitly carry minimal tail-table rows for symbols absent from the base. Existing
        /// symbols still remap to vanilla; pointer/id collisions are re-keyed deterministically.
        #[arg(long)]
        allow_new_symbols: bool,
        /// Output path for the remapped 1-module mini-cache.
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Semantic byte-faithfulness oracle: diff a VANILLA cache against a REGEN (re-compilation of
    /// our decompiled source) per function, after normalizing away build-noise (ref keys N1, jump
    /// absolutes N3, constant encodings N4; opt-in slot-allocation proofs N2). Classifies each aligned
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
        /// Enable OPT-IN fail-closed N2 slot-allocation normalization (default OFF; see FORMAT.md).
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
            println!(
                "tail_len : {} bytes (global ref tables)",
                bytes.len() - tail
            );
        }
        AsCmd::Decompile { file, needle, max } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let mut refs = gore_as::cache::refs::RefResolver::build(&bytes).context("resolver")?;
            // Mirror `emit`/`emit-all`: load the class hierarchy and native arity table so
            // decompile output matches emitted source (subclass casts, native-call trimming).
            let mods = gore_as::cache::model::parse_modules(&bytes).context("parse modules")?;
            gore_as::cache::emit_all::prepare_resolver_semantics(
                &mods,
                &mut refs,
                load_native_api(&file),
            );
            let funcs =
                gore_as::cache::walk_modules::collect_function_bytecodes(&bytes).context("walk")?;
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
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let mut refs = gore_as::cache::refs::RefResolver::build(&bytes).context("resolver")?;
            let mods = gore_as::cache::model::parse_modules(&bytes).context("parse modules")?;
            let stats = gore_as::cache::emit_all::emit_all_tree(
                &mods,
                &mut refs,
                load_native_api(&file),
                &outdir,
            )
            .with_context(|| format!("emitting to {}", outdir.display()))?;
            eprintln!(
                "emitted {} modules / {} body-bearing functions to {} ({} cache function records; {} modules / {} functions contain a stubbed body)",
                stats.written,
                stats.functions,
                outdir.display(),
                stats.cache_function_records,
                stats.stubbed,
                stats.stubbed_functions
            );
        }
        AsCmd::Emit { file, needle, max } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let mut refs = gore_as::cache::refs::RefResolver::build(&bytes).context("resolver")?;
            let mods = gore_as::cache::model::parse_modules(&bytes).context("parse modules")?;
            let prepared = gore_as::cache::emit_all::PreparedEmit::new(
                &mods,
                &mut refs,
                load_native_api(&file),
            )
            .context("prepare emitted modules")?;
            let mut n = 0;
            for (module_index, _) in mods
                .iter()
                .enumerate()
                .filter(|(_, module)| module.name.contains(&needle))
            {
                if n >= max {
                    break;
                }
                println!("{}", prepared.emit_module(module_index)?);
                n += 1;
            }
            eprintln!("({n} module(s))");
        }
        AsCmd::StaticNames { file, indices } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let refs = gore_as::cache::refs::RefResolver::build(&bytes).context("resolver")?;
            println!("StaticNames count: {}", refs.static_name_count());
            let show: Vec<i64> = if indices.is_empty() {
                (0..10).collect()
            } else {
                indices
            };
            for i in show {
                match refs.static_name(i) {
                    Some(n) => println!("  [{i}] {n:?}"),
                    None => println!("  [{i}] <out of range>"),
                }
            }
        }
        AsCmd::Disasm { file, needle, max } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let funcs =
                gore_as::cache::walk_modules::collect_function_bytecodes(&bytes).context("walk")?;
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
        AsCmd::DiagnosticsCheck { exe, game } => {
            let exe = match exe {
                Some(exe) => exe,
                None => {
                    let root = gore_loc::config::game_root(game).context("resolving game path")?;
                    let g1r = if root.file_name().is_some_and(|name| name == "G1R") {
                        root
                    } else {
                        root.join("G1R")
                    };
                    g1r.join("Binaries")
                        .join("Win64")
                        .join("G1R-Win64-Shipping.exe")
                }
            };
            let probe = gore_as::diagnostics::probe_executable(&exe)
                .map_err(anyhow::Error::msg)
                .with_context(|| format!("scanning {}", exe.display()))?;
            println!("exe: {}", exe.display());
            println!("sha256: {}", probe.sha256);
            println!("signature matches: {}", probe.match_count);
            for rva in &probe.matched_rvas {
                println!("matched RVA: 0x{rva:x}");
            }
            println!(
                "callback structure: {}",
                match (probe.match_count, probe.callback_shape_verified) {
                    (1, true) => "verified",
                    (1, false) => "mismatch",
                    _ => "not checked (signature not unique)",
                }
            );
            if probe.match_count != 1 {
                anyhow::bail!(
                    "diagnostics hook unavailable: signature matched {} times in {} (need exactly 1; normal `gore as compile` will fall back)",
                    probe.match_count,
                    exe.display()
                );
            }
            if !probe.callback_shape_verified {
                anyhow::bail!(
                    "diagnostics hook unavailable: the unique signature in {} did not match the verified asSMessageInfo callback structure (normal `gore as compile` will fall back)",
                    exe.display()
                );
            }
            println!("diagnostics hook compatible");
        }
        AsCmd::Compile {
            src,
            out,
            game,
            no_backup,
            no_diagnostics,
            diagnostics_hook,
            diagnostics_inject_delay_ms,
        } => {
            let game = gore_loc::config::game_root(game).context("resolving game path")?;
            let opts = gore_as::compile::PrecompileOpts {
                game_dir: game,
                src,
                out,
                backup: !no_backup,
            };
            let diagnostics = gore_as::diagnostics::DiagnosticsOptions {
                disabled: no_diagnostics,
                hook_dll: diagnostics_hook,
                inject_delay: std::time::Duration::from_millis(diagnostics_inject_delay_ms),
            };
            let cache = gore_as::compile::precompile_with_diagnostics(&opts, &diagnostics)
                .map_err(anyhow::Error::msg)?;
            match std::fs::metadata(&cache) {
                Ok(m) => println!("compiled -> {} ({} bytes)", cache.display(), m.len()),
                Err(_) => println!("game ran, but no cache found at {}", cache.display()),
            }
        }
        AsCmd::CompileModule {
            op,
            module,
            rel_path,
            source,
            work_dir,
            allow_new_symbols,
            out,
            game,
            no_diagnostics,
            diagnostics_hook,
            diagnostics_inject_delay_ms,
        } => {
            let game = gore_loc::config::game_root(game).context("resolving game path")?;
            let base_override = gore_mod::pristine_script_cache(&game)
                .context("reading the drift-aware pristine script cache")?;
            let opts = gore_as::compile::CompileOpts {
                game_dir: game,
                op,
                module_name: module,
                rel_path,
                as_path: source,
                work_dir,
                allow_new_symbols,
                base_override: Some(base_override),
            };
            let diagnostics = gore_as::diagnostics::DiagnosticsOptions {
                disabled: no_diagnostics,
                hook_dll: diagnostics_hook,
                inject_delay: std::time::Duration::from_millis(diagnostics_inject_delay_ms),
            };
            let compiled = gore_as::compile::compile_module(&opts, |game, tree| {
                gore_as::compile::game_run_regen_with_diagnostics(game, tree, &diagnostics)
            })
            .context("compiling module")?;
            let mini = std::fs::read(&compiled.mini_path).with_context(|| {
                format!(
                    "reading compiled mini-cache {}",
                    compiled.mini_path.display()
                )
            })?;
            if let Some(parent) = out.parent().filter(|parent| !parent.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating {}", parent.display()))?;
            }
            std::fs::write(&out, &mini).with_context(|| format!("writing {}", out.display()))?;
            println!(
                "compiled module {:?} -> {} ({} bytes)",
                compiled.module_name,
                out.display(),
                mini.len()
            );
        }
        AsCmd::Replace {
            base,
            mini,
            target,
            out,
        } => {
            let base_b =
                std::fs::read(&base).with_context(|| format!("reading {}", base.display()))?;
            let mini_b =
                std::fs::read(&mini).with_context(|| format!("reading {}", mini.display()))?;
            let n = module_count(&base_b);
            let res = gore_as::cache::splice::replace_module(&base_b, &mini_b, &target)
                .context("replace")?;
            std::fs::write(&out, &res).with_context(|| format!("writing {}", out.display()))?;
            println!(
                "replaced {:?}: {} modules (unchanged) ; {} -> {} bytes ; wrote {}",
                target,
                n,
                base_b.len(),
                res.len(),
                out.display()
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
            let b =
                std::fs::read(&cache).with_context(|| format!("reading {}", cache.display()))?;
            let n = module_count(&b);
            let mini = gore_as::cache::splice::extract_module(&b, &module).context("extract")?;
            std::fs::write(&out, &mini).with_context(|| format!("writing {}", out.display()))?;
            println!(
                "extracted {:?} from {} modules -> 1-module mini ; {} bytes ; wrote {}",
                module,
                n,
                mini.len(),
                out.display()
            );
        }
        AsCmd::ExtractRemap {
            regen_cache,
            module,
            base_cache,
            allow_new_symbols,
            out,
        } => {
            let regen_b = std::fs::read(&regen_cache)
                .with_context(|| format!("reading {}", regen_cache.display()))?;
            let base_b = std::fs::read(&base_cache)
                .with_context(|| format!("reading {}", base_cache.display()))?;
            let n = module_count(&regen_b);
            let mini =
                gore_as::cache::splice::extract_module(&regen_b, &module).context("extract")?;
            let (remapped, counts) = gore_as::cache::remap::remap_module_to_base_with_options(
                &mini,
                &base_b,
                gore_as::cache::remap::RemapOptions { allow_new_symbols },
            )
            .context("remap")?;
            std::fs::write(&out, &remapped)
                .with_context(|| format!("writing {}", out.display()))?;
            println!(
                "extract-remap {:?} from {} modules -> remapped 1-module mini ; {} bytes ; wrote {}",
                module,
                n,
                remapped.len(),
                out.display()
            );
            println!(
                "refs remapped: {} total (bytecode: global={} func_ptr={} type_ptr={} func_id={} type_id={} ; embedded: type_ptr={} func_id={})",
                counts.total(),
                counts.global_ptr,
                counts.func_ptr,
                counts.type_ptr,
                counts.func_id,
                counts.type_id,
                counts.embed_type_ptr,
                counts.embed_func_id
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

            let opts = NormOpts {
                n2_slots: norm_slots,
                n5_scope: !no_norm_scope,
                n6_reguard: !no_norm_reguard,
                ..Default::default()
            };
            let filters = Filters {
                module: module.clone(),
                func: func.clone(),
            };

            let report =
                bytediff::run(&v_bytes, &r_bytes, &opts, &filters, context).context("bytediff")?;

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
                            println!(
                                "{}  IDENTICAL  (v={} ops, r={} ops)",
                                d.name, d.v_ops, d.r_ops
                            );
                        }
                    }
                    Verdict::Benign => {
                        let labels = d.fired.labels();
                        if show_benign {
                            println!(
                                "{}  BENIGN-DIFF  [{}]  (v={} ops, r={} ops)",
                                d.name,
                                labels.join(" "),
                                d.v_ops,
                                d.r_ops
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
                            esc(&d.name),
                            d.v_ops,
                            d.r_ops,
                            esc(hint)
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
