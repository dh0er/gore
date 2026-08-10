//! Read-only first-run facts for Mod Manager V1.
//!
//! This module deliberately reports evidence, not write authority. It never creates a probe file,
//! repairs a deploy record, reconciles import staging, or falls back to a configured/Steam install.
//! Paths in the result are bounded display strings only; callers must send the selected game root
//! back to the mutating command, which performs its own authoritative preflight.

use std::io::Read as _;
use std::path::{Path, PathBuf};

use gore_as::compile::{
    InstallCompileArtifactKind, InstallCompileGameProcessDisposition,
    InstallCompileInspectionIssueKind, InstallCompileStateDisposition, InstallCompileStateProbe,
};
use serde::Serialize;

use super::loadout::Loadout;
use super::model::{ComponentInfo, LibraryRoot, SecureDirectory, SecureNode};
use super::status::ManagerStatus;

const FORMAT: u32 = 1;
const MAX_LOADOUT_BYTES: u64 = 1024 * 1024;
const MAX_CHECK_ITEMS: usize = 16;
const MAX_DETAIL_BYTES: usize = 2 * 1024;
const MAX_ITEM_BYTES: usize = 1024;

/// One of the fixed Mod Manager V1 first-run checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflightCheckIdV1 {
    GameRoot,
    Install,
    Loadout,
    Deployment,
    InstallMutation,
    Ue4ss,
    WriteAccess,
}

/// A check's evidence state. `Unverified` is intentionally distinct from `Unknown`: no write
/// probe was attempted, whereas `Unknown` means a requested read could not establish the fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflightStateV1 {
    Ok,
    Problem,
    Unknown,
    NotRequired,
    Unverified,
}

/// One bounded first-run finding. `code` and `action` are closed native vocabulary; `detail` and
/// `items` are human-readable evidence and must never be interpreted as filesystem authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreflightCheckV1 {
    pub id: PreflightCheckIdV1,
    pub state: PreflightStateV1,
    pub code: &'static str,
    pub action: &'static str,
    pub detail: String,
    pub items: Vec<String>,
}

impl PreflightCheckV1 {
    fn new(
        id: PreflightCheckIdV1,
        state: PreflightStateV1,
        code: &'static str,
        action: &'static str,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            id,
            state,
            code,
            action,
            detail: bounded_text(&detail.into(), MAX_DETAIL_BYTES),
            items: Vec::new(),
        }
    }

    fn with_items(mut self, items: impl IntoIterator<Item = String>) -> Self {
        let mut source = items.into_iter();
        self.items = source
            .by_ref()
            .take(MAX_CHECK_ITEMS)
            .map(|item| bounded_text(&item, MAX_ITEM_BYTES))
            .collect();
        if source.next().is_some() {
            if self.items.len() == MAX_CHECK_ITEMS {
                self.items.pop();
            }
            self.items
                .push("additional evidence omitted by the native bound".to_owned());
        }
        self
    }
}

/// Fixed-order, bounded, read-only first-run evidence for Mod Manager V1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagerPreflightV1 {
    pub format: u32,
    pub checks: [PreflightCheckV1; 7],
}

struct RootInspection {
    check: PreflightCheckV1,
    root: Option<PathBuf>,
    directory: Option<SecureDirectory>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Wanted {
    File,
    Directory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Occupant {
    Wanted,
    Missing,
    Obstructed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ue4ssRequirement {
    Required,
    NotRequired,
    Unknown,
}

struct LoadoutInspection {
    check: PreflightCheckV1,
    loadout: Option<Loadout>,
    ue4ss: Ue4ssRequirement,
}

enum EvidenceFailure {
    Problem(String),
    Unknown(String),
}

impl EvidenceFailure {
    fn message(self) -> String {
        match self {
            Self::Problem(message) | Self::Unknown(message) => message,
        }
    }
}

/// Build the production snapshot. The selected `game_root` is always explicit; optional Manager
/// paths are resolved by the caller (the FFI uses the shared native defaults when omitted).
pub fn preflight_v1(
    game_root: &Path,
    library_dir: &Path,
    loadout_path: &Path,
) -> ManagerPreflightV1 {
    preflight_v1_with(
        game_root,
        library_dir,
        loadout_path,
        super::status::status,
        gore_as::compile::probe_install_compile_state,
        crate::deploy_recovery_required,
    )
}

/// The same read-only snapshot with the machine-global process answer supplied by a native
/// adapter test. Production callers must use [`preflight_v1`]; this mirrors gore-as's cross-crate
/// seam so an FFI test never depends on whether the developer happens to have the game open.
#[doc(hidden)]
pub fn preflight_v1_with_stated_game_process<C>(
    game_root: &Path,
    library_dir: &Path,
    loadout_path: &Path,
    check_game_process: C,
) -> ManagerPreflightV1
where
    C: FnOnce() -> Result<bool, String>,
{
    preflight_v1_with(
        game_root,
        library_dir,
        loadout_path,
        super::status::status,
        |root| {
            gore_as::compile::probe_install_compile_state_with_stated_game_process(
                root,
                check_game_process,
            )
        },
        crate::deploy_recovery_required,
    )
}

fn preflight_v1_with<S, P, R>(
    game_root: &Path,
    library_dir: &Path,
    loadout_path: &Path,
    status: S,
    install_state: P,
    deploy_recovery: R,
) -> ManagerPreflightV1
where
    S: FnOnce(&Path, &Path, &Loadout) -> crate::Result<ManagerStatus>,
    P: FnOnce(&Path) -> InstallCompileStateProbe,
    R: FnOnce(&Path) -> crate::Result<bool>,
{
    let root = inspect_game_root(game_root);
    let install = inspect_install(root.directory.as_ref());
    let loadout = inspect_loadout(library_dir, loadout_path);
    let loadout_healthy = loadout.check.state == PreflightStateV1::Ok;
    let (deployment, deployment_recovery_seen) = inspect_deployment(
        root.root.as_deref(),
        library_dir,
        loadout.loadout.as_ref(),
        loadout_healthy,
        status,
    );
    let install_mutation = inspect_install_mutation(
        root.root.as_deref(),
        deployment_recovery_seen,
        install_state,
        deploy_recovery,
    );
    let ue4ss = inspect_ue4ss(root.directory.as_ref(), loadout.ue4ss);
    let write_access = PreflightCheckV1::new(
        PreflightCheckIdV1::WriteAccess,
        PreflightStateV1::Unverified,
        "unverified_read_only",
        "verify_during_apply",
        "write access was not tested because this preflight is strictly read-only",
    );

    ManagerPreflightV1 {
        format: FORMAT,
        checks: [
            root.check,
            install,
            loadout.check,
            deployment,
            install_mutation,
            ue4ss,
            write_access,
        ],
    }
}

fn inspect_game_root(selected: &Path) -> RootInspection {
    if selected.as_os_str().is_empty() {
        return RootInspection {
            check: PreflightCheckV1::new(
                PreflightCheckIdV1::GameRoot,
                PreflightStateV1::Problem,
                "game_root_missing",
                "select_game_root",
                "no game root was selected",
            ),
            root: None,
            directory: None,
        };
    }
    let selected_display = display_path(selected);
    let canonical = match std::fs::canonicalize(selected) {
        Ok(path) => path,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return RootInspection {
                check: PreflightCheckV1::new(
                    PreflightCheckIdV1::GameRoot,
                    PreflightStateV1::Problem,
                    "game_root_not_found",
                    "select_game_root",
                    "the selected game root does not exist",
                )
                .with_items([format!("selected: {selected_display}")]),
                root: None,
                directory: None,
            }
        }
        Err(error) => {
            return RootInspection {
                check: PreflightCheckV1::new(
                    PreflightCheckIdV1::GameRoot,
                    PreflightStateV1::Unknown,
                    "game_root_inspection_failed",
                    "inspect_permissions",
                    "the selected game root could not be resolved",
                )
                .with_items([
                    format!("selected: {selected_display}"),
                    format!("inspection: {error}"),
                ]),
                root: None,
                directory: None,
            }
        }
    };
    match std::fs::symlink_metadata(&canonical) {
        Ok(metadata) if !metadata.is_dir() => {
            return RootInspection {
                check: PreflightCheckV1::new(
                    PreflightCheckIdV1::GameRoot,
                    PreflightStateV1::Problem,
                    "game_root_not_directory",
                    "select_game_root",
                    "the selected game root resolves to a non-directory node",
                )
                .with_items([format!("selected: {selected_display}")]),
                root: None,
                directory: None,
            }
        }
        Err(error) => {
            return RootInspection {
                check: PreflightCheckV1::new(
                    PreflightCheckIdV1::GameRoot,
                    PreflightStateV1::Unknown,
                    "game_root_inspection_failed",
                    "inspect_permissions",
                    "the selected game root target could not be inspected",
                )
                .with_items([
                    format!("selected: {selected_display}"),
                    format!("inspection: {error}"),
                ]),
                root: None,
                directory: None,
            }
        }
        Ok(_) => {}
    }
    let semantic = crate::semantic_install_root(&canonical);
    match super::model::open_directory_chain_nofollow(&semantic, "selected game root") {
        Ok(directory) => RootInspection {
            check: PreflightCheckV1::new(
                PreflightCheckIdV1::GameRoot,
                PreflightStateV1::Ok,
                "game_root_selected",
                "none",
                "the explicit game root resolves to a readable real directory",
            )
            .with_items([
                format!("selected: {selected_display}"),
                format!("resolved: {}", display_path(&semantic)),
            ]),
            root: Some(semantic),
            directory: Some(directory),
        },
        Err(error) => RootInspection {
            check: PreflightCheckV1::new(
                PreflightCheckIdV1::GameRoot,
                PreflightStateV1::Unknown,
                "game_root_inspection_failed",
                "inspect_permissions",
                "the resolved game root could not be safely opened",
            )
            .with_items([
                format!("selected: {selected_display}"),
                format!("inspection: {error}"),
            ]),
            root: None,
            directory: None,
        },
    }
}

fn inspect_install(root: Option<&SecureDirectory>) -> PreflightCheckV1 {
    let Some(root) = root else {
        return PreflightCheckV1::new(
            PreflightCheckIdV1::Install,
            PreflightStateV1::Unknown,
            "install_not_inspected",
            "select_game_root",
            "the install cannot be inspected until the game root is readable",
        );
    };
    let required: [(&str, &[&str], Wanted); 4] = [
        (
            r"G1R\Binaries\Win64\G1R-Win64-Shipping.exe",
            &["G1R", "Binaries", "Win64", "G1R-Win64-Shipping.exe"],
            Wanted::File,
        ),
        (
            r"G1R\Content\Paks",
            &["G1R", "Content", "Paks"],
            Wanted::Directory,
        ),
        (
            r"G1R\Story\Cache",
            &["G1R", "Story", "Cache"],
            Wanted::Directory,
        ),
        (r"G1R\Script", &["G1R", "Script"], Wanted::Directory),
    ];
    let mut missing = Vec::new();
    let mut obstructed = Vec::new();
    let mut failures = Vec::new();
    for (label, components, wanted) in required {
        match inspect_relative(root, components, wanted) {
            Ok(Occupant::Wanted) => {}
            Ok(Occupant::Missing) => missing.push(label.to_owned()),
            Ok(Occupant::Obstructed) => obstructed.push(label.to_owned()),
            Err(error) => failures.push(format!("{label}: {error}")),
        }
    }
    if !failures.is_empty() {
        return PreflightCheckV1::new(
            PreflightCheckIdV1::Install,
            PreflightStateV1::Unknown,
            "install_inspection_failed",
            "inspect_permissions",
            "one or more identifying install paths could not be safely inspected",
        )
        .with_items(failures);
    }
    if !obstructed.is_empty() {
        return PreflightCheckV1::new(
            PreflightCheckIdV1::Install,
            PreflightStateV1::Problem,
            "install_paths_obstructed",
            "remove_obstruction",
            "one or more install paths are occupied by the wrong kind of node",
        )
        .with_items(obstructed);
    }
    if missing.is_empty() {
        return PreflightCheckV1::new(
            PreflightCheckIdV1::Install,
            PreflightStateV1::Ok,
            "install_recognized",
            "none",
            "the executable, Paks, Story\\Cache, and Script paths are present",
        );
    }
    let executable_missing = missing.iter().any(|item| item.ends_with(".exe"));
    PreflightCheckV1::new(
        PreflightCheckIdV1::Install,
        PreflightStateV1::Problem,
        if executable_missing {
            "install_not_recognized"
        } else {
            "install_incomplete"
        },
        if executable_missing {
            "select_game_root"
        } else {
            "verify_game_files"
        },
        if executable_missing {
            "the selected root does not identify a Gothic 1 Remake installation"
        } else {
            "the installation is missing paths required by current GORE operations"
        },
    )
    .with_items(missing)
}

fn inspect_relative(
    root: &SecureDirectory,
    components: &[&str],
    wanted: Wanted,
) -> Result<Occupant, String> {
    let mut directory = root.clone();
    for (index, component) in components.iter().enumerate() {
        let name = std::ffi::OsStr::new(component);
        let child_path = directory.path().join(name);
        match std::fs::symlink_metadata(&child_path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Occupant::Missing)
            }
            Err(error) => {
                return Err(format!(
                    "inspecting preflight path {}: {error}",
                    display_path(&child_path)
                ))
            }
            Ok(metadata) if super::model::metadata_is_link(&metadata) => {
                return Err(format!(
                    "preflight path is a symbolic link or reparse point: {}",
                    display_path(&child_path)
                ))
            }
            Ok(_) => {}
        }
        let final_component = index + 1 == components.len();
        match directory
            .open_child(name, "preflight path")
            .map_err(|error| error.to_string())?
        {
            SecureNode::File(_) if final_component && wanted == Wanted::File => {
                return Ok(Occupant::Wanted)
            }
            SecureNode::Directory(_) if final_component && wanted == Wanted::Directory => {
                return Ok(Occupant::Wanted)
            }
            SecureNode::Directory(child) if !final_component => directory = child,
            SecureNode::File(_) | SecureNode::Directory(_) => return Ok(Occupant::Obstructed),
        }
    }
    Ok(Occupant::Missing)
}

fn inspect_loadout(library_dir: &Path, loadout_path: &Path) -> LoadoutInspection {
    let loadout = match read_loadout_bounded(loadout_path) {
        Ok(loadout) => loadout,
        Err(failure) => return loadout_failure(failure),
    };
    let enabled = loadout
        .entries
        .iter()
        .filter(|entry| entry.enabled)
        .collect::<Vec<_>>();
    if enabled.is_empty() {
        return LoadoutInspection {
            check: PreflightCheckV1::new(
                PreflightCheckIdV1::Loadout,
                PreflightStateV1::Ok,
                "loadout_empty",
                "none",
                "the loadout is valid and has no enabled mods",
            ),
            loadout: Some(loadout),
            ue4ss: Ue4ssRequirement::NotRequired,
        };
    }

    let library = match inspect_library_root(library_dir) {
        Ok(library) => library,
        Err(failure) => {
            let (state, code, action, detail) = match &failure {
                EvidenceFailure::Problem(_) => (
                    PreflightStateV1::Problem,
                    "enabled_library_unavailable",
                    "repair_library",
                    "enabled mods exist but the Manager library is missing or obstructed",
                ),
                EvidenceFailure::Unknown(_) => (
                    PreflightStateV1::Unknown,
                    "enabled_library_inspection_failed",
                    "inspect_permissions",
                    "enabled mods exist but the Manager library could not be safely inspected",
                ),
            };
            return LoadoutInspection {
                check: PreflightCheckV1::new(
                    PreflightCheckIdV1::Loadout,
                    state,
                    code,
                    action,
                    detail,
                )
                .with_items([failure.message()]),
                loadout: Some(loadout),
                ue4ss: Ue4ssRequirement::Unknown,
            };
        }
    };
    let mut problems = Vec::new();
    let mut unknowns = Vec::new();
    let mut requires_ue4ss = false;
    for entry in &enabled {
        match inspect_enabled_meta(&library, library_dir, &entry.id) {
            Ok(meta) => {
                requires_ue4ss |= meta
                    .components
                    .iter()
                    .any(|component| matches!(component, ComponentInfo::Ue4ssLua { .. }));
            }
            Err(error) => match error {
                EvidenceFailure::Problem(message) => {
                    push_bounded_evidence(&mut problems, format!("{}: {message}", entry.id));
                }
                EvidenceFailure::Unknown(message) => {
                    push_bounded_evidence(&mut unknowns, format!("{}: {message}", entry.id));
                }
            },
        }
    }
    if !unknowns.is_empty() {
        unknowns.extend(problems);
        return LoadoutInspection {
            check: PreflightCheckV1::new(
                PreflightCheckIdV1::Loadout,
                PreflightStateV1::Unknown,
                "enabled_mod_inspection_failed",
                "inspect_permissions",
                "one or more enabled metadata entries could not be safely inspected",
            )
            .with_items(unknowns),
            loadout: Some(loadout),
            ue4ss: if requires_ue4ss {
                Ue4ssRequirement::Required
            } else {
                Ue4ssRequirement::Unknown
            },
        };
    }
    if !problems.is_empty() {
        return LoadoutInspection {
            check: PreflightCheckV1::new(
                PreflightCheckIdV1::Loadout,
                PreflightStateV1::Problem,
                "enabled_mods_unreadable",
                "repair_library",
                "one or more enabled mods do not have readable validated metadata",
            )
            .with_items(problems),
            loadout: Some(loadout),
            ue4ss: if requires_ue4ss {
                Ue4ssRequirement::Required
            } else {
                Ue4ssRequirement::Unknown
            },
        };
    }
    LoadoutInspection {
        check: PreflightCheckV1::new(
            PreflightCheckIdV1::Loadout,
            PreflightStateV1::Ok,
            "loadout_valid",
            "none",
            format!(
                "the loadout and {} enabled mod metadata entries are valid",
                enabled.len()
            ),
        ),
        loadout: Some(loadout),
        ue4ss: if requires_ue4ss {
            Ue4ssRequirement::Required
        } else {
            Ue4ssRequirement::NotRequired
        },
    }
}

fn inspect_enabled_meta(
    library: &LibraryRoot,
    library_dir: &Path,
    id: &str,
) -> Result<super::model::ModEntryMeta, EvidenceFailure> {
    let entry_path = library_dir.join(id);
    match std::fs::symlink_metadata(&entry_path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(EvidenceFailure::Problem(format!(
                "enabled library entry is missing: {}",
                display_path(&entry_path)
            )))
        }
        Err(error) => {
            return Err(EvidenceFailure::Unknown(format!(
                "inspecting enabled library entry {}: {error}",
                display_path(&entry_path)
            )))
        }
        Ok(metadata) if super::model::metadata_is_link(&metadata) || !metadata.is_dir() => {
            return Err(EvidenceFailure::Problem(format!(
                "enabled library entry is not a real directory: {}",
                display_path(&entry_path)
            )))
        }
        Ok(_) => {}
    }
    let sidecar = entry_path.join(super::model::META_FILE);
    match std::fs::symlink_metadata(&sidecar) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(EvidenceFailure::Problem(format!(
                "enabled library sidecar is missing: {}",
                display_path(&sidecar)
            )))
        }
        Err(error) => {
            return Err(EvidenceFailure::Unknown(format!(
                "inspecting enabled library sidecar {}: {error}",
                display_path(&sidecar)
            )))
        }
        Ok(metadata) if super::model::metadata_is_link(&metadata) || !metadata.is_file() => {
            return Err(EvidenceFailure::Problem(format!(
                "enabled library sidecar is not a real file: {}",
                display_path(&sidecar)
            )))
        }
        Ok(_) => {}
    }
    library
        .entry(id)
        .and_then(|library_entry| library_entry.read_meta())
        .map_err(classify_mod_error)
}

fn loadout_failure(failure: EvidenceFailure) -> LoadoutInspection {
    let (state, code, action, detail) = match &failure {
        EvidenceFailure::Problem(_) => (
            PreflightStateV1::Problem,
            "loadout_unreadable",
            "repair_loadout",
            "the Manager loadout is invalid, obstructed, or exceeds its bound",
        ),
        EvidenceFailure::Unknown(_) => (
            PreflightStateV1::Unknown,
            "loadout_inspection_failed",
            "inspect_permissions",
            "the Manager loadout could not be safely inspected",
        ),
    };
    LoadoutInspection {
        check: PreflightCheckV1::new(PreflightCheckIdV1::Loadout, state, code, action, detail)
            .with_items([failure.message()]),
        loadout: None,
        ue4ss: Ue4ssRequirement::Unknown,
    }
}

fn inspect_library_root(path: &Path) -> Result<LibraryRoot, EvidenceFailure> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(EvidenceFailure::Problem(format!(
                "manager library does not exist: {}",
                display_path(path)
            )))
        }
        Err(error) => {
            return Err(EvidenceFailure::Unknown(format!(
                "inspecting manager library {}: {error}",
                display_path(path)
            )))
        }
        Ok(metadata) if !metadata.is_dir() && !super::model::metadata_is_link(&metadata) => {
            return Err(EvidenceFailure::Problem(format!(
                "manager library is not a directory: {}",
                display_path(path)
            )))
        }
        Ok(_) => {}
    }
    let canonical = std::fs::canonicalize(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            EvidenceFailure::Problem(format!(
                "manager library target does not exist: {}",
                display_path(path)
            ))
        } else {
            EvidenceFailure::Unknown(format!(
                "resolving manager library {}: {error}",
                display_path(path)
            ))
        }
    })?;
    match std::fs::symlink_metadata(&canonical) {
        Ok(metadata) if !metadata.is_dir() => {
            return Err(EvidenceFailure::Problem(format!(
                "manager library resolves to a non-directory node: {}",
                display_path(&canonical)
            )))
        }
        Err(error) => {
            return Err(EvidenceFailure::Unknown(format!(
                "inspecting manager library target {}: {error}",
                display_path(&canonical)
            )))
        }
        Ok(_) => {}
    }
    LibraryRoot::open(path).map_err(|error| {
        EvidenceFailure::Unknown(format!(
            "opening manager library {}: {error}",
            display_path(path)
        ))
    })
}

fn classify_mod_error(error: crate::ModError) -> EvidenceFailure {
    match error {
        crate::ModError::Io(message) => EvidenceFailure::Unknown(format!("io: {message}")),
        other => EvidenceFailure::Problem(other.to_string()),
    }
}

fn push_bounded_evidence(items: &mut Vec<String>, item: String) {
    if items.len() <= MAX_CHECK_ITEMS {
        items.push(item);
    }
}

fn read_loadout_bounded(path: &Path) -> Result<Loadout, EvidenceFailure> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Loadout::default()),
        Err(error) => {
            return Err(EvidenceFailure::Unknown(format!(
                "inspecting loadout {}: {error}",
                display_path(path)
            )))
        }
        Ok(metadata) if super::model::metadata_is_link(&metadata) => {
            return Err(EvidenceFailure::Problem(format!(
                "loadout is a symbolic link or reparse point: {}",
                display_path(path)
            )))
        }
        Ok(metadata) if !metadata.is_file() => {
            return Err(EvidenceFailure::Problem(format!(
                "loadout is not a regular file: {}",
                display_path(path)
            )))
        }
        Ok(_) => {}
    }
    let mut file = super::model::open_file_nofollow(path, "manager loadout")
        .map_err(|error| EvidenceFailure::Unknown(error.to_string()))?;
    if file.len() > MAX_LOADOUT_BYTES {
        return Err(EvidenceFailure::Problem(format!(
            "loadout exceeds the {MAX_LOADOUT_BYTES}-byte limit: {}",
            display_path(path)
        )));
    }
    let expected = file.len();
    let capacity = usize::try_from(expected)
        .map_err(|_| EvidenceFailure::Problem("loadout is too large".to_owned()))?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(capacity).map_err(|_| {
        EvidenceFailure::Unknown("could not reserve bounded loadout bytes".to_owned())
    })?;
    file.file
        .by_ref()
        .take(MAX_LOADOUT_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| {
            EvidenceFailure::Unknown(format!("reading loadout {}: {error}", display_path(path)))
        })?;
    if bytes.len() as u64 > MAX_LOADOUT_BYTES {
        return Err(EvidenceFailure::Problem(format!(
            "loadout exceeds the {MAX_LOADOUT_BYTES}-byte limit: {}",
            display_path(path)
        )));
    }
    file.verify_len(expected, "manager loadout")
        .map_err(|error| EvidenceFailure::Unknown(error.to_string()))?;
    let parsed: Loadout = serde_json::from_slice(&bytes).map_err(|error| {
        EvidenceFailure::Problem(format!("parsing loadout {}: {error}", display_path(path)))
    })?;
    if parsed.format > FORMAT {
        return Err(EvidenceFailure::Problem(format!(
            "loadout format {} is newer than this tool supports",
            parsed.format
        )));
    }
    parsed
        .validate()
        .map_err(|error| EvidenceFailure::Problem(error.to_string()))?;
    Ok(parsed)
}

fn inspect_deployment<S>(
    game_root: Option<&Path>,
    library_dir: &Path,
    loadout: Option<&Loadout>,
    loadout_healthy: bool,
    status: S,
) -> (PreflightCheckV1, bool)
where
    S: FnOnce(&Path, &Path, &Loadout) -> crate::Result<ManagerStatus>,
{
    let Some(game_root) = game_root else {
        return (
            PreflightCheckV1::new(
                PreflightCheckIdV1::Deployment,
                PreflightStateV1::Unknown,
                "deployment_not_inspected",
                "repair_preflight_inputs",
                "deployment state needs a readable game root and valid loadout",
            ),
            false,
        );
    };
    // Status resolves the deploy-record states that do not depend on the target loadout before it
    // compares loadouts. Preserve those recovery/studio/drift findings even when the loadout is
    // unreadable, but never project an InSync/ChangesPending conclusion from the fallback target.
    let fallback_loadout = Loadout::default();
    let status = status(game_root, library_dir, loadout.unwrap_or(&fallback_loadout));
    let deployment_recovery_seen = matches!(&status, Ok(ManagerStatus::RecoveryRequired));
    let check = match status {
        Ok(ManagerStatus::NothingDeployed) => PreflightCheckV1::new(
            PreflightCheckIdV1::Deployment,
            PreflightStateV1::Ok,
            "deployment_none",
            "none",
            "no GORE deployment record is active",
        ),
        Ok(ManagerStatus::RecoveryRequired) => PreflightCheckV1::new(
            PreflightCheckIdV1::Deployment,
            PreflightStateV1::Problem,
            "deployment_recovery_required",
            "recover_deployment",
            "the active deployment record requires recovery before another apply",
        ),
        Ok(ManagerStatus::StudioDeployActive { mod_name }) => PreflightCheckV1::new(
            PreflightCheckIdV1::Deployment,
            PreflightStateV1::Problem,
            "studio_deployment_active",
            "remove_studio_deployment",
            "a Mod Studio deployment is active and Manager must not replace it",
        )
        .with_items([format!("active mod: {mod_name}")]),
        Ok(ManagerStatus::InSync { .. } | ManagerStatus::ChangesPending { .. })
            if !loadout_healthy =>
        {
            PreflightCheckV1::new(
                PreflightCheckIdV1::Deployment,
                PreflightStateV1::Unknown,
                "deployment_not_inspected",
                "resolve_loadout_check",
                "resolve the preceding Loadout finding before comparing deployment state",
            )
        }
        Ok(ManagerStatus::InSync { loadout }) => PreflightCheckV1::new(
            PreflightCheckIdV1::Deployment,
            PreflightStateV1::Ok,
            "deployment_in_sync",
            "none",
            "the active Manager deployment matches the selected loadout",
        )
        .with_items(
            loadout
                .into_iter()
                .map(|entry| format!("enabled: {}", entry.id)),
        ),
        Ok(ManagerStatus::ChangesPending { deployed, target }) => {
            const SAMPLE_PER_SIDE: usize = 6;
            let deployed_count = deployed.len();
            let target_count = target.len();
            let mut items = vec![
                format!("deployed count: {deployed_count}"),
                format!("target count: {target_count}"),
            ];
            items.extend(
                deployed
                    .into_iter()
                    .take(SAMPLE_PER_SIDE)
                    .map(|entry| format!("deployed: {}", entry.id)),
            );
            items.extend(
                target
                    .into_iter()
                    .take(SAMPLE_PER_SIDE)
                    .map(|entry| format!("target: {}", entry.id)),
            );
            if deployed_count > SAMPLE_PER_SIDE || target_count > SAMPLE_PER_SIDE {
                items.push("additional deployed/target entries omitted by the native bound".into());
            }
            PreflightCheckV1::new(
                PreflightCheckIdV1::Deployment,
                PreflightStateV1::Problem,
                "deployment_changes_pending",
                "apply_loadout",
                "the selected loadout differs from the active Manager deployment",
            )
            .with_items(items)
        }
        Ok(ManagerStatus::GameUpdated { drifted }) => PreflightCheckV1::new(
            PreflightCheckIdV1::Deployment,
            PreflightStateV1::Problem,
            "deployment_game_updated",
            "reapply_after_update",
            "files owned by the active Manager deployment changed outside GORE",
        )
        .with_items(drifted.into_iter().map(|path| format!("drifted: {path}"))),
        Err(error) => PreflightCheckV1::new(
            PreflightCheckIdV1::Deployment,
            PreflightStateV1::Unknown,
            "deployment_inspection_failed",
            "inspect_deployment",
            "the bounded deployment record/status inspection failed",
        )
        .with_items([error.to_string()]),
    };
    (check, deployment_recovery_seen)
}

fn inspect_install_mutation<P, R>(
    game_root: Option<&Path>,
    deployment_recovery_seen: bool,
    install_state: P,
    deploy_recovery: R,
) -> PreflightCheckV1
where
    P: FnOnce(&Path) -> InstallCompileStateProbe,
    R: FnOnce(&Path) -> crate::Result<bool>,
{
    let Some(game_root) = game_root else {
        return PreflightCheckV1::new(
            PreflightCheckIdV1::InstallMutation,
            PreflightStateV1::Unknown,
            "install_mutation_not_inspected",
            "select_game_root",
            "process, lock, and recovery state need a readable game root",
        );
    };
    let probe = install_state(game_root);
    let recovery = deploy_recovery(game_root);
    let mut items = Vec::new();
    items.extend(probe.artifacts.iter().map(|artifact| {
        format!(
            "{}: {}{}",
            artifact_kind_label(artifact.kind),
            artifact.path,
            if artifact.path_truncated {
                " [truncated]"
            } else {
                ""
            }
        )
    }));
    items.extend(probe.issues.iter().map(|issue| {
        let path = issue
            .path
            .as_deref()
            .map(|path| {
                format!(
                    " at {path}{}",
                    if issue.path_truncated {
                        " [truncated]"
                    } else {
                        ""
                    }
                )
            })
            .unwrap_or_default();
        format!(
            "{}{path}: {}{}",
            issue_kind_label(issue.kind),
            issue.message,
            if issue.message_truncated {
                " [truncated]"
            } else {
                ""
            }
        )
    }));
    if deployment_recovery_seen {
        items.push("deployment status: recovery required".to_owned());
    }
    if recovery.as_ref().is_ok_and(|required| *required) {
        items.push("deploy record: recovery required".to_owned());
    }
    if let Err(error) = &recovery {
        items.push(format!("deploy recovery inspection: {error}"));
    }

    if probe.disposition == InstallCompileStateDisposition::InspectionFailed
        || probe.game_process == InstallCompileGameProcessDisposition::InspectionFailed
        || !probe.issues.is_empty()
        || recovery.is_err()
    {
        return PreflightCheckV1::new(
            PreflightCheckIdV1::InstallMutation,
            PreflightStateV1::Unknown,
            "install_mutation_inspection_failed",
            "inspect_permissions",
            "process or recovery state could not be established safely",
        )
        .with_items(items);
    }
    if probe.game_process == InstallCompileGameProcessDisposition::Running
        || probe.disposition == InstallCompileStateDisposition::GameProcessRunning
    {
        return PreflightCheckV1::new(
            PreflightCheckIdV1::InstallMutation,
            PreflightStateV1::Problem,
            "game_process_running",
            "close_game",
            "the shipping game process is running",
        )
        .with_items(items);
    }
    if deployment_recovery_seen
        || recovery.is_ok_and(|required| required)
        || probe.disposition == InstallCompileStateDisposition::RecoveryArtifactsPresent
        || !probe.artifacts.is_empty()
    {
        return PreflightCheckV1::new(
            PreflightCheckIdV1::InstallMutation,
            PreflightStateV1::Problem,
            "install_recovery_required",
            "recover_install",
            "a lock, journal, backup, or deploy recovery record blocks a new install mutation",
        )
        .with_items(items);
    }
    PreflightCheckV1::new(
        PreflightCheckIdV1::InstallMutation,
        PreflightStateV1::Ok,
        "install_mutation_clear",
        "none",
        "the game is closed and no known GORE recovery artifact is present",
    )
}

fn inspect_ue4ss(
    root: Option<&SecureDirectory>,
    requirement: Ue4ssRequirement,
) -> PreflightCheckV1 {
    match requirement {
        Ue4ssRequirement::NotRequired => {
            return PreflightCheckV1::new(
                PreflightCheckIdV1::Ue4ss,
                PreflightStateV1::NotRequired,
                "ue4ss_not_required",
                "none",
                "no readable enabled component requires UE4SS Lua",
            )
        }
        Ue4ssRequirement::Unknown => {
            return PreflightCheckV1::new(
                PreflightCheckIdV1::Ue4ss,
                PreflightStateV1::Unknown,
                "ue4ss_requirement_unknown",
                "resolve_loadout_check",
                "resolve the preceding Loadout finding before deciding whether UE4SS is required",
            )
        }
        Ue4ssRequirement::Required => {}
    }
    let Some(root) = root else {
        return PreflightCheckV1::new(
            PreflightCheckIdV1::Ue4ss,
            PreflightStateV1::Unknown,
            "ue4ss_not_inspected",
            "select_game_root",
            "UE4SS is required but the game root is not readable",
        );
    };
    let ue4ss = &["G1R", "Binaries", "Win64", "ue4ss"];
    let dll = &["G1R", "Binaries", "Win64", "ue4ss", "UE4SS.dll"];
    let mods = &["G1R", "Binaries", "Win64", "ue4ss", "Mods"];
    let proxy = &["G1R", "Binaries", "Win64", "dwmapi.dll"];
    let inspect = |components, wanted| inspect_relative(root, components, wanted);
    let ue4ss_state = inspect(ue4ss, Wanted::Directory);
    let dll_state = inspect(dll, Wanted::File);
    let mods_state = inspect(mods, Wanted::Directory);
    let proxy_state = inspect(proxy, Wanted::File);
    let mut failures = Vec::new();
    for (label, state) in [
        ("ue4ss", &ue4ss_state),
        ("UE4SS.dll", &dll_state),
        ("Mods", &mods_state),
        ("dwmapi.dll", &proxy_state),
    ] {
        if let Err(error) = state {
            failures.push(format!("{label}: {error}"));
        }
    }
    if !failures.is_empty() {
        return PreflightCheckV1::new(
            PreflightCheckIdV1::Ue4ss,
            PreflightStateV1::Unknown,
            "ue4ss_inspection_failed",
            "inspect_permissions",
            "required UE4SS paths could not be safely inspected",
        )
        .with_items(failures);
    }
    if matches!(ue4ss_state, Ok(Occupant::Obstructed))
        || matches!(dll_state, Ok(Occupant::Obstructed))
    {
        return PreflightCheckV1::new(
            PreflightCheckIdV1::Ue4ss,
            PreflightStateV1::Problem,
            "ue4ss_paths_obstructed",
            "remove_obstruction",
            "a required UE4SS directory or DLL path is occupied by the wrong kind of node",
        );
    }
    if !matches!(ue4ss_state, Ok(Occupant::Wanted)) || !matches!(dll_state, Ok(Occupant::Wanted)) {
        return PreflightCheckV1::new(
            PreflightCheckIdV1::Ue4ss,
            PreflightStateV1::Problem,
            "ue4ss_dll_missing",
            "install_ue4ss",
            "an enabled Lua component requires UE4SS, but UE4SS.dll is not ready",
        );
    }
    if matches!(mods_state, Ok(Occupant::Obstructed)) {
        return PreflightCheckV1::new(
            PreflightCheckIdV1::Ue4ss,
            PreflightStateV1::Problem,
            "ue4ss_mods_obstructed",
            "remove_obstruction",
            "UE4SS Mods is occupied by a non-directory node",
        );
    }
    let mut items = Vec::new();
    if matches!(mods_state, Ok(Occupant::Missing)) {
        items.push("Mods: missing; a later apply may create it".to_owned());
    }
    if !matches!(proxy_state, Ok(Occupant::Wanted)) {
        items.push(
            "dwmapi.dll: not proven; UE4SS may use a supported alternate proxy name".to_owned(),
        );
        return PreflightCheckV1::new(
            PreflightCheckIdV1::Ue4ss,
            PreflightStateV1::Ok,
            "ue4ss_proxy_unverified",
            "verify_ue4ss_proxy",
            "UE4SS.dll is present; the known loader proxy is absent or obstructed",
        )
        .with_items(items);
    }
    PreflightCheckV1::new(
        PreflightCheckIdV1::Ue4ss,
        PreflightStateV1::Ok,
        "ue4ss_ready",
        "none",
        "UE4SS.dll and the known loader proxy are present",
    )
    .with_items(items)
}

fn artifact_kind_label(kind: InstallCompileArtifactKind) -> &'static str {
    match kind {
        InstallCompileArtifactKind::InstallMutationLock => "install_mutation_lock",
        InstallCompileArtifactKind::CompileLock => "compile_lock",
        InstallCompileArtifactKind::RecoveryJournal => "recovery_journal",
        InstallCompileArtifactKind::ShippingCacheBackup => "shipping_cache_backup",
        InstallCompileArtifactKind::JittedCodeBackup => "jitted_code_backup",
        InstallCompileArtifactKind::Ue4ssProxyBackup => "ue4ss_proxy_backup",
    }
}

fn issue_kind_label(kind: InstallCompileInspectionIssueKind) -> &'static str {
    match kind {
        InstallCompileInspectionIssueKind::GameProcessEnumeration => "game_process_enumeration",
        InstallCompileInspectionIssueKind::ArtifactMetadata => "artifact_metadata",
    }
}

fn display_path(path: &Path) -> String {
    bounded_text(&path.as_os_str().to_string_lossy(), MAX_ITEM_BYTES / 2)
}

fn bounded_text(value: &str, limit: usize) -> String {
    if value.len() <= limit {
        return value.to_owned();
    }
    let mut end = limit;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    let suffix = " [truncated]";
    let keep = end.saturating_sub(suffix.len());
    let mut keep = keep;
    while keep > 0 && !value.is_char_boundary(keep) {
        keep -= 1;
    }
    format!("{}{}", &value[..keep], suffix)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use gore_as::compile::{InstallCompileArtifact, InstallCompileInspectionIssue};

    use super::*;
    use crate::mgr::{LoadoutEntry, ModEntryMeta, ModKind};

    fn install_fixture() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        for directory in [
            "G1R/Binaries/Win64",
            "G1R/Content/Paks",
            "G1R/Story/Cache",
            "G1R/Script",
        ] {
            fs::create_dir_all(root.path().join(directory)).unwrap();
        }
        fs::write(
            root.path()
                .join("G1R/Binaries/Win64/G1R-Win64-Shipping.exe"),
            b"exe",
        )
        .unwrap();
        root
    }

    fn safe_probe() -> InstallCompileStateProbe {
        InstallCompileStateProbe {
            disposition: InstallCompileStateDisposition::SafeToCompile,
            safe_to_compile: true,
            game_process: InstallCompileGameProcessDisposition::NotRunning,
            artifacts: Vec::new(),
            issues: Vec::new(),
        }
    }

    fn run_with<S, P, R>(
        root: &Path,
        library: &Path,
        loadout: &Path,
        status: S,
        probe: P,
        recovery: R,
    ) -> ManagerPreflightV1
    where
        S: FnOnce(&Path, &Path, &Loadout) -> crate::Result<ManagerStatus>,
        P: FnOnce(&Path) -> InstallCompileStateProbe,
        R: FnOnce(&Path) -> crate::Result<bool>,
    {
        preflight_v1_with(root, library, loadout, status, probe, recovery)
    }

    #[test]
    fn fixed_order_and_read_only_write_access_are_stable() {
        let install = install_fixture();
        let state = run_with(
            install.path(),
            &install.path().join("library"),
            &install.path().join("loadout.json"),
            |_, _, _| Ok(ManagerStatus::NothingDeployed),
            |_| safe_probe(),
            |_| Ok(false),
        );
        assert_eq!(state.format, 1);
        assert_eq!(
            state.checks.each_ref().map(|check| check.id),
            [
                PreflightCheckIdV1::GameRoot,
                PreflightCheckIdV1::Install,
                PreflightCheckIdV1::Loadout,
                PreflightCheckIdV1::Deployment,
                PreflightCheckIdV1::InstallMutation,
                PreflightCheckIdV1::Ue4ss,
                PreflightCheckIdV1::WriteAccess,
            ]
        );
        let write = &state.checks[6];
        assert_eq!(write.state, PreflightStateV1::Unverified);
        assert_eq!(write.code, "unverified_read_only");
    }

    #[test]
    fn install_distinguishes_missing_executable_and_incomplete_tree() {
        let install = install_fixture();
        fs::remove_file(
            install
                .path()
                .join("G1R/Binaries/Win64/G1R-Win64-Shipping.exe"),
        )
        .unwrap();
        let root = inspect_game_root(install.path());
        assert_eq!(
            inspect_install(root.directory.as_ref()).code,
            "install_not_recognized"
        );

        fs::write(
            install
                .path()
                .join("G1R/Binaries/Win64/G1R-Win64-Shipping.exe"),
            b"exe",
        )
        .unwrap();
        fs::remove_dir(install.path().join("G1R/Script")).unwrap();
        let root = inspect_game_root(install.path());
        assert_eq!(
            inspect_install(root.directory.as_ref()).code,
            "install_incomplete"
        );
    }

    #[test]
    fn a_selected_file_named_g1r_is_not_folded_into_a_valid_parent() {
        let root = tempfile::tempdir().unwrap();
        let selected = root.path().join("G1R");
        fs::write(&selected, b"not a directory").unwrap();

        let inspected = inspect_game_root(&selected);
        assert_eq!(inspected.check.state, PreflightStateV1::Problem);
        assert_eq!(inspected.check.code, "game_root_not_directory");
        assert!(inspected.root.is_none());
    }

    #[test]
    fn loadout_is_bounded_and_never_opens_library_when_empty() {
        let root = tempfile::tempdir().unwrap();
        let loadout = root.path().join("loadout.json");
        let oversized = fs::File::create(&loadout).unwrap();
        oversized.set_len(MAX_LOADOUT_BYTES + 1).unwrap();
        drop(oversized);
        assert_eq!(
            inspect_loadout(&root.path().join("missing-library"), &loadout)
                .check
                .code,
            "loadout_unreadable"
        );
        fs::remove_file(&loadout).unwrap();
        let empty = inspect_loadout(&root.path().join("missing-library"), &loadout);
        assert_eq!(empty.check.code, "loadout_empty");
        assert!(matches!(empty.ue4ss, Ue4ssRequirement::NotRequired));
    }

    fn write_library_meta(library: &Path, id: &str, components: Vec<ComponentInfo>) {
        let entry = library.join(id);
        fs::create_dir_all(&entry).unwrap();
        let meta = ModEntryMeta {
            id: id.to_owned(),
            kind: ModKind::Goremod,
            name: id.to_owned(),
            version: String::new(),
            author: String::new(),
            imported_at: "2026-08-10T00:00:00Z".to_owned(),
            source: String::new(),
            components,
        };
        fs::write(
            entry.join(super::super::model::META_FILE),
            serde_json::to_vec(&meta).unwrap(),
        )
        .unwrap();
    }

    fn write_enabled_loadout(path: &Path, ids: &[&str]) {
        fs::write(
            path,
            serde_json::to_vec(&Loadout {
                format: 1,
                entries: ids
                    .iter()
                    .map(|id| LoadoutEntry {
                        id: (*id).to_owned(),
                        enabled: true,
                    })
                    .collect(),
            })
            .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn only_enabled_readable_lua_metadata_requires_ue4ss() {
        let root = tempfile::tempdir().unwrap();
        let library = root.path().join("library");
        fs::create_dir(&library).unwrap();
        write_library_meta(
            &library,
            "lua",
            vec![ComponentInfo::Ue4ssLua {
                name: "lua".to_owned(),
                rel: "main.lua".to_owned(),
                targets: Vec::new(),
                opaque: false,
            }],
        );
        write_library_meta(&library, "plain", Vec::new());
        let loadout = root.path().join("loadout.json");
        write_enabled_loadout(&loadout, &["plain"]);
        assert!(matches!(
            inspect_loadout(&library, &loadout).ue4ss,
            Ue4ssRequirement::NotRequired
        ));
        write_enabled_loadout(&loadout, &["lua"]);
        assert!(matches!(
            inspect_loadout(&library, &loadout).ue4ss,
            Ue4ssRequirement::Required
        ));
        write_enabled_loadout(&loadout, &["missing"]);
        let missing = inspect_loadout(&library, &loadout);
        assert_eq!(missing.check.state, PreflightStateV1::Problem);
        assert_eq!(missing.check.code, "enabled_mods_unreadable");
        assert!(matches!(missing.ue4ss, Ue4ssRequirement::Unknown));

        write_enabled_loadout(&loadout, &["lua", "missing"]);
        let partial = inspect_loadout(&library, &loadout);
        assert_eq!(partial.check.state, PreflightStateV1::Problem);
        assert!(matches!(partial.ue4ss, Ue4ssRequirement::Required));
    }

    #[test]
    fn evidence_failures_distinguish_repairs_from_uninspectable_io() {
        let problem = loadout_failure(EvidenceFailure::Problem("corrupt".to_owned()));
        assert_eq!(problem.check.state, PreflightStateV1::Problem);
        assert_eq!(problem.check.code, "loadout_unreadable");
        assert_eq!(problem.check.action, "repair_loadout");

        let unknown = loadout_failure(EvidenceFailure::Unknown("denied".to_owned()));
        assert_eq!(unknown.check.state, PreflightStateV1::Unknown);
        assert_eq!(unknown.check.code, "loadout_inspection_failed");
        assert_eq!(unknown.check.action, "inspect_permissions");
        assert_eq!(
            inspect_ue4ss(None, Ue4ssRequirement::Unknown).action,
            "resolve_loadout_check"
        );

        assert!(matches!(
            classify_mod_error(crate::ModError::Io("sharing violation".to_owned())),
            EvidenceFailure::Unknown(_)
        ));
        assert!(matches!(
            classify_mod_error(crate::ModError::Other("corrupt sidecar".to_owned())),
            EvidenceFailure::Problem(_)
        ));
    }

    #[test]
    fn ue4ss_requires_dll_but_not_mods_and_proxy_is_advisory() {
        let install = install_fixture();
        let root = inspect_game_root(install.path());
        assert_eq!(
            inspect_ue4ss(root.directory.as_ref(), Ue4ssRequirement::Required).code,
            "ue4ss_dll_missing"
        );
        let ue4ss = install.path().join("G1R/Binaries/Win64/ue4ss");
        fs::create_dir(&ue4ss).unwrap();
        fs::write(ue4ss.join("UE4SS.dll"), b"dll").unwrap();
        let root = inspect_game_root(install.path());
        let advisory = inspect_ue4ss(root.directory.as_ref(), Ue4ssRequirement::Required);
        assert_eq!(advisory.state, PreflightStateV1::Ok);
        assert_eq!(advisory.code, "ue4ss_proxy_unverified");
        assert!(advisory
            .items
            .iter()
            .any(|item| item.contains("Mods: missing")));
        fs::write(
            install.path().join("G1R/Binaries/Win64/dwmapi.dll"),
            b"proxy",
        )
        .unwrap();
        let root = inspect_game_root(install.path());
        assert_eq!(
            inspect_ue4ss(root.directory.as_ref(), Ue4ssRequirement::Required).code,
            "ue4ss_ready"
        );

        fs::remove_file(ue4ss.join("UE4SS.dll")).unwrap();
        fs::create_dir(ue4ss.join("UE4SS.dll")).unwrap();
        let root = inspect_game_root(install.path());
        let obstructed = inspect_ue4ss(root.directory.as_ref(), Ue4ssRequirement::Required);
        assert_eq!(obstructed.code, "ue4ss_paths_obstructed");
        assert_eq!(obstructed.action, "remove_obstruction");
    }

    #[test]
    fn status_failure_is_an_unknown_environmental_finding() {
        let install = install_fixture();
        let state = run_with(
            install.path(),
            &install.path().join("library"),
            &install.path().join("loadout.json"),
            |_, _, _| Err(crate::ModError::Other("status denied".to_owned())),
            |_| safe_probe(),
            |_| Ok(false),
        );
        assert_eq!(state.checks[3].state, PreflightStateV1::Unknown);
        assert_eq!(state.checks[3].code, "deployment_inspection_failed");
    }

    #[test]
    fn unreadable_loadout_preserves_loadout_independent_deployment_findings() {
        let install = install_fixture();
        let library = install.path().join("library");
        let loadout = install.path().join("loadout.json");
        fs::write(&loadout, b"{").unwrap();

        let recovery = run_with(
            install.path(),
            &library,
            &loadout,
            |_, _, target| {
                assert!(target.entries.is_empty());
                Ok(ManagerStatus::RecoveryRequired)
            },
            |_| safe_probe(),
            |_| Ok(false),
        );
        assert_eq!(recovery.checks[2].code, "loadout_unreadable");
        assert_eq!(recovery.checks[3].code, "deployment_recovery_required");
        assert_eq!(recovery.checks[4].code, "install_recovery_required");

        let independent = [
            (
                ManagerStatus::NothingDeployed,
                "deployment_none",
                PreflightStateV1::Ok,
            ),
            (
                ManagerStatus::StudioDeployActive {
                    mod_name: "studio".to_owned(),
                },
                "studio_deployment_active",
                PreflightStateV1::Problem,
            ),
            (
                ManagerStatus::GameUpdated {
                    drifted: vec!["changed.bin".to_owned()],
                },
                "deployment_game_updated",
                PreflightStateV1::Problem,
            ),
        ];
        for (status, code, state) in independent {
            let report = run_with(
                install.path(),
                &library,
                &loadout,
                move |_, _, _| Ok(status),
                |_| safe_probe(),
                |_| Ok(false),
            );
            assert_eq!(report.checks[3].code, code);
            assert_eq!(report.checks[3].state, state);
        }

        for status in [
            ManagerStatus::InSync {
                loadout: Vec::new(),
            },
            ManagerStatus::ChangesPending {
                deployed: Vec::new(),
                target: Vec::new(),
            },
        ] {
            let report = run_with(
                install.path(),
                &library,
                &loadout,
                move |_, _, _| Ok(status),
                |_| safe_probe(),
                |_| Ok(false),
            );
            assert_eq!(report.checks[3].state, PreflightStateV1::Unknown);
            assert_eq!(report.checks[3].code, "deployment_not_inspected");
        }
    }

    #[test]
    fn unhealthy_enabled_metadata_never_recommends_apply() {
        let install = install_fixture();
        let library = install.path().join("library");
        fs::create_dir(&library).unwrap();
        let loadout = install.path().join("loadout.json");
        write_enabled_loadout(&loadout, &["missing"]);

        let report = run_with(
            install.path(),
            &library,
            &loadout,
            |_, _, target| {
                Ok(ManagerStatus::ChangesPending {
                    deployed: Vec::new(),
                    target: target.entries.clone(),
                })
            },
            |_| safe_probe(),
            |_| Ok(false),
        );
        assert_eq!(report.checks[2].code, "enabled_mods_unreadable");
        assert_eq!(report.checks[3].state, PreflightStateV1::Unknown);
        assert_eq!(report.checks[3].code, "deployment_not_inspected");
        assert_eq!(report.checks[3].action, "resolve_loadout_check");
    }

    #[test]
    fn every_manager_status_maps_to_stable_bounded_deployment_vocabulary() {
        let entry = |id: &str| LoadoutEntry {
            id: id.to_owned(),
            enabled: true,
        };
        let cases = [
            (
                ManagerStatus::NothingDeployed,
                PreflightStateV1::Ok,
                "deployment_none",
                "none",
            ),
            (
                ManagerStatus::RecoveryRequired,
                PreflightStateV1::Problem,
                "deployment_recovery_required",
                "recover_deployment",
            ),
            (
                ManagerStatus::StudioDeployActive {
                    mod_name: "studio".to_owned(),
                },
                PreflightStateV1::Problem,
                "studio_deployment_active",
                "remove_studio_deployment",
            ),
            (
                ManagerStatus::InSync {
                    loadout: vec![entry("one")],
                },
                PreflightStateV1::Ok,
                "deployment_in_sync",
                "none",
            ),
            (
                ManagerStatus::ChangesPending {
                    deployed: (0..40)
                        .map(|index| entry(&format!("old-{index}")))
                        .collect(),
                    target: (0..40)
                        .map(|index| entry(&format!("new-{index}")))
                        .collect(),
                },
                PreflightStateV1::Problem,
                "deployment_changes_pending",
                "apply_loadout",
            ),
            (
                ManagerStatus::GameUpdated {
                    drifted: vec!["changed.bin".to_owned()],
                },
                PreflightStateV1::Problem,
                "deployment_game_updated",
                "reapply_after_update",
            ),
        ];
        for (status, state, code, action) in cases {
            let (check, recovery_seen) = inspect_deployment(
                Some(Path::new("C:/display-only-game")),
                Path::new("C:/display-only-library"),
                Some(&Loadout::default()),
                true,
                move |_, _, _| Ok(status),
            );
            assert_eq!(check.state, state, "code: {code}");
            assert_eq!(check.code, code);
            assert_eq!(check.action, action);
            assert!(check.items.len() <= MAX_CHECK_ITEMS);
            assert!(check.items.iter().all(|item| item.len() <= MAX_ITEM_BYTES));
            if code == "deployment_changes_pending" {
                assert!(check
                    .items
                    .iter()
                    .any(|item| item.starts_with("deployed: ")));
                assert!(check.items.iter().any(|item| item.starts_with("target: ")));
            }
            assert_eq!(recovery_seen, code == "deployment_recovery_required");
        }
    }

    #[test]
    fn process_recovery_and_inspection_failures_fail_closed() {
        let install = install_fixture();
        let running = InstallCompileStateProbe {
            disposition: InstallCompileStateDisposition::GameProcessRunning,
            safe_to_compile: false,
            game_process: InstallCompileGameProcessDisposition::Running,
            artifacts: Vec::new(),
            issues: Vec::new(),
        };
        let state = run_with(
            install.path(),
            &install.path().join("library"),
            &install.path().join("loadout.json"),
            |_, _, _| Ok(ManagerStatus::NothingDeployed),
            |_| running.clone(),
            |_| Ok(false),
        );
        assert_eq!(state.checks[4].code, "game_process_running");

        let state = run_with(
            install.path(),
            &install.path().join("library"),
            &install.path().join("loadout.json"),
            |_, _, _| Ok(ManagerStatus::NothingDeployed),
            |_| safe_probe(),
            |_| Ok(true),
        );
        assert_eq!(state.checks[4].code, "install_recovery_required");

        let state = run_with(
            install.path(),
            &install.path().join("library"),
            &install.path().join("loadout.json"),
            |_, _, _| Ok(ManagerStatus::RecoveryRequired),
            |_| safe_probe(),
            |_| Ok(false),
        );
        assert_eq!(state.checks[3].code, "deployment_recovery_required");
        assert_eq!(state.checks[4].code, "install_recovery_required");
        assert!(state.checks[4]
            .items
            .iter()
            .any(|item| item == "deployment status: recovery required"));

        let failed = InstallCompileStateProbe {
            disposition: InstallCompileStateDisposition::InspectionFailed,
            safe_to_compile: false,
            game_process: InstallCompileGameProcessDisposition::InspectionFailed,
            artifacts: vec![InstallCompileArtifact {
                kind: InstallCompileArtifactKind::CompileLock,
                path: "x".repeat(MAX_ITEM_BYTES * 2),
                path_truncated: false,
            }],
            issues: vec![InstallCompileInspectionIssue {
                kind: InstallCompileInspectionIssueKind::ArtifactMetadata,
                path: Some("C:/game/.gore-install-mutation.lock".to_owned()),
                path_truncated: true,
                message: "denied".repeat(MAX_ITEM_BYTES),
                message_truncated: false,
            }],
        };
        let state = run_with(
            install.path(),
            &install.path().join("library"),
            &install.path().join("loadout.json"),
            |_, _, _| Ok(ManagerStatus::NothingDeployed),
            |_| failed.clone(),
            |_| Err(crate::ModError::Other("recovery denied".repeat(1000))),
        );
        assert_eq!(state.checks[4].state, PreflightStateV1::Unknown);
        assert!(state.checks[4]
            .items
            .iter()
            .any(|item| item.contains("C:/game/.gore-install-mutation.lock [truncated]")));
        assert!(state.checks[4]
            .items
            .iter()
            .all(|item| item.len() <= MAX_ITEM_BYTES));
    }

    #[test]
    fn evidence_vectors_are_capped() {
        let check = PreflightCheckV1::new(
            PreflightCheckIdV1::Deployment,
            PreflightStateV1::Problem,
            "test",
            "none",
            "d".repeat(MAX_DETAIL_BYTES * 2),
        )
        .with_items((0..100).map(|_| "i".repeat(MAX_ITEM_BYTES * 2)));
        assert!(check.detail.len() <= MAX_DETAIL_BYTES);
        assert_eq!(check.items.len(), MAX_CHECK_ITEMS);
        assert!(check.items.iter().all(|item| item.len() <= MAX_ITEM_BYTES));
        assert_eq!(
            check.items.last().unwrap(),
            "additional evidence omitted by the native bound"
        );
    }

    #[test]
    fn preflight_never_reconciles_replacing_artifacts() {
        let install = install_fixture();
        let library = install.path().join("library");
        let replacing = library.join(".replacing-preserve-me");
        fs::create_dir_all(&replacing).unwrap();
        let evidence = replacing.join("bytes.bin");
        fs::write(&evidence, b"byte-identical-after-preflight").unwrap();
        let before = fs::read(&evidence).unwrap();

        let _ = run_with(
            install.path(),
            &library,
            &install.path().join("loadout.json"),
            |_, _, _| Ok(ManagerStatus::NothingDeployed),
            |_| safe_probe(),
            |_| Ok(false),
        );

        assert!(replacing.is_dir());
        assert_eq!(fs::read(&evidence).unwrap(), before);
    }
}
