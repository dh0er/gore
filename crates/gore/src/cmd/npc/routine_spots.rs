//! Routine locations, with interaction compatibility read from the installed cook.
//!
//! A location name proves that navigation has a target; it does not prove that a
//! chair, bed, guard action, or alchemy station can be used there. Keep these two
//! kinds of evidence separate, and never infer an interaction from a spot name.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use gore_catalog::location::{LocationCatalog, SpotEntry};
use serde::{Deserialize, Serialize};

use super::routine_plan::Activity;

const SOURCE_RELATIVE_PATH: &str = "G1R/Script/Map/MainMap/InteractionSpots.json";
const BUNDLED_EVIDENCE: &str = "bundled location catalog";
const HUMAN_REQUIREMENT: &str = "/Script/G1R.CharacterSpeciesRequirements(AllowedSpecies=(GameplayTags=((TagName=\"Species.Human\"))))";
const SUGGESTIONS: usize = 8;

/// Direct stand/read/drink states navigate without requesting a spot action.
pub fn needs_interaction_evidence(activity: Activity) -> bool {
    interaction_tag(activity).is_some()
}

/// A canonical location and the evidence used to accept this activity there.
#[derive(Debug, Clone, Serialize)]
pub struct ValidatedSpot {
    pub name: String,
    pub area: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f64,
    pub activity: String,
    pub actions: Vec<String>,
    pub evidence: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SpotList {
    pub activity: String,
    pub area: Option<String>,
    pub prefix: Option<String>,
    pub matched_count: usize,
    pub listed_count: usize,
    pub truncated: bool,
    pub evidence: String,
    pub notes: Vec<String>,
    pub spots: Vec<ValidatedSpot>,
}

pub struct SpotCatalog {
    locations: LocationCatalog,
    interactions: Option<InteractionCatalog>,
    source: Option<PathBuf>,
}

impl SpotCatalog {
    /// `None` is sufficient for direct activities. An explicit installation
    /// must provide readable action metadata; a missing file is never treated
    /// as an empty set of restrictions.
    pub fn load(game: Option<&Path>) -> Result<Self> {
        let locations = LocationCatalog::bundled().context("read bundled location catalog")?;
        let source = game.map(|game| game.join(SOURCE_RELATIVE_PATH));
        let interactions = source
            .as_ref()
            .map(|path| {
                let text = fs::read_to_string(path).with_context(|| {
                    format!(
                        "interaction evidence unavailable: cannot read {}",
                        path.display()
                    )
                })?;
                parse_interaction_source(&text).with_context(|| {
                    format!(
                        "interaction evidence unavailable: cannot parse {}",
                        path.display()
                    )
                })
            })
            .transpose()?;
        Ok(Self {
            locations,
            interactions,
            source,
        })
    }

    /// Fail before a routine is written if its location or action is unproven.
    pub fn validate(&self, activity: Activity, name: &str) -> Result<ValidatedSpot> {
        let exact = self.locations.spots.iter().find(|spot| spot.n == name);
        let location = if let Some(exact) = exact {
            Some(exact)
        } else {
            let folded: Vec<_> = self
                .locations
                .spots
                .iter()
                .filter(|spot| spot.n.eq_ignore_ascii_case(name))
                .collect();
            if folded.len() > 1 {
                bail!(
                    "ambiguous routine spot '{name}': use an exact spelling, such as {}",
                    folded
                        .iter()
                        .map(|spot| spot.n.as_str())
                        .collect::<Vec<_>>()
                        .join(" or ")
                );
            }
            folded.first().copied()
        };
        let Some(location) = location else {
            let near = self.locations.suggest(name, SUGGESTIONS);
            let suggestion = if near.is_empty() {
                format!(
                    "use `gore npc routine spots --activity {}` to choose a known spot",
                    activity.as_str()
                )
            } else {
                format!(
                    "nearby names: {} (check their activity compatibility)",
                    near.join(", ")
                )
            };
            bail!("unknown routine spot '{name}'; the game can silently ignore unknown names; {suggestion}");
        };
        let interaction = if let Some(tag) = interaction_tag(activity) {
            let catalog = self.require_interactions(activity)?;
            let rows = catalog.resolve(name).with_context(|| {
                format!(
                    "interaction evidence unavailable for known spot '{}': no entry in {}",
                    location.n,
                    self.evidence(activity)
                )
            })?;
            let row = unique_row(&rows).with_context(|| {
                format!("interaction evidence unavailable for spot '{}'", location.n)
            })?;
            if let Some(problem) = incompatibility(row, tag) {
                bail!(
                    "spot '{}' cannot be validated for {}: {problem}; use `gore npc routine spots --activity {}` for compatible locations",
                    location.n,
                    activity.as_str(),
                    activity.as_str()
                );
            }
            Some(row)
        } else {
            None
        };
        Ok(self.validated(location, activity, interaction))
    }

    /// Filter before truncating, so counts describe all compatible results.
    /// Object activities omit restricted, ambiguous, or unproven entries.
    pub fn list(
        &self,
        activity: Activity,
        area: Option<&str>,
        prefix: Option<&str>,
        max: usize,
    ) -> Result<SpotList> {
        if let Some(area) = area {
            if self.locations.area(area).is_none() {
                bail!(
                    "unknown area '{area}'; known areas: {}",
                    self.locations
                        .areas
                        .iter()
                        .map(|area| area.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }
        let tag = interaction_tag(activity);
        let interactions = if tag.is_some() {
            Some(self.require_interactions(activity)?)
        } else {
            None
        };
        let mut spots = Vec::new();
        let mut matched_count = 0;
        for location in self.locations.list(area, prefix) {
            let interaction = if let (Some(catalog), Some(tag)) = (interactions, tag) {
                let Some(rows) = catalog.resolve(&location.n) else {
                    continue;
                };
                let Ok(row) = unique_row(&rows) else {
                    continue;
                };
                if incompatibility(row, tag).is_some() {
                    continue;
                }
                Some(row)
            } else {
                None
            };
            matched_count += 1;
            if spots.len() < max {
                spots.push(self.validated(location, activity, interaction));
            }
        }
        let listed_count = spots.len();
        Ok(SpotList {
            activity: activity.as_str().to_owned(),
            area: area.map(str::to_owned),
            prefix: prefix.map(str::to_owned),
            matched_count,
            listed_count,
            truncated: listed_count < matched_count,
            evidence: self.evidence(activity),
            notes: evidence_notes(activity),
            spots,
        })
    }

    fn require_interactions(&self, activity: Activity) -> Result<&InteractionCatalog> {
        self.interactions.as_ref().with_context(|| {
            format!(
                "interaction evidence unavailable for {}: select an installed game containing {SOURCE_RELATIVE_PATH}; the bundled location catalog has no action or restriction metadata",
                activity.as_str()
            )
        })
    }

    fn evidence(&self, activity: Activity) -> String {
        if needs_interaction_evidence(activity) {
            self.source
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "interaction evidence unavailable".to_owned())
        } else {
            BUNDLED_EVIDENCE.to_owned()
        }
    }

    fn validated(
        &self,
        location: &SpotEntry,
        activity: Activity,
        interaction: Option<&InteractionSpot>,
    ) -> ValidatedSpot {
        ValidatedSpot {
            name: location.n.clone(),
            area: location.a.clone(),
            x: interaction.map_or(location.x, |row| row.location.x),
            y: interaction.map_or(location.y, |row| row.location.y),
            z: interaction.map_or(location.z, |row| row.location.z),
            yaw: interaction.map_or(location.w, |row| row.rotation.yaw),
            activity: activity.as_str().to_owned(),
            actions: interaction
                .map(|row| {
                    row.possible_actions
                        .gameplay_tags
                        .iter()
                        .map(|tag| tag.tag_name.clone())
                        .collect()
                })
                .unwrap_or_default(),
            evidence: self.evidence(activity),
            notes: evidence_notes(activity),
        }
    }
}

fn evidence_notes(activity: Activity) -> Vec<String> {
    let mut notes = vec![
        "Area labels come from the bundled location catalog and may be inferred from nearby spots."
            .to_owned(),
        "Catalog compatibility does not prove runtime reachability, availability, or occupancy."
            .to_owned(),
    ];
    if needs_interaction_evidence(activity) {
        notes.push("Actions, restrictions, and coordinates come from the installed interaction source; only empty or exact human-only requirements are accepted. Other conditions and duplicate names are excluded.".to_owned());
    } else {
        notes.push("This activity uses the spot only as a navigation destination; interaction actions and their restrictions are not requested. Coordinates come from the bundled location catalog.".to_owned());
    }
    notes
}

/// These are the tags requested by the shipped Sit, Sleep, GuardWatch and
/// PotionAlchemy AI states, not categories guessed from object names.
fn interaction_tag(activity: Activity) -> Option<&'static str> {
    match activity {
        Activity::Stand | Activity::Read | Activity::Drink => None,
        Activity::Sit => Some("Action.Interact.Sit"),
        Activity::Sleep => Some("Action.Ambient.Sleep"),
        Activity::Guard => Some("Action.Ambient.GuardWatch"),
        Activity::Alchemy => Some("Action.Ambient.PotionAlchemy"),
    }
}

fn matches_tag(actual: &str, requested: &str) -> bool {
    actual == requested
        || actual
            .strip_prefix(requested)
            .is_some_and(|suffix| suffix.starts_with('.'))
}

fn incompatibility(row: &InteractionSpot, requested: &str) -> Option<String> {
    if !row
        .possible_actions
        .gameplay_tags
        .iter()
        .any(|tag| matches_tag(&tag.tag_name, requested))
    {
        return Some(format!(
            "unsupported action: source does not advertise {requested}"
        ));
    }
    if let Some(requirement) = row
        .custom_requirements
        .iter()
        .find(|requirement| requirement.as_str() != HUMAN_REQUIREMENT)
    {
        return Some(format!(
            "restricted or conditional spot: cannot establish that this NPC satisfies {requirement}"
        ));
    }
    None
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InteractionSource {
    interaction_spots: Vec<InteractionSpot>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InteractionSpot {
    name: String,
    location: Position,
    rotation: Rotation,
    possible_actions: Actions,
    // Deliberately required: missing metadata must not mean unrestricted.
    custom_requirements: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Position {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Debug, Deserialize)]
struct Rotation {
    yaw: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Actions {
    gameplay_tags: Vec<ActionTag>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActionTag {
    tag_name: String,
}

struct InteractionCatalog {
    spots: BTreeMap<String, Vec<InteractionSpot>>,
}

impl InteractionCatalog {
    fn resolve(&self, name: &str) -> Option<Vec<&InteractionSpot>> {
        // The shipped location catalog contains distinct Box_Top and Box_top spots.
        // Preserve an exact match; only a fallback spelling must consider every folded match.
        if let Some(rows) = self.spots.get(name) {
            return Some(rows.iter().collect());
        }
        let rows: Vec<_> = self
            .spots
            .iter()
            .filter(|(key, _)| key.eq_ignore_ascii_case(name))
            .flat_map(|(_, rows)| rows.iter())
            .collect();
        (!rows.is_empty()).then_some(rows)
    }
}

fn parse_interaction_source(text: &str) -> Result<InteractionCatalog> {
    let source: InteractionSource = serde_json::from_str(text)?;
    if source.interaction_spots.is_empty() {
        bail!("interactionSpots contains no metadata");
    }
    let mut spots: BTreeMap<String, Vec<InteractionSpot>> = BTreeMap::new();
    for row in source.interaction_spots {
        if row.name.trim().is_empty() {
            bail!("interaction source contains an unnamed spot");
        }
        spots.entry(row.name.clone()).or_default().push(row);
    }
    Ok(InteractionCatalog { spots })
}

fn unique_row<'a>(rows: &[&'a InteractionSpot]) -> Result<&'a InteractionSpot> {
    if rows.len() != 1 {
        bail!(
            "{} source entries share this name; compatibility is ambiguous",
            rows.len()
        );
    }
    Ok(rows[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(name: &str, action: &str, requirements: &[&str]) -> serde_json::Value {
        json!({
            "name": name,
            "location": {"x": 1.0, "y": 2.0, "z": 3.0},
            "rotation": {"yaw": 4.0},
            "possibleActions": {"gameplayTags": [{"tagName": action}]},
            "customRequirements": requirements,
        })
    }

    fn catalog(rows: Vec<serde_json::Value>) -> SpotCatalog {
        SpotCatalog {
            locations: LocationCatalog::bundled().unwrap(),
            interactions: Some(
                parse_interaction_source(&json!({"interactionSpots": rows}).to_string()).unwrap(),
            ),
            source: Some(PathBuf::from("fixture/InteractionSpots.json")),
        }
    }

    #[test]
    fn proven_object_pairings_match_the_requested_tags() {
        let cases = [
            (Activity::Sit, "IO_OC_CHAIR_81", "Action.Interact.Sit.Chair"),
            (
                Activity::Sleep,
                "Interactive_Bed_Xardas472597",
                "Action.Ambient.Sleep.High.Right",
            ),
            (
                Activity::Guard,
                "BreadcrumbActor_OC_NIGHTWATCH_GUARD10_5",
                "Action.Ambient.GuardWatch",
            ),
            (
                Activity::Alchemy,
                "IO_XT_POTION_ALCHEMY_01",
                "Action.Ambient.PotionAlchemy",
            ),
        ];
        let catalog = catalog(
            cases
                .iter()
                .map(|(_, name, action)| row(name, action, &[]))
                .collect(),
        );
        for (activity, name, _) in cases {
            let validated = catalog
                .validate(activity, &name.to_ascii_lowercase())
                .unwrap();
            assert_eq!(validated.name, name);
            assert_eq!(validated.x, 1.0);
        }
        assert!(catalog
            .validate(Activity::Sleep, "IO_OC_CHAIR_81")
            .unwrap_err()
            .to_string()
            .contains("unsupported action"));
        assert!(!matches_tag(
            "Action.Ambient.PotionAlchemyFemale",
            "Action.Ambient.PotionAlchemy"
        ));
    }

    #[test]
    fn restrictions_are_not_inferred_from_names_or_partial_matches() {
        let restricted = catalog(vec![row(
            "IO_OC_CHAIR_81",
            "Action.Interact.Sit.Chair",
            &["/Script/G1R.UniqueNameRequirements(MustHaveAnyOfTheseUniqueNames=(\"XT_DMB_Xardas_404\"))"],
        )]);
        assert!(restricted
            .validate(Activity::Sit, "IO_OC_CHAIR_81")
            .unwrap_err()
            .to_string()
            .contains("restricted or conditional"));
        assert_eq!(
            restricted
                .list(Activity::Sit, None, None, 10)
                .unwrap()
                .matched_count,
            0
        );
        // The same restriction does not govern navigation-only states.
        assert!(restricted
            .validate(Activity::Stand, "IO_OC_CHAIR_81")
            .is_ok());

        let human = catalog(vec![row(
            "IO_OC_CHAIR_81",
            "Action.Interact.Sit.Chair",
            &[HUMAN_REQUIREMENT],
        )]);
        assert!(human.validate(Activity::Sit, "IO_OC_CHAIR_81").is_ok());
        let creature = catalog(vec![row("IO_OC_CHAIR_81", "Action.Interact.Sit.Chair", &["/Script/G1R.CharacterSpeciesRequirements(AllowedSpecies=(GameplayTags=((TagName=\"Species.Creature.Wolf\"))))"])]);
        assert!(creature.validate(Activity::Sit, "IO_OC_CHAIR_81").is_err());
    }

    #[test]
    fn missing_metadata_is_unavailable_not_an_unrestricted_spot() {
        let bundled = SpotCatalog::load(None).unwrap();
        assert!(bundled.validate(Activity::Read, "IO_OC_CHAIR_81").is_ok());
        assert!(bundled
            .validate(Activity::Sit, "IO_OC_CHAIR_81")
            .unwrap_err()
            .to_string()
            .contains("evidence unavailable"));
        assert!(bundled
            .validate(Activity::Stand, "IO_OC_CHAIR_8x")
            .unwrap_err()
            .to_string()
            .contains("nearby names"));

        let mut incomplete = row("IO_OC_CHAIR_81", "Action.Interact.Sit.Chair", &[]);
        incomplete
            .as_object_mut()
            .unwrap()
            .remove("customRequirements");
        assert!(
            parse_interaction_source(&json!({"interactionSpots": [incomplete]}).to_string())
                .is_err()
        );
    }

    #[test]
    fn duplicate_entries_are_ambiguous_and_counts_precede_truncation() {
        let chair = row("IO_OC_CHAIR_81", "Action.Interact.Sit.Chair", &[]);
        let duplicate = catalog(vec![chair.clone(), chair.clone()]);
        assert!(duplicate
            .validate(Activity::Sit, "IO_OC_CHAIR_81")
            .unwrap_err()
            .to_string()
            .contains("evidence unavailable"));
        assert_eq!(
            duplicate
                .list(Activity::Sit, None, None, 10)
                .unwrap()
                .matched_count,
            0
        );

        // The cook has two distinct locations with these case-only names. Exact
        // lookup must preserve both; a spelling matching neither exactly is ambiguous.
        let mut lower = row("Box_top", "Action.Interact.Sit.Chair", &[]);
        lower["location"]["x"] = json!(2.0);
        let differently_cased = catalog(vec![
            row("Box_Top", "Action.Interact.Sit.Chair", &[]),
            lower,
        ]);
        assert_eq!(
            differently_cased
                .validate(Activity::Sit, "Box_Top")
                .unwrap()
                .x,
            1.0
        );
        assert_eq!(
            differently_cased
                .validate(Activity::Sit, "Box_top")
                .unwrap()
                .x,
            2.0
        );
        for activity in [Activity::Sit, Activity::Stand] {
            assert!(differently_cased
                .validate(activity, "BOX_TOP")
                .unwrap_err()
                .to_string()
                .contains("ambiguous routine spot"));
        }
        let folded = differently_cased
            .interactions
            .as_ref()
            .unwrap()
            .resolve("BOX_TOP")
            .unwrap();
        assert!(unique_row(&folded).is_err());

        let single = catalog(vec![chair]);
        let listed = single
            .list(Activity::Sit, Some("oc"), Some("io_oc_chair"), 0)
            .unwrap();
        assert_eq!(listed.matched_count, 1);
        assert_eq!(listed.listed_count, 0);
        assert!(listed.truncated);
        assert!(single.list(Activity::Sit, Some("typo"), None, 10).is_err());
    }
}
