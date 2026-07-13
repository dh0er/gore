use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use gore_as::diagnostics::probe_executable;

struct ReleaseFixture {
    version: &'static str,
    byte_len: u64,
    sha256: &'static str,
    callback_rva: u64,
}

const RELEASE_FIXTURES: &[ReleaseFixture] = &[
    ReleaseFixture {
        version: "1.0.0",
        byte_len: 171_437_056,
        sha256: "740abfa9fbaae95beb5378c472ef4454df66205c140c3574eb5ba3695be53c55",
        callback_rva: 0x467e760,
    },
    ReleaseFixture {
        version: "1.0.1",
        byte_len: 171_482_112,
        sha256: "77f3d48ccde47756a6fa94b4b031f0ad58e2b57dcba93451415a5ed1af03f4ab",
        callback_rva: 0x467ea50,
    },
    ReleaseFixture {
        version: "1.0.2",
        byte_len: 171_627_008,
        sha256: "d9f45c72e624f6e27032379a7c3e51454562fd58a7eb9ac9cdaf6574c398afa9",
        callback_rva: 0x467e200,
    },
    ReleaseFixture {
        version: "1.0.3",
        byte_len: 171_698_176,
        sha256: "f406f969d3e73b6e58ea6e7aa10df7380318d97e7974d3be6e5a01183a4524f5",
        callback_rva: 0x467f5b0,
    },
];

fn default_fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("work")
        .join("reversing")
        .join("binaries")
}

/// Offline-only regression over the archived release executables. The fixtures are intentionally
/// not distributed with the crate. CI without them skips; setting `GORE_AS_RELEASE_MATRIX_DIR`
/// makes their absence a test failure. A locally present default matrix must also be complete so a
/// partially copied archive cannot look like four-version coverage.
#[test]
fn archived_release_executables_keep_the_verified_callback_capability() {
    let explicit_root = std::env::var_os("GORE_AS_RELEASE_MATRIX_DIR").map(PathBuf::from);
    let root = explicit_root.clone().unwrap_or_else(default_fixture_root);
    let present = RELEASE_FIXTURES
        .iter()
        .filter(|fixture| executable(&root, fixture.version).is_file())
        .count();
    if present == 0 && explicit_root.is_none() {
        eprintln!(
            "skip: archived diagnostics matrix is absent at {}; set GORE_AS_RELEASE_MATRIX_DIR",
            root.display()
        );
        return;
    }

    let mut hashes = BTreeSet::new();
    let mut rvas = BTreeSet::new();
    for fixture in RELEASE_FIXTURES {
        let exe = executable(&root, fixture.version);
        assert!(
            exe.is_file(),
            "archived release fixture {} is missing: {}",
            fixture.version,
            exe.display()
        );
        let byte_len = std::fs::metadata(&exe).unwrap().len();
        assert_eq!(
            byte_len, fixture.byte_len,
            "archived release {} byte length changed",
            fixture.version
        );

        let probe = probe_executable(&exe).unwrap();
        eprintln!(
            "G1R {}: bytes={} sha256={} matches={} rvas={:?} shape={}",
            fixture.version,
            byte_len,
            probe.sha256,
            probe.match_count,
            probe.matched_rvas,
            probe.callback_shape_verified
        );
        assert_eq!(
            probe.sha256, fixture.sha256,
            "archived release {} SHA-256 changed",
            fixture.version
        );
        assert_eq!(
            probe.match_count, 1,
            "archived release {} callback signature is not unique",
            fixture.version
        );
        assert_eq!(
            probe.matched_rvas,
            [fixture.callback_rva],
            "archived release {} callback RVA changed",
            fixture.version
        );
        assert!(
            probe.callback_shape_verified,
            "archived release {} callback layout is not verified",
            fixture.version
        );
        assert!(hashes.insert(probe.sha256));
        assert!(rvas.insert(fixture.callback_rva));
    }

    // Different RVAs across all four releases prove the regression is exercising AOB discovery,
    // not accidentally validating one build-specific fixed address.
    assert_eq!(hashes.len(), RELEASE_FIXTURES.len());
    assert_eq!(rvas.len(), RELEASE_FIXTURES.len());
}

fn executable(root: &Path, version: &str) -> PathBuf {
    root.join(version).join("G1R-Win64-Shipping.exe")
}
