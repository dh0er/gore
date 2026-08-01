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

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::CoreError;
use crate::properties::{
    ContainerEdit, Property, PropertyValue, RootObject, ScalarValue, StructValue,
    encode_fstring_value, find_property_by_name, map_key_to_string, parse_path, parse_private_root,
    patch_container, patch_map_value_tag_container, patch_scalar, patch_string, patch_value_bytes,
    resolve_chain,
};

const ATTRIBUTES_TYPE: &str = "CharacterStateSaveGameData_Attributes";
const INVENTORY_TYPE: &str = "CharacterStateSaveGameData_Inventory";
/// The saved-pose map (`PositionByGlobalId`). Same map family as
/// `_Attributes`/`_Inventory`, so [`find_character_map_path`] finds it unchanged.
const POSITION_TYPE: &str = "CharacterStateSaveGameData_Position";

/// Per-character personal relationship records. Values are inline
/// `CharacterStateSaveGameData_Relationship` structs whose
/// `ActivePersonalRelationshipModifiers` object array can contain modifiers for
/// several targets. Only permanent modifiers targeting `Hero` belong to the
/// editable NPC-to-player relationship.
const RELATIONSHIP_MAP: &str = "RelationshipByGlobalId";
const ACTIVE_RELATIONSHIP_MODIFIERS: &str = "ActivePersonalRelationshipModifiers";
/// Editor-created indefinite override. Unlike the unfortunately named
/// `_Story_Permanent` class, `_Story` has a class-default Weight of 1000 that
/// survives SaveGame reconstruction; it neither ticks nor expires.
const RELATIONSHIP_OVERRIDE_CLASS: &str =
    "/Script/Angelscript.ActivePersonalRelationshipModifier_Story";
/// Vanilla/legacy saves use this class too. Its runtime constructor receives
/// Weight 1000, but Weight is not a SaveGame field and the class has no CDO
/// default, so it reloads at the native Weight 1. Keep reading and patching it,
/// while adding the stronger `_Story` form on the next editor write.
const LEGACY_RELATIONSHIP_OVERRIDE_CLASS: &str =
    "/Script/Angelscript.ActivePersonalRelationshipModifier_Story_Permanent";
const HERO_GLOBAL_ID: &str = "Hero";

fn is_relationship_override_class(class: &str) -> bool {
    matches!(
        class,
        RELATIONSHIP_OVERRIDE_CLASS | LEGACY_RELATIONSHIP_OVERRIDE_CLASS
    )
}

/// The deliberately small relationship-override vocabulary exposed by the
/// editor. Gothic's internal enum also has `Hostile` and `Angry`; those both
/// collapse to `Enemy` when reading an explicit stored override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum PersonalRelationship {
    Friend,
    Neutral,
    Enemy,
}

impl PersonalRelationship {
    pub(crate) fn parse(value: &str) -> Result<Self, CoreError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "friend" => Ok(Self::Friend),
            "neutral" => Ok(Self::Neutral),
            "enemy" => Ok(Self::Enemy),
            _ => Err(CoreError::InvalidRequest(format!(
                "relationship must be Friend, Neutral, or Enemy; got {value:?}"
            ))),
        }
    }

    fn enum_label(self) -> &'static str {
        match self {
            Self::Friend => "ERelationship::Friend",
            Self::Neutral => "ERelationship::Neutral",
            Self::Enemy => "ERelationship::Enemy",
        }
    }

    fn severity(self) -> u8 {
        match self {
            Self::Friend => 0,
            Self::Neutral => 1,
            Self::Enemy => 2,
        }
    }

    fn worse(self, other: Self) -> Self {
        if self.severity() >= other.severity() {
            self
        } else {
            other
        }
    }
}

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
const KILL_EVENT_TAGS: &[&str] = &["Memory.Character.Defeated.Kill", "Memory.Execution"];

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
    /// Explicit permanent NPC-to-Hero relationship override, with internal
    /// Hostile/Angry values collapsed to Enemy. `None` means the save has no
    /// such override; it does not imply a neutral runtime relationship.
    pub personal_relationship: Option<PersonalRelationship>,
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
    /// The character's UniqueName knowledge-map key. When the actor has knowledge
    /// this is the actual stored `CharacterKnowledgeByUniqueName` key (exact case,
    /// so the case-sensitive knowledge query resolves); otherwise the original-case
    /// GlobalId prefix (before the first `-`).
    pub unique_name: String,
    pub is_dead: bool,
    /// Explicit permanent NPC-to-Hero relationship override, collapsed to the
    /// editor's Friend/Neutral/Enemy vocabulary. `None` means no override is
    /// stored; the runtime relationship depends on game data and other state.
    pub personal_relationship: Option<PersonalRelationship>,
    pub has_inventory: bool,
    pub has_knowledge: bool,
    pub has_events: bool,
}

/// The character's UniqueName key: the GlobalId prefix before the first `-`, in
/// ORIGINAL case. It must match the case-sensitive `CharacterKnowledgeByUniqueName`
/// map key so the frontend's knowledge read/edit query resolves. Case-insensitive
/// membership tests lowercase both sides separately.
pub(crate) fn char_key(global_id: &str) -> String {
    global_id.split('-').next().unwrap_or(global_id).to_string()
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
            PropertyValue::ObjectInstances(objs) => objs
                .iter()
                .find_map(|o| in_props(&o.properties, struct_type)),
            PropertyValue::Array { elements } | PropertyValue::Set { elements, .. } => {
                elements.iter().find_map(|e| in_value(e, struct_type))
            }
            PropertyValue::Map { entries, .. } => {
                entries.iter().find_map(|(_k, v)| in_value(v, struct_type))
            }
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
        PropertyValue::Map { entries, .. } => entries.iter().any(|(_k, v)| value_has_tag(v, tag)),
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
/// - `personal_relationship` = an explicit permanent NPC-to-Hero override when
///   present. Absence is returned as `None`, never guessed as Neutral; the game's
///   effective relationship also depends on static and active runtime modifiers.
pub fn list_npcs(root: &RootObject) -> Result<Vec<NpcSummary>, CoreError> {
    let attributes = find_character_map(root, ATTRIBUTES_TYPE)
        .ok_or_else(|| CoreError::Parse(format!("no {ATTRIBUTES_TYPE} map found in save")))?;
    // The loose-tags map (keyed by the same GlobalId) carries each NPC's persisted
    // GAS tag-state; `State.Dead` there is the authoritative dead marker.
    let loose =
        find_property_by_name(root, LOOSE_TAGS_MAP).and_then(|(_p, prop)| match &prop.value {
            PropertyValue::Map { entries, .. } => Some(entries.as_slice()),
            _ => None,
        });
    let personal_relationships = personal_relationships_by_id(root);

    let mut out = Vec::with_capacity(attributes.len());
    for (key, value) in attributes {
        let Some(id) = map_key_to_string(key) else {
            continue;
        };
        let hp = health_floats(value);
        let is_dead = loose
            .and_then(|entries| lookup_entry(entries, &id))
            .is_some_and(|tags| value_has_tag(tags, DEAD_TAG_STATE));
        let personal_relationship = personal_relationships.get(&id).copied();
        out.push(NpcSummary {
            personal_relationship,
            id,
            is_dead,
            hp: hp.health_current.or(hp.health_base),
            max_hp: hp.max_health_base,
        });
    }
    Ok(out)
}

/// Collect the stringified keys of a named MapProperty found anywhere in the
/// tree, in ORIGINAL case (used for the knowledge map, whose key case must be
/// preserved for the case-sensitive knowledge query).
fn map_keys(root: &RootObject, name: &str) -> Vec<String> {
    match find_property_by_name(root, name) {
        Some((_, prop)) => match &prop.value {
            PropertyValue::Map { entries, .. } => entries
                .iter()
                .filter_map(|(k, _)| map_key_to_string(k))
                .collect(),
            _ => Vec::new(),
        },
        None => Vec::new(),
    }
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

fn relationship_member<'a>(props: &'a [Property], name: &str) -> Option<&'a Property> {
    props.iter().find(|property| property.name == name)
}

/// Collapse Gothic's five-value relationship enum to the three values exposed
/// by the editor. `Hostile` and `Angry` both collapse to Enemy from the user's point of
/// view; unknown labels are ignored rather than misreported.
fn relationship_from_enum(label: &str) -> Option<PersonalRelationship> {
    match label.rsplit("::").next().unwrap_or(label) {
        "Friend" => Some(PersonalRelationship::Friend),
        "Neutral" => Some(PersonalRelationship::Neutral),
        "Hostile" | "Angry" | "Enemy" => Some(PersonalRelationship::Enemy),
        _ => None,
    }
}

/// Index every explicit permanent NPC-to-Hero relationship by actor GlobalId.
/// If malformed or legacy data contains multiple such modifiers, expose their
/// most restrictive explicit value deterministically. This remains a stored
/// override only; it is not the relationship computed by the running game.
fn personal_relationships_by_id(root: &RootObject) -> HashMap<String, PersonalRelationship> {
    let Some((_path, map_property)) = find_property_by_name(root, RELATIONSHIP_MAP) else {
        return HashMap::new();
    };
    let PropertyValue::Map { entries, .. } = &map_property.value else {
        return HashMap::new();
    };

    let mut relationships = HashMap::new();
    for (key, entry_value) in entries {
        let Some(id) = map_key_to_string(key) else {
            continue;
        };
        let Some(entry_properties) = entry_props(entry_value) else {
            continue;
        };
        let Some(PropertyValue::ObjectInstances(modifiers)) =
            relationship_member(entry_properties, ACTIVE_RELATIONSHIP_MODIFIERS)
                .map(|property| &property.value)
        else {
            continue;
        };

        let mut stored: Option<PersonalRelationship> = None;
        for modifier in modifiers {
            if !is_relationship_override_class(&modifier.class) {
                continue;
            }
            let targets_hero = relationship_member(&modifier.properties, "TargetCharacterGlobalID")
                .is_some_and(|property| {
                    matches!(
                        &property.value,
                        PropertyValue::Name(value) | PropertyValue::Str(value)
                            if value == HERO_GLOBAL_ID
                    )
                });
            if !targets_hero {
                continue;
            }
            let Some(relationship) = relationship_member(&modifier.properties, "Relationship")
                .and_then(|property| match &property.value {
                    PropertyValue::Enum(value) => relationship_from_enum(value),
                    _ => None,
                })
            else {
                continue;
            };
            stored = Some(stored.map_or(relationship, |old| old.worse(relationship)));
        }
        if let Some(stored) = stored {
            relationships.insert(id, stored);
        }
    }
    relationships
}

/// Build the unified character list: every spawned actor (from [`list_npcs`])
/// annotated with availability flags, followed by knowledge-only orphan rows
/// (a knowledge UniqueName with no matching actor charKey). The join is the
/// proven prefix rule ([`char_key`]).
pub fn list_characters(root: &RootObject) -> Result<Vec<CharacterSummary>, CoreError> {
    // Knowledge keys in ORIGINAL case, indexed by lowercased form for the join.
    let knowledge_orig = map_keys(root, "CharacterKnowledgeByUniqueName");
    let knowledge_by_lower: HashMap<String, String> = knowledge_orig
        .iter()
        .map(|k| (k.to_ascii_lowercase(), k.clone()))
        .collect();
    let events = map_keys_lower(root, LONG_TERM_MEMORY_MAP);
    let inventory = character_map_keys_lower(root, INVENTORY_TYPE);

    let npcs = list_npcs(root)?;
    let mut actor_keys_lower: HashSet<String> = HashSet::new();
    let mut out: Vec<CharacterSummary> = Vec::with_capacity(npcs.len());
    for npc in &npcs {
        let prefix = char_key(&npc.id); // original case
        let lk = prefix.to_ascii_lowercase();
        actor_keys_lower.insert(lk.clone());
        let id_lower = npc.id.to_ascii_lowercase();
        // Prefer the actual stored knowledge key (exact case) so the frontend's
        // knowledge read/edit query resolves; fall back to the GlobalId prefix
        // (the key used when ADDING knowledge to an NPC that has none).
        let (unique_name, has_knowledge) = match knowledge_by_lower.get(&lk) {
            Some(stored) => (stored.clone(), true),
            None => (prefix, false),
        };
        out.push(CharacterSummary {
            global_id: Some(npc.id.clone()),
            unique_name,
            is_dead: npc.is_dead,
            personal_relationship: npc.personal_relationship,
            has_inventory: inventory.contains(&id_lower),
            has_knowledge,
            has_events: events.contains(&id_lower),
        });
    }
    // Knowledge-only orphans (original case), no matching actor charKey.
    let mut orphans: Vec<String> = knowledge_orig
        .into_iter()
        .filter(|k| !actor_keys_lower.contains(&k.to_ascii_lowercase()))
        .collect();
    orphans.sort();
    for key in orphans {
        out.push(CharacterSummary {
            global_id: None,
            unique_name: key,
            is_dead: false,
            personal_relationship: None,
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
    let (entries, mut path) = find_character_map_path(root, ATTRIBUTES_TYPE)
        .ok_or_else(|| CoreError::Parse(format!("no {ATTRIBUTES_TYPE} map found in save")))?;
    let value = lookup_entry(entries, id).ok_or_else(|| {
        CoreError::Parse(format!("NPC {id:?} not found in {ATTRIBUTES_TYPE} map"))
    })?;

    // The entry is addressed by its map key; descend from there.
    path.push(map_key_segment(id));
    let mut rows = Vec::new();
    collect_attribute_rows(value, &mut path, &mut rows);
    Ok(rows)
}

/// A world-space point read from a `Vector`-descriptor native struct.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// An orientation read from a `Rotator`-descriptor native struct, in the order
/// the engine serialises it: Pitch, Yaw, Roll.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Rot3 {
    pub pitch: f64,
    pub yaw: f64,
    pub roll: f64,
}

/// One NPC's saved pose, with the full typed paths `private.typed.setValue`
/// needs to write each of the four leaves.
///
/// The `*_path` fields are segment lists in the form
/// [`crate::properties::parse_path`] accepts (property names verbatim,
/// `{mapKey}` for map keys), rooted at the private root — exactly like
/// [`NpcAttributeRow`]'s `base_path`/`current_path`.
///
/// **This pose is a SNAPSHOT, not an input.** `private.typed.setValue` can
/// address these leaves and the bytes do change, but the game discards them on
/// load: a UE4SS runtime probe rewrote `CharacterLocation`, `SpawnLocation` and
/// `DailyRoutineClass` for two NPCs (one streamed out, one simulated), loaded
/// the byte-verified save, and read back the ORIGINAL pre-edit values in every
/// field. Placement authority is the level's WorldPointActor named in the NPC's
/// GlobalId. NPC *attributes* in the same `{CharacterStates}` blob do apply, so
/// the blob is read — these records are simply never used. Reading stays
/// worthwhile (CLI, MCP, diagnostics); do not build a mover on top of it.
///
/// **Why the rotations are renamed here and nowhere else.** The generic property
/// parser collapses BOTH the `Vector` and the `Rotator` descriptor into the same
/// `StructValue::Vector3 { x, y, z }` variant (see `read_struct_value` in
/// `properties.rs`), so by the time a value reaches a generic consumer the
/// descriptor — the only thing that says "this triplet is an orientation" — is
/// gone. A *curated* command like [`npc_position`] still knows which member it
/// asked for, so it is the right and only place to restore the engine's names:
/// x=Pitch, y=Yaw, z=Roll, matching the memory order `private_rotator_ref_at`
/// reads in `lib.rs`. The generic All-Data browse path deliberately keeps
/// `x/y/z`. Any future curated command that surfaces a `Rotator` must do the
/// same renaming itself — there is no shared layer that can do it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcPose {
    pub location: Option<Vec3>,
    pub rotation: Option<Rot3>,
    pub spawn_location: Option<Vec3>,
    pub spawn_rotation: Option<Rot3>,
    pub location_path: Vec<String>,
    pub rotation_path: Vec<String>,
    pub spawn_location_path: Vec<String>,
    pub spawn_rotation_path: Vec<String>,
}

/// Read NPC `id`'s saved pose out of the `_Position` map, each leaf paired with
/// the typed path `private.typed.setValue` resolves against the private root.
///
/// Errors if the `_Position` map is absent; errors with a not-found message if
/// the map exists but holds no entry for `id`. A member that is missing (or is
/// not a triplet native struct) comes back as `None` while its path is still
/// reported, so a caller can tell "absent leaf" from "absent NPC".
pub fn npc_position(root: &RootObject, id: &str) -> Result<NpcPose, CoreError> {
    let (entries, mut path) = find_character_map_path(root, POSITION_TYPE)
        .ok_or_else(|| CoreError::Parse(format!("no {POSITION_TYPE} map found in save")))?;
    let value = lookup_entry(entries, id)
        .ok_or_else(|| CoreError::Parse(format!("NPC {id:?} not found in {POSITION_TYPE} map")))?;

    // The entry is addressed by its map key; descend from there.
    path.push(map_key_segment(id));
    let member_path = |name: &str| {
        let mut p = path.clone();
        p.push(name.to_string());
        p
    };
    // `Vector` and `Rotator` both parse to Vector3 (f64) — or to Vector3f when
    // the save stores the compact f32 form; accept either.
    let triplet = |name: &str| match struct_member(value, name) {
        Some(PropertyValue::Struct(StructValue::Vector3 { x, y, z })) => Some((*x, *y, *z)),
        Some(PropertyValue::Struct(StructValue::Vector3f { x, y, z })) => {
            Some((*x as f64, *y as f64, *z as f64))
        }
        _ => None,
    };
    let point = |name: &str| triplet(name).map(|(x, y, z)| Vec3 { x, y, z });
    let rotation = |name: &str| triplet(name).map(|(pitch, yaw, roll)| Rot3 { pitch, yaw, roll });

    Ok(NpcPose {
        location: point("CharacterLocation"),
        rotation: rotation("CharacterRotation"),
        spawn_location: point("SpawnLocation"),
        spawn_rotation: rotation("SpawnRotation"),
        location_path: member_path("CharacterLocation"),
        rotation_path: member_path("CharacterRotation"),
        spawn_location_path: member_path("SpawnLocation"),
        spawn_rotation_path: member_path("SpawnRotation"),
    })
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
            map_key_to_string(k)
                .as_deref()
                .is_some_and(|k| is_corpse_key_for(k, id))
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
            let removed =
                patch_map_value_tag_container(payload, &target, &enclosing, entry_index, tag)?;
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
        let health = rows
            .iter()
            .find(|r| r.key == "Health")
            .ok_or_else(|| CoreError::Parse(format!("NPC {id:?} has no Health attribute")))?;
        let max_hp = rows
            .iter()
            .find(|r| r.key == "MaxHealth")
            .and_then(|r| r.base)
            .ok_or_else(|| {
                CoreError::Parse(format!(
                    "NPC {id:?} has no MaxHealth BaseValue to revive to"
                ))
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

fn relationship_name_property(name: &str, value: &str) -> Vec<u8> {
    let body = encode_fstring_value(value);
    let mut output = encode_fstring_value(name);
    output.extend_from_slice(&encode_fstring_value("NameProperty"));
    output.extend_from_slice(&0u32.to_le_bytes()); // array_index
    output.extend_from_slice(&(body.len() as u32).to_le_bytes());
    output.push(0); // tag_flags
    output.extend_from_slice(&body);
    output
}

fn relationship_enum_property(value: PersonalRelationship) -> Vec<u8> {
    let body = encode_fstring_value(value.enum_label());
    let mut output = encode_fstring_value("Relationship");
    output.extend_from_slice(&encode_fstring_value("EnumProperty"));
    output.extend_from_slice(&1u32.to_le_bytes());
    output.extend_from_slice(&encode_fstring_value("ERelationship"));
    output.extend_from_slice(&1u32.to_le_bytes());
    output.extend_from_slice(&encode_fstring_value("/Script/G1R"));
    output.extend_from_slice(&1u32.to_le_bytes());
    output.extend_from_slice(&encode_fstring_value("ByteProperty"));
    output.extend_from_slice(&0u32.to_le_bytes()); // array_index
    output.extend_from_slice(&(body.len() as u32).to_le_bytes());
    output.push(0); // tag_flags
    output.extend_from_slice(&body);
    output
}

/// A self-contained inline `_Story` relationship object. Only its two actual
/// SaveGame fields are serialized; the inherited Weight comes from this
/// class's 1000-valued CDO and is intentionally not an inert injected tag.
fn relationship_override_modifier_bytes(value: PersonalRelationship) -> Vec<u8> {
    relationship_modifier_bytes_for_class(RELATIONSHIP_OVERRIDE_CLASS, value)
}

fn relationship_modifier_bytes_for_class(class: &str, value: PersonalRelationship) -> Vec<u8> {
    let mut output = encode_fstring_value(class);
    output.push(0); // object flag
    output.extend_from_slice(&relationship_enum_property(value));
    output.extend_from_slice(&relationship_name_property(
        "TargetCharacterGlobalID",
        HERO_GLOBAL_ID,
    ));
    output.extend_from_slice(&encode_fstring_value("None"));
    output.extend_from_slice(&0u32.to_le_bytes()); // object footer
    output
}

fn active_relationship_modifiers_property(value: PersonalRelationship) -> Vec<u8> {
    let modifier = relationship_override_modifier_bytes(value);
    let mut body = 1u32.to_le_bytes().to_vec();
    body.extend_from_slice(&modifier);

    let mut output = encode_fstring_value(ACTIVE_RELATIONSHIP_MODIFIERS);
    output.extend_from_slice(&encode_fstring_value("ArrayProperty"));
    output.extend_from_slice(&1u32.to_le_bytes());
    output.extend_from_slice(&encode_fstring_value("ObjectProperty"));
    output.extend_from_slice(&0u32.to_le_bytes()); // array_index
    output.extend_from_slice(&(body.len() as u32).to_le_bytes());
    output.push(0); // tag_flags
    output.extend_from_slice(&body);
    output
}

/// Inline value of one `RelationshipByGlobalId` map entry (a tagged struct
/// property list, without a separate size prefix).
fn relationship_entry_value_bytes(value: PersonalRelationship) -> Vec<u8> {
    let mut output = active_relationship_modifiers_property(value);
    output.extend_from_slice(&encode_fstring_value("None"));
    output
}

fn modifier_targets_hero(properties: &[Property]) -> bool {
    relationship_member(properties, "TargetCharacterGlobalID").is_some_and(|property| {
        matches!(
            &property.value,
            PropertyValue::Name(value) | PropertyValue::Str(value) if value == HERO_GLOBAL_ID
        )
    })
}

/// Set one NPC's explicit permanent relationship override towards Hero. This
/// edits only the dedicated relationship map and leaves death state and crime
/// history untouched. The editor does not claim to reproduce the game's full
/// runtime relationship evaluation. Handles an existing modifier, an
/// empty/populated object array, and a completely empty relationship map.
pub fn apply_relationship(
    payload: &mut Vec<u8>,
    id: &str,
    relationship: PersonalRelationship,
) -> Result<(), CoreError> {
    let id = id.trim();
    if id.is_empty() {
        return Err(CoreError::InvalidRequest(
            "private.npc.setRelationship requires a non-empty id".to_string(),
        ));
    }

    // Refuse accidental map entries for non-characters/typos.
    let root = parse_private_root(payload)?;
    let attributes = find_character_map(&root, ATTRIBUTES_TYPE)
        .ok_or_else(|| CoreError::Parse(format!("no {ATTRIBUTES_TYPE} map found in save")))?;
    if lookup_entry(attributes, id).is_none() {
        return Err(CoreError::InvalidRequest(format!(
            "NPC {id:?} does not exist in the save"
        )));
    }

    // Patch every recognized Hero override (including the weak legacy class),
    // then ensure the entry also has the indefinite `_Story` class whose CDO
    // retains Weight 1000 after SaveGame load. Reparse after every splice
    // because Friend/Neutral/Enemy have different byte lengths.
    let mut missing_entry = false;
    loop {
        let root = parse_private_root(payload)?;
        let Some((map_path, map_property)) = find_property_by_name(&root, RELATIONSHIP_MAP) else {
            return Err(CoreError::Parse(format!(
                "{RELATIONSHIP_MAP} map not found in save"
            )));
        };
        let PropertyValue::Map { entries, .. } = &map_property.value else {
            return Err(CoreError::Parse(format!("{RELATIONSHIP_MAP} is not a map")));
        };
        let Some((_key, entry_value)) = entries
            .iter()
            .find(|(key, _)| map_key_to_string(key).as_deref() == Some(id))
        else {
            missing_entry = true;
            break;
        };
        let Some(entry_properties) = entry_props(entry_value) else {
            return Err(CoreError::Parse(format!(
                "{RELATIONSHIP_MAP}[{id:?}] is not a property-list struct"
            )));
        };
        let Some(array_property) =
            relationship_member(entry_properties, ACTIVE_RELATIONSHIP_MODIFIERS)
        else {
            return Err(CoreError::Parse(format!(
                "{RELATIONSHIP_MAP}[{id:?}] has no {ACTIVE_RELATIONSHIP_MODIFIERS} array"
            )));
        };
        let mut next_mismatch = None;
        let mut found_strong_override = false;
        match &array_property.value {
            PropertyValue::ObjectInstances(modifiers) => {
                for (index, modifier) in modifiers.iter().enumerate() {
                    if !is_relationship_override_class(&modifier.class)
                        || !modifier_targets_hero(&modifier.properties)
                    {
                        continue;
                    }
                    let Some(property) = relationship_member(&modifier.properties, "Relationship")
                    else {
                        continue;
                    };
                    let PropertyValue::Enum(current) = &property.value else {
                        continue;
                    };
                    if modifier.class == RELATIONSHIP_OVERRIDE_CLASS {
                        found_strong_override = true;
                    }
                    if current != relationship.enum_label() {
                        next_mismatch = Some(index);
                        break;
                    }
                }
            }
            PropertyValue::Array { elements } if elements.is_empty() => {}
            _ => {
                return Err(CoreError::Parse(format!(
                    "{ACTIVE_RELATIONSHIP_MODIFIERS} has an unsupported array encoding"
                )));
            }
        }

        if let Some(index) = next_mismatch {
            let mut path = map_path;
            path.push(format!("{{{id}}}"));
            path.push(ACTIVE_RELATIONSHIP_MODIFIERS.to_string());
            path.push(format!("[{index}]"));
            path.push("Relationship".to_string());
            let chain = resolve_chain(&root.properties, &parse_path(&path)?)?;
            let target = chain.target.clone();
            let enclosing = chain.enclosing_size_fields.clone();
            patch_string(payload, &target, &enclosing, relationship.enum_label())?;
            continue;
        }

        if found_strong_override {
            break;
        }

        // The map entry exists but has only unrelated modifiers or a weak
        // legacy `_Story_Permanent` override. Append the strong, indefinite
        // `_Story` representation while preserving every existing object.
        let mut path = map_path;
        path.push(format!("{{{id}}}"));
        path.push(ACTIVE_RELATIONSHIP_MODIFIERS.to_string());
        let chain = resolve_chain(&root.properties, &parse_path(&path)?)?;
        let target = chain.target.clone();
        let enclosing = chain.enclosing_size_fields.clone();
        let object_bytes = relationship_override_modifier_bytes(relationship);
        match &target.value {
            PropertyValue::Array { elements } if elements.is_empty() => {
                patch_container(
                    payload,
                    &target,
                    &enclosing,
                    &ContainerEdit::ArrayInsertBytes(object_bytes),
                )?;
            }
            PropertyValue::ObjectInstances(objects) => {
                let end = target
                    .value_offset
                    .checked_add(target.value_size)
                    .filter(|end| *end <= payload.len())
                    .ok_or_else(|| {
                        CoreError::Parse(
                            "relationship modifier array points outside payload".to_string(),
                        )
                    })?;
                if target.value_size < 4 {
                    return Err(CoreError::Parse(
                        "relationship modifier array is missing its count".to_string(),
                    ));
                }
                let mut body = payload[target.value_offset..end].to_vec();
                let new_count = u32::try_from(objects.len() + 1).map_err(|_| {
                    CoreError::InvalidRequest(
                        "too many personal relationship modifiers".to_string(),
                    )
                })?;
                body[..4].copy_from_slice(&new_count.to_le_bytes());
                body.extend_from_slice(&object_bytes);
                patch_value_bytes(payload, &target, &enclosing, &body)?;
            }
            _ => unreachable!("array encoding validated above"),
        }
        // Reparse and verify the newly appended object before finishing.
    }

    if missing_entry {
        // No entry existed: append a fully-formed inline map key/value pair.
        let root = parse_private_root(payload)?;
        let (map_path, map_property) = find_property_by_name(&root, RELATIONSHIP_MAP)
            .ok_or_else(|| CoreError::Parse(format!("{RELATIONSHIP_MAP} map not found in save")))?;
        let PropertyValue::Map { entries, .. } = &map_property.value else {
            return Err(CoreError::Parse(format!("{RELATIONSHIP_MAP} is not a map")));
        };
        if entries
            .iter()
            .any(|(key, _)| map_key_to_string(key).as_deref() == Some(id))
        {
            return Err(CoreError::Parse(format!(
                "failed to create a permanent relationship modifier for {id:?}"
            )));
        }
        let mut entry_bytes = encode_fstring_value(id);
        entry_bytes.extend_from_slice(&relationship_entry_value_bytes(relationship));
        let chain = resolve_chain(&root.properties, &parse_path(&map_path)?)?;
        let target = chain.target.clone();
        let enclosing = chain.enclosing_size_fields.clone();
        patch_container(
            payload,
            &target,
            &enclosing,
            &ContainerEdit::MapInsert { entry_bytes },
        )?;
    }

    let reparsed = parse_private_root(payload).map_err(|error| {
        CoreError::Parse(format!(
            "setRelationship produced an inconsistent payload: {error}"
        ))
    })?;
    let stored = personal_relationships_by_id(&reparsed).get(id).copied();
    if stored != Some(relationship) {
        return Err(CoreError::Validation(format!(
            "NPC {id:?} relationship override re-read as {stored:?}, expected {relationship:?}"
        )));
    }
    Ok(())
}

/// REVIVE NPC `id` in a decoded private payload, in place — restore a KILLED NPC
/// to its alive state by undoing all persisted death state.
///
/// 1. Scan every `LongTermMemoryByGlobalId` owner and remove only defeat/kill
///    events that refer to this NPC.
/// 2. Strip the authoritative `State.Dead`, kill-bounty, and execution-bounty
///    loose tags while preserving unrelated tags.
/// 3. Remove the NPC's lootable corpse entry from `m_SavedInventories`.
/// 4. Restore Health to MaxHealth.
///
/// Each structural step reparses before continuing, so shifted offsets and
/// indices are never reused. An already-alive NPC with full HP is a clean no-op.
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
        patch_container(
            payload,
            &target,
            &enclosing,
            &ContainerEdit::ArrayRemove(index),
        )?;
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

        let path =
            npc_inventory_path(&root, id).unwrap_or_else(|| panic!("no inventory path for {id}"));
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

    use crate::properties::{TAG_FLAG_BOOL_TRUE, TAG_FLAG_NATIVE_SERIALIZE};

    fn fstring(value: &str) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&((value.len() + 1) as i32).to_le_bytes());
        out.extend_from_slice(value.as_bytes());
        out.push(0);
        out
    }

    fn property_tag(name: &str, type_name: &str) -> Vec<u8> {
        let mut out = fstring(name);
        out.extend_from_slice(&fstring(type_name));
        out
    }

    fn property_header(size: u32, flags: u8) -> Vec<u8> {
        let mut out = 0u32.to_le_bytes().to_vec();
        out.extend_from_slice(&size.to_le_bytes());
        out.push(flags);
        out
    }

    fn int_property(name: &str, value: i32) -> Vec<u8> {
        let mut out = property_tag(name, "IntProperty");
        out.extend_from_slice(&property_header(4, 0));
        out.extend_from_slice(&value.to_le_bytes());
        out
    }

    fn bool_property(name: &str, value: bool) -> Vec<u8> {
        let mut out = property_tag(name, "BoolProperty");
        out.extend_from_slice(&property_header(
            0,
            if value { TAG_FLAG_BOOL_TRUE } else { 0 },
        ));
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
        let entries: Vec<(&str, Vec<u8>)> = keys.iter().map(|k| (*k, fstring("None"))).collect();
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

    /// A real-shaped `CrimeMemoryPersistentData` entry in `m_GenericData`.
    /// It contains both an unsuppressed Hero crime and an already-suppressed
    /// non-Hero crime for `witness`, so the relationship writer's non-interference
    /// guarantee covers the complete persisted crime subtree.
    fn crime_memory_property(witness: &str) -> Vec<u8> {
        fn struct_array(name: &str, struct_type: &str, entries: &[Vec<u8>]) -> Vec<u8> {
            let mut descriptor = 1u32.to_le_bytes().to_vec();
            descriptor.extend_from_slice(&fstring("StructProperty"));
            descriptor.extend_from_slice(&1u32.to_le_bytes());
            descriptor.extend_from_slice(&fstring(struct_type));
            descriptor.extend_from_slice(&1u32.to_le_bytes());
            descriptor.extend_from_slice(&fstring("/Script/G1R"));

            let mut body = (entries.len() as u32).to_le_bytes().to_vec();
            for entry in entries {
                body.extend_from_slice(entry);
                body.extend_from_slice(&fstring("None"));
            }

            let mut out = property_tag(name, "ArrayProperty");
            out.extend_from_slice(&descriptor);
            out.extend_from_slice(&property_header(body.len() as u32, 0));
            out.extend_from_slice(&body);
            out
        }

        let global_entry = |id, forgiven, criminal: &str| {
            let mut out = int_property("ID", id);
            out.extend_from_slice(&bool_property("bIsForgiven", forgiven));
            out.extend_from_slice(&name_property("CriminalGlobalID", criminal));
            out
        };
        let globals = struct_array(
            "GlobalCrimeDataEntries",
            "FGlobalCrimeDataEntry",
            &[
                global_entry(100, false, "Hero"),
                global_entry(200, true, "OC_GRD_Other"),
            ],
        );

        let relative_entry = |id, suppressed| {
            let mut out = int_property("ID", id);
            out.extend_from_slice(&bool_property("bIsSuppressed", suppressed));
            out
        };
        let relative_crimes = struct_array(
            "RelativeCrimes",
            "FRelativeCrimeDataEntry",
            &[relative_entry(100, false), relative_entry(200, true)],
        );

        let mut witness_value = relative_crimes;
        witness_value.extend_from_slice(&fstring("None"));
        let mut relative_descriptor = 2u32.to_le_bytes().to_vec();
        relative_descriptor.extend_from_slice(&fstring("NameProperty"));
        relative_descriptor.extend_from_slice(&0u32.to_le_bytes());
        relative_descriptor.extend_from_slice(&fstring("StructProperty"));
        relative_descriptor.extend_from_slice(&1u32.to_le_bytes());
        relative_descriptor.extend_from_slice(&fstring("FRelativeCrimesContainer"));
        relative_descriptor.extend_from_slice(&1u32.to_le_bytes());
        relative_descriptor.extend_from_slice(&fstring("/Script/G1R"));
        let mut relative_body = 0u32.to_le_bytes().to_vec();
        relative_body.extend_from_slice(&1u32.to_le_bytes());
        relative_body.extend_from_slice(&fstring(witness));
        relative_body.extend_from_slice(&witness_value);
        let mut relatives = property_tag("RelativeCrimeDataEntries", "MapProperty");
        relatives.extend_from_slice(&relative_descriptor);
        relatives.extend_from_slice(&property_header(relative_body.len() as u32, 0));
        relatives.extend_from_slice(&relative_body);

        let mut crime_body = globals;
        crime_body.extend_from_slice(&relatives);
        crime_body.extend_from_slice(&fstring("None"));
        let mut crime_instance = fstring("/Script/G1R.GothicCrimeMemorySaveGameData");
        crime_instance.extend_from_slice(&(crime_body.len() as u32).to_le_bytes());
        crime_instance.extend_from_slice(&crime_body);

        let mut generic_body = 0u32.to_le_bytes().to_vec();
        generic_body.extend_from_slice(&1u32.to_le_bytes());
        generic_body.extend_from_slice(&fstring("CrimeMemoryPersistentData"));
        generic_body.extend_from_slice(&crime_instance);
        let mut generic_descriptor = 2u32.to_le_bytes().to_vec();
        generic_descriptor.extend_from_slice(&fstring("NameProperty"));
        generic_descriptor.extend_from_slice(&0u32.to_le_bytes());
        generic_descriptor.extend_from_slice(&fstring("StructProperty"));
        generic_descriptor.extend_from_slice(&1u32.to_le_bytes());
        generic_descriptor.extend_from_slice(&fstring("InstancedStruct"));
        generic_descriptor.extend_from_slice(&1u32.to_le_bytes());
        generic_descriptor.extend_from_slice(&fstring("/Script/StructUtils"));
        let mut generic = property_tag("m_GenericData", "MapProperty");
        generic.extend_from_slice(&generic_descriptor);
        generic.extend_from_slice(&property_header(generic_body.len() as u32, 0));
        generic.extend_from_slice(&generic_body);
        generic
    }

    fn empty_relationship_entry_value() -> Vec<u8> {
        let body = 0u32.to_le_bytes();
        let mut out = fstring(ACTIVE_RELATIONSHIP_MODIFIERS);
        out.extend_from_slice(&fstring("ArrayProperty"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("ObjectProperty"));
        out.extend_from_slice(&0u32.to_le_bytes()); // array_index
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.push(0); // tag_flags
        out.extend_from_slice(&body);
        out.extend_from_slice(&fstring("None"));
        out
    }

    fn relationship_entry_value_for_class(class: &str, value: PersonalRelationship) -> Vec<u8> {
        let modifier = relationship_modifier_bytes_for_class(class, value);
        let mut body = 1u32.to_le_bytes().to_vec();
        body.extend_from_slice(&modifier);

        let mut out = fstring(ACTIVE_RELATIONSHIP_MODIFIERS);
        out.extend_from_slice(&fstring("ArrayProperty"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("ObjectProperty"));
        out.extend_from_slice(&0u32.to_le_bytes()); // array_index
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.push(0); // tag_flags
        out.extend_from_slice(&body);
        out.extend_from_slice(&fstring("None"));
        out
    }

    fn relationship_payload(id: &str, relationship_entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
        private_root(&[
            character_state_map(
                "AttributesMap",
                ATTRIBUTES_TYPE,
                &[(id, attributes_entry_value(100.0, 100.0))],
            ),
            character_state_map(
                RELATIONSHIP_MAP,
                "CharacterStateSaveGameData_Relationship",
                relationship_entries,
            ),
        ])
    }

    fn relationship_payload_with_crimes(
        id: &str,
        relationship_entries: &[(&str, Vec<u8>)],
    ) -> Vec<u8> {
        // Keep the crime property first. Relationship edits can change byte
        // lengths later in the root without changing any crime subtree offsets,
        // so a full parsed-property equality check is exact and meaningful.
        private_root(&[
            crime_memory_property(id),
            character_state_map(
                "AttributesMap",
                ATTRIBUTES_TYPE,
                &[(id, attributes_entry_value(100.0, 100.0))],
            ),
            character_state_map(
                RELATIONSHIP_MAP,
                "CharacterStateSaveGameData_Relationship",
                relationship_entries,
            ),
        ])
    }

    fn crime_property_snapshot(payload: &[u8]) -> Property {
        let root = parse_private_root(payload).expect("strict crime payload parse");
        assert_eq!(root.consumed, payload.len());
        find_property_by_name(&root, "m_GenericData")
            .expect("m_GenericData crime property")
            .1
            .clone()
    }

    fn assert_relationship_summary(
        payload: &[u8],
        id: &str,
        expected: Option<PersonalRelationship>,
    ) {
        let root = parse_private_root(payload).expect("strict relationship payload parse");
        assert_eq!(root.consumed, payload.len());
        let row = list_npcs(&root)
            .unwrap()
            .into_iter()
            .find(|row| row.id == id)
            .expect("NPC summary row");
        assert_eq!(row.personal_relationship, expected);
    }

    #[test]
    fn relationship_summary_serializes_absent_override_as_null_without_effective_field() {
        let id = "NC_ORG_Buster_780-WorldPointActor_Buster";
        let payload = relationship_payload(id, &[]);
        let root = parse_private_root(&payload).unwrap();
        let row = list_npcs(&root)
            .unwrap()
            .into_iter()
            .find(|row| row.id == id)
            .unwrap();
        let value = serde_json::to_value(row).unwrap();
        assert_eq!(value["personalRelationship"], serde_json::Value::Null);
        assert!(value.get("effectiveRelationship").is_none());
    }

    #[test]
    fn set_relationship_patches_existing_permanent_modifier() {
        let id = "OM_GRD_Asghan_263-WorldPointActor_Asghan";
        let existing = relationship_entry_value_bytes(PersonalRelationship::Friend);
        let mut payload = relationship_payload(id, &[(id, existing)]);
        assert_relationship_summary(&payload, id, Some(PersonalRelationship::Friend));

        apply_relationship(&mut payload, id, PersonalRelationship::Enemy).unwrap();
        assert_relationship_summary(&payload, id, Some(PersonalRelationship::Enemy));

        // A second length-changing enum rewrite remains byte-clean.
        apply_relationship(&mut payload, id, PersonalRelationship::Neutral).unwrap();
        assert_relationship_summary(&payload, id, Some(PersonalRelationship::Neutral));
    }

    #[test]
    fn set_relationship_migrates_legacy_weak_modifier_to_story_override() {
        let id = "OM_GRD_Asghan_263-WorldPointActor_Asghan";
        let legacy = relationship_entry_value_for_class(
            LEGACY_RELATIONSHIP_OVERRIDE_CLASS,
            PersonalRelationship::Friend,
        );
        let mut payload = relationship_payload(id, &[(id, legacy)]);
        assert_relationship_summary(&payload, id, Some(PersonalRelationship::Friend));

        // Even selecting the already stored value must add the stronger
        // `_Story` class; merely patching `_Story_Permanent` would reload at
        // native Weight 1 and lose ties to static relationships.
        apply_relationship(&mut payload, id, PersonalRelationship::Friend).unwrap();

        let root = parse_private_root(&payload).unwrap();
        let (_path, map) = find_property_by_name(&root, RELATIONSHIP_MAP).unwrap();
        let PropertyValue::Map { entries, .. } = &map.value else {
            panic!("relationship map expected");
        };
        let (_key, entry) = entries
            .iter()
            .find(|(key, _)| map_key_to_string(key).as_deref() == Some(id))
            .unwrap();
        let props = entry_props(entry).unwrap();
        let array = relationship_member(props, ACTIVE_RELATIONSHIP_MODIFIERS).unwrap();
        let PropertyValue::ObjectInstances(modifiers) = &array.value else {
            panic!("modifier object array expected");
        };
        assert!(
            modifiers
                .iter()
                .any(|modifier| modifier.class == LEGACY_RELATIONSHIP_OVERRIDE_CLASS)
        );
        assert!(modifiers.iter().any(|modifier| {
            modifier.class == RELATIONSHIP_OVERRIDE_CLASS
                && modifier_targets_hero(&modifier.properties)
        }));

        // A later edit keeps legacy and strong representations coherent, so
        // the reader never reports a stale stricter value from the old object.
        apply_relationship(&mut payload, id, PersonalRelationship::Enemy).unwrap();
        assert_relationship_summary(&payload, id, Some(PersonalRelationship::Enemy));
        let root = parse_private_root(&payload).unwrap();
        let (_path, map) = find_property_by_name(&root, RELATIONSHIP_MAP).unwrap();
        let PropertyValue::Map { entries, .. } = &map.value else {
            panic!("relationship map expected");
        };
        let (_key, entry) = entries
            .iter()
            .find(|(key, _)| map_key_to_string(key).as_deref() == Some(id))
            .unwrap();
        let props = entry_props(entry).unwrap();
        let array = relationship_member(props, ACTIVE_RELATIONSHIP_MODIFIERS).unwrap();
        let PropertyValue::ObjectInstances(modifiers) = &array.value else {
            panic!("modifier object array expected");
        };
        for modifier in modifiers.iter().filter(|modifier| {
            is_relationship_override_class(&modifier.class)
                && modifier_targets_hero(&modifier.properties)
        }) {
            assert_eq!(
                relationship_member(&modifier.properties, "Relationship")
                    .map(|property| &property.value),
                Some(&PropertyValue::Enum(
                    PersonalRelationship::Enemy.enum_label().to_string()
                ))
            );
        }
    }

    #[test]
    fn set_relationship_preserves_all_crime_entries_and_flags() {
        let id = "OC_STT_Diego";

        for desired in [
            PersonalRelationship::Friend,
            PersonalRelationship::Neutral,
            PersonalRelationship::Enemy,
        ] {
            let empty = empty_relationship_entry_value();
            let mut payload = relationship_payload_with_crimes(id, &[(id, empty)]);
            let crime_before = crime_property_snapshot(&payload);

            apply_relationship(&mut payload, id, desired).unwrap();

            let reparsed = parse_private_root(&payload).expect("strict post-edit reparse");
            assert_eq!(reparsed.consumed, payload.len());
            assert_eq!(
                personal_relationships_by_id(&reparsed).get(id),
                Some(&desired),
                "{desired:?} permanent override must be stored"
            );
            assert_eq!(
                crime_property_snapshot(&payload),
                crime_before,
                "{desired:?} must leave the entire persisted crime subtree byte-semantically unchanged"
            );
        }
    }

    #[test]
    fn set_relationship_appends_to_empty_modifier_array() {
        let id = "PC_THF_Diego_100-WorldPointActor_Diego";
        let empty = empty_relationship_entry_value();
        let mut payload = relationship_payload(id, &[(id, empty)]);
        assert_relationship_summary(&payload, id, None);

        apply_relationship(&mut payload, id, PersonalRelationship::Friend).unwrap();
        assert_relationship_summary(&payload, id, Some(PersonalRelationship::Friend));
        let root = parse_private_root(&payload).unwrap();
        let relationships = personal_relationships_by_id(&root);
        assert_eq!(relationships.get(id), Some(&PersonalRelationship::Friend));
    }

    #[test]
    fn set_relationship_inserts_into_empty_map() {
        let id = "NC_ORG_Buster_780-WorldPointActor_Buster";
        let mut payload = relationship_payload(id, &[]);
        assert_relationship_summary(&payload, id, None);

        apply_relationship(&mut payload, id, PersonalRelationship::Neutral).unwrap();
        assert_relationship_summary(&payload, id, Some(PersonalRelationship::Neutral));

        let root = parse_private_root(&payload).unwrap();
        let (_path, map) = find_property_by_name(&root, RELATIONSHIP_MAP).unwrap();
        let PropertyValue::Map { entries, .. } = &map.value else {
            panic!("relationship map expected");
        };
        assert!(
            entries
                .iter()
                .any(|(key, _)| { map_key_to_string(key).as_deref() == Some(id) })
        );
        assert_eq!(
            personal_relationships_by_id(&root).get(id),
            Some(&PersonalRelationship::Neutral)
        );
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

    /// A `SetProperty<NameProperty>` named `name` carrying `values` (inline
    /// FName elements). Mirrors `properties::name_set_property`.
    fn name_set_property(name: &str, values: &[&str]) -> Vec<u8> {
        let mut body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        body.extend_from_slice(&(values.len() as u32).to_le_bytes());
        for v in values {
            body.extend_from_slice(&fstring(v));
        }
        let mut out = fstring(name);
        out.extend_from_slice(&fstring("SetProperty"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("NameProperty")); // element type
        out.extend_from_slice(&0u32.to_le_bytes()); // array_index
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.push(0); // tag_flags
        out.extend_from_slice(&body);
        out
    }

    /// The `CharacterKnowledgeByUniqueName` MapProperty<NameProperty,
    /// StructProperty(KnowledgeSet)> keyed by UniqueName. Each value is an inline
    /// struct proplist holding one (empty) `Knowledge` name set, terminated by
    /// "None" — exactly the real-save shape (`list_characters` only reads the map
    /// KEYS via `map_key_to_string`, but a faithful value keeps the parse honest).
    /// Mirrors `properties::knowledge_map_property` in `npc.rs` inline style.
    fn knowledge_map(unique_names: &[&str]) -> Vec<u8> {
        let empty_value = || {
            let mut v = name_set_property("Knowledge", &[]);
            v.extend_from_slice(&fstring("None"));
            v
        };
        let mut body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        body.extend_from_slice(&(unique_names.len() as u32).to_le_bytes()); // count
        for name in unique_names {
            body.extend_from_slice(&fstring(name)); // inline Name key
            body.extend_from_slice(&empty_value()); // inline struct value
        }

        let mut out = fstring("CharacterKnowledgeByUniqueName");
        out.extend_from_slice(&fstring("MapProperty"));
        out.extend_from_slice(&2u32.to_le_bytes()); // descriptor count
        out.extend_from_slice(&fstring("NameProperty")); // key type
        out.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        out.extend_from_slice(&fstring("StructProperty")); // value type
        out.extend_from_slice(&1u32.to_le_bytes()); // struct descriptor count
        out.extend_from_slice(&fstring("KnowledgeSet")); // value struct type
        out.extend_from_slice(&1u32.to_le_bytes()); // package count
        out.extend_from_slice(&fstring("/Script/G1R")); // package
        out.extend_from_slice(&0u32.to_le_bytes()); // array_index
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.push(0); // tag_flags
        out.extend_from_slice(&body);
        out
    }

    /// A private root holding BOTH a `CharacterStateSaveGameData_Attributes` map
    /// (one enumerable actor per GlobalId, so `list_npcs` returns them) AND a
    /// `CharacterKnowledgeByUniqueName` map keyed by the given UniqueNames — the
    /// two inputs `list_characters` joins on the `char_key` prefix rule.
    fn build_root_with_actor_and_knowledge(
        actor_global_ids: &[&str],
        knowledge_unique_names: &[&str],
    ) -> RootObject {
        let actors: Vec<(&str, Vec<u8>)> = actor_global_ids
            .iter()
            .map(|id| (*id, attributes_entry_value(100.0, 100.0)))
            .collect();
        let payload = private_root(&[
            character_state_map("AttributesMap", ATTRIBUTES_TYPE, &actors),
            knowledge_map(knowledge_unique_names),
        ]);
        parse_private_root(&payload).unwrap()
    }

    #[test]
    fn char_key_strips_after_first_dash() {
        assert_eq!(char_key("NC_ORG_Lares_801-WP_OC_SPAWN"), "NC_ORG_Lares_801");
        assert_eq!(char_key("Hero"), "Hero");
        assert_eq!(char_key("A-B-C"), "A");
    }

    #[test]
    fn list_characters_flags_and_orphans() {
        // One enumerable actor whose knowledge entry matches by the char_key prefix
        // rule, plus a knowledge UniqueName with NO actor (an orphan row).
        let root = build_root_with_actor_and_knowledge(
            &["NC_ORG_Lares_801-WP_X"], // actor GlobalIds (dashed -> prefix is the key)
            &["NC_ORG_Lares_801", "ST_VLK_Mud_Sleeper"], // knowledge UniqueNames
        );
        let chars = list_characters(&root).unwrap();

        // The actor row: global_id preserved verbatim, unique_name = the actual
        // stored knowledge key in ORIGINAL case, knowledge flag set because the map
        // has its charKey.
        let lares = chars
            .iter()
            .find(|c| c.global_id.as_deref() == Some("NC_ORG_Lares_801-WP_X"))
            .unwrap();
        assert_eq!(lares.unique_name, "NC_ORG_Lares_801");
        assert!(lares.has_knowledge);
        // No long-term-memory / inventory maps in this fixture => both false.
        assert!(!lares.has_events);
        assert!(!lares.has_inventory);

        // The orphan row: a knowledge UniqueName with no matching actor charKey is
        // appended with global_id: None (original case), only has_knowledge true.
        let orphan = chars
            .iter()
            .find(|c| c.unique_name == "ST_VLK_Mud_Sleeper")
            .unwrap();
        assert!(orphan.global_id.is_none());
        assert!(orphan.has_knowledge && !orphan.has_events && !orphan.has_inventory);
        assert!(!orphan.is_dead);

        // Exactly the two rows (one actor + one orphan); the actor's own charKey is
        // NOT double-counted as an orphan.
        assert_eq!(chars.len(), 2, "one actor row + one orphan row");
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
                    &[
                        "State.KillBountyGranted",
                        "State.ExecutedBountyGranted",
                        "State.Dead",
                    ],
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
        assert!(
            k.is_dead,
            "State.Dead in LooseTags => dead even with positive HP"
        );
        assert_eq!(k.hp, Some(60.0));
        let d = npcs.iter().find(|n| n.id == defeated_only).unwrap();
        assert!(
            !d.is_dead,
            "loose tags without State.Dead => ALIVE (merely defeated)"
        );
        let z = npcs.iter().find(|n| n.id == zero_hp_no_tags).unwrap();
        assert!(
            !z.is_dead,
            "HP 0 with no loose-tags entry is alive (HP is not the signal)"
        );
        let m = npcs
            .iter()
            .find(|n| n.id == kill_memory_no_dead_tag)
            .unwrap();
        assert!(
            !m.is_dead,
            "kill memory alone (no State.Dead loose tag) is NOT dead"
        );
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
        let n0 = list_npcs(&before)
            .unwrap()
            .into_iter()
            .find(|n| n.id == id)
            .unwrap();
        assert!(n0.is_dead, "NPC starts dead (State.Dead in LooseTags)");
        assert_eq!(n0.hp, Some(0.0));

        apply_revive(&mut payload, id).unwrap();

        let root = parse_private_root(&payload).unwrap();
        assert_eq!(root.consumed, payload.len(), "payload must re-parse fully");
        let n = list_npcs(&root)
            .unwrap()
            .into_iter()
            .find(|n| n.id == id)
            .unwrap();
        assert!(!n.is_dead, "revived NPC is no longer dead");
        assert_eq!(n.hp, Some(80.0), "HP restored to MaxHealth base");
        assert_eq!(n.max_hp, Some(80.0));
        // The three dead loose tags are gone; the unrelated loose tag survives.
        let tags = loose_tags(&root, id).expect("loose-tags entry still present");
        for dead in DEAD_LOOSE_TAGS {
            assert!(
                !tags.iter().any(|t| t == dead),
                "dead loose tag {dead} must be gone"
            );
        }
        assert!(
            tags.iter().any(|t| t == "State.Aggro"),
            "unrelated loose tag must survive"
        );
        // Every revive memory tag is gone; the unrelated quest event survives.
        for tag in REVIVE_EVENT_TAGS {
            assert!(
                !memory_has_tag(&root, id, tag),
                "revive tag {tag} must be gone"
            );
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
        entries.iter().any(|(k, _v)| {
            map_key_to_string(k)
                .as_deref()
                .is_some_and(|k| is_corpse_key_for(k, id))
        })
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
                    &[
                        "State.KillBountyGranted",
                        "State.ExecutedBountyGranted",
                        "State.Dead",
                        "State.Aggro",
                    ],
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
        assert_eq!(
            root.consumed,
            payload.len(),
            "payload must re-parse fully (byte-clean)"
        );

        // 1. NPC's own death residue is gone, but its unrelated memory — and a
        //    kill it committed against ANOTHER character — survive.
        let npc_mem = long_term_memory_value(&root, id).unwrap();
        let PropertyValue::Array {
            elements: npc_events,
        } = struct_member(npc_mem, "MemorizedEvents").unwrap()
        else {
            panic!("NPC MemorizedEvents not an array");
        };
        for tag in REVIVE_EVENT_TAGS {
            let residue = npc_events.iter().any(|e| {
                event_has_any_tag(e, &[tag]) && affected_character_id(e).map_or(true, |a| a == id)
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
            npc_events
                .iter()
                .any(|e| event_has_any_tag(e, &["Memory.Execution"])
                    && affected_character_id(e) == Some(other)),
            "NPC's memory of executing a DIFFERENT character must survive"
        );

        // 2. Hero's kill-about-id event gone; unrelated Hero memories survive.
        let hero_mem = long_term_memory_value(&root, hero).unwrap();
        let PropertyValue::Array { elements } = struct_member(hero_mem, "MemorizedEvents").unwrap()
        else {
            panic!("hero MemorizedEvents not an array");
        };
        assert_eq!(
            elements.len(),
            2,
            "exactly the one kill-about-id event was removed"
        );
        // No kill-tagged event affecting `id` remains in ANY owner.
        for tag in KILL_EVENT_TAGS {
            let kill_about_id = memory_owners(&root).iter().any(|(_owner, mem)| {
                let Some(PropertyValue::Array { elements }) = struct_member(mem, "MemorizedEvents")
                else {
                    return false;
                };
                elements
                    .iter()
                    .any(|e| event_has_any_tag(e, &[tag]) && affected_character_id(e) == Some(id))
            });
            assert!(
                !kill_about_id,
                "no {tag} event affecting {id} may remain in any owner"
            );
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
        assert!(
            !is_dead_by_loose_tags(&root, id),
            "no State.Dead => not dead"
        );

        // HP restored to max.
        let n = list_npcs(&root)
            .unwrap()
            .into_iter()
            .find(|n| n.id == id)
            .unwrap();
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
                &[
                    "State.KillBountyGranted",
                    "State.ExecutedBountyGranted",
                    "State.Dead",
                ],
            )]),
        ]);

        let before = parse_private_root(&payload).unwrap();
        assert!(
            is_dead_by_loose_tags(&before, id),
            "State.Dead loose tag => dead"
        );
        let n0 = list_npcs(&before)
            .unwrap()
            .into_iter()
            .find(|n| n.id == id)
            .unwrap();
        assert!(n0.is_dead);

        apply_revive(&mut payload, id).unwrap();

        let root = parse_private_root(&payload).unwrap();
        assert_eq!(root.consumed, payload.len(), "byte-clean re-parse");
        // All three dead tags gone; the (now-empty) container entry remains parseable.
        let tags = loose_tags(&root, id).expect("loose-tags entry present");
        assert!(
            tags.is_empty(),
            "all dead tags stripped; container now empty"
        );
        assert!(!is_dead_by_loose_tags(&root, id));
        let n = list_npcs(&root)
            .unwrap()
            .into_iter()
            .find(|n| n.id == id)
            .unwrap();
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
        assert_eq!(
            payload, snapshot,
            "no defeat events + full HP => byte-identical"
        );
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
        let n = list_npcs(&root)
            .unwrap()
            .into_iter()
            .find(|n| n.id == id)
            .unwrap();
        assert!(!n.is_dead);
        assert_eq!(
            n.hp,
            Some(80.0),
            "HP restored to max even when it was positive"
        );
    }

    #[test]
    fn corpse_key_matches_exact_and_numeric_suffix() {
        let id = "OM_GRD_Drake_260-WorldPointActor_Drake";
        assert!(is_corpse_key_for(&format!("Character_{id}"), id), "exact");
        assert!(
            is_corpse_key_for(&format!("Character_{id}_2146328221"), id),
            "numeric spawn suffix"
        );
        assert!(
            !is_corpse_key_for(&format!("Character_{id}_abc"), id),
            "non-digit suffix"
        );
        assert!(
            !is_corpse_key_for(&format!("Character_{id}X"), id),
            "no underscore"
        );
        assert!(
            !is_corpse_key_for(&format!("Character_{id}_"), id),
            "empty suffix"
        );
        assert!(
            !is_corpse_key_for("Character_OtherNpc-1", id),
            "different id"
        );
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
        let n = list_npcs(&root)
            .unwrap()
            .into_iter()
            .find(|n| n.id == id)
            .unwrap();
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
                let Some(PropertyValue::Array { elements }) = struct_member(mem, "MemorizedEvents")
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
