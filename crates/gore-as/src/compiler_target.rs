//! Read-only, handle-pinned validation of the exact game artifacts selected by a compiler profile.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};

use sha1::{Digest as _, Sha1};
use sha2::Sha256;

use crate::compiler_profile::manifest::{
    CompilerProfileV1, FileSealV1, PeCodeViewV1, Sha1Digest, Sha256Digest,
};
use crate::standalone_sidecar::ValidatedCompilerProfilePackageV1;

const MAX_EXECUTABLE_BYTES_V1: u64 = 512 * 1024 * 1024;
const MAX_SHIPPING_CACHE_BYTES_V1: u64 = 512 * 1024 * 1024;
const MAX_BINDS_CACHE_BYTES_V1: u64 = 128 * 1024 * 1024;
const MAX_PE_SECTIONS_V1: usize = 256;
const MAX_PE_DEBUG_DIRECTORIES_V1: usize = 256;

/// Exact paths chosen by product-owned install resolution. FFI/wire callers must never supply
/// these paths as compiler-package authority.
#[derive(Debug, Clone, Copy)]
pub struct CompilerTargetInputPathsV1<'a> {
    pub executable: &'a Path,
    pub shipping_cache: &'a Path,
    pub binds_cache: &'a Path,
}

/// Opaque proof that EXE, Shipping and Binds all match one qualified profile.
///
/// The open handles and directory pins are retained for the complete compiler attempt. On Windows
/// they deliberately omit delete sharing, closing the replace/rename window between validation and
/// use. Shipping/Binds bytes come from those exact handles and require both profile SHA-256 and
/// Steam content SHA-1.
#[derive(Debug)]
pub struct ValidatedCompilerTargetInputsV1 {
    profile_sha256: Sha256Digest,
    executable: File,
    shipping: File,
    binds: File,
    shipping_bytes: Vec<u8>,
    binds_bytes: Vec<u8>,
    _directory_pins: Vec<File>,
    paths: CompilerTargetOwnedPathsV1,
}

#[derive(Debug, Clone)]
pub(crate) struct CompilerTargetOwnedPathsV1 {
    executable: PathBuf,
    shipping_cache: PathBuf,
    binds_cache: PathBuf,
}

pub(crate) struct CompilerTargetPinHandlesV1 {
    pub(crate) executable: File,
    pub(crate) shipping: File,
    pub(crate) binds: File,
    pub(crate) directory_pins: Vec<File>,
    pub(crate) paths: CompilerTargetOwnedPathsV1,
}

impl ValidatedCompilerTargetInputsV1 {
    pub fn load(
        package: &ValidatedCompilerProfilePackageV1,
        paths: CompilerTargetInputPathsV1<'_>,
    ) -> Result<Self, CompilerTargetInputError> {
        Self::load_profile(package.profile(), paths)
    }

    /// Qualification-only target pin. An unqualified profile may select inputs for an
    /// authorized oracle run, but it still cannot enter any product standalone path.
    pub(crate) fn load_unqualified_profile_for_qualification(
        profile: &CompilerProfileV1,
        paths: CompilerTargetInputPathsV1<'_>,
    ) -> Result<Self, CompilerTargetInputError> {
        if profile.qualification.qualified {
            return Err(CompilerTargetInputError::Mismatch(
                "qualification profile state",
            ));
        }
        Self::load_profile(profile, paths)
    }

    fn load_profile(
        profile: &CompilerProfileV1,
        paths: CompilerTargetInputPathsV1<'_>,
    ) -> Result<Self, CompilerTargetInputError> {
        #[cfg(not(windows))]
        {
            let _ = (profile, paths);
            return Err(CompilerTargetInputError::UnsupportedPlatform);
        }

        #[cfg(windows)]
        {
            let mut directory_pins = Vec::new();
            for path in [paths.executable, paths.shipping_cache, paths.binds_cache] {
                directory_pins.extend(pin_absolute_parent_chain(path)?);
            }

            let mut executable = open_regular_no_follow(paths.executable, "executable")?;
            verify_streaming_seal(
                &mut executable,
                &profile.oracle.executable,
                MAX_EXECUTABLE_BYTES_V1,
                "executable",
            )?;
            let codeview = read_pe_codeview(&mut executable)?;
            if !codeview
                .guid
                .eq_ignore_ascii_case(&profile.oracle.pe_codeview.guid)
                || codeview.age != profile.oracle.pe_codeview.age
            {
                return Err(CompilerTargetInputError::Mismatch("executable CodeView"));
            }

            let mut shipping = open_regular_no_follow(paths.shipping_cache, "Shipping cache")?;
            let shipping_bytes = read_and_verify_seal(
                &mut shipping,
                &profile.oracle.shipping_cache,
                MAX_SHIPPING_CACHE_BYTES_V1,
                "Shipping cache",
            )?;
            let mut binds = open_regular_no_follow(paths.binds_cache, "Binds cache")?;
            let binds_bytes = read_and_verify_seal(
                &mut binds,
                &profile.oracle.binds_cache,
                MAX_BINDS_CACHE_BYTES_V1,
                "Binds cache",
            )?;

            Ok(Self {
                profile_sha256: profile.profile_sha256,
                executable,
                shipping,
                binds,
                shipping_bytes,
                binds_bytes,
                _directory_pins: directory_pins,
                paths: CompilerTargetOwnedPathsV1 {
                    executable: paths.executable.to_path_buf(),
                    shipping_cache: paths.shipping_cache.to_path_buf(),
                    binds_cache: paths.binds_cache.to_path_buf(),
                },
            })
        }
    }

    pub fn profile_sha256(&self) -> Sha256Digest {
        self.profile_sha256
    }

    pub fn shipping_cache(&self) -> &[u8] {
        &self.shipping_bytes
    }

    pub fn binds_cache(&self) -> &[u8] {
        &self.binds_bytes
    }

    /// Keep the exact opened executable identity live without exposing its path.
    pub fn executable_handle(&self) -> &File {
        &self.executable
    }

    pub fn shipping_handle(&self) -> &File {
        &self.shipping
    }

    pub fn binds_handle(&self) -> &File {
        &self.binds
    }

    pub(crate) fn into_pin_handles(self) -> CompilerTargetPinHandlesV1 {
        CompilerTargetPinHandlesV1 {
            executable: self.executable,
            shipping: self.shipping,
            binds: self.binds,
            directory_pins: self._directory_pins,
            paths: self.paths,
        }
    }
}

/// Reacquire the complete no-follow parent chain after the compiler transaction has moved its
/// sibling JIT directory, then prove that all three names still resolve to the exact file objects
/// retained by the original validation handles. Windows directory handles that deny delete
/// sharing also deny renaming a child of that directory, so the transaction must briefly release
/// only those directory handles for its own quarantine rename. The target files themselves remain
/// open and non-replaceable throughout this handoff.
pub(crate) fn repin_compiler_target_parent_chains_v1(
    paths: &CompilerTargetOwnedPathsV1,
    executable: &File,
    shipping: &File,
    binds: &File,
) -> Result<Vec<File>, CompilerTargetInputError> {
    #[cfg(not(windows))]
    {
        let _ = (paths, executable, shipping, binds);
        return Err(CompilerTargetInputError::UnsupportedPlatform);
    }

    #[cfg(windows)]
    {
        let mut pins = Vec::new();
        for path in [
            paths.executable.as_path(),
            paths.shipping_cache.as_path(),
            paths.binds_cache.as_path(),
        ] {
            pins.extend(pin_absolute_parent_chain(path)?);
        }
        for (path, retained, label) in [
            (paths.executable.as_path(), executable, "executable"),
            (paths.shipping_cache.as_path(), shipping, "Shipping cache"),
            (paths.binds_cache.as_path(), binds, "Binds cache"),
        ] {
            let reopened = open_regular_no_follow(path, label)
                .map_err(|_| CompilerTargetInputError::Changed(label))?;
            if !same_windows_file_identity(retained, &reopened, label)? {
                return Err(CompilerTargetInputError::Changed(label));
            }
        }
        Ok(pins)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CompilerTargetInputError {
    #[error("compiler target identity pinning is supported only on Windows")]
    UnsupportedPlatform,
    #[error("compiler target {0} path is unsafe or unavailable")]
    UnsafePath(&'static str),
    #[error("compiler target {0} is not a single regular non-reparse file")]
    UnsafeFile(&'static str),
    #[error("compiler target {0} exceeds its bounded size")]
    TooLarge(&'static str),
    #[error("compiler target {0} changed while it was read")]
    Changed(&'static str),
    #[error("compiler target {0} does not match the compiler profile")]
    Mismatch(&'static str),
    #[error("compiler target executable has an invalid PE CodeView directory")]
    InvalidCodeView,
}

fn read_and_verify_seal(
    file: &mut File,
    expected: &FileSealV1,
    max: u64,
    label: &'static str,
) -> Result<Vec<u8>, CompilerTargetInputError> {
    let length = checked_length(file, expected, max, label)?;
    let capacity =
        usize::try_from(length).map_err(|_| CompilerTargetInputError::TooLarge(label))?;
    file.seek(SeekFrom::Start(0))
        .map_err(|_| CompilerTargetInputError::Changed(label))?;
    let mut bytes = Vec::with_capacity(capacity);
    file.take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| CompilerTargetInputError::Changed(label))?;
    if bytes.len() as u64 != length {
        return Err(CompilerTargetInputError::Changed(label));
    }
    verify_digests(&bytes, expected, label)?;
    if file
        .metadata()
        .map_err(|_| CompilerTargetInputError::Changed(label))?
        .len()
        != length
    {
        return Err(CompilerTargetInputError::Changed(label));
    }
    Ok(bytes)
}

fn verify_streaming_seal(
    file: &mut File,
    expected: &FileSealV1,
    max: u64,
    label: &'static str,
) -> Result<(), CompilerTargetInputError> {
    let length = checked_length(file, expected, max, label)?;
    file.seek(SeekFrom::Start(0))
        .map_err(|_| CompilerTargetInputError::Changed(label))?;
    let mut sha256 = Sha256::new();
    let mut sha1 = Sha1::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| CompilerTargetInputError::Changed(label))?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .filter(|value| *value <= max)
            .ok_or(CompilerTargetInputError::TooLarge(label))?;
        sha256.update(&buffer[..read]);
        sha1.update(&buffer[..read]);
    }
    if total != length
        || Sha256Digest::from_bytes(sha256.finalize().into()) != expected.sha256
        || expected.steam_content_sha1 != Some(Sha1Digest::from_bytes(sha1.finalize().into()))
    {
        return Err(CompilerTargetInputError::Mismatch(label));
    }
    if file
        .metadata()
        .map_err(|_| CompilerTargetInputError::Changed(label))?
        .len()
        != length
    {
        return Err(CompilerTargetInputError::Changed(label));
    }
    Ok(())
}

fn checked_length(
    file: &File,
    expected: &FileSealV1,
    max: u64,
    label: &'static str,
) -> Result<u64, CompilerTargetInputError> {
    let length = file
        .metadata()
        .map_err(|_| CompilerTargetInputError::UnsafeFile(label))?
        .len();
    if length == 0 || length > max || expected.byte_len > max {
        return Err(CompilerTargetInputError::TooLarge(label));
    }
    if length != expected.byte_len || expected.steam_content_sha1.is_none() {
        return Err(CompilerTargetInputError::Mismatch(label));
    }
    Ok(length)
}

fn verify_digests(
    bytes: &[u8],
    expected: &FileSealV1,
    label: &'static str,
) -> Result<(), CompilerTargetInputError> {
    let sha256 = Sha256Digest::from_bytes(Sha256::digest(bytes).into());
    let sha1 = Sha1Digest::from_bytes(Sha1::digest(bytes).into());
    if sha256 != expected.sha256 || expected.steam_content_sha1 != Some(sha1) {
        Err(CompilerTargetInputError::Mismatch(label))
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct SectionV1 {
    virtual_size: u32,
    virtual_address: u32,
    raw_size: u32,
    raw_offset: u32,
}

fn read_pe_codeview(file: &mut File) -> Result<PeCodeViewV1, CompilerTargetInputError> {
    let file_len = file
        .metadata()
        .map_err(|_| CompilerTargetInputError::InvalidCodeView)?
        .len();
    let mut dos = [0u8; 0x40];
    read_exact_at(file, 0, &mut dos, file_len)?;
    if &dos[..2] != b"MZ" {
        return Err(CompilerTargetInputError::InvalidCodeView);
    }
    let pe_offset = u64::from(le_u32(&dos[0x3c..0x40]));
    let mut pe = [0u8; 24];
    read_exact_at(file, pe_offset, &mut pe, file_len)?;
    if &pe[..4] != b"PE\0\0" {
        return Err(CompilerTargetInputError::InvalidCodeView);
    }
    let section_count = usize::from(le_u16(&pe[6..8]));
    let optional_size = usize::from(le_u16(&pe[20..22]));
    if section_count == 0
        || section_count > MAX_PE_SECTIONS_V1
        || optional_size < 176
        || optional_size > 4096
    {
        return Err(CompilerTargetInputError::InvalidCodeView);
    }
    let optional_offset = pe_offset
        .checked_add(24)
        .ok_or(CompilerTargetInputError::InvalidCodeView)?;
    let mut optional = vec![0u8; optional_size];
    read_exact_at(file, optional_offset, &mut optional, file_len)?;
    if le_u16(&optional[..2]) != 0x20b || le_u32(&optional[108..112]) <= 6 {
        return Err(CompilerTargetInputError::InvalidCodeView);
    }
    let debug_rva = le_u32(&optional[160..164]);
    let debug_size = le_u32(&optional[164..168]);
    if debug_rva == 0
        || debug_size < 28
        || debug_size % 28 != 0
        || debug_size as usize / 28 > MAX_PE_DEBUG_DIRECTORIES_V1
    {
        return Err(CompilerTargetInputError::InvalidCodeView);
    }

    let section_offset = optional_offset
        .checked_add(optional_size as u64)
        .ok_or(CompilerTargetInputError::InvalidCodeView)?;
    let section_bytes_len = section_count
        .checked_mul(40)
        .ok_or(CompilerTargetInputError::InvalidCodeView)?;
    let mut section_bytes = vec![0u8; section_bytes_len];
    read_exact_at(file, section_offset, &mut section_bytes, file_len)?;
    let sections = section_bytes
        .chunks_exact(40)
        .map(|section| SectionV1 {
            virtual_size: le_u32(&section[8..12]),
            virtual_address: le_u32(&section[12..16]),
            raw_size: le_u32(&section[16..20]),
            raw_offset: le_u32(&section[20..24]),
        })
        .collect::<Vec<_>>();
    let debug_offset = rva_to_file_offset(debug_rva, debug_size, &sections, file_len)?;
    let mut debug_bytes = vec![0u8; debug_size as usize];
    read_exact_at(file, debug_offset, &mut debug_bytes, file_len)?;

    let mut codeview = None;
    for directory in debug_bytes.chunks_exact(28) {
        if le_u32(&directory[12..16]) != 2 {
            continue;
        }
        let data_size = le_u32(&directory[16..20]);
        let raw_offset = u64::from(le_u32(&directory[24..28]));
        if data_size < 24
            || raw_offset
                .checked_add(u64::from(data_size))
                .is_none_or(|end| end > file_len)
        {
            return Err(CompilerTargetInputError::InvalidCodeView);
        }
        let mut rsds = [0u8; 24];
        read_exact_at(file, raw_offset, &mut rsds, file_len)?;
        if &rsds[..4] != b"RSDS" || codeview.is_some() {
            return Err(CompilerTargetInputError::InvalidCodeView);
        }
        codeview = Some(PeCodeViewV1 {
            guid: format_guid(&rsds[4..20]),
            age: le_u32(&rsds[20..24]),
        });
    }
    codeview.ok_or(CompilerTargetInputError::InvalidCodeView)
}

fn rva_to_file_offset(
    rva: u32,
    size: u32,
    sections: &[SectionV1],
    file_len: u64,
) -> Result<u64, CompilerTargetInputError> {
    let end_rva = rva
        .checked_add(size)
        .ok_or(CompilerTargetInputError::InvalidCodeView)?;
    for section in sections {
        let span = section.virtual_size.max(section.raw_size);
        let section_end = section
            .virtual_address
            .checked_add(span)
            .ok_or(CompilerTargetInputError::InvalidCodeView)?;
        if rva >= section.virtual_address && end_rva <= section_end {
            let within = rva - section.virtual_address;
            let raw_end = within
                .checked_add(size)
                .ok_or(CompilerTargetInputError::InvalidCodeView)?;
            if raw_end > section.raw_size {
                return Err(CompilerTargetInputError::InvalidCodeView);
            }
            let offset = u64::from(section.raw_offset) + u64::from(within);
            if offset
                .checked_add(u64::from(size))
                .is_none_or(|end| end > file_len)
            {
                return Err(CompilerTargetInputError::InvalidCodeView);
            }
            return Ok(offset);
        }
    }
    Err(CompilerTargetInputError::InvalidCodeView)
}

fn read_exact_at(
    file: &mut File,
    offset: u64,
    bytes: &mut [u8],
    file_len: u64,
) -> Result<(), CompilerTargetInputError> {
    if offset
        .checked_add(bytes.len() as u64)
        .is_none_or(|end| end > file_len)
    {
        return Err(CompilerTargetInputError::InvalidCodeView);
    }
    file.seek(SeekFrom::Start(offset))
        .and_then(|_| file.read_exact(bytes))
        .map_err(|_| CompilerTargetInputError::InvalidCodeView)
}

fn format_guid(bytes: &[u8]) -> String {
    format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        le_u32(&bytes[0..4]),
        le_u16(&bytes[4..6]),
        le_u16(&bytes[6..8]),
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15],
    )
}

fn le_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes(bytes.try_into().expect("fixed PE u16 slice"))
}

fn le_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().expect("fixed PE u32 slice"))
}

fn pin_absolute_parent_chain(path: &Path) -> Result<Vec<File>, CompilerTargetInputError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(CompilerTargetInputError::UnsafePath("artifact"));
    }
    let parent = path
        .parent()
        .ok_or(CompilerTargetInputError::UnsafePath("artifact"))?;
    let mut ancestors = parent.ancestors().collect::<Vec<_>>();
    ancestors.reverse();
    ancestors
        .into_iter()
        .map(open_directory_pin_no_follow)
        .collect()
}

#[cfg(windows)]
fn open_directory_pin_no_follow(path: &Path) -> Result<File, CompilerTargetInputError> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ,
        FILE_SHARE_READ,
    };
    let mut options = std::fs::OpenOptions::new();
    options
        .access_mode(FILE_GENERIC_READ)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    let file = options
        .open(path)
        .map_err(|_| CompilerTargetInputError::UnsafePath("parent"))?;
    let metadata = file
        .metadata()
        .map_err(|_| CompilerTargetInputError::UnsafePath("parent"))?;
    if !metadata.is_dir() || metadata_is_reparse(&metadata) {
        return Err(CompilerTargetInputError::UnsafePath("parent"));
    }
    Ok(file)
}

#[cfg(unix)]
fn open_directory_pin_no_follow(path: &Path) -> Result<File, CompilerTargetInputError> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| CompilerTargetInputError::UnsafePath("parent"))?;
    if !file
        .metadata()
        .map_err(|_| CompilerTargetInputError::UnsafePath("parent"))?
        .is_dir()
    {
        return Err(CompilerTargetInputError::UnsafePath("parent"));
    }
    Ok(file)
}

#[cfg(windows)]
fn open_regular_no_follow(
    path: &Path,
    label: &'static str,
) -> Result<File, CompilerTargetInputError> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ};
    let file = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(|_| CompilerTargetInputError::UnsafeFile(label))?;
    validate_open_regular(&file, label)?;
    Ok(file)
}

#[cfg(unix)]
fn open_regular_no_follow(
    path: &Path,
    label: &'static str,
) -> Result<File, CompilerTargetInputError> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| CompilerTargetInputError::UnsafeFile(label))?;
    validate_open_regular(&file, label)?;
    Ok(file)
}

#[cfg(windows)]
fn validate_open_regular(file: &File, label: &'static str) -> Result<(), CompilerTargetInputError> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY,
        FILE_ATTRIBUTE_REPARSE_POINT,
    };
    let mut info = unsafe { std::mem::zeroed::<BY_HANDLE_FILE_INFORMATION>() };
    // SAFETY: `file` owns a valid handle and `info` is writable for the call.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0
        || info.dwFileAttributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0
        || info.nNumberOfLinks != 1
    {
        return Err(CompilerTargetInputError::UnsafeFile(label));
    }
    Ok(())
}

#[cfg(windows)]
fn same_windows_file_identity(
    retained: &File,
    reopened: &File,
    label: &'static str,
) -> Result<bool, CompilerTargetInputError> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    fn identity(
        file: &File,
        label: &'static str,
    ) -> Result<(u32, u32, u32), CompilerTargetInputError> {
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        // SAFETY: `file` owns a valid handle and `info` is writable for the call.
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
            return Err(CompilerTargetInputError::Changed(label));
        }
        Ok((
            info.dwVolumeSerialNumber,
            info.nFileIndexHigh,
            info.nFileIndexLow,
        ))
    }

    Ok(identity(retained, label)? == identity(reopened, label)?)
}

#[cfg(unix)]
fn validate_open_regular(file: &File, label: &'static str) -> Result<(), CompilerTargetInputError> {
    use std::os::unix::fs::MetadataExt as _;
    let metadata = file
        .metadata()
        .map_err(|_| CompilerTargetInputError::UnsafeFile(label))?;
    if !metadata.is_file() || metadata.nlink() != 1 {
        return Err(CompilerTargetInputError::UnsafeFile(label));
    }
    Ok(())
}

#[cfg(windows)]
fn metadata_is_reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
    metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_reparse(metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_pe() -> (Vec<u8>, PeCodeViewV1) {
        let mut bytes = vec![0u8; 0x500];
        bytes[0..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        bytes[0x84..0x86].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
        bytes[0x94..0x96].copy_from_slice(&0xf0u16.to_le_bytes());
        let optional = 0x98usize;
        bytes[optional..optional + 2].copy_from_slice(&0x20bu16.to_le_bytes());
        bytes[optional + 108..optional + 112].copy_from_slice(&16u32.to_le_bytes());
        bytes[optional + 160..optional + 164].copy_from_slice(&0x1100u32.to_le_bytes());
        bytes[optional + 164..optional + 168].copy_from_slice(&28u32.to_le_bytes());
        let section = optional + 0xf0;
        bytes[section + 8..section + 12].copy_from_slice(&0x200u32.to_le_bytes());
        bytes[section + 12..section + 16].copy_from_slice(&0x1000u32.to_le_bytes());
        bytes[section + 16..section + 20].copy_from_slice(&0x300u32.to_le_bytes());
        bytes[section + 20..section + 24].copy_from_slice(&0x200u32.to_le_bytes());
        let debug = 0x300usize;
        bytes[debug + 12..debug + 16].copy_from_slice(&2u32.to_le_bytes());
        bytes[debug + 16..debug + 20].copy_from_slice(&24u32.to_le_bytes());
        bytes[debug + 24..debug + 28].copy_from_slice(&0x380u32.to_le_bytes());
        let guid = [
            0x67, 0x45, 0x23, 0x01, 0xab, 0x89, 0xef, 0xcd, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
            0xcd, 0xef,
        ];
        bytes[0x380..0x384].copy_from_slice(b"RSDS");
        bytes[0x384..0x394].copy_from_slice(&guid);
        bytes[0x394..0x398].copy_from_slice(&7u32.to_le_bytes());
        (
            bytes,
            PeCodeViewV1 {
                guid: "01234567-89ab-cdef-0123-456789abcdef".to_owned(),
                age: 7,
            },
        )
    }

    #[test]
    fn pe_codeview_parser_is_directory_bound_and_exact() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("synthetic.exe");
        let (bytes, expected) = synthetic_pe();
        std::fs::write(&path, &bytes).unwrap();
        let mut file = File::open(&path).unwrap();
        assert_eq!(read_pe_codeview(&mut file).unwrap(), expected);

        let mut invalid = bytes;
        invalid[0x300 + 24..0x300 + 28].copy_from_slice(&0x4f8u32.to_le_bytes());
        std::fs::write(&path, invalid).unwrap();
        let mut file = File::open(&path).unwrap();
        assert_eq!(
            read_pe_codeview(&mut file),
            Err(CompilerTargetInputError::InvalidCodeView)
        );
    }

    #[test]
    fn exact_sha256_and_steam_sha1_are_both_required() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("cache.bin");
        let bytes = b"sealed compiler input";
        std::fs::write(&path, bytes).unwrap();
        let seal = FileSealV1 {
            byte_len: bytes.len() as u64,
            sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
            steam_content_sha1: Some(Sha1Digest::from_bytes(Sha1::digest(bytes).into())),
        };
        let mut file = File::open(&path).unwrap();
        assert_eq!(
            read_and_verify_seal(&mut file, &seal, 1024, "fixture").unwrap(),
            bytes
        );

        let mut wrong_sha1 = seal;
        wrong_sha1.steam_content_sha1 = Some(Sha1Digest::from_bytes([0x5a; 20]));
        let mut file = File::open(&path).unwrap();
        assert_eq!(
            read_and_verify_seal(&mut file, &wrong_sha1, 1024, "fixture"),
            Err(CompilerTargetInputError::Mismatch("fixture"))
        );
    }

    #[cfg(windows)]
    #[test]
    fn target_file_and_parent_identity_remain_pinned_until_release() {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join("install/G1R/Script");
        std::fs::create_dir_all(&parent).unwrap();
        let path = parent.join("Binds.Cache");
        let displaced = parent.join("Binds.displaced");
        let moved_parent = root.path().join("moved-script");
        std::fs::write(&path, b"target").unwrap();

        let pins = pin_absolute_parent_chain(&path).unwrap();
        let file = open_regular_no_follow(&path, "fixture").unwrap();
        assert!(
            std::fs::rename(&path, &displaced).is_err(),
            "the opened target must deny path replacement"
        );
        assert!(
            std::fs::rename(&parent, &moved_parent).is_err(),
            "the pinned parent chain must deny directory replacement"
        );

        drop(file);
        drop(pins);
        std::fs::rename(&path, &displaced).unwrap();
        std::fs::rename(&parent, &moved_parent).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn target_parent_pins_can_be_identity_checked_across_sibling_quarantine() {
        let root = tempfile::tempdir().unwrap();
        let install = root.path().join("install");
        let win64 = install.join("G1R/Binaries/Win64");
        let script = install.join("G1R/Script");
        let jitted = install.join("AS_JITTED_CODE");
        let backup = install.join("AS_JITTED_CODE.gore-compile-bak");
        std::fs::create_dir_all(&win64).unwrap();
        std::fs::create_dir_all(&script).unwrap();
        std::fs::create_dir_all(&jitted).unwrap();
        let paths = CompilerTargetOwnedPathsV1 {
            executable: win64.join("G1R-Win64-Shipping.exe"),
            shipping_cache: script.join("PrecompiledScript_Shipping.Cache"),
            binds_cache: script.join("Binds.Cache"),
        };
        std::fs::write(&paths.executable, b"exe").unwrap();
        std::fs::write(&paths.shipping_cache, b"shipping").unwrap();
        std::fs::write(&paths.binds_cache, b"binds").unwrap();

        let mut directory_pins = Vec::new();
        for path in [
            paths.executable.as_path(),
            paths.shipping_cache.as_path(),
            paths.binds_cache.as_path(),
        ] {
            directory_pins.extend(pin_absolute_parent_chain(path).unwrap());
        }
        let executable = open_regular_no_follow(&paths.executable, "executable").unwrap();
        let shipping = open_regular_no_follow(&paths.shipping_cache, "Shipping cache").unwrap();
        let binds = open_regular_no_follow(&paths.binds_cache, "Binds cache").unwrap();
        assert!(
            std::fs::rename(&jitted, &backup).is_err(),
            "the install-root directory pin must explain the production quarantine conflict"
        );

        drop(directory_pins);
        std::fs::rename(&jitted, &backup).unwrap();
        let repinned =
            repin_compiler_target_parent_chains_v1(&paths, &executable, &shipping, &binds).unwrap();
        assert!(
            std::fs::rename(&backup, &jitted).is_err(),
            "the verified re-pin must immediately close the parent replacement window again"
        );
        drop(repinned);
        std::fs::rename(&backup, &jitted).unwrap();
    }
}
