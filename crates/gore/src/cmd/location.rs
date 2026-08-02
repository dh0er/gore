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
use std::collections::BTreeSet;

/// How many near names a miss suggests.
const SUGGESTIONS: usize = 8;

/// How many existing prefixes a listing that matched nothing offers back.
const PREFIX_HINTS: usize = 12;

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
    // A prefix nobody has is the one answer a bare empty list cannot be read
    // correctly: it looks exactly like an area that happens to be empty.
    let no_match = match (matched.is_empty(), prefix) {
        (true, Some(prefix)) => no_match_notice(catalog, area, prefix),
        _ => None,
    };

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
        if let Some(notice) = &no_match {
            document["no_match_notice"] = serde_json::json!(notice);
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
    if let Some(notice) = &no_match {
        println!("{notice}");
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

/// One sentence answering "then what does exist" for a `--prefix` nothing
/// starts with — the courtesy [`resolve`] already extends to a name nothing
/// matches. Without it a zero-match listing is indistinguishable from an empty
/// area, and the only way forward is another guess.
///
/// `None` when the catalog holds nothing at all in scope, which has no useful
/// prefix to offer.
fn no_match_notice(catalog: &LocationCatalog, area: Option<&str>, prefix: &str) -> Option<String> {
    let (stem, mut ranked, count) = existing_prefixes(catalog, area, prefix)?;
    let hidden = ranked.len().saturating_sub(PREFIX_HINTS);
    ranked.truncate(PREFIX_HINTS);
    let more = if hidden > 0 {
        format!(", … (+{hidden} more)")
    } else {
        String::new()
    };
    let shown = ranked
        .iter()
        .map(|(name, spots)| format!("{name} ({spots})"))
        .collect::<Vec<_>>()
        .join(", ");
    let scope = match area {
        Some(area) => format!(" in area {area}"),
        None => String::new(),
    };
    Some(if stem.is_empty() {
        format!(
            "no name starts with {prefix}{scope}. Every name there starts with one of these — \
             pass one to --prefix: {shown}{more}"
        )
    } else {
        format!(
            "no name starts with {prefix}{scope}. {count} do start with {stem} — pass one of \
             these to --prefix: {shown}{more}"
        )
    })
}

/// The longest stem of `prefix` that something in scope does start with, its
/// distinct one-segment continuations with a spot count each, and how many
/// spots the stem covers in total.
///
/// Continuations are taken from the catalog's own spelling, not from the query,
/// so what comes back can be pasted into `--prefix` as-is. They are ranked by
/// how much they cover: an area's names are one or two big families plus a long
/// tail of singletons, and alphabetical order would spend the whole hint list on
/// the tail.
fn existing_prefixes(
    catalog: &LocationCatalog,
    area: Option<&str>,
    prefix: &str,
) -> Option<(String, Vec<(String, usize)>, usize)> {
    for stem in prefix_backoffs(prefix) {
        let scope = catalog.list(area, Some(stem));
        if scope.is_empty() {
            continue;
        }
        let distinct: BTreeSet<String> = scope
            .iter()
            .filter_map(|spot| continuation(&spot.n, stem.len()))
            .collect();
        // Counted by the same rule `list` filters by, not by which group a name
        // was binned into: `FP_OC_PLAYDARTS` is its own leaf AND the head of
        // `FP_OC_PLAYDARTS_1`, so a bin count would promise one and deliver two.
        let mut ranked: Vec<(String, usize)> = distinct
            .into_iter()
            .map(|next| {
                let hits = scope
                    .iter()
                    .filter(|spot| starts_with_ignoring_case(&spot.n, &next))
                    .count();
                (next, hits)
            })
            .collect();
        ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        return Some((stem.to_string(), ranked, scope.len()));
    }
    None
}

/// `FP_OC_START` → `FP_OC_`, `FP_`, `""`: the query with one more trailing
/// `_`-separated segment dropped each time, keeping the separator so every stem
/// is still a prefix a caller could have typed. The empty stem is last and
/// always matches, so the walk ends at "here is what this area has".
fn prefix_backoffs(prefix: &str) -> Vec<&str> {
    let mut stems = Vec::new();
    let mut end = prefix.len();
    while end > 0 {
        // Drop a separator the query itself ends with first, so `FP_OC_` backs
        // off to `FP_` rather than searching the same empty set twice.
        let head = &prefix[..end];
        let head = head.strip_suffix('_').unwrap_or(head);
        let Some(cut) = head.rfind('_') else {
            break;
        };
        end = cut + 1;
        stems.push(&prefix[..end]);
    }
    stems.push("");
    stems
}

/// The prefix test [`LocationCatalog::list`] filters by, so a count taken here
/// is the number of names that listing the same prefix will print.
fn starts_with_ignoring_case(name: &str, prefix: &str) -> bool {
    name.as_bytes()
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix.as_bytes()))
}

/// `name` cut to one segment past `stem_len`, with its trailing `_` kept:
/// `FP_OC_ARENA_1` past `FP_` is `FP_OC_`. The result is a prefix of `name`, so
/// listing it again cannot come back empty.
fn continuation(name: &str, stem_len: usize) -> Option<String> {
    let head = name.get(..stem_len)?;
    let rest = name.get(stem_len..)?;
    Some(match rest.find('_') {
        Some(cut) => format!("{head}{}_", &rest[..cut]),
        None => format!("{head}{rest}"),
    })
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
    fn a_prefix_nothing_starts_with_is_answered_with_the_prefixes_that_do_exist() {
        // Three guesses in one session — `--area OC --prefix FP_CAMP_ENTRANCE`, then
        // `--prefix FP_OC_START` — each answered with an empty array out of 10,075 names, which
        // reads exactly like an empty area. `resolve` already tells a miss what is near it; a
        // listing that says nothing at all is an invitation to guess again.
        let catalog = LocationCatalog::bundled().unwrap();

        let deep = no_match_notice(&catalog, Some("OC"), "FP_OC_START")
            .expect("the catalog has names under FP_OC_");
        assert!(
            deep.contains("FP_OC_") && deep.contains("--prefix"),
            "a near miss must hand back the stem that does exist: {deep}"
        );

        // A prefix with no stem in common with anything falls all the way back to "here is what
        // this area has", rather than to nothing.
        let wild =
            no_match_notice(&catalog, Some("OC"), "CAMP_ENTRANCE").expect("area OC is not empty");
        assert!(
            wild.contains("--prefix: FP_ ("),
            "the area's biggest family must be offered first, ahead of its singletons: {wild}"
        );

        // Every offered prefix is one that lists something — the point of taking them from the
        // catalog's own spelling rather than from the query.
        for (prefix, count) in existing_prefixes(&catalog, Some("OC"), "FP_OC_START").unwrap().1 {
            assert_eq!(
                catalog.list(Some("OC"), Some(&prefix)).len(),
                count,
                "offered prefix {prefix} does not list what it claims"
            );
        }

        // And a miss is still an answer, not a failure: `list` is a filter, and an empty result
        // is a legitimate one.
        assert!(list(&catalog, Some("OC"), Some("FP_OC_START"), 10, false).is_ok());
        assert!(list(&catalog, Some("OC"), Some("FP_OC_START"), 10, true).is_ok());
    }

    #[test]
    fn a_backoff_drops_one_segment_at_a_time_and_ends_at_everything() {
        assert_eq!(prefix_backoffs("FP_OC_START"), ["FP_OC_", "FP_", ""]);
        // A prefix that already ends in a separator backs off past it rather than to itself,
        // which would search the same empty set twice.
        assert_eq!(prefix_backoffs("FP_OC_"), ["FP_", ""]);
        assert_eq!(prefix_backoffs("FP"), [""]);
        assert_eq!(prefix_backoffs(""), [""]);
        // A multi-byte tail must not be sliced through: this is a value the user typed.
        assert_eq!(prefix_backoffs("FP_ü"), ["FP_", ""]);
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
