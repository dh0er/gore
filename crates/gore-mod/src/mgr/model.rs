//! Library-entry metadata: the cross-tool contract describing one imported mod.
//!
//! Every mod in the manager library is a normalized dir with a [`META_FILE`] sidecar holding a
//! [`ModEntryMeta`]. These shapes are a locked contract shared across the manager UI, CLI and
//! engine — do not rename fields or variants.

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Sidecar filename inside each library mod dir.
pub const META_FILE: &str = "gore-manager-meta.json";
const LIBRARY_META_MAX_BYTES: u64 = 16 * 1024 * 1024;

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileIdentity {
    volume: u64,
    id: [u8; 16],
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileIdentity {
    device: u64,
    inode: u64,
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileRevision {
    last_write_time: i64,
    change_time: i64,
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileRevision {
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[derive(Debug)]
struct DirectoryAnchor {
    file: std::fs::File,
    final_path: PathBuf,
    identity: FileIdentity,
}

/// A directory proven by its opened handle to be a real, non-link filesystem object.  Keeping the
/// handle alive prevents replacement on Windows (the handle deliberately does not share DELETE),
/// and supplies the parent fd for Unix `openat(O_NOFOLLOW)` traversal.
#[derive(Debug, Clone)]
pub(crate) struct SecureDirectory {
    anchor: Arc<DirectoryAnchor>,
    parents: Vec<Arc<DirectoryAnchor>>,
}

/// Directory handle opened with rename-compatible sharing. It retains the parent's filesystem
/// identity across a Windows rename while allowing that rename to proceed.
#[derive(Debug)]
pub(crate) struct RenameDirectoryGuard {
    file: std::fs::File,
    final_path: PathBuf,
    identity: FileIdentity,
}

impl RenameDirectoryGuard {
    pub(crate) fn path(&self) -> &Path {
        let _retained_handle = &self.file;
        &self.final_path
    }

    pub(crate) fn identity(&self) -> FileIdentity {
        self.identity
    }
}

/// A regular file whose no-follow handle, stable identity, final path, and length were captured in
/// one operation. Reads/copies use this exact handle rather than reopening the checked pathname.
#[derive(Debug)]
pub(crate) struct SecureFile {
    pub(crate) file: std::fs::File,
    final_path: PathBuf,
    identity: FileIdentity,
    revision: FileRevision,
    len: u64,
    _parents: Vec<Arc<DirectoryAnchor>>,
}

#[derive(Debug)]
pub(crate) enum SecureNode {
    File(SecureFile),
    Directory(SecureDirectory),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TreeSnapshotLimits {
    pub(crate) max_entries: usize,
    pub(crate) max_path_bytes: usize,
    pub(crate) max_depth: usize,
    pub(crate) max_file_bytes: u64,
    pub(crate) max_total_bytes: u64,
}

#[derive(Debug)]
pub(crate) struct PayloadTreeSnapshot {
    temp: tempfile::TempDir,
    root: PathBuf,
    entries: usize,
    bytes: u64,
}

impl PayloadTreeSnapshot {
    pub(crate) fn path(&self) -> &Path {
        &self.root
    }

    pub(crate) fn bundle_root(&self) -> &Path {
        self.temp.path()
    }

    pub(crate) fn entries(&self) -> usize {
        self.entries
    }

    pub(crate) fn bytes(&self) -> u64 {
        self.bytes
    }
}

#[derive(Debug)]
struct OpenedNode {
    file: std::fs::File,
    final_path: PathBuf,
    identity: FileIdentity,
    metadata: std::fs::Metadata,
}

impl SecureFile {
    pub(crate) fn len(&self) -> u64 {
        self.len
    }

    pub(crate) fn path(&self) -> &Path {
        &self.final_path
    }

    pub(crate) fn verify_len(&self, expected: u64, label: &str) -> crate::Result<()> {
        let observed = self
            .file
            .metadata()
            .map_err(crate::io(&format!("rechecking opened {label} metadata")))?
            .len();
        let current_identity = identity_from_open_file(&self.file, label)?;
        let current_revision = revision_from_open_file(&self.file, label)?;
        if observed != expected
            || current_identity != self.identity
            || current_revision != self.revision
        {
            return Err(crate::ModError::Other(format!(
                "{label} changed identity/size/content revision through its opened handle: expected {expected}, observed {observed}: {}",
                self.final_path.display()
            )));
        }
        Ok(())
    }
}

#[cfg(windows)]
fn identity_from_open_file(file: &std::fs::File, label: &str) -> crate::Result<FileIdentity> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FileIdInfo, GetFileInformationByHandleEx, FILE_ID_INFO,
    };
    let mut id = FILE_ID_INFO::default();
    let ok = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileIdInfo,
            (&mut id as *mut FILE_ID_INFO).cast(),
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    };
    if ok == 0 {
        return Err(crate::io(&format!("querying opened {label} identity"))(
            std::io::Error::last_os_error(),
        ));
    }
    Ok(FileIdentity {
        volume: id.VolumeSerialNumber,
        id: id.FileId.Identifier,
    })
}

#[cfg(windows)]
fn revision_from_open_file(file: &std::fs::File, label: &str) -> crate::Result<FileRevision> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FileBasicInfo, GetFileInformationByHandleEx, FILE_BASIC_INFO,
    };

    let mut basic = FILE_BASIC_INFO::default();
    let ok = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileBasicInfo,
            (&mut basic as *mut FILE_BASIC_INFO).cast(),
            std::mem::size_of::<FILE_BASIC_INFO>() as u32,
        )
    };
    if ok == 0 {
        return Err(crate::io(&format!(
            "querying opened {label} content revision"
        ))(std::io::Error::last_os_error()));
    }
    Ok(FileRevision {
        last_write_time: basic.LastWriteTime,
        change_time: basic.ChangeTime,
    })
}

#[cfg(unix)]
fn identity_from_open_file(file: &std::fs::File, label: &str) -> crate::Result<FileIdentity> {
    use std::os::unix::fs::MetadataExt as _;
    let metadata = file
        .metadata()
        .map_err(crate::io(&format!("querying opened {label} identity")))?;
    Ok(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[cfg(unix)]
fn revision_from_open_file(file: &std::fs::File, label: &str) -> crate::Result<FileRevision> {
    use std::os::unix::fs::MetadataExt as _;
    let metadata = file.metadata().map_err(crate::io(&format!(
        "querying opened {label} content revision"
    )))?;
    Ok(FileRevision {
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    })
}

impl SecureDirectory {
    pub(crate) fn path(&self) -> &Path {
        &self.anchor.final_path
    }

    pub(crate) fn identity(&self) -> FileIdentity {
        // Reading the retained handle here is intentional: on Windows its no-DELETE share mode is
        // itself part of the identity guarantee even though child opens use the final path.
        let _retained_handle = &self.anchor.file;
        self.anchor.identity
    }

    /// Consume the no-delete traversal anchor after opening the same directory with sharing that
    /// permits an atomic child rename. The returned handle preserves identity while the original
    /// Windows handle (and its no-delete share restriction) is closed.
    pub(crate) fn into_rename_guard(self, label: &str) -> crate::Result<RenameDirectoryGuard> {
        let guard = open_directory_rename_compatible(&self.anchor.final_path, label)?;
        if guard.identity != self.anchor.identity {
            return Err(crate::ModError::Other(format!(
                "{label} changed identity while preparing atomic rename: {}",
                self.anchor.final_path.display()
            )));
        }
        Ok(guard)
    }

    pub(crate) fn sync_after_mutation(&self, _label: &str) -> crate::Result<()> {
        #[cfg(unix)]
        {
            self.anchor
                .file
                .sync_all()
                .map_err(crate::io(&format!("syncing opened {_label} directory")))?;
        }
        #[cfg(windows)]
        {
            // Windows exposes no portable directory FlushFileBuffers operation. Callers publish
            // replacement renames with MOVEFILE_WRITE_THROUGH; retaining this anchor still keeps
            // the validated parent from being renamed while its child mutation is performed.
            let _retained_anchor = &self.anchor.file;
        }
        Ok(())
    }

    pub(crate) fn read_dir(&self, label: &str) -> crate::Result<std::fs::ReadDir> {
        let path = secure_directory_enumeration_path(&self.anchor)?;
        std::fs::read_dir(&path).map_err(crate::io(&format!(
            "reading opened {label} directory {}",
            self.anchor.final_path.display()
        )))
    }

    pub(crate) fn contains_child(
        &self,
        name: &std::ffi::OsStr,
        label: &str,
    ) -> crate::Result<bool> {
        validate_plain_component(name, label)?;
        for entry in self.read_dir(label)? {
            let entry = entry.map_err(crate::io(&format!("reading {label} directory entry")))?;
            if child_names_equal(&entry.file_name(), name) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(crate) fn open_child(
        &self,
        name: &std::ffi::OsStr,
        label: &str,
    ) -> crate::Result<SecureNode> {
        validate_plain_component(name, label)?;
        let opened = open_child_node(&self.anchor, name, label)?;
        if opened.metadata.is_dir() {
            let mut parents = self.parents.clone();
            parents.push(self.anchor.clone());
            Ok(SecureNode::Directory(SecureDirectory {
                anchor: Arc::new(DirectoryAnchor {
                    file: opened.file,
                    final_path: opened.final_path,
                    identity: opened.identity,
                }),
                parents,
            }))
        } else if opened.metadata.is_file() {
            let mut parents = self.parents.clone();
            parents.push(self.anchor.clone());
            let revision = revision_from_open_file(&opened.file, label)?;
            Ok(SecureNode::File(SecureFile {
                len: opened.metadata.len(),
                file: opened.file,
                final_path: opened.final_path,
                identity: opened.identity,
                revision,
                _parents: parents,
            }))
        } else {
            Err(crate::ModError::Other(format!(
                "{label} is neither a regular file nor directory: {}",
                opened.final_path.display()
            )))
        }
    }

    pub(crate) fn open_optional_child_directory(
        &self,
        name: &std::ffi::OsStr,
        label: &str,
    ) -> crate::Result<Option<SecureDirectory>> {
        validate_plain_component(name, label)?;
        if !self.contains_child(name, label)? {
            return Ok(None);
        }
        match self.open_child(name, label)? {
            SecureNode::Directory(directory) => Ok(Some(directory)),
            SecureNode::File(file) => Err(crate::ModError::Other(format!(
                "{label} must be a real directory: {}",
                file.path().display()
            ))),
        }
    }

    pub(crate) fn open_relative_file(&self, rel: &Path, label: &str) -> crate::Result<SecureFile> {
        let components: Vec<_> = rel.components().collect();
        if components.is_empty() {
            return Err(crate::ModError::Other(format!("{label} path is empty")));
        }
        let mut directory = self.clone();
        for (index, component) in components.iter().enumerate() {
            let std::path::Component::Normal(name) = component else {
                return Err(crate::ModError::Other(format!(
                    "{label} path is not plain relative syntax: {}",
                    rel.display()
                )));
            };
            match directory.open_child(name, label)? {
                SecureNode::File(file) if index + 1 == components.len() => return Ok(file),
                SecureNode::Directory(child) if index + 1 < components.len() => {
                    directory = child;
                }
                SecureNode::Directory(child) => {
                    return Err(crate::ModError::Other(format!(
                        "{label} must be a regular file: {}",
                        child.path().display()
                    )))
                }
                SecureNode::File(file) => {
                    return Err(crate::ModError::Other(format!(
                        "{label} path crosses a regular file: {}",
                        file.path().display()
                    )))
                }
            }
        }
        unreachable!("non-empty relative path returns its final file")
    }

    /// Create one direct child directory beneath this retained, no-follow parent and immediately
    /// bind the new child to its own handle. On Unix the creation itself is `mkdirat`; on Windows
    /// the retained parent handle denies rename/delete while `CreateDirectory` and the no-follow
    /// child open run. A hostile replacement can therefore at worst substitute another real child
    /// inside this parent; a junction/reparse substitution is rejected before it can be used.
    pub(crate) fn create_child_directory_new(
        &self,
        name: &std::ffi::OsStr,
        label: &str,
    ) -> crate::Result<SecureDirectory> {
        self.try_create_child_directory_new(name, label)?
            .ok_or_else(|| {
                crate::ModError::Other(format!(
                    "{label} already exists below retained parent {}",
                    self.anchor.final_path.display()
                ))
            })
    }

    /// Structured create-new form used by unique-name retry loops. `None` means another creator
    /// won the direct-child name; all other failures remain hard errors.
    pub(crate) fn try_create_child_directory_new(
        &self,
        name: &std::ffi::OsStr,
        label: &str,
    ) -> crate::Result<Option<SecureDirectory>> {
        validate_plain_component(name, label)?;
        run_create_child_directory_precreate_race_hook(&self.anchor.final_path.join(name));
        match create_child_directory_new(&self.anchor, name) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => return Ok(None),
            Err(error) => {
                return Err(crate::io(&format!(
                    "creating {label} relative to retained parent {}",
                    self.anchor.final_path.display()
                ))(error))
            }
        }
        run_create_child_directory_race_hook(&self.anchor.final_path.join(name));
        match self.open_child(name, label) {
            Ok(SecureNode::Directory(directory)) => {
                self.sync_after_mutation(label)?;
                Ok(Some(directory))
            }
            Ok(SecureNode::File(file)) => Err(crate::ModError::Other(format!(
                "new {label} was replaced by a regular file: {}",
                file.path().display()
            ))),
            Err(error) => Err(error),
        }
    }

    /// Create one direct regular child with exclusive create semantics relative to this retained
    /// parent. The returned file handle is the object callers write and sync; no checked pathname is
    /// reopened for the write.
    pub(crate) fn create_child_file_new(
        &self,
        name: &std::ffi::OsStr,
        label: &str,
    ) -> crate::Result<(std::fs::File, FileIdentity)> {
        validate_plain_component(name, label)?;
        let file = create_child_file_new(&self.anchor, name, label)?;
        let identity = identity_from_open_file(&file, label)?;
        Ok((file, identity))
    }

    /// Remove one direct child relative to this retained parent. Cleanup deliberately never walks
    /// a pathname assembled from mutable nested directories, so a junction swap cannot redirect a
    /// failed-write cleanup outside the directory handles created by the writer.
    pub(crate) fn remove_child_file(
        &self,
        name: &std::ffi::OsStr,
        label: &str,
    ) -> crate::Result<()> {
        validate_plain_component(name, label)?;
        remove_child(&self.anchor, name, false, label)?;
        self.sync_after_mutation(label)
    }

    pub(crate) fn remove_child_directory(
        &self,
        name: &std::ffi::OsStr,
        label: &str,
    ) -> crate::Result<()> {
        validate_plain_component(name, label)?;
        remove_child(&self.anchor, name, true, label)?;
        self.sync_after_mutation(label)
    }

    /// Remove one direct regular child only when its current no-follow identity is the exact
    /// object created by the caller. This makes best-effort cleanup refuse a name that another
    /// process replaced after creation.
    pub(crate) fn remove_child_file_if_identity(
        &self,
        name: &std::ffi::OsStr,
        expected: FileIdentity,
        label: &str,
    ) -> crate::Result<()> {
        let actual = match self.open_child(name, label)? {
            SecureNode::File(file) => file,
            SecureNode::Directory(directory) => {
                return Err(crate::ModError::Other(format!(
                "refusing to remove replaced {label}: expected a regular file, found directory {}",
                directory.path().display()
            )))
            }
        };
        if actual.identity != expected {
            return Err(crate::ModError::Other(format!(
                "refusing to remove replaced {label}: filesystem identity changed at {}",
                actual.path().display()
            )));
        }
        drop(actual);
        self.remove_child_file(name, label)
    }

    /// Directory counterpart to [`Self::remove_child_file_if_identity`]. Removal remains
    /// non-recursive and relative to this retained parent.
    pub(crate) fn remove_child_directory_if_identity(
        &self,
        name: &std::ffi::OsStr,
        expected: FileIdentity,
        label: &str,
    ) -> crate::Result<()> {
        let actual = match self.open_child(name, label)? {
            SecureNode::Directory(directory) => directory,
            SecureNode::File(file) => {
                return Err(crate::ModError::Other(format!(
                    "refusing to remove replaced {label}: expected a directory, found file {}",
                    file.path().display()
                )))
            }
        };
        if actual.identity() != expected {
            return Err(crate::ModError::Other(format!(
                "refusing to remove replaced {label}: filesystem identity changed at {}",
                actual.path().display()
            )));
        }
        drop(actual);
        self.remove_child_directory(name, label)
    }
}

/// Open an existing absolute directory component-by-component. Unlike `canonicalize` followed by
/// one final no-follow open, this never permits an intermediate symlink/junction to choose a new
/// subtree between validation and binding.
pub(crate) fn open_directory_chain_nofollow(
    path: &Path,
    label: &str,
) -> crate::Result<SecureDirectory> {
    if !path.is_absolute() {
        return Err(crate::ModError::Other(format!(
            "{label} must be absolute for no-follow traversal: {}",
            path.display()
        )));
    }

    let mut root = PathBuf::new();
    let mut children = Vec::new();
    let mut saw_child = false;
    for component in path.components() {
        match component {
            std::path::Component::Prefix(_) | std::path::Component::RootDir if !saw_child => {
                root.push(component.as_os_str());
            }
            std::path::Component::Normal(name) => {
                saw_child = true;
                children.push(name.to_os_string());
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir
            | std::path::Component::Prefix(_)
            | std::path::Component::RootDir => {
                return Err(crate::ModError::Other(format!(
                    "{label} contains non-plain absolute traversal: {}",
                    path.display()
                )));
            }
        }
    }
    if root.as_os_str().is_empty() {
        return Err(crate::ModError::Other(format!(
            "{label} has no absolute filesystem root: {}",
            path.display()
        )));
    }

    let mut directory = open_directory_nofollow(&root, label)?;
    for name in children {
        directory = match directory.open_child(&name, label)? {
            SecureNode::Directory(child) => child,
            SecureNode::File(file) => {
                return Err(crate::ModError::Other(format!(
                    "{label} crosses a regular file: {}",
                    file.path().display()
                )))
            }
        };
    }
    Ok(directory)
}

#[cfg(windows)]
fn create_child_directory_new(
    parent: &DirectoryAnchor,
    name: &std::ffi::OsStr,
) -> std::io::Result<()> {
    let child = parent.final_path.join(name);
    std::fs::create_dir(&child)
}

#[cfg(unix)]
fn create_child_directory_new(
    parent: &DirectoryAnchor,
    name: &std::ffi::OsStr,
) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt as _;
    use std::os::unix::io::AsRawFd as _;

    let name = std::ffi::CString::new(name.as_bytes())
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let result = unsafe { libc::mkdirat(parent.file.as_raw_fd(), name.as_ptr(), 0o700) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(any(windows, unix)))]
fn create_child_directory_new(
    parent: &DirectoryAnchor,
    name: &std::ffi::OsStr,
) -> std::io::Result<()> {
    std::fs::create_dir(parent.final_path.join(name))
}

#[cfg(windows)]
fn create_child_file_new(
    parent: &DirectoryAnchor,
    name: &std::ffi::OsStr,
    label: &str,
) -> crate::Result<std::fs::File> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ};

    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(parent.final_path.join(name))
        .map_err(crate::io(&format!(
            "creating {label} relative to retained parent {}",
            parent.final_path.display()
        )))
}

#[cfg(unix)]
fn create_child_file_new(
    parent: &DirectoryAnchor,
    name: &std::ffi::OsStr,
    label: &str,
) -> crate::Result<std::fs::File> {
    use std::os::unix::ffi::OsStrExt as _;
    use std::os::unix::io::{AsRawFd as _, FromRawFd as _};

    let name = std::ffi::CString::new(name.as_bytes())
        .map_err(|_| crate::ModError::Other(format!("{label} child name contains NUL")))?;
    let fd = unsafe {
        libc::openat(
            parent.file.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err(crate::io(&format!(
            "creating {label} relative to retained parent {}",
            parent.final_path.display()
        ))(std::io::Error::last_os_error()));
    }
    Ok(unsafe { std::fs::File::from_raw_fd(fd) })
}

#[cfg(not(any(windows, unix)))]
fn create_child_file_new(
    parent: &DirectoryAnchor,
    name: &std::ffi::OsStr,
    label: &str,
) -> crate::Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(parent.final_path.join(name))
        .map_err(crate::io(&format!(
            "creating {label} relative to retained parent {}",
            parent.final_path.display()
        )))
}

#[cfg(unix)]
fn remove_child(
    parent: &DirectoryAnchor,
    name: &std::ffi::OsStr,
    directory: bool,
    label: &str,
) -> crate::Result<()> {
    use std::os::unix::ffi::OsStrExt as _;
    use std::os::unix::io::AsRawFd as _;

    let name = std::ffi::CString::new(name.as_bytes())
        .map_err(|_| crate::ModError::Other(format!("{label} child name contains NUL")))?;
    let flags = if directory { libc::AT_REMOVEDIR } else { 0 };
    let result = unsafe { libc::unlinkat(parent.file.as_raw_fd(), name.as_ptr(), flags) };
    if result == 0 {
        Ok(())
    } else {
        Err(crate::io(&format!(
            "removing {label} relative to retained parent {}",
            parent.final_path.display()
        ))(std::io::Error::last_os_error()))
    }
}

#[cfg(not(unix))]
fn remove_child(
    parent: &DirectoryAnchor,
    name: &std::ffi::OsStr,
    directory: bool,
    label: &str,
) -> crate::Result<()> {
    let child = parent.final_path.join(name);
    let result = if directory {
        std::fs::remove_dir(&child)
    } else {
        std::fs::remove_file(&child)
    };
    result.map_err(crate::io(&format!(
        "removing {label} relative to retained parent {}",
        parent.final_path.display()
    )))
}

pub(crate) fn open_file_nofollow(path: &Path, label: &str) -> crate::Result<SecureFile> {
    let opened = open_absolute_node(path, label)?;
    if !opened.metadata.is_file() {
        return Err(crate::ModError::Other(format!(
            "{label} is not a regular file: {}",
            opened.final_path.display()
        )));
    }
    let revision = revision_from_open_file(&opened.file, label)?;
    Ok(SecureFile {
        len: opened.metadata.len(),
        file: opened.file,
        final_path: opened.final_path,
        identity: opened.identity,
        revision,
        _parents: Vec::new(),
    })
}

#[cfg(windows)]
fn open_directory_rename_compatible(
    path: &Path,
    label: &str,
) -> crate::Result<RenameDirectoryGuard> {
    use std::os::windows::fs::{MetadataExt as _, OpenOptionsExt as _};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };

    let file = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .map_err(crate::io(&format!(
            "opening {label} with rename-compatible sharing {}",
            path.display()
        )))?;
    let metadata = file
        .metadata()
        .map_err(crate::io(&format!("reading opened {label} metadata")))?;
    if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(crate::ModError::Other(format!(
            "{label} is not a real non-reparse directory: {}",
            path.display()
        )));
    }
    let identity = identity_from_open_file(&file, label)?;
    Ok(RenameDirectoryGuard {
        file,
        final_path: path.to_path_buf(),
        identity,
    })
}

#[cfg(not(windows))]
fn open_directory_rename_compatible(
    path: &Path,
    label: &str,
) -> crate::Result<RenameDirectoryGuard> {
    let directory = open_directory_nofollow(path, label)?;
    let file = directory
        .anchor
        .file
        .try_clone()
        .map_err(crate::io(&format!(
            "cloning opened {label} directory handle"
        )))?;
    Ok(RenameDirectoryGuard {
        file,
        final_path: directory.anchor.final_path.clone(),
        identity: directory.anchor.identity,
    })
}

pub(crate) fn open_directory_nofollow(path: &Path, label: &str) -> crate::Result<SecureDirectory> {
    let opened = open_absolute_node(path, label)?;
    if !opened.metadata.is_dir() {
        return Err(crate::ModError::Other(format!(
            "{label} is not a real directory: {}",
            opened.final_path.display()
        )));
    }
    Ok(SecureDirectory {
        anchor: Arc::new(DirectoryAnchor {
            file: opened.file,
            final_path: opened.final_path,
            identity: opened.identity,
        }),
        parents: Vec::new(),
    })
}

fn validate_plain_component(name: &std::ffi::OsStr, label: &str) -> crate::Result<()> {
    let mut components = Path::new(name).components();
    if !matches!(components.next(), Some(std::path::Component::Normal(_)))
        || components.next().is_some()
    {
        return Err(crate::ModError::Other(format!(
            "{label} child name is not one plain path component: {name:?}"
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn child_names_equal(left: &std::ffi::OsStr, right: &std::ffi::OsStr) -> bool {
    handle_paths_equal(Path::new(left), Path::new(right))
}

#[cfg(unix)]
fn child_names_equal(left: &std::ffi::OsStr, right: &std::ffi::OsStr) -> bool {
    left == right
}

#[cfg(test)]
type RaceHook = Option<Box<dyn FnOnce(&Path)>>;

#[cfg(test)]
thread_local! {
    static OPEN_CHILD_RACE_HOOK: std::cell::RefCell<RaceHook> = std::cell::RefCell::new(None);
    static TREE_ENTRY_RACE_HOOK: std::cell::RefCell<RaceHook> = std::cell::RefCell::new(None);
    static CREATE_CHILD_DIRECTORY_RACE_HOOK: std::cell::RefCell<RaceHook> =
        std::cell::RefCell::new(None);
    static CREATE_CHILD_DIRECTORY_PRECREATE_RACE_HOOK: std::cell::RefCell<RaceHook> =
        std::cell::RefCell::new(None);
}

#[cfg(test)]
fn run_open_child_race_hook(path: &Path) {
    OPEN_CHILD_RACE_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook(path);
        }
    });
}

#[cfg(not(test))]
fn run_open_child_race_hook(_path: &Path) {}

#[cfg(test)]
pub(crate) fn inject_open_child_race(hook: impl FnOnce(&Path) + 'static) {
    OPEN_CHILD_RACE_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(test)]
fn run_create_child_directory_race_hook(path: &Path) {
    CREATE_CHILD_DIRECTORY_RACE_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook(path);
        }
    });
}

#[cfg(not(test))]
fn run_create_child_directory_race_hook(_path: &Path) {}

#[cfg(test)]
pub(crate) fn inject_create_child_directory_race(hook: impl FnOnce(&Path) + 'static) {
    CREATE_CHILD_DIRECTORY_RACE_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(test)]
fn run_create_child_directory_precreate_race_hook(path: &Path) {
    CREATE_CHILD_DIRECTORY_PRECREATE_RACE_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook(path);
        }
    });
}

#[cfg(not(test))]
fn run_create_child_directory_precreate_race_hook(_path: &Path) {}

#[cfg(test)]
pub(crate) fn inject_create_child_directory_precreate_race(hook: impl FnOnce(&Path) + 'static) {
    CREATE_CHILD_DIRECTORY_PRECREATE_RACE_HOOK
        .with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(test)]
fn run_tree_entry_race_hook(path: &Path) {
    TREE_ENTRY_RACE_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook(path);
        }
    });
}

#[cfg(not(test))]
fn run_tree_entry_race_hook(_path: &Path) {}

#[cfg(test)]
fn inject_tree_entry_race(hook: impl FnOnce(&Path) + 'static) {
    TREE_ENTRY_RACE_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(windows)]
fn open_absolute_node(path: &Path, label: &str) -> crate::Result<OpenedNode> {
    use std::os::windows::ffi::OsStringExt as _;
    use std::os::windows::fs::OpenOptionsExt as _;
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FileAttributeTagInfo, FileIdInfo, GetFileInformationByHandleEx, GetFinalPathNameByHandleW,
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_INFO, FILE_NAME_NORMALIZED, FILE_SHARE_READ,
        VOLUME_NAME_DOS,
    };

    let file = std::fs::OpenOptions::new()
        .read(true)
        // Excluding FILE_SHARE_DELETE prevents a validated object from being renamed/replaced
        // while this handle (or a retained parent anchor) participates in traversal.
        // Do not share WRITE or DELETE: the bytes and path must stay immutable for the lifetime of
        // the validated handle. A writer that was already open is rejected by CreateFile's
        // symmetric share checks; the revision checks below remain defense in depth and provide the
        // corresponding race detection on Unix, whose open API has no mandatory share modes.
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .map_err(crate::io(&format!(
            "opening {label} without following reparse points {}",
            path.display()
        )))?;
    let handle = file.as_raw_handle();
    let mut attributes = FILE_ATTRIBUTE_TAG_INFO::default();
    let attribute_ok = unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileAttributeTagInfo,
            (&mut attributes as *mut FILE_ATTRIBUTE_TAG_INFO).cast(),
            std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    };
    if attribute_ok == 0 {
        return Err(crate::io(&format!(
            "querying opened {label} reparse attributes {}",
            path.display()
        ))(std::io::Error::last_os_error()));
    }
    if attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(crate::ModError::Other(format!(
            "{label} is a symbolic link or reparse point: {}",
            path.display()
        )));
    }

    let mut id = FILE_ID_INFO::default();
    let id_ok = unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileIdInfo,
            (&mut id as *mut FILE_ID_INFO).cast(),
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    };
    if id_ok == 0 {
        return Err(crate::io(&format!(
            "querying opened {label} file identity {}",
            path.display()
        ))(std::io::Error::last_os_error()));
    }

    let mut path_buffer = vec![0u16; 512];
    let final_path = loop {
        let length = unsafe {
            GetFinalPathNameByHandleW(
                handle,
                path_buffer.as_mut_ptr(),
                path_buffer.len() as u32,
                FILE_NAME_NORMALIZED | VOLUME_NAME_DOS,
            )
        };
        if length == 0 {
            return Err(crate::io(&format!(
                "querying opened {label} final path {}",
                path.display()
            ))(std::io::Error::last_os_error()));
        }
        if (length as usize) < path_buffer.len() {
            path_buffer.truncate(length as usize);
            break PathBuf::from(std::ffi::OsString::from_wide(&path_buffer));
        }
        path_buffer.resize(length as usize + 1, 0);
    };
    let metadata = file
        .metadata()
        .map_err(crate::io(&format!("reading opened {label} metadata")))?;
    Ok(OpenedNode {
        file,
        final_path,
        identity: FileIdentity {
            volume: id.VolumeSerialNumber,
            id: id.FileId.Identifier,
        },
        metadata,
    })
}

#[cfg(windows)]
fn open_child_node(
    parent: &DirectoryAnchor,
    name: &std::ffi::OsStr,
    label: &str,
) -> crate::Result<OpenedNode> {
    let child = parent.final_path.join(name);
    run_open_child_race_hook(&child);
    let opened = open_absolute_node(&child, label)?;
    if !handle_path_is_direct_child(&parent.final_path, &opened.final_path) {
        return Err(crate::ModError::Other(format!(
            "opened {label} escaped its validated parent: {} -> {}",
            child.display(),
            opened.final_path.display()
        )));
    }
    Ok(opened)
}

#[cfg(windows)]
fn normalized_handle_path(path: &Path) -> String {
    let mut text = path.to_string_lossy().replace('/', "\\");
    if let Some(stripped) = text.strip_prefix("\\\\?\\UNC\\") {
        text = format!("\\\\{stripped}");
    } else if let Some(stripped) = text.strip_prefix("\\\\?\\") {
        text = stripped.to_owned();
    }
    text.trim_end_matches('\\').to_lowercase()
}

#[cfg(windows)]
fn handle_paths_equal(left: &Path, right: &Path) -> bool {
    normalized_handle_path(left) == normalized_handle_path(right)
}

#[cfg(windows)]
fn handle_path_is_direct_child(parent: &Path, child: &Path) -> bool {
    child
        .parent()
        .is_some_and(|actual| handle_paths_equal(parent, actual))
}

#[cfg(windows)]
fn secure_directory_enumeration_path(anchor: &DirectoryAnchor) -> crate::Result<PathBuf> {
    Ok(anchor.final_path.clone())
}

#[cfg(unix)]
fn open_absolute_node(path: &Path, label: &str) -> crate::Result<OpenedNode> {
    use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _};
    use std::os::unix::io::AsRawFd as _;

    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(crate::io(&format!(
            "opening {label} with O_NOFOLLOW {}",
            path.display()
        )))?;
    let metadata = file
        .metadata()
        .map_err(crate::io(&format!("reading opened {label} metadata")))?;
    let identity = FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    };
    let fd_path = PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()));
    let final_path = std::fs::read_link(&fd_path)
        .or_else(|_| std::fs::canonicalize(path))
        .map_err(crate::io(&format!("resolving opened {label} handle path")))?;
    Ok(OpenedNode {
        file,
        final_path,
        identity,
        metadata,
    })
}

#[cfg(unix)]
fn open_child_node(
    parent: &DirectoryAnchor,
    name: &std::ffi::OsStr,
    label: &str,
) -> crate::Result<OpenedNode> {
    use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};
    use std::os::unix::fs::MetadataExt as _;
    use std::os::unix::io::{AsRawFd as _, FromRawFd as _};

    let raw_name = name.as_bytes();
    let c_name = std::ffi::CString::new(raw_name)
        .map_err(|_| crate::ModError::Other(format!("{label} child name contains NUL")))?;
    run_open_child_race_hook(&parent.final_path.join(name));
    let fd = unsafe {
        libc::openat(
            parent.file.as_raw_fd(),
            c_name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(crate::io(&format!(
            "opening {label} relative to validated parent {}",
            parent.final_path.display()
        ))(std::io::Error::last_os_error()));
    }
    let file = unsafe { std::fs::File::from_raw_fd(fd) };
    let metadata = file
        .metadata()
        .map_err(crate::io(&format!("reading opened {label} metadata")))?;
    Ok(OpenedNode {
        file,
        final_path: parent
            .final_path
            .join(std::ffi::OsString::from_vec(raw_name.to_vec())),
        identity: FileIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
        },
        metadata,
    })
}

#[cfg(unix)]
fn secure_directory_enumeration_path(anchor: &DirectoryAnchor) -> crate::Result<PathBuf> {
    use std::os::unix::io::AsRawFd as _;
    let proc_path = PathBuf::from(format!("/proc/self/fd/{}", anchor.file.as_raw_fd()));
    if proc_path.exists() {
        return Ok(proc_path);
    }
    let dev_path = PathBuf::from(format!("/dev/fd/{}", anchor.file.as_raw_fd()));
    if dev_path.exists() {
        return Ok(dev_path);
    }
    Err(crate::ModError::Other(
        "secure directory enumeration requires /proc/self/fd or /dev/fd on this Unix platform"
            .into(),
    ))
}

/// Canonical manager-library root used to prove that entry and payload paths stay inside the
/// configured library. The library root itself may be reached through a user-configured alias;
/// entries and payloads may not introduce further symbolic links/reparse points.
#[derive(Debug, Clone)]
pub(crate) struct LibraryRoot {
    directory: SecureDirectory,
}

/// One validated, direct child of a [`LibraryRoot`].
#[derive(Debug, Clone)]
pub(crate) struct LibraryEntry {
    library: SecureDirectory,
    directory: SecureDirectory,
    id: String,
}

impl LibraryRoot {
    pub(crate) fn open(path: &Path) -> crate::Result<Self> {
        // The configured library root itself may be an intentional alias. Resolve that one alias,
        // then anchor the real target by handle; every entry/payload below it is opened no-follow.
        let canonical = std::fs::canonicalize(path).map_err(crate::io(&format!(
            "resolving manager library {}",
            path.display()
        )))?;
        let directory = open_directory_nofollow(&canonical, "manager library")?;
        Ok(Self { directory })
    }

    /// Resolve `id` as one real, direct library child. A safe lexical component is necessary but
    /// not sufficient: the child may have been replaced by a symlink/junction after import.
    pub(crate) fn entry(&self, id: &str) -> crate::Result<LibraryEntry> {
        validate_library_id(id)?;
        let directory = match self
            .directory
            .open_child(std::ffi::OsStr::new(id), "library entry")?
        {
            SecureNode::Directory(directory) => directory,
            SecureNode::File(file) => {
                return Err(crate::ModError::Other(format!(
                    "library entry must be a real directory: {}",
                    file.path().display()
                )))
            }
        };
        Ok(LibraryEntry {
            library: self.directory.clone(),
            directory,
            id: id.to_owned(),
        })
    }

    pub(crate) fn sync_after_mutation(&self) -> crate::Result<()> {
        self.directory.sync_after_mutation("manager library")
    }
}

impl LibraryEntry {
    pub(crate) fn path(&self) -> &Path {
        self.directory.path()
    }

    /// Read the bounded sidecar only after proving it is a regular in-entry file, then require the
    /// claimed id to identify this directory. On Windows the filesystem decides identity, so a
    /// harmless casing difference is accepted while a different sibling remains a mismatch.
    pub(crate) fn read_meta(&self) -> crate::Result<ModEntryMeta> {
        let bytes = self.read_payload_bounded(
            Path::new(META_FILE),
            "library sidecar",
            LIBRARY_META_MAX_BYTES,
        )?;
        let meta: ModEntryMeta = serde_json::from_slice(&bytes).map_err(|error| {
            crate::ModError::Other(format!(
                "corrupt library sidecar for {:?} at {}: {error}",
                self.id,
                self.path().join(META_FILE).display()
            ))
        })?;
        self.validate_claimed_id(&meta.id)?;
        Ok(meta)
    }

    /// Read one already-validated library payload through an opened file handle and a hard byte
    /// ceiling.  The metadata snapshot, `limit + 1` read, and final handle/path metadata checks
    /// make truncation or growth during the read a hard error instead of accepting a partial or
    /// unexpectedly large allocation.
    pub(crate) fn read_payload_bounded(
        &self,
        rel: &Path,
        label: &str,
        limit: u64,
    ) -> crate::Result<Vec<u8>> {
        self.read_payload_bounded_with(rel, label, limit, || {})
    }

    fn read_payload_bounded_with<F>(
        &self,
        rel: &Path,
        label: &str,
        limit: u64,
        after_metadata: F,
    ) -> crate::Result<Vec<u8>>
    where
        F: FnOnce(),
    {
        let (path, mut file, expected) = self.open_payload_bounded(rel, label, limit)?;
        after_metadata();

        let capacity = usize::try_from(expected).map_err(|_| {
            crate::ModError::Other(format!(
                "{label} is too large for this process address space: {}",
                path.display()
            ))
        })?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(capacity).map_err(|_| {
            crate::ModError::Other(format!(
                "could not reserve {expected} bytes for {label}: {}",
                path.display()
            ))
        })?;
        file.file
            .by_ref()
            .take(limit.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(crate::io(&format!("reading {label} {}", path.display())))?;
        let observed = u64::try_from(bytes.len()).map_err(|_| {
            crate::ModError::Other(format!("{label} byte count overflowed: {}", path.display()))
        })?;
        if observed > limit {
            return Err(crate::ModError::Other(format!(
                "{label} exceeds the {limit} byte limit: {}",
                path.display()
            )));
        }
        self.verify_open_payload_unchanged(&file, label, expected, observed)?;
        Ok(bytes)
    }

    /// Stream a validated payload to a private temporary file without retaining it in memory.
    /// This is used for whole-file raw replacements that may be several GiB.  The returned temp
    /// owns the verified snapshot and disappears automatically if plan construction later fails.
    pub(crate) fn snapshot_payload_bounded(
        &self,
        rel: &Path,
        label: &str,
        limit: u64,
    ) -> crate::Result<(tempfile::TempPath, u64)> {
        let (path, mut file, expected) = self.open_payload_bounded(rel, label, limit)?;
        self.snapshot_opened_payload_bounded(path, &mut file, expected, label)
    }

    fn snapshot_opened_payload_bounded(
        &self,
        path: PathBuf,
        file: &mut SecureFile,
        expected: u64,
        label: &str,
    ) -> crate::Result<(tempfile::TempPath, u64)> {
        let mut temp = tempfile::Builder::new()
            .prefix(".gore-manager-payload-")
            .tempfile()
            .map_err(crate::io("creating manager payload snapshot"))?;
        // `expected + 1` detects growth at the first unexpected byte.  The initial metadata gate
        // has already proved expected <= limit, while the final metadata checks catch shrinkage or
        // a grow-and-stop race even when no extra byte was observed by `copy`.
        let copied = std::io::copy(
            &mut file.file.by_ref().take(expected.saturating_add(1)),
            temp.as_file_mut(),
        )
        .map_err(crate::io(&format!(
            "snapshotting {label} {}",
            path.display()
        )))?;
        self.verify_open_payload_unchanged(file, label, expected, copied)?;
        temp.as_file()
            .sync_all()
            .map_err(crate::io("syncing manager payload snapshot"))?;
        Ok((temp.into_temp_path(), copied))
    }

    /// Snapshot an optional payload without ever publishing its mutable library path. A missing
    /// path is the only `None` case; any present link, directory, special file, or raced open is a
    /// hard error. The eventual copy still uses exactly one anchored no-follow file handle.
    pub(crate) fn snapshot_optional_payload_bounded(
        &self,
        rel: &Path,
        label: &str,
        limit: u64,
    ) -> crate::Result<Option<(tempfile::TempPath, u64)>> {
        let Some(mut file) = self.open_optional_payload_file(rel, label)? else {
            return Ok(None);
        };
        let path = file.path().to_path_buf();
        if file.len() > limit {
            return Err(crate::ModError::Other(format!(
                "{label} exceeds the {limit} byte limit: {}",
                path.display()
            )));
        }
        let expected = file.len();
        self.snapshot_opened_payload_bounded(path, &mut file, expected, label)
            .map(Some)
    }

    fn open_payload_bounded(
        &self,
        rel: &Path,
        label: &str,
        limit: u64,
    ) -> crate::Result<(PathBuf, SecureFile, u64)> {
        let file = self.open_payload_file(rel, label)?;
        let path = file.path().to_path_buf();
        if file.len() > limit {
            return Err(crate::ModError::Other(format!(
                "{label} exceeds the {limit} byte limit: {}",
                path.display()
            )));
        }
        let len = file.len();
        Ok((path, file, len))
    }

    fn verify_open_payload_unchanged(
        &self,
        file: &SecureFile,
        label: &str,
        expected: u64,
        observed: u64,
    ) -> crate::Result<()> {
        let handle_len = file
            .file
            .metadata()
            .map_err(crate::io(&format!(
                "rechecking opened {label} metadata {}",
                file.path().display()
            )))?
            .len();
        if observed != expected || handle_len != expected {
            return Err(crate::ModError::Other(format!(
                "{label} changed size while being read: {} (expected {expected}, observed {observed}, current {handle_len})",
                file.path().display()
            )));
        }
        file.verify_len(expected, label)
    }

    fn open_payload_node(&self, rel: &Path, label: &str) -> crate::Result<SecureNode> {
        let rel_text = rel.to_string_lossy();
        if !crate::is_safe_rel_path(&rel_text) {
            return Err(crate::ModError::Other(format!(
                "unsafe {label} path in library entry {:?}: {rel_text:?}",
                self.id
            )));
        }
        let components: Vec<_> = rel.components().collect();
        if components.is_empty() {
            return Err(crate::ModError::Other(format!(
                "empty {label} path in library entry {:?}",
                self.id
            )));
        }
        let mut directory = self.directory.clone();
        for (index, component) in components.iter().enumerate() {
            let std::path::Component::Normal(name) = component else {
                return Err(crate::ModError::Other(format!(
                    "unsafe {label} path component in {rel_text:?}"
                )));
            };
            let node = directory.open_child(name, label)?;
            if index + 1 == components.len() {
                return Ok(node);
            }
            directory = match node {
                SecureNode::Directory(directory) => directory,
                SecureNode::File(file) => {
                    return Err(crate::ModError::Other(format!(
                        "{label} path crosses a regular file: {}",
                        file.path().display()
                    )))
                }
            };
        }
        unreachable!("non-empty component list returns its final node")
    }

    fn open_payload_file(&self, rel: &Path, label: &str) -> crate::Result<SecureFile> {
        match self.open_payload_node(rel, label)? {
            SecureNode::File(file) => Ok(file),
            SecureNode::Directory(directory) => Err(crate::ModError::Other(format!(
                "{label} must be a regular file: {}",
                directory.path().display()
            ))),
        }
    }

    fn open_optional_payload_file(
        &self,
        rel: &Path,
        label: &str,
    ) -> crate::Result<Option<SecureFile>> {
        let rel_text = rel.to_string_lossy();
        if !crate::is_safe_rel_path(&rel_text) {
            return Err(crate::ModError::Other(format!(
                "unsafe {label} path in library entry {:?}: {rel_text:?}",
                self.id
            )));
        }
        let components: Vec<_> = rel.components().collect();
        if components.is_empty() {
            return Err(crate::ModError::Other(format!(
                "empty {label} path in library entry {:?}",
                self.id
            )));
        }
        let mut directory = self.directory.clone();
        for (index, component) in components.iter().enumerate() {
            let std::path::Component::Normal(name) = component else {
                return Err(crate::ModError::Other(format!(
                    "unsafe {label} path component in {rel_text:?}"
                )));
            };
            if !directory.contains_child(name, label)? {
                return Ok(None);
            }
            match directory.open_child(name, label)? {
                SecureNode::File(file) if index + 1 == components.len() => {
                    return Ok(Some(file));
                }
                SecureNode::Directory(child) if index + 1 < components.len() => {
                    directory = child;
                }
                SecureNode::Directory(child) => {
                    return Err(crate::ModError::Other(format!(
                        "{label} must be a regular file: {}",
                        child.path().display()
                    )));
                }
                SecureNode::File(file) => {
                    return Err(crate::ModError::Other(format!(
                        "{label} path crosses a regular file: {}",
                        file.path().display()
                    )));
                }
            }
        }
        unreachable!("non-empty optional component list returns its final file")
    }

    fn open_payload_directory(&self, rel: &Path, label: &str) -> crate::Result<SecureDirectory> {
        match self.open_payload_node(rel, label)? {
            SecureNode::Directory(directory) => Ok(directory),
            SecureNode::File(file) => Err(crate::ModError::Other(format!(
                "{label} must be a directory: {}",
                file.path().display()
            ))),
        }
    }

    /// Traverse a directory exclusively through anchored, no-follow handles and copy its exact
    /// validated bytes to a private snapshot. Deferred deploy work consumes only this snapshot,
    /// never the mutable manager-library tree that was checked earlier.
    pub(crate) fn snapshot_payload_tree(
        &self,
        rel: &Path,
        label: &str,
        limits: TreeSnapshotLimits,
    ) -> crate::Result<PayloadTreeSnapshot> {
        let source = self.open_payload_directory(rel, label)?;
        let temp = tempfile::Builder::new()
            .prefix(".gore-manager-tree-")
            .tempdir()
            .map_err(crate::io("creating manager tree snapshot"))?;
        let root = temp.path().join(rel);
        std::fs::create_dir_all(&root).map_err(crate::io("creating manager tree snapshot root"))?;
        let mut entries = 0usize;
        let mut bytes = 0u64;
        snapshot_secure_directory(
            &source,
            &root,
            Path::new(""),
            label,
            limits,
            0,
            &mut entries,
            &mut bytes,
        )?;
        Ok(PayloadTreeSnapshot {
            temp,
            root,
            entries,
            bytes,
        })
    }

    fn validate_claimed_id(&self, claimed: &str) -> crate::Result<()> {
        validate_library_id(claimed).map_err(|_| {
            crate::ModError::Other(format!(
                "library sidecar for directory {:?} contains invalid id {claimed:?}",
                self.id
            ))
        })?;
        if self.id == claimed || self.claimed_id_identifies_entry(claimed) {
            return Ok(());
        }
        Err(crate::ModError::Other(format!(
            "library sidecar id mismatch: directory/loadout id {:?}, metadata id {claimed:?}",
            self.id
        )))
    }

    fn claimed_id_identifies_entry(&self, claimed: &str) -> bool {
        match self
            .library
            .open_child(std::ffi::OsStr::new(claimed), "claimed library entry")
        {
            Ok(SecureNode::Directory(directory)) => {
                directory.identity() == self.directory.identity()
            }
            Ok(SecureNode::File(_)) | Err(_) => false,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn snapshot_secure_directory(
    source: &SecureDirectory,
    destination: &Path,
    relative: &Path,
    label: &str,
    limits: TreeSnapshotLimits,
    depth: usize,
    entries: &mut usize,
    bytes: &mut u64,
) -> crate::Result<()> {
    for entry in source.read_dir(label)? {
        let entry = entry.map_err(crate::io(&format!("reading {label} directory entry")))?;
        let name = entry.file_name();
        validate_plain_component(&name, label)?;
        let child_relative = relative.join(&name);
        let portable = child_relative
            .components()
            .map(|component| {
                let std::path::Component::Normal(name) = component else {
                    return Err(crate::ModError::Other(format!(
                        "{label} snapshot path is not plain relative syntax: {}",
                        child_relative.display()
                    )));
                };
                name.to_str().map(str::to_owned).ok_or_else(|| {
                    crate::ModError::Other(format!(
                        "{label} snapshot path is not valid Unicode: {}",
                        child_relative.display()
                    ))
                })
            })
            .collect::<crate::Result<Vec<_>>>()?
            .join("/");
        if portable.len() > limits.max_path_bytes || !crate::is_safe_rel_path(&portable) {
            return Err(crate::ModError::Other(format!(
                "{label} snapshot contains an unsafe or overlong path: {portable:?}"
            )));
        }
        *entries = entries
            .checked_add(1)
            .ok_or_else(|| crate::ModError::Other(format!("{label} entry count overflowed")))?;
        if *entries > limits.max_entries {
            return Err(crate::ModError::Other(format!(
                "{label} snapshot entry limit exceeded: {} > {}",
                *entries, limits.max_entries
            )));
        }

        let target = destination.join(&name);
        run_tree_entry_race_hook(&source.path().join(&name));
        match source.open_child(&name, label)? {
            SecureNode::Directory(directory) => {
                if depth >= limits.max_depth {
                    return Err(crate::ModError::Other(format!(
                        "{label} snapshot depth limit exceeded at {portable:?}"
                    )));
                }
                std::fs::create_dir(&target)
                    .map_err(crate::io(&format!("creating {label} snapshot directory")))?;
                snapshot_secure_directory(
                    &directory,
                    &target,
                    &child_relative,
                    label,
                    limits,
                    depth + 1,
                    entries,
                    bytes,
                )?;
            }
            SecureNode::File(mut file) => {
                let expected = file.len();
                if expected > limits.max_file_bytes {
                    return Err(crate::ModError::Other(format!(
                        "{label} snapshot file exceeds the {} byte limit: {}",
                        limits.max_file_bytes,
                        file.path().display()
                    )));
                }
                let next_total = bytes.checked_add(expected).ok_or_else(|| {
                    crate::ModError::Other(format!("{label} snapshot byte count overflowed"))
                })?;
                if next_total > limits.max_total_bytes {
                    return Err(crate::ModError::Other(format!(
                        "{label} snapshot total byte limit exceeded: {next_total} > {}",
                        limits.max_total_bytes
                    )));
                }
                // Charge before creating/copying the file so aggregate exhaustion never performs
                // one additional full snapshot operation.
                *bytes = next_total;
                let mut output = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&target)
                    .map_err(crate::io(&format!("creating {label} snapshot file")))?;
                let copied = std::io::copy(
                    &mut file.file.by_ref().take(expected.saturating_add(1)),
                    &mut output,
                )
                .map_err(crate::io(&format!("copying {label} snapshot file")))?;
                let mut probe = [0u8; 1];
                let has_more = file
                    .file
                    .read(&mut probe)
                    .map_err(crate::io(&format!("probing {label} snapshot source")))?
                    != 0;
                file.verify_len(expected, label)?;
                if has_more || copied != expected {
                    drop(output);
                    let _ = std::fs::remove_file(&target);
                    return Err(crate::ModError::Other(format!(
                        "{label} changed while its tree snapshot was copied: {}",
                        file.path().display()
                    )));
                }
                output
                    .sync_all()
                    .map_err(crate::io(&format!("syncing {label} snapshot file")))?;
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_library_id(id: &str) -> crate::Result<()> {
    if crate::is_safe_mod_name(id) {
        Ok(())
    } else {
        Err(crate::ModError::Other(format!(
            "invalid library entry id {id:?}: expected one safe path component"
        )))
    }
}

pub(crate) fn metadata_is_link(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;

        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

/// One imported mod in the manager library.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModEntryMeta {
    /// Stable library id (dir name under the library root).
    pub id: String,
    pub kind: ModKind,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
    /// RFC 3339 timestamp of the import.
    pub imported_at: String,
    /// Where the mod came from (original archive/dir path), informational.
    #[serde(default)]
    pub source: String,
    pub components: Vec<ComponentInfo>,
}

impl ModEntryMeta {
    /// A content fingerprint that changes when the mod is re-imported as an update (same id, new
    /// components/bytes). `imported_at` is microsecond-resolution (set at import), so it differs on
    /// every re-import — even within the same second, with unchanged component descriptors and only
    /// changed payload bytes — giving mgr_status a reliable "content changed" signal. The serialized
    /// components are folded in too, so a structural change flips the fingerprint on its own.
    pub fn fingerprint(&self) -> String {
        let body = serde_json::to_string(&self.components).unwrap_or_default();
        crate::name_hash(&format!("{}|{}", self.imported_at, body))
    }
}

/// How the mod was recognized at import.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModKind {
    Goremod,
    ForeignTriplet,
    ForeignPak,
    ForeignUe4ss,
    ForeignRawfile,
    ForeignMixed,
}

/// How completely a component's metadata describes its conflict-analysis footprint.
///
/// This is derived from [`ComponentInfo`] rather than stored in the library sidecar. It says
/// nothing about runtime precedence: even an exact footprint can still have unproven ordering
/// semantics in the game.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FootprintCoverage {
    Exact,
    Partial,
    Advisory,
    Opaque,
}

/// One deployable component of a library mod. `rel`/`rel_base` are paths inside the mod's
/// library dir; `targets` are the game-side footprint keys used for conflict analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ComponentInfo {
    Ue4ssLua {
        name: String,
        rel: String,
        /// Known exact `Class.Field` CDO targets. This remains useful partial metadata when
        /// `opaque` is true, but must not be interpreted as the complete script footprint.
        #[serde(default)]
        targets: Vec<String>,
        /// Whether the script can affect targets or runtime state beyond `targets`.
        #[serde(default)]
        opaque: bool,
    },
    LocPatch {
        rel: String,
        targets: Vec<String>,
    },
    AudioPatch {
        rel: String,
        targets: Vec<String>,
    },
    TexturePatch {
        rel: String,
        targets: Vec<String>,
    },
    AngelScriptPatch {
        rel: String,
        targets: Vec<String>,
    },
    FilePatch {
        rel: String,
        /// Game-root-relative, forward-slash destinations this component replaces wholesale, e.g.
        /// `G1R/Content/Slate/Cursors/Normal/Normal.PNG`.
        targets: Vec<String>,
    },
    PakFilePatch {
        rel: String,
        /// The SAME game-root-relative destinations [`ComponentInfo::FilePatch`] names, claimed
        /// from an additive `~mods` pak instead of overwritten on disk. Spelling them identically
        /// is what lets conflict analysis see the two mechanisms fighting over one file.
        targets: Vec<String>,
    },
    VoiceArchivePatch {
        rel: String,
        /// `"<archive>|<member path>"` targets used for soft, order-dependent conflicts.
        targets: Vec<String>,
    },
    Triplet {
        rel_base: String,
        targets: Vec<String>,
    },
    LoosePak {
        rel: String,
        targets: Vec<String>,
    },
    RawFile {
        rel: String,
        target_file: RawTarget,
    },
}

impl ComponentInfo {
    /// Derive how completely this component's metadata describes its footprint.
    ///
    /// Container scans remain compatible with corrupt or unreadable inputs: an empty scan is
    /// opaque rather than an import failure. IoStore package discovery is advisory even when it
    /// finds packages, while a successfully indexed plain Pak is exhaustive.
    #[must_use]
    pub fn footprint_coverage(&self) -> FootprintCoverage {
        match self {
            Self::Ue4ssLua {
                targets, opaque, ..
            } => match (*opaque, targets.is_empty()) {
                (false, _) => FootprintCoverage::Exact,
                (true, false) => FootprintCoverage::Partial,
                (true, true) => FootprintCoverage::Opaque,
            },
            Self::Triplet { targets, .. } => {
                if targets.is_empty() {
                    FootprintCoverage::Opaque
                } else {
                    FootprintCoverage::Advisory
                }
            }
            Self::LoosePak { targets, .. } => {
                if targets.is_empty() {
                    FootprintCoverage::Opaque
                } else {
                    FootprintCoverage::Exact
                }
            }
            Self::LocPatch { .. }
            | Self::AudioPatch { .. }
            | Self::TexturePatch { .. }
            | Self::AngelScriptPatch { .. }
            | Self::FilePatch { .. }
            | Self::PakFilePatch { .. }
            | Self::VoiceArchivePatch { .. }
            | Self::RawFile { .. } => FootprintCoverage::Exact,
        }
    }
}

/// The single live game file a [`ComponentInfo::RawFile`] replaces wholesale.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RawTarget {
    Lcache,
    Bank { name: String },
    ScriptCache,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    #[test]
    fn footprint_coverage_matrix_is_conservative_and_derived() {
        let exact = vec![
            ComponentInfo::Ue4ssLua {
                name: "Precise".into(),
                rel: "ue4ss/Precise".into(),
                targets: vec![],
                opaque: false,
            },
            ComponentInfo::LocPatch {
                rel: "loc".into(),
                targets: vec![],
            },
            ComponentInfo::AudioPatch {
                rel: "audio".into(),
                targets: vec![],
            },
            ComponentInfo::TexturePatch {
                rel: "texture".into(),
                targets: vec![],
            },
            ComponentInfo::AngelScriptPatch {
                rel: "scripts".into(),
                targets: vec![],
            },
            ComponentInfo::FilePatch {
                rel: "files".into(),
                targets: vec![],
            },
            ComponentInfo::PakFilePatch {
                rel: "pak_files".into(),
                targets: vec![],
            },
            ComponentInfo::VoiceArchivePatch {
                rel: "voice".into(),
                targets: vec![],
            },
            ComponentInfo::LoosePak {
                rel: "indexed.pak".into(),
                targets: vec!["/Game/Indexed".into()],
            },
            ComponentInfo::RawFile {
                rel: "raw/Game.lcache".into(),
                target_file: RawTarget::Lcache,
            },
        ];
        for component in exact {
            assert_eq!(
                component.footprint_coverage(),
                FootprintCoverage::Exact,
                "component: {component:?}"
            );
        }

        let cases = [
            (
                ComponentInfo::Ue4ssLua {
                    name: "Partial".into(),
                    rel: "ue4ss/Partial".into(),
                    targets: vec!["Class.Field".into()],
                    opaque: true,
                },
                FootprintCoverage::Partial,
            ),
            (
                ComponentInfo::Ue4ssLua {
                    name: "Opaque".into(),
                    rel: "ue4ss/Opaque".into(),
                    targets: vec![],
                    opaque: true,
                },
                FootprintCoverage::Opaque,
            ),
            (
                ComponentInfo::Triplet {
                    rel_base: "container".into(),
                    targets: vec!["/Game/Observed".into()],
                },
                FootprintCoverage::Advisory,
            ),
            (
                ComponentInfo::Triplet {
                    rel_base: "unreadable".into(),
                    targets: vec![],
                },
                FootprintCoverage::Opaque,
            ),
            (
                ComponentInfo::LoosePak {
                    rel: "unreadable.pak".into(),
                    targets: vec![],
                },
                FootprintCoverage::Opaque,
            ),
        ];
        for (component, expected) in cases {
            assert_eq!(
                component.footprint_coverage(),
                expected,
                "component: {component:?}"
            );
        }

        let serialized = serde_json::to_value(ComponentInfo::LoosePak {
            rel: "indexed.pak".into(),
            targets: vec!["/Game/Indexed".into()],
        })
        .unwrap();
        assert!(
            serialized.get("coverage").is_none(),
            "derived coverage must not enter library metadata: {serialized}"
        );
        assert_eq!(
            [
                FootprintCoverage::Exact,
                FootprintCoverage::Partial,
                FootprintCoverage::Advisory,
                FootprintCoverage::Opaque,
            ]
            .map(|coverage| serde_json::to_value(coverage).unwrap()),
            [
                serde_json::json!("exact"),
                serde_json::json!("partial"),
                serde_json::json!("advisory"),
                serde_json::json!("opaque"),
            ]
        );
    }

    #[cfg(unix)]
    fn make_file_link(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).unwrap();
        true
    }

    #[cfg(windows)]
    fn make_file_link(target: &Path, link: &Path) -> bool {
        match std::os::windows::fs::symlink_file(target, link) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("creating test file symlink failed: {error}"),
        }
    }

    #[test]
    fn read_meta_rejects_sidecars_over_16_mib_without_reading_them() {
        let temp = tempfile::tempdir().unwrap();
        let library_path = temp.path().join("library");
        let entry_path = library_path.join("entry-a");
        std::fs::create_dir_all(&entry_path).unwrap();
        let sidecar = std::fs::File::create(entry_path.join(META_FILE)).unwrap();
        sidecar.set_len(LIBRARY_META_MAX_BYTES + 1).unwrap();
        drop(sidecar);

        let library = LibraryRoot::open(&library_path).unwrap();
        let error = library
            .entry("entry-a")
            .unwrap()
            .read_meta()
            .unwrap_err()
            .to_string();
        assert!(error.contains("16777216 byte limit"), "{error}");
    }

    #[test]
    fn bounded_payload_read_rejects_limit_and_detects_growth_or_denies_writer() {
        let temp = tempfile::tempdir().unwrap();
        let library_path = temp.path().join("library");
        let entry_path = library_path.join("entry-a");
        std::fs::create_dir_all(&entry_path).unwrap();
        std::fs::write(entry_path.join("payload.bin"), b"12345").unwrap();
        // `LibraryEntry` requires a real sidecar only when read_meta is called; payload validation
        // itself is intentionally independent of metadata parsing.
        let library = LibraryRoot::open(&library_path).unwrap();
        let entry = library.entry("entry-a").unwrap();

        let error = entry
            .read_payload_bounded(Path::new("payload.bin"), "test payload", 4)
            .unwrap_err()
            .to_string();
        assert!(error.contains("4 byte limit"), "{error}");

        let payload = entry_path.join("payload.bin");
        let mut writer_was_denied = None;
        let result =
            entry.read_payload_bounded_with(Path::new("payload.bin"), "test payload", 16, || {
                match std::fs::OpenOptions::new().append(true).open(&payload) {
                    Ok(mut writer) => {
                        writer.write_all(b"6").unwrap();
                        writer.sync_all().unwrap();
                        writer_was_denied = Some(false);
                    }
                    Err(_) => writer_was_denied = Some(true),
                }
            });
        assert!(writer_was_denied.is_some(), "the growth hook must run");
        if writer_was_denied == Some(true) {
            #[cfg(not(windows))]
            panic!("Unix must permit and then detect the write");
            assert_eq!(result.unwrap(), b"12345");
        } else {
            let error = result.unwrap_err().to_string();
            assert!(error.contains("changed size while being read"), "{error}");
        }
    }

    #[test]
    fn bounded_payload_read_rejects_same_size_mutation_or_denies_the_writer() {
        let temp = tempfile::tempdir().unwrap();
        let library_path = temp.path().join("library");
        let entry_path = library_path.join("entry-a");
        std::fs::create_dir_all(&entry_path).unwrap();
        let payload = entry_path.join("payload.bin");
        std::fs::write(&payload, b"12345").unwrap();
        let entry = LibraryRoot::open(&library_path)
            .unwrap()
            .entry("entry-a")
            .unwrap();

        let mut writer_was_denied = None;
        let result = entry.read_payload_bounded_with(
            Path::new("payload.bin"),
            "same-size payload",
            16,
            || match std::fs::OpenOptions::new().write(true).open(&payload) {
                Ok(mut writer) => {
                    writer.write_all(b"abcde").unwrap();
                    writer.sync_all().unwrap();
                    writer_was_denied = Some(false);
                }
                Err(_) => writer_was_denied = Some(true),
            },
        );
        assert!(writer_was_denied.is_some(), "the mutation hook must run");
        if writer_was_denied == Some(true) {
            #[cfg(not(windows))]
            panic!("Unix must permit and then detect the write");
            assert_eq!(result.unwrap(), b"12345");
        } else {
            let error = result.unwrap_err().to_string();
            assert!(error.contains("content revision"), "{error}");
        }
    }

    #[cfg(any(unix, windows))]
    #[cfg_attr(
        windows,
        ignore = "requires Windows symbolic-link privilege; run explicitly on a privileged worker"
    )]
    #[test]
    fn read_meta_rejects_a_link_swapped_in_immediately_before_handle_open() {
        let temp = tempfile::tempdir().unwrap();
        let library_path = temp.path().join("library");
        let entry_path = library_path.join("entry-a");
        std::fs::create_dir_all(&entry_path).unwrap();
        let sidecar = entry_path.join(META_FILE);
        std::fs::write(
            &sidecar,
            br#"{"id":"entry-a","kind":"goremod","name":"safe","imported_at":"x","components":[]}"#,
        )
        .unwrap();
        let outside = temp.path().join("outside.json");
        std::fs::write(
            &outside,
            br#"{"id":"entry-a","kind":"goremod","name":"escaped","imported_at":"x","components":[]}"#,
        )
        .unwrap();
        let staged_link = temp.path().join("sidecar-link");
        assert!(
            make_file_link(&outside, &staged_link),
            "test requires symbolic-link creation support"
        );

        let library = LibraryRoot::open(&library_path).unwrap();
        let entry = library.entry("entry-a").unwrap();
        inject_open_child_race(move |opened_path| {
            assert_eq!(
                opened_path.file_name(),
                Some(std::ffi::OsStr::new(META_FILE))
            );
            std::fs::remove_file(opened_path).unwrap();
            std::fs::rename(&staged_link, opened_path).unwrap();
        });
        let error = entry.read_meta().unwrap_err().to_string();
        assert!(
            error.contains("symbolic link")
                || error.contains("reparse point")
                || error.contains("without following"),
            "{error}"
        );
    }

    #[cfg(any(unix, windows))]
    #[cfg_attr(
        windows,
        ignore = "requires Windows symbolic-link privilege; run explicitly on a privileged worker"
    )]
    #[test]
    fn tree_snapshot_rejects_entry_swapped_to_link_after_enumeration() {
        let temp = tempfile::tempdir().unwrap();
        let library_path = temp.path().join("library");
        let entry_path = library_path.join("entry-a");
        let tree = entry_path.join("payload");
        std::fs::create_dir_all(&tree).unwrap();
        std::fs::write(tree.join("safe.bin"), b"safe").unwrap();
        let outside = temp.path().join("outside.bin");
        std::fs::write(&outside, b"escaped").unwrap();
        let staged_link = temp.path().join("tree-link");
        assert!(
            make_file_link(&outside, &staged_link),
            "test requires symbolic-link creation support"
        );

        let entry = LibraryRoot::open(&library_path)
            .unwrap()
            .entry("entry-a")
            .unwrap();
        inject_tree_entry_race(move |enumerated_path| {
            assert_eq!(
                enumerated_path.file_name(),
                Some(std::ffi::OsStr::new("safe.bin"))
            );
            std::fs::remove_file(enumerated_path).unwrap();
            std::fs::rename(&staged_link, enumerated_path).unwrap();
        });
        let error = entry
            .snapshot_payload_tree(
                Path::new("payload"),
                "test tree",
                TreeSnapshotLimits {
                    max_entries: 8,
                    max_path_bytes: 128,
                    max_depth: 4,
                    max_file_bytes: 1024,
                    max_total_bytes: 1024,
                },
            )
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("symbolic link")
                || error.contains("reparse point")
                || error.contains("without following"),
            "{error}"
        );
    }

    /// The library metadata contract must survive a JSON round-trip for EVERY component and
    /// raw-target variant, with the agreed snake_case wire tags.
    #[test]
    fn meta_roundtrips_json() {
        let meta = ModEntryMeta {
            id: "my-mod-1a2b".into(),
            kind: ModKind::ForeignMixed,
            name: "My Mod".into(),
            version: "1.2.3".into(),
            author: "someone".into(),
            imported_at: "2026-07-03T12:00:00Z".into(),
            source: "C:/downloads/my-mod.zip".into(),
            components: vec![
                ComponentInfo::Ue4ssLua {
                    name: "MyMod".into(),
                    rel: "ue4ss/MyMod".into(),
                    targets: vec!["ue4ss:MyMod".into()],
                    opaque: true,
                },
                ComponentInfo::LocPatch {
                    rel: "loc/edits.json".into(),
                    targets: vec!["loc:itfo_x".into()],
                },
                ComponentInfo::AudioPatch {
                    rel: "audio".into(),
                    targets: vec!["bank:SFX.bank".into()],
                },
                ComponentInfo::TexturePatch {
                    rel: "texture".into(),
                    targets: vec!["tex:/Game/UI/T_X".into()],
                },
                ComponentInfo::AngelScriptPatch {
                    rel: "scripts".into(),
                    targets: vec!["as:MyModule".into()],
                },
                ComponentInfo::VoiceArchivePatch {
                    rel: "voice".into(),
                    targets: vec!["German.zip|NPC/hello.ogg".into()],
                },
                ComponentInfo::PakFilePatch {
                    rel: "pak_files".into(),
                    targets: vec!["G1R/Content/Slate/Cursors/Normal/Normal.PNG".into()],
                },
                ComponentInfo::Triplet {
                    rel_base: "paks/zzz_MyMod_P".into(),
                    targets: vec!["pak:zzz_MyMod_P".into()],
                },
                ComponentInfo::LoosePak {
                    rel: "paks/extra.pak".into(),
                    targets: vec!["pak:extra".into()],
                },
                ComponentInfo::RawFile {
                    rel: "raw/loc.lcache".into(),
                    target_file: RawTarget::Lcache,
                },
                ComponentInfo::RawFile {
                    rel: "raw/SFX.bank".into(),
                    target_file: RawTarget::Bank {
                        name: "SFX.bank".into(),
                    },
                },
                ComponentInfo::RawFile {
                    rel: "raw/script.cache".into(),
                    target_file: RawTarget::ScriptCache,
                },
            ],
        };
        let json = serde_json::to_string_pretty(&meta).unwrap();
        let back: ModEntryMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(meta, back);

        // Lock the wire format: adjacently-known snake_case tags and kind names.
        assert!(json.contains("\"foreign_mixed\""), "kind tag: {json}");
        assert!(json.contains("\"ue4ss_lua\""), "component tag: {json}");
        assert!(
            json.contains("\"angel_script_patch\""),
            "component tag: {json}"
        );
        assert!(
            json.contains("\"voice_archive_patch\""),
            "component tag: {json}"
        );
        assert!(json.contains("\"loose_pak\""), "component tag: {json}");
        assert!(json.contains("\"pak_file_patch\""), "component tag: {json}");
        assert!(json.contains("\"raw_file\""), "component tag: {json}");
        assert!(json.contains("\"script_cache\""), "raw target tag: {json}");

        // Defaulted fields may be absent in hand-written metadata.
        let minimal: ModEntryMeta = serde_json::from_str(
            r#"{
                "id": "x",
                "kind": "goremod",
                "name": "X",
                "imported_at": "2026-07-03T12:00:00Z",
                "components": [
                    { "type": "ue4ss_lua", "name": "X", "rel": "ue4ss/X" }
                ]
            }"#,
        )
        .unwrap();
        assert_eq!(minimal.version, "");
        assert_eq!(minimal.author, "");
        assert_eq!(minimal.source, "");
        assert_eq!(
            minimal.components[0],
            ComponentInfo::Ue4ssLua {
                name: "X".into(),
                rel: "ue4ss/X".into(),
                targets: vec![],
                opaque: false
            }
        );
    }
}
