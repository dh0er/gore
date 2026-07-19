//! Retained, component-by-component no-follow guard for one existing managed Store root.
//!
//! Read-only authoring routes use this guard to close the mutable pathname window around an
//! exact-current Store inspection. Capturing the guard retains every directory in the absolute
//! path chain; revalidation rejects a same-path replacement even when every Store byte is
//! identical.
//!
//! On Windows, the retained handles omit delete/write sharing and therefore prevent a path-chain
//! rename for the lifetime of the guard. Linux additionally attaches a nonblocking inotify monitor
//! to every exact retained directory handle; move/delete/unmount, exact child-entry replacement,
//! ignored watches, and queue overflow are latched as drift even if the original spelling is later
//! restored. The exact retained root mount ID additionally binds every file opened by the two
//! read-only planners, rejecting transient root, bind, and FUSE mounts before their bytes or hashes
//! are used. Other Unix targets fail capture closed until they have equivalent primitives.

use std::io;
use std::path::{Component, Path, PathBuf};

#[cfg(target_os = "linux")]
use gore_authoring::WorkingStoreLinuxMountId;
use gore_authoring::{WorkingProjectStore, WorkingStoreError, WorkingStoreLimits};

use crate::voice::SecureDirectDirectory;
#[cfg(target_os = "linux")]
use crate::voice::SecureRetainedPathMonitor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetainedStoreRootError {
    Unavailable,
    Changed,
}

pub(crate) struct RetainedStoreRoot {
    // Historical API name: this is the normalized absolute no-follow path, deliberately obtained
    // without `canonicalize` because canonicalization would follow a mutable ancestor.
    canonical: PathBuf,
    held: SecureDirectDirectory,
    #[cfg(target_os = "linux")]
    mount_id: WorkingStoreLinuxMountId,
    #[cfg(target_os = "linux")]
    monitor: SecureRetainedPathMonitor,
}

impl RetainedStoreRoot {
    pub(crate) fn capture(requested: &Path) -> Result<Self, RetainedStoreRootError> {
        #[cfg(not(any(windows, target_os = "linux")))]
        {
            let _ = requested;
            return Err(RetainedStoreRootError::Unavailable);
        }

        #[cfg(any(windows, target_os = "linux"))]
        {
            let canonical = normalized_absolute_path(requested)
                .map_err(|_| RetainedStoreRootError::Unavailable)?;
            // `SecureDirectDirectory` opens from the filesystem root one component at a time. Unix
            // uses openat(O_NOFOLLOW) against the retained parent fd. Windows retains every accepted
            // ancestor without DELETE/WRITE sharing before opening the next prefix, and opens every
            // prefix with OPEN_REPARSE_POINT so a junction cannot redirect traversal.
            let held = SecureDirectDirectory::open(&canonical)
                .map_err(|_| RetainedStoreRootError::Unavailable)?;
            #[cfg(target_os = "linux")]
            let mount_id =
                WorkingStoreLinuxMountId::from_open_file(held.retained_directory_handle())
                    .map_err(|_| RetainedStoreRootError::Unavailable)?;
            #[cfg(target_os = "linux")]
            let monitor = held
                .monitor_retained_path_changes()
                .map_err(|_| RetainedStoreRootError::Unavailable)?;

            // Capture becomes visible only after the ambient spelling still resolves through the
            // exact no-follow chain and no watched mutation occurred during monitor installation.
            held.revalidate()
                .map_err(|_| RetainedStoreRootError::Unavailable)?;
            #[cfg(target_os = "linux")]
            monitor
                .revalidate()
                .map_err(|_| RetainedStoreRootError::Unavailable)?;
            #[cfg(target_os = "linux")]
            revalidate_linux_mount(&canonical, mount_id)
                .map_err(|_| RetainedStoreRootError::Unavailable)?;

            Ok(Self {
                canonical,
                held,
                #[cfg(target_os = "linux")]
                mount_id,
                #[cfg(target_os = "linux")]
                monitor,
            })
        }
    }

    pub(crate) fn canonical(&self) -> &Path {
        &self.canonical
    }

    pub(crate) fn open_existing_store(
        &self,
        limits: WorkingStoreLimits,
    ) -> Result<WorkingProjectStore, WorkingStoreError> {
        #[cfg(target_os = "linux")]
        {
            return WorkingProjectStore::open_existing_read_only_on_mount(
                &self.canonical,
                limits,
                self.mount_id,
            );
        }
        #[cfg(not(target_os = "linux"))]
        WorkingProjectStore::open_existing(&self.canonical, limits)
    }

    pub(crate) fn revalidate(&self) -> Result<(), RetainedStoreRootError> {
        #[cfg(target_os = "linux")]
        self.monitor
            .revalidate()
            .map_err(|_| RetainedStoreRootError::Changed)?;
        self.held
            .revalidate()
            .map_err(|_| RetainedStoreRootError::Changed)?;
        #[cfg(target_os = "linux")]
        revalidate_linux_mount(&self.canonical, self.mount_id)
            .map_err(|_| RetainedStoreRootError::Changed)?;
        #[cfg(target_os = "linux")]
        self.monitor
            .revalidate()
            .map_err(|_| RetainedStoreRootError::Changed)?;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn revalidate_linux_mount(
    path: &Path,
    expected: WorkingStoreLinuxMountId,
) -> Result<(), RetainedStoreRootError> {
    let rebound = SecureDirectDirectory::open(path).map_err(|_| RetainedStoreRootError::Changed)?;
    let actual = WorkingStoreLinuxMountId::from_open_file(rebound.retained_directory_handle())
        .map_err(|_| RetainedStoreRootError::Changed)?;
    if actual != expected {
        return Err(RetainedStoreRootError::Changed);
    }
    Ok(())
}

fn normalized_absolute_path(path: &Path) -> io::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };

    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "managed Store root must not contain parent traversal",
                ));
            }
        }
    }
    if !normalized.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "managed Store root has no absolute filesystem root",
        ));
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn stable_real_directory_is_retained_and_parent_traversal_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("nested").join("store");
        fs::create_dir_all(&root).unwrap();

        let retained = RetainedStoreRoot::capture(&root).unwrap();
        assert!(retained.canonical().is_absolute());
        assert_eq!(retained.revalidate(), Ok(()));
        assert!(matches!(
            RetainedStoreRoot::capture(&root.join("..")),
            Err(RetainedStoreRootError::Unavailable)
        ));
    }

    #[test]
    fn link_or_reparse_ancestor_is_never_accepted_as_the_store_root_chain() {
        let temp = tempfile::tempdir().unwrap();
        let outside = temp.path().join("outside");
        let linked = temp.path().join("linked");
        fs::create_dir(&outside).unwrap();
        fs::create_dir(outside.join("store")).unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &linked).unwrap();
        #[cfg(windows)]
        {
            let status = std::process::Command::new("cmd")
                .args(["/c", "mklink", "/J"])
                .arg(&linked)
                .arg(&outside)
                .status()
                .unwrap();
            if !status.success() {
                return;
            }
        }

        assert!(matches!(
            RetainedStoreRoot::capture(&linked.join("store")),
            Err(RetainedStoreRootError::Unavailable)
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn retained_root_detects_intermediate_directory_symlink_swap() {
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().join("parent");
        let moved_parent = temp.path().join("moved-parent");
        let replacement = temp.path().join("replacement");
        let root = parent.join("store");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(replacement.join("store")).unwrap();
        let retained = RetainedStoreRoot::capture(&root).unwrap();

        fs::rename(&parent, &moved_parent).unwrap();
        std::os::unix::fs::symlink(&replacement, &parent).unwrap();

        assert_eq!(retained.revalidate(), Err(RetainedStoreRootError::Changed));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn retained_linux_monitor_detects_transient_swap_after_original_chain_is_restored() {
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().join("parent");
        let moved_parent = temp.path().join("moved-parent");
        let replacement = temp.path().join("replacement");
        let root = parent.join("store");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(replacement.join("store")).unwrap();
        let retained = RetainedStoreRoot::capture(&root).unwrap();

        fs::rename(&parent, &moved_parent).unwrap();
        std::os::unix::fs::symlink(&replacement, &parent).unwrap();
        fs::remove_file(&parent).unwrap();
        fs::rename(&moved_parent, &parent).unwrap();

        assert_eq!(retained.revalidate(), Err(RetainedStoreRootError::Changed));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn retained_linux_mount_id_binds_ambient_root_and_store_constructor() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("store");
        fs::create_dir(&root).unwrap();
        let mut retained = RetainedStoreRoot::capture(&root).unwrap();

        assert_eq!(revalidate_linux_mount(&root, retained.mount_id), Ok(()));

        let proc = SecureDirectDirectory::open(Path::new("/proc")).unwrap();
        let foreign =
            WorkingStoreLinuxMountId::from_open_file(proc.retained_directory_handle()).unwrap();
        if foreign == retained.mount_id {
            return;
        }
        assert_eq!(
            revalidate_linux_mount(&root, foreign),
            Err(RetainedStoreRootError::Changed)
        );

        retained.mount_id = foreign;
        let failure = retained
            .open_existing_store(WorkingStoreLimits::default())
            .unwrap_err();
        assert!(failure.is_read_mount_changed());
    }

    #[cfg(all(unix, not(target_os = "linux")))]
    #[test]
    fn unix_without_a_retained_change_primitive_fails_capture_closed() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("store");
        fs::create_dir(&root).unwrap();
        assert!(matches!(
            RetainedStoreRoot::capture(&root),
            Err(RetainedStoreRootError::Unavailable)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn retained_windows_ancestor_handles_block_intermediate_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().join("parent");
        let moved_parent = temp.path().join("moved-parent");
        let root = parent.join("store");
        fs::create_dir_all(&root).unwrap();
        let retained = RetainedStoreRoot::capture(&root).unwrap();

        // Every path component is held without DELETE sharing, so the attacker cannot create the
        // replacement window while the guard participates in planning.
        assert!(fs::rename(&parent, &moved_parent).is_err());
        assert_eq!(retained.revalidate(), Ok(()));
    }
}
