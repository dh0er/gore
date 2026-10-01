//! A normal CLI must carry the catalog that authenticates its compiler package.
//! Check the linked library's actual bytes, not just an environment variable in this crate.

pub const UNBUNDLED_DEVELOPMENT: bool = cfg!(all(feature = "development-cli", debug_assertions))
    && gore_as::standalone_package::EMBEDDED_PRODUCT_STANDALONE_COMPILER_CATALOG_JSON_V1.is_empty();

// Unit-test harnesses are not installable CLIs. Integration tests use the explicit debug feature;
// release builds must carry a catalog even with --all-features or --features development-cli.
#[cfg(not(test))]
const _: () = assert!(
    UNBUNDLED_DEVELOPMENT
        || !gore_as::standalone_package::EMBEDDED_PRODUCT_STANDALONE_COMPILER_CATALOG_JSON_V1.is_empty(),
    "GORE CLI build refused: no embedded standalone compiler catalog. Build an installable CLI with `python build.py gore-cli dist` (or `build --release`). For debug development/tests only, use `cargo build -p gore --features development-cli`. Never install an unbundled development CLI."
);

pub const VERSION: &str = if UNBUNDLED_DEVELOPMENT {
    concat!(env!("CARGO_PKG_VERSION"), "-development-unbundled")
} else {
    env!("CARGO_PKG_VERSION")
};

pub const ABOUT: &str = if UNBUNDLED_DEVELOPMENT {
    "GORE development CLI without a compiler package; not for installation. Use python build.py gore-cli dist for the product."
} else {
    "GORE Command Line Tools"
};
