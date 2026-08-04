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

/// List the actor's skills: every learned skill (from its ActiveEffects array)
/// plus every catalogued skill it has not learned (the learnable roster). Each
/// entry carries the tier options the UI renders. `actor` is normally [`HERO`].
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
    // still surface it as a raw entry so nothing is silently hidden). Emit one
    // row per base: the UI keys pending edits by base and `apply_skill_set`
    // targets the first matching element, so a save with duplicate effects for
    // the same skill must not produce multiple (divergent) rows.
    let mut emitted_bases: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for (base, tier) in &learned {
        if !emitted_bases.insert(base.as_str()) {
            continue;
        }
        let def = catalog::find(base);
        let kind = def.map(|d| d.kind).unwrap_or(Kind::Ladder);
        let current = current_value(tier.as_deref());
        let (label, category, has_untrained, mut options) = match def {
            Some(d) => (
                d.label.to_string(),
                d.category.to_string(),
                d.has_untrained,
                tier_options(d, true, &current),
            ),
            None => (
                base.replace('_', " "),
                "Other".to_string(),
                false,
                vec![json!({ "value": current })],
            ),
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

    // Roster: every catalogued skill the actor has not learned.
    let learned_bases: std::collections::HashSet<&str> =
        learned.iter().map(|(b, _)| b.as_str()).collect();
    for def in catalog::SKILLS {
        if learned_bases.contains(def.base) {
            continue;
        }
        skills.push(json!({
            "base": def.base,
            "label": def.label,
            "category": def.category,
            "kind": kind_str(def.kind),
            "learned": false,
            "current": "Untrained",
            "hasUntrained": def.has_untrained,
            "options": tier_options(def, false, "Untrained"),
        }));
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
        let def = catalog::find(&base).ok_or_else(|| {
            CoreError::InvalidRequest(format!("private.skills.set: unknown skill base {base:?}"))
        })?;
        if !catalog::valid_tiers(def).contains(&tier.as_str()) {
            return Err(CoreError::InvalidRequest(format!(
                "private.skills.set: tier {tier:?} is not valid for skill {base:?}"
            )));
        }
        Ok(SkillSetEdit { actor, base, tier })
    }
}

/// Apply one skill intent to the decompressed private `payload`.
///
/// Transitions (resolved fresh from the payload each call, addressed by skill
/// base, so a batch of these is order-independent and offset-safe):
/// - learned, `tier == Untrained`, no `_Untrained` class → remove the element.
/// - learned, target class differs → retarget the element's `Def` in place
///   (string patch; also covers lowering to an existing `_Untrained` class).
/// - not learned, `tier != Untrained` → clone a donor element (or the embedded
///   template on an empty array) and retarget its `Def`.
/// - otherwise a no-op.
pub fn apply_skill_set(payload: &mut Vec<u8>, edit: &SkillSetEdit) -> Result<(), CoreError> {
    let root = properties::parse_private_root(payload)?;
    let (base_path, elements) = locate_active_effects(&root, &edit.actor).ok_or_else(|| {
        CoreError::UnsupportedEdit(format!(
            "actor {:?} has no ActiveEffects array to edit skills in",
            edit.actor
        ))
    })?;

    let def = catalog::find(&edit.base);
    let has_untrained = def.map(|d| d.has_untrained).unwrap_or(false);
    let want_untrained = edit.tier == "Untrained";
    let target_class = catalog::skill_class(&edit.base, &edit.tier);

    // Find an existing element for this base (and note a same-category donor for
    // the learn path).
    let mut existing: Option<usize> = None;
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
            // First match wins, matching list_skills' dedup (keep-first) so the
            // row the UI shows and the element this edits are the same one.
            existing.get_or_insert(idx);
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

    match existing {
        Some(idx) => {
            if want_untrained && !has_untrained {
                // Unlearn: no `_Untrained` class exists, so remove the element.
                container_edit(payload, &array_path, ContainerEdit::ArrayRemove(idx))
            } else if element_class_at(payload, &array_path, idx)?.as_deref()
                == Some(target_class.as_str())
            {
                Ok(()) // already the requested class
            } else {
                retarget_def(payload, &array_path, idx, &target_class)
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
                        &array_path,
                        ContainerEdit::ArrayDuplicate(donor_idx),
                    )?;
                    // ArrayDuplicate inserts the copy right after the source.
                    retarget_def(payload, &array_path, donor_idx + 1, &target_class)
                }
                None => {
                    container_edit(
                        payload,
                        &array_path,
                        ContainerEdit::ArrayInsertBytes(donor::donor_template()),
                    )?;
                    // ArrayInsertBytes appends at the end of the array.
                    retarget_def(payload, &array_path, element_count, &target_class)
                }
            }
        }
    }
}

/// Re-parse the payload and read the class string of the element at `index`.
fn element_class_at(
    payload: &[u8],
    array_path: &[String],
    index: usize,
) -> Result<Option<String>, CoreError> {
    let root = properties::parse_private_root(payload)?;
    let segs = properties::parse_path(array_path)?;
    let resolved = properties::resolve_chain(&root.properties, &segs)?;
    let PropertyValue::Array { elements } = &resolved.target.value else {
        return Ok(None);
    };
    Ok(elements
        .get(index)
        .and_then(element_class)
        .map(str::to_string))
}

/// Apply a structural container edit to the array at `array_path`, validating on
/// a scratch copy before committing (mirrors the generic typed container apply).
fn container_edit(
    payload: &mut Vec<u8>,
    array_path: &[String],
    edit: ContainerEdit,
) -> Result<(), CoreError> {
    let root = properties::parse_private_root(payload)?;
    let segs = properties::parse_path(array_path)?;
    let resolved = properties::resolve_chain(&root.properties, &segs)?;
    let target = resolved.target.clone();
    let mut patched = payload.clone();
    properties::patch_container(
        &mut patched,
        &target,
        &resolved.enclosing_size_fields,
        &edit,
    )?;
    properties::parse_private_root(&patched).map_err(|err| {
        CoreError::Parse(format!(
            "skill container edit produced an inconsistent payload: {err}"
        ))
    })?;
    *payload = patched;
    Ok(())
}

/// Retarget the `EffectSpec/Def` ObjectProperty of the array element at `index`
/// to `new_class` (a length-changing string patch), validating on a scratch copy.
fn retarget_def(
    payload: &mut Vec<u8>,
    array_path: &[String],
    index: usize,
    new_class: &str,
) -> Result<(), CoreError> {
    let mut def_path = array_path.to_vec();
    def_path.push(format!("[{index}]"));
    def_path.push("EffectSpec".to_string());
    def_path.push("Def".to_string());

    let root = properties::parse_private_root(payload)?;
    let segs = properties::parse_path(&def_path)?;
    let resolved = properties::resolve_chain(&root.properties, &segs)?;
    let target = resolved.target.clone();
    let mut patched = payload.clone();
    properties::patch_string(
        &mut patched,
        &target,
        &resolved.enclosing_size_fields,
        new_class,
    )?;
    properties::parse_private_root(&patched).map_err(|err| {
        CoreError::Parse(format!(
            "skill Def retarget produced an inconsistent payload: {err}"
        ))
    })?;
    *payload = patched;
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
