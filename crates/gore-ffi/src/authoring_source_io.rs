use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    volume: u64,
    file: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChangeStamp([i64; 4]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HandleSnapshot {
    identity: FileIdentity,
    byte_len: u64,
    link_count: u64,
    change_stamp: ChangeStamp,
    is_directory: bool,
    is_reparse: bool,
}

#[derive(Debug)]
pub(super) enum SourceReadError {
    Missing,
    Unsafe,
    Limit,
    Changed,
    Io,
}

pub(super) fn read_source_no_follow(
    path: &Path,
    max_bytes: u64,
) -> Result<Vec<u8>, SourceReadError> {
    let (mut file, initial) = open_regular_no_follow(path)?;
    if initial.byte_len == 0 || initial.byte_len > max_bytes {
        return Err(SourceReadError::Limit);
    }
    let capacity = usize::try_from(initial.byte_len).map_err(|_| SourceReadError::Limit)?;
    let mut bytes = Vec::with_capacity(capacity);
    let read_limit = max_bytes.checked_add(1).ok_or(SourceReadError::Limit)?;
    file.by_ref()
        .take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(|_| SourceReadError::Io)?;
    if bytes.len() as u64 > max_bytes {
        return Err(SourceReadError::Limit);
    }
    if bytes.len() as u64 != initial.byte_len {
        return Err(SourceReadError::Changed);
    }
    let final_snapshot = snapshot_open_handle(&file)?;
    validate_snapshot(final_snapshot)?;
    if final_snapshot != initial {
        return Err(SourceReadError::Changed);
    }
    let (_reopened, reopened) = open_regular_no_follow(path)?;
    if reopened != initial {
        return Err(SourceReadError::Changed);
    }
    Ok(bytes)
}

fn open_regular_no_follow(path: &Path) -> Result<(File, HandleSnapshot), SourceReadError> {
    let file = open_regular_handle_no_follow(path).map_err(classify_open_error)?;
    let snapshot = snapshot_open_handle(&file)?;
    validate_snapshot(snapshot)?;
    Ok((file, snapshot))
}

fn validate_snapshot(snapshot: HandleSnapshot) -> Result<(), SourceReadError> {
    if snapshot.is_directory || snapshot.is_reparse || snapshot.link_count != 1 {
        return Err(SourceReadError::Unsafe);
    }
    Ok(())
}

fn classify_open_error(error: io::Error) -> SourceReadError {
    if error.kind() == io::ErrorKind::NotFound {
        return SourceReadError::Missing;
    }
    #[cfg(unix)]
    if error.raw_os_error() == Some(libc::ELOOP) {
        return SourceReadError::Unsafe;
    }
    SourceReadError::Io
}

#[cfg(windows)]
fn open_regular_handle_no_follow(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    options.open(path)
}

#[cfg(unix)]
fn open_regular_handle_no_follow(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    options.open(path)
}

#[cfg(windows)]
fn snapshot_open_handle(file: &File) -> Result<HandleSnapshot, SourceReadError> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        FileBasicInfo, GetFileInformationByHandle, GetFileInformationByHandleEx,
        BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_BASIC_INFO,
    };

    let mut info = unsafe { std::mem::zeroed::<BY_HANDLE_FILE_INFORMATION>() };
    // SAFETY: `file` owns a valid handle and `info` is writable for the call.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(SourceReadError::Io);
    }
    let mut basic = FILE_BASIC_INFO::default();
    // SAFETY: `file` owns a valid handle and `basic` is a correctly sized writable buffer.
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileBasicInfo,
            std::ptr::addr_of_mut!(basic).cast(),
            std::mem::size_of::<FILE_BASIC_INFO>() as u32,
        )
    } == 0
    {
        return Err(SourceReadError::Io);
    }
    Ok(HandleSnapshot {
        identity: FileIdentity {
            volume: u64::from(info.dwVolumeSerialNumber),
            file: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        },
        byte_len: (u64::from(info.nFileSizeHigh) << 32) | u64::from(info.nFileSizeLow),
        link_count: u64::from(info.nNumberOfLinks),
        change_stamp: ChangeStamp([
            basic.ChangeTime,
            basic.LastWriteTime,
            basic.CreationTime,
            i64::from(basic.FileAttributes),
        ]),
        is_directory: info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0,
        is_reparse: info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0,
    })
}

#[cfg(unix)]
fn snapshot_open_handle(file: &File) -> Result<HandleSnapshot, SourceReadError> {
    use std::os::unix::fs::MetadataExt;

    let metadata = file.metadata().map_err(|_| SourceReadError::Io)?;
    Ok(HandleSnapshot {
        identity: FileIdentity {
            volume: metadata.dev(),
            file: metadata.ino(),
        },
        byte_len: metadata.len(),
        link_count: metadata.nlink(),
        change_stamp: ChangeStamp([
            metadata.mtime(),
            metadata.mtime_nsec(),
            metadata.ctime(),
            metadata.ctime_nsec(),
        ]),
        is_directory: metadata.is_dir(),
        is_reparse: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn guarded_reader_is_bounded_and_rejects_hard_links() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source.cache");
        fs::write(&source, b"bounded source").unwrap();
        assert_eq!(
            read_source_no_follow(&source, 1024).unwrap(),
            b"bounded source"
        );
        assert!(matches!(
            read_source_no_follow(&source, 3),
            Err(SourceReadError::Limit)
        ));

        let alias = root.path().join("alias.cache");
        fs::hard_link(&source, &alias).unwrap();
        assert!(matches!(
            read_source_no_follow(&source, 1024),
            Err(SourceReadError::Unsafe)
        ));
    }
}
