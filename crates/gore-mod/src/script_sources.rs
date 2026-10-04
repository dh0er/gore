//! Original compile-time source payloads for script patches. These are optional for imported
//! binary mods, but authored producers retain them before compiling; source is never recovered
//! from a mini-cache and described as the author's original input.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Files, ModError, Result, ScriptEntry};

pub const SCRIPT_SOURCES_SCHEMA_VERSION_V1: u32 = 1;
pub const SCRIPT_MODULE_FINGERPRINT_ALGORITHM_V1: &str = "gore-as-semantic-module-v1";
pub const MAX_SCRIPT_SOURCES_MANIFEST_BYTES_V1: u64 = 16 * 1024 * 1024;
pub const MAX_SCRIPT_SOURCE_BYTES_V1: u64 = 16 * 1024 * 1024;
// Keep component packaging inside the same envelope as one native sparse-overlay rebuild.
// Manager applies these shared limits to the complete enabled source loadout as well.
pub const MAX_SCRIPT_SOURCE_TOTAL_BYTES_V1: u64 = 64 * 1024 * 1024;
pub const MAX_SCRIPT_SOURCE_ENTRIES_V1: usize = 256;
const MAX_IDENTITY_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptModuleFingerprintV1 {
    pub algorithm: String,
    /// Digest of a normalized semantic module record, never serialized runtime IDs.
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptSourceEntryV1 {
    pub module: String,
    /// Actual module operation against the original base: `add` or `edit`. The root must match
    /// its manifest entry's operation; other modules in the same mini may use a different op.
    pub op: String,
    /// Canonical compiler path, relative to its source tree, ending in `.as`.
    pub relative_path: String,
    /// Bundle-root-relative original UTF-8 source payload.
    pub source: String,
    pub source_sha256: String,
    /// Exact bundle-root-relative mini in scripts/manifest.json. A multi-module mini is atomic.
    pub mini: String,
    pub mini_sha256: String,
    /// Required for edits; absent for additions. Algorithm identity is part of the contract.
    pub original_module: Option<ScriptModuleFingerprintV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptSourcesManifestV1 {
    pub schema_version: u32,
    /// Lowercase hex of the cache's 16-byte GUID in serialized byte order.
    pub base_cache_guid: String,
    pub base_cache_sha256: String,
    pub entries: Vec<ScriptSourceEntryV1>,
}

/// Byte-backed input retained from the exact source supplied to the compiler. A producer must
/// not reopen a mutable author's file after compilation to populate this value.
#[derive(Debug, Clone)]
pub struct ScriptSourceInputV1 {
    pub module: String,
    pub op: String,
    pub relative_path: String,
    pub mini: String,
    pub source: Vec<u8>,
}

fn invalid(message: impl Into<String>) -> ModError {
    ModError::Other(format!("script sources: {}", message.into()))
}

pub fn script_source_sha256_v1(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn path(value: &str) -> bool {
    value.len() <= MAX_IDENTITY_BYTES && crate::is_safe_rel_path(value)
}

fn source_path(value: &str) -> bool {
    path(value)
        && value
            .get(value.len().saturating_sub(3)..)
            .is_some_and(|suffix| suffix.eq_ignore_ascii_case(".as"))
}

fn source_bytes(bytes: &[u8], limit: u64) -> Result<()> {
    if bytes.len() as u64 > limit {
        return Err(invalid(format!(
            "source payload exceeds the {limit}-byte limit"
        )));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("source payload is not UTF-8"))?;
    if text.contains('\0') {
        return Err(invalid("source payload contains NUL"));
    }
    Ok(())
}

fn remaining_source_payload_limit(retained: u64) -> Result<u64> {
    MAX_SCRIPT_SOURCE_TOTAL_BYTES_V1
        .checked_sub(retained)
        .map(|remaining| remaining.min(MAX_SCRIPT_SOURCE_BYTES_V1))
        .ok_or_else(|| invalid("source payloads exceed the cumulative byte limit"))
}

fn charge_source_payload_bytes(retained: &mut u64, byte_len: u64) -> Result<()> {
    let limit = remaining_source_payload_limit(*retained)?;
    if byte_len > limit {
        return Err(invalid(format!(
            "source payload exceeds the {limit}-byte remaining limit"
        )));
    }
    *retained += byte_len;
    Ok(())
}

impl ScriptSourcesManifestV1 {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != SCRIPT_SOURCES_SCHEMA_VERSION_V1 {
            return Err(invalid(format!(
                "unsupported schema version {}",
                self.schema_version
            )));
        }
        if !hex(&self.base_cache_guid, 32) || !hex(&self.base_cache_sha256, 64) {
            return Err(invalid("invalid base cache GUID or SHA-256"));
        }
        if self.entries.is_empty() || self.entries.len() > MAX_SCRIPT_SOURCE_ENTRIES_V1 {
            return Err(invalid("empty or oversized source entry list"));
        }
        let mut identities = BTreeSet::new();
        let mut source_paths = BTreeSet::new();
        let mut compiler_paths = BTreeSet::new();
        for entry in &self.entries {
            if entry.module.is_empty()
                || entry.module.len() > MAX_IDENTITY_BYTES
                || entry.module.chars().any(char::is_control)
                || !source_path(&entry.relative_path)
                || !source_path(&entry.source)
                || !path(&entry.mini)
                || !hex(&entry.source_sha256, 64)
                || !hex(&entry.mini_sha256, 64)
            {
                return Err(invalid(format!(
                    "invalid source identity for {:?}",
                    entry.module
                )));
            }
            match (entry.op.as_str(), &entry.original_module) {
                ("add", None) => {}
                ("edit", Some(fingerprint))
                    if fingerprint.algorithm == SCRIPT_MODULE_FINGERPRINT_ALGORITHM_V1
                        && hex(&fingerprint.sha256, 64) => {}
                _ => {
                    return Err(invalid(format!(
                        "invalid operation/baseline for {:?}",
                        entry.module
                    )))
                }
            }
            let mini = entry.mini.to_lowercase();
            if !identities.insert((mini.clone(), entry.module.to_lowercase()))
                || !compiler_paths.insert((mini, entry.relative_path.to_lowercase()))
                || !source_paths.insert(entry.source.to_lowercase())
            {
                return Err(invalid("duplicate or case-colliding source identity"));
            }
        }
        Ok(())
    }

    /// Partial components may contain existing binary-only minis alongside authored minis.
    /// Rebuild eligibility is per complete mini group, never inferred merely from sidecar presence.
    pub fn covers_all_scripts(&self, scripts: &[ScriptEntry]) -> bool {
        scripts
            .iter()
            .all(|script| self.entries.iter().any(|source| source.mini == script.mini))
    }

    pub fn to_json(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self)?;
        if bytes.len() as u64 > MAX_SCRIPT_SOURCES_MANIFEST_BYTES_V1 {
            return Err(invalid("source manifest exceeds byte limit"));
        }
        Ok(bytes)
    }
}

pub fn read_script_sources_manifest_v1(bytes: &[u8]) -> Result<ScriptSourcesManifestV1> {
    if bytes.len() as u64 > MAX_SCRIPT_SOURCES_MANIFEST_BYTES_V1 {
        return Err(invalid("source manifest exceeds byte limit"));
    }
    let manifest: ScriptSourcesManifestV1 = serde_json::from_slice(bytes)?;
    manifest.validate()?;
    Ok(manifest)
}

/// Shared validation for packaging and manager import/apply. The reader must reject links and
/// enforce its supplied bound. Source totals are charged across all mini groups before retention.
pub fn validate_script_sources_v1(
    manifest: &ScriptSourcesManifestV1,
    scripts: &[ScriptEntry],
    mut read: impl FnMut(&str, u64) -> Result<Vec<u8>>,
) -> Result<()> {
    manifest.validate()?;
    let mut groups = BTreeMap::<&str, Vec<&ScriptSourceEntryV1>>::new();
    let mut retained = 0u64;
    for entry in &manifest.entries {
        let limit = remaining_source_payload_limit(retained)?;
        let bytes = read(&entry.source, limit)?;
        source_bytes(&bytes, limit)?;
        if script_source_sha256_v1(&bytes) != entry.source_sha256 {
            return Err(invalid(format!(
                "source hash mismatch for {:?}",
                entry.module
            )));
        }
        charge_source_payload_bytes(&mut retained, bytes.len() as u64)?;
        groups.entry(&entry.mini).or_default().push(entry);
    }
    let mut mini_total = 0u64;
    for (mini, sources) in groups {
        let owners: Vec<_> = scripts
            .iter()
            .filter(|script| script.mini == mini)
            .collect();
        let [owner] = owners.as_slice() else {
            return Err(invalid(format!(
                "mini {mini:?} must match exactly one script manifest entry"
            )));
        };
        if owner.op != "add" && owner.op != "edit" {
            return Err(invalid(
                "source-owned mini has an invalid manifest operation",
            ));
        }
        if let Some(root) = sources.iter().find(|source| source.module == owner.module) {
            if root.op != owner.op {
                return Err(invalid(format!(
                    "root source operation for {:?} differs from the owning script manifest entry",
                    owner.module
                )));
            }
        }
        let mini_limit =
            crate::MAX_SCRIPT_MINI_BYTES.min(crate::MAX_SCRIPT_MINI_TOTAL_BYTES - mini_total);
        let bytes = read(mini, mini_limit)?;
        if bytes.len() as u64 > mini_limit {
            return Err(invalid("mini payload exceeds byte limit"));
        }
        mini_total += bytes.len() as u64;
        let sha = script_source_sha256_v1(&bytes);
        if sources.iter().any(|source| source.mini_sha256 != sha) {
            return Err(invalid(format!("mini hash mismatch for {mini:?}")));
        }
        let header = gore_as::cache::header::CacheHeader::parse(&bytes)
            .map_err(|error| invalid(format!("invalid mini header: {error}")))?;
        let guid: String = header
            .hash
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        if guid != manifest.base_cache_guid {
            return Err(invalid(format!(
                "mini GUID differs from source baseline for {mini:?}"
            )));
        }
        let modules = gore_as::compile::base_full_graph_manifest_v1(&bytes)
            .map_err(|error| invalid(format!("invalid source-owned mini: {error}")))?;
        if modules.len() != sources.len()
            || !modules
                .iter()
                .any(|module| module.module_name == owner.module)
            || !modules.iter().all(|module| {
                sources.iter().any(|source| {
                    source.module == module.module_name
                        && source.relative_path == module.relative_path
                })
            })
        {
            return Err(invalid(format!(
                "sources do not completely cover canonical modules in {mini:?}"
            )));
        }
    }
    Ok(())
}

/// Load an optional sidecar from one component. Absence means binary-only; malformed metadata
/// or changed payloads are errors, never a reason to silently downgrade rebuild eligibility.
pub fn load_script_sources_v1(
    bundle_root: &Path,
    component_path: &str,
    scripts: &[ScriptEntry],
) -> Result<Option<ScriptSourcesManifestV1>> {
    if !path(component_path) {
        return Err(invalid("unsafe component path"));
    }
    let relative = format!("{component_path}/sources.json");
    match std::fs::symlink_metadata(bundle_root.join(&relative)) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(crate::io("reading script source manifest metadata")(error)),
        Ok(_) => {}
    }
    let bytes = crate::read_safe_bundle_file(
        bundle_root,
        Path::new(&relative),
        "script sources manifest",
        MAX_SCRIPT_SOURCES_MANIFEST_BYTES_V1,
    )?;
    let manifest = read_script_sources_manifest_v1(&bytes)?;
    validate_script_sources_v1(&manifest, scripts, |relative, limit| {
        crate::read_safe_bundle_file(
            bundle_root,
            Path::new(relative),
            "script sources payload",
            limit,
        )
    })?;
    Ok(Some(manifest))
}

/// Produce a sidecar and original source files against the sealed cache used for compilation.
/// The caller supplies only complete mini groups. Uncovered pre-existing minis remain binary-only.
pub fn package_script_sources_v1(
    files: &mut Files,
    component_path: &str,
    scripts: &[ScriptEntry],
    base_cache: &[u8],
    mut authored: Vec<ScriptSourceInputV1>,
) -> Result<ScriptSourcesManifestV1> {
    if !path(component_path) || authored.is_empty() || authored.len() > MAX_SCRIPT_SOURCE_ENTRIES_V1
    {
        return Err(invalid("invalid source component or source count"));
    }
    let header = gore_as::cache::header::CacheHeader::parse(base_cache)
        .map_err(|error| invalid(format!("invalid base cache: {error}")))?;
    let observation =
        gore_as::cache::semantic_observer::observe_whole_cache_semantics_v1(base_cache, None)
            .map_err(|error| invalid(format!("cannot fingerprint original modules: {error}")))?;
    let base_modules = gore_as::compile::base_full_graph_manifest_v1(base_cache)
        .map_err(|error| invalid(format!("invalid base module identities: {error}")))?;
    authored.sort_by(|a, b| (&a.mini, &a.module).cmp(&(&b.mini, &b.module)));
    let mut payloads = Files::new();
    let mut entries = Vec::new();
    let mut retained = 0u64;
    for (index, input) in authored.into_iter().enumerate() {
        let limit = remaining_source_payload_limit(retained)?;
        source_bytes(&input.source, limit)?;
        charge_source_payload_bytes(&mut retained, input.source.len() as u64)?;
        let original_module = match input.op.as_str() {
            "edit" => {
                let original = observation
                    .module_identities()
                    .iter()
                    .find(|module| module.map_key() == input.module)
                    .ok_or_else(|| {
                        invalid(format!("edit module {:?} missing from base", input.module))
                    })?;
                if !base_modules.iter().any(|module| {
                    module.module_name == input.module
                        && module.relative_path == input.relative_path
                }) {
                    return Err(invalid("edit module compiler path differs from the base"));
                }
                Some(ScriptModuleFingerprintV1 {
                    algorithm: SCRIPT_MODULE_FINGERPRINT_ALGORITHM_V1.into(),
                    sha256: original
                        .semantic_sha256()
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect(),
                })
            }
            "add"
                if !base_modules.iter().any(|module| {
                    module.module_name.to_lowercase() == input.module.to_lowercase()
                        || module.relative_path.to_lowercase() == input.relative_path.to_lowercase()
                }) =>
            {
                None
            }
            _ => {
                return Err(invalid(
                    "invalid add/edit source operation or existing add identity",
                ))
            }
        };
        let mini = files
            .get(&input.mini)
            .ok_or_else(|| invalid("source-owned mini missing"))?;
        let source = format!("{component_path}/source/{index}.as");
        entries.push(ScriptSourceEntryV1 {
            module: input.module,
            op: input.op,
            relative_path: input.relative_path,
            source: source.clone(),
            source_sha256: script_source_sha256_v1(&input.source),
            mini: input.mini,
            mini_sha256: script_source_sha256_v1(mini),
            original_module,
        });
        payloads.insert(source, input.source);
    }
    let manifest = ScriptSourcesManifestV1 {
        schema_version: SCRIPT_SOURCES_SCHEMA_VERSION_V1,
        base_cache_guid: header
            .hash
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        base_cache_sha256: script_source_sha256_v1(base_cache),
        entries,
    };
    validate_script_sources_v1(&manifest, scripts, |relative, limit| {
        let bytes = payloads
            .get(relative)
            .or_else(|| files.get(relative))
            .ok_or_else(|| invalid(format!("missing source payload {relative:?}")))?;
        if bytes.len() as u64 > limit {
            return Err(invalid("payload exceeds byte limit"));
        }
        Ok(bytes.clone())
    })?;
    payloads.insert(
        format!("{component_path}/sources.json"),
        manifest.to_json()?,
    );
    if payloads.keys().any(|key| files.contains_key(key)) {
        return Err(invalid(
            "source packaging would overwrite an existing bundle file",
        ));
    }
    files.extend(payloads);
    Ok(manifest)
}

/// Compiler producers publish this adjacent manifest as `<mini filename>.sources.json`.
pub fn script_source_provenance_path_v1(mini: &Path) -> PathBuf {
    let mut path = mini.as_os_str().to_os_string();
    path.push(".sources.json");
    PathBuf::from(path)
}

/// Run before compilation so an existing source publication cannot turn a successful lengthy
/// compiler run into a late no-clobber failure.
pub fn preflight_script_source_provenance_v1(mini_path: &Path) -> Result<()> {
    let mini_name = mini_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| crate::is_safe_filename(name))
        .ok_or_else(|| invalid("unsafe compiler output filename"))?;
    let parent = mini_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    for destination in [
        script_source_provenance_path_v1(mini_path),
        parent.join(format!("{mini_name}.sources")),
    ] {
        match std::fs::symlink_metadata(&destination) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
            Err(error) => return Err(crate::io("checking compiler source output")(error)),
            Ok(_) => return Err(invalid(format!("source output already exists at {}; choose a new mini output or remove the previous generated output before recompiling", destination.display()))),
        }
    }
    Ok(())
}

/// Capture one freshly compiled mini, including every authored module it carries, and publish
/// provenance adjacent to it. Callers retain the source bytes and pin their hash for compilation.
/// Publication never overwrites a previous source directory or manifest.
pub fn write_script_source_provenance_v1(
    mini_path: &Path,
    base_cache: &[u8],
    authored: Vec<ScriptSourceInputV1>,
) -> Result<()> {
    let mini = crate::read_regular_file_limited(
        mini_path,
        "compiled source mini",
        crate::MAX_SCRIPT_MINI_BYTES,
    )?;
    write_script_source_provenance_from_bytes_v1(mini_path, &mini, base_cache, authored)
}

/// Publish source provenance bound to the exact retained compiler output bytes. Validate the
/// published mini against those bytes rather than accepting a replaced compiler-output path.
pub fn write_script_source_provenance_from_bytes_v1(
    mini_path: &Path,
    compiled_mini: &[u8],
    base_cache: &[u8],
    mut authored: Vec<ScriptSourceInputV1>,
) -> Result<()> {
    preflight_script_source_provenance_v1(mini_path)?;
    let mini_name = mini_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| crate::is_safe_filename(name))
        .ok_or_else(|| invalid("unsafe compiler output filename"))?;
    let parent = mini_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mini = crate::read_safe_bundle_file(
        parent,
        Path::new(mini_name),
        "compiled source mini",
        crate::MAX_SCRIPT_MINI_BYTES,
    )?;
    if mini != compiled_mini {
        return Err(invalid(
            "published mini differs from the retained compiler output",
        ));
    }
    let first = authored
        .iter()
        .find(|source| source.op == "edit")
        .or_else(|| authored.first())
        .ok_or_else(|| invalid("compiled mini has no retained source"))?;
    let entries = vec![ScriptEntry {
        op: first.op.clone(),
        module: first.module.clone(),
        mini: mini_name.into(),
    }];
    for input in &mut authored {
        input.mini = mini_name.into();
    }
    let component = format!("{mini_name}.sources");
    let mut files = Files::from([(mini_name.into(), mini)]);
    let manifest =
        package_script_sources_v1(&mut files, &component, &entries, base_cache, authored)?;
    let directory = parent.join(&component);
    std::fs::create_dir(&directory).map_err(crate::io("creating compiler source provenance"))?;
    let result = (|| {
        std::fs::create_dir(directory.join("source"))
            .map_err(crate::io("creating compiler source payloads"))?;
        for entry in &manifest.entries {
            let bytes = &files[&entry.source];
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(parent.join(&entry.source))
                .map_err(crate::io("publishing compiler source payload"))?;
            file.write_all(bytes)
                .map_err(crate::io("writing compiler source payload"))?;
            file.sync_all()
                .map_err(crate::io("syncing compiler source payload"))?;
        }
        use std::io::Write;
        let mut staging = tempfile::NamedTempFile::new_in(parent)
            .map_err(crate::io("staging compiler source manifest"))?;
        staging
            .write_all(&manifest.to_json()?)
            .map_err(crate::io("writing compiler source manifest"))?;
        staging
            .as_file()
            .sync_all()
            .map_err(crate::io("syncing compiler source manifest"))?;
        staging
            .persist_noclobber(script_source_provenance_path_v1(mini_path))
            .map_err(crate::io("publishing compiler source manifest"))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(directory);
    }
    result
}

/// Add compile-time sidecars associated with supplied minis to the newly packaged component.
/// Existing explicit mini inputs without adjacent provenance remain accepted as binary-only.
pub(crate) fn package_adjacent_script_sources_v1(
    files: &mut Files,
    component_path: &str,
    scripts: &[ScriptEntry],
    mini_paths: &[PathBuf],
) -> Result<()> {
    let mut combined: Option<ScriptSourcesManifestV1> = None;
    let mut payloads = Files::new();
    let mut source_total = 0u64;
    for (script, mini_path) in scripts.iter().zip(mini_paths) {
        let sidecar = script_source_provenance_path_v1(mini_path);
        match std::fs::symlink_metadata(&sidecar) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(crate::io("reading compiler provenance metadata")(error)),
            Ok(_) => {}
        }
        let root = mini_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let relative = sidecar
            .file_name()
            .ok_or_else(|| invalid("missing compiler provenance name"))?;
        let bytes = crate::read_safe_bundle_file(
            root,
            Path::new(relative),
            "compiler source provenance",
            MAX_SCRIPT_SOURCES_MANIFEST_BYTES_V1,
        )?;
        let mut manifest = read_script_sources_manifest_v1(&bytes)?;
        if combined
            .as_ref()
            .map_or(0, |manifest| manifest.entries.len())
            + manifest.entries.len()
            > MAX_SCRIPT_SOURCE_ENTRIES_V1
        {
            return Err(invalid("combined compiler sources exceed entry limit"));
        }
        let groups: BTreeSet<_> = manifest
            .entries
            .iter()
            .map(|source| source.mini.clone())
            .collect();
        if groups.len() != 1 {
            return Err(invalid(
                "adjacent provenance must identify exactly one mini",
            ));
        }
        let original_mini = groups.into_iter().next().unwrap();
        let source_entries = vec![ScriptEntry {
            op: script.op.clone(),
            module: script.module.clone(),
            mini: original_mini.clone(),
        }];
        // Validate against the mini bytes already retained by packaging. Never reopen the mini
        // after source capture and accidentally bind a later output to earlier source metadata.
        let mut captured = Files::new();
        validate_script_sources_v1(&manifest, &source_entries, |relative, limit| {
            let bytes = if relative == original_mini {
                files
                    .get(&script.mini)
                    .ok_or_else(|| invalid("packaged source mini missing"))?
                    .clone()
            } else {
                let limit = limit.min(remaining_source_payload_limit(source_total)?);
                let bytes = crate::read_safe_bundle_file(
                    root,
                    Path::new(relative),
                    "compiler source payload",
                    limit,
                )?;
                source_bytes(&bytes, limit)?;
                charge_source_payload_bytes(&mut source_total, bytes.len() as u64)?;
                bytes
            };
            if bytes.len() as u64 > limit {
                return Err(invalid("payload exceeds byte limit"));
            }
            captured.insert(relative.into(), bytes.clone());
            Ok(bytes)
        })?;
        if let Some(existing) = &combined {
            if existing.base_cache_guid != manifest.base_cache_guid
                || existing.base_cache_sha256 != manifest.base_cache_sha256
            {
                return Err(invalid(
                    "authored minis were compiled against different base caches; rebuild together",
                ));
            }
        }
        let offset = combined
            .as_ref()
            .map_or(0, |manifest| manifest.entries.len());
        for (index, entry) in manifest.entries.iter_mut().enumerate() {
            let original_source = entry.source.clone();
            entry.source = format!("{component_path}/source/{}.as", offset + index);
            entry.mini = script.mini.clone();
            payloads.insert(
                entry.source.clone(),
                captured
                    .remove(&original_source)
                    .ok_or_else(|| invalid("captured source missing"))?,
            );
        }
        match &mut combined {
            Some(existing) => existing.entries.extend(manifest.entries),
            None => combined = Some(manifest),
        }
    }
    if let Some(manifest) = combined {
        validate_script_sources_v1(&manifest, scripts, |relative, limit| {
            let bytes = payloads
                .get(relative)
                .or_else(|| files.get(relative))
                .ok_or_else(|| invalid("packaged source payload missing"))?;
            if bytes.len() as u64 > limit {
                return Err(invalid("payload exceeds byte limit"));
            }
            Ok(bytes.clone())
        })?;
        payloads.insert(
            format!("{component_path}/sources.json"),
            manifest.to_json()?,
        );
        if payloads.keys().any(|key| files.contains_key(key)) {
            return Err(invalid("source payload collision"));
        }
        files.extend(payloads);
    }
    Ok(())
}

/// Package an explicit source manifest whose payload/mini paths are relative to the spec's
/// directory. This supports externally compiled authored inputs without pretending binary minis
/// contain the original text. The mini paths must exactly match the corresponding build inputs.
pub fn package_explicit_script_sources_v1(
    files: &mut Files,
    component_path: &str,
    scripts: &[ScriptEntry],
    original_scripts: &[ScriptEntry],
    source_root: &Path,
    manifest: &ScriptSourcesManifestV1,
) -> Result<()> {
    if !path(component_path) || scripts.len() < original_scripts.len() {
        return Err(invalid("invalid explicit source mapping"));
    }
    let manifest_path = format!("{component_path}/sources.json");
    let mut combined = match files.get(&manifest_path) {
        Some(bytes) => {
            let existing = read_script_sources_manifest_v1(bytes)?;
            if existing.base_cache_guid != manifest.base_cache_guid
                || existing.base_cache_sha256 != manifest.base_cache_sha256
            {
                return Err(invalid(
                    "explicit and captured sources have different base caches",
                ));
            }
            existing
        }
        None => ScriptSourcesManifestV1 {
            entries: Vec::new(),
            ..manifest.clone()
        },
    };
    if combined.entries.len() + manifest.entries.len() > MAX_SCRIPT_SOURCE_ENTRIES_V1 {
        return Err(invalid("combined source manifest exceeds entry limit"));
    }
    let existing_minis: BTreeSet<_> = combined
        .entries
        .iter()
        .map(|entry| entry.mini.clone())
        .collect();
    let mut source_total = 0u64;
    for entry in &combined.entries {
        let length = files
            .get(&entry.source)
            .ok_or_else(|| invalid("existing source payload missing"))?
            .len() as u64;
        source_total = source_total
            .checked_add(length)
            .filter(|total| *total <= MAX_SCRIPT_SOURCE_TOTAL_BYTES_V1)
            .ok_or_else(|| invalid("existing sources exceed total limit"))?;
    }
    let mut captured = Files::new();
    validate_script_sources_v1(manifest, original_scripts, |relative, limit| {
        let bytes = if let Some(index) = original_scripts
            .iter()
            .position(|entry| entry.mini == relative)
        {
            files
                .get(&scripts[index].mini)
                .ok_or_else(|| invalid("explicit source mini missing from bundle"))?
                .clone()
        } else {
            let limit = limit.min(remaining_source_payload_limit(source_total)?);
            let bytes = crate::read_safe_bundle_file(
                source_root,
                Path::new(relative),
                "explicit script source",
                limit,
            )?;
            source_bytes(&bytes, limit)?;
            charge_source_payload_bytes(&mut source_total, bytes.len() as u64)?;
            bytes
        };
        if bytes.len() as u64 > limit {
            return Err(invalid("explicit source exceeds byte limit"));
        }
        captured.insert(relative.into(), bytes.clone());
        Ok(bytes)
    })?;
    let mut payloads = Files::new();
    let offset = combined.entries.len();
    for (index, original) in manifest.entries.iter().enumerate() {
        let owner = original_scripts
            .iter()
            .position(|script| script.mini == original.mini)
            .ok_or_else(|| invalid("explicit source mini has no build input"))?;
        let mut entry = original.clone();
        entry.mini = scripts[owner].mini.clone();
        if existing_minis.contains(&entry.mini) {
            return Err(invalid(
                "mini already has captured sources; omit its explicit source metadata",
            ));
        }
        entry.source = format!("{component_path}/source/{}.as", offset + index);
        payloads.insert(
            entry.source.clone(),
            captured
                .remove(&original.source)
                .ok_or_else(|| invalid("explicit source payload missing"))?,
        );
        combined.entries.push(entry);
    }
    validate_script_sources_v1(&combined, scripts, |relative, limit| {
        let bytes = payloads
            .get(relative)
            .or_else(|| files.get(relative))
            .ok_or_else(|| invalid("combined source payload missing"))?;
        if bytes.len() as u64 > limit {
            return Err(invalid("combined source exceeds byte limit"));
        }
        Ok(bytes.clone())
    })?;
    if payloads.keys().any(|key| files.contains_key(key)) {
        return Err(invalid("explicit source payload collision"));
    }
    payloads.insert(manifest_path, combined.to_json()?);
    files.extend(payloads);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cache(names: &[&str], guid: u8) -> Vec<u8> {
        cache_with_paths(
            &names
                .iter()
                .map(|name| (*name, format!("{name}.as")))
                .collect::<Vec<_>>(),
            guid,
        )
    }

    fn cache_with_paths(modules: &[(&str, String)], guid: u8) -> Vec<u8> {
        fn string(value: &str, fstring: bool) -> Vec<u8> {
            let length = value.len() as i32 + i32::from(fstring);
            let mut bytes = length.to_le_bytes().to_vec();
            if !value.is_empty() || fstring {
                bytes.extend_from_slice(value.as_bytes());
                bytes.push(0);
            }
            bytes
        }
        let mut bytes = vec![guid; 16];
        bytes.extend_from_slice(&gore_as::cache::header::CACHE_MAGIC.to_le_bytes());
        bytes.extend_from_slice(&(modules.len() as u32).to_le_bytes());
        for (name, relative_path) in modules {
            bytes.extend(string(name, true));
            bytes.extend(string(name, false));
            bytes.extend_from_slice(&[0; 32]);
            bytes.extend(string("", false));
            bytes.extend_from_slice(&[0; 8]);
            bytes.extend(string(relative_path, false));
            bytes.extend_from_slice(&[0; 4]);
        }
        bytes.extend(vec![0; 4 * gore_as::cache::tables::N_TABLES]);
        bytes
    }

    fn input(module: &str, op: &str, mini: &str) -> ScriptSourceInputV1 {
        ScriptSourceInputV1 {
            module: module.into(),
            op: op.into(),
            relative_path: format!("{module}.as"),
            mini: mini.into(),
            source: format!("// authored {module}: ä\r\n// comments and spacing must survive\r\n")
                .into_bytes(),
        }
    }

    fn fixture() -> (Files, Vec<ScriptEntry>, ScriptSourcesManifestV1) {
        let mut files =
            Files::from([("scripts/mixed.cache".into(), cache(&["Vanilla", "New"], 1))]);
        let scripts = vec![ScriptEntry {
            module: "Vanilla".into(),
            op: "edit".into(),
            mini: "scripts/mixed.cache".into(),
        }];
        let manifest = package_script_sources_v1(
            &mut files,
            "scripts",
            &scripts,
            &cache(&["Vanilla"], 1),
            vec![
                input("Vanilla", "edit", "scripts/mixed.cache"),
                input("New", "add", "scripts/mixed.cache"),
            ],
        )
        .unwrap();
        (files, scripts, manifest)
    }

    fn validate(
        manifest: &ScriptSourcesManifestV1,
        scripts: &[ScriptEntry],
        files: &Files,
    ) -> Result<()> {
        validate_script_sources_v1(manifest, scripts, |path, limit| {
            let bytes = files
                .get(path)
                .ok_or_else(|| invalid("test payload missing"))?;
            if bytes.len() as u64 > limit {
                return Err(invalid("test read limit"));
            }
            Ok(bytes.clone())
        })
    }

    #[test]
    fn mixed_multi_module_mini_retains_actual_operations_and_original_sources() {
        let (files, scripts, manifest) = fixture();
        validate(&manifest, &scripts, &files).unwrap();
        assert!(manifest.covers_all_scripts(&scripts));
        let add = manifest
            .entries
            .iter()
            .find(|entry| entry.module == "New")
            .unwrap();
        assert_eq!(add.op, "add");
        assert!(add.original_module.is_none());
        let edit = manifest
            .entries
            .iter()
            .find(|entry| entry.module == "Vanilla")
            .unwrap();
        assert_eq!(edit.op, "edit");
        assert_eq!(
            edit.original_module.as_ref().unwrap().algorithm,
            SCRIPT_MODULE_FINGERPRINT_ALGORITHM_V1
        );
        assert_eq!(files[&edit.source], input("Vanilla", "edit", "").source);
        assert_eq!(
            read_script_sources_manifest_v1(&files["scripts/sources.json"]).unwrap(),
            manifest
        );
    }

    #[test]
    fn rejects_partial_or_unrelated_mini_groups_and_changed_payloads() {
        let (files, scripts, manifest) = fixture();
        let mut partial = manifest.clone();
        partial.entries.pop();
        assert!(validate(&partial, &scripts, &files)
            .unwrap_err()
            .to_string()
            .contains("completely cover"));
        let mut unrelated = scripts.clone();
        unrelated[0].module = "MissingRoot".into();
        assert!(validate(&manifest, &unrelated, &files).is_err());
        let mut changed = files.clone();
        changed.insert(
            manifest.entries[0].source.clone(),
            b"// different source".to_vec(),
        );
        assert!(validate(&manifest, &scripts, &changed)
            .unwrap_err()
            .to_string()
            .contains("source hash"));
        let mut changed_mini = files.clone();
        changed_mini.get_mut("scripts/mixed.cache").unwrap()[0] ^= 1;
        assert!(validate(&manifest, &scripts, &changed_mini)
            .unwrap_err()
            .to_string()
            .contains("mini hash"));
        let mut bad_guid = manifest.clone();
        bad_guid.base_cache_guid = "02".repeat(16);
        assert!(validate(&bad_guid, &scripts, &files)
            .unwrap_err()
            .to_string()
            .contains("GUID"));
    }

    #[test]
    fn rejects_mismatched_root_operations_while_other_module_operations_may_differ() {
        let (files, scripts, manifest) = fixture();
        // The root edits Vanilla and the same atomic mini also adds New.
        validate(&manifest, &scripts, &files).unwrap();
        let mut add_owner = scripts.clone();
        add_owner[0].op = "add".into();
        assert!(validate(&manifest, &add_owner, &files)
            .unwrap_err()
            .to_string()
            .contains("root source operation"));

        let mut mislabeled_root = manifest.clone();
        let root = mislabeled_root
            .entries
            .iter_mut()
            .find(|entry| entry.module == "Vanilla")
            .unwrap();
        root.op = "add".into();
        root.original_module = None;
        // Entry-level schema validation succeeds; the owning manifest supplies the missing proof.
        mislabeled_root.validate().unwrap();
        assert!(validate(&mislabeled_root, &scripts, &files)
            .unwrap_err()
            .to_string()
            .contains("root source operation"));

        let new_root = vec![ScriptEntry {
            op: "add".into(),
            module: "New".into(),
            mini: scripts[0].mini.clone(),
        }];
        validate(&manifest, &new_root, &files).unwrap();
    }

    #[test]
    fn schema_envelope_matches_native_rebuild_and_rejects_the_257th_source() {
        assert_eq!(
            MAX_SCRIPT_SOURCE_ENTRIES_V1,
            gore_as::manager_rebuild::MAX_MANAGER_REBUILD_MODULES_V1
        );
        assert_eq!(
            MAX_SCRIPT_SOURCE_TOTAL_BYTES_V1,
            gore_as::manager_rebuild::MAX_MANAGER_REBUILD_SOURCE_BYTES_V1 as u64
        );
        assert_eq!(
            MAX_SCRIPT_SOURCE_BYTES_V1,
            gore_as::manager_rebuild::MAX_MANAGER_REBUILD_SOURCE_FILE_BYTES_V1 as u64
        );
        let (_, _, mut manifest) = fixture();
        let template = manifest
            .entries
            .iter()
            .find(|entry| entry.op == "add")
            .unwrap()
            .clone();
        manifest.entries = (0..MAX_SCRIPT_SOURCE_ENTRIES_V1)
            .map(|index| ScriptSourceEntryV1 {
                module: format!("Mod{index}"),
                relative_path: format!("Mod{index}.as"),
                source: format!("scripts/source/{index}.as"),
                ..template.clone()
            })
            .collect();
        manifest.validate().unwrap();
        manifest.entries.push(ScriptSourceEntryV1 {
            module: "Extra".into(),
            relative_path: "Extra.as".into(),
            source: "scripts/source/extra.as".into(),
            ..template
        });
        assert!(manifest
            .validate()
            .unwrap_err()
            .to_string()
            .contains("oversized source entry list"));
    }

    #[test]
    fn cumulative_source_budget_rejects_five_13_mib_files_without_allocating_payloads() {
        let mib = 1024 * 1024;
        let mut retained = 0;
        for _ in 0..4 {
            assert!(remaining_source_payload_limit(retained).unwrap() >= 13 * mib);
            charge_source_payload_bytes(&mut retained, 13 * mib).unwrap();
        }
        assert_eq!(retained, 52 * mib);
        assert_eq!(remaining_source_payload_limit(retained).unwrap(), 12 * mib);
        assert!(charge_source_payload_bytes(&mut retained, 13 * mib).is_err());
        assert_eq!(retained, 52 * mib);
        charge_source_payload_bytes(&mut retained, 12 * mib).unwrap();
        assert_eq!(retained, MAX_SCRIPT_SOURCE_TOTAL_BYTES_V1);
        assert_eq!(remaining_source_payload_limit(retained).unwrap(), 0);
        assert!(charge_source_payload_bytes(&mut retained, 1).is_err());
        assert!(remaining_source_payload_limit(retained + 1).is_err());
    }

    #[test]
    fn strict_schema_rejects_unknown_fields_versions_unsafe_paths_and_baselines() {
        let (_, _, manifest) = fixture();
        for level in [0, 1, 2] {
            let mut json = serde_json::to_value(&manifest).unwrap();
            let object = match level {
                0 => json.as_object_mut().unwrap(),
                1 => json["entries"][0].as_object_mut().unwrap(),
                _ => json["entries"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|entry| entry["op"] == "edit")
                    .unwrap()["original_module"]
                    .as_object_mut()
                    .unwrap(),
            };
            object.insert("future".into(), true.into());
            assert!(read_script_sources_manifest_v1(&serde_json::to_vec(&json).unwrap()).is_err());
        }
        let mut unknown_version = manifest.clone();
        unknown_version.schema_version = 2;
        assert!(unknown_version.validate().is_err());
        for unsafe_path in [
            "../escape.as",
            "C:/escape.as",
            "a\\b.as",
            "CON.as",
            "a/../b.as",
            "a/.as:stream",
        ] {
            let mut bad = manifest.clone();
            bad.entries[0].source = unsafe_path.into();
            assert!(bad.validate().is_err(), "{unsafe_path}");
        }
        let mut baseline_missing = manifest.clone();
        baseline_missing
            .entries
            .iter_mut()
            .find(|entry| entry.op == "edit")
            .unwrap()
            .original_module = None;
        assert!(baseline_missing.validate().is_err());
        let mut duplicate = manifest.clone();
        duplicate.entries.push(manifest.entries[0].clone());
        assert!(duplicate.validate().is_err());
        let mut oversized = manifest.clone();
        oversized.entries = vec![manifest.entries[0].clone(); MAX_SCRIPT_SOURCE_ENTRIES_V1 + 1];
        assert!(oversized.validate().is_err());
    }

    #[test]
    fn source_reader_rejects_oversized_non_utf8_and_nul_payloads_even_if_hashes_match() {
        let (files, scripts, manifest) = fixture();
        for bytes in [
            vec![0xff],
            vec![0],
            vec![b'x'; MAX_SCRIPT_SOURCE_BYTES_V1 as usize + 1],
        ] {
            let mut changed = files.clone();
            let mut metadata = manifest.clone();
            metadata.entries[0].source_sha256 = script_source_sha256_v1(&bytes);
            changed.insert(metadata.entries[0].source.clone(), bytes);
            assert!(
                validate_script_sources_v1(
                    &metadata,
                    &scripts,
                    |path, _| Ok(changed[path].clone())
                )
                .is_err()
            );
        }
    }

    #[test]
    fn baseline_fingerprint_ignores_build_guid_and_binary_only_groups_remain_explicit() {
        let (_, _, first) = fixture();
        let mut files = Files::from([("scripts/edit.cache".into(), cache(&["Vanilla"], 8))]);
        let scripts = vec![ScriptEntry {
            module: "Vanilla".into(),
            op: "edit".into(),
            mini: "scripts/edit.cache".into(),
        }];
        let second = package_script_sources_v1(
            &mut files,
            "scripts",
            &scripts,
            &cache(&["Vanilla"], 8),
            vec![input("Vanilla", "edit", "scripts/edit.cache")],
        )
        .unwrap();
        assert_ne!(first.base_cache_guid, second.base_cache_guid);
        assert_ne!(first.base_cache_sha256, second.base_cache_sha256);
        assert_eq!(
            first
                .entries
                .iter()
                .find(|entry| entry.op == "edit")
                .unwrap()
                .original_module,
            second.entries[0].original_module
        );
        let mut with_binary = scripts;
        with_binary.push(ScriptEntry {
            module: "Binary".into(),
            op: "add".into(),
            mini: "scripts/binary.cache".into(),
        });
        assert!(!second.covers_all_scripts(&with_binary));
        validate(&second, &with_binary, &files).unwrap();
    }

    #[test]
    fn packager_automatically_includes_compiler_provenance_and_never_reconstructs_binary_sources() {
        let temp = tempfile::tempdir().unwrap();
        let mini_path = temp.path().join("authored.cache");
        std::fs::write(&mini_path, cache(&["Vanilla", "New"], 1)).unwrap();
        write_script_source_provenance_v1(
            &mini_path,
            &cache(&["Vanilla"], 1),
            vec![input("Vanilla", "edit", ""), input("New", "add", "")],
        )
        .unwrap();
        let spec: crate::BuildSpec = serde_json::from_value(serde_json::json!({
            "meta": {"name": "Authored"},
            "scripts": [{"op": "edit", "module_name": "Vanilla", "mini_cache": "authored.cache"}],
        }))
        .unwrap();
        let bundle = crate::build_bundle_relative_to(&spec, temp.path()).unwrap();
        let scripts: Vec<ScriptEntry> =
            serde_json::from_slice(&bundle.files["scripts/manifest.json"]).unwrap();
        let manifest =
            read_script_sources_manifest_v1(&bundle.files["scripts/sources.json"]).unwrap();
        validate(&manifest, &scripts, &bundle.files).unwrap();
        assert_eq!(
            bundle.files[&manifest
                .entries
                .iter()
                .find(|entry| entry.module == "New")
                .unwrap()
                .source],
            input("New", "add", "").source
        );
        let destination = temp.path().join("bundle");
        crate::write_bundle(&destination, &bundle).unwrap();
        assert_eq!(
            load_script_sources_v1(&destination, "scripts", &scripts).unwrap(),
            Some(manifest)
        );
        assert!(write_script_source_provenance_v1(
            &mini_path,
            &cache(&["Vanilla"], 1),
            vec![input("Vanilla", "edit", ""), input("New", "add", "")]
        )
        .is_err());
        std::fs::remove_file(script_source_provenance_path_v1(&mini_path)).unwrap();
        let binary = crate::build_bundle_relative_to(&spec, temp.path()).unwrap();
        assert!(!binary.files.contains_key("scripts/sources.json"));
        std::fs::write(
            script_source_provenance_path_v1(&mini_path),
            b"{\"schema_version\":99}",
        )
        .unwrap();
        assert!(crate::build_bundle_relative_to(&spec, temp.path()).is_err());
    }

    #[test]
    fn compiler_provenance_round_trips_empty_and_uppercase_extension_sources() {
        for (relative_path, source) in [
            ("Vanilla.as", ""),
            ("Vanilla.AS", "// uppercase extension\n"),
            ("Vanilla.As", "// mixed case extension\n"),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let mini_path = temp.path().join("authored.cache");
            let base = cache_with_paths(&[("Vanilla", relative_path.into())], 1);
            std::fs::write(&mini_path, &base).unwrap();
            write_script_source_provenance_v1(
                &mini_path,
                &base,
                vec![ScriptSourceInputV1 {
                    relative_path: relative_path.into(),
                    source: source.as_bytes().to_vec(),
                    ..input("Vanilla", "edit", "")
                }],
            )
            .unwrap();
            let spec: crate::BuildSpec = serde_json::from_value(serde_json::json!({
                "meta": {"name": "Authored"},
                "scripts": [{"op": "edit", "module_name": "Vanilla", "mini_cache": "authored.cache"}],
            }))
            .unwrap();
            let bundle = crate::build_bundle_relative_to(&spec, temp.path()).unwrap();
            let scripts: Vec<ScriptEntry> =
                serde_json::from_slice(&bundle.files["scripts/manifest.json"]).unwrap();
            let manifest =
                read_script_sources_manifest_v1(&bundle.files["scripts/sources.json"]).unwrap();
            assert_eq!(manifest.entries[0].relative_path, relative_path);
            assert_eq!(bundle.files[&manifest.entries[0].source], source.as_bytes());
            let destination = temp.path().join("bundle");
            crate::write_bundle(&destination, &bundle).unwrap();
            assert_eq!(
                load_script_sources_v1(&destination, "scripts", &scripts).unwrap(),
                Some(manifest)
            );
        }
    }

    #[test]
    fn explicit_source_payload_accepts_uppercase_extension() {
        let (mut files, scripts, mut manifest) = fixture();
        let entry = &mut manifest.entries[0];
        let bytes = files.remove(&entry.source).unwrap();
        entry.source = "sources/Vanilla.AS".into();
        files.insert(entry.source.clone(), bytes);
        validate(&manifest, &scripts, &files).unwrap();
    }

    #[test]
    fn explicit_source_metadata_rebases_a_complete_mixed_mini_group() {
        let (source_files, original, manifest) = fixture();
        let temp = tempfile::tempdir().unwrap();
        for entry in &manifest.entries {
            let path = temp.path().join(&entry.source);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, &source_files[&entry.source]).unwrap();
        }
        let scripts = vec![ScriptEntry {
            mini: "rebased/0.cache".into(),
            ..original[0].clone()
        }];
        let mut files = Files::from([(
            "rebased/0.cache".into(),
            source_files["scripts/mixed.cache"].clone(),
        )]);
        package_explicit_script_sources_v1(
            &mut files,
            "rebased",
            &scripts,
            &original,
            temp.path(),
            &manifest,
        )
        .unwrap();
        let rebased = read_script_sources_manifest_v1(&files["rebased/sources.json"]).unwrap();
        validate(&rebased, &scripts, &files).unwrap();
        assert!(rebased
            .entries
            .iter()
            .all(|entry| entry.mini == "rebased/0.cache"
                && entry.source.starts_with("rebased/source/")));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_linked_source_payloads_and_linked_sidecars() {
        let (files, scripts, manifest) = fixture();
        let temp = tempfile::tempdir().unwrap();
        for (relative, bytes) in &files {
            let path = temp.path().join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
        let payload = temp.path().join(&manifest.entries[0].source);
        let elsewhere = temp.path().join("outside.as");
        std::fs::rename(&payload, &elsewhere).unwrap();
        std::os::unix::fs::symlink(&elsewhere, &payload).unwrap();
        assert!(load_script_sources_v1(temp.path(), "scripts", &scripts).is_err());
        std::fs::remove_file(&payload).unwrap();
        std::fs::rename(&elsewhere, &payload).unwrap();
        let sidecar = temp.path().join("scripts/sources.json");
        let elsewhere = temp.path().join("outside.json");
        std::fs::rename(&sidecar, &elsewhere).unwrap();
        std::os::unix::fs::symlink(&elsewhere, &sidecar).unwrap();
        assert!(load_script_sources_v1(temp.path(), "scripts", &scripts).is_err());
    }
}
