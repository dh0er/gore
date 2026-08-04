//! `gore location-catalog` — build the named-location catalog from the game's
//! `InteractionSpots.json`.
//!
//! Its own subcommand rather than a fourth `catalog --kind`: every `--kind`
//! takes a UE4SS object dump, and this one takes a file the game ships loose,
//! so it needs neither a dump nor a running game. `story-catalog` is the same
//! shape for the same reason.
//!
//! Output is byte-identical to the `scripts/build_location_catalog.py` this
//! replaced — see `crates/gore-catalog/tests/location_catalog_test.rs`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use gore_catalog::location::{build_location_catalog, LocationCatalogReport};

/// Where the main map's interaction spots live inside an install.
const SPOTS_IN_GAME: &str = "G1R/Script/Map/MainMap/InteractionSpots.json";

pub fn run(source: Option<PathBuf>, out: PathBuf) -> Result<()> {
    let source = match source {
        Some(path) => path,
        None => gore_loc::config::game_root(None)
            .context(
                "no InteractionSpots.json given and no game path resolved — pass the file, or run \
                 'gore config set game-path <path>'",
            )?
            .join(SPOTS_IN_GAME),
    };

    let text = fs::read_to_string(&source)
        .with_context(|| format!("reading interaction spots '{}'", source.display()))?;
    let known = known_loc_ids();
    let build = build_location_catalog(&text, known.as_ref())
        .with_context(|| format!("parsing interaction spots '{}'", source.display()))?;

    print_report(&build.report, &source);

    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating '{}'", parent.display()))?;
        }
    }
    fs::write(&out, build.json.as_bytes())
        .with_context(|| format!("writing catalog to '{}'", out.display()))?;

    let size = build.json.len();
    println!("wrote {}", out.display());
    println!(
        "  {} areas, {} spots, {size} bytes ({:.0} KB)",
        build.report.areas,
        build.report.spots,
        size as f64 / 1024.0
    );
    Ok(())
}

/// Every id in the shared localization catalog, or `None` when it has not been
/// extracted on this machine.
///
/// The curated area labels reference loc ids by hand, and an id that no longer
/// exists would ship as a reference the editor silently cannot resolve. Reading
/// the catalog is the only way to notice; without one the ids ship unverified,
/// which is what the note says.
fn known_loc_ids() -> Option<BTreeSet<String>> {
    let path = gore_loc::paths::loc_catalog_path();
    let Ok(bytes) = fs::read(&path) else {
        println!(
            "note: {} not found; loc ids shipped unverified",
            path.display()
        );
        return None;
    };
    // Values are ignored: this only ever answers "does this id exist", and the
    // catalog is ~28 MB of per-language strings nothing here reads.
    match serde_json::from_slice::<BTreeMap<String, serde::de::IgnoredAny>>(&bytes) {
        Ok(catalog) => Some(catalog.into_keys().collect()),
        Err(error) => {
            println!(
                "note: {} is unreadable ({error}); loc ids shipped unverified",
                path.display()
            );
            None
        }
    }
}

/// The same summary the Python builder printed, in the same order: what came
/// in, what was dropped and why, what each labelling pass reached.
fn print_report(report: &LocationCatalogReport, source: &std::path::Path) {
    println!(
        "read {} interaction spots from {}",
        report.read,
        source.display()
    );
    println!("  dropped {} at (0,0,0)", report.dropped_zero);
    println!(
        "  dropped {} in dev-only data layers:",
        report.dropped_layers.values().sum::<usize>()
    );
    for (layer, count) in &report.dropped_layers {
        println!("    {count:4}  {layer}");
    }
    println!(
        "  dropped {} duplicate/unnamed (first wins)",
        report.dropped_duplicate
    );
    println!("  kept {}", report.kept);

    if let Some(verified) = report.verified_loc_ids {
        if report.dead_loc_ids.is_empty() {
            println!("  loc ids: all {verified} verified");
        } else {
            for (code, loc_id) in &report.dead_loc_ids {
                println!("  DEAD loc id, dropped to null: {code} -> {loc_id}");
            }
        }
    }

    println!(
        "pass A (lexical): {} labelled, {} left",
        report.pass_a,
        report.kept - report.pass_a
    );
    if !report.unused_codes.is_empty() {
        // A curated code that no spot name carries is the symptom of the two
        // naming schemes disagreeing (the Tundra is `HC` in the territory
        // classes and `TA` in every spot name), so it is printed, not hidden.
        println!(
            "  curated codes with no spot: {}",
            report.unused_codes.join(", ")
        );
    }
    println!(
        "pass B (spatial): {} labelled, {} left with no area",
        report.pass_b, report.unlabelled
    );
    if !report.outliers.is_empty() {
        println!(
            "  {} pass-B assignment(s) over 10000 uu from their nearest label:",
            report.outliers.len()
        );
        for (distance, name, area) in &report.outliers {
            println!("    {distance:9.0} uu  {area:<6} {name}");
        }
    }
}
