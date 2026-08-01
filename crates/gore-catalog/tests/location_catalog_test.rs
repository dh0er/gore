//! The named-location catalog generator, checked against the asset it produced.
//!
//! The bar for this port is byte parity, the same one
//! `gore catalog --kind item|npc|knowledge` holds to: the generator is a
//! transcription of `scripts/build_location_catalog.py`, and a transcription
//! that quietly rounds one coordinate differently or breaks a tie the other way
//! is not a transcription. Comparing the bytes is the only check that notices.
//!
//! The input is the game's own `InteractionSpots.json`, so the test is gated on
//! that file being on disk: set `GORE_INTERACTION_SPOTS` to point at it, or have
//! the game installed where the default expects. Without it the test skips, so a
//! machine with no game installation still runs a green suite.

use std::path::PathBuf;

use gore_catalog::location::{build_location_catalog, LocationCatalog, BUNDLED_CATALOG_JSON};

/// `$GAME\G1R\Script\Map\MainMap\InteractionSpots.json`, or `None` when it is
/// not on this machine.
fn interaction_spots() -> Option<PathBuf> {
    let path = std::env::var_os("GORE_INTERACTION_SPOTS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(
                r"D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Script\Map\MainMap\InteractionSpots.json",
            )
        });
    path.is_file().then_some(path)
}

#[test]
fn regenerating_reproduces_the_bundled_asset_byte_for_byte() {
    let Some(source) = interaction_spots() else {
        eprintln!("skip: set GORE_INTERACTION_SPOTS to the game's InteractionSpots.json");
        return;
    };
    let text = std::fs::read_to_string(&source).expect("the game's spot list is readable text");

    // `None`: the loc-id verification pass can only ever drop ids, and what it
    // would drop depends on whether this machine has extracted the shared
    // localization catalog. The asset ships the curated ids, so parity is
    // checked against the unverified build.
    let build = build_location_catalog(&text, None).expect("the game's spot list parses");

    assert_eq!(
        build.json.len(),
        BUNDLED_CATALOG_JSON.len(),
        "regenerated catalog is {} bytes, the bundled asset is {}",
        build.json.len(),
        BUNDLED_CATALOG_JSON.len()
    );
    if build.json != BUNDLED_CATALOG_JSON {
        let at = build
            .json
            .bytes()
            .zip(BUNDLED_CATALOG_JSON.bytes())
            .position(|(a, b)| a != b)
            .expect("the lengths matched, so a difference has an offset");
        let window = at.saturating_sub(60)..(at + 60).min(build.json.len());
        panic!(
            "regenerated catalog differs from the bundled asset at byte {at}\n\
             generated: …{}…\n bundled: …{}…",
            &build.json[window.clone()],
            &BUNDLED_CATALOG_JSON[window]
        );
    }
}

#[test]
fn the_bundled_asset_is_the_shape_the_editor_and_the_lookup_expect() {
    let catalog = LocationCatalog::parse(BUNDLED_CATALOG_JSON).expect("the asset is valid JSON");
    assert_eq!(catalog.version, 1);
    assert!(!catalog.areas.is_empty());
    assert!(
        catalog.spots.len() > 10_000,
        "the main map has ten thousand named spots"
    );

    // Every area code a spot carries has to be in the table, or the editor
    // shows a spot filed under an area it cannot name.
    for spot in &catalog.spots {
        if !spot.a.is_empty() {
            assert!(
                catalog.area(&spot.a).is_some(),
                "{} names area {}",
                spot.n,
                spot.a
            );
        }
    }

    // Names are unique: the lookup resolves the first match and would otherwise
    // be answering with an arbitrary one of several.
    let mut names: Vec<&str> = catalog.spots.iter().map(|spot| spot.n.as_str()).collect();
    names.sort_unstable();
    let total = names.len();
    names.dedup();
    assert_eq!(names.len(), total, "duplicate spot names in the catalog");
}
