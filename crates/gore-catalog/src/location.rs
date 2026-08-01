//! Named-location catalog: build it from the game's `InteractionSpots.json`,
//! and look spot names up in the bundled copy.
//!
//! # Where the data comes from
//!
//! The game ships every interaction spot of the main map as loose, unencrypted
//! JSON:
//!
//! ```text
//! $GAME\G1R\Script\Map\MainMap\InteractionSpots.json
//! ```
//!
//! Those spot names are exactly what a save file references when it records
//! where a character stands (`UsedSpot > Spotname`), and exactly what the
//! AngelScript teleport helpers take — so the catalog is the join table that
//! turns a raw coordinate triple into "Old Camp / FP_OC_STAND_YARD_1" and back.
//! [`build_location_catalog`] reduces the ~10 MB source down to the ~900 KB the
//! save editor bundles: no dev scaffolding, no duplicate names, one decimal of
//! precision, and yaw only.
//!
//! The output is **byte-identical** to the `scripts/build_location_catalog.py`
//! this module replaced, which is what
//! `crates/gore-catalog/tests/location_catalog_test.rs` asserts against the
//! committed asset whenever the game's JSON is on disk.
//!
//! # Why the lookup is here too
//!
//! `TeleportToWaypointAndExchangeDailyRoutineToClass(…, FName Waypoint)` and
//! `TeleportToSpot(…, FName)` both resolve through `FInteractionSpotHandle`,
//! whose `IsValid()` failure branch is empty: an unknown name is a **silent
//! no-op** — no log, no crash, nothing at all happens in game. Checking a name
//! against [`BUNDLED_CATALOG_JSON`] before launching is the only cheap way to
//! catch a typo, so the catalog is compiled into the binary and needs neither
//! the game nor a regenerated file to answer.
//!
//! The catalog is cook-specific: regenerate it after a game patch.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde::{Deserialize, Serialize};

/// Schema version written into (and expected from) the catalog JSON.
pub const SCHEMA_VERSION: u32 = 1;

/// The catalog the save editor bundles, compiled into this crate.
///
/// Embedded rather than located on disk so `gore location resolve` answers with
/// no game installation, no config and no prior generation step — the whole
/// point of the lookup is that it is cheaper than launching the game.
pub const BUNDLED_CATALOG_JSON: &str =
    include_str!("../../../apps/save-editor/assets/location_catalog.json");

// ─── Curated areas ───────────────────────────────────────────────────────────

/// Alias -> canonical area code.
///
/// Several codes are spellings of one place (the Exchange Zone alone appears as
/// `EZ`, `EZF` and `ExF`); they collapse onto one canonical area so the editor
/// groups them together instead of showing the same place thrice.
///
/// `TA` -> `HC` is the one case where the two spellings come from different
/// schemes rather than from sloppiness: the territory classes call the Tundra
/// `HC` (`UTerritoryConfig_HC_Tundra_EnclosedBasin`) but no spot name uses that
/// code — they all say `TA`. Without the alias the Tundra has no lexical anchor
/// at all and pass B scatters ~150 of its spots into the Old Mine.
const AREA_ALIASES: &[(&str, &str)] = &[
    ("AMR", "MR"), // Monastery Ruins
    ("EZF", "EZ"), // Exchange Zone
    ("ExF", "EZ"), // Exchange Zone
    ("FME", "FM"), // Free Mine
    ("NEF", "NC"), // New Camp
    ("OCC", "OC"), // Old Camp
    ("OCR", "OC"), // Old Camp
    ("TA", "HC"),  // Tundra
];

/// Canonical code -> (English label, localization id).
///
/// Derived from `Regions/Territory/TerritoryConfigs_<Area>.as` in the
/// decompiled AngelScript. A naive scan of the name prefixes yields 69 tokens,
/// most of them junk (`ROAM`, `HUT`, `PATH`, `BED`), so this allow-list — not
/// the token regex — is what decides whether a token is an area.
///
/// The loc ids are not mechanically derivable from the code, hence the explicit
/// mapping. The eight areas left with `None` are the ones the game has no clean
/// label for anywhere in its 43,851 strings — only quest titles and dialogue
/// lines mention them — so the editor names those itself, from its own ARB
/// (`locationArea*`), and falls back to the English label here only for an area
/// nobody has translated yet.
///
/// Left as one line per area on purpose: this is a hand-curated table that is
/// read and re-checked as a table after every game patch, and rustfmt breaks
/// the longer loc ids across four lines each, which turns 26 legible rows into
/// 130 lines nobody can scan.
#[rustfmt::skip]
const AREAS: &[(&str, &str, Option<&str>)] = &[
    ("AM",    "Abandoned Mine",      Some("area_abandonedmine_notification")),
    ("BC",    "Bandit Camp",         Some("area_banditscamp_plateau_notification")),
    ("CR",    "Castle Ruins",        Some("area_castleruins_keep_notification")),
    ("CV",    "Cavalorn Valley",     None),
    ("EF",    "East Forest",         None),
    ("EZ",    "Exchange Zone",       Some("area_exchangezone_elevator_notification")),
    ("FM",    "Free Mine",           Some("area_freemine_interior_notification")),
    ("FT",    "Fog Tower",           None),
    ("HC",    "Tundra",              None),
    ("IWM",   "Illegal Weed Mixers", None),
    ("MF",    "Mountain Fortress",   Some("area_mountainfortress_keep_notification")),
    ("MR",    "Monastery Ruins",     Some("area_monasteryruins_cloister_notification")),
    ("NC",    "New Camp",            Some("area_newcamp_notification")),
    ("OA",    "Orc Arena",           None),
    ("OC",    "Old Camp",            Some("area_oldcamp_notification")),
    ("OG",    "Orc Graveyard",       None),
    ("OM",    "Old Mine",            Some("area_oldmine_interior_notification")),
    ("OT",    "Orc Territory",       Some("area_orcterritory_notification")),
    ("OTOWN", "Orc Enclave",         Some("area_orcenclave_notification")),
    ("OW",    "Overworld",           Some("ui_map_valleyofmines")),
    ("SC",    "Swamp Camp",          Some("area_swampcamp_notification")),
    ("SNT",   "Sunken Tower",        Some("area_xardassunkentower_notification")),
    ("ST",    "Sleeper Temple",      Some("area_sleepertemple_notification")),
    ("SW",    "Shipwreck",           None),
    ("TC",    "Troll Canyon",        Some("area_trollcanyon_notification")),
    ("XT",    "Xardas Tower",        Some("area_xardastower_notification")),
];

// ─── Tuning constants ────────────────────────────────────────────────────────

/// Pass B: how many labelled neighbours vote on an unlabelled spot's area.
const NEIGHBOURS: usize = 5;
/// uu; beyond this the spot stays unlabelled rather than guessing.
const MAX_NEIGHBOUR_DISTANCE: f64 = 20_000.0;
/// uu; reported so the far guesses are visible in the summary.
const REPORT_NEIGHBOUR_DISTANCE: f64 = 10_000.0;
/// uu; the spatial index's cell edge.
const GRID_CELL: f64 = 5_000.0;

// ─── On-disk shape ───────────────────────────────────────────────────────────

/// One area of the map, as written to the catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AreaEntry {
    /// Canonical area code, e.g. `OC`.
    pub id: String,
    /// English label, used when there is no localized name.
    pub label: String,
    /// Localization id, or `null` when the area has no localized name.
    #[serde(rename = "locId")]
    pub loc_id: Option<String>,
}

/// One named spot. The keys are single letters because there are ten thousand
/// of them and the asset ships inside a desktop app.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpotEntry {
    /// Spot name — the `FName` the game and the save file use.
    pub n: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// Yaw only. A spot's pitch would visibly tilt a standing pawn, so pitch
    /// and roll are absent from the asset rather than merely ignored.
    pub w: f64,
    /// Area code into [`LocationCatalog::areas`], or `""` when unlabelled.
    pub a: String,
}

/// The catalog as a whole. Field order is the serialized key order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocationCatalog {
    pub version: u32,
    pub areas: Vec<AreaEntry>,
    pub spots: Vec<SpotEntry>,
}

impl LocationCatalog {
    /// Parse a catalog JSON document.
    pub fn parse(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    /// The catalog compiled into this binary ([`BUNDLED_CATALOG_JSON`]).
    pub fn bundled() -> serde_json::Result<Self> {
        Self::parse(BUNDLED_CATALOG_JSON)
    }

    /// The spot with this name, compared **case-insensitively**.
    ///
    /// `FName` comparison in the game is case-insensitive, and the spellings
    /// demonstrably drift: the same waypoint is `WP_ExF_…` in AngelScript and
    /// `WP_EXf_…` in a save. A case-sensitive lookup would report a spot that
    /// exists as missing, which is exactly the wrong answer for a typo-catcher.
    pub fn resolve(&self, name: &str) -> Option<&SpotEntry> {
        self.spots
            .iter()
            .find(|spot| spot.n.eq_ignore_ascii_case(name))
    }

    /// The area with this code, compared case-insensitively.
    pub fn area(&self, id: &str) -> Option<&AreaEntry> {
        self.areas
            .iter()
            .find(|area| area.id.eq_ignore_ascii_case(id))
    }

    /// Spots filtered by area code and/or name prefix, both case-insensitive.
    /// Order is the catalog's own: by area, then by name.
    pub fn list(&self, area: Option<&str>, prefix: Option<&str>) -> Vec<&SpotEntry> {
        self.spots
            .iter()
            .filter(|spot| match area {
                Some(code) => spot.a.eq_ignore_ascii_case(code),
                None => true,
            })
            .filter(|spot| match prefix {
                // Byte-sliced rather than `str`-sliced: a caller's prefix may be
                // any UTF-8 they typed, and a char boundary inside a name is not
                // this function's business to panic over.
                Some(start) => spot
                    .n
                    .as_bytes()
                    .get(..start.len())
                    .is_some_and(|head| head.eq_ignore_ascii_case(start.as_bytes())),
                None => true,
            })
            .collect()
    }

    /// Up to `limit` names closest to `name` by edit distance — the suggestion
    /// list a failed [`LocationCatalog::resolve`] prints.
    ///
    /// Bounded by a distance budget that grows with the name's length, so a
    /// short name does not match half the catalog and a long one still
    /// tolerates the two-character slip that produced the miss.
    pub fn suggest(&self, name: &str, limit: usize) -> Vec<&str> {
        let needle: Vec<u8> = name.bytes().map(|b| b.to_ascii_lowercase()).collect();
        let budget = (needle.len() / 5).clamp(2, 5);

        let mut scored: Vec<(usize, &str)> = Vec::new();
        let mut candidate: Vec<u8> = Vec::new();
        for spot in &self.spots {
            candidate.clear();
            candidate.extend(spot.n.bytes().map(|b| b.to_ascii_lowercase()));
            // Cheap reject before the quadratic part: a length gap alone
            // already exceeds the budget.
            if candidate.len().abs_diff(needle.len()) > budget {
                continue;
            }
            if let Some(distance) = bounded_edit_distance(&needle, &candidate, budget) {
                scored.push((distance, spot.n.as_str()));
            }
        }
        // Ties break on the name so the suggestion list is stable run to run.
        scored.sort_unstable();
        scored.truncate(limit);
        scored.into_iter().map(|(_, name)| name).collect()
    }
}

/// Levenshtein distance, or `None` once it is certain to exceed `budget`.
fn bounded_edit_distance(a: &[u8], b: &[u8], budget: usize) -> Option<usize> {
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current: Vec<usize> = vec![0; b.len() + 1];

    for (i, &ac) in a.iter().enumerate() {
        current[0] = i + 1;
        let mut row_best = current[0];
        for (j, &bc) in b.iter().enumerate() {
            let substitution = previous[j] + usize::from(ac != bc);
            current[j + 1] = substitution.min(previous[j + 1] + 1).min(current[j] + 1);
            row_best = row_best.min(current[j + 1]);
        }
        if row_best > budget {
            return None;
        }
        std::mem::swap(&mut previous, &mut current);
    }

    let distance = previous[b.len()];
    (distance <= budget).then_some(distance)
}

// ─── Generator ───────────────────────────────────────────────────────────────

/// The game's `InteractionSpots.json`. Every other field of a spot is ignored.
#[derive(Deserialize)]
struct SourceFile {
    #[serde(rename = "interactionSpots")]
    interaction_spots: Vec<SourceSpot>,
}

#[derive(Deserialize)]
struct SourceSpot {
    #[serde(default)]
    location: Option<Vec3>,
    #[serde(default)]
    rotation: Option<Rotation>,
    #[serde(default)]
    name: Option<String>,
    /// `/Game/…/DataLayers/MainMap/Foo.Foo`, or the literal string `"None"`.
    #[serde(default, rename = "dataLayer")]
    data_layer: Option<String>,
}

#[derive(Default, Deserialize)]
struct Vec3 {
    #[serde(default)]
    x: f64,
    #[serde(default)]
    y: f64,
    #[serde(default)]
    z: f64,
}

#[derive(Default, Deserialize)]
struct Rotation {
    #[serde(default)]
    yaw: f64,
}

/// What one generator run did, for the summary the CLI prints.
#[derive(Debug, Clone, Default)]
pub struct LocationCatalogReport {
    /// Spots in the source file.
    pub read: usize,
    /// Dropped because they sit at the world origin (the `WayPoints` layer has
    /// no coordinates at all).
    pub dropped_zero: usize,
    /// Dropped per dev-only data layer, by layer name.
    pub dropped_layers: BTreeMap<String, usize>,
    /// Dropped as a duplicate or unnamed spot (first wins).
    pub dropped_duplicate: usize,
    /// Spots that survived the filters.
    pub kept: usize,
    /// Loc ids checked against the shared catalog, when one was supplied.
    pub verified_loc_ids: Option<usize>,
    /// `(area code, loc id)` pairs dropped to `null` because the shared catalog
    /// does not have them.
    pub dead_loc_ids: Vec<(String, String)>,
    /// Spots labelled from their own name.
    pub pass_a: usize,
    /// Curated area codes that no spot name carries — the symptom of a code
    /// that only exists in the territory classes (see `TA`/`HC` above).
    pub unused_codes: Vec<String>,
    /// Spots labelled by the neighbour vote.
    pub pass_b: usize,
    /// Spots left with no area at all.
    pub unlabelled: usize,
    /// Pass-B assignments taken from far away, as
    /// `(distance, spot name, area)`, worst first.
    pub outliers: Vec<(f64, String, String)>,
    /// Areas in the written catalog.
    pub areas: usize,
    /// Spots in the written catalog.
    pub spots: usize,
}

/// One generated catalog plus the report of how it got that way.
#[derive(Debug, Clone)]
pub struct LocationCatalogBuild {
    /// The minified JSON document, ready to write verbatim.
    pub json: String,
    pub catalog: LocationCatalog,
    pub report: LocationCatalogReport,
}

/// Build the catalog from the text of the game's `InteractionSpots.json`.
///
/// `known_loc_ids` is every id in the shared localization catalog, when it is
/// available. Any curated loc id missing from it is dropped to `null` rather
/// than shipped as a reference the editor cannot resolve; pass `None` to ship
/// the curated ids unverified (which is also what keeps the byte-parity test
/// independent of whatever is on the machine running it).
pub fn build_location_catalog(
    source: &str,
    known_loc_ids: Option<&BTreeSet<String>>,
) -> serde_json::Result<LocationCatalogBuild> {
    // The game writes this file with a UTF-8 BOM on some cooks.
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let parsed: SourceFile = serde_json::from_str(source)?;

    let mut report = LocationCatalogReport {
        read: parsed.interaction_spots.len(),
        ..Default::default()
    };
    let mut seen: HashSet<String> = HashSet::new();
    let mut kept: Vec<Kept> = Vec::new();

    for spot in parsed.interaction_spots {
        let location = spot.location.unwrap_or_default();
        if location.x == 0.0 && location.y == 0.0 && location.z == 0.0 {
            report.dropped_zero += 1;
            continue;
        }
        let layer = short_layer(spot.data_layer.as_deref());
        if is_scaffolding(layer) {
            *report.dropped_layers.entry(layer.to_string()).or_insert(0) += 1;
            continue;
        }
        let name = spot.name.unwrap_or_default();
        if name.is_empty() || seen.contains(&name) {
            report.dropped_duplicate += 1;
            continue;
        }
        seen.insert(name.clone());
        kept.push(Kept {
            n: name,
            x: location.x,
            y: location.y,
            z: location.z,
            w: spot.rotation.unwrap_or_default().yaw,
            a: "",
        });
    }
    report.kept = kept.len();

    let areas = resolve_areas(known_loc_ids, &mut report);

    // Pass A — lexical. The stored code is the curated table's own `'static`
    // string, not the slice of the spot name it was recognised by.
    for spot in &mut kept {
        let curated = area_token(&spot.n)
            .map(canonical_code)
            .and_then(|code| areas.iter().find(|(id, _, _)| *id == code))
            .map(|(id, _, _)| *id);
        spot.a = curated.unwrap_or("");
    }
    let labelled: Vec<(f64, f64, f64, &'static str)> = kept
        .iter()
        .filter(|spot| !spot.a.is_empty())
        .map(|spot| (spot.x, spot.y, spot.z, spot.a))
        .collect();
    report.pass_a = labelled.len();
    let anchored: HashSet<&str> = labelled.iter().map(|(_, _, _, area)| *area).collect();
    report.unused_codes = areas
        .iter()
        .map(|(id, _, _)| *id)
        .filter(|id| !anchored.contains(id))
        .map(str::to_string)
        .collect();
    report.unused_codes.sort();

    // Pass B — spatial majority vote over the pass-A labels.
    let index = SpatialIndex::new(&labelled);
    for spot in &mut kept {
        if !spot.a.is_empty() {
            continue;
        }
        let near = index.nearest(spot.x, spot.y, spot.z, NEIGHBOURS);
        let Some(&(closest, _)) = near.first() else {
            report.unlabelled += 1;
            continue;
        };
        if closest > MAX_NEIGHBOUR_DISTANCE {
            report.unlabelled += 1;
            continue;
        }
        spot.a = majority(&near);
        report.pass_b += 1;
        if closest > REPORT_NEIGHBOUR_DISTANCE {
            report
                .outliers
                .push((closest, spot.n.clone(), spot.a.to_string()));
        }
    }
    // Worst first, matching the summary the Python builder printed.
    report
        .outliers
        .sort_by(|a, b| b.partial_cmp(a).unwrap_or(Ordering::Equal));

    let mut spots: Vec<SpotEntry> = kept
        .into_iter()
        .map(|spot| SpotEntry {
            n: spot.n,
            x: round1(spot.x),
            y: round1(spot.y),
            z: round1(spot.z),
            w: round1(spot.w),
            a: spot.a.to_string(),
        })
        .collect();
    spots.sort_by(|a, b| (&a.a, &a.n).cmp(&(&b.a, &b.n)));

    let used: BTreeSet<&str> = spots
        .iter()
        .filter(|spot| !spot.a.is_empty())
        .map(|spot| spot.a.as_str())
        .collect();
    let catalog = LocationCatalog {
        version: SCHEMA_VERSION,
        areas: used
            .iter()
            .map(|code| {
                let curated = areas
                    .iter()
                    .find(|(id, _, _)| id == code)
                    .expect("a used code came from the curated table");
                AreaEntry {
                    id: curated.0.to_string(),
                    label: curated.1.to_string(),
                    loc_id: curated.2.map(str::to_string),
                }
            })
            .collect(),
        spots,
    };

    report.areas = catalog.areas.len();
    report.spots = catalog.spots.len();
    let json = serde_json::to_string(&catalog)?;
    Ok(LocationCatalogBuild {
        json,
        catalog,
        report,
    })
}

/// A spot mid-pipeline, before rounding and before its area is decided.
struct Kept {
    n: String,
    x: f64,
    y: f64,
    z: f64,
    w: f64,
    a: &'static str,
}

/// [`AREAS`] with every loc id that is not actually in the shared catalog
/// dropped to `None`.
fn resolve_areas(
    known_loc_ids: Option<&BTreeSet<String>>,
    report: &mut LocationCatalogReport,
) -> Vec<(&'static str, &'static str, Option<&'static str>)> {
    let Some(known) = known_loc_ids else {
        return AREAS.to_vec();
    };
    report.verified_loc_ids = Some(AREAS.iter().filter(|(_, _, id)| id.is_some()).count());

    AREAS
        .iter()
        .map(|(code, label, loc_id)| {
            let live = loc_id.filter(|id| known.contains(*id));
            if loc_id.is_some() && live.is_none() {
                report
                    .dead_loc_ids
                    .push(((*code).to_string(), loc_id.expect("checked").to_string()));
            }
            (*code, *label, live)
        })
        .collect()
}

/// `/Game/…/DataLayers/MainMap/Foo.Foo` -> `Foo`; `None`/missing -> `""`.
fn short_layer(raw: Option<&str>) -> &str {
    match raw {
        None | Some("None") => "",
        Some(path) => {
            let tail = path.rsplit('/').next().unwrap_or(path);
            tail.split('.').next().unwrap_or(tail)
        }
    }
}

/// Data layers that only exist for level design: greybox geometry, the loading
/// screen photo set, and demo builds. Conditional `MainQuest_*` / story layers
/// are kept — relocating a character into a quest area is a legitimate edit.
fn is_scaffolding(layer: &str) -> bool {
    layer.ends_with("_Blockout") || layer == "LoadingScreenShots" || layer.starts_with("Demo_")
}

/// The area token of a spot name: `FP_`**`OC`**`_STAND_YARD_1`, `IO_`**`SC`**`_ANVIL_2`.
///
/// Deliberately wider than the all-caps 2-4 letters most codes use — `OTOWN` is
/// five letters and `ExF` is mixed case — because [`AREAS`], matched
/// case-sensitively, is the gate.
fn area_token(name: &str) -> Option<&str> {
    let bytes = name.as_bytes();
    let separator = bytes.iter().position(|byte| *byte == b'_')?;
    if separator == 0 || !bytes[..separator].iter().all(u8::is_ascii_alphabetic) {
        return None;
    }
    let start = separator + 1;
    let end = start
        + bytes[start..]
            .iter()
            .take_while(|b| b.is_ascii_alphabetic())
            .count();
    // The token has to end where it ends: a longer or shorter run would not be
    // followed by the `_` that closes it.
    if !(2..=5).contains(&(end - start)) || bytes.get(end) != Some(&b'_') {
        return None;
    }
    Some(&name[start..end])
}

/// An area token mapped through [`AREA_ALIASES`], or itself.
fn canonical_code(token: &str) -> &str {
    AREA_ALIASES
        .iter()
        .find(|(alias, _)| *alias == token)
        .map_or(token, |(_, canonical)| *canonical)
}

/// The most common area among the neighbours; ties go to the **nearer** one.
///
/// `near` is ordered by distance and the tally keeps first-seen order, so the
/// tie-break falls to whichever area a closer neighbour belongs to. Expressed
/// as `min_by_key(Reverse(count))` because `max_by_key` returns the *last*
/// maximum, which would hand a tie to the farther area instead.
fn majority(near: &[(f64, &'static str)]) -> &'static str {
    let mut counts: Vec<(&'static str, usize)> = Vec::with_capacity(near.len());
    for (_, area) in near {
        match counts.iter_mut().find(|(name, _)| name == area) {
            Some((_, count)) => *count += 1,
            None => counts.push((area, 1)),
        }
    }
    counts
        .iter()
        .min_by_key(|(_, count)| std::cmp::Reverse(*count))
        .map_or("", |(area, _)| *area)
}

/// `round(value, 1)` with Python's semantics: correctly round the *exact*
/// binary value to one decimal (ties to even), then take the nearest double to
/// that decimal. `(v * 10.0).round() / 10.0` is a different function and
/// disagrees on values this catalog is full of.
fn round1(value: f64) -> f64 {
    format!("{value:.1}").parse().unwrap_or(value)
}

/// Uniform grid over the pass-A spots, for exact k-nearest queries.
///
/// A plain O(n·m) scan over ~7,800 labelled and ~2,300 unlabelled spots is
/// quadratic busywork; this keeps the whole run well under a second.
struct SpatialIndex<'a> {
    points: &'a [(f64, f64, f64, &'static str)],
    cells: HashMap<(i64, i64, i64), Vec<usize>>,
}

impl<'a> SpatialIndex<'a> {
    fn new(points: &'a [(f64, f64, f64, &'static str)]) -> Self {
        let mut cells: HashMap<(i64, i64, i64), Vec<usize>> = HashMap::new();
        for (index, point) in points.iter().enumerate() {
            cells
                .entry(cell(point.0, point.1, point.2))
                .or_default()
                .push(index);
        }
        Self { points, cells }
    }

    /// The `k` nearest points as `(distance, area)`, nearest first.
    fn nearest(&self, x: f64, y: f64, z: f64, k: usize) -> Vec<(f64, &'static str)> {
        let (cx, cy, cz) = cell(x, y, z);
        let mut found: Vec<(f64, &'static str)> = Vec::new();
        let mut ring: i64 = 0;
        loop {
            found.clear();
            for ix in cx - ring..=cx + ring {
                for iy in cy - ring..=cy + ring {
                    for iz in cz - ring..=cz + ring {
                        let Some(bucket) = self.cells.get(&(ix, iy, iz)) else {
                            continue;
                        };
                        for index in bucket {
                            let point = &self.points[*index];
                            found.push((distance(x, y, z, point.0, point.1, point.2), point.3));
                        }
                    }
                }
            }
            found.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));

            // Everything outside the searched block is at least ring*CELL away,
            // so once the k-th hit is closer than that the answer is exact.
            let guaranteed = ring as f64 * GRID_CELL;
            if found.len() >= k && found[k - 1].0 <= guaranteed {
                break;
            }
            if guaranteed > MAX_NEIGHBOUR_DISTANCE * 4.0 {
                break; // far enough out that the vote is moot anyway
            }
            ring += 1;
        }
        found.truncate(k);
        found
    }
}

fn cell(x: f64, y: f64, z: f64) -> (i64, i64, i64) {
    (
        (x / GRID_CELL).floor() as i64,
        (y / GRID_CELL).floor() as i64,
        (z / GRID_CELL).floor() as i64,
    )
}

fn distance(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> f64 {
    let (dx, dy, dz) = (ax - bx, ay - by, az - bz);
    (dx * dx + dy * dy + dz * dz).sqrt()
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// A miniature `InteractionSpots.json` exercising every filter.
    const SOURCE: &str = r#"{
      "interactionSpots": [
        {"location": {"x": 0.0, "y": 0.0, "z": 0.0}, "name": "FP_OC_AT_ORIGIN"},
        {"location": {"x": 10.04, "y": 20.0, "z": 30.0},
         "rotation": {"pitch": 5, "yaw": -159.999999, "roll": 0},
         "name": "FP_OC_STAND_YARD_1",
         "dataLayer": "None"},
        {"location": {"x": 11.0, "y": 21.0, "z": 31.0}, "name": "FP_OC_STAND_YARD_1"},
        {"location": {"x": 12.0, "y": 22.0, "z": 32.0}, "name": "FP_OC_STAND_YARD_2",
         "dataLayer": "/Game/Maps/DataLayers/MainMap/Camp_Blockout.Camp_Blockout"},
        {"location": {"x": 13.0, "y": 23.0, "z": 33.0}, "name": "FP_TA_TUNDRA_1"},
        {"location": {"x": 14.0, "y": 24.0, "z": 34.0}, "name": "NAMELESS_NEIGHBOUR"},
        {"location": {"x": 900000.0, "y": 900000.0, "z": 0.0}, "name": "FAR_AWAY_SPOT"}
      ]
    }"#;

    fn build() -> LocationCatalogBuild {
        build_location_catalog(SOURCE, None).expect("the fixture parses")
    }

    #[test]
    fn drops_origin_duplicates_and_dev_layers() {
        let report = build().report;
        assert_eq!(report.read, 7);
        assert_eq!(report.dropped_zero, 1);
        assert_eq!(report.dropped_duplicate, 1, "the second FP_OC_STAND_YARD_1");
        assert_eq!(report.dropped_layers.get("Camp_Blockout"), Some(&1));
        assert_eq!(report.kept, 4);
    }

    #[test]
    fn labels_lexically_then_by_neighbour_vote() {
        let build = build();
        assert_eq!(
            build.report.pass_a, 2,
            "FP_OC_… and FP_TA_… carry their own code"
        );
        assert_eq!(
            build.report.pass_b, 1,
            "NAMELESS_NEIGHBOUR sits on top of them"
        );
        assert_eq!(
            build.report.unlabelled, 1,
            "FAR_AWAY_SPOT has no labelled neighbour near"
        );
        assert_eq!(build.catalog.resolve("FAR_AWAY_SPOT").expect("kept").a, "");
    }

    #[test]
    fn the_tundra_alias_survives() {
        // TA is what every spot name says; HC is what the territory classes
        // call it. Dropping the alias silently scatters the Tundra.
        let build = build();
        assert_eq!(
            build.catalog.resolve("FP_TA_TUNDRA_1").expect("kept").a,
            "HC"
        );
        assert_eq!(build.catalog.area("HC").expect("used").label, "Tundra");
    }

    #[test]
    fn rounds_to_one_decimal_and_keeps_yaw_only() {
        let build = build();
        let spot = build.catalog.resolve("FP_OC_STAND_YARD_1").expect("kept");
        assert_eq!(spot.x, 10.0);
        assert_eq!(
            spot.w, -160.0,
            "yaw is rounded; pitch and roll never make it in"
        );
    }

    #[test]
    fn output_is_minified_with_the_short_keys() {
        let json = build().json;
        assert!(
            json.starts_with(r#"{"version":1,"areas":[{"id":"HC","label":"Tundra","locId":null}"#)
        );
        assert!(json.contains(
            r#"{"n":"FP_OC_STAND_YARD_1","x":10.0,"y":20.0,"z":30.0,"w":-160.0,"a":"OC"}"#
        ));
        assert!(
            !json.ends_with('\n'),
            "the asset carries no trailing newline"
        );
    }

    #[test]
    fn a_dead_loc_id_is_dropped_to_null() {
        let known: BTreeSet<String> = ["area_oldcamp_notification".to_string()]
            .into_iter()
            .collect();
        let build = build_location_catalog(SOURCE, Some(&known)).expect("the fixture parses");
        assert_eq!(
            build.catalog.area("OC").expect("used").loc_id.as_deref(),
            Some("area_oldcamp_notification")
        );
        assert!(
            !build.report.dead_loc_ids.is_empty(),
            "every other curated id is unknown here"
        );
        assert_eq!(build.report.verified_loc_ids, Some(18));
    }

    #[test]
    fn area_tokens_match_the_python_regex() {
        assert_eq!(area_token("FP_OC_STAND_YARD_1"), Some("OC"));
        assert_eq!(area_token("WP_OTOWN_GATE_01"), Some("OTOWN"));
        assert_eq!(area_token("WP_ExF_BRIDGE"), Some("ExF"));
        assert_eq!(
            area_token("FP_A_X"),
            None,
            "a one-letter token is not an area"
        );
        assert_eq!(
            area_token("FP_ABCDEF_X"),
            None,
            "six letters is over the cap"
        );
        assert_eq!(
            area_token("FP2_OC_X"),
            None,
            "the prefix must be all letters"
        );
        assert_eq!(area_token("_OC_X"), None);
        assert_eq!(
            area_token("FP_OC1_X"),
            None,
            "the token must end at the underscore"
        );
        assert_eq!(area_token("NoUnderscores"), None);
    }

    #[test]
    fn short_layer_takes_the_last_component() {
        assert_eq!(
            short_layer(Some("/Game/Maps/DataLayers/MainMap/Foo.Foo")),
            "Foo"
        );
        assert_eq!(short_layer(Some("None")), "");
        assert_eq!(short_layer(None), "");
    }

    #[test]
    fn rounding_matches_python_round() {
        // Ties go to even, on the exact binary value — the case
        // `(v * 10.0).round() / 10.0` gets wrong.
        assert_eq!(round1(0.25), 0.2);
        assert_eq!(round1(0.35), 0.3);
        assert_eq!(round1(-159.999999), -160.0);
        assert_eq!(round1(2.675), 2.7);
    }

    #[test]
    fn the_bundled_catalog_parses_and_resolves_case_insensitively() {
        let catalog = LocationCatalog::bundled().expect("the bundled asset is valid");
        assert_eq!(catalog.version, SCHEMA_VERSION);
        let spot = catalog
            .resolve("fp_oc_stand_yard_1")
            .expect("a spot every cook has");
        assert_eq!(spot.n, "FP_OC_STAND_YARD_1");
        assert_eq!(catalog.area(&spot.a).expect("labelled").label, "Old Camp");
    }

    #[test]
    fn a_typo_suggests_the_name_it_missed() {
        let catalog = LocationCatalog::bundled().expect("the bundled asset is valid");
        assert!(catalog.resolve("FP_OC_STAND_YARDD_1").is_none());
        assert!(catalog
            .suggest("FP_OC_STAND_YARDD_1", 5)
            .contains(&"FP_OC_STAND_YARD_1"));
    }

    #[test]
    fn list_filters_by_area_and_prefix() {
        let catalog = LocationCatalog::bundled().expect("the bundled asset is valid");
        let filtered = catalog.list(Some("oc"), Some("fp"));
        assert!(!filtered.is_empty());
        assert!(filtered
            .iter()
            .all(|spot| spot.a == "OC" && spot.n.starts_with("FP")));
        assert!(filtered.len() < catalog.list(Some("OC"), None).len());
    }
}
