//! Compile a staged `.as` into a 1-module mini-cache by driving the game's precompiled-data
//! generation, then extracting (add) / extract-remapping (edit) the target module.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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
    /// Explicitly allow the edited/generated module to introduce symbols absent from the base.
    /// Default callers must pass `false`; the strict historical remap remains the safe default.
    pub allow_new_symbols: bool,
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

/// What happened to the live game installation around a compiler attempt.
///
/// This is deliberately separate from compiler success: a syntax error after the generator exited
/// can still have restored every live path exactly, while an otherwise useful compiler report may
/// require manual recovery when process termination or cleanup could not be proven.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallRestoreDisposition {
    /// The transactional generator runner was never entered (for example, module preflight failed).
    NotStarted,
    /// The generator is confirmed absent and every live path was restored to its pre-call state.
    RestoredExact,
    /// Generator exit could not be confirmed, so isolation and disk recovery artifacts were kept.
    RecoveryRequiredProcessExitUnconfirmed,
    /// Restore/finalization failed while disk recovery artifacts were still retained.
    RecoveryRequiredRestoreFailed,
}

/// Result of one compile-module attempt. A failure remains an ordinary [`CompileError`]; the
/// surrounding report separately preserves whether enhanced diagnostics were captured, fell back,
/// or were unavailable after the original process had already completed.
#[derive(Debug)]
pub enum CompileModuleReportOutcome {
    Compiled(CompileOutput),
    Failed(CompileError),
}

/// Structured companion to [`compile_module`].
///
/// `diagnostics` is `None` only when the operation failed before a game compiler process produced
/// a report (for example during source/base preflight or install transaction setup). Once the
/// compiler path starts, success and failure both retain a bounded report without parsing the
/// human-readable [`CompileError`] string.
#[derive(Debug)]
pub struct CompileModuleReport {
    pub outcome: CompileModuleReportOutcome,
    diagnostics: Option<crate::diagnostics::CompilerDiagnosticsReport>,
    install_restore: InstallRestoreDisposition,
}

impl CompileModuleReport {
    pub fn diagnostics(&self) -> Option<&crate::diagnostics::CompilerDiagnosticsReport> {
        self.diagnostics.as_ref()
    }

    pub fn install_restore_disposition(&self) -> InstallRestoreDisposition {
        self.install_restore
    }

    pub fn into_parts(
        self,
    ) -> (
        CompileModuleReportOutcome,
        Option<crate::diagnostics::CompilerDiagnosticsReport>,
    ) {
        (self.outcome, self.diagnostics)
    }
}

/// Compile one module through the transactional game compiler while retaining bounded structured
/// diagnostics and the exact capture/fallback disposition.
///
/// The existing [`compile_module`] API remains the injectable compatibility primitive. This
/// higher-level production entry point uses the same default diagnostics options and never derives
/// structured messages by reparsing an error string.
pub fn compile_module_with_diagnostics_report(
    opts: &CompileOpts,
    diagnostics: &crate::diagnostics::DiagnosticsOptions,
) -> CompileModuleReport {
    compile_module_report_with(opts, |game_dir, source_tree| {
        game_run_regen_with_extended_diagnostics_report(game_dir, source_tree, diagnostics)
    })
}

fn compile_module_report_with<R>(opts: &CompileOpts, run_regen: R) -> CompileModuleReport
where
    R: Fn(&Path, &Path) -> Result<GameRunRegenExtendedReport, String>,
{
    let report = std::cell::RefCell::new(None);
    let result = compile_module(opts, |game_dir, source_tree| {
        let generated = run_regen(game_dir, source_tree)?;
        let mut slot = report.borrow_mut();
        if slot.is_some() {
            return Err("compile-module diagnostics runner was invoked more than once".to_owned());
        }
        let result = generated.result;
        *slot = Some((generated.diagnostics, generated.install_restore));
        result
    });
    let generated = report.into_inner();
    let install_restore = generated
        .as_ref()
        .map(|(_, install_restore)| *install_restore)
        .unwrap_or(InstallRestoreDisposition::NotStarted);
    CompileModuleReport {
        outcome: match result {
            Ok(output) => CompileModuleReportOutcome::Compiled(output),
            Err(error) => CompileModuleReportOutcome::Failed(error),
        },
        diagnostics: generated.and_then(|(diagnostics, _)| diagnostics),
        install_restore,
    }
}

/// Return the compiler-generated class methods that the source emitter deliberately omits.
/// Replacing an existing module without carrying these records forward would silently erase CDO
/// defaults (NPC/quest/dialog configuration among them), so `edit` must fail closed until the
/// records can be preserved byte-for-byte. `PreparedEmit::prepare_compile_overlay` has already
/// proved that `module_name` identifies exactly one base module before this helper is called.
fn omitted_generated_methods(
    mods: &[model::Module],
    module_name: &str,
) -> Result<Vec<String>, CompileError> {
    let matches = mods
        .iter()
        .filter(|module| module.name == module_name)
        .collect::<Vec<_>>();
    let [module] = matches.as_slice() else {
        return Err(CompileError::Other(format!(
            "cannot inventory compiler-generated methods for edit module {module_name:?}: \
             expected exactly one base module, found {}",
            matches.len()
        )));
    };

    // A generated method is identified by class + method name. Refuse malformed/ambiguous class
    // identities even though PreparedEmit normally rejects the surrounding edit first; this
    // helper must never turn ambiguity into an empty inventory if its call order changes later.
    let mut class_names = std::collections::HashSet::new();
    let mut omitted = Vec::new();
    for class in &module.classes {
        if !class_names.insert(class.name.as_str()) {
            return Err(CompileError::Other(format!(
                "cannot inventory compiler-generated methods for edit module {module_name:?}: \
                 duplicate class identity {:?}",
                class.name
            )));
        }
        let mut generated_names = std::collections::HashSet::new();
        for method in &class.methods {
            if !method.name.starts_with("__") {
                continue;
            }
            if !generated_names.insert(method.name.as_str()) {
                return Err(CompileError::Other(format!(
                    "cannot inventory compiler-generated methods for edit module \
                     {module_name:?}: duplicate generated method identity {}::{}",
                    class.name, method.name
                )));
            }
            omitted.push(format!("{}::{}", class.name, method.name));
        }
    }
    Ok(omitted)
}

fn prepare_generated_defaults_edit(
    op: &str,
    mods: &[model::Module],
    module_name: &str,
    base: &[u8],
    overlay: &str,
    allow_new_symbols: bool,
) -> Result<Option<crate::cache::generated_defaults::GeneratedDefaultsPlan>, CompileError> {
    if op != "edit" {
        return Ok(None);
    }
    let omitted = omitted_generated_methods(mods, module_name)?;
    if omitted.is_empty() {
        return Ok(None);
    }
    let preview = omitted
        .iter()
        .take(4)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let more = omitted.len().saturating_sub(4);
    let suffix = if more == 0 {
        String::new()
    } else {
        format!(", and {more} more")
    };
    let refusal = |reason: &str| {
        CompileError::Other(format!(
            "refusing to edit module {module_name:?}: it contains {} compiler-generated `__*` \
             method(s) omitted by source emission ({preview}{suffix}); {reason}",
            omitted.len()
        ))
    };
    if allow_new_symbols {
        return Err(refusal(
            "generated-default carry requires strict base-keyspace remap; disable \
             --allow-new-symbols or use `add` for a new module",
        ));
    }
    if source_contains_default_token(overlay).map_err(|reason| refusal(&reason))? {
        return Err(refusal(
            "the authored overlay contains a `default` code token, so carrying old defaults \
             would be stale; remove the authored defaults or use a new module",
        ));
    }
    let plan =
        crate::cache::generated_defaults::GeneratedDefaultsPlan::prepare(base, mods, module_name)
            .map_err(|reason| {
                refusal(&format!(
                    "exact generated-default carry is unproven: {reason}"
                ))
            })?
            .ok_or_else(|| {
                refusal(
                    "the raw base module did not contain the generated methods found by the model",
                )
            })?;
    if plan.generated_count() != omitted.len() {
        return Err(refusal(&format!(
            "raw/model generated-method inventory mismatch ({}/{})",
            plan.generated_count(),
            omitted.len()
        )));
    }
    Ok(Some(plan))
}

/// Find a real `default` token while ignoring comments and quoted literals. A malformed lexical
/// construct is an error, not an excuse to launch the compiler without proving the overlay safe.
fn source_contains_default_token(source: &str) -> Result<bool, String> {
    let bytes = source.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while index < bytes.len() && !matches!(bytes[index], b'\r' | b'\n') {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index += 2;
                let mut closed = false;
                while index < bytes.len() {
                    if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                        index += 2;
                        closed = true;
                        break;
                    } else {
                        index += 1;
                    }
                }
                if !closed {
                    return Err(
                        "authored overlay has an unterminated block comment before generated-default preflight"
                            .into(),
                    );
                }
            }
            quote @ (b'\'' | b'"') => {
                index += 1;
                let mut closed = false;
                while index < bytes.len() {
                    if bytes[index] == b'\\' {
                        index = (index + 2).min(bytes.len());
                    } else if bytes[index] == quote {
                        index += 1;
                        closed = true;
                        break;
                    } else {
                        index += 1;
                    }
                }
                if !closed {
                    return Err(
                        "authored overlay has an unterminated quoted literal before generated-default preflight"
                            .into(),
                    );
                }
            }
            byte if byte.is_ascii_alphabetic() || byte == b'_' => {
                let start = index;
                index += 1;
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
                {
                    index += 1;
                }
                if &bytes[start..index] == b"default"
                    && !default_token_is_switch_label(bytes, index)?
                {
                    return Ok(true);
                }
            }
            _ => index += 1,
        }
    }
    Ok(false)
}

/// `default:` is a normal switch label and does not author a CDO default. Skip trivia after the
/// token so `default /* comment */ :` is classified correctly; malformed comments fail closed.
fn default_token_is_switch_label(bytes: &[u8], mut index: usize) -> Result<bool, String> {
    loop {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if bytes.get(index..index + 2) == Some(b"//") {
            index += 2;
            while index < bytes.len() && !matches!(bytes[index], b'\r' | b'\n') {
                index += 1;
            }
            continue;
        }
        if bytes.get(index..index + 2) == Some(b"/*") {
            index += 2;
            let mut closed = false;
            while index < bytes.len() {
                if bytes.get(index..index + 2) == Some(b"*/") {
                    index += 2;
                    closed = true;
                    break;
                } else {
                    index += 1;
                }
            }
            if !closed {
                return Err(
                    "authored overlay has an unterminated block comment after a `default` token"
                        .into(),
                );
            }
            continue;
        }
        return Ok(bytes.get(index) == Some(&b':'));
    }
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

/// The install root containing `G1R/`. AngelScript writes `AS_JITTED_CODE` beside `G1R`, not
/// inside it, even when the process working directory is `G1R`.
fn game_root_dir(game_dir: &Path) -> PathBuf {
    if game_dir.file_name().is_some_and(|n| n == "G1R") {
        game_dir.parent().unwrap_or(game_dir).to_path_buf()
    } else {
        game_dir.to_path_buf()
    }
}

fn vanilla_cache(game_dir: &Path) -> PathBuf {
    g1r_dir(game_dir)
        .join("Script")
        .join("PrecompiledScript_Shipping.Cache")
}

/// The deploy backup path for a live cache: the live path with `.gore-bak` APPENDED to the full
/// filename (so `…Shipping.Cache` -> `…Shipping.Cache.gore-bak`). Mirrors gore-mod's `bak_path`;
/// built via `OsString::push` (NOT `with_extension`, which would clobber the `.Cache` extension).
fn deploy_bak_path(live: &Path) -> PathBuf {
    let mut s = live.as_os_str().to_os_string();
    s.push(".gore-bak");
    PathBuf::from(s)
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}

fn compile_bak_path(live: &Path) -> PathBuf {
    append_suffix(live, ".gore-compile-bak")
}

fn compile_lock_path(game_dir: &Path) -> PathBuf {
    game_root_dir(game_dir).join(".gore-as-compile.lock")
}

#[derive(Debug)]
struct CompileLock {
    path: PathBuf,
    active: bool,
}

impl CompileLock {
    fn acquire(game_dir: &Path) -> Result<Self, String> {
        let path = compile_lock_path(game_dir);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    format!(
                        "another AngelScript compile is active (lock exists: {}); if no compile is \
                         running, inspect and remove the stale lock manually",
                        path.display()
                    )
                } else {
                    format!("creating compile lock {}: {e}", path.display())
                }
            })?;
        let payload = format!("pid={}\n", std::process::id());
        if let Err(e) = file
            .write_all(payload.as_bytes())
            .and_then(|_| file.sync_all())
        {
            drop(file);
            let cleanup = std::fs::remove_file(&path).err();
            return Err(match cleanup {
                Some(cleanup) => format!(
                    "initializing compile lock {}: {e}; additionally failed to remove it: \
                     {cleanup}",
                    path.display()
                ),
                None => format!("initializing compile lock {}: {e}", path.display()),
            });
        }
        Ok(Self { path, active: true })
    }

    fn release(&mut self) -> Result<(), String> {
        if !self.active {
            return Ok(());
        }
        std::fs::remove_file(&self.path)
            .map_err(|e| format!("removing compile lock {}: {e}", self.path.display()))?;
        self.active = false;
        Ok(())
    }
}

impl Drop for CompileLock {
    fn drop(&mut self) {
        if self.active && std::fs::remove_file(&self.path).is_ok() {
            self.active = false;
        }
    }
}

#[derive(Debug)]
struct ShippingRecovery {
    path: PathBuf,
    active: bool,
}

impl ShippingRecovery {
    fn create(live: &Path, bytes: &[u8]) -> Result<Self, String> {
        let path = compile_bak_path(live);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    format!(
                        "compile backup already exists: {} (recover or remove it manually)",
                        path.display()
                    )
                } else {
                    format!("creating compile backup {}: {e}", path.display())
                }
            })?;
        if let Err(e) = file.write_all(bytes).and_then(|_| file.sync_all()) {
            drop(file);
            let cleanup = std::fs::remove_file(&path).err();
            return Err(match cleanup {
                Some(cleanup) => format!(
                    "initializing compile backup {}: {e}; additionally failed to remove the \
                     incomplete backup: {cleanup}",
                    path.display()
                ),
                None => format!("initializing compile backup {}: {e}", path.display()),
            });
        }
        drop(file);
        Ok(Self { path, active: true })
    }

    fn retire(&mut self) -> Result<(), String> {
        if !self.active {
            return Ok(());
        }
        std::fs::remove_file(&self.path)
            .map_err(|e| format!("removing compile backup {}: {e}", self.path.display()))?;
        self.active = false;
        Ok(())
    }
}

/// Create the user-requested persistent `.gore-bak` immediately before an in-place install.
/// Returns true only when this call created it, so a later failed install can remove the artifact.
fn validate_existing_deploy_backup(path: &Path, meta: &std::fs::Metadata) -> Result<(), String> {
    if !meta.is_file() || metadata_is_link_or_reparse(meta) {
        return Err(format!(
            "refusing existing deploy backup {} because it is not a regular non-reparse file",
            path.display()
        ));
    }
    Ok(())
}

fn create_deploy_backup_if_absent(live: &Path, bytes: &[u8]) -> Result<bool, String> {
    let path = deploy_bak_path(live);
    match std::fs::symlink_metadata(&path) {
        Ok(meta) => {
            validate_existing_deploy_backup(&path, &meta)?;
            return Ok(false);
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("inspecting deploy backup {}: {e}", path.display())),
    }
    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // Another actor created the reserved path after the first inspection. Never accept its
            // path type implicitly: re-inspect without following links before treating it as the
            // persistent recovery copy.
            let meta = std::fs::symlink_metadata(&path).map_err(|inspect| {
                format!(
                    "inspecting raced deploy backup {}: {inspect}",
                    path.display()
                )
            })?;
            validate_existing_deploy_backup(&path, &meta)?;
            return Ok(false);
        }
        Err(e) => return Err(format!("creating deploy backup {}: {e}", path.display())),
    };
    if let Err(e) = file.write_all(bytes).and_then(|_| file.sync_all()) {
        drop(file);
        let cleanup = std::fs::remove_file(&path).err();
        return Err(match cleanup {
            Some(cleanup) => format!(
                "initializing deploy backup {}: {e}; additionally failed to remove the incomplete \
                 backup: {cleanup}",
                path.display()
            ),
            None => format!("initializing deploy backup {}: {e}", path.display()),
        });
    }
    Ok(true)
}

/// Validate the whole generated container before any caller accepts or installs its bytes. A
/// header-only or module-only prefix is not sufficient: all declared modules and all seven global
/// tail tables must parse, and the final table must end exactly at EOF.
fn validate_generated_cache(bytes: &[u8]) -> Result<(), String> {
    let header = crate::cache::header::CacheHeader::parse(bytes)
        .map_err(|e| format!("invalid generated cache header: {e}"))?;
    if header.type_count == 0 {
        return Err("invalid generated cache: it declares zero modules".into());
    }
    let tail = crate::cache::walk_modules::module_region_end(bytes)
        .map_err(|e| format!("invalid generated cache modules: {e}"))?;
    let tables = crate::cache::tables::parse_tail_tables(bytes, tail)
        .map_err(|e| format!("invalid generated cache tail tables: {e}"))?;
    if tables.end != bytes.len() {
        return Err(format!(
            "invalid generated cache: tail tables end at {:#x}, but file length is {:#x}",
            tables.end,
            bytes.len()
        ));
    }
    Ok(())
}

/// Minis are intermediate module containers; every add/replace splice path publishes the base
/// cache's outer header, never the mini's. Normalize the per-regeneration FGuid anyway so identical
/// source/base inputs produce byte-identical mini artifacts across compiler runs.
fn canonicalize_mini_guid(mini: &mut [u8], base: &[u8]) -> Result<(), String> {
    const GUID_BYTES: usize = 16;
    if mini.len() < GUID_BYTES || base.len() < GUID_BYTES {
        return Err(format!(
            "cannot canonicalize mini FGuid: mini/base shorter than {GUID_BYTES} bytes ({}/{})",
            mini.len(),
            base.len()
        ));
    }
    mini[..GUID_BYTES].copy_from_slice(&base[..GUID_BYTES]);
    Ok(())
}

/// Recreate the fixed `work_dir/tree` child from scratch. Refuse links and containment surprises
/// before recursive deletion, so a hostile/stale tree cannot redirect cleanup outside work_dir.
fn reset_compile_tree(work_dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(work_dir)
        .map_err(|e| format!("creating compile work dir {}: {e}", work_dir.display()))?;
    let work_real = work_dir
        .canonicalize()
        .map_err(|e| format!("resolving compile work dir {}: {e}", work_dir.display()))?;
    let tree = work_dir.join("tree");

    match std::fs::symlink_metadata(&tree) {
        Ok(meta) => {
            if meta.file_type().is_symlink() || !meta.is_dir() {
                return Err(format!(
                    "refusing to clear compile tree {} because it is not a real directory",
                    tree.display()
                ));
            }
            let tree_real = tree
                .canonicalize()
                .map_err(|e| format!("resolving compile tree {}: {e}", tree.display()))?;
            if tree_real == work_real || !tree_real.starts_with(&work_real) {
                return Err(format!(
                    "refusing to clear compile tree {} outside work dir {}",
                    tree_real.display(),
                    work_real.display()
                ));
            }
            std::fs::remove_dir_all(&tree_real)
                .map_err(|e| format!("clearing compile tree {}: {e}", tree_real.display()))?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("inspecting compile tree {}: {e}", tree.display())),
    }

    std::fs::create_dir(&tree)
        .map_err(|e| format!("creating clean compile tree {}: {e}", tree.display()))?;
    let tree_real = tree
        .canonicalize()
        .map_err(|e| format!("resolving clean compile tree {}: {e}", tree.display()))?;
    if tree_real.parent() != Some(work_real.as_path()) {
        return Err(format!(
            "clean compile tree {} is not a direct child of work dir {}",
            tree_real.display(),
            work_real.display()
        ));
    }
    Ok(tree)
}

/// Snapshot a file that may legitimately be absent. Generation writes
/// `PrecompiledScript.Cache`, and a developer may already have one there; callers must put that
/// exact prior state back instead of leaving the newly generated development cache installed.
fn snapshot_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("reading {}: {e}", path.display())),
    }
}

fn remove_if_exists(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("removing stale {}: {e}", path.display())),
    }
}

/// Restore an optional file snapshot exactly: rewrite the old bytes when it existed, otherwise
/// remove whatever generation created.
fn restore_optional(path: &Path, saved: &Option<Vec<u8>>) -> Result<(), String> {
    match saved {
        Some(bytes) => {
            std::fs::write(path, bytes).map_err(|e| format!("restoring {}: {e}", path.display()))
        }
        None => remove_if_exists(path).map_err(|e| format!("restoring absent file: {e}")),
    }
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
        return Err(CompileError::Io(format!(
            "source .as not found: {}",
            opts.as_path.display()
        )));
    }
    // Read the overlay before clearing work_dir/tree. This also makes an input that intentionally
    // lives below that old tree safe: its bytes survive the clean rebuild, never its stale siblings.
    let overlay = std::fs::read(&opts.as_path).map_err(io("reading source .as"))?;
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

    let mut refs =
        RefResolver::build(&base).map_err(|e| CompileError::Other(format!("resolver: {e}")))?;
    let mods =
        model::parse_modules(&base).map_err(|e| CompileError::Other(format!("parse: {e}")))?;
    // Use the exact same resolver preparation as `as emit-all`. Class fields, method-shadow names,
    // and id-based free-function collision renames are all compile-significant; the old partial
    // setup produced 287 divergent vanilla files on the 1.0.3 cache before the authored overlay
    // was even considered.
    let prepared = emit_all::PreparedEmit::new(&mods, &mut refs, native_api(&base_path))
        .map_err(|error| CompileError::Other(format!("preparing base modules: {error}")))?;
    let overlay = std::str::from_utf8(&overlay)
        .map_err(|error| CompileError::Other(format!("source .as is not valid UTF-8: {error}")))?;
    let (overlay, overlay_rel_path) = prepared
        .prepare_compile_overlay(&opts.op, &opts.module_name, &opts.rel_path, overlay)
        .map_err(|error| CompileError::Other(format!("preparing authored overlay: {error}")))?;

    let generated_defaults = prepare_generated_defaults_edit(
        &opts.op,
        &mods,
        &opts.module_name,
        &base,
        &overlay,
        opts.allow_new_symbols,
    )?;

    // 1. Only after all base and authored target checks succeed, clear and rebuild the tree.
    let tree = reset_compile_tree(&opts.work_dir).map_err(CompileError::Other)?;
    prepared
        .emit_tree(&tree)
        .map_err(|e| CompileError::Other(format!("emit tree: {e}")))?;

    // 2. Overlay the user's .as at its rel path.
    let dst = tree.join(&overlay_rel_path);
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(io("mkdir overlay"))?;
    }
    std::fs::write(&dst, overlay.as_bytes()).map_err(io("overlay .as"))?;

    // 3. Drive the game to regenerate the precompiled cache from `tree`.
    let regen_path = run_regen(&opts.game_dir, &tree).map_err(CompileError::Regen)?;
    if !regen_path.exists() {
        return Err(CompileError::NoRegen(regen_path.display().to_string()));
    }
    let regen = std::fs::read(&regen_path).map_err(io("reading regen cache"))?;
    validate_generated_cache(&regen).map_err(CompileError::Other)?;

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
                let mut added = regen_names
                    .iter()
                    .filter(|n| !base_set.contains(n.as_str()));
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

    // 4. Extract + remap the target module against the vanilla base, for BOTH ops. Strict mode
    //    emits the historical empty-tail mini. Explicit new-symbol mode instead carries only the
    //    new rows that cannot resolve in vanilla; it never copies the regen's full global tables.
    //    Deploy still differs by op — gore-mod uses `splice_auto` for add and `replace_module` for
    //    edit — while both accept either minimal shape.
    let mut mini = {
        let out = splice::extract_module(&regen, &target)
            .map_err(|e| CompileError::Other(format!("extract: {e}")))?;
        remap::remap_module_to_base_with_options(
            &out,
            &base,
            remap::RemapOptions {
                allow_new_symbols: opts.allow_new_symbols,
            },
        )
        .map_err(|e| CompileError::Other(format!("remap: {e}")))?
        .0
    };
    if let Some(plan) = generated_defaults {
        mini = plan.apply(&mini).map_err(|reason| {
            CompileError::Other(format!(
                "refusing generated-default carry for edit module {:?}: {reason}",
                opts.module_name
            ))
        })?;
    }
    canonicalize_mini_guid(&mut mini, &base).map_err(CompileError::Other)?;

    let mini_path = opts.work_dir.join("module.cache");
    std::fs::write(&mini_path, &mini).map_err(io("writing mini"))?;
    Ok(CompileOutput {
        mini_path,
        module_name: target,
    })
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

#[derive(Clone, Copy, Debug)]
enum QuarantineKind {
    File,
    Directory,
}

/// One path whose exact pre-launch presence is restored after generation. Existing content is
/// moved with a same-volume rename; absent content is kept absent by removing anything the game
/// creates at that exact path.
#[derive(Debug)]
struct QuarantinedPath {
    original: PathBuf,
    backup: PathBuf,
    kind: QuarantineKind,
    existed: bool,
    active: bool,
}

impl QuarantinedPath {
    /// Preflight only. In particular, check the reserved backup before ANY path is moved so an
    /// interrupted earlier compile is never overwritten or mistaken for disposable output.
    fn plan(original: PathBuf, backup: PathBuf, kind: QuarantineKind) -> Result<Self, String> {
        match std::fs::symlink_metadata(&backup) {
            Ok(_) => {
                return Err(format!(
                    "compile quarantine backup already exists: {} (recover or remove it manually)",
                    backup.display()
                ));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("inspecting backup {}: {e}", backup.display())),
        }
        let existed = match std::fs::symlink_metadata(&original) {
            Ok(meta) => {
                let expected = match kind {
                    QuarantineKind::File => meta.is_file() && !meta.file_type().is_symlink(),
                    QuarantineKind::Directory => meta.is_dir() && !meta.file_type().is_symlink(),
                };
                if !expected {
                    return Err(format!(
                        "refusing to quarantine {} because it is not a real {}",
                        original.display(),
                        match kind {
                            QuarantineKind::File => "file",
                            QuarantineKind::Directory => "directory",
                        }
                    ));
                }
                true
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => return Err(format!("inspecting {}: {e}", original.display())),
        };
        Ok(Self {
            original,
            backup,
            kind,
            existed,
            active: false,
        })
    }

    fn activate(&mut self) -> Result<(), String> {
        if self.existed {
            std::fs::rename(&self.original, &self.backup).map_err(|e| {
                format!(
                    "quarantining {} as {}: {e}",
                    self.original.display(),
                    self.backup.display()
                )
            })?;
        }
        self.active = true;
        Ok(())
    }

    fn remove_generated_original(&self) -> Result<(), String> {
        let meta = match std::fs::symlink_metadata(&self.original) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(format!("inspecting {}: {e}", self.original.display())),
        };
        if meta.file_type().is_symlink() {
            return Err(format!(
                "refusing to remove unexpected symlink at generated path {}",
                self.original.display()
            ));
        }
        match self.kind {
            QuarantineKind::File if meta.is_file() => std::fs::remove_file(&self.original),
            QuarantineKind::Directory if meta.is_dir() => std::fs::remove_dir_all(&self.original),
            QuarantineKind::File => {
                return Err(format!(
                    "expected generated file at {}, found another path type",
                    self.original.display()
                ));
            }
            QuarantineKind::Directory => {
                return Err(format!(
                    "expected generated directory at {}, found another path type",
                    self.original.display()
                ));
            }
        }
        .map_err(|e| format!("removing generated {}: {e}", self.original.display()))
    }

    fn restore(&mut self) -> Result<(), String> {
        if !self.active {
            return Ok(());
        }
        // If removal fails, do not touch the backup: it remains the recoverable pre-call state.
        self.remove_generated_original()?;
        if self.existed {
            std::fs::rename(&self.backup, &self.original).map_err(|e| {
                format!(
                    "restoring {} from {}: {e}",
                    self.original.display(),
                    self.backup.display()
                )
            })?;
        }
        self.active = false;
        Ok(())
    }
}

impl Drop for QuarantinedPath {
    fn drop(&mut self) {
        // Explicit restoration remains the normal/reporting path. This is only the unwind safety
        // net after the owning transaction has already had a chance to retain recovery artifacts.
        let _ = self.restore();
    }
}

/// Generation isolation for the two known non-cache side effects:
/// - `<install>/AS_JITTED_CODE`, written by AngelScript;
/// - the UE4SS `dwmapi.dll` loader proxy, temporarily moved so generation runs without hooks.
struct GenerationIsolation {
    jitted: QuarantinedPath,
    proxy: Option<QuarantinedPath>,
}

impl GenerationIsolation {
    /// Plan both quarantines without mutating either path. The transaction stores this owner before
    /// activation, so even a partial activation remains visible to its reporting restore path.
    fn plan(game_dir: &Path, g1r: &Path) -> Result<Self, String> {
        let jitted = game_root_dir(game_dir).join("AS_JITTED_CODE");
        let jitted = QuarantinedPath::plan(
            jitted.clone(),
            append_suffix(&jitted, ".gore-compile-bak"),
            QuarantineKind::Directory,
        )?;

        let win64 = g1r.join("Binaries").join("Win64");
        let proxy_path = win64.join("dwmapi.dll");
        let proxy_exists = match std::fs::symlink_metadata(&proxy_path) {
            Ok(_) => true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => return Err(format!("inspecting {}: {e}", proxy_path.display())),
        };
        let ue4ss_payload = win64.join("ue4ss").join("UE4SS.dll");
        let clearly_ue4ss = ue4ss_payload.is_file();
        // A present dwmapi.dll without the UE4SS payload is not clearly the local proxy; leave it
        // entirely alone. An absent path is still tracked so it remains absent after generation.
        let proxy = if !proxy_exists || clearly_ue4ss {
            Some(QuarantinedPath::plan(
                proxy_path.clone(),
                append_suffix(&proxy_path, ".gore-compile-bak"),
                QuarantineKind::File,
            )?)
        } else {
            None
        };

        Ok(Self { jitted, proxy })
    }

    fn activate(&mut self) -> Result<(), String> {
        self.activate_after_jitted(|| {})
    }

    fn activate_after_jitted<F>(&mut self, after_jitted: F) -> Result<(), String>
    where
        F: FnOnce(),
    {
        // Both plans (including collision checks) completed before this first rename. Do not
        // locally roll back a partial activation: the transaction owns `self` and must report any
        // failed restoration before deciding whether its journal/backups can be retired.
        self.jitted.activate()?;
        after_jitted();
        if let Some(proxy) = &mut self.proxy {
            proxy.activate()?;
        }
        Ok(())
    }

    fn restore(&mut self) -> Result<(), String> {
        let mut errors = Vec::new();
        if let Some(proxy) = &mut self.proxy {
            if let Err(e) = proxy.restore() {
                errors.push(e);
            }
        }
        if let Err(e) = self.jitted.restore() {
            errors.push(e);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }
}

impl Drop for GenerationIsolation {
    fn drop(&mut self) {
        // Normal paths call restore explicitly so errors are reported. This fallback covers a
        // panic/unwind and deliberately leaves any un-restorable backups in place.
        let _ = self.restore();
    }
}

struct RestoreReport {
    errors: Vec<String>,
    shipping_restored: bool,
}

impl RestoreReport {
    fn clean(&self) -> bool {
        self.errors.is_empty()
    }
}

fn recovery_journal_path(game_dir: &Path) -> PathBuf {
    game_root_dir(game_dir).join(".gore-as-compile-recovery")
}

/// Disk-backed copies of every in-memory snapshot needed to recover an intentionally-preserved
/// transaction after a generator process could not be confirmed dead. The mirrored layout is
/// deliberately human-readable: `overwritten/` files copy back into `G1R/Script/`, paths mirrored
/// under `created/` must be deleted, and `development-cache/` records the pre-call dev-cache state.
struct RecoveryJournal {
    root: PathBuf,
    active: bool,
}

impl RecoveryJournal {
    fn create(game_dir: &Path, saved_dev: &Option<Vec<u8>>) -> Result<Self, String> {
        let root = recovery_journal_path(game_dir);
        match std::fs::create_dir(&root) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(format!(
                    "compile recovery journal already exists: {} (recover the previous compile \
                     before retrying)",
                    root.display()
                ));
            }
            Err(e) => return Err(format!("creating recovery journal {}: {e}", root.display())),
        }

        let initialize = (|| -> Result<(), String> {
            let instructions = b"GORE AngelScript compile recovery\n\
Kill the reported generator process tree before restoring anything.\n\
Copy files from overwritten/ over the same relative paths under G1R/Script/.\n\
Delete the same relative paths listed as zero-byte files under created/.\n\
development-cache/PrecompiledScript.Cache is the pre-call dev cache;\n\
development-cache.absent means that cache did not exist.\n\
Restore *.gore-compile-bak paths beside their originals, then remove the compile lock.\n";
            std::fs::write(root.join("README.txt"), instructions)
                .map_err(|e| format!("writing recovery instructions: {e}"))?;
            match saved_dev {
                Some(bytes) => {
                    let dev_dir = root.join("development-cache");
                    std::fs::create_dir(&dev_dir)
                        .map_err(|e| format!("creating dev-cache recovery directory: {e}"))?;
                    std::fs::write(dev_dir.join("PrecompiledScript.Cache"), bytes)
                        .map_err(|e| format!("writing dev-cache recovery snapshot: {e}"))?;
                }
                None => {
                    std::fs::write(root.join("development-cache.absent"), b"")
                        .map_err(|e| format!("writing dev-cache absence marker: {e}"))?;
                }
            }
            Ok(())
        })();

        if let Err(error) = initialize {
            let cleanup = std::fs::remove_dir_all(&root).err();
            return Err(match cleanup {
                Some(cleanup) => format!(
                    "{error}; additionally failed to remove partial recovery journal {}: \
                     {cleanup}",
                    root.display()
                ),
                None => error,
            });
        }
        Ok(Self { root, active: true })
    }

    fn record_staged(
        &self,
        staged: &[(PathBuf, Option<Vec<u8>>)],
        script_dir: &Path,
    ) -> Result<(), String> {
        for (path, prior) in staged {
            let rel = path.strip_prefix(script_dir).map_err(|e| {
                format!(
                    "staged recovery path {} escaped Script/ {}: {e}",
                    path.display(),
                    script_dir.display()
                )
            })?;
            let bucket = if prior.is_some() {
                "overwritten"
            } else {
                "created"
            };
            let recovery = self.root.join(bucket).join(rel);
            if let Some(parent) = recovery.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    format!("creating recovery directory {}: {e}", parent.display())
                })?;
            }
            std::fs::write(&recovery, prior.as_deref().unwrap_or_default()).map_err(|e| {
                format!(
                    "writing staged recovery snapshot {}: {e}",
                    recovery.display()
                )
            })?;
        }
        Ok(())
    }

    fn retire(&mut self) -> Result<(), String> {
        if !self.active {
            return Ok(());
        }
        std::fs::remove_dir_all(&self.root)
            .map_err(|e| format!("removing recovery journal {}: {e}", self.root.display()))?;
        self.active = false;
        Ok(())
    }
}

/// Owns every live-install mutation made by either compile entry point. Its Drop implementation is
/// the panic/unwind safety net; normal paths call the same restore methods so cleanup errors remain
/// visible to the caller.
struct CompileTransaction {
    game_dir: PathBuf,
    g1r: PathBuf,
    script_dir: PathBuf,
    shipping_cache: PathBuf,
    dev_cache: PathBuf,
    saved_shipping: Vec<u8>,
    saved_dev: Option<Vec<u8>>,
    staged: Vec<(PathBuf, Option<Vec<u8>>)>,
    isolation: Option<GenerationIsolation>,
    recovery: ShippingRecovery,
    journal: RecoveryJournal,
    lock: CompileLock,
    rollback_needed: bool,
    /// A user-facing `.gore-bak` created immediately before install, removed unless install commits.
    ephemeral_deploy_backup: Option<PathBuf>,
}

impl CompileTransaction {
    /// Acquire the cross-entry-point lock first; the recovery backup is the next and only mutation
    /// before the fully-owned transaction exists.
    fn begin(game_dir: &Path, g1r: &Path, script_dir: &Path) -> Result<Self, String> {
        let mut lock = CompileLock::acquire(game_dir)?;
        let shipping_cache = script_dir.join("PrecompiledScript_Shipping.Cache");
        let dev_cache = script_dir.join("PrecompiledScript.Cache");
        let saved_shipping = std::fs::read(&shipping_cache).map_err(|e| {
            format!(
                "reading live shipping cache {}: {e}",
                shipping_cache.display()
            )
        })?;
        let saved_dev = snapshot_optional(&dev_cache)?;
        let mut recovery = ShippingRecovery::create(&shipping_cache, &saved_shipping)?;
        let journal = match RecoveryJournal::create(game_dir, &saved_dev) {
            Ok(journal) => journal,
            Err(error) => {
                let recovery_error = recovery.retire().err();
                let lock_error = lock.release().err();
                let mut errors = vec![error];
                errors.extend(recovery_error);
                errors.extend(lock_error);
                return Err(errors.join("; additionally "));
            }
        };
        Ok(Self {
            game_dir: game_dir.to_path_buf(),
            g1r: g1r.to_path_buf(),
            script_dir: script_dir.to_path_buf(),
            shipping_cache,
            dev_cache,
            saved_shipping,
            saved_dev,
            staged: Vec::new(),
            isolation: None,
            recovery,
            journal,
            lock,
            rollback_needed: true,
            ephemeral_deploy_backup: None,
        })
    }

    fn begin_isolation(&mut self) -> Result<(), String> {
        if self.isolation.is_none() {
            self.isolation = Some(GenerationIsolation::plan(&self.game_dir, &self.g1r)?);
        }
        self.isolation
            .as_mut()
            .expect("generation isolation was planned above")
            .activate()
    }

    #[cfg(test)]
    fn begin_isolation_after_jitted<F>(&mut self, after_jitted: F) -> Result<(), String>
    where
        F: FnOnce(),
    {
        if self.isolation.is_none() {
            self.isolation = Some(GenerationIsolation::plan(&self.game_dir, &self.g1r)?);
        }
        self.isolation
            .as_mut()
            .expect("generation isolation was planned above")
            .activate_after_jitted(after_jitted)
    }

    fn stage(&mut self, src: &Path) -> Result<(), String> {
        copy_tree(src, &self.script_dir, &mut self.staged)
            .map_err(|e| format!("staging source tree: {e}"))?;
        self.journal.record_staged(&self.staged, &self.script_dir)
    }

    /// Restore all live paths. A completely clean restore disarms the rollback portion of Drop,
    /// while recovery-backup retirement and lock release remain explicit finalization steps.
    fn restore_install(&mut self) -> RestoreReport {
        if !self.rollback_needed {
            return RestoreReport {
                errors: Vec::new(),
                shipping_restored: true,
            };
        }
        let mut errors = Vec::new();
        if let Some(isolation) = &mut self.isolation {
            if let Err(e) = isolation.restore() {
                errors.push(format!("failed to restore generation isolation: {e}"));
            }
        }
        if !self.staged.is_empty() {
            match restore_or_remove(&self.staged, &self.script_dir) {
                Ok(()) => self.staged.clear(),
                Err(e) => errors.push(format!("failed to clean staged sources: {e}")),
            }
        }
        let shipping_restored = match std::fs::write(&self.shipping_cache, &self.saved_shipping) {
            Ok(()) => true,
            Err(e) => {
                errors.push(format!(
                    "FAILED to restore the live shipping cache ({e}); restore it from {}",
                    self.recovery.path.display()
                ));
                false
            }
        };
        if let Err(e) = restore_optional(&self.dev_cache, &self.saved_dev) {
            errors.push(format!("failed to restore development cache: {e}"));
        }
        if errors.is_empty() {
            self.rollback_needed = false;
        }
        RestoreReport {
            errors,
            shipping_restored,
        }
    }

    /// Call immediately before an intentional in-place Shipping write, so a panic or partial write
    /// re-arms Drop's rollback behavior.
    fn arm_install_rollback(&mut self) {
        self.rollback_needed = true;
    }

    fn prepare_deploy_backup(&mut self, enabled: bool) -> Result<(), String> {
        if enabled && create_deploy_backup_if_absent(&self.shipping_cache, &self.saved_shipping)? {
            self.ephemeral_deploy_backup = Some(deploy_bak_path(&self.shipping_cache));
        }
        Ok(())
    }

    fn remove_ephemeral_deploy_backup(&mut self) -> Result<(), String> {
        let Some(path) = self.ephemeral_deploy_backup.take() else {
            return Ok(());
        };
        if let Err(e) = std::fs::remove_file(&path) {
            self.ephemeral_deploy_backup = Some(path.clone());
            return Err(format!(
                "removing deploy backup created by failed compile {}: {e}",
                path.display()
            ));
        }
        Ok(())
    }

    fn mark_install_committed(&mut self) {
        self.rollback_needed = false;
        // The requested deploy backup is now persistent rather than transactional.
        self.ephemeral_deploy_backup = None;
    }

    /// Retire recovery only after Shipping was restored/committed, then release the common lock.
    fn finish(&mut self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.rollback_needed {
            errors.push("internal error: compile transaction finalized before restore".into());
            return errors;
        }
        if let Err(e) = self.journal.retire() {
            errors.push(e);
            return errors; // keep Shipping recovery + lock while the journal remains
        }
        if let Err(e) = self.recovery.retire() {
            errors.push(e);
            return errors; // keep the lock until Drop retries retirement
        }
        if let Err(e) = self.lock.release() {
            errors.push(e);
        }
        errors
    }

    fn recovery_retained(&self) -> bool {
        self.journal.active || self.recovery.active
    }

    /// A confirmed-dead generator may release the compile lock after restore/finalization failed,
    /// but the recovery artifacts must remain exactly as reported to the caller. Do not let Drop
    /// turn a structured `RecoveryRequiredRestoreFailed` into a transient, unobservable retry.
    fn preserve_for_restore_failure(mut self) {
        let _ = self.remove_ephemeral_deploy_backup();
        let _ = self.lock.release();
        std::mem::forget(self);
    }

    /// A generator process that might still be alive must retain exclusive ownership of every
    /// path it can touch. Deliberately leak the transaction guards so Drop cannot race that process
    /// by restoring Script/JIT/proxy state or releasing the compile lock. The disk recovery backup
    /// and quarantine paths make the pre-call state recoverable after the process is killed.
    fn preserve_for_unconfirmed_generator(self, cause: String) -> String {
        let recovery = self.recovery.path.display().to_string();
        let journal = self.journal.root.display().to_string();
        let lock = self.lock.path.display().to_string();
        let game_root = game_root_dir(&self.game_dir).display().to_string();
        std::mem::forget(self);
        format!(
            "{cause}; cleanup was intentionally NOT run because the generator's exit could not be \
             confirmed. Kill the reported process tree before recovery; the Shipping recovery \
             cache is {recovery}, the source/dev recovery journal is {journal}, quarantined side \
             effects are beside their originals under {game_root}, and the compile lock remains \
             at {lock}"
        )
    }
}

impl Drop for CompileTransaction {
    fn drop(&mut self) {
        let mut restore_failed = false;
        if self.rollback_needed {
            let report = self.restore_install();
            // If Shipping itself could not be restored, preserve the recovery backup. Other
            // cleanup failures also keep it as the conservative crash-recovery artifact.
            if !report.clean() || !report.shipping_restored {
                restore_failed = true;
            }
        }
        let _ = self.remove_ephemeral_deploy_backup();
        if restore_failed {
            let _ = self.lock.release();
            return;
        }
        if self.journal.retire().is_ok() && self.recovery.retire().is_ok() {
            let _ = self.lock.release();
        }
    }
}

/// The real game launch. Places the loose `.as` tree where the game reads it, launches the
/// shipping exe in AngelScript development/generation mode, waits for the generated development
/// cache (`PrecompiledScript.Cache`), and returns a workspace copy of that cache.
///
/// Compiling normally leaves the install unchanged: on every confirmed-process exit path this restores both
/// `PrecompiledScript_Shipping.Cache` and the optional pre-existing `PrecompiledScript.Cache` to
/// their exact pre-call states, then undoes every staged source file. If generator termination
/// cannot be confirmed, isolation and the lock intentionally remain in place for manual recovery.
pub fn game_run_regen(game_dir: &Path, src_dir: &Path) -> Result<PathBuf, String> {
    game_run_regen_with_diagnostics(game_dir, src_dir, &Default::default())
}

/// Transactional generator result paired with the bounded diagnostics report produced after the
/// compiler process started. Setup failures that occur before a process/report exists remain the
/// outer `Err` of [`game_run_regen_with_diagnostics_report`].
#[derive(Debug)]
pub struct GameRunRegenReport {
    result: Result<PathBuf, String>,
    diagnostics: crate::diagnostics::CompilerDiagnosticsReport,
    install_restore: InstallRestoreDisposition,
}

impl GameRunRegenReport {
    pub fn result(&self) -> Result<&Path, &str> {
        self.result.as_deref().map_err(String::as_str)
    }

    pub fn diagnostics(&self) -> &crate::diagnostics::CompilerDiagnosticsReport {
        &self.diagnostics
    }

    pub fn install_restore_disposition(&self) -> InstallRestoreDisposition {
        self.install_restore
    }

    pub fn into_parts(
        self,
    ) -> (
        Result<PathBuf, String>,
        crate::diagnostics::CompilerDiagnosticsReport,
    ) {
        (self.result, self.diagnostics)
    }
}

/// Internal superset used by module compilation. Unlike the public compatibility report, this can
/// represent a transactional setup/restore failure before the diagnostics runner was reached.
#[derive(Debug)]
struct GameRunRegenExtendedReport {
    result: Result<PathBuf, String>,
    diagnostics: Option<crate::diagnostics::CompilerDiagnosticsReport>,
    install_restore: InstallRestoreDisposition,
}

/// Same transactional compiler path as [`game_run_regen`], with explicit diagnostics discovery /
/// opt-out settings. The helper is temporary and optional; generator availability never depends on
/// it.
pub fn game_run_regen_with_diagnostics(
    game_dir: &Path,
    src_dir: &Path,
    diagnostics: &crate::diagnostics::DiagnosticsOptions,
) -> Result<PathBuf, String> {
    match game_run_regen_with_diagnostics_report(game_dir, src_dir, diagnostics) {
        Ok(report) => report.result,
        Err(error) => Err(error),
    }
}

/// Same transactional install-restoring compiler path as [`game_run_regen_with_diagnostics`], but
/// preserve the structured capture disposition and messages without deriving them from stderr or
/// a formatted error string.
pub fn game_run_regen_with_diagnostics_report(
    game_dir: &Path,
    src_dir: &Path,
    diagnostics: &crate::diagnostics::DiagnosticsOptions,
) -> Result<GameRunRegenReport, String> {
    let extended = game_run_regen_with_extended_diagnostics_report(game_dir, src_dir, diagnostics)?;
    let GameRunRegenExtendedReport {
        result,
        diagnostics,
        install_restore,
    } = extended;
    let Some(diagnostics) = diagnostics else {
        return Err(result.err().unwrap_or_else(|| {
            "game compiler completed without producing its diagnostics disposition".to_owned()
        }));
    };
    Ok(GameRunRegenReport {
        result,
        diagnostics,
        install_restore,
    })
}

fn game_run_regen_with_extended_diagnostics_report(
    game_dir: &Path,
    src_dir: &Path,
    diagnostics: &crate::diagnostics::DiagnosticsOptions,
) -> Result<GameRunRegenExtendedReport, String> {
    let diagnostic_report = std::cell::RefCell::new(None);
    let generated = game_run_regen_with_install_report(game_dir, src_dir, |exe, g1r, cache| {
        let generated = real_generate_with_timeout_and_diagnostics_report(
            exe,
            g1r,
            cache,
            Duration::from_secs(30 * 60),
            diagnostics,
        );
        *diagnostic_report.borrow_mut() = Some(generated.diagnostics);
        GeneratorRunResult {
            result: generated.result,
            process_exit: generated.process_exit,
        }
    })?;
    Ok(GameRunRegenExtendedReport {
        result: generated.result,
        diagnostics: diagnostic_report.into_inner(),
        install_restore: generated.install_restore,
    })
}

/// Testable core of [`game_run_regen`]. `generate` receives the executable, G1R directory, and
/// the *development* cache path. It must return the bytes generated there.
#[cfg(test)]
fn game_run_regen_with<G>(game_dir: &Path, src_dir: &Path, generate: G) -> Result<PathBuf, String>
where
    G: FnOnce(&Path, &Path, &Path) -> Result<Vec<u8>, String>,
{
    game_run_regen_with_install_report(game_dir, src_dir, |exe, g1r, cache| {
        let result = generate(exe, g1r, cache);
        let process_exit = if result
            .as_ref()
            .err()
            .is_some_and(|error| generator_exit_unconfirmed(error))
        {
            GeneratorProcessExitDisposition::Unconfirmed
        } else {
            GeneratorProcessExitDisposition::Confirmed
        };
        GeneratorRunResult {
            result,
            process_exit,
        }
    })
    .and_then(|report| report.result)
}

#[derive(Debug)]
struct GameRunInstallReport {
    result: Result<PathBuf, String>,
    install_restore: InstallRestoreDisposition,
}

fn game_run_regen_with_install_report<G>(
    game_dir: &Path,
    src_dir: &Path,
    generate: G,
) -> Result<GameRunInstallReport, String>
where
    G: FnOnce(&Path, &Path, &Path) -> GeneratorRunResult<Vec<u8>>,
{
    game_run_regen_with_install_report_and(
        game_dir,
        src_dir,
        CompileTransaction::begin_isolation,
        generate,
    )
}

fn game_run_regen_with_install_report_and<I, G>(
    game_dir: &Path,
    src_dir: &Path,
    begin_isolation: I,
    generate: G,
) -> Result<GameRunInstallReport, String>
where
    I: FnOnce(&mut CompileTransaction) -> Result<(), String>,
    G: FnOnce(&Path, &Path, &Path) -> GeneratorRunResult<Vec<u8>>,
{
    let g1r = g1r_dir(game_dir);
    let exe = g1r
        .join("Binaries")
        .join("Win64")
        .join("G1R-Win64-Shipping.exe");
    if !exe.exists() {
        return Err(format!("game exe not found: {}", exe.display()));
    }
    let script_dir = g1r.join("Script");
    let dev_cache = script_dir.join("PrecompiledScript.Cache");

    // Existing loose scripts whose relative path is absent from our complete staged tree would be
    // compiled too but never appear in `written`, silently contaminating the regen. Matching paths
    // are safe: copy_tree snapshots and restores their exact bytes.
    if let Some(stray) = first_uncovered_loose_script(&script_dir, src_dir)
        .map_err(|e| format!("inspecting the game's Script/ tree: {e}"))?
    {
        return Err(format!(
            "the game's Script/ directory contains a loose script not present in the staged tree \
             ({}); refusing a contaminated compile",
            stray.display()
        ));
    }

    let mut txn = CompileTransaction::begin(game_dir, &g1r, &script_dir)?;
    let regen_out = src_dir.join("regen.cache");
    let _ = std::fs::remove_file(&regen_out);
    let mut process_exit = GeneratorProcessExitDisposition::NotStarted;
    let result = (|| -> Result<PathBuf, String> {
        // Quarantine process-wide side effects before staging or deleting either cache. From this
        // point onward CompileTransaction::drop can roll back an unwind at any instruction.
        begin_isolation(&mut txn)?;
        txn.stage(src_dir)?;
        // A source tree may accidentally contain this filename, so remove the stale/staged dev
        // cache immediately before launch. The saved pre-call state is restored below.
        remove_if_exists(&txn.dev_cache)?;
        let generated = generate(&exe, &g1r, &dev_cache);
        process_exit = generated.process_exit;
        let regen = generated.result?;
        if regen.is_empty() {
            return Err("the game produced an empty PrecompiledScript.Cache".into());
        }
        validate_generated_cache(&regen)?;
        std::fs::write(&regen_out, &regen).map_err(|e| format!("writing regen copy: {e}"))?;
        Ok(regen_out.clone())
    })();

    if process_exit == GeneratorProcessExitDisposition::Unconfirmed {
        let error = result
            .err()
            .unwrap_or_else(|| "generator exit was unconfirmed after reporting success".to_owned());
        return Ok(GameRunInstallReport {
            result: Err(txn.preserve_for_unconfirmed_generator(error)),
            install_restore: InstallRestoreDisposition::RecoveryRequiredProcessExitUnconfirmed,
        });
    }

    // Undo staged files first: they may include either cache filename. Explicit cache restoration
    // then wins and guarantees the exact snapshots even if the generator unexpectedly touched the
    // shipping cache too.
    let report = txn.restore_install();
    let mut cleanup_errors = report.errors;
    if cleanup_errors.is_empty() {
        cleanup_errors.extend(txn.finish());
    }

    let result = match result {
        Ok(p) if cleanup_errors.is_empty() => Ok(p),
        Ok(p) => Err(format!(
            "compiled to {}, but {}",
            p.display(),
            cleanup_errors.join("; ")
        )),
        Err(e) if cleanup_errors.is_empty() => Err(e),
        Err(e) => Err(format!("{e}; additionally {}", cleanup_errors.join("; "))),
    };
    if txn.recovery_retained() {
        txn.preserve_for_restore_failure();
        return Ok(GameRunInstallReport {
            result,
            install_restore: InstallRestoreDisposition::RecoveryRequiredRestoreFailed,
        });
    }
    Ok(GameRunInstallReport {
        result,
        install_restore: InstallRestoreDisposition::RestoredExact,
    })
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
    precompile_with_generator_report(opts, |exe, g1r, cache| {
        real_generate_report(exe, g1r, cache, &Default::default())
    })
}

/// [`precompile`] with explicit optional compiler-diagnostic capture settings.
pub fn precompile_with_diagnostics(
    opts: &PrecompileOpts,
    diagnostics: &crate::diagnostics::DiagnosticsOptions,
) -> Result<PathBuf, String> {
    precompile_with_generator_report(opts, |exe, g1r, cache| {
        real_generate_report(exe, g1r, cache, diagnostics)
    })
}

/// The first loose `.as` file found anywhere under `dir` (recursively), or `None`. Used to reject a
/// dirty Script/ before staging a SRC tree, so the game never compiles leftover scripts alongside it.
#[cfg(test)]
fn first_loose_script(dir: &Path) -> std::io::Result<Option<PathBuf>> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path)?;
        if metadata_is_link_or_reparse(&meta) {
            return Err(std::io::Error::other(format!(
                "refusing linked/reparse path while scanning Script/: {}",
                path.display()
            )));
        }
        if meta.is_dir() {
            if let Some(found) = first_loose_script(&path)? {
                return Ok(Some(found));
            }
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("as"))
        {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

/// First loose `.as` under `script_dir` whose relative path is not a real file in `src_dir`.
/// Used by the full-tree compile backend: colliding files are safe because staging records/restores
/// them, while an uncovered extra file would be compiled silently alongside the requested tree.
fn first_uncovered_loose_script(
    script_dir: &Path,
    src_dir: &Path,
) -> std::io::Result<Option<PathBuf>> {
    fn walk(dir: &Path, root: &Path, src: &Path) -> std::io::Result<Option<PathBuf>> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let meta = std::fs::symlink_metadata(&path)?;
            if metadata_is_link_or_reparse(&meta) {
                return Err(std::io::Error::other(format!(
                    "refusing linked/reparse path while scanning Script/: {}",
                    path.display()
                )));
            }
            if meta.is_dir() {
                if let Some(found) = walk(&path, root, src)? {
                    return Ok(Some(found));
                }
            } else if path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("as"))
            {
                let rel = path.strip_prefix(root).map_err(|e| {
                    std::io::Error::other(format!(
                        "walked path {} escaped root {}: {e}",
                        path.display(),
                        root.display()
                    ))
                })?;
                let covered = src.join(rel);
                match std::fs::symlink_metadata(&covered) {
                    Ok(meta) if metadata_is_link_or_reparse(&meta) => {
                        return Err(std::io::Error::other(format!(
                            "refusing linked/reparse source coverage path: {}",
                            covered.display()
                        )));
                    }
                    Ok(meta) if meta.is_file() => {}
                    Ok(_) => return Ok(Some(path)),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        return Ok(Some(path));
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(None)
    }
    walk(script_dir, script_dir, src_dir)
}

/// Resolve `out` to an absolute path for the "output must be outside Script/" containment check:
/// relative paths are taken relative to `cwd` (so a relative `-o` can't slip past the check), then
/// canonicalized as far as the path exists — the file itself, else its existing parent joined with
/// the filename, else the lexical absolute path. Extracted so the guard is testable without mutating
/// the process cwd.
fn resolve_out_real(out: &Path, cwd: &Path) -> PathBuf {
    let abs = if out.is_absolute() {
        out.to_path_buf()
    } else {
        cwd.join(out)
    };
    abs.canonicalize()
        .unwrap_or_else(|_| match (abs.parent(), abs.file_name()) {
            (Some(parent), Some(name)) => parent
                .canonicalize()
                .map(|p| p.join(name))
                .unwrap_or_else(|_| abs.clone()),
            _ => abs.clone(),
        })
}

/// Testable core of [`precompile`]. `generate(exe, g1r, dev_cache)` must make the game write
/// `PrecompiledScript.Cache` and return its bytes; the real public paths use
/// [`real_generate_report`], while tests inject a stub so orchestration stays offline.
#[cfg(test)]
fn precompile_with<G>(opts: &PrecompileOpts, generate: G) -> Result<PathBuf, String>
where
    G: FnOnce(&Path, &Path, &Path) -> Result<Vec<u8>, String>,
{
    precompile_with_generator_report(opts, |exe, g1r, cache| {
        GeneratorRunResult::confirmed(generate(exe, g1r, cache))
    })
}

fn precompile_with_generator_report<G>(
    opts: &PrecompileOpts,
    generate: G,
) -> Result<PathBuf, String>
where
    G: FnOnce(&Path, &Path, &Path) -> GeneratorRunResult<Vec<u8>>,
{
    let g1r = g1r_dir(&opts.game_dir);
    let exe = g1r
        .join("Binaries")
        .join("Win64")
        .join("G1R-Win64-Shipping.exe");
    if !exe.exists() {
        return Err(format!("game exe not found: {}", exe.display()));
    }
    let script_dir = g1r.join("Script");
    let shipping_cache = script_dir.join("PrecompiledScript_Shipping.Cache");
    let dev_cache = script_dir.join("PrecompiledScript.Cache");

    // Reject a source tree that contains (or IS) the Script destination: `copy_tree` would copy the
    // install into its own subtree, recursing `Script/…/Script` until the path or disk blows up while
    // polluting the live install. (Mirrors deploy_shared's self-copy guard.)
    if let Some(src) = &opts.src {
        let src_real = src.canonicalize().unwrap_or_else(|_| src.clone());
        let dst_real = script_dir
            .canonicalize()
            .unwrap_or_else(|_| script_dir.clone());
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
        let script_real = script_dir
            .canonicalize()
            .unwrap_or_else(|_| script_dir.clone());
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

    // When compiling a specific SRC tree, the game must see ONLY paths covered by that tree. The
    // transaction snapshots and restores existing files at matching relative paths, so a normal
    // installed source tree is safe to overlay. Refuse only an uncovered loose script: otherwise it
    // would silently participate in the generated cache.
    if let Some(src) = &opts.src {
        if let Some(stray) = first_uncovered_loose_script(&script_dir, src)
            .map_err(|e| format!("inspecting the game's Script/ tree: {e}"))?
        {
            return Err(format!(
                "the game's Script/ directory ({}) contains a loose script not present in the \
                 staged source tree ({}); refusing a contaminated compile",
                script_dir.display(),
                stray.display()
            ));
        }
    }

    // Both compile entry points share the same lock and disk-backed Shipping recovery guard.
    // Generation isolation begins before staging or deleting the development cache.
    let mut txn = CompileTransaction::begin(&opts.game_dir, &g1r, &script_dir)?;
    let mut process_exit = GeneratorProcessExitDisposition::NotStarted;
    let result = (|| -> Result<Vec<u8>, String> {
        txn.begin_isolation()?;
        if let Some(src) = &opts.src {
            txn.stage(src)?;
        }
        // Delete the old (or accidentally staged) development cache immediately before launch so
        // existence/size can only describe this run. The optional original is restored below.
        remove_if_exists(&txn.dev_cache)?;
        let generated = generate(&exe, &g1r, &dev_cache);
        process_exit = generated.process_exit;
        let regen = generated.result?;
        if regen.is_empty() {
            return Err("the game produced an empty PrecompiledScript.Cache".into());
        }
        validate_generated_cache(&regen)?;
        Ok(regen)
    })();

    if process_exit == GeneratorProcessExitDisposition::Unconfirmed {
        let error = result
            .err()
            .unwrap_or_else(|| "generator exit was unconfirmed after reporting success".to_owned());
        return Err(txn.preserve_for_unconfirmed_generator(error));
    }

    // Always restore the complete install before either publishing an output artifact or starting
    // the explicit in-place install phase.
    let restore = txn.restore_install();
    if !restore.clean() {
        let primary = result.err().unwrap_or_else(|| {
            "compiled, but refusing output/install because cleanup failed".into()
        });
        return Err(format!(
            "{primary}; additionally {}",
            restore.errors.join("; ")
        ));
    }

    let regen = match result {
        Ok(regen) => regen,
        Err(e) => {
            let finish = txn.finish();
            return if finish.is_empty() {
                Err(e)
            } else {
                Err(format!("{e}; additionally {}", finish.join("; ")))
            };
        }
    };

    if let Some(out) = &opts.out {
        let finish = txn.finish();
        if !finish.is_empty() {
            return Err(format!(
                "compiled, but refusing output because transaction cleanup failed: {}",
                finish.join("; ")
            ));
        }
        std::fs::write(out, &regen)
            .map_err(|e| format!("writing output {}: {e}", out.display()))?;
        return Ok(out.clone());
    }

    // A persistent deploy `.gore-bak` is created only now: generation and all restoration already
    // succeeded. A failed install removes a backup created by this call; a pre-existing one is
    // never overwritten or removed.
    if let Err(e) = txn.prepare_deploy_backup(opts.backup) {
        let finish = txn.finish();
        return if finish.is_empty() {
            Err(e)
        } else {
            Err(format!("{e}; additionally {}", finish.join("; ")))
        };
    }

    txn.arm_install_rollback();
    match std::fs::write(&shipping_cache, &regen) {
        Ok(()) => {
            txn.mark_install_committed();
            let finish = txn.finish();
            if finish.is_empty() {
                Ok(shipping_cache)
            } else {
                Err(format!(
                    "installed generated cache, but transaction cleanup failed: {}",
                    finish.join("; ")
                ))
            }
        }
        Err(install_error) => {
            let restore = txn.restore_install();
            let restore_clean = restore.clean();
            let mut errors = restore.errors;
            if let Err(e) = txn.remove_ephemeral_deploy_backup() {
                errors.push(e);
            }
            if restore_clean && errors.is_empty() {
                errors.extend(txn.finish());
            }
            let primary = format!("installing generated cache in place: {install_error}");
            if errors.is_empty() {
                Err(primary)
            } else {
                Err(format!("{primary}; additionally {}", errors.join("; ")))
            }
        }
    }
}

/// Launch the game with the proven AngelScript generation flags, then read the newly-created
/// `PrecompiledScript.Cache`. The caller removes that file before launch, so mere existence is a
/// fresh-run signal; the shipping cache is never the generator output.
fn real_generate_report(
    exe: &Path,
    g1r: &Path,
    cache: &Path,
    diagnostics: &crate::diagnostics::DiagnosticsOptions,
) -> GeneratorRunResult<Vec<u8>> {
    let generated = real_generate_with_timeout_and_diagnostics_report(
        exe,
        g1r,
        cache,
        Duration::from_secs(30 * 60),
        diagnostics,
    );
    GeneratorRunResult {
        result: generated.result,
        process_exit: generated.process_exit,
    }
}

const GENERATOR_EXIT_UNCONFIRMED: &str = "[gore:generator-exit-unconfirmed]";

#[cfg(test)]
fn generator_exit_unconfirmed(error: &str) -> bool {
    error.contains(GENERATOR_EXIT_UNCONFIRMED)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GeneratorProcessExitDisposition {
    NotStarted,
    Confirmed,
    Unconfirmed,
}

#[derive(Debug)]
struct GeneratorRunResult<T> {
    result: Result<T, String>,
    process_exit: GeneratorProcessExitDisposition,
}

impl<T> GeneratorRunResult<T> {
    fn not_started(result: Result<T, String>) -> Self {
        Self {
            result,
            process_exit: GeneratorProcessExitDisposition::NotStarted,
        }
    }

    fn confirmed(result: Result<T, String>) -> Self {
        Self {
            result,
            process_exit: GeneratorProcessExitDisposition::Confirmed,
        }
    }

    fn unconfirmed(error: String) -> Self {
        Self {
            result: Err(error),
            process_exit: GeneratorProcessExitDisposition::Unconfirmed,
        }
    }
}

/// Spawn/try_wait implementation with a real wall-clock deadline. Keeping the timeout injectable
/// makes the termination path testable without weakening the production 30-minute maximum.
const GENERATOR_ARGS: &[&str] = &[
    "-as-development-mode",
    "-as-generate-precompiled-data",
    "-as-skip-threaded-initialize",
    "-as-exit-on-error",
];

fn run_normal_generator_report(
    exe: &Path,
    g1r: &Path,
    cache: &Path,
    timeout: Duration,
) -> GeneratorRunResult<Vec<u8>> {
    let mut child = match std::process::Command::new(exe)
        .args(GENERATOR_ARGS)
        .current_dir(g1r)
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return GeneratorRunResult::not_started(Err(format!("launching game: {error}")));
        }
    };

    finish_generator_child_report(&mut child, cache, timeout)
}

/// A failed hook attempt may have started the generator and written a partial development cache
/// before its process was confirmed gone. Remove that first-attempt artifact before the fallback
/// launch so the normal result cannot accidentally accept stale bytes.
fn run_clean_fallback_generator_report(
    exe: &Path,
    g1r: &Path,
    cache: &Path,
    timeout: Duration,
) -> GeneratorRunResult<Vec<u8>> {
    if let Err(error) = clear_partial_cache_before_fallback(cache) {
        return GeneratorRunResult::not_started(Err(error));
    }
    run_normal_generator_report(exe, g1r, cache, timeout)
}

fn clear_partial_cache_before_fallback(cache: &Path) -> Result<(), String> {
    remove_if_exists(cache)
}

fn finish_generator_child_report(
    child: &mut std::process::Child,
    cache: &Path,
    timeout: Duration,
) -> GeneratorRunResult<Vec<u8>> {
    let status = match wait_for_child_with_timeout_report(
        child,
        timeout,
        Duration::from_millis(250),
        Duration::from_secs(2),
        "AngelScript generation",
    ) {
        Ok(status) => status,
        Err(error) => {
            return match error.process_exit {
                GeneratorProcessExitDisposition::Unconfirmed => {
                    GeneratorRunResult::unconfirmed(error.message)
                }
                GeneratorProcessExitDisposition::Confirmed => {
                    GeneratorRunResult::confirmed(Err(error.message))
                }
                GeneratorProcessExitDisposition::NotStarted => {
                    GeneratorRunResult::not_started(Err(error.message))
                }
            };
        }
    };
    GeneratorRunResult::confirmed(read_completed_generated_cache(
        cache,
        status.success(),
        &status.to_string(),
    ))
}

#[derive(Debug)]
enum DiagnosticAttempt<T> {
    Completed(GeneratorDiagnosticsResult<T>),
    Disabled,
    Unavailable(String),
    Fatal(GeneratorDiagnosticsResult<T>),
}

#[derive(Debug)]
struct GeneratorDiagnosticsResult<T> {
    result: Result<T, String>,
    diagnostics: crate::diagnostics::CompilerDiagnosticsReport,
    process_exit: GeneratorProcessExitDisposition,
}

/// Infrastructure failure is deliberately not a compiler failure: once the first process is
/// confirmed absent, execute the unchanged normal generator and return its result byte-for-byte.
fn resolve_diagnostic_attempt_report<T, N>(
    attempt: DiagnosticAttempt<T>,
    normal: N,
) -> GeneratorDiagnosticsResult<T>
where
    N: FnOnce() -> GeneratorRunResult<T>,
{
    match attempt {
        DiagnosticAttempt::Completed(report) | DiagnosticAttempt::Fatal(report) => report,
        DiagnosticAttempt::Disabled => {
            let normal = normal();
            GeneratorDiagnosticsResult {
                result: normal.result,
                diagnostics: crate::diagnostics::CompilerDiagnosticsReport::empty(
                    crate::diagnostics::DiagnosticsCaptureDisposition::Disabled,
                ),
                process_exit: normal.process_exit,
            }
        }
        DiagnosticAttempt::Unavailable(reason) => {
            eprintln!(
                "gore: AngelScript diagnostics unavailable ({reason}); falling back to the normal generator"
            );
            let normal = normal();
            GeneratorDiagnosticsResult {
                result: normal.result,
                diagnostics: crate::diagnostics::CompilerDiagnosticsReport::empty(
                    crate::diagnostics::DiagnosticsCaptureDisposition::UnavailableFallback,
                ),
                process_exit: normal.process_exit,
            }
        }
    }
}

#[cfg(test)]
fn resolve_diagnostic_attempt<T, N>(attempt: DiagnosticAttempt<T>, normal: N) -> Result<T, String>
where
    N: FnOnce() -> Result<T, String>,
{
    resolve_diagnostic_attempt_report(attempt, || {
        let result = normal();
        if result
            .as_ref()
            .err()
            .is_some_and(|error| generator_exit_unconfirmed(error))
        {
            GeneratorRunResult {
                result,
                process_exit: GeneratorProcessExitDisposition::Unconfirmed,
            }
        } else {
            GeneratorRunResult::confirmed(result)
        }
    })
    .result
}

struct DiagnosticArtifacts {
    dir: PathBuf,
    capture: PathBuf,
    status: PathBuf,
    cleanup: bool,
}

impl DiagnosticArtifacts {
    fn create() -> Result<Self, String> {
        let temp = std::env::temp_dir();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        for suffix in 0..32u32 {
            let dir = temp.join(format!(
                "gore-as-diagnostics-{}-{stamp}-{suffix}",
                std::process::id()
            ));
            match std::fs::create_dir(&dir) {
                Ok(()) => {
                    return Ok(Self {
                        capture: dir.join("capture.txt"),
                        status: dir.join("status.txt"),
                        dir,
                        cleanup: true,
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => {
                    return Err(format!(
                        "creating diagnostics temp directory {}: {e}",
                        dir.display()
                    ));
                }
            }
        }
        Err("could not reserve a unique diagnostics temp directory".into())
    }

    fn preserve(mut self) -> PathBuf {
        self.cleanup = false;
        self.dir.clone()
    }
}

impl Drop for DiagnosticArtifacts {
    fn drop(&mut self) {
        if !self.cleanup {
            return;
        }
        let _ = std::fs::remove_file(&self.capture);
        let _ = std::fs::remove_file(&self.status);
        let _ = std::fs::remove_dir(&self.dir);
    }
}

fn append_captured_diagnostics(
    generated: GeneratorRunResult<Vec<u8>>,
    artifacts: &DiagnosticArtifacts,
    disposition: crate::diagnostics::DiagnosticsCaptureDisposition,
) -> GeneratorDiagnosticsResult<Vec<u8>> {
    let GeneratorRunResult {
        result,
        process_exit,
    } = generated;
    let (capture, diagnostics, capture_failure) = match crate::diagnostics::read_bounded(
        &artifacts.capture,
        crate::diagnostics::MAX_CAPTURE_BYTES,
    ) {
        Ok((capture, truncated)) => {
            let protocol_truncated = capture.lines().any(|line| {
                line.trim_end_matches('\r') == crate::diagnostics::CAPTURE_TRUNCATED_TOKEN
            });
            let mut capture_failure =
                (truncated || protocol_truncated).then(|| "was truncated".to_owned());
            let report_disposition = if capture_failure.is_some() {
                crate::diagnostics::DiagnosticsCaptureDisposition::CaptureInvalid
            } else {
                disposition
            };
            let diagnostics =
                match crate::diagnostics::CompilerDiagnosticsReport::from_bounded_capture(
                    report_disposition,
                    &capture,
                ) {
                    Ok(report) => report,
                    Err(error) => {
                        capture_failure = Some(format!(
                            "could not be represented as bounded structured diagnostics ({error})"
                        ));
                        crate::diagnostics::CompilerDiagnosticsReport::empty(
                            crate::diagnostics::DiagnosticsCaptureDisposition::CaptureInvalid,
                        )
                    }
                };
            let mut formatted = crate::diagnostics::format_capture(&capture);
            const CAPTURE_TRUNCATED: &str = "<diagnostics truncated after 8 MiB>\n";
            if truncated
                && !protocol_truncated
                && formatted.len().saturating_add(CAPTURE_TRUNCATED.len())
                    <= crate::diagnostics::MAX_FORMATTED_BYTES
            {
                formatted.push_str(CAPTURE_TRUNCATED);
            }
            (formatted, diagnostics, capture_failure)
        }
        Err(_error) if !artifacts.capture.exists() => (
            String::new(),
            crate::diagnostics::CompilerDiagnosticsReport::empty(disposition),
            None,
        ),
        Err(error) => (
            format!("<diagnostics capture unreadable: {error}>\n"),
            crate::diagnostics::CompilerDiagnosticsReport::empty(
                crate::diagnostics::DiagnosticsCaptureDisposition::CaptureInvalid,
            ),
            Some("could not be read".to_owned()),
        ),
    };
    let has_compiler_error = diagnostics
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.severity == crate::diagnostics::DiagnosticSeverity::Error);
    let result = if capture.trim().is_empty() {
        match result {
            Ok(_) if capture_failure.is_some() => Err(format!(
                "AngelScript diagnostics capture {}; refusing to accept an unverified cache",
                capture_failure.as_deref().unwrap_or("was invalid")
            )),
            result => result,
        }
    } else {
        match result {
            Ok(_) if has_compiler_error => Err(format!(
                "AngelScript compiler reported an error despite producing a structurally complete cache\n--- AngelScript compiler diagnostics ---\n{}",
                capture.trim_end()
            )),
            Ok(_) if capture_failure.is_some() => Err(format!(
                "AngelScript diagnostics capture {}; refusing to accept an unverified cache\n--- AngelScript compiler diagnostics ---\n{}",
                capture_failure.as_deref().unwrap_or("was invalid"),
                capture.trim_end()
            )),
            Ok(bytes) => {
                eprint!("{capture}");
                Ok(bytes)
            }
            Err(error) => Err(format!(
                "{error}\n--- AngelScript compiler diagnostics ---\n{}",
                capture.trim_end()
            )),
        }
    };
    GeneratorDiagnosticsResult {
        result,
        diagnostics,
        process_exit,
    }
}

fn preserve_unconfirmed_diagnostic_attempt(
    error: String,
    artifacts: DiagnosticArtifacts,
    prep: crate::diagnostics::HookPreparation,
) -> DiagnosticAttempt<Vec<u8>> {
    let diagnostics_dir = artifacts.preserve();
    let helper_dir = prep.preserve_owned();
    let helper_note = helper_dir
        .as_deref()
        .map(|path| format!(", embedded helper directory {}", path.display()))
        .unwrap_or_default();
    DiagnosticAttempt::Fatal(GeneratorDiagnosticsResult {
        result: Err(format!(
            "{error}; process exit is unconfirmed, so diagnostics files were intentionally preserved at {}{}",
            diagnostics_dir.display(),
            helper_note
        )),
        diagnostics: crate::diagnostics::CompilerDiagnosticsReport::empty(
            crate::diagnostics::DiagnosticsCaptureDisposition::ProcessExitUnconfirmed,
        ),
        process_exit: GeneratorProcessExitDisposition::Unconfirmed,
    })
}

fn classify_hooked_result(
    generated: GeneratorRunResult<Vec<u8>>,
    artifacts: DiagnosticArtifacts,
    prep: crate::diagnostics::HookPreparation,
) -> DiagnosticAttempt<Vec<u8>> {
    let generated = match generated.process_exit {
        GeneratorProcessExitDisposition::Unconfirmed => {
            // The child may still own and append to the capture. Preserve the whole directory for
            // recovery, but never read or expose a snapshot as if it were a completed report.
            return preserve_unconfirmed_diagnostic_attempt(
                generated.result.unwrap_err(),
                artifacts,
                prep,
            );
        }
        _ => generated,
    };
    let report = append_captured_diagnostics(
        generated,
        &artifacts,
        crate::diagnostics::DiagnosticsCaptureDisposition::Captured,
    );
    DiagnosticAttempt::Completed(report)
}

fn classify_started_hook_termination(
    termination: ChildWaitFailure,
    artifacts: DiagnosticArtifacts,
    prep: crate::diagnostics::HookPreparation,
) -> DiagnosticAttempt<Vec<u8>> {
    if termination.process_exit == GeneratorProcessExitDisposition::Unconfirmed {
        preserve_unconfirmed_diagnostic_attempt(termination.message, artifacts, prep)
    } else {
        DiagnosticAttempt::Unavailable(termination.message)
    }
}

fn real_generate_with_timeout_and_diagnostics_report(
    exe: &Path,
    g1r: &Path,
    cache: &Path,
    timeout: Duration,
    diagnostics: &crate::diagnostics::DiagnosticsOptions,
) -> GeneratorDiagnosticsResult<Vec<u8>> {
    if diagnostics.disabled {
        return resolve_diagnostic_attempt_report(DiagnosticAttempt::Disabled, || {
            run_clean_fallback_generator_report(exe, g1r, cache, timeout)
        });
    }
    let prep = match crate::diagnostics::prepare_hook(exe, diagnostics) {
        Ok(prep) => prep,
        Err(reason) => {
            return resolve_diagnostic_attempt_report(
                DiagnosticAttempt::Unavailable(reason),
                || run_clean_fallback_generator_report(exe, g1r, cache, timeout),
            );
        }
    };
    let artifacts = match DiagnosticArtifacts::create() {
        Ok(artifacts) => artifacts,
        Err(reason) => {
            return resolve_diagnostic_attempt_report(
                DiagnosticAttempt::Unavailable(reason),
                || run_clean_fallback_generator_report(exe, g1r, cache, timeout),
            );
        }
    };
    let attempt = match crate::diagnostics::spawn_hooked(
        exe,
        g1r,
        GENERATOR_ARGS,
        &prep,
        &artifacts.capture,
        &artifacts.status,
        diagnostics.inject_delay,
    ) {
        Ok(crate::diagnostics::HookSpawnOutcome::Hooked(mut child)) => {
            let result = finish_generator_child_report(&mut child, cache, timeout);
            classify_hooked_result(result, artifacts, prep)
        }
        Ok(crate::diagnostics::HookSpawnOutcome::ExitedBeforeInjection(mut child)) => {
            let generated = finish_generator_child_report(&mut child, cache, timeout);
            if generated.process_exit == GeneratorProcessExitDisposition::Unconfirmed {
                preserve_unconfirmed_diagnostic_attempt(
                    generated.result.unwrap_err(),
                    artifacts,
                    prep,
                )
            } else {
                DiagnosticAttempt::Completed(append_captured_diagnostics(
                    generated,
                    &artifacts,
                    crate::diagnostics::DiagnosticsCaptureDisposition::UnavailableWithoutFallback,
                ))
            }
        }
        Ok(crate::diagnostics::HookSpawnOutcome::ExitedAfterInjectionBeforeReady {
            child,
            status,
        }) => {
            // `try_wait` already confirmed direct-process exit. Keep the child handle alive until
            // this point, then discard the injected attempt and let run_clean_fallback_generator
            // remove any partial cache before relaunching without diagnostics.
            drop(child);
            DiagnosticAttempt::Unavailable(format!(
                "generator exited after diagnostics injection but before helper readiness ({status})"
            ))
        }
        Err(crate::diagnostics::HookSpawnError::SafeFallback(reason)) => {
            DiagnosticAttempt::Unavailable(reason)
        }
        Err(crate::diagnostics::HookSpawnError::Started { mut child, reason }) => {
            match child.try_wait() {
                Ok(Some(status)) => DiagnosticAttempt::Unavailable(format!(
                    "{reason}; first generator already exited ({status})"
                )),
                _ => {
                    let termination = terminate_child_bounded_report(
                        &mut child,
                        &reason,
                        Duration::from_millis(20),
                        Duration::from_secs(5),
                    );
                    classify_started_hook_termination(termination, artifacts, prep)
                }
            }
        }
    };
    resolve_diagnostic_attempt_report(attempt, || {
        run_clean_fallback_generator_report(exe, g1r, cache, timeout)
    })
}

/// Read and structurally validate the generator output. The shipping game build used by G1R exits
/// with status 1 after a successful `-as-generate-precompiled-data` run, so process status alone is
/// not an acceptance signal. Conversely, a merely present/non-empty file is unsafe after an error:
/// accept it only when every module and all seven tail tables parse exactly to EOF.
fn read_completed_generated_cache(
    cache: &Path,
    status_success: bool,
    status_label: &str,
) -> Result<Vec<u8>, String> {
    if !cache.exists() {
        return Err(format!(
            "AngelScript generation exited with {status_label} but produced no {}",
            cache.display()
        ));
    }
    let bytes = std::fs::read(cache).map_err(|e| format!("reading regen cache: {e}"))?;
    validate_generated_cache(&bytes).map_err(|e| {
        if status_success {
            e
        } else {
            format!(
                "AngelScript generation exited unsuccessfully ({status_label}) and its output was \
                 incomplete: {e}"
            )
        }
    })?;
    Ok(bytes)
}

/// Wait for a direct child up to a hard execution deadline. On timeout or polling failure, request
/// termination and observe it only for the separately bounded `termination_grace`; this function
/// never calls blocking `Child::wait` after a failed kill (or at all).
#[cfg(test)]
fn wait_for_child_with_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
    poll_interval: Duration,
    termination_grace: Duration,
    context: &str,
) -> Result<std::process::ExitStatus, String> {
    wait_for_child_with_timeout_report(child, timeout, poll_interval, termination_grace, context)
        .map_err(|failure| failure.message)
}

#[derive(Debug)]
struct ChildWaitFailure {
    message: String,
    process_exit: GeneratorProcessExitDisposition,
}

fn wait_for_child_with_timeout_report(
    child: &mut std::process::Child,
    timeout: Duration,
    poll_interval: Duration,
    termination_grace: Duration,
    context: &str,
) -> Result<std::process::ExitStatus, ChildWaitFailure> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| ChildWaitFailure {
            message: format!("{context} timeout is too large"),
            process_exit: GeneratorProcessExitDisposition::Unconfirmed,
        })?;
    let poll_interval = poll_interval.max(Duration::from_millis(1));
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {
                let now = Instant::now();
                if now >= deadline {
                    let cause = format!("{context} exceeded the {timeout:?} timeout");
                    return Err(terminate_child_bounded_report(
                        child,
                        &cause,
                        poll_interval,
                        termination_grace,
                    ));
                }
                std::thread::sleep(poll_interval.min(deadline.saturating_duration_since(now)));
            }
            Err(e) => {
                let cause = format!("waiting for {context}: {e}");
                return Err(terminate_child_bounded_report(
                    child,
                    &cause,
                    poll_interval,
                    termination_grace,
                ));
            }
        }
    }
}

/// Process-tree termination with a bounded observation window. On Windows, `taskkill /T /F` first
/// handles descendants; `Child::kill` remains the direct-child fallback on every platform. An
/// unconfirmed exit is marked so transaction owners preserve isolation instead of racing cleanup.
fn terminate_child_bounded_report(
    child: &mut std::process::Child,
    cause: &str,
    poll_interval: Duration,
    termination_grace: Duration,
) -> ChildWaitFailure {
    let pid = child.id();
    let deadline = Instant::now()
        .checked_add(termination_grace)
        .unwrap_or_else(Instant::now);
    let tree = request_process_tree_termination(pid, deadline);
    let kill_error = child.kill().err();
    let poll_interval = poll_interval.max(Duration::from_millis(1));
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !tree.confirmed {
                    return ChildWaitFailure {
                        message: format!(
                            "{GENERATOR_EXIT_UNCONFIRMED} {cause}; direct child {pid} exited during \
                         termination ({status}), but descendant termination was not confirmed \
                         ({}). Isolation must remain in place",
                            tree.note
                        ),
                        process_exit: GeneratorProcessExitDisposition::Unconfirmed,
                    };
                }
                let message = match kill_error {
                    Some(kill_error) => format!(
                        "{cause}; child {pid} exited during termination ({status}; direct kill \
                         reported: {kill_error}; {})",
                        tree.note
                    ),
                    None => format!(
                        "{cause}; process tree rooted at child {pid} was terminated ({status}; \
                         {})",
                        tree.note
                    ),
                };
                return ChildWaitFailure {
                    message,
                    process_exit: GeneratorProcessExitDisposition::Confirmed,
                };
            }
            Ok(None) => {
                let now = Instant::now();
                if now >= deadline {
                    let kill_note = kill_error
                        .as_ref()
                        .map(|e| format!("direct kill reported: {e}"))
                        .unwrap_or_else(|| "direct kill was requested".into());
                    return ChildWaitFailure {
                        message: format!(
                            "{GENERATOR_EXIT_UNCONFIRMED} {cause}; termination was requested for \
                         process tree {pid}, but exit was not observed within \
                         {termination_grace:?} ({kill_note}; {})",
                            tree.note
                        ),
                        process_exit: GeneratorProcessExitDisposition::Unconfirmed,
                    };
                }
                std::thread::sleep(poll_interval.min(deadline.saturating_duration_since(now)));
            }
            Err(e) => {
                return ChildWaitFailure {
                    message: format!(
                        "{GENERATOR_EXIT_UNCONFIRMED} {cause}; termination was requested for process \
                     tree {pid}, but querying its exit failed: {e} ({})",
                        tree.note
                    ),
                    process_exit: GeneratorProcessExitDisposition::Unconfirmed,
                };
            }
        }
    }
}

struct TreeTermination {
    confirmed: bool,
    note: String,
}

#[cfg(windows)]
fn request_process_tree_termination(pid: u32, deadline: Instant) -> TreeTermination {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut killer = match std::process::Command::new("taskkill.exe")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
    {
        Ok(killer) => killer,
        Err(e) => {
            return TreeTermination {
                confirmed: false,
                note: format!("process-tree terminator could not start: {e}"),
            };
        }
    };
    loop {
        match killer.try_wait() {
            Ok(Some(status)) if status.success() => {
                return TreeTermination {
                    confirmed: true,
                    note: "taskkill confirmed process-tree termination".into(),
                };
            }
            Ok(Some(status)) => {
                return TreeTermination {
                    confirmed: false,
                    note: format!("taskkill exited unsuccessfully: {status}"),
                };
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(None) => {
                let _ = killer.kill();
                return TreeTermination {
                    confirmed: false,
                    note: "taskkill did not finish within the termination grace".into(),
                };
            }
            Err(e) => {
                return TreeTermination {
                    confirmed: false,
                    note: format!("querying taskkill failed: {e}"),
                };
            }
        }
    }
}

#[cfg(not(windows))]
fn request_process_tree_termination(_pid: u32, _deadline: Instant) -> TreeTermination {
    TreeTermination {
        confirmed: true,
        note: "platform uses the direct child as the generation process tree".into(),
    }
}

/// Recursively copy `src` into `dst`, recording every destination FILE path written into `out`
/// together with its PRIOR bytes (`None` if it didn't exist, `Some(bytes)` if the copy overwrote a
/// pre-existing file) — so the caller can delete what it created and RESTORE what it overwrote.
/// Directories created are not recorded individually — empty ones are pruned bottom-up by
/// [`restore_or_remove`].
fn copy_tree(
    src: &Path,
    dst: &Path,
    out: &mut Vec<(PathBuf, Option<Vec<u8>>)>,
) -> std::io::Result<()> {
    copy_tree_with(src, dst, out, &mut |from, to| {
        std::fs::copy(from, to).map(|_| ())
    })
}

fn copy_tree_with<C>(
    src: &Path,
    dst: &Path,
    out: &mut Vec<(PathBuf, Option<Vec<u8>>)>,
    copy_file: &mut C,
) -> std::io::Result<()>
where
    C: FnMut(&Path, &Path) -> std::io::Result<()>,
{
    ensure_real_directory(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let source_meta = std::fs::symlink_metadata(entry.path())?;
        if metadata_is_link_or_reparse(&source_meta) {
            return Err(std::io::Error::other(format!(
                "refusing linked/reparse source path {}",
                entry.path().display()
            )));
        }
        let to = dst.join(entry.file_name());
        if source_meta.is_dir() {
            copy_tree_with(&entry.path(), &to, out, copy_file)?;
        } else if source_meta.is_file() {
            // Capture the pre-existing bytes (if any) BEFORE overwriting, so cleanup can restore a
            // user's own loose script that happens to share this path with the emitted tree.
            let prior = match std::fs::symlink_metadata(&to) {
                Ok(meta) => {
                    if metadata_is_link_or_reparse(&meta) {
                        return Err(std::io::Error::other(format!(
                            "refusing linked/reparse destination path {}",
                            to.display()
                        )));
                    }
                    if !meta.is_file() {
                        return Err(std::io::Error::other(format!(
                            "destination path is not a regular file: {}",
                            to.display()
                        )));
                    }
                    Some(std::fs::read(&to)?)
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(e),
            };
            // Register rollback BEFORE copy: std::fs::copy may truncate/create the destination and
            // then fail, so recording only after success would leak a partial file.
            out.push((to.clone(), prior));
            copy_file(&entry.path(), &to)?;
        } else {
            return Err(std::io::Error::other(format!(
                "source path is not a regular file or directory: {}",
                entry.path().display()
            )));
        }
    }
    Ok(())
}

/// Rust's ordinary metadata follows links. Compile staging is a privileged write into the live
/// game tree, so reject symlinks and (on Windows) every reparse point, including junctions.
fn metadata_is_link_or_reparse(meta: &std::fs::Metadata) -> bool {
    if meta.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

/// Create a directory one component at a time while proving every existing component is a real
/// directory. `create_dir_all` would otherwise traverse a pre-existing symlink/junction before the
/// caller gets a chance to inspect it.
fn ensure_real_directory(path: &Path) -> std::io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            if metadata_is_link_or_reparse(&meta) {
                return Err(std::io::Error::other(format!(
                    "refusing linked/reparse destination directory {}",
                    path.display()
                )));
            }
            if !meta.is_dir() {
                return Err(std::io::Error::other(format!(
                    "destination path is not a directory: {}",
                    path.display()
                )));
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let parent = path.parent().ok_or_else(|| {
                std::io::Error::other(format!(
                    "cannot create destination directory without a parent: {}",
                    path.display()
                ))
            })?;
            if parent.as_os_str().is_empty() {
                ensure_real_directory(Path::new("."))?;
            } else if parent != path {
                ensure_real_directory(parent)?;
            }
            match std::fs::create_dir(path) {
                Ok(()) => Ok(()),
                // Another actor may have created it between inspection and creation. Re-inspect
                // rather than accepting a newly-planted reparse point.
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    ensure_real_directory(path)
                }
                Err(e) => Err(e),
            }
        }
        Err(e) => Err(e),
    }
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
                    if e.kind() != std::io::ErrorKind::NotFound {
                        errs.push(format!("delete {}: {e}", f.display()));
                    }
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
    use crate::cache::model::{Class, Func, Module};
    use crate::cache::types::DataType;

    static PROCESS_TIMEOUT_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn test_function(name: &str) -> Func {
        Func {
            name: name.into(),
            namespace: String::new(),
            ret: DataType::default(),
            params: Vec::new(),
            bytecode: Vec::new(),
            obj_locals: Vec::new(),
            is_ufunction: false,
            traits: 0,
        }
    }

    #[test]
    fn edit_preflight_identifies_only_omitted_generated_class_methods() {
        let modules = vec![Module {
            name: "QuestModule".into(),
            file: "QuestModule.as".into(),
            functions: Vec::new(),
            classes: vec![Class {
                name: "UQuestFixture".into(),
                super_class: None,
                fields: Vec::new(),
                methods: vec![test_function("Tick"), test_function("__InitDefaults")],
                ctors: Vec::new(),
                flags: 0,
            }],
            enums: Vec::new(),
            globals: Vec::new(),
        }];

        assert_eq!(
            omitted_generated_methods(&modules, "QuestModule").unwrap(),
            ["UQuestFixture::__InitDefaults"]
        );
        assert!(omitted_generated_methods(&modules, "Missing")
            .unwrap_err()
            .to_string()
            .contains("expected exactly one base module, found 0"));
        assert!(prepare_generated_defaults_edit(
            "add",
            &modules,
            "QuestModule",
            &[],
            "default Foo = 1;",
            true,
        )
        .unwrap()
        .is_none());
        let error = prepare_generated_defaults_edit(
            "edit",
            &modules,
            "QuestModule",
            &[],
            "class UQuestFixture {}",
            true,
        )
        .expect_err("edit must not mix carried defaults with new-symbol remap")
        .to_string();
        assert!(error.contains("UQuestFixture::__InitDefaults"), "{error}");
        assert!(error.contains("strict base-keyspace remap"), "{error}");
    }

    #[test]
    fn edit_preflight_never_treats_ambiguous_identities_as_an_empty_inventory() {
        let module = |classes| Module {
            name: "QuestModule".into(),
            file: "QuestModule.as".into(),
            functions: Vec::new(),
            classes,
            enums: Vec::new(),
            globals: Vec::new(),
        };
        let class = |name: &str, methods| Class {
            name: name.into(),
            super_class: None,
            fields: Vec::new(),
            methods,
            ctors: Vec::new(),
            flags: 0,
        };

        let duplicate_modules = vec![module(Vec::new()), module(Vec::new())];
        let error = prepare_generated_defaults_edit(
            "edit",
            &duplicate_modules,
            "QuestModule",
            &[],
            "",
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("expected exactly one base module, found 2"),
            "{error}"
        );

        let duplicate_classes = vec![module(vec![
            class("UQuestFixture", Vec::new()),
            class("UQuestFixture", vec![test_function("__InitDefaults")]),
        ])];
        let error = prepare_generated_defaults_edit(
            "edit",
            &duplicate_classes,
            "QuestModule",
            &[],
            "",
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("duplicate class identity"), "{error}");

        let duplicate_methods = vec![module(vec![class(
            "UQuestFixture",
            vec![
                test_function("__InitDefaults"),
                test_function("__InitDefaults"),
            ],
        )])];
        let error = prepare_generated_defaults_edit(
            "edit",
            &duplicate_methods,
            "QuestModule",
            &[],
            "",
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("duplicate generated method identity"),
            "{error}"
        );

        // `add` never consults the base-module inventory, so new-module authoring remains normal.
        assert!(prepare_generated_defaults_edit(
            "add",
            &duplicate_modules,
            "QuestModule",
            &[],
            "default Foo = 1;",
            true,
        )
        .unwrap()
        .is_none());
    }

    #[test]
    fn generated_default_source_gate_ignores_literals_and_comments_but_not_code() {
        assert!(!source_contains_default_token(
            r#"// default Foo = 1;
               string A = "default";
               FName B = n"default";
               /* ordinary default comment */
               void NodefaultValue() {}"#,
        )
        .unwrap());
        assert!(source_contains_default_token("/* /* */ default Health = 100; // */").unwrap());
        assert!(source_contains_default_token("default Health = 100;").unwrap());
        assert!(!source_contains_default_token("switch (X) { default: break; }").unwrap());
        assert!(
            !source_contains_default_token("switch (X) { default /* label */ : break; }").unwrap()
        );
        assert!(source_contains_default_token("default /* CDO */ Health = 100;").unwrap());
        assert!(source_contains_default_token("/* unterminated")
            .unwrap_err()
            .contains("unterminated block comment"));
        assert!(source_contains_default_token("string X = \"unterminated")
            .unwrap_err()
            .contains("unterminated quoted literal"));
    }

    #[test]
    fn g1r_dir_appends_or_keeps() {
        assert_eq!(
            g1r_dir(Path::new("games/Gothic")),
            PathBuf::from("games/Gothic/G1R")
        );
        assert_eq!(
            g1r_dir(Path::new("games/Gothic/G1R")),
            PathBuf::from("games/Gothic/G1R")
        );
    }

    #[test]
    fn precompile_errors_when_exe_missing() {
        // No shipping exe: the guard fires and the generator is NEVER invoked.
        let dir = std::env::temp_dir().join("gore-as-no-exe-xyz");
        let opts = PrecompileOpts {
            game_dir: dir,
            src: None,
            out: None,
            backup: true,
        };
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

    fn dev_cache(shipping_cache: &Path) -> PathBuf {
        shipping_cache
            .parent()
            .unwrap()
            .join("PrecompiledScript.Cache")
    }

    fn sia(s: &str) -> Vec<u8> {
        if s.is_empty() {
            return 0i32.to_le_bytes().to_vec();
        }
        let mut out = (s.len() as i32).to_le_bytes().to_vec();
        out.extend_from_slice(s.as_bytes());
        out.push(0);
        out
    }

    fn fstring(s: &str) -> Vec<u8> {
        let mut out = ((s.len() + 1) as i32).to_le_bytes().to_vec();
        out.extend_from_slice(s.as_bytes());
        out.push(0);
        out
    }

    fn cache_with_empty_modules(modules: &[(&str, &str)]) -> Vec<u8> {
        let mut out = vec![0u8; 16];
        out.extend_from_slice(&crate::cache::header::CACHE_MAGIC.to_le_bytes());
        out.extend_from_slice(&(modules.len() as u32).to_le_bytes());
        for (module, file) in modules {
            out.extend_from_slice(&fstring(module));
            out.extend_from_slice(&sia(module));
            out.extend_from_slice(&0i32.to_le_bytes()); // functions
            out.extend_from_slice(&0i32.to_le_bytes()); // classes
            out.extend_from_slice(&0i32.to_le_bytes()); // enums
            out.extend_from_slice(&0i32.to_le_bytes()); // globals
            out.extend_from_slice(&0i32.to_le_bytes()); // function imports
            out.extend_from_slice(&0i64.to_le_bytes()); // code hash
            out.extend_from_slice(&0i32.to_le_bytes()); // imported modules
            out.extend_from_slice(&sia("")); // statics class
            out.extend_from_slice(&0i32.to_le_bytes()); // events
            out.extend_from_slice(&0i32.to_le_bytes()); // delegates
            out.extend_from_slice(&sia(file));
            out.extend_from_slice(&0i32.to_le_bytes()); // post-init functions
        }
        for _ in 0..crate::cache::tables::N_TABLES {
            out.extend_from_slice(&0i32.to_le_bytes());
        }
        out
    }

    /// Small but structurally complete cache: one empty module followed by all seven empty tails.
    fn valid_cache() -> Vec<u8> {
        cache_with_empty_modules(&[("TestModule", "TestModule.as")])
    }

    /// Stub generator: emulate the game creating a complete development cache and return it.
    fn gen_new(_exe: &Path, _g1r: &Path, cache: &Path) -> Result<Vec<u8>, String> {
        assert_eq!(cache.file_name().unwrap(), "PrecompiledScript.Cache");
        assert!(
            !cache.exists(),
            "caller must remove a stale development cache first"
        );
        let bytes = valid_cache();
        std::fs::write(cache, &bytes).map_err(|e| e.to_string())?;
        Ok(bytes)
    }

    #[test]
    fn generated_cache_validation_requires_header_modules_all_tails_and_eof() {
        let good = valid_cache();
        validate_generated_cache(&good).unwrap();

        let mut bad_magic = good.clone();
        bad_magic[0x10..0x14].copy_from_slice(&0u32.to_le_bytes());
        assert!(validate_generated_cache(&bad_magic)
            .unwrap_err()
            .contains("header"));

        let mut no_modules = vec![0u8; 16];
        no_modules.extend_from_slice(&crate::cache::header::CACHE_MAGIC.to_le_bytes());
        no_modules.extend_from_slice(&0u32.to_le_bytes());
        for _ in 0..crate::cache::tables::N_TABLES {
            no_modules.extend_from_slice(&0u32.to_le_bytes());
        }
        assert!(validate_generated_cache(&no_modules)
            .unwrap_err()
            .contains("zero modules"));

        assert!(validate_generated_cache(&good[..good.len() - 1])
            .unwrap_err()
            .contains("tail tables"));

        let mut trailing = good;
        trailing.push(0);
        assert!(validate_generated_cache(&trailing)
            .unwrap_err()
            .contains("file length"));
    }

    #[test]
    fn compile_target_and_layout_preflight_never_reset_or_run_regen_on_rejection() {
        let duplicate_paths =
            cache_with_empty_modules(&[("Alpha", "Dir/Foo.as"), ("Beta", "dir\\foo.AS")]);
        let prefix_paths =
            cache_with_empty_modules(&[("PrefixFile", "Foo.as"), ("PrefixChild", "foo.AS/Bar.as")]);
        let existing_directory = cache_with_empty_modules(&[("Nested", "Existing/Child.as")]);
        let cases = [
            (duplicate_paths, "add", "New", "New.as", "module layout"),
            (prefix_paths, "add", "New", "New.as", "module layout"),
            (
                valid_cache(),
                "edit",
                "TestModule",
                "Wrong.as",
                "does not match",
            ),
            (valid_cache(), "add", "testmodule", "New.as", "module name"),
            (
                valid_cache(),
                "add",
                "NewModule",
                "testmodule.AS",
                "add path",
            ),
            (
                valid_cache(),
                "add",
                "NewModule",
                "New?.as",
                "unsafe Windows output path",
            ),
            (
                valid_cache(),
                "add",
                "NewModule",
                "testmodule.AS/Child.as",
                "file/directory ancestor",
            ),
            (
                existing_directory,
                "add",
                "NewModule",
                "EXISTING",
                "file/directory ancestor",
            ),
        ];

        for (case, (base_cache, op, module_name, rel_path, expected)) in
            cases.into_iter().enumerate()
        {
            let root = std::env::temp_dir().join(format!(
                "gore-as-compile-preflight-{}-{case}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&root);
            let tree = root.join("tree");
            std::fs::create_dir_all(&tree).unwrap();
            let sentinel = tree.join("sentinel.as");
            std::fs::write(&sentinel, b"keep").unwrap();
            let source = root.join("overlay.as");
            std::fs::write(&source, b"// overlay").unwrap();
            let opts = CompileOpts {
                game_dir: root.join("game"),
                op: op.into(),
                module_name: module_name.into(),
                rel_path: rel_path.into(),
                as_path: source,
                work_dir: root.clone(),
                allow_new_symbols: false,
                base_override: Some(base_cache),
            };
            let called = std::cell::Cell::new(false);
            let error = compile_module(&opts, |_, _| {
                called.set(true);
                Err("regen must not run".into())
            })
            .unwrap_err();

            assert!(error.to_string().contains(expected), "{error}");
            assert!(!called.get(), "regen callback ran for case {case}");
            assert_eq!(std::fs::read(&sentinel).unwrap(), b"keep");
            assert!(!root.join("module.cache").exists());
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn mini_guid_is_canonicalized_to_base_without_changing_payload() {
        let mut mini = valid_cache();
        let mut base = mini.clone();
        mini[..16].fill(0x11);
        base[..16].fill(0xa5);
        let remainder = mini[16..].to_vec();
        let published_before =
            crate::cache::splice::replace_module(&base, &mini, "TestModule").unwrap();

        canonicalize_mini_guid(&mut mini, &base).unwrap();

        assert_eq!(&mini[..16], &base[..16]);
        assert_eq!(&mini[16..], remainder);
        let published_after =
            crate::cache::splice::replace_module(&base, &mini, "TestModule").unwrap();
        assert_eq!(published_after, published_before);
        assert_eq!(&published_after[..16], &base[..16]);
        assert!(canonicalize_mini_guid(&mut mini[..8], &base).is_err());
    }

    #[test]
    fn nonzero_generator_status_accepts_only_a_complete_cache() {
        let base = std::env::temp_dir().join("gore-as-nonzero-complete-cache");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let cache = base.join("PrecompiledScript.Cache");
        let good = valid_cache();
        std::fs::write(&cache, &good).unwrap();

        assert_eq!(
            read_completed_generated_cache(&cache, false, "exit code: 1").unwrap(),
            good,
            "G1R's post-generation exit code 1 is acceptable only with a fully valid cache"
        );

        std::fs::write(&cache, b"partial").unwrap();
        let err = read_completed_generated_cache(&cache, false, "exit code: 1").unwrap_err();
        assert!(err.contains("exited unsuccessfully"), "got: {err}");
        assert!(err.contains("incomplete"), "got: {err}");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn reset_compile_tree_removes_stale_scripts_and_rebuilds_empty_directory() {
        let base = std::env::temp_dir().join("gore-as-reset-tree");
        let _ = std::fs::remove_dir_all(&base);
        let tree = base.join("tree");
        std::fs::create_dir_all(tree.join("Old")).unwrap();
        std::fs::write(tree.join("Old").join("Stale.as"), b"stale").unwrap();

        let rebuilt = reset_compile_tree(&base).unwrap();
        assert_eq!(rebuilt, tree);
        assert!(rebuilt.is_dir());
        assert_eq!(std::fs::read_dir(&rebuilt).unwrap().count(), 0);

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Invoked only as a subprocess by `child_wait_timeout_is_hard_and_bounded`. Keeping it ignored
    /// prevents an ordinary test run from sleeping; the private environment flag prevents even an
    /// explicit `--ignored` run from doing so accidentally.
    #[test]
    #[ignore = "subprocess helper for the timeout test"]
    fn timeout_helper_process() {
        if std::env::var_os("GORE_AS_TIMEOUT_HELPER_PROCESS").is_some() {
            std::thread::sleep(Duration::from_secs(30));
        }
    }

    #[test]
    fn child_wait_timeout_is_hard_and_bounded() {
        let _serial = PROCESS_TIMEOUT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let test_exe = std::env::current_exe().unwrap();
        let mut child = std::process::Command::new(test_exe)
            .args([
                "--ignored",
                "--exact",
                "compile::tests::timeout_helper_process",
                "--test-threads=1",
            ])
            .env("GORE_AS_TIMEOUT_HELPER_PROCESS", "1")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();

        let timeout = Duration::from_millis(150);
        let termination_grace = Duration::from_millis(750);
        let started = Instant::now();
        let err = wait_for_child_with_timeout(
            &mut child,
            timeout,
            Duration::from_millis(10),
            termination_grace,
            "timeout-test helper",
        )
        .unwrap_err();
        let elapsed = started.elapsed();

        assert!(err.contains("exceeded"), "got: {err}");
        assert!(
            err.contains("terminated") || err.contains(GENERATOR_EXIT_UNCONFIRMED),
            "timeout must report either confirmed termination or the fail-closed unconfirmed-exit marker; got: {err}"
        );
        assert!(
            elapsed <= timeout + termination_grace + Duration::from_secs(1),
            "timeout path exceeded its hard bounds: elapsed={elapsed:?}, error={err}"
        );
        // A heavily loaded Windows host can consume the deliberately short production-observation
        // window inside `taskkill` itself. The production path correctly reports that as
        // unconfirmed and preserves isolation. Give the test helper a separate best-effort cleanup
        // window so a valid fail-closed result neither leaks the sleeper nor poisons the serial
        // mutex for the next test.
        let cleanup_deadline = Instant::now() + Duration::from_secs(5);
        while child.try_wait().unwrap().is_none() && Instant::now() < cleanup_deadline {
            let _ = child.kill();
            std::thread::sleep(Duration::from_millis(25));
        }
        assert!(
            child.try_wait().unwrap().is_some(),
            "direct helper child must exit during the test-only cleanup window"
        );
    }

    #[test]
    fn game_run_regen_quarantines_jitted_code_and_clear_ue4ss_proxy_then_restores() {
        let base = std::env::temp_dir().join("gore-as-game-isolation");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let jitted = base.join("AS_JITTED_CODE");
        std::fs::create_dir_all(&jitted).unwrap();
        std::fs::write(jitted.join("old.bin"), b"OLD-JIT").unwrap();
        let win64 = base.join("G1R").join("Binaries").join("Win64");
        let payload = win64.join("ue4ss").join("UE4SS.dll");
        std::fs::create_dir_all(payload.parent().unwrap()).unwrap();
        std::fs::write(&payload, b"UE4SS").unwrap();
        let proxy = win64.join("dwmapi.dll");
        std::fs::write(&proxy, b"OLD-PROXY").unwrap();

        game_run_regen_with(&game, &src, |_, _, dev| {
            assert!(!jitted.exists(), "old JIT dir must be quarantined");
            assert!(!proxy.exists(), "UE4SS proxy must be disabled");
            std::fs::create_dir_all(&jitted).unwrap();
            std::fs::write(jitted.join("new.bin"), b"NEW-JIT").unwrap();
            std::fs::write(&proxy, b"UNEXPECTED-NEW-PROXY").unwrap();
            let bytes = valid_cache();
            std::fs::write(dev, &bytes).unwrap();
            Ok(bytes)
        })
        .unwrap();

        assert_eq!(std::fs::read(jitted.join("old.bin")).unwrap(), b"OLD-JIT");
        assert!(!jitted.join("new.bin").exists());
        assert_eq!(std::fs::read(&proxy).unwrap(), b"OLD-PROXY");
        assert!(!append_suffix(&jitted, ".gore-compile-bak").exists());
        assert!(!append_suffix(&proxy, ".gore-compile-bak").exists());
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn game_run_regen_restores_absent_side_effect_paths_on_failure() {
        let base = std::env::temp_dir().join("gore-as-game-isolation-absent");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();
        let jitted = base.join("AS_JITTED_CODE");
        let proxy = base
            .join("G1R")
            .join("Binaries")
            .join("Win64")
            .join("dwmapi.dll");

        let err = game_run_regen_with(&game, &src, |_, _, dev| {
            std::fs::create_dir_all(&jitted).unwrap();
            std::fs::write(jitted.join("new.bin"), b"NEW-JIT").unwrap();
            std::fs::write(&proxy, b"NEW-PROXY").unwrap();
            std::fs::write(dev, b"partial").unwrap();
            Err("generation failed".into())
        })
        .unwrap_err();

        assert!(err.contains("generation failed"), "got: {err}");
        assert!(!jitted.exists());
        assert!(!proxy.exists());
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn regen_diagnostics_report_keeps_legacy_outer_error_before_runner_invocation() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-regen-report-before-runner-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        // The reserved path makes isolation planning fail after the transaction has created its
        // recovery artifacts, but before any generator/diagnostics runner can be invoked.
        let jitted = base.join("AS_JITTED_CODE");
        let collision = append_suffix(&jitted, ".gore-compile-bak");
        std::fs::create_dir(&collision).unwrap();

        let error = game_run_regen_with_diagnostics_report(&game, &src, &Default::default())
            .expect_err("the public report requires a diagnostics-runner disposition");
        assert!(
            error.contains("compile quarantine backup already exists"),
            "got: {error}"
        );

        let report =
            game_run_regen_with_extended_diagnostics_report(&game, &src, &Default::default())
                .expect("the internal module-compile report retains transactional status");
        assert!(report.diagnostics.is_none());
        assert_eq!(
            report.install_restore,
            InstallRestoreDisposition::RestoredExact
        );
        assert!(report
            .result
            .unwrap_err()
            .contains("compile quarantine backup already exists"));
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
        assert!(!compile_bak_path(&shipping).exists());
        assert!(!recovery_journal_path(&game).exists());
        assert!(!compile_lock_path(&game).exists());

        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn partial_isolation_begin_failure_retains_recovery_and_never_reports_exact_restore() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-partial-isolation-failure-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let jitted = base.join("AS_JITTED_CODE");
        std::fs::create_dir(&jitted).unwrap();
        std::fs::write(jitted.join("old.bin"), b"OLD-JIT").unwrap();
        let jitted_backup = append_suffix(&jitted, ".gore-compile-bak");
        let win64 = base.join("G1R").join("Binaries").join("Win64");
        let payload = win64.join("ue4ss").join("UE4SS.dll");
        std::fs::create_dir_all(payload.parent().unwrap()).unwrap();
        std::fs::write(&payload, b"UE4SS").unwrap();
        let proxy = win64.join("dwmapi.dll");
        std::fs::write(&proxy, b"OLD-PROXY").unwrap();

        let generator_calls = std::cell::Cell::new(0);
        let report = game_run_regen_with_install_report_and(
            &game,
            &src,
            |txn| {
                txn.begin_isolation_after_jitted(|| {
                    // JIT has been moved. Make proxy activation fail, then block JIT restoration
                    // with the wrong path type so the partial begin cannot clean itself up.
                    std::fs::remove_file(&proxy).unwrap();
                    std::fs::write(&jitted, b"RESTORE-BLOCKER").unwrap();
                })
            },
            |_, _, _| {
                generator_calls.set(generator_calls.get() + 1);
                GeneratorRunResult::confirmed(Ok(valid_cache()))
            },
        )
        .unwrap();

        assert_eq!(generator_calls.get(), 0, "runner must not be invoked");
        assert_eq!(
            report.install_restore,
            InstallRestoreDisposition::RecoveryRequiredRestoreFailed
        );
        let error = report.result.unwrap_err();
        assert!(error.contains("quarantining"), "got: {error}");
        assert!(
            error.contains("failed to restore generation isolation"),
            "got: {error}"
        );
        assert!(jitted_backup.exists(), "JIT recovery must remain");
        assert_eq!(
            std::fs::read(&jitted_backup.join("old.bin")).unwrap(),
            b"OLD-JIT"
        );
        assert_eq!(std::fs::read(compile_bak_path(&shipping)).unwrap(), b"OLD");
        assert!(
            recovery_journal_path(&game).exists(),
            "journal must not be retired after a failed isolation restore"
        );
        assert!(
            !compile_lock_path(&game).exists(),
            "no process started, so manual recovery must not retain the compile lock"
        );

        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn regen_report_marks_confirmed_syntax_failure_as_restored_exact() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-regen-report-syntax-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Broken.as"), b"void Broken( {").unwrap();

        let report = game_run_regen_with_install_report(&game, &src, |_, _, _| {
            GeneratorRunResult::confirmed(Err("AngelScript syntax/regen failure".to_owned()))
        })
        .unwrap();

        assert_eq!(
            report.install_restore,
            InstallRestoreDisposition::RestoredExact
        );
        assert!(report.result.unwrap_err().contains("syntax/regen failure"));
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
        assert!(!compile_bak_path(&shipping).exists());
        assert!(!recovery_journal_path(&game).exists());
        assert!(!compile_lock_path(&game).exists());
        assert!(!shipping.parent().unwrap().join("Broken.as").exists());

        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn regen_report_marks_structured_unconfirmed_exit_as_recovery_required() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-regen-report-unconfirmed-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let report = game_run_regen_with_install_report(&game, &src, |_, _, _| {
            // Deliberately omit the legacy text marker: the disposition must drive recovery.
            GeneratorRunResult::unconfirmed("simulated generator still alive".to_owned())
        })
        .unwrap();

        assert_eq!(
            report.install_restore,
            InstallRestoreDisposition::RecoveryRequiredProcessExitUnconfirmed
        );
        assert!(report.result.unwrap_err().contains("intentionally NOT run"));
        assert!(compile_bak_path(&shipping).exists());
        assert!(recovery_journal_path(&game).exists());
        assert!(compile_lock_path(&game).exists());
        assert!(shipping.parent().unwrap().join("Mod.as").exists());

        // The fake runner has no real process. Removing the isolated fixture is its test-only
        // equivalent of following the retained recovery instructions.
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn regen_report_marks_failed_restore_with_retained_backup_as_recovery_required() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-regen-report-restore-failure-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Broken.as"), b"void Broken( {").unwrap();

        let report = game_run_regen_with_install_report(&game, &src, |_, _, _| {
            std::fs::remove_file(&shipping).unwrap();
            std::fs::create_dir(&shipping).unwrap();
            GeneratorRunResult::confirmed(Err("AngelScript syntax/regen failure".to_owned()))
        })
        .unwrap();

        assert_eq!(
            report.install_restore,
            InstallRestoreDisposition::RecoveryRequiredRestoreFailed
        );
        let error = report.result.unwrap_err();
        assert!(error.contains("syntax/regen failure"), "got: {error}");
        assert!(error.contains("FAILED to restore"), "got: {error}");
        let recovery = compile_bak_path(&shipping);
        assert_eq!(std::fs::read(&recovery).unwrap(), b"OLD");
        assert!(recovery_journal_path(&game).exists());
        assert!(
            !compile_lock_path(&game).exists(),
            "a confirmed-dead generator must release the lock for manual recovery"
        );

        std::fs::remove_dir(&shipping).unwrap();
        std::fs::rename(&recovery, &shipping).unwrap();
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn game_run_regen_leaves_unidentified_dwmapi_untouched() {
        let base = std::env::temp_dir().join("gore-as-game-non-ue4ss-proxy");
        let _ = std::fs::remove_dir_all(&base);
        let (game, _) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();
        let proxy = base
            .join("G1R")
            .join("Binaries")
            .join("Win64")
            .join("dwmapi.dll");
        std::fs::write(&proxy, b"NOT-KNOWN-TO-BE-UE4SS").unwrap();

        game_run_regen_with(&game, &src, |_, _, dev| {
            assert_eq!(std::fs::read(&proxy).unwrap(), b"NOT-KNOWN-TO-BE-UE4SS");
            let bytes = valid_cache();
            std::fs::write(dev, &bytes).unwrap();
            Ok(bytes)
        })
        .unwrap();
        assert_eq!(std::fs::read(&proxy).unwrap(), b"NOT-KNOWN-TO-BE-UE4SS");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn game_run_regen_refuses_existing_compile_and_quarantine_backups() {
        for which in ["cache", "jitted"] {
            let base = std::env::temp_dir().join(format!("gore-as-backup-collision-{which}"));
            let _ = std::fs::remove_dir_all(&base);
            let (game, shipping) = fake_install(&base);
            let src = base.join("src");
            std::fs::create_dir_all(&src).unwrap();
            std::fs::write(src.join("Mod.as"), b"script").unwrap();
            let collision = if which == "cache" {
                compile_bak_path(&shipping)
            } else {
                append_suffix(&base.join("AS_JITTED_CODE"), ".gore-compile-bak")
            };
            if which == "jitted" {
                std::fs::create_dir_all(base.join("AS_JITTED_CODE")).unwrap();
            }
            std::fs::write(&collision, b"KEEP-ME").unwrap();

            let err = game_run_regen_with(&game, &src, |_, _, _| panic!("must not generate"))
                .unwrap_err();
            assert!(
                err.contains("backup already exists"),
                "which={which}: {err}"
            );
            assert_eq!(std::fs::read(&collision).unwrap(), b"KEEP-ME");
            assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");

            let _ = std::fs::remove_dir_all(&base);
        }
    }

    #[test]
    fn game_run_regen_keeps_compile_backup_if_shipping_restore_fails() {
        let base = std::env::temp_dir().join("gore-as-backup-restore-failure");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let err = game_run_regen_with(&game, &src, |_, _, dev| {
            std::fs::remove_file(&shipping).unwrap();
            std::fs::create_dir(&shipping).unwrap();
            let bytes = valid_cache();
            std::fs::write(dev, &bytes).unwrap();
            Ok(bytes)
        })
        .unwrap_err();

        let backup = compile_bak_path(&shipping);
        assert!(err.contains("FAILED to restore"), "got: {err}");
        assert_eq!(std::fs::read(&backup).unwrap(), b"OLD");
        assert!(shipping.is_dir());

        // Manual recovery for the fake install, mirroring the error's instruction.
        std::fs::remove_dir(&shipping).unwrap();
        std::fs::rename(&backup, &shipping).unwrap();
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_refuses_existing_shipping_recovery_backup_without_mutation() {
        let base = std::env::temp_dir().join("gore-as-precompile-recovery-collision");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let backup = compile_bak_path(&shipping);
        std::fs::write(&backup, b"KEEP-RECOVERY").unwrap();
        let opts = PrecompileOpts {
            game_dir: game.clone(),
            src: None,
            out: Some(base.join("out.Cache")),
            backup: false,
        };

        let err = precompile_with(&opts, |_, _, _| panic!("must not generate")).unwrap_err();
        assert!(err.contains("compile backup already exists"), "got: {err}");
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
        assert_eq!(std::fs::read(&backup).unwrap(), b"KEEP-RECOVERY");
        assert!(!compile_lock_path(&game).exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_preserves_recovery_backup_when_shipping_restore_fails() {
        let base = std::env::temp_dir().join("gore-as-precompile-restore-failure");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let opts = PrecompileOpts {
            game_dir: game.clone(),
            src: None,
            out: Some(base.join("out.Cache")),
            backup: false,
        };

        let err = precompile_with(&opts, |_, _, dev| {
            std::fs::remove_file(&shipping).unwrap();
            std::fs::create_dir(&shipping).unwrap();
            let bytes = valid_cache();
            std::fs::write(dev, &bytes).unwrap();
            Ok(bytes)
        })
        .unwrap_err();

        let recovery = compile_bak_path(&shipping);
        assert!(err.contains("FAILED to restore"), "got: {err}");
        assert_eq!(std::fs::read(&recovery).unwrap(), b"OLD");
        assert!(shipping.is_dir());
        assert!(!compile_lock_path(&game).exists());

        std::fs::remove_dir(&shipping).unwrap();
        std::fs::rename(&recovery, &shipping).unwrap();
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn common_compile_lock_rejects_parallel_cross_entry_point_compile() {
        let _serial = PROCESS_TIMEOUT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let base = std::env::temp_dir().join("gore-as-parallel-compile-lock");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let game_for_thread = game.clone();
        let src_for_thread = src.clone();

        let first = std::thread::spawn(move || {
            game_run_regen_with(&game_for_thread, &src_for_thread, |_, _, dev| {
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                let bytes = valid_cache();
                std::fs::write(dev, &bytes).unwrap();
                Ok(bytes)
            })
        });
        entered_rx.recv().unwrap();
        assert!(compile_lock_path(&game).exists());

        let second_opts = PrecompileOpts {
            game_dir: game.clone(),
            src: None,
            out: Some(base.join("parallel-out.Cache")),
            backup: false,
        };
        let second =
            precompile_with(&second_opts, |_, _, _| panic!("must not generate")).unwrap_err();
        assert!(second.contains("compile is active"), "got: {second}");

        release_tx.send(()).unwrap();
        first.join().unwrap().unwrap();
        assert!(!compile_lock_path(&game).exists());
        assert!(!compile_bak_path(&shipping).exists());
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn panic_unwind_restores_entire_compile_transaction() {
        let base = std::env::temp_dir().join("gore-as-compile-panic-rollback");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let dev = dev_cache(&shipping);
        std::fs::write(&dev, b"DEV-OLD").unwrap();
        let live_mod = shipping.parent().unwrap().join("Mod.as");
        std::fs::write(&live_mod, b"LIVE-OLD").unwrap();
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"STAGED").unwrap();

        let jitted = base.join("AS_JITTED_CODE");
        std::fs::create_dir_all(&jitted).unwrap();
        std::fs::write(jitted.join("old.bin"), b"JIT-OLD").unwrap();
        let win64 = base.join("G1R").join("Binaries").join("Win64");
        let payload = win64.join("ue4ss").join("UE4SS.dll");
        std::fs::create_dir_all(payload.parent().unwrap()).unwrap();
        std::fs::write(&payload, b"UE4SS").unwrap();
        let proxy = win64.join("dwmapi.dll");
        std::fs::write(&proxy, b"PROXY-OLD").unwrap();

        let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = game_run_regen_with(&game, &src, |_, _, dev_path| {
                assert!(!jitted.exists());
                assert!(!proxy.exists());
                std::fs::write(&shipping, b"SHIPPING-PARTIAL").unwrap();
                std::fs::write(dev_path, b"DEV-PARTIAL").unwrap();
                std::fs::create_dir_all(&jitted).unwrap();
                std::fs::write(jitted.join("new.bin"), b"JIT-NEW").unwrap();
                std::fs::write(&proxy, b"PROXY-NEW").unwrap();
                panic!("injected generator panic");
                #[allow(unreachable_code)]
                Ok(Vec::new())
            });
        }));
        assert!(unwind.is_err());

        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
        assert_eq!(std::fs::read(&dev).unwrap(), b"DEV-OLD");
        assert_eq!(std::fs::read(&live_mod).unwrap(), b"LIVE-OLD");
        assert_eq!(std::fs::read(jitted.join("old.bin")).unwrap(), b"JIT-OLD");
        assert!(!jitted.join("new.bin").exists());
        assert_eq!(std::fs::read(&proxy).unwrap(), b"PROXY-OLD");
        assert!(!compile_bak_path(&shipping).exists());
        assert!(!compile_lock_path(&game).exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_rejects_structurally_incomplete_cache_and_rolls_back() {
        let base = std::env::temp_dir().join("gore-as-invalid-generated-cache");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();
        let opts = PrecompileOpts {
            game_dir: game,
            src: Some(src),
            out: None,
            backup: true,
        };

        let err = precompile_with(&opts, |_, _, dev| {
            std::fs::write(dev, b"not-a-cache").unwrap();
            Ok(b"not-a-cache".to_vec())
        })
        .unwrap_err();
        assert!(err.contains("invalid generated cache"), "got: {err}");
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
        assert!(!dev_cache(&shipping).exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn game_run_regen_uses_dev_cache_and_restores_both_caches_and_colliding_source() {
        let base = std::env::temp_dir().join("gore-as-game-regen-dev");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let dev = dev_cache(&shipping);
        std::fs::write(&dev, b"DEV-OLD").unwrap();
        // A matching loose path is safe: staging overwrites it, then cleanup must restore it.
        let live_mod = shipping.parent().unwrap().join("Mod.as");
        std::fs::write(&live_mod, b"LIVE-OLD").unwrap();
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"STAGED-NEW").unwrap();

        let regen = game_run_regen_with(&game, &src, |_, _, dev_path| {
            assert_eq!(dev_path, dev);
            assert!(
                !dev_path.exists(),
                "stale dev cache must be removed before generation"
            );
            // Even an unexpected Shipping write by the game must be undone.
            std::fs::write(
                dev_path
                    .parent()
                    .unwrap()
                    .join("PrecompiledScript_Shipping.Cache"),
                b"TOUCHED",
            )
            .unwrap();
            let bytes = valid_cache();
            std::fs::write(dev_path, &bytes).unwrap();
            Ok(bytes)
        })
        .unwrap();

        assert_eq!(std::fs::read(regen).unwrap(), valid_cache());
        assert_eq!(
            std::fs::read(&shipping).unwrap(),
            b"OLD",
            "Shipping restored exactly"
        );
        assert_eq!(
            std::fs::read(&dev).unwrap(),
            b"DEV-OLD",
            "old dev cache restored"
        );
        assert_eq!(
            std::fs::read(&live_mod).unwrap(),
            b"LIVE-OLD",
            "colliding source restored"
        );
        assert!(!shipping.with_extension("Cache.gore-compile-bak").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn game_run_regen_failure_removes_new_dev_cache_and_restores_shipping() {
        let base = std::env::temp_dir().join("gore-as-game-regen-fail");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let dev = dev_cache(&shipping);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let err = game_run_regen_with(&game, &src, |_, _, dev_path| {
            std::fs::write(dev_path, b"PARTIAL-DEV").unwrap();
            std::fs::write(
                dev_path
                    .parent()
                    .unwrap()
                    .join("PrecompiledScript_Shipping.Cache"),
                b"PARTIAL-SHIPPING",
            )
            .unwrap();
            Err("compile failed".into())
        })
        .unwrap_err();

        assert!(err.contains("compile failed"), "got: {err}");
        assert_eq!(std::fs::read(&shipping).unwrap(), b"OLD");
        assert!(
            !dev.exists(),
            "new dev cache removed when none existed before"
        );
        assert!(!shipping.parent().unwrap().join("Mod.as").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn game_run_regen_rejects_uncovered_loose_script_before_staging() {
        let base = std::env::temp_dir().join("gore-as-game-regen-stray");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let stray = shipping.parent().unwrap().join("OnlyLive.as");
        std::fs::write(&stray, b"do not compile me").unwrap();
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Wanted.as"), b"wanted").unwrap();

        let err =
            game_run_regen_with(&game, &src, |_, _, _| panic!("must not generate")).unwrap_err();
        assert!(err.contains("not present in the staged tree"), "got: {err}");
        assert_eq!(std::fs::read(&stray).unwrap(), b"do not compile me");
        assert!(!shipping.with_extension("Cache.gore-compile-bak").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_out_mode_writes_artifact_and_restores_install() {
        let base = std::env::temp_dir().join("gore-as-compile-out");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        let dev = dev_cache(&cache);
        std::fs::write(&dev, b"DEV-OLD").unwrap();
        let src = base.join("src");
        std::fs::create_dir_all(src.join("AI")).unwrap();
        std::fs::write(src.join("AI").join("Mod.as"), b"script").unwrap();
        let live_src = cache.parent().unwrap().join("AI").join("Mod.as");
        std::fs::create_dir_all(live_src.parent().unwrap()).unwrap();
        std::fs::write(&live_src, b"live-script").unwrap();
        let out = base.join("out.Cache");

        let opts = PrecompileOpts {
            game_dir: game,
            src: Some(src),
            out: Some(out.clone()),
            backup: true,
        };
        let res = precompile_with(&opts, gen_new).unwrap();

        assert_eq!(res, out);
        assert_eq!(
            std::fs::read(&out).unwrap(),
            valid_cache(),
            "artifact holds the compiled bytes"
        );
        assert_eq!(
            std::fs::read(&cache).unwrap(),
            b"OLD",
            "live cache restored (install pristine)"
        );
        assert_eq!(
            std::fs::read(&dev).unwrap(),
            b"DEV-OLD",
            "old dev cache restored exactly"
        );
        assert_eq!(
            std::fs::read(&live_src).unwrap(),
            b"live-script",
            "covered pre-existing source is restored exactly"
        );
        assert!(
            !deploy_bak_path(&cache).exists(),
            "out-mode leaves no .gore-bak"
        );

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
        assert_eq!(
            std::fs::read(&cache).unwrap(),
            valid_cache(),
            "new cache installed in place"
        );
        assert!(
            !dev_cache(&cache).exists(),
            "new dev cache removed after in-place install"
        );
        assert_eq!(
            std::fs::read(deploy_bak_path(&cache)).unwrap(),
            b"OLD",
            "previous cache backed up to .gore-bak"
        );
        assert!(
            !cache.parent().unwrap().join("Mod.as").exists(),
            "staged .as cleaned"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_in_place_rejects_directory_as_existing_deploy_backup() {
        let base = std::env::temp_dir().join("gore-as-compile-invalid-deploy-backup");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        let backup = deploy_bak_path(&cache);
        std::fs::create_dir(&backup).unwrap();
        let opts = PrecompileOpts {
            game_dir: game.clone(),
            src: None,
            out: None,
            backup: true,
        };

        let err = precompile_with(&opts, gen_new).unwrap_err();

        assert!(
            err.contains("not a regular non-reparse file"),
            "unexpected error: {err}"
        );
        assert_eq!(
            std::fs::read(&cache).unwrap(),
            b"OLD",
            "invalid backup must fail before installing generated bytes"
        );
        assert!(backup.is_dir(), "the rejected path must remain untouched");
        assert!(!dev_cache(&cache).exists());
        assert!(!compile_bak_path(&cache).exists());
        assert!(!compile_lock_path(&game).exists());
        assert!(!recovery_journal_path(&game).exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_in_place_preserves_valid_existing_deploy_backup() {
        let base = std::env::temp_dir().join("gore-as-compile-existing-deploy-backup");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        let backup = deploy_bak_path(&cache);
        std::fs::write(&backup, b"EARLIEST").unwrap();
        let opts = PrecompileOpts {
            game_dir: game,
            src: None,
            out: None,
            backup: true,
        };

        precompile_with(&opts, gen_new).unwrap();

        assert_eq!(std::fs::read(&cache).unwrap(), valid_cache());
        assert_eq!(
            std::fs::read(&backup).unwrap(),
            b"EARLIEST",
            "a valid existing backup must never be overwritten"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_in_place_rejects_link_as_existing_deploy_backup() {
        let base = std::env::temp_dir().join("gore-as-compile-linked-deploy-backup");
        let _ = std::fs::remove_dir_all(&base);
        let (game, cache) = fake_install(&base);
        let backup = deploy_bak_path(&cache);
        let link_target = base.join("not-the-backup.Cache");
        std::fs::write(&link_target, b"DO-NOT-USE").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&link_target, &backup).unwrap();
        #[cfg(windows)]
        if std::os::windows::fs::symlink_file(&link_target, &backup).is_err() {
            // Windows without Developer Mode/elevation cannot create this fixture. Reparse paths
            // still hit the same production predicate exercised by the staging-link test.
            let _ = std::fs::remove_dir_all(&base);
            return;
        }

        let opts = PrecompileOpts {
            game_dir: game,
            src: None,
            out: None,
            backup: true,
        };
        let err = precompile_with(&opts, gen_new).unwrap_err();

        assert!(
            err.contains("not a regular non-reparse file"),
            "unexpected error: {err}"
        );
        assert_eq!(std::fs::read(&cache).unwrap(), b"OLD");
        assert_eq!(std::fs::read(&link_target).unwrap(), b"DO-NOT-USE");

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
        std::fs::write(src.join("PrecompiledScript.Cache"), b"STALE-DEV").unwrap();
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
            valid_cache(),
            "freshly compiled cache kept, not clobbered by the staged src cache file"
        );
        assert!(
            !dev_cache(&cache).exists(),
            "staged/generated development cache cleaned"
        );
        assert!(
            !cache.parent().unwrap().join("Mod.as").exists(),
            "staged .as cleaned"
        );

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
        let err =
            precompile_with(&opts, |_, _, _| panic!("must not stage or generate")).unwrap_err();
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
        let dev = dev_cache(&cache);
        std::fs::write(&dev, b"DEV-OLD").unwrap();
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let opts = PrecompileOpts {
            game_dir: game,
            src: Some(src),
            out: None,
            backup: true,
        };
        // Stub emulates a partial development-cache write plus an unexpected Shipping write.
        let err = precompile_with(&opts, |_, _, dev_cache| {
            std::fs::write(dev_cache, b"PARTIAL-DEV").unwrap();
            std::fs::write(
                dev_cache
                    .parent()
                    .unwrap()
                    .join("PrecompiledScript_Shipping.Cache"),
                b"PARTIAL-SHIPPING",
            )
            .unwrap();
            Err("boom".to_string())
        })
        .unwrap_err();

        assert!(
            err.contains("boom"),
            "surfaces the original error; got: {err}"
        );
        assert_eq!(
            std::fs::read(&cache).unwrap(),
            b"OLD",
            "live cache rolled back"
        );
        assert_eq!(
            std::fs::read(&dev).unwrap(),
            b"DEV-OLD",
            "development cache rolled back"
        );
        assert!(
            !cache.parent().unwrap().join("Mod.as").exists(),
            "staged .as removed on rollback"
        );
        assert!(
            !deploy_bak_path(&cache).exists(),
            "failed generation must not create a persistent deploy backup"
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
            assert!(
                err.contains("Script/ directory"),
                "out={:?} got: {err}",
                out
            );
        }
        assert_eq!(
            std::fs::read(&cache).unwrap(),
            b"OLD",
            "live cache left untouched"
        );
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

        assert!(
            err.contains("writing output"),
            "surfaces the write error; got: {err}"
        );
        assert_eq!(
            std::fs::read(&cache).unwrap(),
            b"OLD",
            "live cache restored despite write failure"
        );
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
        assert!(
            rel.starts_with(&base_real),
            "relative out resolved under cwd: {rel:?}"
        );

        // An absolute out elsewhere stays where it is (not under cwd).
        let other = std::env::temp_dir()
            .join("gore-as-resolve-other")
            .join("x.Cache");
        let abs = resolve_out_real(&other, &base);
        assert!(
            !abs.starts_with(&base_real),
            "absolute out stays put: {abs:?}"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn game_run_regen_report_keeps_legacy_diagnostics_and_into_parts_signatures() {
        let report = GameRunRegenReport {
            result: Err("simulated compiler rejection".to_owned()),
            diagnostics: crate::diagnostics::CompilerDiagnosticsReport::empty(
                crate::diagnostics::DiagnosticsCaptureDisposition::Disabled,
            ),
            install_restore: InstallRestoreDisposition::RestoredExact,
        };

        let diagnostics: &crate::diagnostics::CompilerDiagnosticsReport = report.diagnostics();
        assert_eq!(
            diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::Disabled
        );
        assert_eq!(
            report.install_restore_disposition(),
            InstallRestoreDisposition::RestoredExact
        );
        let (_result, diagnostics): (
            Result<PathBuf, String>,
            crate::diagnostics::CompilerDiagnosticsReport,
        ) = report.into_parts();
        assert_eq!(
            diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::Disabled
        );
    }

    #[test]
    fn compile_module_report_retains_structured_diagnostics_on_success_and_failure() {
        let root = std::env::temp_dir().join(format!(
            "gore-as-compile-report-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("overlay.as");
        std::fs::write(&source, b"// generated module\n").unwrap();
        let opts = CompileOpts {
            game_dir: root.join("game"),
            op: "add".to_owned(),
            module_name: "NewModule".to_owned(),
            rel_path: "NewModule.as".to_owned(),
            as_path: source,
            work_dir: root.join("work"),
            allow_new_symbols: true,
            base_override: Some(cache_with_empty_modules(&[("Base", "Base.as")])),
        };
        let generated =
            cache_with_empty_modules(&[("Base", "Base.as"), ("NewModule", "NewModule.as")]);
        let success = compile_module_report_with(&opts, |_, _| {
            let path = root.join("generated.cache");
            std::fs::write(&path, &generated).unwrap();
            let report = GameRunRegenExtendedReport {
                result: Ok(path),
                diagnostics: Some(
                    crate::diagnostics::CompilerDiagnosticsReport::from_bounded_capture(
                        crate::diagnostics::DiagnosticsCaptureDisposition::Captured,
                        "=== NewModule.as ===\n(1:1) [W] retained warning\n",
                    )
                    .unwrap(),
                ),
                install_restore: InstallRestoreDisposition::RestoredExact,
            };
            assert_eq!(
                report.install_restore,
                InstallRestoreDisposition::RestoredExact
            );
            Ok(report)
        });
        assert!(matches!(
            &success.outcome,
            CompileModuleReportOutcome::Compiled(_)
        ));
        let diagnostics = success.diagnostics().unwrap();
        assert_eq!(
            diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::Captured
        );
        assert_eq!(diagnostics.diagnostics().len(), 1);
        assert_eq!(diagnostics.diagnostics()[0].message, "retained warning");
        assert_eq!(
            success.install_restore_disposition(),
            InstallRestoreDisposition::RestoredExact
        );

        let failed = compile_module_report_with(&opts, |_, _| {
            Ok(GameRunRegenExtendedReport {
                result: Err("compiler rejected the source".to_owned()),
                diagnostics: Some(
                    crate::diagnostics::CompilerDiagnosticsReport::from_bounded_capture(
                        crate::diagnostics::DiagnosticsCaptureDisposition::Captured,
                        "=== NewModule.as ===\n(3:4) [E] broken expression\n",
                    )
                    .unwrap(),
                ),
                install_restore: InstallRestoreDisposition::RestoredExact,
            })
        });
        assert!(matches!(
            &failed.outcome,
            CompileModuleReportOutcome::Failed(CompileError::Regen(_))
        ));
        assert_eq!(
            failed.diagnostics().unwrap().diagnostics()[0].message,
            "broken expression"
        );
        assert_eq!(
            failed.install_restore_disposition(),
            InstallRestoreDisposition::RestoredExact,
            "an ordinary compiler rejection still restores the install exactly"
        );

        let recovery_required = compile_module_report_with(&opts, |_, _| {
            Ok(GameRunRegenExtendedReport {
                result: Err("generator exit could not be confirmed".to_owned()),
                diagnostics: Some(crate::diagnostics::CompilerDiagnosticsReport::empty(
                    crate::diagnostics::DiagnosticsCaptureDisposition::ProcessExitUnconfirmed,
                )),
                install_restore: InstallRestoreDisposition::RecoveryRequiredProcessExitUnconfirmed,
            })
        });
        assert!(matches!(
            &recovery_required.outcome,
            CompileModuleReportOutcome::Failed(CompileError::Regen(_))
        ));
        assert_eq!(
            recovery_required.install_restore_disposition(),
            InstallRestoreDisposition::RecoveryRequiredProcessExitUnconfirmed
        );

        let recovery_before_runner = compile_module_report_with(&opts, |_, _| {
            Ok(GameRunRegenExtendedReport {
                result: Err("isolation setup failed and its rollback also failed".to_owned()),
                diagnostics: None,
                install_restore: InstallRestoreDisposition::RecoveryRequiredRestoreFailed,
            })
        });
        assert!(matches!(
            &recovery_before_runner.outcome,
            CompileModuleReportOutcome::Failed(CompileError::Regen(_))
        ));
        assert!(recovery_before_runner.diagnostics().is_none());
        assert_eq!(
            recovery_before_runner.install_restore_disposition(),
            InstallRestoreDisposition::RecoveryRequiredRestoreFailed,
            "the report must be stored before its inner compiler error is returned"
        );

        let mut invalid_opts = opts;
        invalid_opts.op = "invalid".to_owned();
        let not_run = compile_module_report_with(&invalid_opts, |_, _| {
            panic!("invalid preflight must not launch the compiler")
        });
        assert!(matches!(
            &not_run.outcome,
            CompileModuleReportOutcome::Failed(CompileError::Other(_))
        ));
        assert!(not_run.diagnostics().is_none());
        assert_eq!(
            not_run.install_restore_disposition(),
            InstallRestoreDisposition::NotStarted
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn no_match_fallback_returns_the_normal_generator_result() {
        let called = std::cell::Cell::new(0);
        let report = resolve_diagnostic_attempt_report(
            DiagnosticAttempt::Unavailable("signature matched 0 times".into()),
            || {
                called.set(called.get() + 1);
                GeneratorRunResult::confirmed(Ok::<_, String>(b"real-cache".to_vec()))
            },
        );
        assert_eq!(report.result.unwrap(), b"real-cache");
        assert_eq!(
            report.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::UnavailableFallback
        );
        assert!(report.diagnostics.diagnostics().is_empty());
        assert_eq!(called.get(), 1);
    }

    #[test]
    fn captured_compiler_error_rejects_a_structurally_complete_cache() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-captured-error-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&base).unwrap();
        let artifacts = DiagnosticArtifacts {
            capture: base.join("capture.txt"),
            status: base.join("status.txt"),
            dir: base.clone(),
            cleanup: true,
        };
        std::fs::write(
            &artifacts.capture,
            "=== Test.as ===\n(4:2) [E] No matching signatures to 'Broken()'\n",
        )
        .unwrap();
        let complete = valid_cache();
        validate_generated_cache(&complete).expect("fixture must be structurally complete");
        let captured = append_captured_diagnostics(
            GeneratorRunResult::confirmed(Ok(complete)),
            &artifacts,
            crate::diagnostics::DiagnosticsCaptureDisposition::Captured,
        );
        assert_eq!(
            captured.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::Captured
        );
        assert_eq!(captured.diagnostics.diagnostics().len(), 1);
        let error = captured.result.unwrap_err();
        assert!(error.contains("compiler reported an error"), "got: {error}");
        assert!(error.contains("Test.as:4:2: error"), "got: {error}");
    }

    #[test]
    fn truncated_capture_rejects_a_cache_even_without_a_visible_error() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-truncated-capture-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&base).unwrap();
        let artifacts = DiagnosticArtifacts {
            capture: base.join("capture.txt"),
            status: base.join("status.txt"),
            dir: base.clone(),
            cleanup: true,
        };
        std::fs::write(
            &artifacts.capture,
            format!(
                "=== Test.as ===\n[W] warnings filled the capture\n{}\n",
                crate::diagnostics::CAPTURE_TRUNCATED_TOKEN
            ),
        )
        .unwrap();
        let complete = valid_cache();
        validate_generated_cache(&complete).expect("fixture must be structurally complete");
        let captured = append_captured_diagnostics(
            GeneratorRunResult::confirmed(Ok(complete)),
            &artifacts,
            crate::diagnostics::DiagnosticsCaptureDisposition::Captured,
        );
        assert_eq!(
            captured.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::CaptureInvalid
        );
        assert_eq!(captured.diagnostics.diagnostics().len(), 1);
        assert_eq!(
            captured.diagnostics.diagnostics()[0].message,
            "warnings filled the capture"
        );
        let error = captured.result.unwrap_err();
        assert!(error.contains("capture was truncated"), "got: {error}");
        assert!(error.contains("refusing to accept"), "got: {error}");
    }

    #[test]
    fn unreadable_existing_capture_rejects_an_unverified_cache() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-unreadable-capture-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(base.join("capture.txt")).unwrap();
        let artifacts = DiagnosticArtifacts {
            capture: base.join("capture.txt"), // a directory: exists, but cannot be read as bytes
            status: base.join("status.txt"),
            dir: base.clone(),
            cleanup: true,
        };
        let complete = valid_cache();
        validate_generated_cache(&complete).expect("fixture must be structurally complete");
        let captured = append_captured_diagnostics(
            GeneratorRunResult::confirmed(Ok(complete)),
            &artifacts,
            crate::diagnostics::DiagnosticsCaptureDisposition::Captured,
        );
        assert_eq!(
            captured.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::CaptureInvalid
        );
        let error = captured.result.unwrap_err();
        assert!(error.contains("could not be read"), "got: {error}");
        assert!(error.contains("refusing to accept"), "got: {error}");

        drop(artifacts);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn structured_capture_limit_rejects_an_unverified_cache() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-structured-capture-limit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&base).unwrap();
        let artifacts = DiagnosticArtifacts {
            capture: base.join("capture.txt"),
            status: base.join("status.txt"),
            dir: base,
            cleanup: true,
        };
        let oversized_file =
            "x".repeat(crate::diagnostics::MAX_STRUCTURED_DIAGNOSTIC_FILE_BYTES + 1);
        std::fs::write(
            &artifacts.capture,
            format!("=== {oversized_file} ===\n[E] failure\n"),
        )
        .unwrap();

        let captured = append_captured_diagnostics(
            GeneratorRunResult::confirmed(Ok(valid_cache())),
            &artifacts,
            crate::diagnostics::DiagnosticsCaptureDisposition::Captured,
        );
        assert_eq!(
            captured.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::CaptureInvalid
        );
        assert!(captured.diagnostics.diagnostics().is_empty());
        let error = captured.result.unwrap_err();
        assert!(
            error.contains("bounded structured diagnostics"),
            "got: {error}"
        );
        assert!(error.contains("refusing to accept"), "got: {error}");
    }

    #[test]
    fn unconfirmed_hooked_timeout_preserves_recovery_without_exposing_live_capture() {
        let root = std::env::temp_dir().join(format!(
            "gore-as-unconfirmed-hooked-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let diagnostics_dir = root.join("diagnostics");
        let helper_dir = root.join("helper");
        std::fs::create_dir_all(&diagnostics_dir).unwrap();
        std::fs::create_dir_all(&helper_dir).unwrap();
        let artifacts = DiagnosticArtifacts {
            capture: diagnostics_dir.join("capture.txt"),
            status: diagnostics_dir.join("status.txt"),
            dir: diagnostics_dir.clone(),
            cleanup: true,
        };
        std::fs::write(&artifacts.capture, "[W] retained warning\n").unwrap();
        let helper = helper_dir.join("gore-as-diagnostics-hook.dll");
        std::fs::write(&helper, b"test helper").unwrap();
        let prep =
            crate::diagnostics::HookPreparation::owned_for_test(helper.clone(), helper_dir.clone());

        let attempt = classify_hooked_result(
            GeneratorRunResult::unconfirmed(format!(
                "{GENERATOR_EXIT_UNCONFIRMED} simulated live generator"
            )),
            artifacts,
            prep,
        );
        let fallback_calls = std::cell::Cell::new(0);
        let report = resolve_diagnostic_attempt_report(attempt, || {
            fallback_calls.set(fallback_calls.get() + 1);
            GeneratorRunResult::confirmed(Ok::<_, String>(b"unsafe fallback".to_vec()))
        });
        assert_eq!(
            report.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::ProcessExitUnconfirmed
        );
        assert!(report.diagnostics.diagnostics().is_empty());
        assert_eq!(fallback_calls.get(), 0);
        let error = report.result.unwrap_err();
        assert!(
            error.contains(&diagnostics_dir.display().to_string()),
            "got: {error}"
        );
        assert!(
            error.contains(&helper_dir.display().to_string()),
            "got: {error}"
        );
        assert!(
            diagnostics_dir.is_dir(),
            "diagnostics directory was dropped"
        );
        assert!(helper.is_file(), "mapped helper was dropped");
        assert_eq!(
            std::fs::read_to_string(diagnostics_dir.join("capture.txt")).unwrap(),
            "[W] retained warning\n"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn started_hook_unconfirmed_preserves_recovery_without_exposing_live_capture() {
        let root = std::env::temp_dir().join(format!(
            "gore-as-unconfirmed-started-hook-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let diagnostics_dir = root.join("diagnostics");
        let helper_dir = root.join("helper");
        std::fs::create_dir_all(&diagnostics_dir).unwrap();
        std::fs::create_dir_all(&helper_dir).unwrap();
        let capture = diagnostics_dir.join("capture.txt");
        std::fs::write(&capture, "[E] possibly partial live message\n").unwrap();
        let artifacts = DiagnosticArtifacts {
            capture: capture.clone(),
            status: diagnostics_dir.join("status.txt"),
            dir: diagnostics_dir.clone(),
            cleanup: true,
        };
        let helper = helper_dir.join("gore-as-diagnostics-hook.dll");
        std::fs::write(&helper, b"test helper").unwrap();
        let prep =
            crate::diagnostics::HookPreparation::owned_for_test(helper.clone(), helper_dir.clone());

        let attempt = classify_started_hook_termination(
            ChildWaitFailure {
                message: format!("{GENERATOR_EXIT_UNCONFIRMED} simulated failed termination"),
                process_exit: GeneratorProcessExitDisposition::Unconfirmed,
            },
            artifacts,
            prep,
        );
        let fallback_calls = std::cell::Cell::new(0);
        let report = resolve_diagnostic_attempt_report(attempt, || {
            fallback_calls.set(fallback_calls.get() + 1);
            GeneratorRunResult::confirmed(Ok::<_, String>(b"unsafe fallback".to_vec()))
        });
        assert_eq!(
            report.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::ProcessExitUnconfirmed
        );
        assert!(report.diagnostics.diagnostics().is_empty());
        assert_eq!(fallback_calls.get(), 0);
        let error = report.result.unwrap_err();
        assert!(
            error.contains(&diagnostics_dir.display().to_string()),
            "got: {error}"
        );
        assert!(
            error.contains(&helper_dir.display().to_string()),
            "got: {error}"
        );
        assert_eq!(
            std::fs::read_to_string(&capture).unwrap(),
            "[E] possibly partial live message\n"
        );
        assert!(helper.is_file(), "mapped helper was dropped");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn explicit_diagnostics_opt_out_runs_normal_path_without_unavailable_state() {
        let called = std::cell::Cell::new(0);
        let report = resolve_diagnostic_attempt_report(DiagnosticAttempt::Disabled, || {
            called.set(called.get() + 1);
            GeneratorRunResult::confirmed(Ok::<_, String>("normal"))
        });
        assert_eq!(report.result.unwrap(), "normal");
        assert_eq!(
            report.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::Disabled
        );
        assert_eq!(called.get(), 1);
    }

    #[test]
    fn injection_failure_fallback_preserves_the_normal_error() {
        let report = resolve_diagnostic_attempt_report::<Vec<u8>, _>(
            DiagnosticAttempt::Unavailable("CreateRemoteThread failed".into()),
            || {
                GeneratorRunResult::confirmed(
                    Err("normal generator failed exactly this way".into()),
                )
            },
        );
        assert_eq!(
            report.result.unwrap_err(),
            "normal generator failed exactly this way"
        );
        assert_eq!(
            report.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::UnavailableFallback
        );
    }

    #[test]
    fn pre_injection_exit_keeps_the_first_generator_result() {
        let fallback_calls = std::cell::Cell::new(0);
        let report = resolve_diagnostic_attempt_report(
            DiagnosticAttempt::Completed(GeneratorDiagnosticsResult {
                result: Ok::<_, String>(b"first-normal-result".to_vec()),
                diagnostics: crate::diagnostics::CompilerDiagnosticsReport::empty(
                    crate::diagnostics::DiagnosticsCaptureDisposition::UnavailableWithoutFallback,
                ),
                process_exit: GeneratorProcessExitDisposition::Confirmed,
            }),
            || {
                fallback_calls.set(fallback_calls.get() + 1);
                GeneratorRunResult::confirmed(Ok(b"unexpected-relaunch".to_vec()))
            },
        );
        assert_eq!(report.result.unwrap(), b"first-normal-result");
        assert_eq!(
            report.diagnostics.disposition(),
            crate::diagnostics::DiagnosticsCaptureDisposition::UnavailableWithoutFallback
        );
        assert_eq!(fallback_calls.get(), 0);
    }

    #[test]
    fn post_injection_pre_ready_exit_retries_after_partial_cache_cleanup() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-post-injection-fallback-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&base);
        std::fs::write(&base, b"partial-from-injected-attempt").unwrap();
        let result = resolve_diagnostic_attempt(
            DiagnosticAttempt::Unavailable(
                "generator exited after injection before helper readiness".into(),
            ),
            || {
                clear_partial_cache_before_fallback(&base)?;
                Ok::<_, String>(b"clean-normal-result".to_vec())
            },
        )
        .unwrap();
        assert_eq!(result, b"clean-normal-result");
        assert!(!base.exists());
    }

    #[test]
    fn fallback_deletes_partial_first_attempt_cache() {
        let base =
            std::env::temp_dir().join(format!("gore-as-partial-fallback-{}", std::process::id()));
        let _ = std::fs::remove_file(&base);
        std::fs::write(&base, b"partial-from-hook-attempt").unwrap();
        clear_partial_cache_before_fallback(&base).unwrap();
        assert!(!base.exists());
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

    #[test]
    fn loose_script_walkers_fail_closed_on_io_errors_and_non_file_coverage() {
        let base = std::env::temp_dir().join("gore-as-loose-walker-errors");
        let _ = std::fs::remove_dir_all(&base);
        let missing = base.join("missing");
        assert!(first_loose_script(&missing).is_err());
        assert!(first_uncovered_loose_script(&missing, &base).is_err());

        let live = base.join("live");
        let src = base.join("src");
        std::fs::create_dir_all(&live).unwrap();
        std::fs::create_dir_all(src.join("Same.as")).unwrap(); // directory is not valid coverage
        let live_script = live.join("Same.as");
        std::fs::write(&live_script, b"live").unwrap();
        assert_eq!(
            first_uncovered_loose_script(&live, &src).unwrap(),
            Some(live_script)
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn copy_tree_records_rollback_before_injected_partial_copy_failure() {
        let base = std::env::temp_dir().join("gore-as-copy-partial-failure");
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("src");
        let dst = base.join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(src.join("Mod.as"), b"NEW-COMPLETE").unwrap();
        let target = dst.join("Mod.as");
        std::fs::write(&target, b"OLD").unwrap();
        let mut written = Vec::new();

        let err = copy_tree_with(&src, &dst, &mut written, &mut |_, to| {
            std::fs::write(to, b"PARTIAL")?;
            Err(std::io::Error::other("injected copy failure"))
        })
        .unwrap_err();
        assert!(err.to_string().contains("injected copy failure"));
        assert_eq!(written.len(), 1, "rollback entry registered before copy");
        assert_eq!(written[0].1.as_deref(), Some(b"OLD".as_slice()));
        assert_eq!(std::fs::read(&target).unwrap(), b"PARTIAL");

        restore_or_remove(&written, &dst).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"OLD");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn copy_tree_propagates_destination_snapshot_errors_before_copy() {
        let base = std::env::temp_dir().join("gore-as-copy-snapshot-error");
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("src");
        let dst = base.join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(dst.join("Mod.as")).unwrap();
        std::fs::write(src.join("Mod.as"), b"NEW").unwrap();
        let mut written = Vec::new();
        let copied = std::cell::Cell::new(false);

        assert!(copy_tree_with(&src, &dst, &mut written, &mut |_, _| {
            copied.set(true);
            Ok(())
        })
        .is_err());
        assert!(
            !copied.get(),
            "copy must not run without a reliable snapshot"
        );
        assert!(written.is_empty());
        assert!(dst.join("Mod.as").is_dir());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn copy_tree_rejects_linked_destination_directory() {
        let base = std::env::temp_dir().join("gore-as-copy-linked-destination");
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("src");
        let dst = base.join("dst");
        let outside = base.join("outside");
        std::fs::create_dir_all(src.join("AI")).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(src.join("AI").join("Mod.as"), b"NEW").unwrap();
        let linked = dst.join("AI");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &linked).unwrap();
        #[cfg(windows)]
        if std::os::windows::fs::symlink_dir(&outside, &linked).is_err() {
            // Windows without Developer Mode/elevation cannot create the fixture. Production
            // junctions still carry FILE_ATTRIBUTE_REPARSE_POINT and hit the same guard.
            let _ = std::fs::remove_dir_all(&base);
            return;
        }

        let err = copy_tree(&src, &dst, &mut Vec::new()).unwrap_err();
        assert!(err.to_string().contains("linked/reparse"), "got: {err}");
        assert!(
            !outside.join("Mod.as").exists(),
            "staging must not write through the destination link"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn unconfirmed_generator_preserves_isolation_recovery_and_lock() {
        let base = std::env::temp_dir().join("gore-as-unconfirmed-generator");
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        std::fs::write(dev_cache(&shipping), b"DEV-OLD").unwrap();
        let live_mod = shipping.parent().unwrap().join("Mod.as");
        std::fs::write(&live_mod, b"LIVE-OLD").unwrap();
        let win64 = base.join("G1R").join("Binaries").join("Win64");
        std::fs::create_dir_all(win64.join("ue4ss")).unwrap();
        std::fs::write(win64.join("ue4ss").join("UE4SS.dll"), b"ue4ss").unwrap();
        std::fs::write(win64.join("dwmapi.dll"), b"proxy").unwrap();
        let jitted = base.join("AS_JITTED_CODE");
        std::fs::create_dir_all(&jitted).unwrap();
        std::fs::write(jitted.join("old.bin"), b"jit").unwrap();
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();

        let err = game_run_regen_with(&game, &src, |_, _, _| {
            Err(format!(
                "{GENERATOR_EXIT_UNCONFIRMED} simulated live generator 123"
            ))
        })
        .unwrap_err();

        assert!(err.contains("intentionally NOT run"), "got: {err}");
        assert!(
            compile_lock_path(&game).exists(),
            "compile lock must remain"
        );
        assert!(
            compile_bak_path(&shipping).exists(),
            "Shipping recovery backup must remain"
        );
        assert_eq!(
            std::fs::read(&live_mod).unwrap(),
            b"script",
            "staged source must remain isolated until the child is killed"
        );
        let journal = recovery_journal_path(&game);
        assert_eq!(
            std::fs::read(journal.join("overwritten").join("Mod.as")).unwrap(),
            b"LIVE-OLD",
            "overwritten loose-script bytes must be recoverable from disk"
        );
        assert_eq!(
            std::fs::read(
                journal
                    .join("development-cache")
                    .join("PrecompiledScript.Cache")
            )
            .unwrap(),
            b"DEV-OLD",
            "the pre-call development cache must be recoverable from disk"
        );
        assert!(!jitted.exists(), "original JIT path must stay quarantined");
        assert!(append_suffix(&jitted, ".gore-compile-bak").exists());
        let proxy = win64.join("dwmapi.dll");
        assert!(!proxy.exists(), "UE4SS proxy must stay quarantined");
        assert!(append_suffix(&proxy, ".gore-compile-bak").exists());

        // The transaction is intentionally leaked; remove the isolated temp fixture as a whole.
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn precompile_uses_structured_unconfirmed_exit_without_string_marker() {
        let base = std::env::temp_dir().join(format!(
            "gore-as-precompile-structured-unconfirmed-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let (game, shipping) = fake_install(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("Mod.as"), b"script").unwrap();
        let opts = PrecompileOpts {
            game_dir: game.clone(),
            src: Some(src),
            out: Some(base.join("compiled.cache")),
            backup: true,
        };
        let message = "simulated process tree still alive without a legacy marker";
        assert!(!message.contains(GENERATOR_EXIT_UNCONFIRMED));

        let error = precompile_with_generator_report(&opts, |_, _, _| {
            GeneratorRunResult::unconfirmed(message.to_owned())
        })
        .unwrap_err();

        assert!(error.contains(message), "got: {error}");
        assert!(error.contains("intentionally NOT run"), "got: {error}");
        assert!(compile_lock_path(&game).exists());
        assert_eq!(std::fs::read(compile_bak_path(&shipping)).unwrap(), b"OLD");
        assert!(recovery_journal_path(&game).exists());
        assert!(shipping.parent().unwrap().join("Mod.as").exists());
        assert!(!opts.out.as_ref().unwrap().exists());

        // No real process exists in this injected test; removing the isolated fixture is the
        // test-only equivalent of completing the documented recovery sequence.
        std::fs::remove_dir_all(base).unwrap();
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
        assert!(written
            .iter()
            .any(|(p, prior)| p == &over && prior.as_deref() == Some(b"old")));

        // Cleanup succeeds: the colliding file restores and the copied-only files delete cleanly.
        restore_or_remove(&written, &dst).expect("cleanup should succeed in a writable tmp tree");

        // Copied-only files + the dir the copy created are gone.
        assert!(!top.exists(), "copied top-level file should be removed");
        assert!(!nested.exists(), "copied nested file should be removed");
        assert!(
            !dst.join("AI").exists(),
            "now-empty created dir should be pruned"
        );
        // Non-colliding pre-existing file and the dst root itself survive.
        assert!(pre.exists(), "pre-existing file must be left untouched");
        assert!(dst.exists(), "dst root must not be removed");
        // The overwritten pre-existing file is RESTORED to its original bytes (not deleted).
        assert!(
            over.exists(),
            "overwritten pre-existing file must be restored, not deleted"
        );
        assert_eq!(
            std::fs::read(&over).unwrap(),
            b"old",
            "restored bytes must be the original"
        );

        let _ = std::fs::remove_dir_all(&base);
    }
}
