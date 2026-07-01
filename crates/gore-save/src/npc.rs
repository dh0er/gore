//! NPC record locator: resolve a character's records by GlobalId across the
//! parallel `CharacterStateSaveGameData_*` maps and read summary state.
//!
//! The save stores character data as several parallel maps keyed by GlobalId (a
//! String), nested deep inside the profile tree (under `m_Profile` /
//! `m_GenericData`, not at the top level). Each map's entry *values* are structs
//! whose map-value descriptor `struct_type` is one of
//! `CharacterStateSaveGameData_Attributes`, `_ActiveEffects`, `_Inventory`, etc.
//! The `_Attributes` map (1484 entries) is the canonical NPC set.
//!
//! This module walks the parsed tree to find those maps by descriptor type and
//! exposes lookups by GlobalId. It is read-only — no payload mutation.

use std::collections::HashSet;

use serde::Serialize;

use crate::CoreError;
use crate::properties::{
    ContainerEdit, Property, PropertyValue, RootObject, ScalarValue, StructValue,
    find_property_by_name, map_key_to_string, parse_path, parse_private_root, patch_container,
    patch_map_value_tag_container, patch_scalar, resolve_chain,
};

const ATTRIBUTES_TYPE: &str = "CharacterStateSaveGameData_Attributes";
const INVENTORY_TYPE: &str = "CharacterStateSaveGameData_Inventory";

/// The map (a `MapProperty` keyed by GlobalId, value = native `GameplayTagContainer`)
/// holding each character's persisted GAS *loose* tag-state. Native RE pinned this
/// as the AUTHORITATIVE death gate: on load AngelScript re-applies these tags, so an
/// NPC is dead iff its entry carries [`DEAD_TAG_STATE`] / `State.Dead`.
const LOOSE_TAGS_MAP: &str = "LooseTagsByGlobalId";

/// The authoritative on-load "this NPC is dead" marker: a killed NPC's
/// `LooseTagsByGlobalId[<id>]` GameplayTagContainer contains exactly
/// `State.KillBountyGranted`, `State.ExecutedBountyGranted`, `State.Dead`. An alive
/// (incl. merely-defeated) NPC carries NONE of these. `State.Dead` is the decisive
/// one; `is_dead` keys off it, and Revive strips all three (see [`apply_revive`]).
const DEAD_LOOSE_TAGS: &[&str] = &[
    "State.KillBountyGranted",
    "State.ExecutedBountyGranted",
    "State.Dead",
];

/// The single decisive loose tag the game reads to decide dead-vs-alive on load.
const DEAD_TAG_STATE: &str = "State.Dead";

/// The map (under `LongTermMemoryByGlobalId`) keyed by GlobalId whose entry values
/// hold each NPC's `CharacterStateSaveGameData_LongTermMemory` — a `MemorizedEvents`
/// array of memory-event structs, each carrying an `EventTags` GameplayTagContainer.
const LONG_TERM_MEMORY_MAP: &str = "LongTermMemoryByGlobalId";

/// The global map (under `m_GenericData/{GameStateDataBase}`) holding lootable
/// corpse inventories, keyed by `"Character_" + GlobalId`. A killed NPC gets an
/// entry here (its droppable loot); an ALIVE NPC has none. Removing the entry is
/// (almost certainly) the on-load "this NPC is dead" signal — see [`apply_revive`].
const SAVED_INVENTORIES_MAP: &str = "m_SavedInventories";

/// The memory-event tags that mark an NPC as **killed** (dead). Death is binary:
/// only an actual kill counts — a merely *defeated* (knocked-out) NPC is ALIVE.
/// HP is NOT the signal (a killed NPC keeps positive HP; a defeated one has HP 0).
/// `is_dead` is driven by these tags on a `MemorizedEvents` element's `EventTags`.
const KILL_EVENT_TAGS: &[&str] = &[
    "Memory.Character.Defeated.Kill",
    "Memory.Execution",
];

/// Memory-event tags cleared by Revive (a superset of [`KILL_EVENT_TAGS`]): reviving
/// a killed NPC also wipes any defeat residue so it returns to a clean alive state.
/// Revive only applies to dead (killed) NPCs; on an only-defeated NPC it never runs.
const REVIVE_EVENT_TAGS: &[&str] = &[
    "Memory.Combat.WasDefeated",
    "Memory.SaveAndLoad.Defeated",
    "Memory.Character.Defeated.Kill",
    "Memory.Execution",
    "Memory.KilledInsideCamp",
];

/// One NPC's summary state: HP (from its `_Attributes` record) plus whether it is
/// dead (killed — from its long-term memory events).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcSummary {
    pub id: String,
    /// True iff the NPC is DEAD per the authoritative native gate: `State.Dead` is
    /// present in its `LooseTagsByGlobalId` GameplayTagContainer (the GAS tag-state
    /// AngelScript re-applies on load). A merely-defeated/knocked-out NPC carries no
    /// such tag and is NOT dead. Drives the dead avatar + the Revive action.
    pub is_dead: bool,
    pub hp: Option<f32>,
    pub max_hp: Option<f32>,
}

/// A character row for the unified `private.characters.list`. Extends the NPC
/// summary with the knowledge UniqueName and per-aspect availability flags, so
/// the frontend needs one call to build the master list. `global_id` is `None`
/// for knowledge-only "orphan" rows that have no spawned actor.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterSummary {
    pub global_id: Option<String>,
    /// GlobalId prefix (before the first `-`), the key the knowledge map uses.
    pub unique_name: String,
    pub is_dead: bool,
    pub has_inventory: bool,
    pub has_knowledge: bool,
    pub has_events: bool,
}

/// The knowledge/UniqueName key derived from a GlobalId: the substring before
/// the first `-`, lower-cased. Proven (spec measurement) to equal the knowledge
/// map key for every non-orphan character.
pub(crate) fn char_key(global_id: &str) -> String {
    global_id
        .split('-')
        .next()
        .unwrap_or(global_id)
        .to_ascii_lowercase()
}

/// The struct type of a Map property's entry *values*, which lives on the map
/// *descriptor's* value `InnerDescriptor` (the parsed `StructValue` carries no
/// type name). `None` for non-map / non-struct-valued maps.
fn map_value_struct_type(prop: &Property) -> Option<&str> {
    let (_key, value) = prop.descriptor.map.as_deref()?;
    value.struct_type.as_ref().map(|(ty, _pkg)| ty.as_str())
}

/// Find the character-state map of `struct_type` and return its entries keyed by
/// stringified GlobalId. There is exactly one such map per type in the tree;
/// `find_map` returns the first match.
fn find_character_map<'a>(
    root: &'a RootObject,
    struct_type: &str,
) -> Option<&'a [(PropertyValue, PropertyValue)]> {
    // The two `if`s are not collapsible: on a non-matching Map we still fall
    // through to recurse into it.
    #[allow(clippy::collapsible_if)]
    fn in_props<'a>(
        props: &'a [Property],
        struct_type: &str,
    ) -> Option<&'a [(PropertyValue, PropertyValue)]> {
        for p in props {
            if let PropertyValue::Map { entries, .. } = &p.value {
                if map_value_struct_type(p) == Some(struct_type) {
                    return Some(entries);
                }
            }
            // Otherwise recurse: the target map is nested deep under the profile.
            if let Some(found) = in_value(&p.value, struct_type) {
                return Some(found);
            }
        }
        None
    }
    fn in_value<'a>(
        value: &'a PropertyValue,
        struct_type: &str,
    ) -> Option<&'a [(PropertyValue, PropertyValue)]> {
        match value {
            PropertyValue::Struct(StructValue::Properties(inner)) => in_props(inner, struct_type),
            PropertyValue::Struct(StructValue::Instanced(Some(i))) => {
                in_props(&i.properties, struct_type)
            }
            PropertyValue::ObjectInstances(objs) => {
                objs.iter().find_map(|o| in_props(&o.properties, struct_type))
            }
            PropertyValue::Array { elements } | PropertyValue::Set { elements, .. } => {
                elements.iter().find_map(|e| in_value(e, struct_type))
            }
            PropertyValue::Map { entries, .. } => entries
                .iter()
                .find_map(|(_k, v)| in_value(v, struct_type)),
            _ => None,
        }
    }
    in_props(&root.properties, struct_type)
}

/// Look up a single entry value in a character-state map by stringified key.
fn lookup_entry<'a>(
    entries: &'a [(PropertyValue, PropertyValue)],
    id: &str,
) -> Option<&'a PropertyValue> {
    entries
        .iter()
        .find(|(k, _v)| map_key_to_string(k).as_deref() == Some(id))
        .map(|(_k, v)| v)
}

/// The entry value as a property list (every character-state entry value is a
/// proplist struct).
fn entry_props(value: &PropertyValue) -> Option<&[Property]> {
    match value {
        PropertyValue::Struct(StructValue::Properties(p)) => Some(p),
        PropertyValue::Struct(StructValue::Instanced(Some(i))) => Some(&i.properties),
        _ => None,
    }
}

/// Borrow the top-level property of a character-state entry value for `id`.
///
/// A map entry is a `(key, value)` pair, not a named `Property`, so there is no
/// `&Property` wrapping the whole entry. Each record's value is a struct proplist
/// holding exactly one meaningful payload property (the record's content), so the
/// accessors return that first property — later tasks (5–7) descend into its
/// value. Returns `None` if the map or the `id` is absent.
fn npc_entry<'a>(root: &'a RootObject, id: &str, struct_type: &str) -> Option<&'a Property> {
    let entries = find_character_map(root, struct_type)?;
    let value = lookup_entry(entries, id)?;
    entry_props(value)?.first()
}

/// `_Attributes` record property for `id`, or `None` if absent.
pub fn npc_attributes_entry<'a>(root: &'a RootObject, id: &str) -> Option<&'a Property> {
    npc_entry(root, id, ATTRIBUTES_TYPE)
}

/// The full property list of the `_Attributes` record value for `id`, or `None`
/// if the map or `id` is absent.
///
/// Unlike [`npc_attributes_entry`] (which returns only the entry value's *first*
/// property), this exposes the whole entry value so callers can navigate every
/// attribute it contains. Task 5+ descends from here.
pub fn npc_attributes_props<'a>(root: &'a RootObject, id: &str) -> Option<&'a [Property]> {
    let entries = find_character_map(root, ATTRIBUTES_TYPE)?;
    let value = lookup_entry(entries, id)?;
    entry_props(value)
}

/// `_Inventory` record property for `id`, or `None` if absent.
pub fn npc_inventory_entry<'a>(root: &'a RootObject, id: &str) -> Option<&'a Property> {
    npc_entry(root, id, INVENTORY_TYPE)
}

/// `(BaseValue, CurrentValue)` of a `GameplayAttributeData`-style struct.
fn gameplay_attribute_floats(props: &[Property]) -> (Option<f32>, Option<f32>) {
    let mut base = None;
    let mut current = None;
    for p in props {
        if let PropertyValue::Float(f) = p.value {
            match p.name.as_str() {
                "BaseValue" => base = Some(f),
                "CurrentValue" => current = Some(f),
                _ => {}
            }
        }
    }
    (base, current)
}

/// Walk an `_Attributes` entry value recursively for the `Health` / `MaxHealth`
/// attributes. Mirrors `deep_floats_health` in `work/rd6.py`: any map entry
/// whose key stringifies to "Health"/"MaxHealth" and whose value is a
/// `GameplayAttributeData` proplist contributes its `BaseValue`/`CurrentValue`.
fn health_floats(value: &PropertyValue) -> HealthFloats {
    let mut out = HealthFloats::default();
    walk_health(value, &mut out);
    out
}

#[derive(Default)]
struct HealthFloats {
    health_base: Option<f32>,
    health_current: Option<f32>,
    max_health_base: Option<f32>,
}

fn walk_health(value: &PropertyValue, out: &mut HealthFloats) {
    match value {
        PropertyValue::Map { entries, .. } => {
            for (k, v) in entries {
                if let (Some(key), Some(props)) = (map_key_to_string(k), entry_props(v)) {
                    match key.as_str() {
                        "Health" => {
                            let (base, current) = gameplay_attribute_floats(props);
                            out.health_base = out.health_base.or(base);
                            out.health_current = out.health_current.or(current);
                        }
                        "MaxHealth" => {
                            let (base, _current) = gameplay_attribute_floats(props);
                            out.max_health_base = out.max_health_base.or(base);
                        }
                        _ => {}
                    }
                }
                walk_health(v, out);
            }
        }
        PropertyValue::Struct(StructValue::Properties(inner)) => {
            for p in inner {
                walk_health(&p.value, out);
            }
        }
        PropertyValue::Struct(StructValue::Instanced(Some(i))) => {
            for p in &i.properties {
                walk_health(&p.value, out);
            }
        }
        PropertyValue::Array { elements } | PropertyValue::Set { elements, .. } => {
            for e in elements {
                walk_health(e, out);
            }
        }
        PropertyValue::ObjectInstances(objs) => {
            for o in objs {
                for p in &o.properties {
                    walk_health(&p.value, out);
                }
            }
        }
        _ => {}
    }
}

/// Does any `GameplayTagContainer` reachable inside `value` carry `tag`?
fn value_has_tag(value: &PropertyValue, tag: &str) -> bool {
    match value {
        PropertyValue::Struct(StructValue::GameplayTagContainer(tags)) => {
            tags.iter().any(|t| t == tag)
        }
        PropertyValue::Struct(StructValue::Properties(inner)) => {
            inner.iter().any(|p| value_has_tag(&p.value, tag))
        }
        PropertyValue::Struct(StructValue::Instanced(Some(i))) => {
            i.properties.iter().any(|p| value_has_tag(&p.value, tag))
        }
        PropertyValue::ObjectInstances(objs) => objs
            .iter()
            .any(|o| o.properties.iter().any(|p| value_has_tag(&p.value, tag))),
        PropertyValue::Array { elements } | PropertyValue::Set { elements, .. } => {
            elements.iter().any(|e| value_has_tag(e, tag))
        }
        PropertyValue::Map { entries, .. } => entries
            .iter()
            .any(|(_k, v)| value_has_tag(v, tag)),
        _ => false,
    }
}

/// NPC `id`'s persisted loose tags: the `GameplayTagContainer` stored as
/// `LooseTagsByGlobalId[<id>]`. Returns the tag list (possibly empty) if the map
/// and the entry exist, else `None` (no map / no entry for this id).
///
/// The map value is parsed inline as a native `GameplayTagContainer`
/// ([`StructValue::GameplayTagContainer`]) — the value descriptor's struct_type is
/// `GameplayTagContainer`, so the entry value is the container directly (no proplist
/// wrapper). Test helper (only the test build references it directly).
#[cfg_attr(not(test), allow(dead_code))]
fn loose_tags<'a>(root: &'a RootObject, id: &str) -> Option<&'a [String]> {
    let (_path, map_prop) = find_property_by_name(root, LOOSE_TAGS_MAP)?;
    let PropertyValue::Map { entries, .. } = &map_prop.value else {
        return None;
    };
    match lookup_entry(entries, id)? {
        PropertyValue::Struct(StructValue::GameplayTagContainer(tags)) => Some(tags.as_slice()),
        _ => None,
    }
}

/// Is NPC `id` dead per the AUTHORITATIVE native gate — `State.Dead` present in its
/// `LooseTagsByGlobalId` GameplayTagContainer? (The game re-applies these GAS tags
/// on load via AngelScript; this is the marker it reads, not HP or kill memory.)
/// Test helper (only the test build references it directly).
#[cfg_attr(not(test), allow(dead_code))]
fn is_dead_by_loose_tags(root: &RootObject, id: &str) -> bool {
    loose_tags(root, id).is_some_and(|tags| tags.iter().any(|t| t == DEAD_TAG_STATE))
}

/// The long-term memory map entry value for NPC `id` (the
/// `CharacterStateSaveGameData_LongTermMemory` whose `MemorizedEvents` array holds
/// the memory-event structs), or `None` if the map / `id` is absent.
fn long_term_memory_value<'a>(root: &'a RootObject, id: &str) -> Option<&'a PropertyValue> {
    let (_path, map_prop) = find_property_by_name(root, LONG_TERM_MEMORY_MAP)?;
    let PropertyValue::Map { entries, .. } = &map_prop.value else {
        return None;
    };
    lookup_entry(entries, id)
}

/// Does the NPC's long-term memory carry `tag` in any `EventTags`
/// `GameplayTagContainer`? Test helper (only the test build references it, so it is
/// dead code in a plain library build).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn memory_has_tag(root: &RootObject, id: &str, tag: &str) -> bool {
    long_term_memory_value(root, id).is_some_and(|v| value_has_tag(v, tag))
}

/// List every NPC (one per `_Attributes` entry) with its summary state.
///
/// - `hp` = Health `CurrentValue` (fallback `BaseValue`) so a wounded NPC shows
///   its current HP, matching the `HP {hp} / {maxHp}` UI; `max_hp` = MaxHealth `BaseValue`.
/// - `is_dead` = the AUTHORITATIVE native death gate: `State.Dead` is present in the
///   NPC's `LooseTagsByGlobalId` GameplayTagContainer (the persisted GAS tag-state
///   AngelScript re-applies on load). HP is NOT the signal, and neither is kill
///   memory: a merely-defeated (HP 0, knocked-out) NPC carries no `State.Dead` and is
///   ALIVE.
pub fn list_npcs(root: &RootObject) -> Result<Vec<NpcSummary>, CoreError> {
    let attributes = find_character_map(root, ATTRIBUTES_TYPE).ok_or_else(|| {
        CoreError::Parse(format!("no {ATTRIBUTES_TYPE} map found in save"))
    })?;
    // The loose-tags map (keyed by the same GlobalId) carries each NPC's persisted
    // GAS tag-state; `State.Dead` there is the authoritative dead marker.
    let loose = find_property_by_name(root, LOOSE_TAGS_MAP).and_then(|(_p, prop)| {
        match &prop.value {
            PropertyValue::Map { entries, .. } => Some(entries.as_slice()),
            _ => None,
        }
    });

    let mut out = Vec::with_capacity(attributes.len());
    for (key, value) in attributes {
        let Some(id) = map_key_to_string(key) else {
            continue;
        };
        let hp = health_floats(value);
        let is_dead = loose
            .and_then(|entries| lookup_entry(entries, &id))
            .is_some_and(|tags| value_has_tag(tags, DEAD_TAG_STATE));
        out.push(NpcSummary {
            id,
            is_dead,
            hp: hp.health_current.or(hp.health_base),
            max_hp: hp.max_health_base,
        });
    }
    Ok(out)
}

/// Collect the stringified keys of a named MapProperty found anywhere in the
/// tree (used for the knowledge + long-term-memory maps), lower-cased.
fn map_keys_lower(root: &RootObject, name: &str) -> HashSet<String> {
    match find_property_by_name(root, name) {
        Some((_, prop)) => match &prop.value {
            PropertyValue::Map { entries, .. } => entries
                .iter()
                .filter_map(|(k, _)| map_key_to_string(k))
                .map(|s| s.to_ascii_lowercase())
                .collect(),
            _ => HashSet::new(),
        },
        None => HashSet::new(),
    }
}

/// Collect the stringified GlobalId keys of a character-state map (by value
/// struct type), lower-cased.
fn character_map_keys_lower(root: &RootObject, struct_type: &str) -> HashSet<String> {
    match find_character_map(root, struct_type) {
        Some(entries) => entries
            .iter()
            .filter_map(|(k, _)| map_key_to_string(k))
            .map(|s| s.to_ascii_lowercase())
            .collect(),
        None => HashSet::new(),
    }
}

/// Build the unified character list: every spawned actor (from [`list_npcs`])
/// annotated with availability flags, followed by knowledge-only orphan rows
/// (a knowledge UniqueName with no matching actor charKey). The join is the
/// proven prefix rule ([`char_key`]).
pub fn list_characters(root: &RootObject) -> Result<Vec<CharacterSummary>, CoreError> {
    let knowledge = map_keys_lower(root, "CharacterKnowledgeByUniqueName");
    let events = map_keys_lower(root, LONG_TERM_MEMORY_MAP);
    let inventory = character_map_keys_lower(root, INVENTORY_TYPE);

    let npcs = list_npcs(root)?;
    let mut actor_keys: HashSet<String> = HashSet::new();
    let mut out: Vec<CharacterSummary> = Vec::with_capacity(npcs.len());
    for npc in &npcs {
        let key = char_key(&npc.id);
        actor_keys.insert(key.clone());
        let id_lower = npc.id.to_ascii_lowercase();
        out.push(CharacterSummary {
            global_id: Some(npc.id.clone()),
            unique_name: key.clone(),
            is_dead: npc.is_dead,
            has_inventory: inventory.contains(&id_lower),
            has_knowledge: knowledge.contains(&key),
            has_events: events.contains(&id_lower),
        });
    }
    // Knowledge-only orphans: a UniqueName with no actor charKey. Typically 0–1.
    let mut orphans: Vec<String> = knowledge
        .iter()
        .filter(|k| !actor_keys.contains(*k))
        .cloned()
        .collect();
    orphans.sort();
    for key in orphans {
        out.push(CharacterSummary {
            global_id: None,
            unique_name: key,
            is_dead: false,
            has_inventory: false,
            has_knowledge: true,
            has_events: false,
        });
    }
    Ok(out)
}

/// A filtered, sorted, paginated slice of NPC summaries.
pub struct NpcPage {
    pub npcs: Vec<NpcSummary>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

/// Filter NPCs by a case-insensitive substring of `query` on the id, sort by id
/// ascending (stable, deterministic pagination), and return the page selected by
/// `offset`/`limit`. An empty/whitespace `query` matches all.
///
/// `total` is the filtered count (before pagination); `offset` is clamped to that
/// total so an out-of-range page yields an empty `npcs` (never a panic).
pub fn paginate_npcs(
    mut npcs: Vec<NpcSummary>,
    query: &str,
    offset: usize,
    limit: usize,
) -> NpcPage {
    let needle = query.trim().to_ascii_lowercase();
    if !needle.is_empty() {
        npcs.retain(|n| n.id.to_ascii_lowercase().contains(&needle));
    }
    npcs.sort_by(|a, b| a.id.cmp(&b.id));

    let total = npcs.len();
    let start = offset.min(total);
    let end = start.saturating_add(limit).min(total);
    let page = npcs[start..end].to_vec();
    NpcPage {
        npcs: page,
        total,
        offset,
        limit,
    }
}

/// One editable attribute of an NPC, with the full typed paths
/// `private.typed.setValue` needs to write its `BaseValue` / `CurrentValue`.
///
/// `base_path`/`current_path` are segment lists in the form
/// [`crate::properties::parse_path`] accepts (property names verbatim,
/// `{mapKey}` for map keys), rooted at the private root — so
/// `parse_path(&row.base_path)` then `resolve(&root.properties, …)` lands on
/// that attribute's `BaseValue` FloatProperty.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcAttributeRow {
    pub key: String,
    pub base: Option<f32>,
    pub current: Option<f32>,
    pub base_path: Vec<String>,
    pub current_path: Vec<String>,
}

/// The `{mapKey}` path segment for a map key. Mirrors
/// [`crate::properties::parse_path`], which strips a single leading `{` and
/// trailing `}`: keys with `-`/`_`/`/` (e.g. dashed GlobalIds, slashed quest
/// class paths) survive verbatim because nothing inside is re-escaped.
fn map_key_segment(key: &str) -> String {
    format!("{{{key}}}")
}

/// A character-state map's entries plus the typed-path segment prefix from the
/// private root down to that map property.
type CharacterMapWithPath<'a> = (&'a [(PropertyValue, PropertyValue)], Vec<String>);

/// Find the character-state map of `struct_type` *and* the typed-path segment
/// prefix from the private root down to that map property (the property-name
/// segments crossed on the way). Mirrors [`find_character_map`]'s descent, but
/// accumulates the path so callers can build `setValue`-resolvable paths.
fn find_character_map_path<'a>(
    root: &'a RootObject,
    struct_type: &str,
) -> Option<CharacterMapWithPath<'a>> {
    // The two `if`s are not collapsible: on a non-matching Map we still fall
    // through to recurse into it.
    #[allow(clippy::collapsible_if)]
    fn in_props<'a>(
        props: &'a [Property],
        struct_type: &str,
        path: &mut Vec<String>,
    ) -> Option<&'a [(PropertyValue, PropertyValue)]> {
        for p in props {
            path.push(p.name.clone());
            if let PropertyValue::Map { entries, .. } = &p.value {
                if map_value_struct_type(p) == Some(struct_type) {
                    return Some(entries);
                }
            }
            if let Some(found) = in_value(&p.value, struct_type, path) {
                return Some(found);
            }
            path.pop();
        }
        None
    }
    fn in_value<'a>(
        value: &'a PropertyValue,
        struct_type: &str,
        path: &mut Vec<String>,
    ) -> Option<&'a [(PropertyValue, PropertyValue)]> {
        match value {
            PropertyValue::Struct(StructValue::Properties(inner)) => {
                in_props(inner, struct_type, path)
            }
            PropertyValue::Struct(StructValue::Instanced(Some(i))) => {
                in_props(&i.properties, struct_type, path)
            }
            PropertyValue::ObjectInstances(objs) => {
                for (idx, o) in objs.iter().enumerate() {
                    path.push(format!("[{idx}]"));
                    if let Some(found) = in_props(&o.properties, struct_type, path) {
                        return Some(found);
                    }
                    path.pop();
                }
                None
            }
            PropertyValue::Array { elements } | PropertyValue::Set { elements, .. } => {
                for (idx, e) in elements.iter().enumerate() {
                    path.push(format!("[{idx}]"));
                    if let Some(found) = in_value(e, struct_type, path) {
                        return Some(found);
                    }
                    path.pop();
                }
                None
            }
            PropertyValue::Map { entries, .. } => {
                for (k, v) in entries {
                    let Some(key) = map_key_to_string(k) else {
                        continue;
                    };
                    path.push(map_key_segment(&key));
                    if let Some(found) = in_value(v, struct_type, path) {
                        return Some(found);
                    }
                    path.pop();
                }
                None
            }
            _ => None,
        }
    }
    let mut path = Vec::new();
    let entries = in_props(&root.properties, struct_type, &mut path)?;
    Some((entries, path))
}

/// Accumulate one [`NpcAttributeRow`] per attribute reachable inside an
/// `_Attributes` entry value. Mirrors [`walk_health`]'s descent, but tracks the
/// running typed path so each row carries the full `BaseValue`/`CurrentValue`
/// paths. Any map entry whose value is a `GameplayAttributeData`-style proplist
/// (i.e. carries a `BaseValue`/`CurrentValue` FloatProperty) becomes a row keyed
/// by the entry's stringified map key (Health, MaxHealth, Mana, Strength, …).
fn collect_attribute_rows(
    value: &PropertyValue,
    path: &mut Vec<String>,
    out: &mut Vec<NpcAttributeRow>,
) {
    match value {
        PropertyValue::Map { entries, .. } => {
            for (k, v) in entries {
                let Some(key) = map_key_to_string(k) else {
                    continue;
                };
                path.push(map_key_segment(&key));
                if let Some(props) = entry_props(v) {
                    let (base, current) = gameplay_attribute_floats(props);
                    if base.is_some() || current.is_some() {
                        let mut base_path = path.clone();
                        base_path.push("BaseValue".to_string());
                        let mut current_path = path.clone();
                        current_path.push("CurrentValue".to_string());
                        out.push(NpcAttributeRow {
                            key,
                            base,
                            current,
                            base_path,
                            current_path,
                        });
                    }
                }
                collect_attribute_rows(v, path, out);
                path.pop();
            }
        }
        PropertyValue::Struct(StructValue::Properties(inner)) => {
            for p in inner {
                path.push(p.name.clone());
                collect_attribute_rows(&p.value, path, out);
                path.pop();
            }
        }
        PropertyValue::Struct(StructValue::Instanced(Some(i))) => {
            for p in &i.properties {
                path.push(p.name.clone());
                collect_attribute_rows(&p.value, path, out);
                path.pop();
            }
        }
        PropertyValue::Array { elements } | PropertyValue::Set { elements, .. } => {
            for (idx, e) in elements.iter().enumerate() {
                path.push(format!("[{idx}]"));
                collect_attribute_rows(e, path, out);
                path.pop();
            }
        }
        PropertyValue::ObjectInstances(objs) => {
            for (idx, o) in objs.iter().enumerate() {
                path.push(format!("[{idx}]"));
                for p in &o.properties {
                    path.push(p.name.clone());
                    collect_attribute_rows(&p.value, path, out);
                    path.pop();
                }
                path.pop();
            }
        }
        _ => {}
    }
}

/// Return one [`NpcAttributeRow`] per editable attribute of NPC `id`, each with
/// the full typed `base_path`/`current_path` that `private.typed.setValue` can
/// resolve against the private root.
///
/// Errors if the `_Attributes` map is absent; errors with a not-found message if
/// the map exists but holds no entry for `id`.
pub fn npc_attributes(root: &RootObject, id: &str) -> Result<Vec<NpcAttributeRow>, CoreError> {
    let (entries, mut path) = find_character_map_path(root, ATTRIBUTES_TYPE).ok_or_else(|| {
        CoreError::Parse(format!("no {ATTRIBUTES_TYPE} map found in save"))
    })?;
    let value = lookup_entry(entries, id)
        .ok_or_else(|| CoreError::Parse(format!("NPC {id:?} not found in {ATTRIBUTES_TYPE} map")))?;

    // The entry is addressed by its map key; descend from there.
    path.push(map_key_segment(id));
    let mut rows = Vec::new();
    collect_attribute_rows(value, &mut path, &mut rows);
    Ok(rows)
}

/// Full typed path (from the private root) to NPC `id`'s inventory container —
/// the property whose value holds the same `m_Keys` (enum array) / `m_Values` /
/// `Items` / `m_Slots` structure the PLAYER inventory traversal navigates (see
/// `resolve_inventory_path` in `lib.rs`). For the real save this property is
/// `InventoryItems`, the sole content property of the `_Inventory` map entry
/// value; the path is `<map prefix> / {id} / InventoryItems`.
///
/// The container property name is read from the parsed entry value's first
/// property (the record's content), mirroring [`npc_entry`]'s "first property is
/// the record content" convention, so it stays correct if the name differs.
/// Returns `None` if the `_Inventory` map / `id` / a content property is absent.
pub fn npc_inventory_path(root: &RootObject, global_id: &str) -> Option<Vec<String>> {
    let (entries, mut path) = find_character_map_path(root, INVENTORY_TYPE)?;
    let value = lookup_entry(entries, global_id)?;
    let container = entry_props(value)?.first()?;
    // The entry is addressed by its map key; then descend to the container prop.
    path.push(map_key_segment(global_id));
    path.push(container.name.clone());
    Some(path)
}

/// Borrow a named member of a struct value (proplist or instanced), or `None`.
/// Mirrors `lib.rs::struct_member`.
fn struct_member<'a>(value: &'a PropertyValue, name: &str) -> Option<&'a PropertyValue> {
    let props = match value {
        PropertyValue::Struct(StructValue::Properties(p)) => p,
        PropertyValue::Struct(StructValue::Instanced(Some(i))) => &i.properties,
        _ => return None,
    };
    props.iter().find(|p| p.name == name).map(|p| &p.value)
}

/// The `EventTags` `GameplayTagContainer` of a single memory-event element, or an
/// empty slice if absent. Each `MemorizedEvents` element is a `MemoryEvent` struct
/// whose `EventTags` member is a native GameplayTagContainer (read at
/// `lib.rs::progression_events`).
fn event_tags(element: &PropertyValue) -> &[String] {
    match struct_member(element, "EventTags") {
        Some(PropertyValue::Struct(StructValue::GameplayTagContainer(tags))) => tags,
        _ => &[],
    }
}

/// Does a memory-event element's `EventTags` carry any tag in `wanted`?
fn event_has_any_tag(element: &PropertyValue, wanted: &[&str]) -> bool {
    let tags = event_tags(element);
    wanted.iter().any(|w| tags.iter().any(|have| have == w))
}

/// A memory-event element's `AffectedCharacterGlobalId` (a NameProperty), or
/// `None` if absent. Used to scope cross-owner kill-memory removal to events
/// that are *about* the revived NPC.
fn affected_character_id(element: &PropertyValue) -> Option<&str> {
    match struct_member(element, "AffectedCharacterGlobalId") {
        Some(PropertyValue::Name(s)) => Some(s.as_str()),
        _ => None,
    }
}

/// Iterate every memory-owner entry of `LongTermMemoryByGlobalId` as
/// `(owner_id, &owner_memory_value)`.
fn memory_owners(root: &RootObject) -> Vec<(String, &PropertyValue)> {
    let Some((_path, map_prop)) = find_property_by_name(root, LONG_TERM_MEMORY_MAP) else {
        return Vec::new();
    };
    let PropertyValue::Map { entries, .. } = &map_prop.value else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|(k, v)| map_key_to_string(k).map(|key| (key, v)))
        .collect()
}

/// The next kill-tagged memory event to remove anywhere in the save, as
/// `(owner_id, event_index)`, or `None` when none remain. Scans every memory
/// owner; for each owner returns the FIRST matching event:
///
/// - The NPC's OWN entry (`owner_id == id`): match a [`REVIVE_EVENT_TAGS`] event
///   only when its `AffectedCharacterGlobalId` is absent or the NPC itself — its
///   own defeat/kill residue. An event whose affected character is someone ELSE
///   (e.g. this NPC executed another character) is NOT residue and must survive.
/// - ANY OTHER owner (Hero etc.): match only events that are kill-tagged
///   ([`KILL_EVENT_TAGS`] — the execution/kill tags) AND whose
///   `AffectedCharacterGlobalId == id`, so unrelated memories of that owner survive.
///
/// Re-scanning from scratch each call (rather than caching indices across the
/// splicing removal loop) is what keeps the loop correct as removals shift indices.
fn next_kill_memory_to_remove(root: &RootObject, id: &str) -> Option<(String, usize)> {
    for (owner_id, memory) in memory_owners(root) {
        let Some(PropertyValue::Array { elements }) = struct_member(memory, "MemorizedEvents")
        else {
            continue;
        };
        let is_own = owner_id == id;
        let found = elements.iter().position(|element| {
            if is_own {
                // Own defeat/kill residue only — an event about a DIFFERENT
                // character (this NPC killed/executed someone else) is real
                // memory and must be preserved across revive.
                event_has_any_tag(element, REVIVE_EVENT_TAGS)
                    && affected_character_id(element).map_or(true, |a| a == id)
            } else {
                event_has_any_tag(element, KILL_EVENT_TAGS)
                    && affected_character_id(element) == Some(id)
            }
        });
        if let Some(index) = found {
            return Some((owner_id, index));
        }
    }
    None
}

/// The typed path (from the private root) to NPC `id`'s `MemorizedEvents` array:
/// `<LongTermMemoryByGlobalId prefix> / {id} / MemorizedEvents`. Returns `None` if
/// the map / `id` is absent.
fn memorized_events_array_path(root: &RootObject, id: &str) -> Option<Vec<String>> {
    let (base_path, map_prop) = find_property_by_name(root, LONG_TERM_MEMORY_MAP)?;
    let PropertyValue::Map { entries, .. } = &map_prop.value else {
        return None;
    };
    lookup_entry(entries, id)?;
    let mut path = base_path;
    path.push(map_key_segment(id));
    path.push("MemorizedEvents".to_string());
    Some(path)
}

/// Does a `m_SavedInventories` key name NPC `id`'s lootable corpse? A corpse key is
/// `"Character_" + id`, but the real save sometimes appends a numeric spawn suffix
/// (`"Character_" + id + "_" + <digits>`, e.g.
/// `Character_OM_GRD_Drake_260-WorldPointActor_Drake_2146328221`). Match the exact
/// form OR the `_<digits>`-suffixed form so a suffixed corpse is still removed.
fn is_corpse_key_for(key: &str, id: &str) -> bool {
    let Some(rest) = key.strip_prefix("Character_") else {
        return false;
    };
    if rest == id {
        return true;
    }
    rest.strip_prefix(id)
        .and_then(|s| s.strip_prefix('_'))
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// Remove NPC `id`'s lootable-corpse entry from the global `m_SavedInventories`
/// map (keyed `"Character_" + id`, possibly with a numeric spawn suffix — see
/// [`is_corpse_key_for`]), in place. A killed NPC has such an entry; an alive one
/// does not. No-op (Ok) if the map or the entry is absent. Removes EVERY matching
/// corpse (re-parse/splice loop) in case more than one exists.
///
/// Each removal is a single map-entry splice ([`ContainerEdit::MapRemove`]): the
/// (key+value) byte range is dropped, the map count decremented, and the enclosing
/// size cascade fixed up by [`patch_container`]. `find_property_by_name` gives the
/// path to the `m_SavedInventories` MapProperty; [`resolve_chain`] re-resolves it
/// for the enclosing size fields. The payload is re-parsed before each splice so
/// offsets and indices stay fresh.
fn remove_corpse_inventory(payload: &mut Vec<u8>, id: &str) -> Result<(), CoreError> {
    loop {
        let root = parse_private_root(payload)?;
        let Some((path, map_prop)) = find_property_by_name(&root, SAVED_INVENTORIES_MAP) else {
            return Ok(()); // no corpse map => nothing to remove
        };
        let PropertyValue::Map { entries, .. } = &map_prop.value else {
            return Ok(());
        };
        let Some(entry_index) = entries.iter().position(|(k, _v)| {
            map_key_to_string(k).as_deref().is_some_and(|k| is_corpse_key_for(k, id))
        }) else {
            return Ok(()); // no corpse for this NPC (e.g. an alive NPC) => done
        };

        let segs = parse_path(&path)?;
        let chain = resolve_chain(&root.properties, &segs)?;
        let target = chain.target.clone();
        let enclosing = chain.enclosing_size_fields.clone();
        patch_container(
            payload,
            &target,
            &enclosing,
            &ContainerEdit::MapRemove { entry_index },
        )?;
    }
}

/// Strip the death loose tags ([`DEAD_LOOSE_TAGS`]: `State.Dead`,
/// `State.KillBountyGranted`, `State.ExecutedBountyGranted`) from NPC `id`'s
/// `LooseTagsByGlobalId[<id>]` GameplayTagContainer, in place. This is the CRITICAL
/// revive step: `State.Dead` here is the authoritative on-load death gate the game
/// reads (AngelScript re-applies it), so without removing it a revived NPC reverts
/// to a corpse on load. Unrelated tags in the container survive.
///
/// Each tag removal is a length-changing splice on a native tag container that is a
/// MAP VALUE (no per-value size header), so it goes through
/// [`patch_map_value_tag_container`] (not [`patch_tag_container`], which would clobber
/// the key). The payload is re-parsed before each removal so offsets stay fresh.
/// No-op (Ok) if the map / entry is absent or carries none of the tags.
fn remove_dead_loose_tags(payload: &mut Vec<u8>, id: &str) -> Result<(), CoreError> {
    for tag in DEAD_LOOSE_TAGS {
        loop {
            let root = parse_private_root(payload)?;
            let Some((path, map_prop)) = find_property_by_name(&root, LOOSE_TAGS_MAP) else {
                return Ok(()); // no loose-tags map => nothing to strip
            };
            let PropertyValue::Map { entries, .. } = &map_prop.value else {
                return Ok(());
            };
            let Some(entry_index) = entries
                .iter()
                .position(|(k, _v)| map_key_to_string(k).as_deref() == Some(id))
            else {
                break; // no entry for this NPC => move to next tag (will also break)
            };
            // Enclosing size fields for the MAP property (ancestors); the map's own
            // size field is handled inside patch_map_value_tag_container.
            let segs = parse_path(&path)?;
            let chain = resolve_chain(&root.properties, &segs)?;
            let target = chain.target.clone();
            let enclosing = chain.enclosing_size_fields.clone();
            let removed = patch_map_value_tag_container(
                payload,
                &target,
                &enclosing,
                entry_index,
                tag,
            )?;
            if !removed {
                break; // this tag is gone => next tag
            }
            // Re-parse and re-scan in case the same tag appears more than once
            // (defensive; a container normally holds each tag at most once).
        }
    }
    Ok(())
}

/// Restore NPC `id`'s `Health` to its `MaxHealth` `BaseValue` (both `BaseValue` and
/// `CurrentValue`), in place. No-op if HP is already at max (avoids a needless
/// patch). The two writes are fixed-size FloatProperty scalars (4 bytes each) in a
/// single parse, so the first patch does not shift the second's offset.
///
/// Errors if the NPC has no `Health` attribute or no `MaxHealth` `BaseValue` to
/// revive to.
fn restore_hp_to_max(payload: &mut [u8], id: &str) -> Result<(), CoreError> {
    let (base_path, current_path, max_hp, needs_patch) = {
        let root = parse_private_root(payload)?;
        let rows = npc_attributes(&root, id)?;
        let health = rows.iter().find(|r| r.key == "Health").ok_or_else(|| {
            CoreError::Parse(format!("NPC {id:?} has no Health attribute"))
        })?;
        let max_hp = rows
            .iter()
            .find(|r| r.key == "MaxHealth")
            .and_then(|r| r.base)
            .ok_or_else(|| {
                CoreError::Parse(format!("NPC {id:?} has no MaxHealth BaseValue to revive to"))
            })?;
        // Only write if HP is actually below max (defeated NPCs sit at 0; killed
        // NPCs may already be full — skip the patch then).
        let needs_patch = health.base != Some(max_hp) || health.current != Some(max_hp);
        (
            health.base_path.clone(),
            health.current_path.clone(),
            max_hp,
            needs_patch,
        )
    };

    if !needs_patch {
        return Ok(());
    }

    let root = parse_private_root(payload)?;
    let base_segs = parse_path(&base_path)?;
    let base_target = resolve_chain(&root.properties, &base_segs)?.target.clone();
    let cur_segs = parse_path(&current_path)?;
    let cur_target = resolve_chain(&root.properties, &cur_segs)?.target.clone();
    patch_scalar(payload, &base_target, ScalarValue::Float(max_hp))?;
    patch_scalar(payload, &cur_target, ScalarValue::Float(max_hp))?;
    Ok(())
}

/// REVIVE NPC `id` in a decoded private payload, in place — restore a KILLED NPC
/// to its alive state by undoing ALL THREE things killing it added.
///
/// Empirical 3-save diff (defeated vs killed) showed killing adds: (1) the NPC's
/// own kill memory residue, (2) the killer's (Hero's) kill memory event ABOUT this
/// NPC, and (3) a lootable-corpse entry in the global `m_SavedInventories` map. The
/// previous revive undid only (1); this undoes all three:
///
/// 1. **Cross-owner kill-memory removal.** Scan EVERY `LongTermMemoryByGlobalId`
///    owner. In the NPC's OWN entry remove a [`REVIVE_EVENT_TAGS`] event only when
///    its `AffectedCharacterGlobalId` is absent or the NPC itself (its own
///    defeat/kill residue) — a kill the NPC committed against ANOTHER character
///    survives; in every OTHER owner (Hero etc.) remove only events that are
///    kill-tagged ([`KILL_EVENT_TAGS`]) AND `AffectedCharacterGlobalId == id`
///    (unrelated memories of that owner survive). Each removal is a
///    length-changing array splice, so the payload is re-parsed and the NEXT event
///    re-located against the fresh tree every iteration — re-scanning from scratch
///    avoids the index-shift bug. Enclosing size cascade fixed up by [`patch_container`].
/// 2. **Loose-tag removal (the CRITICAL native fix).** Strip [`DEAD_LOOSE_TAGS`]
///    (`State.Dead`, `State.KillBountyGranted`, `State.ExecutedBountyGranted`) from
///    `LooseTagsByGlobalId[<id>]` ([`remove_dead_loose_tags`]). `State.Dead` here is
///    the AUTHORITATIVE on-load death gate AngelScript re-applies — leaving it makes
///    a revived NPC revert to a corpse on load. Unrelated loose tags survive.
/// 3. **Corpse removal.** Drop the `m_SavedInventories["Character_" + id]` map
///    entry ([`remove_corpse_inventory`]) — a separate single map-entry splice.
/// 4. **Restore HP** to `MaxHealth` (a defeated NPC sits at HP 0; a killed NPC may
///    keep positive HP, in which case [`restore_hp_to_max`] is a no-op).
///
/// Each step is its own re-parse/splice so byte offsets never go stale across them.
/// A clean no-op (Ok) if the NPC has no dead tags, no kill residue, no corpse, and
/// full HP.
pub fn apply_revive(payload: &mut Vec<u8>, id: &str) -> Result<(), CoreError> {
    // ── Phase 1: strip every kill/defeat memory event across ALL owners ──────
    // Re-parse/re-scan/splice loop: each removal shifts later indices and offsets,
    // so we locate the next matching event afresh every iteration. The scan returns
    // the owner it found so we splice the correct owner's MemorizedEvents array.
    loop {
        let root = parse_private_root(payload)?;
        let Some((owner_id, index)) = next_kill_memory_to_remove(&root, id) else {
            break;
        };
        let path = memorized_events_array_path(&root, &owner_id).ok_or_else(|| {
            CoreError::Parse(format!(
                "memory owner {owner_id:?} has no MemorizedEvents array to revive"
            ))
        })?;
        let segs = parse_path(&path)?;
        let chain = resolve_chain(&root.properties, &segs)?;
        let target = chain.target.clone();
        let enclosing = chain.enclosing_size_fields.clone();
        patch_container(payload, &target, &enclosing, &ContainerEdit::ArrayRemove(index))?;
    }

    // ── Phase 2: strip the authoritative death loose tags (the native gate) ──
    remove_dead_loose_tags(payload, id)?;

    // ── Phase 3: remove the lootable-corpse entry (its own re-parse/splice) ──
    remove_corpse_inventory(payload, id)?;

    // ── Phase 4: restore HP to max (no-op if already full) ───────────────────
    restore_hp_to_max(payload, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::properties::parse_private_root;

    fn load_root() -> RootObject {
        let path = std::env::var("GORESAVE_PAYLOAD_BIN")
            .expect("set GORESAVE_PAYLOAD_BIN to a decompressed payload dump");
        let payload = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        parse_private_root(&payload).unwrap_or_else(|e| panic!("{path}: {e}"))
    }

    /// True if any property named `name` is reachable anywhere inside `value`.
    /// Used to assert the resolved inventory container holds the same `m_Slots`
    /// shape the player traversal expects.
    fn subtree_contains_property(value: &PropertyValue, name: &str) -> bool {
        match value {
            PropertyValue::Struct(StructValue::Properties(props)) => props
                .iter()
                .any(|p| p.name == name || subtree_contains_property(&p.value, name)),
            PropertyValue::Struct(StructValue::Instanced(Some(i))) => i
                .properties
                .iter()
                .any(|p| p.name == name || subtree_contains_property(&p.value, name)),
            PropertyValue::ObjectInstances(objs) => objs.iter().any(|o| {
                o.properties
                    .iter()
                    .any(|p| p.name == name || subtree_contains_property(&p.value, name))
            }),
            PropertyValue::Array { elements } | PropertyValue::Set { elements, .. } => {
                elements.iter().any(|e| subtree_contains_property(e, name))
            }
            PropertyValue::Map { entries, .. } => entries
                .iter()
                .any(|(_k, v)| subtree_contains_property(v, name)),
            _ => false,
        }
    }

    #[test]
    #[ignore = "needs GORESAVE_PAYLOAD_BIN=<decompressed private payload>"]
    fn npc_inventory_path_resolves_for_a_real_npc() {
        let root = load_root();
        let id = "Lizard-WP_EF_SCSLOPE_LIZARD_SPAWN_01-1";

        let path = npc_inventory_path(&root, id)
            .unwrap_or_else(|| panic!("no inventory path for {id}"));
        // The path resolves against the private root...
        let segs = parse_path(&path).expect("inventory path parses");
        let target = resolve_chain(&root.properties, &segs)
            .expect("inventory path resolves")
            .target
            .clone();
        // ...and the resolved container holds the player-shaped m_Keys/m_Slots.
        assert!(
            subtree_contains_property(&target.value, "m_Keys"),
            "NPC inventory container must hold m_Keys (player-shaped inventory)"
        );
        assert!(
            subtree_contains_property(&target.value, "m_Slots"),
            "NPC inventory container must hold m_Slots (player-shaped inventory)"
        );
    }

    #[test]
    #[ignore = "needs GORESAVE_PAYLOAD_BIN=<decompressed private payload>"]
    fn lists_1484_npcs() {
        let root = load_root();
        let npcs = list_npcs(&root).unwrap();
        assert_eq!(npcs.len(), 1484, "expected 1484 NPCs");
    }

    // ── Synthetic GVAS builders ─────────────────────────────────────────────
    //
    // Minimal builders mirroring the real save shape just enough to exercise
    // list_npcs / apply_revive: a private root holding a
    // `CharacterStateSaveGameData_Attributes` map (Health/MaxHealth) keyed by NPC
    // id, and a `LongTermMemoryByGlobalId` map of MemorizedEvents carrying EventTags.

    use crate::properties::TAG_FLAG_NATIVE_SERIALIZE;

    fn fstring(value: &str) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&((value.len() + 1) as i32).to_le_bytes());
        out.extend_from_slice(value.as_bytes());
        out.push(0);
        out
    }

    fn float_property(name: &str, value: f32) -> Vec<u8> {
        let mut out = fstring(name);
        out.extend_from_slice(&fstring("FloatProperty"));
        out.extend_from_slice(&0u32.to_le_bytes()); // array_index
        out.extend_from_slice(&4u32.to_le_bytes()); // value_size
        out.push(0); // tag_flags
        out.extend_from_slice(&value.to_le_bytes());
        out
    }

    /// A `GameplayAttributeData`-style struct value (BaseValue + CurrentValue),
    /// terminated by the "None" property sentinel.
    fn gameplay_attribute_value(base: f32, current: f32) -> Vec<u8> {
        let mut v = float_property("BaseValue", base);
        v.extend_from_slice(&float_property("CurrentValue", current));
        v.extend_from_slice(&fstring("None"));
        v
    }

    /// One NPC `_Attributes` entry value: a struct proplist holding an inner
    /// `Attributes` MapProperty<Str, Struct(GameplayAttributeData)> with
    /// Health/MaxHealth keys, terminated by "None".
    fn attributes_entry_value(health: f32, max_health: f32) -> Vec<u8> {
        let mut map_body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        map_body.extend_from_slice(&2u32.to_le_bytes()); // count
        map_body.extend_from_slice(&fstring("Health"));
        map_body.extend_from_slice(&gameplay_attribute_value(health, health));
        map_body.extend_from_slice(&fstring("MaxHealth"));
        map_body.extend_from_slice(&gameplay_attribute_value(max_health, max_health));

        let mut attr_map = fstring("Attributes");
        attr_map.extend_from_slice(&fstring("MapProperty"));
        attr_map.extend_from_slice(&2u32.to_le_bytes());
        attr_map.extend_from_slice(&fstring("StrProperty")); // key type
        attr_map.extend_from_slice(&0u32.to_le_bytes());
        attr_map.extend_from_slice(&fstring("StructProperty")); // value type
        attr_map.extend_from_slice(&1u32.to_le_bytes());
        attr_map.extend_from_slice(&fstring("GameplayAttributeData"));
        attr_map.extend_from_slice(&1u32.to_le_bytes());
        attr_map.extend_from_slice(&fstring("/Script/G1R"));
        attr_map.extend_from_slice(&0u32.to_le_bytes()); // array_index
        attr_map.extend_from_slice(&(map_body.len() as u32).to_le_bytes());
        attr_map.push(0); // tag_flags
        attr_map.extend_from_slice(&map_body);

        attr_map.extend_from_slice(&fstring("None")); // end of entry proplist
        attr_map
    }

    /// A native `GameplayTagContainer` StructProperty value carrying `tags`.
    fn tag_container_property(name: &str, tags: &[&str]) -> Vec<u8> {
        let mut body = (tags.len() as u32).to_le_bytes().to_vec();
        for t in tags {
            body.extend_from_slice(&fstring(t));
        }
        let mut out = fstring(name);
        out.extend_from_slice(&fstring("StructProperty"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("GameplayTagContainer"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/GameplayTags"));
        out.extend_from_slice(&0u32.to_le_bytes()); // array_index
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.push(TAG_FLAG_NATIVE_SERIALIZE);
        out.extend_from_slice(&body);
        out
    }

    /// One `MemorizedEvents` element: a `MemoryEvent` struct proplist holding an
    /// `EventTags` GameplayTagContainer carrying `tags`, terminated by "None".
    fn memory_event(tags: &[&str]) -> Vec<u8> {
        let mut v = tag_container_property("EventTags", tags);
        v.extend_from_slice(&fstring("None"));
        v
    }

    /// A NameProperty named `name` carrying `value`.
    fn name_property(name: &str, value: &str) -> Vec<u8> {
        let body = fstring(value);
        let mut out = fstring(name);
        out.extend_from_slice(&fstring("NameProperty"));
        out.extend_from_slice(&0u32.to_le_bytes()); // array_index
        out.extend_from_slice(&(body.len() as u32).to_le_bytes()); // value_size
        out.push(0); // tag_flags
        out.extend_from_slice(&body);
        out
    }

    /// One `MemorizedEvents` element carrying `tags` AND an
    /// `AffectedCharacterGlobalId` NameProperty (= the cross-owner "about whom"
    /// field), terminated by "None". Used to build the Hero's kill-memory event.
    fn memory_event_affecting(tags: &[&str], affected: &str) -> Vec<u8> {
        let mut v = tag_container_property("EventTags", tags);
        v.extend_from_slice(&name_property("AffectedCharacterGlobalId", affected));
        v.extend_from_slice(&fstring("None"));
        v
    }

    /// One NPC long-term-memory entry value: a struct proplist holding a
    /// `MemorizedEvents` ArrayProperty<Struct(MemoryEvent)> of `events`, terminated
    /// by "None". Each event is one `memory_event` blob.
    fn long_term_memory_entry_value(events: &[Vec<u8>]) -> Vec<u8> {
        let mut body = (events.len() as u32).to_le_bytes().to_vec();
        for e in events {
            body.extend_from_slice(e);
        }

        let mut out = fstring("MemorizedEvents");
        out.extend_from_slice(&fstring("ArrayProperty"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("StructProperty"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("MemoryEvent"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/G1R"));
        out.extend_from_slice(&0u32.to_le_bytes()); // array_index
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.push(0); // tag_flags
        out.extend_from_slice(&body);

        out.extend_from_slice(&fstring("None")); // end of entry proplist
        out
    }

    /// Encode the `LongTermMemoryByGlobalId` MapProperty<Str, Struct(...)> with
    /// `entries` as `(id, entry_value_bytes)`. The entry-value bytes are NOT
    /// terminated with "None" by this helper (the entry value already is).
    fn long_term_memory_map(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
        character_state_map(
            LONG_TERM_MEMORY_MAP,
            "CharacterStateSaveGameData_LongTermMemory",
            entries,
        )
    }

    /// Encode the global `m_SavedInventories` MapProperty<Str, Struct(...)> with
    /// `keys` (the corpse keys, e.g. `"Character_<id>"`). Each entry's value is a
    /// minimal struct proplist (just the "None" sentinel) — enough to parse; the
    /// real value type is `ReplicatedInventoryMap`, irrelevant to entry removal.
    fn saved_inventories_map(keys: &[&str]) -> Vec<u8> {
        let entries: Vec<(&str, Vec<u8>)> =
            keys.iter().map(|k| (*k, fstring("None"))).collect();
        character_state_map(SAVED_INVENTORIES_MAP, "ReplicatedInventoryMap", &entries)
    }

    /// Encode the `LooseTagsByGlobalId` MapProperty<Str, Struct(GameplayTagContainer)>
    /// with `entries` as `(id, &[tag])`. Each value is an INLINE native
    /// GameplayTagContainer (`u32 count` + count FStrings) — no per-value size header,
    /// exactly as the real save serializes a struct-typed map value.
    fn loose_tags_map(entries: &[(&str, &[&str])]) -> Vec<u8> {
        let mut map_body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        map_body.extend_from_slice(&(entries.len() as u32).to_le_bytes()); // count
        for (id, tags) in entries {
            map_body.extend_from_slice(&fstring(id));
            // inline native GameplayTagContainer value
            map_body.extend_from_slice(&(tags.len() as u32).to_le_bytes());
            for t in *tags {
                map_body.extend_from_slice(&fstring(t));
            }
        }

        let mut prop = fstring(LOOSE_TAGS_MAP);
        prop.extend_from_slice(&fstring("MapProperty"));
        prop.extend_from_slice(&2u32.to_le_bytes());
        prop.extend_from_slice(&fstring("StrProperty")); // key type
        prop.extend_from_slice(&0u32.to_le_bytes());
        prop.extend_from_slice(&fstring("StructProperty")); // value type
        prop.extend_from_slice(&1u32.to_le_bytes());
        prop.extend_from_slice(&fstring("GameplayTagContainer")); // value struct type
        prop.extend_from_slice(&1u32.to_le_bytes());
        prop.extend_from_slice(&fstring("/Script/GameplayTags"));
        prop.extend_from_slice(&0u32.to_le_bytes()); // array_index
        prop.extend_from_slice(&(map_body.len() as u32).to_le_bytes());
        prop.push(0); // tag_flags
        prop.extend_from_slice(&map_body);
        prop
    }

    /// Encode a character-state MapProperty<Str, Struct(struct_type)> named
    /// `map_name`, with `entries` as `(id, entry_value_bytes)`.
    fn character_state_map(
        map_name: &str,
        struct_type: &str,
        entries: &[(&str, Vec<u8>)],
    ) -> Vec<u8> {
        let mut map_body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        map_body.extend_from_slice(&(entries.len() as u32).to_le_bytes()); // count
        for (id, value) in entries {
            map_body.extend_from_slice(&fstring(id));
            map_body.extend_from_slice(value);
        }

        let mut prop = fstring(map_name);
        prop.extend_from_slice(&fstring("MapProperty"));
        prop.extend_from_slice(&2u32.to_le_bytes());
        prop.extend_from_slice(&fstring("StrProperty")); // key type
        prop.extend_from_slice(&0u32.to_le_bytes());
        prop.extend_from_slice(&fstring("StructProperty")); // value type
        prop.extend_from_slice(&1u32.to_le_bytes());
        prop.extend_from_slice(&fstring(struct_type));
        prop.extend_from_slice(&1u32.to_le_bytes());
        prop.extend_from_slice(&fstring("/Script/G1R"));
        prop.extend_from_slice(&0u32.to_le_bytes()); // array_index
        prop.extend_from_slice(&(map_body.len() as u32).to_le_bytes());
        prop.push(0); // tag_flags
        prop.extend_from_slice(&map_body);
        prop
    }

    /// Wrap one or more top-level property blobs in a private root, terminated by
    /// the "None" sentinel + footer. A trailing `m_After` int catches a missed
    /// size fixup (any splice that miscomputes a size would shift it).
    fn private_root(props: &[Vec<u8>]) -> Vec<u8> {
        let mut payload = fstring("/Script/Test.Save");
        payload.push(0); // flag
        for p in props {
            payload.extend_from_slice(p);
        }
        // Trailing sentinel int: proof the size cascade stays consistent.
        payload.extend_from_slice(&{
            let mut out = fstring("m_After");
            out.extend_from_slice(&fstring("IntProperty"));
            out.extend_from_slice(&0u32.to_le_bytes()); // array_index
            out.extend_from_slice(&4u32.to_le_bytes()); // value_size
            out.push(0); // tag_flags
            out.extend_from_slice(&9i32.to_le_bytes());
            out
        });
        payload.extend_from_slice(&fstring("None"));
        payload.extend_from_slice(&0u32.to_le_bytes()); // footer
        payload
    }

    /// Like `attributes_entry_value` but with a distinct Health `CurrentValue`
    /// (a wounded NPC: base/full HP differs from current HP).
    fn wounded_attributes_entry_value(
        health_base: f32,
        health_current: f32,
        max_health: f32,
    ) -> Vec<u8> {
        let mut map_body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        map_body.extend_from_slice(&2u32.to_le_bytes()); // count
        map_body.extend_from_slice(&fstring("Health"));
        map_body.extend_from_slice(&gameplay_attribute_value(health_base, health_current));
        map_body.extend_from_slice(&fstring("MaxHealth"));
        map_body.extend_from_slice(&gameplay_attribute_value(max_health, max_health));

        let mut attr_map = fstring("Attributes");
        attr_map.extend_from_slice(&fstring("MapProperty"));
        attr_map.extend_from_slice(&2u32.to_le_bytes());
        attr_map.extend_from_slice(&fstring("StrProperty")); // key type
        attr_map.extend_from_slice(&0u32.to_le_bytes());
        attr_map.extend_from_slice(&fstring("StructProperty")); // value type
        attr_map.extend_from_slice(&1u32.to_le_bytes());
        attr_map.extend_from_slice(&fstring("GameplayAttributeData"));
        attr_map.extend_from_slice(&1u32.to_le_bytes());
        attr_map.extend_from_slice(&fstring("/Script/G1R"));
        attr_map.extend_from_slice(&0u32.to_le_bytes()); // array_index
        attr_map.extend_from_slice(&(map_body.len() as u32).to_le_bytes());
        attr_map.push(0); // tag_flags
        attr_map.extend_from_slice(&map_body);
        attr_map.extend_from_slice(&fstring("None")); // end of entry proplist
        attr_map
    }


    #[test]
    fn char_key_strips_after_first_dash_and_lowercases() {
        assert_eq!(char_key("NC_ORG_Lares_801-WP_OC_SPAWN"), "nc_org_lares_801");
        assert_eq!(char_key("Hero"), "hero");
        assert_eq!(char_key("A-B-C"), "a");
    }

    #[test]
    fn list_npcs_reports_current_health_for_wounded_npc() {
        // Wounded but alive: Health base 80, current 25, MaxHealth 80, no defeat
        // memory => not defeated.
        let id = "OC_STT_Lizard-1";
        let payload = private_root(&[character_state_map(
            "AttributesMap",
            ATTRIBUTES_TYPE,
            &[(id, wounded_attributes_entry_value(80.0, 25.0, 80.0))],
        )]);
        let npcs = list_npcs(&parse_private_root(&payload).unwrap()).unwrap();
        let n = npcs.iter().find(|n| n.id == id).unwrap();
        assert_eq!(n.hp, Some(25.0), "hp must be CurrentValue, not BaseValue");
        assert_eq!(n.max_hp, Some(80.0));
        assert!(!n.is_dead, "no kill memory event => not dead");
    }

    #[test]
    fn list_npcs_reports_dead_only_from_state_dead_loose_tag() {
        // Death is decided by the AUTHORITATIVE native gate: `State.Dead` in the NPC's
        // `LooseTagsByGlobalId` GameplayTagContainer. A *killed* NPC keeps positive HP
        // but carries State.Dead => dead. A merely *defeated* (knocked-out) NPC at HP 0
        // carries no State.Dead (only unrelated loose tags) => ALIVE. HP 0 with no
        // loose-tags entry is also alive. Kill MEMORY alone (no State.Dead) is NOT dead.
        let killed = "OC_STT_Killed-1";
        let defeated_only = "OC_STT_Defeated-1";
        let zero_hp_no_tags = "OC_STT_ZeroHp-1";
        let kill_memory_no_dead_tag = "OC_STT_MemOnly-1";
        let payload = private_root(&[
            character_state_map(
                "AttributesMap",
                ATTRIBUTES_TYPE,
                &[
                    (killed, attributes_entry_value(60.0, 60.0)),
                    (defeated_only, attributes_entry_value(0.0, 60.0)),
                    (zero_hp_no_tags, attributes_entry_value(0.0, 60.0)),
                    (kill_memory_no_dead_tag, attributes_entry_value(60.0, 60.0)),
                ],
            ),
            loose_tags_map(&[
                // Killed: the full dead-tag set (positive HP is irrelevant).
                (
                    killed,
                    &["State.KillBountyGranted", "State.ExecutedBountyGranted", "State.Dead"],
                ),
                // Defeated-only: carries an unrelated combat tag, but NOT State.Dead.
                (defeated_only, &["State.Aggro", "State.InCombat"]),
            ]),
            // A kill memory event with NO State.Dead loose tag must NOT read as dead.
            long_term_memory_map(&[(
                kill_memory_no_dead_tag,
                long_term_memory_entry_value(&[memory_event(&["Memory.Character.Defeated.Kill"])]),
            )]),
        ]);
        let root = parse_private_root(&payload).unwrap();
        let npcs = list_npcs(&root).unwrap();
        let k = npcs.iter().find(|n| n.id == killed).unwrap();
        assert!(k.is_dead, "State.Dead in LooseTags => dead even with positive HP");
        assert_eq!(k.hp, Some(60.0));
        let d = npcs.iter().find(|n| n.id == defeated_only).unwrap();
        assert!(!d.is_dead, "loose tags without State.Dead => ALIVE (merely defeated)");
        let z = npcs.iter().find(|n| n.id == zero_hp_no_tags).unwrap();
        assert!(!z.is_dead, "HP 0 with no loose-tags entry is alive (HP is not the signal)");
        let m = npcs.iter().find(|n| n.id == kill_memory_no_dead_tag).unwrap();
        assert!(!m.is_dead, "kill memory alone (no State.Dead loose tag) is NOT dead");
        // The standalone detector agrees with list_npcs.
        assert!(is_dead_by_loose_tags(&root, killed));
        assert!(!is_dead_by_loose_tags(&root, defeated_only));
    }

    #[test]
    fn revive_removes_defeat_events_restores_hp_and_reparses_clean() {
        let id = "OC_STT_Lizard-1";
        // Killed NPC: HP 0, MaxHealth 80; the authoritative dead loose tags (plus an
        // unrelated loose tag that must survive); three memory events — two defeat
        // tags plus one unrelated event that must survive.
        let mut payload = private_root(&[
            character_state_map(
                "AttributesMap",
                ATTRIBUTES_TYPE,
                &[(id, attributes_entry_value(0.0, 80.0))],
            ),
            loose_tags_map(&[(
                id,
                &[
                    "State.KillBountyGranted",
                    "State.Aggro", // unrelated, must survive
                    "State.ExecutedBountyGranted",
                    "State.Dead",
                ],
            )]),
            long_term_memory_map(&[(
                id,
                long_term_memory_entry_value(&[
                    memory_event(&["Memory.Combat.WasDefeated"]),
                    memory_event(&["Memory.Quest.Started"]),
                    memory_event(&["Memory.SaveAndLoad.Defeated", "Memory.Execution"]),
                ]),
            )]),
        ]);

        let before = parse_private_root(&payload).unwrap();
        let n0 = list_npcs(&before).unwrap().into_iter().find(|n| n.id == id).unwrap();
        assert!(n0.is_dead, "NPC starts dead (State.Dead in LooseTags)");
        assert_eq!(n0.hp, Some(0.0));

        apply_revive(&mut payload, id).unwrap();

        let root = parse_private_root(&payload).unwrap();
        assert_eq!(root.consumed, payload.len(), "payload must re-parse fully");
        let n = list_npcs(&root).unwrap().into_iter().find(|n| n.id == id).unwrap();
        assert!(!n.is_dead, "revived NPC is no longer dead");
        assert_eq!(n.hp, Some(80.0), "HP restored to MaxHealth base");
        assert_eq!(n.max_hp, Some(80.0));
        // The three dead loose tags are gone; the unrelated loose tag survives.
        let tags = loose_tags(&root, id).expect("loose-tags entry still present");
        for dead in DEAD_LOOSE_TAGS {
            assert!(!tags.iter().any(|t| t == dead), "dead loose tag {dead} must be gone");
        }
        assert!(
            tags.iter().any(|t| t == "State.Aggro"),
            "unrelated loose tag must survive"
        );
        // Every revive memory tag is gone; the unrelated quest event survives.
        for tag in REVIVE_EVENT_TAGS {
            assert!(!memory_has_tag(&root, id, tag), "revive tag {tag} must be gone");
        }
        assert!(
            memory_has_tag(&root, id, "Memory.Quest.Started"),
            "unrelated memory event must survive"
        );
    }

    /// Whether the global `m_SavedInventories` map holds an entry keyed `key`.
    fn has_saved_inventory_entry(root: &RootObject, key: &str) -> bool {
        let Some((_p, prop)) = find_property_by_name(root, SAVED_INVENTORIES_MAP) else {
            return false;
        };
        let PropertyValue::Map { entries, .. } = &prop.value else {
            return false;
        };
        entries
            .iter()
            .any(|(k, _v)| map_key_to_string(k).as_deref() == Some(key))
    }

    /// Whether `m_SavedInventories` holds a corpse for NPC `id` (exact or numeric
    /// spawn-suffixed key — see [`is_corpse_key_for`]).
    fn has_corpse_for(root: &RootObject, id: &str) -> bool {
        let Some((_p, prop)) = find_property_by_name(root, SAVED_INVENTORIES_MAP) else {
            return false;
        };
        let PropertyValue::Map { entries, .. } = &prop.value else {
            return false;
        };
        entries
            .iter()
            .any(|(k, _v)| map_key_to_string(k).as_deref().is_some_and(|k| is_corpse_key_for(k, id)))
    }

    #[test]
    fn revive_removes_corpse_and_cross_owner_kill_memory_but_keeps_unrelated() {
        // Build the full "killed NPC" shape: (a) the NPC's OWN kill memory residue,
        // (b) a Hero owner with one kill event about the NPC + one unrelated event,
        // (c) m_SavedInventories with the NPC's corpse + an unrelated corpse.
        let id = "OM_GRD_Guard11_273-WorldPointActor_Guard11_273";
        let hero = "Hero";
        let other = "OM_GRD_OtherGuard-9";
        let corpse_key = format!("Character_{id}");
        let other_corpse_key = format!("Character_{other}");

        let mut payload = private_root(&[
            character_state_map(
                "AttributesMap",
                ATTRIBUTES_TYPE,
                &[(id, wounded_attributes_entry_value(80.0, 12.0, 80.0))],
            ),
            // The authoritative dead loose tags (+ an unrelated one) and an unrelated
            // alive NPC's loose tags that must be untouched.
            loose_tags_map(&[
                (
                    id,
                    &["State.KillBountyGranted", "State.ExecutedBountyGranted", "State.Dead", "State.Aggro"],
                ),
                (other, &["State.Aggro"]),
            ]),
            long_term_memory_map(&[
                // (a) NPC's own kill residue + one unrelated event of its own +
                //     a kill the NPC itself committed against ANOTHER character
                //     (must survive: it is real memory, not death residue).
                (
                    id,
                    long_term_memory_entry_value(&[
                        memory_event(&["Memory.Character.Defeated.Kill"]),
                        memory_event(&["Memory.Quest.Started"]),
                        memory_event_affecting(&["Memory.Execution"], other),
                    ]),
                ),
                // (b) Hero: one kill event ABOUT the NPC (must go) + one unrelated
                //     Hero memory about the NPC that is NOT kill-tagged (must stay)
                //     + one kill event about SOMEONE ELSE (must stay).
                (
                    hero,
                    long_term_memory_entry_value(&[
                        memory_event_affecting(&["Memory.Character.Defeated.Kill"], id),
                        memory_event_affecting(&["Memory.Combat.SawCharacter"], id),
                        memory_event_affecting(&["Memory.Execution"], other),
                    ]),
                ),
            ]),
            // (c) corpse map: the NPC's corpse + an unrelated corpse (must survive).
            saved_inventories_map(&[corpse_key.as_str(), other_corpse_key.as_str()]),
        ]);

        let before = parse_private_root(&payload).unwrap();
        assert!(
            has_saved_inventory_entry(&before, &corpse_key),
            "fixture must start with the NPC's corpse"
        );
        assert!(
            is_dead_by_loose_tags(&before, id),
            "fixture NPC must start dead (State.Dead in LooseTags)"
        );

        apply_revive(&mut payload, id).unwrap();

        let root = parse_private_root(&payload).unwrap();
        assert_eq!(root.consumed, payload.len(), "payload must re-parse fully (byte-clean)");

        // 1. NPC's own death residue is gone, but its unrelated memory — and a
        //    kill it committed against ANOTHER character — survive.
        let npc_mem = long_term_memory_value(&root, id).unwrap();
        let PropertyValue::Array { elements: npc_events } =
            struct_member(npc_mem, "MemorizedEvents").unwrap()
        else {
            panic!("NPC MemorizedEvents not an array");
        };
        for tag in REVIVE_EVENT_TAGS {
            let residue = npc_events.iter().any(|e| {
                event_has_any_tag(e, &[tag])
                    && affected_character_id(e).map_or(true, |a| a == id)
            });
            assert!(!residue, "NPC own death-residue tag {tag} must be gone");
        }
        assert!(
            npc_events
                .iter()
                .any(|e| event_has_any_tag(e, &["Memory.Quest.Started"])),
            "NPC's unrelated own memory must survive"
        );
        assert!(
            npc_events.iter().any(|e| event_has_any_tag(e, &["Memory.Execution"])
                && affected_character_id(e) == Some(other)),
            "NPC's memory of executing a DIFFERENT character must survive"
        );

        // 2. Hero's kill-about-id event gone; unrelated Hero memories survive.
        let hero_mem = long_term_memory_value(&root, hero).unwrap();
        let PropertyValue::Array { elements } =
            struct_member(hero_mem, "MemorizedEvents").unwrap()
        else {
            panic!("hero MemorizedEvents not an array");
        };
        assert_eq!(elements.len(), 2, "exactly the one kill-about-id event was removed");
        // No kill-tagged event affecting `id` remains in ANY owner.
        for tag in KILL_EVENT_TAGS {
            let kill_about_id = memory_owners(&root).iter().any(|(_owner, mem)| {
                let Some(PropertyValue::Array { elements }) =
                    struct_member(mem, "MemorizedEvents")
                else {
                    return false;
                };
                elements.iter().any(|e| {
                    event_has_any_tag(e, &[tag]) && affected_character_id(e) == Some(id)
                })
            });
            assert!(!kill_about_id, "no {tag} event affecting {id} may remain in any owner");
        }
        // The Hero's non-kill memory of the NPC and the kill of someone else survive.
        assert!(
            elements
                .iter()
                .any(|e| event_has_any_tag(e, &["Memory.Combat.SawCharacter"])
                    && affected_character_id(e) == Some(id)),
            "Hero's unrelated (non-kill) memory of the NPC must survive"
        );
        assert!(
            elements
                .iter()
                .any(|e| event_has_any_tag(e, &["Memory.Execution"])
                    && affected_character_id(e) == Some(other)),
            "Hero's kill memory of a DIFFERENT NPC must survive"
        );

        // 3. The corpse is gone; the unrelated corpse survives.
        assert!(
            !has_saved_inventory_entry(&root, &corpse_key),
            "revived NPC's corpse (m_SavedInventories entry) must be removed"
        );
        assert!(
            has_saved_inventory_entry(&root, &other_corpse_key),
            "an unrelated corpse must survive"
        );

        // 4. The dead loose tags are gone; the NPC's unrelated loose tag survives and
        //    the unrelated alive NPC's loose tags are untouched.
        let revived_tags = loose_tags(&root, id).expect("loose-tags entry present");
        for dead in DEAD_LOOSE_TAGS {
            assert!(
                !revived_tags.iter().any(|t| t == dead),
                "dead loose tag {dead} must be stripped"
            );
        }
        assert!(
            revived_tags.iter().any(|t| t == "State.Aggro"),
            "NPC's unrelated loose tag must survive"
        );
        assert_eq!(
            loose_tags(&root, other),
            Some(["State.Aggro".to_string()].as_slice()),
            "an unrelated NPC's loose tags must be untouched"
        );
        assert!(!is_dead_by_loose_tags(&root, id), "no State.Dead => not dead");

        // HP restored to max.
        let n = list_npcs(&root).unwrap().into_iter().find(|n| n.id == id).unwrap();
        assert!(!n.is_dead, "revived NPC no longer dead");
        assert_eq!(n.hp, Some(80.0), "HP restored to MaxHealth base");

        // Idempotent: a second revive changes nothing.
        let snapshot = payload.clone();
        apply_revive(&mut payload, id).unwrap();
        assert_eq!(payload, snapshot, "second revive is a byte-identical no-op");
    }

    #[test]
    fn revive_strips_state_dead_when_only_in_loose_tags() {
        // The authoritative gate stands alone: an NPC with State.Dead ONLY in LooseTags
        // (no kill memory, no corpse) is detected dead and revived clean & idempotent.
        let id = "OM_GRD_Drake-1";
        let mut payload = private_root(&[
            character_state_map(
                "AttributesMap",
                ATTRIBUTES_TYPE,
                &[(id, wounded_attributes_entry_value(100.0, 0.0, 100.0))],
            ),
            loose_tags_map(&[(
                id,
                &["State.KillBountyGranted", "State.ExecutedBountyGranted", "State.Dead"],
            )]),
        ]);

        let before = parse_private_root(&payload).unwrap();
        assert!(is_dead_by_loose_tags(&before, id), "State.Dead loose tag => dead");
        let n0 = list_npcs(&before).unwrap().into_iter().find(|n| n.id == id).unwrap();
        assert!(n0.is_dead);

        apply_revive(&mut payload, id).unwrap();

        let root = parse_private_root(&payload).unwrap();
        assert_eq!(root.consumed, payload.len(), "byte-clean re-parse");
        // All three dead tags gone; the (now-empty) container entry remains parseable.
        let tags = loose_tags(&root, id).expect("loose-tags entry present");
        assert!(tags.is_empty(), "all dead tags stripped; container now empty");
        assert!(!is_dead_by_loose_tags(&root, id));
        let n = list_npcs(&root).unwrap().into_iter().find(|n| n.id == id).unwrap();
        assert!(!n.is_dead, "revived NPC no longer dead");
        assert_eq!(n.hp, Some(100.0), "HP restored to MaxHealth base");

        // Idempotent.
        let snapshot = payload.clone();
        apply_revive(&mut payload, id).unwrap();
        assert_eq!(payload, snapshot, "second revive is a byte-identical no-op");
    }

    #[test]
    fn revive_is_clean_noop_when_nothing_to_revive() {
        let id = "OC_STT_Lizard-1";
        // Full HP, only an unrelated memory event: revive must change nothing.
        let mut payload = private_root(&[
            character_state_map(
                "AttributesMap",
                ATTRIBUTES_TYPE,
                &[(id, attributes_entry_value(80.0, 80.0))],
            ),
            long_term_memory_map(&[(
                id,
                long_term_memory_entry_value(&[memory_event(&["Memory.Quest.Started"])]),
            )]),
        ]);
        let snapshot = payload.clone();

        apply_revive(&mut payload, id).unwrap();
        assert_eq!(payload, snapshot, "no defeat events + full HP => byte-identical");
    }

    #[test]
    fn revive_restores_hp_for_killed_npc_with_positive_hp() {
        let id = "OC_STT_Lizard-1";
        // Killed NPC: positive but reduced HP (current 10, max 80) + a kill event.
        let mut payload = private_root(&[
            character_state_map(
                "AttributesMap",
                ATTRIBUTES_TYPE,
                &[(id, wounded_attributes_entry_value(80.0, 10.0, 80.0))],
            ),
            long_term_memory_map(&[(
                id,
                long_term_memory_entry_value(&[memory_event(&["Memory.Character.Defeated.Kill"])]),
            )]),
        ]);

        apply_revive(&mut payload, id).unwrap();

        let root = parse_private_root(&payload).unwrap();
        assert_eq!(root.consumed, payload.len(), "payload must re-parse fully");
        let n = list_npcs(&root).unwrap().into_iter().find(|n| n.id == id).unwrap();
        assert!(!n.is_dead);
        assert_eq!(n.hp, Some(80.0), "HP restored to max even when it was positive");
    }

    #[test]
    fn corpse_key_matches_exact_and_numeric_suffix() {
        let id = "OM_GRD_Drake_260-WorldPointActor_Drake";
        assert!(is_corpse_key_for(&format!("Character_{id}"), id), "exact");
        assert!(
            is_corpse_key_for(&format!("Character_{id}_2146328221"), id),
            "numeric spawn suffix"
        );
        assert!(!is_corpse_key_for(&format!("Character_{id}_abc"), id), "non-digit suffix");
        assert!(!is_corpse_key_for(&format!("Character_{id}X"), id), "no underscore");
        assert!(!is_corpse_key_for(&format!("Character_{id}_"), id), "empty suffix");
        assert!(!is_corpse_key_for("Character_OtherNpc-1", id), "different id");
        assert!(!is_corpse_key_for(id, id), "missing Character_ prefix");
    }

    #[test]
    #[ignore = "needs GORESAVE_G1R014_FRESH=<C:\\sbx\\goresave\\work\\fresh\\G1R-014.decompressed.bin>"]
    fn revive_killed_npc_real_fixture() {
        // Real killed-NPC fixture: the fresh decompressed G1R-014 blob (full private
        // payload; parse_private_root handles it). NPC OM_GRD_Drake_260 starts dead
        // (State.Dead in LooseTags). Asserts the authoritative gate flips: the three
        // dead loose tags are stripped, the corpse is gone, no kill memory affects
        // Drake, is_dead flips, HP==max, and the payload re-parses byte-clean.
        let path = std::env::var("GORESAVE_G1R014_FRESH")
            .expect("set GORESAVE_G1R014_FRESH to the fresh G1R-014 decompressed blob");
        let mut payload = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let id = "OM_GRD_Drake_260-WorldPointActor_Drake";

        let before = parse_private_root(&payload).unwrap();
        let n0 = list_npcs(&before)
            .unwrap()
            .into_iter()
            .find(|n| n.id == id)
            .unwrap_or_else(|| panic!("{id} not found"));
        assert!(n0.is_dead, "fixture NPC (killed Drake) should start dead");
        assert!(
            is_dead_by_loose_tags(&before, id),
            "Drake must start with State.Dead in LooseTags"
        );
        let before_tags = loose_tags(&before, id).expect("Drake loose-tags entry present");
        assert!(
            before_tags.iter().any(|t| t == "State.Dead"),
            "fixture must start with State.Dead in LooseTags"
        );
        assert!(
            has_corpse_for(&before, id),
            "killed Drake must start with a corpse (m_SavedInventories entry)"
        );

        apply_revive(&mut payload, id).unwrap();

        let root = parse_private_root(&payload).unwrap();
        assert_eq!(root.consumed, payload.len(), "payload must re-parse fully");
        let n = list_npcs(&root).unwrap().into_iter().find(|n| n.id == id).unwrap();
        assert!(!n.is_dead, "revived NPC should no longer be dead");
        assert_eq!(n.hp, n.max_hp, "HP restored to MaxHealth base");
        // State.Dead / KillBounty / Executed gone from LooseTagsByGlobalId[Drake].
        let after_tags = loose_tags(&root, id).expect("Drake loose-tags entry still present");
        for dead in DEAD_LOOSE_TAGS {
            assert!(
                !after_tags.iter().any(|t| t == dead),
                "dead loose tag {dead} must be stripped from LooseTagsByGlobalId[Drake]"
            );
        }
        assert!(
            !is_dead_by_loose_tags(&root, id),
            "no State.Dead loose tag => not dead"
        );
        // Corpse gone.
        assert!(
            !has_corpse_for(&root, id),
            "revived Drake's corpse (m_SavedInventories entry) must be removed"
        );
        // No kill-tagged memory event affecting Drake in ANY owner (Hero etc.).
        for tag in KILL_EVENT_TAGS {
            let kill_about_id = memory_owners(&root).iter().any(|(_owner, mem)| {
                let Some(PropertyValue::Array { elements }) =
                    struct_member(mem, "MemorizedEvents")
                else {
                    return false;
                };
                elements
                    .iter()
                    .any(|e| event_has_any_tag(e, &[tag]) && affected_character_id(e) == Some(id))
            });
            assert!(
                !kill_about_id,
                "no {tag} event affecting {id} may remain in any memory owner"
            );
        }

        // Idempotent on the real blob too.
        let snapshot = payload.clone();
        apply_revive(&mut payload, id).unwrap();
        assert_eq!(payload, snapshot, "second revive is a byte-identical no-op");
    }

    #[test]
    #[ignore = "needs GORESAVE_G1R012_BIN=<G1R-012 decompressed save (defeated, not killed)>"]
    fn g1r012_alive_guard_has_no_corpse_sanity() {
        // Sanity: in the DEFEATED-not-killed save the same guard has NO corpse entry
        // and is not dead — confirming the corpse is the kill-specific artifact.
        let path = std::env::var("GORESAVE_G1R012_BIN")
            .expect("set GORESAVE_G1R012_BIN to the G1R-012 decompressed save");
        let payload = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let id = "OM_GRD_Guard11_273-WorldPointActor_Guard11_273";
        let corpse_key = format!("Character_{id}");
        let root = parse_private_root(&payload).unwrap();
        assert!(
            !has_saved_inventory_entry(&root, &corpse_key),
            "alive/defeated guard must have NO corpse entry"
        );
        let n = list_npcs(&root).unwrap().into_iter().find(|n| n.id == id);
        if let Some(n) = n {
            assert!(!n.is_dead, "defeated-not-killed guard is not dead");
        }
    }
}
