//! Serialized Manager store snapshots.
//!
//! The loadout and library remain two explicit filesystem commits.  This module coordinates
//! every loadout read/modify/write with a crash-released operating-system lock and reconciles a
//! valid loadout against one strict, recovery-complete library snapshot.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use super::loadout::{self, Loadout, LoadoutEntry};
use super::model::{
    acquire_manager_root_locks, manager_process_lock, prepare_existing_manager_root_lock,
    validate_library_id, ManagerRootLock, ModEntryMeta,
};

const MAX_LOADOUT_BYTES: u64 = 1024 * 1024;
#[cfg(test)]
const STORE_LOCK_FILE: &str = super::model::MANAGER_ROOT_LOCK_FILE;
#[cfg(test)]
const STORE_LOCK_MARKER_ENV: &str = "GORE_TEST_STORE_LOCK_MARKER";
#[cfg(test)]
const STORE_LOCK_HOLD_MS_ENV: &str = "GORE_TEST_STORE_LOCK_HOLD_MS";

#[cfg(test)]
thread_local! {
    static LOAD_RACE_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
    #[cfg(unix)]
    static LOAD_OPEN_RACE_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
    static BOOTSTRAP_RACE_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
    #[cfg(unix)]
    static PARENT_LOCK_RACE_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
fn inject_load_race(hook: impl FnOnce() + 'static) {
    LOAD_RACE_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(all(test, unix))]
fn inject_load_open_race(hook: impl FnOnce() + 'static) {
    LOAD_OPEN_RACE_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(all(test, unix))]
fn run_load_open_race_hook() {
    LOAD_OPEN_RACE_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}

#[cfg(test)]
fn run_load_race_hook() {
    LOAD_RACE_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}

#[cfg(test)]
fn inject_bootstrap_race(hook: impl FnOnce() + 'static) {
    BOOTSTRAP_RACE_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(test)]
fn run_bootstrap_race_hook() {
    BOOTSTRAP_RACE_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}

#[cfg(not(test))]
fn run_bootstrap_race_hook() {}

#[cfg(all(test, unix))]
fn inject_parent_lock_race(hook: impl FnOnce() + 'static) {
    PARENT_LOCK_RACE_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(all(test, unix))]
fn run_parent_lock_race_hook() {
    PARENT_LOCK_RACE_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}

#[cfg(any(not(test), not(unix)))]
fn run_parent_lock_race_hook() {}

#[cfg(not(test))]
fn run_load_race_hook() {}

#[cfg(not(all(test, unix)))]
fn run_load_open_race_hook() {}

pub struct StoreSnapshot {
    _kernel: KernelStoreLock,
    loadout_path: PathBuf,
    library: super::import::StrictLibrarySnapshot,
    loadout: Loadout,
}

/// Doctor's advisory projection. It holds only the Library lane, never creates/acquires the Store
/// sentinel, reconciles in memory, and exposes the same target semantics as Manager status.
pub struct ReadOnlyStoreInspection {
    library: super::import::StrictLibrarySnapshot,
    loadout: Loadout,
}

/// Which input prevented Doctor from constructing an authoritative, read-only target.
///
/// Keeping the stages distinct matters because a strict Library refusal is not evidence that the
/// user's loadout is corrupt and must never produce advice to delete or repair that loadout.
#[derive(Debug, thiserror::Error)]
pub enum StoreInspectionError {
    #[error("manager library could not be inspected safely: {0}")]
    Library(#[source] crate::ModError),
    #[error("manager loadout could not be read: {0}")]
    Loadout(#[source] crate::ModError),
}

/// Failure after a read-only projection was built. Library stability is kept separate from the
/// ordinary deployment-status result so Doctor never blames the deploy record for a Library race.
#[derive(Debug, thiserror::Error)]
pub enum StoreInspectionStatusError {
    #[error("manager library changed during status inspection: {0}")]
    Library(#[source] crate::ModError),
    #[error("manager deployment status could not be inspected: {0}")]
    Status(#[source] crate::ModError),
}

impl StoreSnapshot {
    /// Lock the canonical loadout target, take one strict library snapshot, and repair only a
    /// valid loadout.  Corrupt/future loadouts are returned as errors without rewriting bytes.
    pub fn open(library_dir: &Path, loadout_path: &Path) -> crate::Result<Self> {
        let process = manager_process_lock();
        let (parent_path, canonical_loadout) = canonical_parent(loadout_path)?;
        let mut store_prepared =
            prepare_existing_manager_root_lock(&parent_path, "manager store parent", true)?;

        // A missing Library cannot be prepared for composite locking. Serialize bootstrap on the
        // Store root first, read the authoritative loadout there, and refuse any persisted intent
        // before creating an empty Library that reconciliation would otherwise erase.
        let mut bootstrapped_library = false;
        if !path_exists_no_follow(library_dir)? {
            let mut initial = acquire_manager_root_locks(vec![store_prepared])?;
            let initial_store = KernelStoreLock::new(initial.remove(0));
            let loadout = initial_store.load(&canonical_loadout)?;
            if !loadout.entries.is_empty() {
                return Err(crate::ModError::Other(format!(
                    "manager library is missing while the persisted loadout still contains {} entry or entries: {}",
                    loadout.entries.len(),
                    library_dir.display()
                )));
            }
            drop(initial_store);
            run_bootstrap_race_hook();
            if !path_exists_no_follow(library_dir)? {
                std::fs::create_dir_all(library_dir).map_err(crate::io(
                    "creating manager library during empty-store bootstrap",
                ))?;
                bootstrapped_library = true;
            }
            store_prepared =
                prepare_existing_manager_root_lock(&parent_path, "manager store parent", true)?;
        }

        let store_identity = store_prepared.identity();
        let library_prepared = super::import::StrictLibrarySnapshot::prepare(library_dir)?;
        let library_identity = library_prepared.identity();
        let mut roots = acquire_manager_root_locks(vec![store_prepared, library_prepared])?;
        let store_index = roots
            .iter()
            .position(|root| root.identity() == store_identity)
            .expect("acquired root set retains Store identity");
        let kernel = KernelStoreLock::new(roots.remove(store_index));
        let library_index = roots
            .iter()
            .position(|root| root.identity() == library_identity)
            .expect("acquired root set retains Library identity");
        let library_root = roots.remove(library_index);
        debug_assert!(roots.is_empty());
        run_parent_lock_race_hook();
        run_store_lock_acquired_hook()?;
        let library = super::import::StrictLibrarySnapshot::from_prelocked(process, library_root)?;
        let loadout = kernel.load(&canonical_loadout)?;
        if bootstrapped_library && !loadout.entries.is_empty() {
            return Err(crate::ModError::Other(
                "manager loadout gained persisted entries during missing-library bootstrap; the loadout was left untouched"
                    .into(),
            ));
        }
        let reconciled = reconcile(library.mods(), &loadout)?;
        if reconciled != loadout {
            kernel.save(&canonical_loadout, &reconciled)?;
        }
        Ok(Self {
            _kernel: kernel,
            loadout_path: canonical_loadout,
            library,
            loadout: reconciled,
        })
    }

    pub fn inspect_read_only(
        library_dir: &Path,
        loadout_path: &Path,
    ) -> Result<ReadOnlyStoreInspection, StoreInspectionError> {
        // Library is deliberately the only root here. Cooperative Store writers hold the same
        // universal Library-root lock through save, irrespective of composite physical order.
        let library = super::import::StrictLibrarySnapshot::inspect_read_only(library_dir)
            .map_err(StoreInspectionError::Library)?;
        let loadout =
            load_loadout_read_only(loadout_path).map_err(StoreInspectionError::Loadout)?;
        if library.is_missing() && !loadout.entries.is_empty() {
            // An absent Library has no existing kernel object Doctor can lock without creating an
            // artifact. The retained process guard excludes local writers; this stability check
            // catches a cooperative external writer that published the root while the bounded
            // loadout was read. Persisted intent is never reconciled to an empty target.
            library
                .verify_read_only_stable()
                .map_err(StoreInspectionError::Library)?;
            return Err(StoreInspectionError::Library(crate::ModError::Other(
                format!(
                    "manager library is missing while the persisted loadout still contains {} entry or entries: {}",
                    loadout.entries.len(),
                    library_dir.display()
                ),
            )));
        }
        let reconciled =
            reconcile(library.mods(), &loadout).map_err(StoreInspectionError::Library)?;
        library
            .verify_read_only_stable()
            .map_err(StoreInspectionError::Library)?;
        Ok(ReadOnlyStoreInspection {
            library,
            loadout: reconciled,
        })
    }

    pub fn mods(&self) -> &[ModEntryMeta] {
        self.library.mods()
    }

    pub fn loadout(&self) -> &Loadout {
        &self.loadout
    }

    pub fn analyze(&self) -> Vec<super::analyze::Conflict> {
        let refs = self.library.mods().iter().collect::<Vec<_>>();
        super::analyze::analyze(&refs, &self.loadout)
    }

    pub fn apply(&self, game_root: &Path) -> crate::Result<super::apply::ApplyReport> {
        super::apply::apply_loadout_after_store_snapshot(
            game_root,
            self.library.path(),
            &self.loadout,
        )
    }

    pub fn status(&self, game_root: &Path) -> crate::Result<super::status::ManagerStatus> {
        super::status::status(game_root, self.library.path(), &self.loadout)
    }

    /// The authoritative status plus bounded display-only Manager ownership evidence from the
    /// exact same deploy-record read. Existing Rust callers can keep using [`Self::status`].
    pub fn status_report(
        &self,
        game_root: &Path,
    ) -> crate::Result<super::status::ManagerStatusReport> {
        super::status::status_report(game_root, self.library.path(), &self.loadout)
    }

    /// Persist an explicit whole-loadout edit while the store is locked. Existing entry edits are
    /// last-writer-wins; concurrently published library ids are still appended disabled.
    pub fn replace_loadout(&mut self, replacement: Loadout) -> crate::Result<()> {
        replacement.validate()?;
        let reconciled = reconcile(self.library.mods(), &replacement)?;
        if reconciled != self.loadout {
            self._kernel.save(&self.loadout_path, &reconciled)?;
            self.loadout = reconciled;
        }
        Ok(())
    }

    pub fn update_loadout(
        &mut self,
        update: impl FnOnce(&mut Loadout) -> crate::Result<()>,
    ) -> crate::Result<()> {
        let mut next = self.loadout.clone();
        update(&mut next)?;
        self.replace_loadout(next)
    }
}

impl ReadOnlyStoreInspection {
    pub fn loadout(&self) -> &Loadout {
        &self.loadout
    }

    pub fn status(
        &self,
        game_root: &Path,
    ) -> Result<super::status::ManagerStatus, StoreInspectionStatusError> {
        let result = super::status::status(game_root, self.library.path(), &self.loadout)
            .map_err(StoreInspectionStatusError::Status);
        self.library
            .verify_read_only_stable()
            .map_err(StoreInspectionStatusError::Library)?;
        result
    }
}

fn id_key(id: &str) -> String {
    #[cfg(windows)]
    {
        // Mirror the conservative portable Windows name identity used by import. Windows maps
        // names through an uppercase table; lowercase misses aliases such as Greek final sigma.
        id.to_uppercase()
    }
    #[cfg(not(windows))]
    {
        id.to_owned()
    }
}

fn reconcile(mods: &[ModEntryMeta], loadout: &Loadout) -> crate::Result<Loadout> {
    let mut canonical = BTreeMap::<String, String>::new();
    for meta in mods {
        validate_library_id(&meta.id)?;
        let key = id_key(&meta.id);
        if canonical.insert(key, meta.id.clone()).is_some() {
            return Err(crate::ModError::Other(format!(
                "manager library contains ambiguous ids for {:?}",
                meta.id
            )));
        }
    }

    let mut seen = HashSet::<String>::new();
    let mut entries = Vec::with_capacity(canonical.len());
    for entry in &loadout.entries {
        let key = id_key(&entry.id);
        let Some(id) = canonical.get(&key) else {
            continue;
        };
        if seen.insert(key) {
            entries.push(LoadoutEntry {
                id: id.clone(),
                enabled: entry.enabled,
            });
        }
    }
    let mut missing = canonical
        .into_iter()
        .filter(|(key, _)| !seen.contains(key))
        .map(|(_, id)| id)
        .collect::<Vec<_>>();
    missing.sort();
    entries.extend(
        missing
            .into_iter()
            .map(|id| LoadoutEntry { id, enabled: false }),
    );
    let reconciled = Loadout {
        format: loadout.format,
        entries,
    };
    reconciled.validate()?;
    Ok(reconciled)
}

fn path_exists_no_follow(path: &Path) -> crate::Result<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(crate::io(&format!(
            "inspecting manager path {}",
            path.display()
        ))(error)),
    }
}

fn canonical_parent(path: &Path) -> crate::Result<(PathBuf, PathBuf)> {
    let file_name = path.file_name().ok_or_else(|| {
        crate::ModError::Other(format!("loadout path has no file name: {}", path.display()))
    })?;
    let parent = path.parent().ok_or_else(|| {
        crate::ModError::Other(format!("loadout path has no parent: {}", path.display()))
    })?;
    // A child path such as `loadout.json` has an empty parent component. Filesystem APIs do not
    // consistently treat that spelling as the current directory, so normalize it explicitly.
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    std::fs::create_dir_all(parent).map_err(crate::io("creating manager store directory"))?;
    let parent = std::fs::canonicalize(parent).map_err(crate::io(&format!(
        "resolving manager store directory {}",
        parent.display()
    )))?;
    // The configured parent may be an intentional alias, but the loadout itself is always opened
    // no-follow. Canonicalizing an existing final component here would silently bless a symlink.
    let canonical = parent.join(file_name);
    if canonical.file_name().is_some_and(|name| {
        #[cfg(windows)]
        {
            name.to_string_lossy()
                .eq_ignore_ascii_case(super::model::MANAGER_ROOT_LOCK_FILE)
        }
        #[cfg(not(windows))]
        {
            name == super::model::MANAGER_ROOT_LOCK_FILE
        }
    }) {
        return Err(crate::ModError::Other(format!(
            "loadout path collides with reserved manager store lock: {}",
            canonical.display()
        )));
    }
    let canonical_parent = canonical.parent().ok_or_else(|| {
        crate::ModError::Other(format!(
            "canonical loadout target has no parent: {}",
            canonical.display()
        ))
    })?;
    Ok((canonical_parent.to_path_buf(), canonical))
}

fn canonical_loadout_for_read(path: &Path) -> crate::Result<Option<PathBuf>> {
    let file_name = path.file_name().ok_or_else(|| {
        crate::ModError::Other(format!("loadout path has no file name: {}", path.display()))
    })?;
    let parent = path.parent().ok_or_else(|| {
        crate::ModError::Other(format!("loadout path has no parent: {}", path.display()))
    })?;
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    let parent = match std::fs::canonicalize(parent) {
        Ok(parent) => parent,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(crate::io(&format!(
                "resolving manager store directory {}",
                parent.display()
            ))(error))
        }
    };
    Ok(Some(parent.join(file_name)))
}

fn load_loadout_read_only(path: &Path) -> crate::Result<Loadout> {
    let Some(path) = canonical_loadout_for_read(path)? else {
        return Ok(Loadout::default());
    };
    match std::fs::symlink_metadata(&path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Loadout::default()),
        Err(error) => return Err(crate::io("inspecting manager loadout")(error)),
    }
    let mut file = super::model::open_file_nofollow(&path, "manager loadout")?;
    run_load_race_hook();
    let bytes = file.read_all_bounded("manager loadout", MAX_LOADOUT_BYTES)?;
    loadout::parse_bytes(&bytes)
}

#[cfg(test)]
fn run_store_lock_acquired_hook() -> crate::Result<()> {
    if let Some(marker) = std::env::var_os(STORE_LOCK_MARKER_ENV) {
        std::fs::write(marker, b"locked").map_err(crate::io("writing store-lock marker"))?;
    }
    if let Ok(raw) = std::env::var(STORE_LOCK_HOLD_MS_ENV) {
        let milliseconds = raw.parse::<u64>().map_err(|error| {
            crate::ModError::Other(format!("invalid store-lock hold duration {raw:?}: {error}"))
        })?;
        std::thread::sleep(std::time::Duration::from_millis(milliseconds));
    }
    Ok(())
}

#[cfg(not(test))]
fn run_store_lock_acquired_hook() -> crate::Result<()> {
    Ok(())
}

#[cfg(windows)]
#[derive(Debug)]
struct KernelStoreLock {
    root: ManagerRootLock,
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WindowsFileIdentity {
    volume: u64,
    id: [u8; 16],
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WindowsFileRevision {
    last_write_time: i64,
    change_time: i64,
}

#[cfg(windows)]
fn windows_file_identity(file: &std::fs::File, label: &str) -> crate::Result<WindowsFileIdentity> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FileIdInfo, GetFileInformationByHandleEx, FILE_ID_INFO,
    };
    let mut info = FILE_ID_INFO::default();
    let ok = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileIdInfo,
            (&mut info as *mut FILE_ID_INFO).cast(),
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    };
    if ok == 0 {
        return Err(crate::io(&format!("querying {label} identity"))(
            std::io::Error::last_os_error(),
        ));
    }
    Ok(WindowsFileIdentity {
        volume: info.VolumeSerialNumber,
        id: info.FileId.Identifier,
    })
}

#[cfg(windows)]
fn windows_file_revision(file: &std::fs::File, label: &str) -> crate::Result<WindowsFileRevision> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FileBasicInfo, GetFileInformationByHandleEx, FILE_BASIC_INFO,
    };
    let mut info = FILE_BASIC_INFO::default();
    let ok = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileBasicInfo,
            (&mut info as *mut FILE_BASIC_INFO).cast(),
            std::mem::size_of::<FILE_BASIC_INFO>() as u32,
        )
    };
    if ok == 0 {
        return Err(crate::io(&format!("querying {label} revision"))(
            std::io::Error::last_os_error(),
        ));
    }
    Ok(WindowsFileRevision {
        last_write_time: info.LastWriteTime,
        change_time: info.ChangeTime,
    })
}

#[cfg(windows)]
fn windows_metadata_is_reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    metadata.file_attributes() & 0x400 != 0
}

#[cfg(windows)]
impl KernelStoreLock {
    fn new(root: ManagerRootLock) -> Self {
        Self { root }
    }

    fn revalidate(&self) -> crate::Result<()> {
        self.root.revalidate_named()
    }

    fn load(&self, path: &Path) -> crate::Result<Loadout> {
        use std::io::Read as _;
        use std::os::windows::fs::OpenOptionsExt as _;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
        };
        self.revalidate()?;
        let result = (|| -> crate::Result<Loadout> {
            run_load_open_race_hook();
            let mut file = match std::fs::OpenOptions::new()
                .read(true)
                .share_mode(FILE_SHARE_READ)
                .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
                .open(path)
            {
                Ok(file) => file,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(Loadout::default())
                }
                Err(error) => return Err(crate::io("opening locked manager loadout")(error)),
            };
            let metadata = file
                .metadata()
                .map_err(crate::io("reading locked manager loadout metadata"))?;
            if !metadata.is_file() || windows_metadata_is_reparse(&metadata) {
                return Err(crate::ModError::Other(format!(
                    "manager loadout is not a real regular file: {}",
                    path.display()
                )));
            }
            if metadata.len() > MAX_LOADOUT_BYTES {
                return Err(crate::ModError::Other(format!(
                    "loadout exceeds the {MAX_LOADOUT_BYTES}-byte limit: {}",
                    path.display()
                )));
            }
            let identity = windows_file_identity(&file, "manager loadout")?;
            let revision = windows_file_revision(&file, "manager loadout")?;
            run_load_race_hook();
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(metadata.len() as usize)
                .map_err(|_| {
                    crate::ModError::Other("could not reserve bounded loadout bytes".into())
                })?;
            file.by_ref()
                .take(MAX_LOADOUT_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(crate::io("reading locked manager loadout"))?;
            let after = file
                .metadata()
                .map_err(crate::io("rechecking locked manager loadout metadata"))?;
            if bytes.len() as u64 > MAX_LOADOUT_BYTES
                || bytes.len() as u64 != after.len()
                || metadata.len() != after.len()
                || windows_file_identity(&file, "manager loadout")? != identity
                || windows_file_revision(&file, "manager loadout")? != revision
            {
                return Err(crate::ModError::Other(format!(
                    "manager loadout changed or exceeded its bound while being read: {}",
                    path.display()
                )));
            }
            loadout::parse_bytes(&bytes)
        })();
        self.revalidate()?;
        result
    }

    fn save(&self, path: &Path, value: &Loadout) -> crate::Result<()> {
        self.revalidate()?;
        let bytes = loadout::serialized_bytes(value)?;
        if bytes.len() as u64 > MAX_LOADOUT_BYTES {
            return Err(crate::ModError::Other(format!(
                "loadout serialization exceeds the {MAX_LOADOUT_BYTES}-byte limit"
            )));
        }
        let result = crate::atomic_write(path, &bytes);
        self.revalidate()?;
        result
    }
}

#[cfg(unix)]
#[derive(Debug)]
struct KernelStoreLock {
    root: ManagerRootLock,
}

#[cfg(unix)]
fn unix_child_name(path: &Path, label: &str) -> crate::Result<std::ffi::CString> {
    use std::os::unix::ffi::OsStrExt as _;
    let name = path.file_name().ok_or_else(|| {
        crate::ModError::Other(format!("{label} has no file name: {}", path.display()))
    })?;
    std::ffi::CString::new(name.as_bytes()).map_err(|_| {
        crate::ModError::Other(format!(
            "{label} file name contains NUL: {}",
            path.display()
        ))
    })
}

#[cfg(unix)]
fn openat_retry(
    directory: std::os::unix::io::RawFd,
    name: &std::ffi::CStr,
    flags: i32,
    mode: libc::mode_t,
) -> std::io::Result<std::os::unix::io::RawFd> {
    loop {
        let fd = unsafe { libc::openat(directory, name.as_ptr(), flags, mode) };
        if fd >= 0 {
            return Ok(fd);
        }
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

#[cfg(unix)]
fn renameat_retry(
    directory: std::os::unix::io::RawFd,
    from: &std::ffi::CStr,
    to: &std::ffi::CStr,
) -> std::io::Result<()> {
    loop {
        if unsafe { libc::renameat(directory, from.as_ptr(), directory, to.as_ptr()) } == 0 {
            return Ok(());
        }
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

#[cfg(unix)]
fn unlinkat_file(directory: std::os::unix::io::RawFd, name: &std::ffi::CStr) {
    loop {
        if unsafe { libc::unlinkat(directory, name.as_ptr(), 0) } == 0 {
            return;
        }
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::Interrupted {
            return;
        }
    }
}

#[cfg(unix)]
impl KernelStoreLock {
    fn new(root: ManagerRootLock) -> Self {
        Self { root }
    }

    fn parent(&self) -> &std::fs::File {
        self.root.directory_file()
    }

    fn revalidate(&self) -> crate::Result<()> {
        self.root.revalidate_named()
    }

    fn load(&self, path: &Path) -> crate::Result<Loadout> {
        use std::io::Read as _;
        use std::os::unix::fs::MetadataExt as _;
        use std::os::unix::io::{AsRawFd as _, FromRawFd as _};

        self.revalidate()?;
        let result = (|| -> crate::Result<Loadout> {
            run_load_open_race_hook();
            let name = unix_child_name(path, "manager loadout")?;
            let fd = match openat_retry(
                self.parent().as_raw_fd(),
                &name,
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                0,
            ) {
                Ok(fd) => fd,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(Loadout::default())
                }
                Err(error) => return Err(crate::io("opening locked manager loadout")(error)),
            };
            let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
            let before = file
                .metadata()
                .map_err(crate::io("reading locked manager loadout metadata"))?;
            if !before.is_file() {
                return Err(crate::ModError::Other(format!(
                    "manager loadout is not a regular file: {}",
                    path.display()
                )));
            }
            if before.len() > MAX_LOADOUT_BYTES {
                return Err(crate::ModError::Other(format!(
                    "loadout exceeds the {MAX_LOADOUT_BYTES}-byte limit: {}",
                    path.display()
                )));
            }
            run_load_race_hook();
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(before.len() as usize)
                .map_err(|_| {
                    crate::ModError::Other("could not reserve bounded loadout bytes".into())
                })?;
            file.by_ref()
                .take(MAX_LOADOUT_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(crate::io("reading locked manager loadout"))?;
            let after = file
                .metadata()
                .map_err(crate::io("rechecking locked manager loadout metadata"))?;
            if bytes.len() as u64 > MAX_LOADOUT_BYTES
                || before.dev() != after.dev()
                || before.ino() != after.ino()
                || before.len() != after.len()
                || before.mtime() != after.mtime()
                || before.mtime_nsec() != after.mtime_nsec()
                || before.ctime() != after.ctime()
                || before.ctime_nsec() != after.ctime_nsec()
                || bytes.len() as u64 != after.len()
            {
                return Err(crate::ModError::Other(format!(
                    "manager loadout changed or exceeded its bound while being read: {}",
                    path.display()
                )));
            }
            loadout::parse_bytes(&bytes)
        })();
        self.revalidate()?;
        result
    }

    fn save(&self, path: &Path, value: &Loadout) -> crate::Result<()> {
        use std::io::Write as _;
        use std::os::unix::io::{AsRawFd as _, FromRawFd as _};
        use std::sync::atomic::{AtomicU64, Ordering};

        self.revalidate()?;
        let result = (|| -> crate::Result<()> {
            let bytes = loadout::serialized_bytes(value)?;
            if bytes.len() as u64 > MAX_LOADOUT_BYTES {
                return Err(crate::ModError::Other(format!(
                    "loadout serialization exceeds the {MAX_LOADOUT_BYTES}-byte limit"
                )));
            }
            #[cfg(test)]
            if crate::take_injected_atomic_write_failure(path) {
                return Err(crate::ModError::Other(format!(
                    "injected atomic-write failure for {}",
                    path.display()
                )));
            }
            static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
            let destination = unix_child_name(path, "manager loadout")?;
            let directory = self.parent().as_raw_fd();
            let (temp_name, mut temp) = loop {
                let suffix = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
                let raw = format!(".gore-loadout-{}-{suffix}.pending", std::process::id());
                let name = std::ffi::CString::new(raw).expect("generated name contains no NUL");
                match openat_retry(
                    directory,
                    &name,
                    libc::O_WRONLY
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC,
                    0o600,
                ) {
                    Ok(fd) => break (name, unsafe { std::fs::File::from_raw_fd(fd) }),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => {
                        return Err(crate::io("creating locked loadout staging file")(error))
                    }
                }
            };
            let publish = (|| -> crate::Result<()> {
                temp.write_all(&bytes)
                    .map_err(crate::io("writing locked loadout staging file"))?;
                temp.sync_all()
                    .map_err(crate::io("syncing locked loadout staging file"))?;
                drop(temp);
                renameat_retry(directory, &temp_name, &destination)
                    .map_err(crate::io("publishing locked manager loadout"))?;
                self.parent()
                    .sync_all()
                    .map_err(crate::io("syncing manager store directory"))?;
                Ok(())
            })();
            if publish.is_err() {
                unlinkat_file(directory, &temp_name);
            }
            publish
        })();
        self.revalidate()?;
        result
    }
}

#[cfg(not(any(windows, unix)))]
#[derive(Debug)]
struct KernelStoreLock;

#[cfg(not(any(windows, unix)))]
impl KernelStoreLock {
    fn new(_root: ManagerRootLock) -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn meta(id: &str) -> ModEntryMeta {
        ModEntryMeta {
            id: id.into(),
            kind: super::super::model::ModKind::ForeignPak,
            name: id.into(),
            version: String::new(),
            author: String::new(),
            imported_at: "2026-01-01T00:00:00Z".into(),
            source: String::new(),
            components: Vec::new(),
        }
    }

    fn write_entry(library: &Path, id: &str) {
        let entry = library.join(id);
        fs::create_dir_all(&entry).unwrap();
        fs::write(
            entry.join(super::super::model::META_FILE),
            serde_json::to_vec(&meta(id)).unwrap(),
        )
        .unwrap();
    }

    fn write_loadout(path: &Path, entries: &[(&str, bool)]) {
        loadout::save(
            path,
            &Loadout {
                format: 1,
                entries: entries
                    .iter()
                    .map(|(id, enabled)| LoadoutEntry {
                        id: (*id).into(),
                        enabled: *enabled,
                    })
                    .collect(),
            },
        )
        .unwrap();
    }

    fn prepare_read_only_library_lock(library: &Path) {
        #[cfg(windows)]
        fs::write(library.join(".gore-manager-library.lock"), b"").unwrap();
        #[cfg(not(windows))]
        let _ = library;
    }

    #[test]
    fn reconcile_keeps_first_known_slot_and_appends_missing_sorted_disabled() {
        let loadout = Loadout {
            format: 1,
            entries: vec![
                LoadoutEntry {
                    id: "b".into(),
                    enabled: true,
                },
                LoadoutEntry {
                    id: "stale".into(),
                    enabled: true,
                },
                LoadoutEntry {
                    id: "b".into(),
                    enabled: false,
                },
            ],
        };
        assert_eq!(
            reconcile(&[meta("c"), meta("b"), meta("a")], &loadout)
                .unwrap()
                .entries,
            vec![
                LoadoutEntry {
                    id: "b".into(),
                    enabled: true
                },
                LoadoutEntry {
                    id: "a".into(),
                    enabled: false
                },
                LoadoutEntry {
                    id: "c".into(),
                    enabled: false
                },
            ]
        );
    }

    #[test]
    fn open_repairs_valid_drift_but_never_rewrites_an_equal_loadout() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("library");
        let path = temp.path().join("loadout.json");
        write_entry(&library, "a");
        write_entry(&library, "b");
        write_loadout(&path, &[("a", true), ("stale", true), ("a", false)]);

        let first = StoreSnapshot::open(&library, &path).unwrap();
        assert_eq!(
            first.loadout().entries,
            vec![
                LoadoutEntry {
                    id: "a".into(),
                    enabled: true
                },
                LoadoutEntry {
                    id: "b".into(),
                    enabled: false
                },
            ]
        );
        drop(first);

        // A save would trip this injection. An already-reconciled authoritative read must not
        // rewrite merely to refresh its timestamp or inode.
        crate::fail_next_atomic_write(&path);
        let second = StoreSnapshot::open(&library, &path).unwrap();
        assert_eq!(second.loadout().entries.len(), 2);
    }

    #[test]
    fn read_only_inspection_reconciles_only_in_memory_and_creates_no_store_artifact() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("library");
        let path = temp.path().join("loadout.json");
        write_entry(&library, "a");
        write_entry(&library, "b");
        prepare_read_only_library_lock(&library);
        write_loadout(&path, &[("a", true), ("stale", true), ("a", false)]);
        let before = fs::read(&path).unwrap();
        crate::fail_next_atomic_write(&path);

        let inspection = StoreSnapshot::inspect_read_only(&library, &path).unwrap();
        assert_eq!(
            inspection.loadout().entries,
            vec![
                LoadoutEntry {
                    id: "a".into(),
                    enabled: true,
                },
                LoadoutEntry {
                    id: "b".into(),
                    enabled: false,
                },
            ]
        );
        drop(inspection);
        assert_eq!(fs::read(&path).unwrap(), before);
        assert!(!temp.path().join(STORE_LOCK_FILE).exists());
        assert!(!fs::read_dir(temp.path()).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("pending")));

        // The read-only path did not even attempt a save: the injected failure is still armed for
        // the next writable reconciliation.
        assert!(StoreSnapshot::open(&library, &path).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
    }

    #[test]
    fn read_only_inspection_of_missing_store_creates_nothing() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("missing-library");
        let path = temp.path().join("missing-store").join("loadout.json");
        let inspection = StoreSnapshot::inspect_read_only(&library, &path).unwrap();
        assert!(inspection.loadout().entries.is_empty());
        drop(inspection);
        assert!(!library.exists());
        assert!(!path.parent().unwrap().exists());
        assert!(!temp.path().join(STORE_LOCK_FILE).exists());
    }

    #[test]
    fn read_only_inspection_refuses_missing_library_with_persisted_intent_without_artifacts() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("missing-library");
        let store = temp.path().join("store");
        let path = store.join("loadout.json");
        fs::create_dir(&store).unwrap();
        write_loadout(&path, &[("still-wanted", true)]);
        let before = fs::read(&path).unwrap();

        let error = match StoreSnapshot::inspect_read_only(&library, &path) {
            Err(StoreInspectionError::Library(error)) => error.to_string(),
            Err(error) => panic!("missing Library was classified as {error}"),
            Ok(_) => panic!("missing Library erased persisted intent in memory"),
        };
        assert!(error.contains("library is missing"), "{error}");
        assert_eq!(fs::read(&path).unwrap(), before);
        assert!(!library.exists());
        assert!(!store.join(STORE_LOCK_FILE).exists());
        assert!(!library.join(STORE_LOCK_FILE).exists());
        assert!(!fs::read_dir(&store).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("pending")));
    }

    #[test]
    fn read_only_missing_library_intent_detects_library_appearance_during_load() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("missing-library");
        let path = temp.path().join("loadout.json");
        write_loadout(&path, &[("still-wanted", true)]);
        let before = fs::read(&path).unwrap();
        let raced_library = library.clone();
        inject_load_race(move || fs::create_dir(&raced_library).unwrap());

        let error = match StoreSnapshot::inspect_read_only(&library, &path) {
            Err(StoreInspectionError::Library(error)) => error.to_string(),
            Err(error) => panic!("Library appearance was classified as {error}"),
            Ok(_) => panic!("Library appearance was accepted as a stable absent snapshot"),
        };
        assert!(error.contains("appeared during"), "{error}");
        assert_eq!(fs::read(&path).unwrap(), before);
        assert!(!temp.path().join(STORE_LOCK_FILE).exists());
        assert!(!library.join(STORE_LOCK_FILE).exists());
    }

    #[test]
    fn read_only_status_classifies_a_library_that_appears_after_projection() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("missing-library");
        let path = temp.path().join("loadout.json");
        let inspection = StoreSnapshot::inspect_read_only(&library, &path).unwrap();
        fs::create_dir(&library).unwrap();

        let error = inspection.status(temp.path()).unwrap_err();
        assert!(
            matches!(error, StoreInspectionStatusError::Library(_)),
            "{error}"
        );
        assert!(!temp.path().join(STORE_LOCK_FILE).exists());
        assert!(!library.join(".gore-manager-library.lock").exists());
    }

    #[test]
    fn read_only_inspection_refuses_replacement_evidence_without_recovery() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("library");
        let path = temp.path().join("loadout.json");
        write_entry(&library, "a");
        prepare_read_only_library_lock(&library);
        write_loadout(&path, &[("a", true)]);
        let transaction = library.join(".replacing-read-only-test");
        fs::create_dir(&transaction).unwrap();
        let evidence = transaction.join("evidence");
        fs::write(&evidence, b"keep").unwrap();
        let before = fs::read(&path).unwrap();

        let error = match StoreSnapshot::inspect_read_only(&library, &path) {
            Ok(_) => panic!("read-only inspection consumed uncertain replacement state"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("requires recovery"), "{error}");
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(fs::read(&evidence).unwrap(), b"keep");
        assert!(!temp.path().join(STORE_LOCK_FILE).exists());
    }

    #[test]
    fn corrupt_or_unsupported_loadout_is_left_byte_exact() {
        for bytes in [
            b"{not json".as_slice(),
            br#"{"format":0,"entries":[]}"#.as_slice(),
            br#"{"format":2,"entries":[]}"#.as_slice(),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let library = temp.path().join("library");
            let path = temp.path().join("loadout.json");
            write_entry(&library, "a");
            fs::write(&path, bytes).unwrap();
            assert!(StoreSnapshot::open(&library, &path).is_err());
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
    }

    #[test]
    fn loadout_over_one_mib_is_refused_without_rewrite() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("library");
        let path = temp.path().join("loadout.json");
        fs::create_dir_all(&library).unwrap();
        let file = fs::File::create(&path).unwrap();
        file.set_len(MAX_LOADOUT_BYTES + 1).unwrap();
        drop(file);
        let before = fs::metadata(&path).unwrap().len();
        let error = match StoreSnapshot::open(&library, &path) {
            Ok(_) => panic!("oversized loadout was accepted"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("1048576-byte limit"), "{error}");
        assert_eq!(fs::metadata(&path).unwrap().len(), before);
    }

    #[test]
    fn same_size_loadout_mutation_is_detected_or_denied_by_the_open_handle() {
        use std::io::{Seek as _, SeekFrom, Write as _};
        use std::sync::{Arc, Mutex};

        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("library");
        let path = temp.path().join("loadout.json");
        write_entry(&library, "aaaa");
        write_loadout(&path, &[("aaaa", true)]);
        let before = fs::read(&path).unwrap();
        let replacement = String::from_utf8(before.clone())
            .unwrap()
            .replace("aaaa", "bbbb")
            .into_bytes();
        assert_eq!(replacement.len(), before.len());
        let result = Arc::new(Mutex::new(None));
        let observed = result.clone();
        let target = path.clone();
        inject_load_race(move || {
            let wrote = match fs::OpenOptions::new().write(true).open(&target) {
                Ok(mut file) => {
                    file.seek(SeekFrom::Start(0)).unwrap();
                    file.write_all(&replacement).unwrap();
                    file.sync_all().unwrap();
                    true
                }
                Err(_) => false,
            };
            *observed.lock().unwrap() = Some(wrote);
        });
        let opened = StoreSnapshot::open(&library, &path);
        let writer_succeeded = *result.lock().unwrap();
        match writer_succeeded {
            Some(true) => {
                let error = match opened {
                    Ok(_) => panic!("same-size mutation escaped revision validation"),
                    Err(error) => error.to_string(),
                };
                assert!(
                    error.contains("changed") || error.contains("revision"),
                    "{error}"
                );
            }
            Some(false) => assert!(opened.is_ok(), "denied writer should preserve valid bytes"),
            None => panic!("load race hook did not run"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn missing_loadout_open_race_revalidates_named_store_root_before_success() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("library");
        write_entry(&library, "a");
        let parent = temp.path().join("store");
        let retained = temp.path().join("retained-store");
        let decoy = temp.path().join("decoy-store");
        fs::create_dir(&parent).unwrap();
        fs::create_dir(&decoy).unwrap();
        let requested = parent.join("missing-loadout.json");
        let hook_parent = parent.clone();
        let hook_retained = retained.clone();
        let hook_decoy = decoy.clone();
        inject_load_open_race(move || {
            fs::rename(&hook_parent, &hook_retained).unwrap();
            std::os::unix::fs::symlink(&hook_decoy, &hook_parent).unwrap();
        });

        let error = match StoreSnapshot::open(&library, &requested) {
            Ok(_) => panic!("missing-loadout race bypassed named-root post-validation"),
            Err(error) => error.to_string(),
        };
        assert!(
            error.contains("named Manager mutation root")
                || error.contains("named filesystem identity"),
            "{error}"
        );
        assert!(!retained.join("missing-loadout.json").exists());
        assert!(!decoy.join("missing-loadout.json").exists());
    }

    #[cfg(any(unix, windows))]
    #[cfg_attr(
        windows,
        ignore = "requires Windows symbolic-link privilege; run explicitly on a privileged worker"
    )]
    #[test]
    fn final_loadout_symlink_is_refused_without_following_it() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("library");
        let outside = temp.path().join("outside.json");
        let path = temp.path().join("loadout.json");
        fs::create_dir_all(&library).unwrap();
        write_loadout(&outside, &[]);
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &path).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&outside, &path).unwrap();
        let before = fs::read(&outside).unwrap();
        assert!(StoreSnapshot::open(&library, &path).is_err());
        assert_eq!(fs::read(&outside).unwrap(), before);
    }

    #[test]
    fn strict_library_uncertainty_refuses_without_repairing_loadout() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("library");
        let path = temp.path().join("loadout.json");
        write_entry(&library, "a");
        fs::create_dir_all(library.join("broken")).unwrap();
        fs::write(
            library.join("broken").join(super::super::model::META_FILE),
            b"not json",
        )
        .unwrap();
        write_loadout(&path, &[("stale", true)]);
        let before = fs::read(&path).unwrap();

        let error = match StoreSnapshot::open(&library, &path) {
            Ok(_) => panic!("strict library accepted a corrupt public entry"),
            Err(error) => error.to_string(),
        };
        assert!(
            error.contains("corrupt") || error.contains("expected"),
            "{error}"
        );
        assert_eq!(fs::read(&path).unwrap(), before);
    }

    #[test]
    fn failed_reconciliation_save_preserves_the_previous_loadout() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("library");
        let path = temp.path().join("loadout.json");
        write_entry(&library, "a");
        write_loadout(&path, &[("stale", true)]);
        let before = fs::read(&path).unwrap();
        crate::fail_next_atomic_write(&path);

        assert!(StoreSnapshot::open(&library, &path).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
    }

    #[test]
    fn explicit_replacement_cannot_drop_a_concurrently_published_library_slot() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("library");
        let path = temp.path().join("loadout.json");
        write_entry(&library, "a");
        write_entry(&library, "new");
        write_loadout(&path, &[("a", false)]);

        let mut store = StoreSnapshot::open(&library, &path).unwrap();
        store
            .replace_loadout(Loadout {
                format: 1,
                entries: vec![LoadoutEntry {
                    id: "a".into(),
                    enabled: true,
                }],
            })
            .unwrap();
        assert_eq!(
            store.loadout().entries,
            vec![
                LoadoutEntry {
                    id: "a".into(),
                    enabled: true
                },
                LoadoutEntry {
                    id: "new".into(),
                    enabled: false
                },
            ]
        );
    }

    #[test]
    fn serialized_rmw_preserves_independent_parallel_edits() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("library");
        let path = temp.path().join("loadout.json");
        write_entry(&library, "a");
        write_entry(&library, "b");
        write_loadout(&path, &[("a", false), ("b", false)]);
        std::thread::scope(|scope| {
            for id in ["a", "b"] {
                let library = library.clone();
                let path = path.clone();
                scope.spawn(move || {
                    let mut store = StoreSnapshot::open(&library, &path).unwrap();
                    store
                        .update_loadout(|loadout| {
                            loadout
                                .entries
                                .iter_mut()
                                .find(|entry| entry.id == id)
                                .unwrap()
                                .enabled = true;
                            Ok(())
                        })
                        .unwrap();
                });
            }
        });
        let final_loadout = loadout::load(&path).unwrap();
        assert!(final_loadout.entries.iter().all(|entry| entry.enabled));
    }

    #[test]
    fn parallel_library_publications_reconcile_without_losing_either_slot() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("library");
        let path = temp.path().join("loadout.json");
        let first = temp.path().join("first_P.pak");
        let second = temp.path().join("second_P.pak");
        fs::write(&first, b"first").unwrap();
        fs::write(&second, b"second").unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        std::thread::scope(|scope| {
            for source in [&first, &second] {
                let barrier = barrier.clone();
                let library = library.clone();
                let path = path.clone();
                scope.spawn(move || {
                    super::super::import::import(&library, source).unwrap();
                    barrier.wait();
                    drop(StoreSnapshot::open(&library, &path).unwrap());
                });
            }
        });
        let final_loadout = loadout::load(&path).unwrap();
        assert_eq!(final_loadout.entries.len(), 2);
        assert!(final_loadout.entries.iter().all(|entry| !entry.enabled));
    }

    #[cfg(windows)]
    #[test]
    fn windows_loadout_alias_is_rewritten_to_the_library_spelling() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("library");
        let path = temp.path().join("loadout.json");
        write_entry(&library, "CaseID");
        write_loadout(&path, &[("caseid", true)]);
        let store = StoreSnapshot::open(&library, &path).unwrap();
        assert_eq!(store.loadout().entries[0].id, "CaseID");
        assert!(store.loadout().entries[0].enabled);
    }

    #[cfg(windows)]
    #[test]
    fn windows_final_sigma_alias_preserves_enabled_state() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("library");
        let path = temp.path().join("loadout.json");
        write_entry(&library, "Σ");
        write_loadout(&path, &[("ς", true)]);
        let store = StoreSnapshot::open(&library, &path).unwrap();
        assert_eq!(store.loadout().entries[0].id, "Σ");
        assert!(store.loadout().entries[0].enabled);
    }

    #[test]
    #[ignore = "child-process worker; invoked explicitly by store lock tests"]
    fn store_lock_child_worker() {
        let library = PathBuf::from(std::env::var_os("GORE_TEST_CHILD_LIBRARY").unwrap());
        let loadout = PathBuf::from(std::env::var_os("GORE_TEST_CHILD_LOADOUT").unwrap());
        match std::env::var("GORE_TEST_CHILD_STORE_MODE")
            .unwrap_or_else(|_| "lock".into())
            .as_str()
        {
            "lock" => drop(StoreSnapshot::open(&library, &loadout).unwrap()),
            "enable" => {
                let id = std::env::var("GORE_TEST_CHILD_STORE_ID").unwrap();
                let mut store = StoreSnapshot::open(&library, &loadout).unwrap();
                store
                    .update_loadout(|loadout| {
                        loadout
                            .entries
                            .iter_mut()
                            .find(|entry| entry.id == id)
                            .unwrap()
                            .enabled = true;
                        Ok(())
                    })
                    .unwrap();
            }
            mode => panic!("unknown child store mode {mode:?}"),
        }
    }

    fn spawn_store_child(
        library: &Path,
        loadout: &Path,
        marker: &Path,
        hold_ms: u64,
        mode: &str,
        id: Option<&str>,
    ) -> std::process::Child {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .arg("store_lock_child_worker")
            .arg("--ignored")
            .arg("--nocapture")
            .env("GORE_TEST_CHILD_LIBRARY", library)
            .env("GORE_TEST_CHILD_LOADOUT", loadout)
            .env(STORE_LOCK_MARKER_ENV, marker)
            .env(STORE_LOCK_HOLD_MS_ENV, hold_ms.to_string())
            .env("GORE_TEST_CHILD_STORE_MODE", mode)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());
        if let Some(id) = id {
            command.env("GORE_TEST_CHILD_STORE_ID", id);
        }
        command.spawn().unwrap()
    }

    fn spawn_crossed_store_child(
        library: &Path,
        loadout: &Path,
        first_marker: &Path,
        first_hold_ms: u64,
    ) -> std::process::Child {
        std::process::Command::new(std::env::current_exe().unwrap())
            .arg("store_lock_child_worker")
            .arg("--ignored")
            .arg("--nocapture")
            .env("GORE_TEST_CHILD_LIBRARY", library)
            .env("GORE_TEST_CHILD_LOADOUT", loadout)
            .env("GORE_TEST_CHILD_STORE_MODE", "lock")
            .env("GORE_TEST_MANAGER_FIRST_ROOT_MARKER", first_marker)
            .env(
                "GORE_TEST_MANAGER_FIRST_ROOT_HOLD_MS",
                first_hold_ms.to_string(),
            )
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap()
    }

    fn spawn_identity_lock_child(
        library: &Path,
        marker: &Path,
        hold_ms: u64,
    ) -> std::process::Child {
        std::process::Command::new(std::env::current_exe().unwrap())
            .arg("library_lock_child_worker")
            .arg("--ignored")
            .arg("--nocapture")
            .env("GORE_TEST_CHILD_LIBRARY", library)
            .env("GORE_TEST_CHILD_MODE", "lock")
            .env("GORE_TEST_LIBRARY_LOCK_MARKER", marker)
            .env("GORE_TEST_LIBRARY_LOCK_HOLD_MS", hold_ms.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap()
    }

    fn child_output_bounded(
        mut child: std::process::Child,
        timeout: std::time::Duration,
    ) -> std::process::Output {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if child.try_wait().unwrap().is_some() {
                return child.wait_with_output().unwrap();
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let output = child.wait_with_output().unwrap();
                panic!(
                    "child timed out; stderr: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    fn assert_child_output_success(output: std::process::Output) {
        assert!(
            output.status.success(),
            "child failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn wait_for_path(path: &Path) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !path.exists() {
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for {}",
                path.display()
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn kernel_store_lock_blocks_another_process_and_crash_releases_it() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("library");
        let loadout = temp.path().join("loadout.json");
        let marker_a = temp.path().join("locked-a");
        let marker_b = temp.path().join("locked-b");
        let mut first = spawn_store_child(&library, &loadout, &marker_a, 60_000, "lock", None);
        wait_for_path(&marker_a);
        let mut second = spawn_store_child(&library, &loadout, &marker_b, 0, "lock", None);
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(
            !marker_b.exists(),
            "second process acquired held store lock"
        );
        assert!(
            second.try_wait().unwrap().is_none(),
            "second process did not block"
        );
        first.kill().unwrap();
        first.wait().unwrap();
        let output = second.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(marker_b.exists());
    }

    #[test]
    fn cross_process_rmw_preserves_independent_existing_slot_edits() {
        let temp = tempfile::tempdir().unwrap();
        let root_a = temp.path().join("root-a");
        let root_b = temp.path().join("root-b");
        fs::create_dir(&root_a).unwrap();
        fs::create_dir(&root_b).unwrap();
        #[cfg(windows)]
        let (library, store_parent) = {
            let identity_a =
                prepare_existing_manager_root_lock(&root_a, "RMW regression root A", false)
                    .unwrap()
                    .identity();
            let identity_b =
                prepare_existing_manager_root_lock(&root_b, "RMW regression root B", false)
                    .unwrap()
                    .identity();
            if identity_a < identity_b {
                (root_a, root_b)
            } else {
                (root_b, root_a)
            }
        };
        #[cfg(not(windows))]
        let (library, store_parent) = (root_a, root_b);
        let loadout = store_parent.join("loadout.json");
        write_entry(&library, "a");
        write_entry(&library, "b");
        write_loadout(&loadout, &[("a", false), ("b", false)]);
        let marker_a = temp.path().join("rmw-a");
        let marker_b = temp.path().join("rmw-b");
        let first = spawn_store_child(&library, &loadout, &marker_a, 1_000, "enable", Some("a"));
        wait_for_path(&marker_a);
        let second = spawn_store_child(&library, &loadout, &marker_b, 0, "enable", Some("b"));
        for child in [first, second] {
            let output = child_output_bounded(child, std::time::Duration::from_secs(10));
            assert!(
                output.status.success(),
                "child failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let stored = loadout::load(&loadout).unwrap();
        assert!(stored.entries.iter().all(|entry| entry.enabled));
    }

    #[test]
    fn crossed_store_library_pairs_finish_in_canonical_physical_order() {
        let temp = tempfile::tempdir().unwrap();
        let root_a = temp.path().join("root-a");
        let root_b = temp.path().join("root-b");
        fs::create_dir(&root_a).unwrap();
        fs::create_dir(&root_b).unwrap();
        let loadout_a = root_a.join(".loadout-a.json");
        let loadout_b = root_b.join(".loadout-b.json");
        write_loadout(&loadout_a, &[]);
        write_loadout(&loadout_b, &[]);
        let marker_a = temp.path().join("first-a");
        let marker_b = temp.path().join("first-b");

        let first = spawn_crossed_store_child(&root_b, &loadout_a, &marker_a, 250);
        let second = spawn_crossed_store_child(&root_a, &loadout_b, &marker_b, 250);
        assert_child_output_success(child_output_bounded(
            first,
            std::time::Duration::from_secs(10),
        ));
        assert_child_output_success(child_output_bounded(
            second,
            std::time::Duration::from_secs(10),
        ));
        assert_eq!(loadout::load(&loadout_a).unwrap(), Loadout::default());
        assert_eq!(loadout::load(&loadout_b).unwrap(), Loadout::default());
    }

    #[test]
    fn crash_while_holding_first_canonical_root_releases_the_waiter() {
        let temp = tempfile::tempdir().unwrap();
        let root_a = temp.path().join("root-a");
        let root_b = temp.path().join("root-b");
        fs::create_dir(&root_a).unwrap();
        fs::create_dir(&root_b).unwrap();
        let loadout_a = root_a.join(".loadout-a.json");
        let loadout_b = root_b.join(".loadout-b.json");
        write_loadout(&loadout_a, &[]);
        write_loadout(&loadout_b, &[]);
        let marker_a = temp.path().join("first-a");
        let marker_b = temp.path().join("first-b");

        let mut killed = spawn_crossed_store_child(&root_b, &loadout_a, &marker_a, 60_000);
        wait_for_path(&marker_a);
        let waiter = spawn_crossed_store_child(&root_a, &loadout_b, &marker_b, 0);
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert!(!marker_b.exists(), "waiter bypassed the held first root");
        killed.kill().unwrap();
        killed.wait().unwrap();
        assert_child_output_success(child_output_bounded(
            waiter,
            std::time::Duration::from_secs(10),
        ));
        assert!(marker_b.exists());
    }

    #[test]
    fn store_and_standalone_identity_contend_on_the_same_universal_root_both_directions() {
        let temp = tempfile::tempdir().unwrap();
        let store_root = temp.path().join("store");
        let library = temp.path().join("library");
        fs::create_dir(&store_root).unwrap();
        fs::create_dir(&library).unwrap();
        let loadout = store_root.join("loadout.json");
        write_loadout(&loadout, &[]);

        let store_marker = temp.path().join("store-first");
        let identity_marker = temp.path().join("identity-waiting");
        let mut store = spawn_store_child(&library, &loadout, &store_marker, 60_000, "lock", None);
        wait_for_path(&store_marker);
        let identity = spawn_identity_lock_child(&library, &identity_marker, 0);
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert!(
            !identity_marker.exists(),
            "standalone Identity bypassed Store's Library-root lock"
        );
        store.kill().unwrap();
        store.wait().unwrap();
        assert_child_output_success(child_output_bounded(
            identity,
            std::time::Duration::from_secs(10),
        ));

        fs::remove_file(&store_marker).unwrap();
        fs::remove_file(&identity_marker).unwrap();
        let mut identity = spawn_identity_lock_child(&library, &identity_marker, 60_000);
        wait_for_path(&identity_marker);
        let store = spawn_store_child(&library, &loadout, &store_marker, 0, "lock", None);
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert!(
            !store_marker.exists(),
            "Store bypassed standalone Identity's universal root lock"
        );
        identity.kill().unwrap();
        identity.wait().unwrap();
        assert_child_output_success(child_output_bounded(
            store,
            std::time::Duration::from_secs(10),
        ));
        assert!(store_marker.exists());
    }

    #[test]
    fn same_root_alias_is_rejected_without_rewriting_loadout() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("one-root");
        fs::create_dir(&root).unwrap();
        let loadout = root.join("loadout.json");
        write_loadout(&loadout, &[]);
        let before = fs::read(&loadout).unwrap();

        let error = match StoreSnapshot::open(&root.join("."), &loadout) {
            Ok(_) => panic!("same physical root was accepted for Store and Library"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("must be different directories"), "{error}");
        assert_eq!(fs::read(&loadout).unwrap(), before);
        assert!(
            !root.join(STORE_LOCK_FILE).exists(),
            "same-root rejection created a persistent Manager lock artifact"
        );
        assert!(!fs::read_dir(&root).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("pending")));
    }

    #[test]
    fn in_process_crossed_pairs_serialize_without_deadlock() {
        let temp = tempfile::tempdir().unwrap();
        let root_a = temp.path().join("root-a");
        let root_b = temp.path().join("root-b");
        fs::create_dir(&root_a).unwrap();
        fs::create_dir(&root_b).unwrap();
        let loadout_a = root_a.join(".loadout-a.json");
        let loadout_b = root_b.join(".loadout-b.json");
        write_loadout(&loadout_a, &[]);
        write_loadout(&loadout_b, &[]);
        let (send, receive) = std::sync::mpsc::channel();
        let mut workers = Vec::new();
        for (library, loadout) in [(root_b.clone(), loadout_a), (root_a.clone(), loadout_b)] {
            let send = send.clone();
            workers.push(std::thread::spawn(move || {
                let result = StoreSnapshot::open(&library, &loadout).map(drop);
                send.send(result.map_err(|error| error.to_string()))
                    .unwrap();
            }));
        }
        drop(send);
        for _ in 0..2 {
            receive
                .recv_timeout(std::time::Duration::from_secs(10))
                .expect("crossed in-process Store pair deadlocked")
                .unwrap();
        }
        for worker in workers {
            worker.join().unwrap();
        }
    }

    #[test]
    fn missing_library_with_persisted_intent_is_left_absent_and_byte_exact() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("missing-library");
        let loadout = temp.path().join("loadout.json");
        write_loadout(&loadout, &[("keep-intent", true)]);
        let before = fs::read(&loadout).unwrap();

        let error = match StoreSnapshot::open(&library, &loadout) {
            Ok(_) => panic!("missing Library erased persisted loadout intent"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("library is missing"), "{error}");
        assert!(!library.exists());
        assert_eq!(fs::read(&loadout).unwrap(), before);
    }

    #[test]
    fn library_published_during_empty_bootstrap_is_snapshotted_not_overwritten() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("raced-library");
        let loadout = temp.path().join("loadout.json");
        write_loadout(&loadout, &[]);
        let raced_library = library.clone();
        inject_bootstrap_race(move || write_entry(&raced_library, "raced-mod"));

        let store = StoreSnapshot::open(&library, &loadout).unwrap();
        assert_eq!(
            store.loadout().entries,
            vec![LoadoutEntry {
                id: "raced-mod".into(),
                enabled: false,
            }]
        );
        drop(store);
        assert_eq!(
            loadout::load(&loadout).unwrap().entries,
            vec![LoadoutEntry {
                id: "raced-mod".into(),
                enabled: false,
            }]
        );
    }

    #[cfg(unix)]
    #[test]
    fn unix_parent_alias_swap_during_lock_is_refused_without_touching_decoy() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("library");
        write_entry(&library, "a");
        let parent = temp.path().join("store");
        let retained = temp.path().join("retained");
        let decoy = temp.path().join("decoy");
        fs::create_dir(&parent).unwrap();
        fs::create_dir(&decoy).unwrap();
        let requested = parent.join("loadout.json");
        let decoy_loadout = decoy.join("loadout.json");
        write_loadout(&requested, &[("a", false)]);
        write_loadout(&decoy_loadout, &[("decoy", true)]);
        let retained_before = fs::read(&requested).unwrap();
        let decoy_before = fs::read(&decoy_loadout).unwrap();

        let hook_parent = parent.clone();
        let hook_retained = retained.clone();
        let hook_decoy = decoy.clone();
        inject_parent_lock_race(move || {
            fs::rename(&hook_parent, &hook_retained).unwrap();
            std::os::unix::fs::symlink(&hook_decoy, &hook_parent).unwrap();
        });

        let error = match StoreSnapshot::open(&library, &requested) {
            Ok(_) => panic!("store accepted a parent alias swapped while locking"),
            Err(error) => error.to_string(),
        };
        assert!(
            error.contains("opening manager store directory")
                || error.contains("opening named Manager mutation root")
                || error.contains("changed filesystem identity"),
            "{error}"
        );
        assert_eq!(
            fs::read(retained.join("loadout.json")).unwrap(),
            retained_before
        );
        assert_eq!(fs::read(&decoy_loadout).unwrap(), decoy_before);
    }

    #[cfg(unix)]
    #[test]
    fn unix_store_io_refuses_after_parent_path_replacement_and_preserves_both_roots() {
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().join("store");
        fs::create_dir(&parent).unwrap();
        let requested = parent.join("loadout.json");
        write_loadout(&requested, &[("old", false)]);
        let _process = manager_process_lock();
        let (parent_path, canonical) = canonical_parent(&requested).unwrap();
        let prepared =
            prepare_existing_manager_root_lock(&parent_path, "test manager Store parent", true)
                .unwrap();
        let kernel = KernelStoreLock::new(
            acquire_manager_root_locks(vec![prepared])
                .unwrap()
                .remove(0),
        );
        assert!(!parent.join(STORE_LOCK_FILE).exists());

        let retained = temp.path().join("retained");
        fs::rename(&parent, &retained).unwrap();
        fs::create_dir(&parent).unwrap();
        write_loadout(&requested, &[("decoy", true)]);
        let retained_bytes = fs::read(retained.join("loadout.json")).unwrap();
        let replacement_bytes = fs::read(&requested).unwrap();

        let error = kernel
            .save(
                &canonical,
                &Loadout {
                    format: 1,
                    entries: vec![LoadoutEntry {
                        id: "new".into(),
                        enabled: true,
                    }],
                },
            )
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("identity") || error.contains("named"),
            "{error}"
        );
        assert_eq!(
            fs::read(retained.join("loadout.json")).unwrap(),
            retained_bytes
        );
        assert_eq!(fs::read(&requested).unwrap(), replacement_bytes);
    }
}
