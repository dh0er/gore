//! Optional, fail-closed capture of the shipping game's AngelScript diagnostics.
//!
//! The game remains the compiler. This module only preflights a version-tolerant masked byte
//! signature in the selected executable, temporarily injects a small capture DLL early in the
//! compiler process, and parses the bounded text stream it produces. A missing helper, unsupported
//! platform, zero/multiple signature matches, or a confirmed injection failure is an availability
//! problem, not a compile failure: callers transparently launch the normal generator instead.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Signature of `LogAngelscriptError(asSMessageInfo*, void*)`, the per-message callback registered
/// by the UE-AngelScript fork. It was observed at RVA `0x467f5b0` in the 2026-07-10 hotfix (the
/// prior build used `0x467e200`), but no fixed RVA is used. Wildcards cover RIP-relative
/// addresses/operands that vary by build. Both the offline preflight and helper DLL use this exact
/// AOB and require one match.
const LOG_ANGELSCRIPT_ERROR_AOB: &[Option<u8>] = &[
    Some(0x40),
    Some(0x55),
    Some(0x56),
    Some(0x57),
    Some(0x48),
    Some(0x8d),
    Some(0xac),
    Some(0x24),
    Some(0x60),
    Some(0xff),
    Some(0xff),
    Some(0xff),
    Some(0x48),
    Some(0x81),
    Some(0xec),
    Some(0xa0),
    Some(0x01),
    Some(0x00),
    Some(0x00),
    Some(0x48),
    Some(0x8b),
    Some(0x05),
    None,
    None,
    None,
    None,
    Some(0x48),
    Some(0x33),
    Some(0xc4),
    Some(0x48),
    Some(0x89),
    Some(0x85),
    Some(0x80),
    Some(0x00),
    Some(0x00),
    Some(0x00),
    Some(0x8b),
    Some(0x15),
    None,
    None,
];

pub const MAX_CAPTURE_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_FORMATTED_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_INJECT_DELAY: std::time::Duration = std::time::Duration::from_secs(30);
pub(crate) const CAPTURE_TRUNCATED_TOKEN: &str = "[GORE] diagnostics capture truncated at 8 MiB";
const MAX_STATUS_BYTES: u64 = 4096;
const BUNDLED_HOOK_SHA256: &str =
    "9a408f1dd2bfd95b9235d261c94b4ff60eb4c562ba5d1d69b398be9499e9f153";

#[derive(Clone, Debug)]
pub struct DiagnosticsOptions {
    /// `false` by default (capture enabled). Set true for an explicit normal-generator-only run.
    pub disabled: bool,
    /// Explicit helper DLL. When absent, discovery checks `GORE_AS_DIAGNOSTICS_HOOK`, then the
    /// executable directory beside `gore[.exe]`.
    pub hook_dll: Option<PathBuf>,
    /// Loader/CRT warm-up before remote `LoadLibraryW`. Injecting at process birth can deadlock on
    /// the loader lock; the proven current-build window is 2 seconds, before AS compilation starts.
    pub inject_delay: std::time::Duration,
}

impl Default for DiagnosticsOptions {
    fn default() -> Self {
        Self {
            disabled: false,
            hook_dll: None,
            inject_delay: std::time::Duration::from_secs(2),
        }
    }
}

fn validate_inject_delay(delay: std::time::Duration) -> Result<(), String> {
    if delay > MAX_INJECT_DELAY {
        return Err(format!(
            "diagnostics injection delay {} ms exceeds the {} ms safety limit",
            delay.as_millis(),
            MAX_INJECT_DELAY.as_millis()
        ));
    }
    Ok(())
}

fn checked_module_rva(module_base: usize, address: usize) -> Result<usize, String> {
    address.checked_sub(module_base).ok_or_else(|| {
        format!("function address 0x{address:x} precedes provider module base 0x{module_base:x}")
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompilerDiagnostic {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub severity: DiagnosticSeverity,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Note,
}

impl DiagnosticSeverity {
    fn label(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
        }
    }
}

#[derive(Debug)]
pub(crate) struct HookPreparation {
    pub hook_dll: PathBuf,
    owned_dir: Option<PathBuf>,
}

impl Drop for HookPreparation {
    fn drop(&mut self) {
        let Some(dir) = &self.owned_dir else { return };
        let _ = std::fs::remove_file(&self.hook_dll);
        let _ = std::fs::remove_dir(dir);
    }
}

impl HookPreparation {
    /// Keep an embedded helper directory when process exit could not be confirmed. The still-live
    /// process may have the DLL mapped/open; returning the path lets the fatal error tell the user
    /// exactly what was intentionally retained instead of silently failing cleanup in `Drop`.
    pub(crate) fn preserve_owned(mut self) -> Option<PathBuf> {
        self.owned_dir.take()
    }

    #[cfg(test)]
    pub(crate) fn owned_for_test(hook_dll: PathBuf, owned_dir: PathBuf) -> Self {
        Self {
            hook_dll,
            owned_dir: Some(owned_dir),
        }
    }
}

/// Discover the helper and prove the AOB is unique in the selected executable before launch.
/// No process is created and no game file is changed.
pub(crate) fn prepare_hook(
    exe: &Path,
    options: &DiagnosticsOptions,
) -> Result<HookPreparation, String> {
    if options.disabled {
        return Err("disabled by --no-diagnostics".into());
    }
    if !cfg!(windows) {
        return Err("diagnostic injection is currently available only on Windows".into());
    }
    validate_inject_delay(options.inject_delay)?;
    let matches = count_aob_matches_in_pe_text(exe, LOG_ANGELSCRIPT_ERROR_AOB)?;
    if matches != 1 {
        return Err(format!(
            "LogAngelscriptError signature matched {matches} times in {} (need exactly 1)",
            exe.display()
        ));
    }
    // Discover/materialize only after compatibility is proven. Construct the RAII owner
    // immediately so every later metadata/canonicalization error removes an embedded temp DLL.
    let (hook, owned_dir, verify_bundled_hash) = discover_hook(options)?;
    let mut prep = HookPreparation {
        hook_dll: hook,
        owned_dir,
    };
    let meta = std::fs::symlink_metadata(&prep.hook_dll).map_err(|e| {
        format!(
            "inspecting diagnostics helper {}: {e}",
            prep.hook_dll.display()
        )
    })?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err(format!(
            "diagnostics helper is not a regular file: {}",
            prep.hook_dll.display()
        ));
    }
    prep.hook_dll = prep
        .hook_dll
        .canonicalize()
        .map_err(|e| format!("resolving diagnostics helper: {e}"))?;
    if verify_bundled_hash {
        let actual = sha256_file(&prep.hook_dll)?;
        if actual != BUNDLED_HOOK_SHA256 {
            return Err(format!(
                "bundled diagnostics helper integrity mismatch: expected {BUNDLED_HOOK_SHA256}, got {actual}"
            ));
        }
    }
    Ok(prep)
}

fn discover_hook(options: &DiagnosticsOptions) -> Result<(PathBuf, Option<PathBuf>, bool), String> {
    if let Some(path) = &options.hook_dll {
        return Ok((path.clone(), None, false));
    }
    if let Some(path) = std::env::var_os("GORE_AS_DIAGNOSTICS_HOOK") {
        return Ok((PathBuf::from(path), None, false));
    }
    let current = std::env::current_exe()
        .map_err(|e| format!("locating gore executable for diagnostics-helper discovery: {e}"))?;
    let sibling = current
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("gore-as-diagnostics-hook.dll");
    if sibling.exists() {
        return Ok((sibling, None, true));
    }
    materialize_embedded_hook().map_err(|error| {
        format!(
            "diagnostics helper not found at {} and embedded helper could not be prepared: {error}",
            sibling.display()
        )
    })
}

#[cfg(windows)]
fn materialize_embedded_hook() -> Result<(PathBuf, Option<PathBuf>, bool), String> {
    const DLL: &[u8] = include_bytes!("../assets/gore-as-diagnostics-hook.dll");
    let temp = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for suffix in 0..32u32 {
        let dir = temp.join(format!(
            "gore-as-hook-{}-{stamp}-{suffix}",
            std::process::id()
        ));
        match std::fs::create_dir(&dir) {
            Ok(()) => {
                let path = dir.join("gore-as-diagnostics-hook.dll");
                let write = (|| -> Result<(), String> {
                    let mut file = std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&path)
                        .map_err(|e| format!("creating {}: {e}", path.display()))?;
                    use std::io::Write as _;
                    file.write_all(DLL)
                        .and_then(|_| file.sync_all())
                        .map_err(|e| format!("writing {}: {e}", path.display()))
                })();
                if let Err(error) = write {
                    let _ = std::fs::remove_file(&path);
                    let _ = std::fs::remove_dir(&dir);
                    return Err(error);
                }
                return Ok((path, Some(dir), true));
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("creating embedded-hook temp directory: {e}")),
        }
    }
    Err("could not reserve an embedded-hook temp directory".into())
}

#[cfg(not(windows))]
fn materialize_embedded_hook() -> Result<(PathBuf, Option<PathBuf>, bool), String> {
    Err("embedded diagnostics helper is a Windows DLL".into())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutableProbe {
    pub sha256: String,
    pub match_count: usize,
    pub matched_rvas: Vec<u64>,
}

/// Offline compatibility report for a selected executable. Hash, count and RVAs make a hotfix or
/// non-Steam build auditable without launching/injecting it.
pub fn probe_executable(exe: &Path) -> Result<ExecutableProbe, String> {
    let file = std::fs::File::open(exe)
        .map_err(|e| format!("opening game executable {}: {e}", exe.display()))?;
    probe_open_executable(file, exe)
}

/// Hash and scan one already-open executable handle. Keeping the same handle across both passes
/// prevents an updater/path replacement from pairing one file's hash with another file's RVAs.
/// A pre/post metadata check also rejects detectable in-place mutation during the audit.
fn probe_open_executable(mut file: std::fs::File, exe: &Path) -> Result<ExecutableProbe, String> {
    let before = file
        .metadata()
        .map_err(|e| format!("reading game executable metadata {}: {e}", exe.display()))?;
    let before_modified = before.modified().ok();
    let sha256 = sha256_open_file(&mut file, exe)?;
    let matched_rvas = aob_rvas_in_open_pe_text(&mut file, exe, LOG_ANGELSCRIPT_ERROR_AOB)?;
    let after = file
        .metadata()
        .map_err(|e| format!("re-reading game executable metadata {}: {e}", exe.display()))?;
    let after_modified = after.modified().ok();
    if before.len() != after.len()
        || matches!((before_modified, after_modified), (Some(a), Some(b)) if a != b)
    {
        return Err(format!(
            "{} changed while its diagnostics compatibility was being audited",
            exe.display()
        ));
    }
    Ok(ExecutableProbe {
        sha256,
        match_count: matched_rvas.len(),
        matched_rvas,
    })
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("opening {} for hashing: {e}", path.display()))?;
    sha256_open_file(&mut file, path)
}

fn sha256_open_file(file: &mut std::fs::File, path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    file.seek(SeekFrom::Start(0))
        .map_err(|e| format!("seeking {} for hashing: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    // Heap-backed: the Windows CLI main thread can have a 1 MiB stack.
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|e| format!("hashing {}: {e}", path.display()))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn le_u16(bytes: &[u8], off: usize) -> Result<u16, String> {
    let src = bytes
        .get(off..off + 2)
        .ok_or_else(|| format!("truncated PE field at 0x{off:x}"))?;
    Ok(u16::from_le_bytes([src[0], src[1]]))
}

fn le_u32(bytes: &[u8], off: usize) -> Result<u32, String> {
    let src = bytes
        .get(off..off + 4)
        .ok_or_else(|| format!("truncated PE field at 0x{off:x}"))?;
    Ok(u32::from_le_bytes([src[0], src[1], src[2], src[3]]))
}

#[cfg(test)]
fn masked_match_offsets(haystack: &[u8], needle: &[Option<u8>]) -> Vec<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return Vec::new();
    }
    let mut matches = Vec::new();
    for start in 0..=haystack.len() - needle.len() {
        if needle
            .iter()
            .enumerate()
            .all(|(i, want)| want.is_none_or(|want| haystack[start + i] == want))
        {
            matches.push(start);
        }
    }
    matches
}

#[cfg(test)]
fn count_masked_matches(haystack: &[u8], needle: &[Option<u8>]) -> usize {
    masked_match_offsets(haystack, needle).len()
}

/// Read only the PE headers and `.text` raw section. A zero or duplicate match is deliberately
/// returned to the caller, which treats it as a safe fallback rather than guessing an address.
fn count_aob_matches_in_pe_text(exe: &Path, needle: &[Option<u8>]) -> Result<usize, String> {
    aob_rvas_in_pe_text(exe, needle).map(|rvas| rvas.len())
}

fn aob_rvas_in_pe_text(exe: &Path, needle: &[Option<u8>]) -> Result<Vec<u64>, String> {
    let mut file = std::fs::File::open(exe)
        .map_err(|e| format!("opening game executable {}: {e}", exe.display()))?;
    aob_rvas_in_open_pe_text(&mut file, exe, needle)
}

fn aob_rvas_in_open_pe_text(
    file: &mut std::fs::File,
    exe: &Path,
    needle: &[Option<u8>],
) -> Result<Vec<u64>, String> {
    let file_len = file
        .metadata()
        .map_err(|e| format!("reading game executable metadata {}: {e}", exe.display()))?
        .len();
    file.seek(SeekFrom::Start(0))
        .map_err(|e| format!("seeking to PE headers in {}: {e}", exe.display()))?;
    let header_len = file_len.min(4 * 1024 * 1024) as usize;
    let mut headers = vec![0u8; header_len];
    file.read_exact(&mut headers)
        .map_err(|e| format!("reading PE headers {}: {e}", exe.display()))?;
    if headers.get(0..2) != Some(b"MZ") {
        return Err(format!("{} is not an MZ executable", exe.display()));
    }
    let pe = le_u32(&headers, 0x3c)? as usize;
    if headers.get(pe..pe + 4) != Some(b"PE\0\0") {
        return Err(format!("{} has no valid PE signature", exe.display()));
    }
    let machine = le_u16(&headers, pe + 4)?;
    if machine != 0x8664 {
        return Err(format!(
            "{} is not an AMD64 PE image (machine=0x{machine:04x}); the x86-64 diagnostics helper is unsupported",
            exe.display()
        ));
    }
    let sections = le_u16(&headers, pe + 6)? as usize;
    let optional_size = le_u16(&headers, pe + 20)? as usize;
    if sections == 0 || sections > 96 {
        return Err(format!(
            "{} declares invalid PE section count {sections}",
            exe.display()
        ));
    }
    let table = pe
        .checked_add(24)
        .and_then(|v| v.checked_add(optional_size))
        .ok_or_else(|| "PE section-table offset overflow".to_string())?;
    let table_end = table
        .checked_add(sections * 40)
        .ok_or_else(|| "PE section-table size overflow".to_string())?;
    if table_end > headers.len() {
        return Err(format!(
            "{} has a truncated PE section table",
            exe.display()
        ));
    }
    let mut text = None;
    for i in 0..sections {
        let sh = table + i * 40;
        let name = &headers[sh..sh + 8];
        if name == b".text\0\0\0" {
            let virtual_size = le_u32(&headers, sh + 8)? as u64;
            let virtual_address = le_u32(&headers, sh + 12)? as u64;
            let raw_size = le_u32(&headers, sh + 16)? as u64;
            let offset = le_u32(&headers, sh + 20)? as u64;
            // The helper scans the mapped image while this preflight scans the file. Restrict both
            // to the raw-backed intersection: raw alignment padding beyond VirtualSize and mapped
            // zero-fill beyond SizeOfRawData are deliberately excluded from both decisions.
            let scan_size = virtual_size.min(raw_size);
            text = Some((offset, raw_size, scan_size, virtual_address));
            break;
        }
    }
    let (offset, raw_size, size, virtual_address) =
        text.ok_or_else(|| format!("{} has no .text section", exe.display()))?;
    if raw_size == 0
        || size == 0
        || raw_size > 1024 * 1024 * 1024
        || offset
            .checked_add(raw_size)
            .is_none_or(|end| end > file_len)
    {
        return Err(format!("{} has invalid .text bounds", exe.display()));
    }
    file.seek(SeekFrom::Start(offset))
        .map_err(|e| format!("seeking to PE .text in {}: {e}", exe.display()))?;
    if needle.is_empty() {
        return Ok(Vec::new());
    }
    // Stream the section with only a needle-sized overlap. Uniqueness is the only safety
    // decision, so two RVAs are sufficient: a second match already means "ambiguous" and avoids
    // both a section-sized allocation and an attacker-controlled unbounded match vector.
    const CHUNK: usize = 1024 * 1024;
    const MATCH_CAP: usize = 2;
    let mut rvas = Vec::with_capacity(MATCH_CAP);
    let mut tail = Vec::<u8>::new();
    let mut section_read = 0u64;
    while section_read < size {
        let take = (size - section_read).min(CHUNK as u64) as usize;
        let mut chunk = vec![0u8; take];
        file.read_exact(&mut chunk)
            .map_err(|e| format!("reading PE .text in {}: {e}", exe.display()))?;
        let tail_len = tail.len();
        let mut window = Vec::with_capacity(tail_len + chunk.len());
        window.extend_from_slice(&tail);
        window.extend_from_slice(&chunk);
        let base = section_read.saturating_sub(tail_len as u64);
        if window.len() >= needle.len() {
            for start in 0..=window.len() - needle.len() {
                if needle
                    .iter()
                    .enumerate()
                    .all(|(i, want)| want.is_none_or(|want| window[start + i] == want))
                {
                    let section_offset = base
                        .checked_add(start as u64)
                        .ok_or_else(|| "matched section offset overflow".to_string())?;
                    let rva = virtual_address
                        .checked_add(section_offset)
                        .ok_or_else(|| "matched RVA overflow".to_string())?;
                    rvas.push(rva);
                    if rvas.len() == MATCH_CAP {
                        return Ok(rvas);
                    }
                }
            }
        }
        let keep = needle.len().saturating_sub(1).min(window.len());
        tail.clear();
        tail.extend_from_slice(&window[window.len() - keep..]);
        section_read += take as u64;
    }
    Ok(rvas)
}

/// Parse the helper's bounded line protocol into normal compiler diagnostics.
pub fn parse_capture(text: &str) -> Vec<CompilerDiagnostic> {
    let mut file = String::from("<unknown>");
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim_end_matches('\r');
        if let Some(section) = line
            .strip_prefix("=== ")
            .and_then(|s| s.strip_suffix(" ==="))
        {
            file = section.to_string();
            continue;
        }
        if let Some(diagnostic) = parse_capture_line(&file, line) {
            out.push(diagnostic);
        }
    }
    out
}

fn parse_capture_line(file: &str, line: &str) -> Option<CompilerDiagnostic> {
    let (line_no, column, rest) = if let Some(rest) = line.strip_prefix('(') {
        let (position, rest) = rest.split_once(") ")?;
        let (line_s, col_s) = position.split_once(':')?;
        (
            line_s.parse::<u32>().ok()?,
            col_s.parse::<u32>().ok()?,
            rest,
        )
    } else {
        (0, 0, line)
    };
    let rest = rest.strip_prefix('[')?;
    let (kind, message) = rest.split_once("] ")?;
    let severity = match kind {
        "E" => DiagnosticSeverity::Error,
        "W" => DiagnosticSeverity::Warning,
        "I" => DiagnosticSeverity::Note,
        _ => return None,
    };
    Some(CompilerDiagnostic {
        file: file.to_string(),
        line: line_no,
        column,
        severity,
        message: message.to_string(),
    })
}

const FORMAT_TRUNCATED_MARKER: &str = "<diagnostics formatting truncated at 8 MiB>\n";

fn append_formatted_bounded(out: &mut String, line: &str) -> bool {
    let content_cap = MAX_FORMATTED_BYTES.saturating_sub(FORMAT_TRUNCATED_MARKER.len());
    if out.len().saturating_add(line.len()) <= content_cap {
        out.push_str(line);
        return true;
    }
    let mut take = content_cap.saturating_sub(out.len()).min(line.len());
    while take > 0 && !line.is_char_boundary(take) {
        take -= 1;
    }
    out.push_str(&line[..take]);
    out.push_str(FORMAT_TRUNCATED_MARKER);
    false
}

pub fn format_capture(text: &str) -> String {
    let mut out = String::new();
    let mut file = String::from("<unknown>");
    for raw in text.lines() {
        let line = raw.trim_end_matches('\r');
        if let Some(section) = line
            .strip_prefix("=== ")
            .and_then(|s| s.strip_suffix(" ==="))
        {
            file = section.to_string();
            continue;
        }
        match parse_capture_line(&file, line) {
            Some(diagnostic)
                if diagnostic.severity == DiagnosticSeverity::Note
                    && diagnostic.message.starts_with("Compiling ") =>
            {
                // Routine per-function progress is enormous on the 7,305-module tree. Keep it in
                // the bounded raw capture/parse API, but omit it from concise human stderr.
            }
            Some(diagnostic) => {
                let rendered = format!(
                    "{}:{}:{}: {}: {}",
                    diagnostic.file,
                    diagnostic.line,
                    diagnostic.column,
                    diagnostic.severity.label(),
                    diagnostic.message
                );
                if !append_formatted_bounded(&mut out, &(rendered + "\n")) {
                    break;
                }
            }
            None if !line.trim().is_empty() => {
                // Never hide a new/changed helper protocol line just because this parser is older.
                let rendered = format!("{}:0:0: note: [raw] {}\n", file, line);
                if !append_formatted_bounded(&mut out, &rendered) {
                    break;
                }
            }
            None => {}
        }
    }
    out
}

pub(crate) fn read_bounded(path: &Path, max: u64) -> Result<(String, bool), String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("opening diagnostics output {}: {e}", path.display()))?;
    let len = file
        .metadata()
        .map_err(|e| {
            format!(
                "reading diagnostics output metadata {}: {e}",
                path.display()
            )
        })?
        .len();
    let mut bytes = Vec::with_capacity(len.min(max) as usize);
    file.take(max)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("reading diagnostics output {}: {e}", path.display()))?;
    // `>=` is intentional: the bundled helper reserves an explicit marker below the cap, while an
    // older/custom helper may stop at exactly `max` with no marker and otherwise hide later errors.
    Ok((
        String::from_utf8_lossy(&bytes).into_owned(),
        max != 0 && len >= max,
    ))
}

pub(crate) fn read_status(path: &Path) -> Result<String, String> {
    read_bounded(path, MAX_STATUS_BYTES).map(|(text, _)| text)
}

fn complete_status_token(text: &str) -> Option<&str> {
    let token = text.strip_suffix('\n')?.trim_end_matches('\r');
    if token == "ready"
        || token
            .strip_prefix("unavailable:")
            .is_some_and(|reason| !reason.trim().is_empty())
    {
        (!token.contains(['\r', '\n'])).then_some(token)
    } else {
        None
    }
}

#[cfg(windows)]
pub(crate) fn spawn_hooked(
    exe: &Path,
    g1r: &Path,
    args: &[&str],
    prep: &HookPreparation,
    capture: &Path,
    status: &Path,
    inject_delay: std::time::Duration,
) -> Result<HookSpawnOutcome, HookSpawnError> {
    windows::spawn_hooked(exe, g1r, args, prep, capture, status, inject_delay)
}

#[cfg(not(windows))]
pub(crate) fn spawn_hooked(
    _exe: &Path,
    _g1r: &Path,
    _args: &[&str],
    _prep: &HookPreparation,
    _capture: &Path,
    _status: &Path,
    _inject_delay: std::time::Duration,
) -> Result<HookSpawnOutcome, HookSpawnError> {
    Err(HookSpawnError::SafeFallback(
        "diagnostic injection is currently available only on Windows".into(),
    ))
}

#[derive(Debug)]
pub(crate) enum HookSpawnOutcome {
    Hooked(std::process::Child),
    /// The normal generator exited during the loader warm-up. This is its real result; do not
    /// relaunch merely because diagnostics could not be attached in time.
    ExitedBeforeInjection(std::process::Child),
    /// `LoadLibraryW` completed, but the process exited before the helper reported `ready`.
    /// Injection/helper initialization may have caused that exit, so its cache/result is not
    /// authoritative. The caller may fall back only after this already-confirmed exit and after
    /// removing any partial development cache from the injected attempt.
    ExitedAfterInjectionBeforeReady {
        child: std::process::Child,
        status: std::process::ExitStatus,
    },
}

#[derive(Debug)]
pub(crate) enum HookSpawnError {
    /// No generator process was started.
    SafeFallback(String),
    /// A generator process was started. The transaction owner must terminate its whole process tree
    /// and confirm exit before either cleanup or a fallback launch.
    Started {
        child: std::process::Child,
        reason: String,
    },
}

#[cfg(windows)]
mod windows {
    use super::{
        complete_status_token, read_status, validate_inject_delay, HookPreparation, HookSpawnError,
        HookSpawnOutcome,
    };
    use std::ffi::{c_void, CString, OsStr};
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::AsRawHandle;
    use std::path::Path;
    use std::ptr;
    use std::time::{Duration, Instant};
    use windows_sys::Win32::Foundation::{
        CloseHandle, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0,
    };
    use windows_sys::Win32::System::Diagnostics::Debug::WriteProcessMemory;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, MODULEENTRY32W, TH32CS_SNAPMODULE,
        TH32CS_SNAPMODULE32,
    };
    use windows_sys::Win32::System::LibraryLoader::{
        GetModuleFileNameW, GetModuleHandleW, GetProcAddress,
    };
    use windows_sys::Win32::System::Memory::{
        VirtualAllocEx, VirtualFreeEx, VirtualQuery, MEMORY_BASIC_INFORMATION, MEM_COMMIT,
        MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
    };
    use windows_sys::Win32::System::Threading::{
        CreateRemoteThread, GetExitCodeThread, WaitForSingleObject,
    };

    struct RemoteAllocation {
        process: HANDLE,
        ptr: *mut c_void,
    }

    struct Snapshot(HANDLE);

    impl Drop for Snapshot {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0) };
        }
    }

    impl Drop for RemoteAllocation {
        fn drop(&mut self) {
            if !self.ptr.is_null() {
                unsafe {
                    VirtualFreeEx(self.process, self.ptr, 0, MEM_RELEASE);
                }
            }
        }
    }

    fn wide_nul(value: &OsStr) -> Result<Vec<u16>, String> {
        let mut wide: Vec<u16> = value.encode_wide().collect();
        if wide.contains(&0) {
            return Err("path contains an interior NUL".into());
        }
        wide.push(0);
        Ok(wide)
    }

    fn remote_module_base(pid: u32, wanted: &str) -> Result<usize, String> {
        let snapshot =
            unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid) };
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(format!(
                "CreateToolhelp32Snapshot({pid}) failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        let snapshot = Snapshot(snapshot);
        let mut entry = MODULEENTRY32W {
            dwSize: std::mem::size_of::<MODULEENTRY32W>() as u32,
            ..Default::default()
        };
        if unsafe { Module32FirstW(snapshot.0, &mut entry) } == 0 {
            return Err(format!(
                "Module32FirstW({pid}) failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        loop {
            let len = entry
                .szModule
                .iter()
                .position(|ch| *ch == 0)
                .unwrap_or(entry.szModule.len());
            if String::from_utf16_lossy(&entry.szModule[..len]).eq_ignore_ascii_case(wanted) {
                return Ok(entry.modBaseAddr as usize);
            }
            if unsafe { Module32NextW(snapshot.0, &mut entry) } == 0 {
                break;
            }
        }
        Err(format!("remote process {pid} has no loaded {wanted}"))
    }

    fn inject_load_library(process: HANDLE, pid: u32, dll: &Path) -> Result<(), String> {
        let dll_w = wide_nul(dll.as_os_str())?;
        let bytes = dll_w.len() * std::mem::size_of::<u16>();
        let remote = unsafe {
            VirtualAllocEx(
                process,
                ptr::null_mut(),
                bytes,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        };
        if remote.is_null() {
            return Err(format!(
                "VirtualAllocEx failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        let allocation = RemoteAllocation {
            process,
            ptr: remote,
        };
        let mut written = 0usize;
        let ok = unsafe {
            WriteProcessMemory(process, remote, dll_w.as_ptr().cast(), bytes, &mut written)
        };
        if ok == 0 || written != bytes {
            return Err(format!(
                "WriteProcessMemory failed after {written}/{bytes} bytes: {}",
                std::io::Error::last_os_error()
            ));
        }
        const KERNEL32: &[u16] = &[
            b'k' as u16,
            b'e' as u16,
            b'r' as u16,
            b'n' as u16,
            b'e' as u16,
            b'l' as u16,
            b'3' as u16,
            b'2' as u16,
            b'.' as u16,
            b'd' as u16,
            b'l' as u16,
            b'l' as u16,
            0,
        ];
        let kernel32 = unsafe { GetModuleHandleW(KERNEL32.as_ptr()) };
        if kernel32.is_null() {
            return Err(format!(
                "GetModuleHandleW(kernel32) failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        let name = CString::new("LoadLibraryW").unwrap();
        let load_library =
            unsafe { GetProcAddress(kernel32, name.as_ptr().cast()) }.ok_or_else(|| {
                format!(
                    "GetProcAddress(LoadLibraryW) failed: {}",
                    std::io::Error::last_os_error()
                )
            })?;
        let local_proc = load_library as usize;
        // GetProcAddress(kernel32, LoadLibraryW) may resolve a forwarded export into KernelBase
        // (or another provider). Find the allocation that ACTUALLY contains the returned address,
        // compute the RVA against that module, then locate the same module in the target process.
        // Assuming the local kernel32 base would create an invalid remote start address whenever
        // the export is forwarded or module ASLR differs between processes.
        let mut provider = MEMORY_BASIC_INFORMATION::default();
        if unsafe {
            VirtualQuery(
                local_proc as *const c_void,
                &mut provider,
                std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        } == 0
            || provider.AllocationBase.is_null()
        {
            return Err(format!(
                "VirtualQuery(local LoadLibraryW) failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        let provider_base = provider.AllocationBase as usize;
        let load_library_rva = super::checked_module_rva(provider_base, local_proc)?;
        let mut provider_path = vec![0u16; 32_768];
        let path_len = unsafe {
            GetModuleFileNameW(
                provider.AllocationBase,
                provider_path.as_mut_ptr(),
                provider_path.len() as u32,
            )
        } as usize;
        if path_len == 0 || path_len >= provider_path.len() {
            return Err(format!(
                "GetModuleFileNameW(LoadLibraryW provider) failed/truncated: {}",
                std::io::Error::last_os_error()
            ));
        }
        let provider_path = &provider_path[..path_len];
        let basename_at = provider_path
            .iter()
            .rposition(|ch| *ch == b'\\' as u16 || *ch == b'/' as u16)
            .map_or(0, |i| i + 1);
        let provider_name = String::from_utf16_lossy(&provider_path[basename_at..]);
        if provider_name.is_empty() {
            return Err("LoadLibraryW provider has an empty module basename".into());
        }
        let remote_base = remote_module_base(pid, &provider_name)?;
        let remote_proc = remote_base
            .checked_add(load_library_rva)
            .ok_or_else(|| "remote LoadLibraryW address overflow".to_string())?;
        let start: unsafe extern "system" fn(*mut c_void) -> u32 =
            unsafe { std::mem::transmute(remote_proc) };
        let thread = unsafe {
            CreateRemoteThread(
                process,
                ptr::null(),
                0,
                Some(start),
                remote,
                0,
                ptr::null_mut(),
            )
        };
        if thread.is_null() {
            return Err(format!(
                "CreateRemoteThread failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        let wait = unsafe { WaitForSingleObject(thread, 10_000) };
        let mut remote_module_low32 = 0u32;
        let exit_ok = if wait == WAIT_OBJECT_0 {
            unsafe { GetExitCodeThread(thread, &mut remote_module_low32) }
        } else {
            0
        };
        unsafe { CloseHandle(thread) };
        if wait != WAIT_OBJECT_0 {
            // The remote thread may still be reading the DLL path. Deliberately retain the
            // allocation; the caller terminates/confirms the process before fallback, and Windows
            // then releases its complete address space. Freeing here would be a remote UAF.
            std::mem::forget(allocation);
            return Err(format!(
                "LoadLibraryW remote thread did not complete (wait={wait:#x}); remote path retained until process exit"
            ));
        }
        drop(allocation);
        if exit_ok == 0 {
            return Err(format!(
                "GetExitCodeThread(LoadLibraryW) failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        if remote_module_low32 == 0 {
            return Err("remote LoadLibraryW returned NULL".into());
        }
        Ok(())
    }

    pub(super) fn spawn_hooked(
        exe: &Path,
        g1r: &Path,
        args: &[&str],
        prep: &HookPreparation,
        capture: &Path,
        status: &Path,
        inject_delay: Duration,
    ) -> Result<HookSpawnOutcome, HookSpawnError> {
        validate_inject_delay(inject_delay).map_err(HookSpawnError::SafeFallback)?;
        let delay_deadline = Instant::now()
            .checked_add(inject_delay)
            .ok_or_else(|| HookSpawnError::SafeFallback("diagnostics delay overflow".into()))?;
        let child = std::process::Command::new(exe)
            .args(args)
            .current_dir(g1r)
            .env("GORE_AS_ERRFILE", capture)
            .env("GORE_AS_STATUSFILE", status)
            .spawn()
            .map_err(|e| HookSpawnError::SafeFallback(format!("launching generator: {e}")))?;
        let mut child = child;
        while Instant::now() < delay_deadline {
            match child.try_wait() {
                Ok(Some(_)) => return Ok(HookSpawnOutcome::ExitedBeforeInjection(child)),
                Ok(None) => std::thread::sleep(
                    Duration::from_millis(20)
                        .min(delay_deadline.saturating_duration_since(Instant::now())),
                ),
                Err(error) => {
                    return Err(HookSpawnError::Started {
                        child,
                        reason: format!("querying generator before diagnostics injection: {error}"),
                    });
                }
            }
        }
        let process = child.as_raw_handle() as HANDLE;
        if let Err(error) = inject_load_library(process, child.id(), &prep.hook_dll) {
            return Err(HookSpawnError::Started {
                child,
                reason: format!("injecting diagnostics helper: {error}"),
            });
        }
        let deadline = Instant::now() + Duration::from_secs(8);
        let hook_status = loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    return Ok(HookSpawnOutcome::ExitedAfterInjectionBeforeReady { child, status });
                }
                Ok(None) => {}
                Err(error) => {
                    return Err(HookSpawnError::Started {
                        child,
                        reason: format!("querying generator after diagnostics injection: {error}"),
                    });
                }
            }
            if let Ok(text) = read_status(status) {
                if let Some(token) = complete_status_token(&text) {
                    break token.to_string();
                }
            }
            if Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
            } else {
                return Err(HookSpawnError::Started {
                    child,
                    reason: "diagnostics helper did not report a complete ready/unavailable token within 8s".into(),
                });
            }
        };
        if hook_status.trim() != "ready" {
            return Err(HookSpawnError::Started {
                child,
                reason: format!("diagnostics helper unavailable: {}", hook_status.trim()),
            });
        }
        Ok(HookSpawnOutcome::Hooked(child))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static HOOK_TEMP_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn fake_pe(text: &[u8]) -> Vec<u8> {
        let mut pe = vec![0u8; 0x200 + text.len()];
        pe[0..2].copy_from_slice(b"MZ");
        pe[0x3c..0x40].copy_from_slice(&(0x80u32).to_le_bytes());
        pe[0x80..0x84].copy_from_slice(b"PE\0\0");
        pe[0x84..0x86].copy_from_slice(&(0x8664u16).to_le_bytes());
        pe[0x86..0x88].copy_from_slice(&(1u16).to_le_bytes());
        pe[0x94..0x96].copy_from_slice(&(0u16).to_le_bytes());
        let sh = 0x98;
        pe[sh..sh + 8].copy_from_slice(b".text\0\0\0");
        pe[sh + 8..sh + 12].copy_from_slice(&(text.len() as u32).to_le_bytes());
        pe[sh + 12..sh + 16].copy_from_slice(&(0x1000u32).to_le_bytes());
        pe[sh + 16..sh + 20].copy_from_slice(&(text.len() as u32).to_le_bytes());
        pe[sh + 20..sh + 24].copy_from_slice(&(0x200u32).to_le_bytes());
        pe[0x200..].copy_from_slice(text);
        pe
    }

    #[test]
    fn masked_aob_counts_zero_one_and_ambiguous() {
        let pattern = [Some(0xaa), None, Some(0xcc)];
        assert_eq!(count_masked_matches(&[0, 1, 2], &pattern), 0);
        assert_eq!(count_masked_matches(&[0xaa, 9, 0xcc], &pattern), 1);
        assert_eq!(
            count_masked_matches(&[0xaa, 1, 0xcc, 0xaa, 2, 0xcc], &pattern),
            2
        );
    }

    #[test]
    fn bundled_hook_bytes_match_the_pinned_integrity_hash() {
        use sha2::{Digest, Sha256};
        let actual = format!(
            "{:x}",
            Sha256::digest(include_bytes!("../assets/gore-as-diagnostics-hook.dll"))
        );
        assert_eq!(actual, BUNDLED_HOOK_SHA256);
    }

    #[test]
    fn pe_text_scan_ignores_signature_outside_text_and_rejects_duplicates() {
        let base = std::env::temp_dir().join(format!("gore-as-pe-test-{}", std::process::id()));
        let _ = std::fs::remove_file(&base);
        let pattern = [Some(0xaa), None, Some(0xcc)];
        std::fs::write(&base, fake_pe(&[0xaa, 7, 0xcc])).unwrap();
        assert_eq!(count_aob_matches_in_pe_text(&base, &pattern).unwrap(), 1);
        std::fs::write(&base, fake_pe(&[0xaa, 1, 0xcc, 0xaa, 2, 0xcc])).unwrap();
        assert_eq!(count_aob_matches_in_pe_text(&base, &pattern).unwrap(), 2);
        std::fs::remove_file(base).unwrap();
    }

    #[test]
    fn pe_text_scan_requires_amd64_and_the_exact_raw_backed_text_section() {
        let base =
            std::env::temp_dir().join(format!("gore-as-pe-domain-test-{}", std::process::id()));
        let pattern = [Some(0xaa), None, Some(0xcc)];

        let mut wrong_machine = fake_pe(&[0xaa, 1, 0xcc]);
        wrong_machine[0x84..0x86].copy_from_slice(&(0x014cu16).to_le_bytes());
        std::fs::write(&base, wrong_machine).unwrap();
        let error = aob_rvas_in_pe_text(&base, &pattern).unwrap_err();
        assert!(error.contains("not an AMD64 PE image"), "got: {error}");

        let mut prefixed_section = fake_pe(&[0xaa, 1, 0xcc]);
        prefixed_section[0x98..0xa0].copy_from_slice(b".textfoo");
        std::fs::write(&base, prefixed_section).unwrap();
        let error = aob_rvas_in_pe_text(&base, &pattern).unwrap_err();
        assert!(error.contains("has no .text section"), "got: {error}");

        // The second signature is raw alignment padding beyond VirtualSize. The mapped helper and
        // offline file scan both exclude it and therefore agree on the one raw-backed match.
        let mut padded = fake_pe(&[0xaa, 1, 0xcc, 0, 0xaa, 2, 0xcc]);
        padded[0xa0..0xa4].copy_from_slice(&(4u32).to_le_bytes());
        std::fs::write(&base, padded).unwrap();
        assert_eq!(aob_rvas_in_pe_text(&base, &pattern).unwrap(), vec![0x1000]);

        std::fs::remove_file(base).unwrap();
    }

    #[test]
    fn pe_text_scan_streams_across_chunk_boundaries_and_caps_ambiguity() {
        let base =
            std::env::temp_dir().join(format!("gore-as-pe-stream-test-{}", std::process::id()));
        let pattern = [Some(0xaa), Some(0xbb), Some(0xcc), Some(0xdd)];
        let mut text = vec![0u8; 1024 * 1024 + 32];
        let crossing = 1024 * 1024 - 2;
        text[crossing..crossing + 4].copy_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd]);
        std::fs::write(&base, fake_pe(&text)).unwrap();
        assert_eq!(
            aob_rvas_in_pe_text(&base, &pattern).unwrap(),
            vec![0x1000 + crossing as u64]
        );

        std::fs::write(
            &base,
            fake_pe(&[
                0xaa, 0xbb, 0xcc, 0xdd, 0, 0xaa, 0xbb, 0xcc, 0xdd, 0, 0xaa, 0xbb, 0xcc, 0xdd,
            ]),
        )
        .unwrap();
        assert_eq!(
            aob_rvas_in_pe_text(&base, &pattern).unwrap().len(),
            2,
            "two RVAs are sufficient to report ambiguity"
        );
        std::fs::remove_file(base).unwrap();
    }

    #[test]
    fn structured_probe_reports_hash_count_and_rva() {
        let exe = std::env::temp_dir().join(format!(
            "gore-as-structured-probe-{}.exe",
            std::process::id()
        ));
        let signature: Vec<u8> = LOG_ANGELSCRIPT_ERROR_AOB
            .iter()
            .map(|byte| byte.unwrap_or(0x42))
            .collect();
        std::fs::write(&exe, fake_pe(&signature)).unwrap();
        let probe = probe_executable(&exe).unwrap();
        assert_eq!(probe.sha256.len(), 64);
        assert!(probe.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_eq!(probe.match_count, 1);
        assert_eq!(probe.matched_rvas, vec![0x1000]);
        std::fs::remove_file(exe).unwrap();
    }

    #[test]
    fn current_game_executable_has_one_offline_signature_match_when_configured() {
        let Some(exe) = std::env::var_os("GORE_AS_REAL_EXE") else {
            eprintln!("skip: set GORE_AS_REAL_EXE");
            return;
        };
        let probe = probe_executable(Path::new(&exe)).unwrap();
        eprintln!("real executable probe: {probe:#?}");
        assert_eq!(probe.match_count, 1, "real executable AOB must be unique");
        assert_eq!(probe.matched_rvas.len(), 1);
    }

    #[cfg(windows)]
    #[test]
    fn signature_mismatch_does_not_materialize_or_leak_embedded_hook() {
        let _serial = HOOK_TEMP_TEST_LOCK.lock().unwrap();
        let exe =
            std::env::temp_dir().join(format!("gore-as-hook-mismatch-{}.exe", std::process::id()));
        let prefix = format!("gore-as-hook-{}-", std::process::id());
        let count_dirs = || {
            std::fs::read_dir(std::env::temp_dir())
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
                .count()
        };
        let before = count_dirs();
        std::fs::write(&exe, fake_pe(b"no matching signature here")).unwrap();
        let error = prepare_hook(&exe, &DiagnosticsOptions::default()).unwrap_err();
        assert!(error.contains("matched 0 times"), "got: {error}");
        assert_eq!(count_dirs(), before, "embedded helper temp leaked");
        std::fs::remove_file(exe).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn embedded_hook_is_materialized_and_raii_cleaned_after_unique_match() {
        let _serial = HOOK_TEMP_TEST_LOCK.lock().unwrap();
        let exe =
            std::env::temp_dir().join(format!("gore-as-hook-match-{}.exe", std::process::id()));
        let signature: Vec<u8> = LOG_ANGELSCRIPT_ERROR_AOB
            .iter()
            .map(|byte| byte.unwrap_or(0x42))
            .collect();
        std::fs::write(&exe, fake_pe(&signature)).unwrap();
        let prep = prepare_hook(&exe, &DiagnosticsOptions::default()).unwrap();
        let dll = prep.hook_dll.clone();
        let dir = dll.parent().unwrap().to_path_buf();
        assert!(dll.is_file());
        drop(prep);
        assert!(!dll.exists(), "embedded DLL was not removed");
        assert!(!dir.exists(), "embedded helper directory was not removed");
        std::fs::remove_file(exe).unwrap();
    }

    #[test]
    fn capture_parser_formats_normal_compiler_locations() {
        let input = "=== D:/Game/G1R/Script/Test.as ===\n\
                     (12:7) [E] No matching signatures to 'Foo()'\n\
                     (13:2) [W] Deprecated call\n\
                     (13:2) [I] Candidate: Foo(int)\n";
        let parsed = parse_capture(input);
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].severity, DiagnosticSeverity::Error);
        assert_eq!(parsed[0].line, 12);
        assert_eq!(
            format_capture(input),
            "D:/Game/G1R/Script/Test.as:12:7: error: No matching signatures to 'Foo()'\n\
             D:/Game/G1R/Script/Test.as:13:2: warning: Deprecated call\n\
             D:/Game/G1R/Script/Test.as:13:2: note: Candidate: Foo(int)\n"
        );
    }

    #[test]
    fn capture_formatter_preserves_unknown_protocol_lines() {
        let input = "=== Test.as ===\nfuture protocol payload\n[E] global error\n";
        assert_eq!(
            format_capture(input),
            "Test.as:0:0: note: [raw] future protocol payload\n\
             Test.as:0:0: error: global error\n"
        );
    }

    #[test]
    fn formatted_capture_is_bounded_under_repeated_long_section_expansion() {
        let section = "s".repeat(2048);
        let mut input = format!("=== {section} ===\n");
        for _ in 0..5_000 {
            input.push_str("[E] x\n");
        }
        let formatted = format_capture(&input);
        assert!(formatted.len() <= MAX_FORMATTED_BYTES);
        assert!(formatted.ends_with(FORMAT_TRUNCATED_MARKER));
    }

    #[test]
    fn status_protocol_waits_for_a_complete_recognized_token() {
        for partial in ["r", "rea", "ready", "unavailable: reason", "unknown\n"] {
            assert_eq!(complete_status_token(partial), None, "accepted {partial:?}");
        }
        assert_eq!(complete_status_token("ready\n"), Some("ready"));
        assert_eq!(
            complete_status_token("unavailable: signature matches=0\n"),
            Some("unavailable: signature matches=0")
        );
    }

    #[test]
    fn bounded_reader_treats_an_exact_cap_as_potentially_incomplete() {
        let path = std::env::temp_dir().join(format!("gore-as-bounded-cap-{}", std::process::id()));
        std::fs::write(&path, b"abcd").unwrap();
        let (text, incomplete) = read_bounded(&path, 4).unwrap();
        assert_eq!(text, "abcd");
        assert!(incomplete);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn injection_delay_has_a_hard_runtime_limit() {
        assert!(validate_inject_delay(MAX_INJECT_DELAY).is_ok());
        assert!(
            validate_inject_delay(MAX_INJECT_DELAY + std::time::Duration::from_millis(1)).is_err()
        );
    }

    #[test]
    fn remote_export_rva_is_relative_to_the_actual_provider_module() {
        assert_eq!(checked_module_rva(0x1_0000, 0x1_2345).unwrap(), 0x2345);
        assert!(checked_module_rva(0x2_0000, 0x1_ffff).is_err());
    }

    #[test]
    fn concise_formatter_hides_only_routine_compiling_info() {
        let input = "=== Test.as ===\n\
                     (1:1) [I] Compiling void Test()\n\
                     (2:3) [E] No matching signatures\n\
                     (2:3) [I] Calling function:\n\
                     (2:3) [I] void Test(int)\n";
        assert_eq!(parse_capture(input).len(), 4, "raw parse retains all info");
        assert_eq!(
            format_capture(input),
            "Test.as:2:3: error: No matching signatures\n\
             Test.as:2:3: note: Calling function:\n\
             Test.as:2:3: note: void Test(int)\n"
        );
    }
}
