use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::Read as _;
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};

const CATALOG_ENV: &str = "GORE_STANDALONE_COMPILER_CATALOG_PATH";
const CATALOG_SHA256_ENV: &str = "GORE_STANDALONE_COMPILER_CATALOG_SHA256";
const EMBEDDED_CATALOG_SHA256_ENV: &str = "GORE_EMBEDDED_STANDALONE_COMPILER_CATALOG_SHA256";
const MAX_CATALOG_BYTES: u64 = 256 * 1024;
const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[allow(clippy::permissions_set_readonly_false)] // The Windows-only branch clears FILE_ATTRIBUTE_READONLY.
fn make_generated_file_writable(path: &Path) {
    let mut permissions = fs::metadata(path)
        .expect("inspect prior generated embedded compiler catalog")
        .permissions();
    #[cfg(windows)]
    permissions.set_readonly(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        permissions.set_mode(permissions.mode() | 0o200);
    }
    fs::set_permissions(path, permissions)
        .expect("unseal prior generated embedded compiler catalog for replacement");
}

#[cfg(windows)]
fn windows_number_of_links(file: &File) -> u32 {
    use std::ffi::c_void;
    use std::os::windows::io::AsRawHandle as _;

    #[repr(C)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    #[repr(C)]
    struct ByHandleFileInformation {
        attributes: u32,
        creation_time: FileTime,
        last_access_time: FileTime,
        last_write_time: FileTime,
        volume_serial_number: u32,
        file_size_high: u32,
        file_size_low: u32,
        number_of_links: u32,
        file_index_high: u32,
        file_index_low: u32,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetFileInformationByHandle(
            file: *mut c_void,
            information: *mut ByHandleFileInformation,
        ) -> i32;
    }

    let mut information = std::mem::MaybeUninit::<ByHandleFileInformation>::uninit();
    // SAFETY: `file` owns a valid open handle and the output points to writable storage with the
    // exact Win32 BY_HANDLE_FILE_INFORMATION layout.
    let succeeded = unsafe {
        GetFileInformationByHandle(file.as_raw_handle().cast(), information.as_mut_ptr())
    };
    assert!(
        succeeded != 0,
        "cannot inspect open embedded compiler catalog file identity: {}",
        std::io::Error::last_os_error()
    );
    // SAFETY: Win32 reports success only after initializing the complete structure.
    unsafe { information.assume_init().number_of_links }
}

fn parse_sha256(value: &str) -> [u8; 32] {
    assert!(
        value.len() == 64,
        "embedded compiler catalog SHA-256 must contain 64 hex digits"
    );
    let mut output = [0u8; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .expect("embedded compiler catalog SHA-256 must be hexadecimal");
    }
    output
}

fn read_catalog_no_follow(path: &Path) -> Vec<u8> {
    let link = fs::symlink_metadata(path)
        .unwrap_or_else(|error| panic!("cannot inspect embedded compiler catalog: {error}"));
    assert!(
        link.is_file() && !link.file_type().is_symlink() && link.len() <= MAX_CATALOG_BYTES,
        "embedded compiler catalog must be a non-link regular file no larger than {MAX_CATALOG_BYTES} bytes"
    );

    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut file: File = options
        .open(path)
        .unwrap_or_else(|error| panic!("cannot open embedded compiler catalog no-follow: {error}"));
    let before = file
        .metadata()
        .unwrap_or_else(|error| panic!("cannot inspect open embedded compiler catalog: {error}"));
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        assert!(
            before.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0
                && windows_number_of_links(&file) == 1,
            "embedded compiler catalog must be non-reparse and single-link"
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        assert!(
            before.nlink() == 1,
            "embedded compiler catalog must be single-link"
        );
    }
    let mut bytes = Vec::with_capacity(before.len() as usize);
    file.by_ref()
        .take(MAX_CATALOG_BYTES + 1)
        .read_to_end(&mut bytes)
        .unwrap_or_else(|error| panic!("cannot read open embedded compiler catalog: {error}"));
    let after = file.metadata().unwrap_or_else(|error| {
        panic!("cannot re-inspect open embedded compiler catalog: {error}")
    });
    assert!(
        bytes.len() as u64 == before.len() && after.len() == before.len(),
        "embedded compiler catalog changed while held open"
    );
    bytes
}

fn main() {
    println!("cargo:rerun-if-env-changed={CATALOG_ENV}");
    println!("cargo:rerun-if-env-changed={CATALOG_SHA256_ENV}");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"))
        .join("product-standalone-compiler-catalog.json");
    let (bytes, expected_hex) = match env::var_os(CATALOG_ENV) {
        None => (Vec::new(), EMPTY_SHA256.to_owned()),
        Some(raw) => {
            let input = PathBuf::from(raw);
            println!("cargo:rerun-if-changed={}", input.display());
            let expected = env::var(CATALOG_SHA256_ENV)
                .unwrap_or_else(|_| panic!("{CATALOG_SHA256_ENV} is required with {CATALOG_ENV}"));
            (
                read_catalog_no_follow(&input),
                expected.to_ascii_lowercase(),
            )
        }
    };
    let expected = parse_sha256(&expected_hex);
    let actual: [u8; 32] = Sha256::digest(&bytes).into();
    assert_eq!(
        actual, expected,
        "embedded compiler catalog does not match its prepared SHA-256"
    );
    if output.exists() {
        make_generated_file_writable(&output);
    }
    fs::write(&output, bytes).expect("write generated embedded compiler catalog");
    let mut permissions = fs::metadata(&output)
        .expect("inspect generated embedded compiler catalog")
        .permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&output, permissions)
        .expect("seal generated embedded compiler catalog read-only");
    println!("cargo:rustc-env={EMBEDDED_CATALOG_SHA256_ENV}={expected_hex}");
}
