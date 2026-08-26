//! Hero skill editing.
//!
//! Gothic 1 Remake stores learned hero skills as `GameplayEffectSpec` elements
//! in the hero's `ActiveEffects` array, at typed path
//! `ActiveEffectsByGlobalId/{Hero}/ActiveEffects/[i]/EffectSpec/Def` — an
//! ObjectProperty whose value is a GE class like
//! `/Script/Angelscript.Default__GE_Skill_Melee_OneHanded_Master`. The tier is
//! encoded in the class name; the game re-derives the effect from that class on
//! load.
//!
//! This module exposes:
//! - [`list_skills`] — read the hero's learned skills plus the full learnable
//!   roster (so a fresh hero can be offered every skill), with per-skill tier
//!   options for the UI.
//! - [`apply_skill_set`] — apply one declarative `{base, tier}` intent. Because
//!   it re-resolves its target by skill base name (never a stale index) and
//!   re-parses the payload, multiple skill edits batch safely in one write even
//!   when some are structural (learn/unlearn) — unlike the generic
//!   index-addressed array ops.
//!
//! The catalog and mechanism are ported from the reference editor
//! (github.com/Xetoxyc/gothic-remake-savegame-editor).

pub mod catalog;
mod donor;

use crate::properties::{self, ContainerEdit, PropertyValue};
use crate::{CoreError, map_key_string, struct_member};
use catalog::{Kind, SkillDef};
use serde_json::{Value, json};

/// The map key of the controlled protagonist's character-state entry.
pub const HERO: &str = "Hero";

/// The GE class string of a hero skill element, if it is a `GE_Skill_*` ref.
fn element_class(element: &PropertyValue) -> Option<&str> {
    match struct_member(element, "EffectSpec").and_then(|s| struct_member(s, "Def")) {
        Some(PropertyValue::Object(class)) => Some(class.as_str()),
        _ => None,
    }
}

/// Locate the actor's `ActiveEffects` array: returns the base path to
/// `ActiveEffectsByGlobalId` and the array's elements. `None` when the actor has
/// no entry (or no ActiveEffects array).
fn locate_active_effects<'a>(
    root: &'a properties::RootObject,
    actor: &str,
) -> Option<(Vec<String>, &'a [PropertyValue])> {
    let (base_path, map_prop) = properties::find_property_by_name(root, "ActiveEffectsByGlobalId")?;
    let PropertyValue::Map { entries, .. } = &map_prop.value else {
        return None;
    };
    let value = entries
        .iter()
        .find(|(key, _)| map_key_string(key) == Some(actor))
        .map(|(_, value)| value)?;
    match struct_member(value, "ActiveEffects") {
        Some(PropertyValue::Array { elements }) => Some((base_path, elements.as_slice())),
        _ => None,
    }
}

/// Whether `actor` has an `ActiveEffects` array to edit into. `private.skills.set`
/// (learn/unlearn/retier) requires this target; callers gate the advertised
/// capability on it so a write that would fail with `UnsupportedEdit` is never
/// offered. `actor` is normally [`HERO`].
pub fn actor_has_active_effects(root: &properties::RootObject, actor: &str) -> bool {
    locate_active_effects(root, actor).is_some()
}

/// The current tier value for a learned skill. A suffix-less class (binary
/// skill) has no tier, so its "learned" value is the sentinel `Learned`.
fn current_value(tier: Option<&str>) -> String {
    tier.map(str::to_string)
        .unwrap_or_else(|| "Learned".to_string())
}

/// The ordered tier option VALUES a skill's dropdown offers, each as
/// `{ "value": <tier> }`. Only the value and order matter: the UI composes the
/// visible label from the value using the game's own localized tier vocabulary
/// (`skillmastery_*`, `skill_crafting_blacksmith_*`, `skill_mage_circle_*`), so
/// no label/roman/suffix metadata is emitted here. The op the value maps to
/// (retarget vs unlearn) is decided by [`build_skill_ops`] from `has_untrained`,
/// not per-option metadata.
///
/// Ladder and circle skills always offer the full ladder (`Untrained` +
/// `def.ladder`); on/off skills (hunting/binary/language) offer their learned
/// class + Untrained when learned, or Untrained + the learn value in the roster.
fn tier_options(def: &SkillDef, learned: bool, current: &str) -> Vec<Value> {
    let opt = |value: &str| json!({ "value": value });
    match def.kind {
        // Ladder and circle always offer the full ladder: Untrained + tiers.
        Kind::Ladder | Kind::Circle => std::iter::once("Untrained")
            .chain(def.ladder.iter().copied())
            .map(opt)
            .collect(),
        // hunting / binary / language: on/off (single learned state).
        _ => {
            if learned {
                let mut opts = vec![opt(current)];
                if current != "Untrained" {
                    opts.push(opt("Untrained"));
                }
                opts
            } else {
                vec![opt("Untrained"), opt(catalog::learn_value(def.kind))]
            }
        }
    }
}

/// What an uncatalogued learned class says about itself: its current value and
/// the options its row offers. Removal is the only thing this editor can
/// honestly do with a class it knows nothing else about, so `Untrained` always
/// has to be reachable — and reachable means DIFFERENT from the current value,
/// since the UI drops an edit that re-states what is already selected.
///
/// A class whose own suffix reads as `Untrained` would otherwise collapse both
/// into one option and strand the element for good. Such a row says `Learned`
/// instead: for a class with no catalogued ladder there is no rank to report,
/// and "the effect is present" is the honest statement.
fn uncatalogued_state(tier: Option<&str>) -> (String, Vec<Value>) {
    let current = match current_value(tier) {
        untrained if untrained == "Untrained" => "Learned".to_string(),
        other => other,
    };
    let options = vec![
        json!({ "value": current.clone() }),
        json!({ "value": "Untrained" }),
    ];
    (current, options)
}

/// The highest rung `base` appears at among the actor's learned classes. A save
/// normally holds one element per skill, but not always — see the scutes ladder
/// in [`catalog`] — and the higher class implies the lower.
fn best_tier(
    learned: &[(String, Option<String>)],
    base: &str,
    def: Option<&SkillDef>,
) -> Option<String> {
    let mut best: Option<&Option<String>> = None;
    for (candidate, tier) in learned {
        if candidate != base {
            continue;
        }
        let better = match (best, def) {
            (None, _) => true,
            (Some(current), Some(def)) => {
                catalog::tier_rank(def, tier.as_deref())
                    > catalog::tier_rank(def, current.as_deref())
            }
            // Uncatalogued class: no ladder to rank it by, so keep the first.
            (Some(_), None) => false,
        };
        if better {
            best = Some(tier);
        }
    }
    best.cloned().flatten()
}

/// List the actor's skills: every learned skill (from its ActiveEffects array)
/// plus every catalogued skill it has not learned (the learnable roster). A
/// learned class the catalog does not know is listed too, under its raw name and
/// with an Untrained option, so it can be dropped again. Each entry carries the
/// tier options the UI renders. `actor` is normally [`HERO`].
pub fn list_skills(root: &properties::RootObject, actor: &str) -> Value {
    let located = locate_active_effects(root, actor);
    let found = located.is_some();

    // Learned skills, keyed by base -> current tier value.
    let mut learned: Vec<(String, Option<String>)> = Vec::new();
    if let Some((_, elements)) = &located {
        for element in *elements {
            let Some(class) = element_class(element) else {
                continue;
            };
            let Some((base, tier)) = catalog::split_class(class) else {
                continue;
            };
            learned.push((base, tier));
        }
    }

    let mut skills: Vec<Value> = Vec::new();

    // Emit learned skills first (a save may hold a class not yet catalogued;
    // still surface it as a raw entry so nothing is silently hidden). One row per
    // base, carrying the HIGHEST rung the save holds for it: the game leaves
    // Cavalorn's first scutes lesson in place when it grants the second, and the
    // higher class implies the lower, so the higher one is what the hero can
    // actually do. Showing the first element instead would report a master
    // hunter as merely trained — and hide the rung an edit then silently drops.
    let mut emitted_bases: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for (base, _) in &learned {
        if !emitted_bases.insert(base.as_str()) {
            continue;
        }
        let def = catalog::find(base);
        let tier = best_tier(&learned, base, def);
        let tier = &tier;
        let kind = def.map(|d| d.kind).unwrap_or(Kind::Ladder);
        let mut current = current_value(tier.as_deref());
        let (label, category, has_untrained, mut options) = match def {
            Some(d) => (
                d.label.to_string(),
                d.category.to_string(),
                d.has_untrained,
                tier_options(d, true, &current),
            ),
            // An uncatalogued class: a skill the game defines but nothing ever
            // grants or reads, one a console `addskill` put there, or one a
            // newer game version added. Surface it under its raw name and offer
            // Untrained, so whatever the save carries can always be dropped
            // again — that removal is the only thing this editor can honestly
            // do with a class it knows nothing else about.
            None => {
                let (raw_current, options) = uncatalogued_state(tier.as_deref());
                current = raw_current;
                (base.replace('_', " "), "Other".to_string(), false, options)
            }
        };
        // The dropdown requires the current value to be a selectable option. A
        // ladder/circle class stored without its tier suffix maps to a `current`
        // ("Learned") that the tier list does not contain; keep the row valid by
        // surfacing that raw value as its own option rather than crashing the UI.
        if !options.iter().any(|o| o["value"] == json!(current)) {
            options.insert(0, json!({ "value": current }));
        }
        skills.push(json!({
            "base": base,
            "label": label,
            "category": category,
            "kind": kind_str(kind),
            "learned": true,
            "current": current,
            "hasUntrained": has_untrained,
            "options": options,
        }));
    }

    // Roster: every catalogued skill the actor has not learned. The catalog only
    // holds skills the game actually teaches or checks, so nothing offered here
    // is dead weight.
    let learned_bases: std::collections::HashSet<&str> =
        learned.iter().map(|(b, _)| b.as_str()).collect();
    for def in catalog::SKILLS {
        if learned_bases.contains(def.base) {
            continue;
        }
        let row = json!({
            "base": def.base,
            "label": def.label,
            "category": def.category,
            "kind": kind_str(def.kind),
            "learned": false,
            "current": "Untrained",
            "hasUntrained": def.has_untrained,
            "options": tier_options(def, false, "Untrained"),
        });
        skills.push(row);
    }

    // Stable UI order: category (catalog order), then learned-before-roster,
    // then label.
    skills.sort_by(|a, b| {
        let cat = |v: &Value| {
            let c = v["category"].as_str().unwrap_or("");
            catalog::CATEGORY_ORDER
                .iter()
                .position(|x| *x == c)
                .unwrap_or(usize::MAX)
        };
        cat(a)
            .cmp(&cat(b))
            .then_with(|| {
                let la = a["learned"].as_bool().unwrap_or(false);
                let lb = b["learned"].as_bool().unwrap_or(false);
                lb.cmp(&la) // learned first
            })
            .then_with(|| {
                a["label"]
                    .as_str()
                    .unwrap_or("")
                    .cmp(b["label"].as_str().unwrap_or(""))
            })
    });

    json!({
        "actor": actor,
        "found": found,
        "skills": skills,
    })
}

fn kind_str(kind: Kind) -> &'static str {
    match kind {
        Kind::Ladder => "ladder",
        Kind::Circle => "circle",
        Kind::Hunting => "hunting",
        Kind::Binary => "binary",
        Kind::Language => "language",
    }
}

/// One declarative skill edit: set skill `base` (for `actor`, default [`HERO`])
/// to `tier`. `tier` is `Untrained` to unlearn (or lower to the Untrained rung),
/// or a tier/learn value from the skill's options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillSetEdit {
    pub actor: String,
    pub base: String,
    pub tier: String,
}

impl SkillSetEdit {
    pub fn from_json(value: &Value) -> Result<Self, CoreError> {
        let obj = value.as_object().ok_or_else(|| {
            CoreError::InvalidRequest("private.skills.set value must be an object".to_string())
        })?;
        let base = obj
            .get("base")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                CoreError::InvalidRequest(
                    "private.skills.set requires a non-empty string value.base".to_string(),
                )
            })?
            .to_string();
        let tier = obj
            .get("tier")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                CoreError::InvalidRequest(
                    "private.skills.set requires a non-empty string value.tier".to_string(),
                )
            })?
            .to_string();
        let actor = obj
            .get("actor")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .unwrap_or(HERO)
            .to_string();
        // Validate against the catalog before this ever composes a GE class: an
        // unknown base or a tier outside the skill's options (a stale UI option,
        // a typo, a manual API caller) would otherwise build and write a
        // `GE_Skill_*` reference the game does not define.
        //
        // Untrained is the one exception, because it composes nothing: it
        // REMOVES the element the save already carries. Refusing it for an
        // uncatalogued base would make whatever the save holds — a skill the
        // game ignores, a console `addskill`, a class from a newer game version
        // — permanently unremovable.
        match catalog::find(&base) {
            Some(def) => {
                if !catalog::valid_tiers(def).contains(&tier.as_str()) {
                    return Err(CoreError::InvalidRequest(format!(
                        "private.skills.set: tier {tier:?} is not valid for skill {base:?}"
                    )));
                }
            }
            None if tier == "Untrained" => {}
            None => {
                return Err(CoreError::InvalidRequest(format!(
                    "private.skills.set: unknown skill base {base:?} can only be set to Untrained"
                )));
            }
        }
        Ok(SkillSetEdit { actor, base, tier })
    }
}

/// Apply one skill intent to the decompressed private `payload`.
///
/// Transitions (resolved fresh from the payload each call, addressed by skill
/// base, so a batch of these is order-independent and offset-safe):
/// - learned, `tier == Untrained`, no `_Untrained` class → remove EVERY element
///   of that skill.
/// - learned, target class differs → keep the first element, remove any further
///   ones, and retarget the survivor's `Def` in place (string patch; also covers
///   lowering to an existing `_Untrained` class).
/// - not learned, `tier != Untrained` → clone a donor element (or the embedded
///   template on an empty array) and retarget its `Def`.
/// - otherwise a no-op.
pub(crate) fn apply_skill_set(
    payload: &mut Vec<u8>,
    edit: &SkillSetEdit,
    cache: &mut crate::PayloadRoot,
) -> Result<(), CoreError> {
    // `fresh`, not `structural`: the decision below reads decoded Def strings off
    // the tree, so a parse that predates an in-place write by an earlier edit in
    // the batch could pick the wrong element.
    let root = cache.fresh(payload)?;
    let (base_path, elements) = locate_active_effects(root, &edit.actor).ok_or_else(|| {
        CoreError::UnsupportedEdit(format!(
            "actor {:?} has no ActiveEffects array to edit skills in",
            edit.actor
        ))
    })?;

    let def = catalog::find(&edit.base);
    let has_untrained = def.map(|d| d.has_untrained).unwrap_or(false);
    let want_untrained = edit.tier == "Untrained";
    let target_class = catalog::skill_class(&edit.base, &edit.tier);

    // Find EVERY element for this base (and note a same-category donor for the
    // learn path). All of them, not just the first: a save can hold two rungs of
    // one skill (the game leaves Cavalorn's first scutes lesson in place when it
    // grants the second), and an edit that touched only one would leave the
    // other behind — the skill would come back on the next read, and the rung the
    // user did not see would decide what the hero can do.
    let mut existing: Vec<usize> = Vec::new();
    let mut same_category_donor: Option<usize> = None;
    let mut any_donor: Option<usize> = None;
    let target_category = def.map(|d| d.category);
    for (idx, element) in elements.iter().enumerate() {
        let Some(class) = element_class(element) else {
            continue;
        };
        let Some((b, _)) = catalog::split_class(class) else {
            continue;
        };
        any_donor.get_or_insert(idx);
        if b == edit.base {
            existing.push(idx);
        }
        if target_category.is_some() && catalog::find(&b).map(|d| d.category) == target_category {
            same_category_donor.get_or_insert(idx);
        }
    }

    let array_path = {
        let mut p = base_path.clone();
        p.push(format!("{{{}}}", edit.actor));
        p.push("ActiveEffects".to_string());
        p
    };
    let element_count = elements.len();
    // Read the existing element's class here, while the tree is still in hand; the
    // borrow of the cache ends with this block so the writes below can hand it their
    // proof parses.
    let keep = existing.first().copied();
    let existing_class = keep
        .and_then(|idx| elements.get(idx))
        .and_then(element_class)
        .map(str::to_string);
    // Every other element of this skill goes, whatever rung it sits on, so the
    // skill ends up saying exactly one thing.
    let surplus: Vec<usize> = existing.iter().skip(1).copied().collect();

    match keep {
        Some(idx) => {
            if want_untrained && !has_untrained {
                // Unlearn: no `_Untrained` class exists, so the elements go.
                return container_edit(
                    payload,
                    cache,
                    &array_path,
                    ContainerEdit::ArrayRemoveMany(existing),
                );
            }
            if !surplus.is_empty() {
                // Removing only higher indices leaves `idx` where it is.
                container_edit(
                    payload,
                    cache,
                    &array_path,
                    ContainerEdit::ArrayRemoveMany(surplus),
                )?;
            }
            if existing_class.as_deref() == Some(target_class.as_str()) {
                Ok(()) // already the requested class
            } else {
                retarget_def(payload, cache, &array_path, idx, &target_class)
            }
        }
        None => {
            if want_untrained {
                return Ok(()); // nothing to unlearn
            }
            // Learn: clone a donor element, else append the embedded template.
            // Clone an existing SKILL effect (same category preferred) so the new
            // element carries a real skill effect's shape. Only `Def` is
            // retargeted, so cloning a non-skill effect (e.g. a lone
            // status/temporary effect when no skill element exists) would build
            // the skill from the wrong serialized effect — in that case append
            // the captured donor template instead of duplicating an arbitrary
            // element.
            match same_category_donor.or(any_donor) {
                Some(donor_idx) => {
                    container_edit(
                        payload,
                        cache,
                        &array_path,
                        ContainerEdit::ArrayDuplicate(donor_idx),
                    )?;
                    // ArrayDuplicate inserts the copy right after the source.
                    retarget_def(payload, cache, &array_path, donor_idx + 1, &target_class)
                }
                None => {
                    container_edit(
                        payload,
                        cache,
                        &array_path,
                        ContainerEdit::ArrayInsertBytes(donor::donor_template()),
                    )?;
                    // ArrayInsertBytes appends at the end of the array.
                    retarget_def(payload, cache, &array_path, element_count, &target_class)
                }
            }
        }
    }
}

/// Resolves through `cache` and hands it the proof parse afterwards, so a run of
/// skill edits in one write parses the payload once per edit rather than per step.
fn container_edit(
    payload: &mut Vec<u8>,
    cache: &mut crate::PayloadRoot,
    array_path: &[String],
    edit: ContainerEdit,
) -> Result<(), CoreError> {
    let (target, enclosing) = {
        let root = cache.fresh(payload)?;
        let segs = properties::parse_path(array_path)?;
        let resolved = properties::resolve_chain(&root.properties, &segs)?;
        (resolved.target.clone(), resolved.enclosing_size_fields)
    };
    let mut patched = payload.clone();
    properties::patch_container(&mut patched, &target, &enclosing, &edit)?;
    let proof = properties::parse_private_root(&patched).map_err(|err| {
        CoreError::Parse(format!(
            "skill container edit produced an inconsistent payload: {err}"
        ))
    })?;
    *payload = patched;
    cache.adopt(proof);
    Ok(())
}

/// Retarget the `EffectSpec/Def` ObjectProperty of the array element at `index`
/// to `new_class` (a length-changing string patch), validating on a scratch copy.
/// Resolves through `cache` and hands it the proof parse afterwards.
fn retarget_def(
    payload: &mut Vec<u8>,
    cache: &mut crate::PayloadRoot,
    array_path: &[String],
    index: usize,
    new_class: &str,
) -> Result<(), CoreError> {
    let mut def_path = array_path.to_vec();
    def_path.push(format!("[{index}]"));
    def_path.push("EffectSpec".to_string());
    def_path.push("Def".to_string());

    let (target, enclosing) = {
        let root = cache.fresh(payload)?;
        let segs = properties::parse_path(&def_path)?;
        let resolved = properties::resolve_chain(&root.properties, &segs)?;
        (resolved.target.clone(), resolved.enclosing_size_fields)
    };
    let mut patched = payload.clone();
    properties::patch_string(&mut patched, &target, &enclosing, new_class)?;
    let proof = properties::parse_private_root(&patched).map_err(|err| {
        CoreError::Parse(format!(
            "skill Def retarget produced an inconsistent payload: {err}"
        ))
    })?;
    *payload = patched;
    // The proof describes exactly the bytes just installed, so the next edit in the
    // batch resolves against it instead of parsing the payload again.
    cache.adopt(proof);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opt_values(opts: &[Value]) -> Vec<String> {
        opts.iter()
            .map(|o| o["value"].as_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn ladder_learned_options_include_untrained_and_tiers() {
        let def = catalog::find("Melee_OneHanded").unwrap();
        let opts = tier_options(def, true, "Master");
        assert_eq!(opt_values(&opts), ["Untrained", "Trained", "Master"]);
    }

    #[test]
    fn roster_ladder_offers_full_ladder() {
        let def = catalog::find("Melee_TwoHanded").unwrap();
        let opts = tier_options(def, false, "Untrained");
        assert_eq!(opt_values(&opts), ["Untrained", "Trained", "Master"]);
    }

    #[test]
    fn binary_learned_offers_unlearn() {
        let def = catalog::find("Acrobatics").unwrap();
        let opts = tier_options(def, true, "Learned");
        assert_eq!(opt_values(&opts), ["Learned", "Untrained"]);
    }

    #[test]
    fn binary_roster_offers_learn() {
        let def = catalog::find("Sneak").unwrap();
        let opts = tier_options(def, false, "Untrained");
        assert_eq!(opt_values(&opts), ["Untrained", "Learned"]);
    }

    #[test]
    fn circle_learned_lists_all_rungs_then_untrained() {
        let def = catalog::find("Mage_Circle").unwrap();
        let opts = tier_options(def, true, "6");
        assert_eq!(
            opt_values(&opts),
            ["Untrained", "Amateur", "1", "2", "3", "4", "5", "6"]
        );
    }

    #[test]
    fn blacksmith_lists_the_full_ladder() {
        let def = catalog::find("Crafting_Blacksmith").unwrap();
        let opts = tier_options(def, false, "Untrained");
        assert_eq!(opt_values(&opts), ["Untrained", "Trained", "Master"]);
    }

    #[test]
    fn the_scutes_ladder_offers_both_of_cavalorns_lessons() {
        let def = catalog::find("Hunting_Scutes").unwrap();
        assert_eq!(
            opt_values(&tier_options(def, true, "Trained")),
            ["Untrained", "Trained", "Master"]
        );
        // The rung that unlocks razor plates has to be expressible at all — the
        // single-state shape this skill used to have could only write Trained.
        assert!(
            SkillSetEdit::from_json(&json!({"base": "Hunting_Scutes", "tier": "Master"})).is_ok()
        );
    }

    #[test]
    fn a_skill_reads_as_its_highest_rung() {
        let def = catalog::find("Hunting_Scutes");
        // Array order must not decide: the game grants the second scutes lesson
        // without removing the first, and the higher class implies the lower.
        let both = vec![
            ("Hunting_Scutes".to_string(), Some("Trained".to_string())),
            ("Hunting_Scutes".to_string(), Some("Master".to_string())),
        ];
        assert_eq!(
            best_tier(&both, "Hunting_Scutes", def),
            Some("Master".to_string())
        );
        let reversed = vec![both[1].clone(), both[0].clone()];
        assert_eq!(
            best_tier(&reversed, "Hunting_Scutes", def),
            Some("Master".to_string())
        );
        // An uncatalogued class has no ladder to rank by: keep what was found.
        let raw = vec![("Whatever".to_string(), Some("Trained".to_string()))];
        assert_eq!(
            best_tier(&raw, "Whatever", None),
            Some("Trained".to_string())
        );
    }

    #[test]
    fn an_uncatalogued_untrained_class_still_offers_removal() {
        // The UI drops an edit that re-states the current value, so a row whose
        // only option IS its current value can never be acted on. A class whose
        // own suffix reads as `Untrained` used to produce exactly that, which
        // left the element in the save for good.
        let (current, options) = uncatalogued_state(Some("Untrained"));
        assert_eq!(current, "Learned");
        assert_eq!(opt_values(&options), ["Learned", "Untrained"]);
        assert_ne!(options[0], options[1]);

        // A rank we cannot place is still reported as itself.
        let (current, options) = uncatalogued_state(Some("Master"));
        assert_eq!(current, "Master");
        assert_eq!(opt_values(&options), ["Master", "Untrained"]);

        // And a suffix-less class keeps the sentinel it always had.
        let (current, options) = uncatalogued_state(None);
        assert_eq!(current, "Learned");
        assert_eq!(opt_values(&options), ["Learned", "Untrained"]);
    }

    #[test]
    fn an_uncatalogued_class_can_always_be_dropped() {
        // Whatever a save carries has to be removable, even a class this editor
        // knows nothing about: a skill the game ignores, a console `addskill`,
        // or one a newer game version added.
        let edit =
            SkillSetEdit::from_json(&json!({"base": "Hunting_Whatever", "tier": "Untrained"}))
                .expect("an unknown base can be unlearned");
        assert_eq!(edit.base, "Hunting_Whatever");
        assert_eq!(edit.tier, "Untrained");
    }

    #[test]
    fn an_uncatalogued_class_cannot_be_learned_or_retiered() {
        // The other direction stays shut: composing a class out of a base this
        // catalog never verified would write a reference the game may not define.
        for tier in ["Trained", "Master", "Learned", "6"] {
            let err = SkillSetEdit::from_json(&json!({"base": "Hunting_Whatever", "tier": tier}))
                .expect_err("an unknown base must not be learnable");
            assert!(
                format!("{err}").contains("can only be set to Untrained"),
                "unexpected error for tier {tier}: {err}"
            );
        }
    }

    #[test]
    fn edit_from_json_defaults_actor_to_hero() {
        let e = SkillSetEdit::from_json(&json!({"base": "Sneak", "tier": "Learned"})).unwrap();
        assert_eq!(e.actor, "Hero");
        assert_eq!(e.base, "Sneak");
        assert_eq!(e.tier, "Learned");
    }

    #[test]
    fn edit_from_json_requires_base_and_tier() {
        assert!(SkillSetEdit::from_json(&json!({"tier": "Learned"})).is_err());
        assert!(SkillSetEdit::from_json(&json!({"base": "Sneak"})).is_err());
    }

    #[test]
    fn edit_from_json_rejects_unknown_base() {
        assert!(SkillSetEdit::from_json(&json!({"base": "NotASkill", "tier": "Master"})).is_err());
    }

    #[test]
    fn edit_from_json_rejects_tier_not_in_skill_options() {
        // Master is not a valid tier for a binary skill (only Untrained/Learned).
        assert!(SkillSetEdit::from_json(&json!({"base": "Sneak", "tier": "Master"})).is_err());
        // Circle 7 is out of the Mage_Circle ladder (Amateur, 1..6).
        assert!(SkillSetEdit::from_json(&json!({"base": "Mage_Circle", "tier": "7"})).is_err());
        // A valid ladder tier is accepted.
        assert!(SkillSetEdit::from_json(&json!({"base": "Ranged_Bow", "tier": "Master"})).is_ok());
    }
}
