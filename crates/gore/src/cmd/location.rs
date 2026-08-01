//! `gore location` — offline lookup over the bundled named-location catalog.
//!
//! # Why this exists
//!
//! `TeleportToWaypointAndExchangeDailyRoutineToClass(npcState, routine, FName
//! Waypoint)` and `TeleportToSpot(charState, FName)` both resolve their name
//! through `FInteractionSpotHandle(FName)`, whose invalid branch is empty: an
//! unknown waypoint is a **silent no-op**. No log line, no crash, no character
//! moved — a typo in a mod script is simply swallowed. Checking the name
//! against the 10,075 the cook actually has is the cheapest place to catch it,
//! and it costs no game install: the catalog is compiled into this binary.
//!
//! Comparison is case-insensitive because `FName` comparison in the game is,
//! and the spellings drift — the same waypoint is `WP_ExF_…` in AngelScript and
//! `WP_EXf_…` in a save.

use anyhow::{bail, Result};
use clap::Subcommand;
use gore_catalog::location::LocationCatalog;

/// How many near names a miss suggests.
const SUGGESTIONS: usize = 8;

#[derive(Subcommand)]
pub enum LocationAction {
    /// Look one spot name up: area, coordinates and yaw, or the near names it was not
    Resolve {
        /// Spot name, e.g. FP_OC_STAND_YARD_1 (case-insensitive)
        name: String,
        /// Emit one JSON document instead of the human-readable block
        #[arg(long)]
        json: bool,
    },
    /// List spot names, narrowed by area code and/or name prefix
    List {
        /// Keep only spots in this area code (e.g. OC). See the `areas` table of the catalog
        #[arg(long)]
        area: Option<String>,
        /// Keep only spots whose name starts with this (e.g. FP)
        #[arg(long)]
        prefix: Option<String>,
        /// Max names to print. The result says how many matched when it stops here
        #[arg(long, default_value_t = 200)]
        max: usize,
        /// Emit one JSON document instead of the human-readable list
        #[arg(long)]
        json: bool,
    },
}

pub fn run(action: LocationAction) -> Result<()> {
    let catalog = LocationCatalog::bundled()?;
    match action {
        LocationAction::Resolve { name, json } => resolve(&catalog, &name, json),
        LocationAction::List {
            area,
            prefix,
            max,
            json,
        } => list(&catalog, area.as_deref(), prefix.as_deref(), max, json),
    }
}

fn resolve(catalog: &LocationCatalog, name: &str, json: bool) -> Result<()> {
    let found = catalog.resolve(name);

    if json {
        let document = match found {
            Some(spot) => serde_json::json!({
                "query": name,
                "found": true,
                // The catalog's own spelling, which is what a script must use.
                "name": spot.n,
                "area": spot.a,
                "area_label": catalog.area(&spot.a).map(|area| area.label.clone()),
                "x": spot.x,
                "y": spot.y,
                "z": spot.z,
                "yaw": spot.w,
            }),
            None => serde_json::json!({
                "query": name,
                "found": false,
                "spot_count": catalog.spots.len(),
                "suggestions": catalog.suggest(name, SUGGESTIONS),
            }),
        };
        println!("{}", serde_json::to_string_pretty(&document)?);
    }

    let Some(spot) = found else {
        let near = catalog.suggest(name, SUGGESTIONS);
        let did_you_mean = if near.is_empty() {
            "no name in the catalog is close to it".to_string()
        } else {
            format!("did you mean: {}", near.join(", "))
        };
        // Non-zero exit, because this is the check a build step runs: the game
        // would accept this name and do nothing at all with it.
        bail!(
            "no spot named '{name}' among the {} in the bundled catalog — the game ignores an \
             unknown waypoint silently, so a script using it would simply not teleport. \
             {did_you_mean}",
            catalog.spots.len()
        );
    };

    if !json {
        println!("{}", spot.n);
        match catalog.area(&spot.a) {
            Some(area) => println!("  area  {} — {}", area.id, area.label),
            None => println!("  area  (none — no labelled spot near enough to vote)"),
        }
        println!("  x     {}", spot.x);
        println!("  y     {}", spot.y);
        println!("  z     {}", spot.z);
        println!("  yaw   {}", spot.w);
    }
    Ok(())
}

fn list(
    catalog: &LocationCatalog,
    area: Option<&str>,
    prefix: Option<&str>,
    max: usize,
    json: bool,
) -> Result<()> {
    // An area code that is not in the table matches nothing, which reads
    // exactly like "this area has no spots". Say which it is.
    if let Some(code) = area {
        if catalog.area(code).is_none() {
            bail!(
                "no area '{code}' in the catalog — it has {}",
                catalog
                    .areas
                    .iter()
                    .map(|area| area.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    // Filter first, cap second, so `matched` is the honest count of what the
    // query selected rather than of what happened to fit.
    let matched = catalog.list(area, prefix);
    let listed = &matched[..matched.len().min(max)];
    let truncated = listed.len() < matched.len();

    if json {
        let mut document = serde_json::json!({
            "area": area,
            "prefix": prefix,
            "spot_count": catalog.spots.len(),
            "matched_count": matched.len(),
            "listed_count": listed.len(),
            "truncated": truncated,
            "spots": listed,
        });
        if truncated {
            document["truncation_notice"] =
                serde_json::json!(truncation_notice(matched.len(), listed.len()));
        }
        println!("{}", serde_json::to_string_pretty(&document)?);
        return Ok(());
    }

    println!(
        "{} of {} spots matched{}",
        matched.len(),
        catalog.spots.len(),
        narrowing(area, prefix)
    );
    for spot in listed {
        println!("{:<6} {}", spot.a, spot.n);
    }
    if truncated {
        // The same marker the MCP server appends to a clipped result, so one
        // learned habit covers both.
        println!(
            "… [truncated: {}]",
            truncation_notice(matched.len(), listed.len())
        );
    }
    Ok(())
}

fn narrowing(area: Option<&str>, prefix: Option<&str>) -> String {
    match (area, prefix) {
        (None, None) => String::new(),
        (Some(area), None) => format!(" (area {area})"),
        (None, Some(prefix)) => format!(" (prefix {prefix})"),
        (Some(area), Some(prefix)) => format!(" (area {area}, prefix {prefix})"),
    }
}

/// One sentence answering "how much am I not seeing" and "what do I type
/// instead". A listing that stopped silently would let a caller read the first
/// `max` names as the whole area and conclude a spot does not exist — which is
/// the very mistake this command exists to prevent.
fn truncation_notice(matched: usize, listed: usize) -> String {
    format!(
        "{matched} spots matched and only the first {listed} are shown. Narrow with --area / \
         --prefix, or raise --max"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_known_name_resolves_and_a_typo_does_not() {
        let catalog = LocationCatalog::bundled().unwrap();
        assert!(resolve(&catalog, "FP_OC_STAND_YARD_1", false).is_ok());
        assert!(resolve(&catalog, "fp_oc_stand_yard_1", false).is_ok());

        let error = resolve(&catalog, "FP_OC_STAND_YARDD_1", false)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("FP_OC_STAND_YARD_1"),
            "the miss must suggest the hit: {error}"
        );
    }

    #[test]
    fn an_unknown_area_is_named_rather_than_answered_with_nothing() {
        let catalog = LocationCatalog::bundled().unwrap();
        let error = list(&catalog, Some("ZZ"), None, 10, false)
            .unwrap_err()
            .to_string();
        assert!(error.contains("no area 'ZZ'"), "{error}");
        assert!(list(&catalog, Some("oc"), Some("FP"), 10, false).is_ok());
    }
}
