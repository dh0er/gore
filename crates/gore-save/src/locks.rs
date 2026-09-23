//! Chest and door locks.
//!
//! A save carries one bucket for them, `m_GenericData["LockPersistentData"]`,
//! and that bucket has exactly one field:
//!
//! ```text
//! LockPersistentData.m_UnlockedLocks : SetProperty<NameProperty>
//! ```
//!
//! The set holds the name of every lock the player has already opened —
//! nothing else. A fresh save carries the set with zero elements, so the
//! property is always there to write into; what grows is its contents.
//!
//! Chest locks and door locks live in the SAME set and the game does not
//! separate them here: `IO_OC_CHEST_DEXTER` sits next to `OC_Santino_Door`.
//! The name is the lock-bearing class's `m_UniqueName`, which is what the
//! bundled lock catalog is keyed by; the class's own `m_Lock` id is a
//! different string and never appears in a save.
//!
//! Removing a name locks that chest or door again. For a door that is only
//! half the story: `GameStateDataBase.m_DoorsOpen` carries the leaf state
//! separately, so a door whose lock is restored while its name stays in
//! `m_DoorsOpen` would be a locked door standing open. [`plan_close_door_leaf`]
//! is what shuts it, mirroring the game's own `Server_SetDoorOpenState(false)`.

use serde::Serialize;

use crate::CoreError;
use crate::properties::{self, PropertyValue, RootObject};

/// `m_GenericData` key of the bucket holding the unlocked-lock set.
pub const LOCK_BUCKET_KEY: &str = "LockPersistentData";

/// The one field in that bucket.
pub const UNLOCKED_LOCKS_PROPERTY: &str = "m_UnlockedLocks";

/// `GameStateDataBase.m_DoorsOpen` — the leaf state, not the lock. The game
/// appends (not add-unique) on every open, so one door occurs many times.
const DOORS_OPEN_PROPERTY: &str = "m_DoorsOpen";

/// Its counterpart. `AG1RGameState::IsDoorOpen` scans `m_DoorsOpen` FIRST and
/// returns true on a hit, so adding a name here while it is still in
/// `m_DoorsOpen` changes nothing — a door is shut only by doing both.
const DOORS_CLOSED_PROPERTY: &str = "m_DoorsClosed";

/// Parallel arrays: a door with a saved verb message replays it on load.
const DOOR_MESSAGE_NAMES_PROPERTY: &str = "m_SavedDoorsMessagesName";
const DOOR_MESSAGE_STRUCTS_PROPERTY: &str = "m_SavedDoorsMessagesStruct";

/// The event whose magnitude is the door's section. The listener converts it
/// with truncation and skips the mesh update when that step is already the
/// component's spawn step (0). A stored 0 therefore leaves an open mesh where
/// it was. A door the player has shut simply has no message.
const DOOR_SECTION_EVENT: &str = "m_CurrentSection";

/// Door entries keyed by a CLASS rather than by one door's unique name.
///
/// A door with no `m_UniqueName` falls back to its class, so these buckets each
/// stand for many doors at once (39, 42 and 4 copies in one real save). No lock
/// is named after one — the catalog is keyed by `m_UniqueName` — so this can
/// only be reached by a future entry that should not have been there; refusing
/// beats shutting every generic door in the game.
const SHARED_DOOR_BUCKETS: [&str; 3] = [
    "GenericDoor",
    "GenericDoorSavedState",
    "GenericDoorNotPlayer",
];

/// Everything a payload must change so one door's leaf reads as shut.
///
/// Mirrors exactly what the game's own `Server_SetDoorOpenState(false, name)`
/// writes, so the result is a state the engine itself produces rather than a
/// combination it never makes.
#[derive(Debug, Clone)]
pub struct DoorLeafPlan {
    /// Addressable path of `m_DoorsOpen`.
    pub open_path: Vec<String>,
    /// EVERY index holding this door's name. The game appends without a
    /// uniqueness check, so a much-used door has dozens of copies and removing
    /// one of them leaves the door open.
    pub open_indices: Vec<usize>,
    /// Addressable path of `m_DoorsClosed`, when the save has that array.
    pub closed_path: Option<Vec<String>>,
    /// Names that still have to be appended to `m_DoorsClosed`, in the spelling
    /// the open list used. One door is often recorded there under a second
    /// name (`OC_Guards_Cell_01_Door` in the lock set, `OC_Guards_Cell_01` in
    /// the leaf arrays); each of those has to be shut or `IsDoorOpen` still
    /// hits the leftover copy.
    pub closed_names: Vec<String>,
    /// Addressable path of `m_SavedDoorsMessagesName`, when that array exists.
    pub message_names_path: Option<Vec<String>>,
    /// Addressable path of `m_SavedDoorsMessagesStruct`. Same indices as the names.
    pub message_structs_path: Option<Vec<String>>,
    /// Section messages for this door. They are removed, not zeroed: a magnitude
    /// of 0 replays as step 0, the component already starts at step 0, and the
    /// mesh update is skipped, so the leaf stays visually open.
    pub message_indices: Vec<usize>,
}

/// Case-insensitive name/string equality, matching UE `FName` semantics.
fn element_is(element: &PropertyValue, name: &str) -> bool {
    match element {
        PropertyValue::Name(value) | PropertyValue::Str(value) => value.eq_ignore_ascii_case(name),
        _ => false,
    }
}

fn array_elements<'a>(
    root: &'a RootObject,
    property: &str,
) -> Option<(Vec<String>, &'a [PropertyValue])> {
    let (path, found) = properties::find_property_by_name(root, property)?;
    match &found.value {
        PropertyValue::Array { elements } => Some((path, elements.as_slice())),
        _ => None,
    }
}

/// What it takes to shut `name`'s leaf, or `None` when there is nothing to do —
/// a chest (chests never appear in `m_DoorsOpen`) or a door already shut.
///
/// Reads only; the caller applies the plan.
pub fn plan_close_door_leaf(
    root: &RootObject,
    name: &str,
) -> Result<Option<DoorLeafPlan>, CoreError> {
    if is_shared_door_bucket(name) {
        return Err(CoreError::UnsupportedEdit(format!(
            "{name:?} names a whole class of doors, not one door; refusing to \
             shut every door that shares it"
        )));
    }
    let Some((open_path, open_elements)) = array_elements(root, DOORS_OPEN_PROPERTY) else {
        return Ok(None);
    };

    // The lock set, the leaf arrays and the replayed section message do not
    // always use the same string. A locked door's unique name is
    // `OC_Guards_Cell_01_Door`, but the leaf is stored as `OC_Guards_Cell_01`;
    // a section message may be stored as `U` + the class or with a `Trigger`
    // suffix. `GlobalResendSavedDoorMessages` replays a magnitude of 1 as
    // "open" after load, so leaving that message (or the alias still sitting
    // in `m_DoorsOpen`) puts the leaf back open on top of a lock that is shut.
    let mut names_to_clear = leaf_aliases(name);

    let (closed_path, closed_elements) = match array_elements(root, DOORS_CLOSED_PROPERTY) {
        Some((path, elements)) => (Some(path), Some(elements)),
        None => (None, None),
    };

    let mut message_names_path = None;
    let mut message_structs_path = None;
    let mut message_indices = Vec::new();
    let message_names = array_elements(root, DOOR_MESSAGE_NAMES_PROPERTY);
    let message_structs = array_elements(root, DOOR_MESSAGE_STRUCTS_PROPERTY);
    if message_names
        .as_ref()
        .map_or(0, |(_, entries)| entries.len())
        != message_structs
            .as_ref()
            .map_or(0, |(_, entries)| entries.len())
    {
        return Err(CoreError::UnsupportedEdit(format!(
            "cannot relock {name:?}: saved door-message name and struct arrays have different lengths"
        )));
    }
    if let (Some((names_path, names)), Some((structs_path, structs))) =
        (message_names, message_structs)
    {
        for (index, entry) in names.iter().enumerate() {
            let Some(message) = structs.get(index).and_then(struct_properties) else {
                continue;
            };
            let trigger = message
                .iter()
                .find(|p| p.name.as_str() == "m_ConnectedTrigger");
            let mentions_door = names_to_clear.iter().any(|alias| element_is(entry, alias))
                || trigger.is_some_and(|property| {
                    names_to_clear
                        .iter()
                        .any(|alias| element_is(&property.value, alias))
                });
            if !mentions_door {
                continue;
            }
            if let Some(spelling) = element_name(entry) {
                remember_name(&mut names_to_clear, spelling);
            }
            let is_section_event = message
                .iter()
                .any(|p| p.name.as_str() == "m_Event" && element_is(&p.value, DOOR_SECTION_EVENT));
            if !is_section_event {
                continue;
            }
            message_names_path = Some(names_path.clone());
            message_structs_path = Some(structs_path.clone());
            message_indices.push(index);
        }
    }

    let mut open_indices = Vec::new();
    let mut closed_names = Vec::new();
    for (index, element) in open_elements.iter().enumerate() {
        if !names_to_clear
            .iter()
            .any(|alias| element_is(element, alias))
        {
            continue;
        }
        open_indices.push(index);
        let Some(spelling) = element_name(element) else {
            continue;
        };
        let already_closed = closed_elements.is_some_and(|elements| {
            elements
                .iter()
                .any(|element| element_is(element, &spelling))
        });
        let already_queued = closed_names
            .iter()
            .any(|queued: &String| queued.eq_ignore_ascii_case(&spelling));
        if !already_closed && !already_queued {
            closed_names.push(spelling);
        }
    }

    if open_indices.is_empty() && message_indices.is_empty() {
        return Ok(None);
    }

    Ok(Some(DoorLeafPlan {
        open_path,
        open_indices,
        closed_path,
        closed_names,
        message_names_path,
        message_structs_path,
        message_indices,
    }))
}

fn is_shared_door_bucket(name: &str) -> bool {
    SHARED_DOOR_BUCKETS
        .iter()
        .any(|bucket| bucket.eq_ignore_ascii_case(name))
}

fn element_name(element: &PropertyValue) -> Option<String> {
    match element {
        PropertyValue::Name(value) | PropertyValue::Str(value) => Some(value.clone()),
        _ => None,
    }
}

fn remember_name(names: &mut Vec<String>, candidate: String) {
    if candidate.is_empty() || is_shared_door_bucket(&candidate) {
        return;
    }
    if names
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(&candidate))
    {
        return;
    }
    names.push(candidate);
}

/// Case-insensitive suffix strip that keeps the original spelling of the stem.
fn strip_ci<'a>(name: &'a str, suffix: &str) -> Option<&'a str> {
    if name.len() <= suffix.len() {
        return None;
    }
    let stem = &name[..name.len() - suffix.len()];
    if name[stem.len()..].eq_ignore_ascii_case(suffix) {
        Some(stem)
    } else {
        None
    }
}

/// Every string this one door is known to be stored under.
///
/// The stem of a `_Door` suffix is included only when that stem is not itself
/// another `_Door` name: `OC_Guards_Cell_01_Door` shuts `OC_Guards_Cell_01`,
/// but `OC_Cellar_Door_02` must not also shut `OC_Cellar_Door`.
///
/// The placed actor is a different string again. The lock set and the leaf
/// arrays use `m_UniqueName` (`OC_Prison_Dungeons_Cell_06_Door`); the section
/// message `GlobalResendSavedDoorMessages` replays is the level actor
/// (`IO_OC_NormalDoor_C_UAID_...`). That pairing is not in the save. It is the
/// interaction spot whose name is the unique name, from the game's
/// `InteractionSpots.json`, baked into [`placed_actor_names`].
fn leaf_aliases(name: &str) -> Vec<String> {
    let mut names = Vec::new();
    remember_name(&mut names, name.to_string());
    if let Some(stem) = strip_ci(name, "_Door") {
        if strip_ci(stem, "_Door").is_none() {
            remember_name(&mut names, stem.to_string());
        }
    }
    if let Some(stem) = strip_ci(name, "Trigger") {
        remember_name(&mut names, stem.to_string());
    }
    if !name.starts_with('U') && !name.starts_with('u') {
        remember_name(&mut names, format!("U{name}"));
    }
    remember_name(&mut names, format!("{name}Trigger"));
    for actor in placed_actor_names(name) {
        remember_name(&mut names, actor.clone());
    }
    names
}

/// Level actor that owns this door's saved section message.
///
/// Keyed by `m_UniqueName`. Values are the object name of
/// `actorToInteractWith` on the interaction spot of the same name. A door
/// with no such spot (no leaf message under a different name) is absent.
fn placed_actor_names(name: &str) -> &'static [String] {
    use std::collections::HashMap;
    use std::sync::LazyLock;

    static ACTORS: LazyLock<HashMap<String, Vec<String>>> = LazyLock::new(|| {
        let raw: HashMap<String, Vec<String>> =
            serde_json::from_str(include_str!("door_leaf_actors.json"))
                .expect("door_leaf_actors.json is a unique-name to actor-name map");
        raw.into_iter()
            .map(|(key, actors)| (key.to_ascii_lowercase(), actors))
            .collect()
    });
    ACTORS
        .get(&name.to_ascii_lowercase())
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn struct_properties(element: &PropertyValue) -> Option<&[properties::Property]> {
    match element {
        PropertyValue::Struct(properties::StructValue::Properties(props)) => Some(props.as_slice()),
        PropertyValue::Struct(properties::StructValue::Instanced(Some(inner))) => {
            Some(inner.properties.as_slice())
        }
        _ => None,
    }
}

/// What the editor needs to know about a save's locks.
#[derive(Debug, Clone, Serialize)]
pub struct LocksSummary {
    /// Every name in `m_UnlockedLocks`, in the order the save stores them.
    pub unlocked: Vec<String>,
    /// Whether the set was found and carries an element type this crate can
    /// splice. False means the panel must stay read-only rather than offer an
    /// edit the write path would refuse.
    pub writable: bool,
}

/// The `m_UnlockedLocks` property, with the addressable path that reaches it.
///
/// Looked up by name rather than by walking to the bucket first: the property
/// name is unique in the tree, and a by-name lookup does not assume where in
/// `m_GenericData` the bucket happens to sit.
fn unlocked_locks<'a>(root: &'a RootObject) -> Option<(Vec<String>, &'a properties::Property)> {
    properties::find_property_by_name(root, UNLOCKED_LOCKS_PROPERTY)
}

/// Whether the set can be spliced: [`properties::ContainerEdit::SetAdd`] and
/// `SetRemove` accept `Name` and `Str` element types and nothing else, so a
/// set of anything else must be reported as read-only instead of failing at
/// save time.
fn set_is_writable(property: &properties::Property) -> bool {
    if property.type_name.as_str() != "SetProperty" {
        return false;
    }
    let Some(inner) = property.descriptor.inner.as_deref() else {
        return false;
    };
    matches!(inner.type_name.as_str(), "NameProperty" | "StrProperty")
}

/// Read the unlocked-lock names out of a parsed private root.
///
/// A save without the bucket is not an error: it answers with an empty list
/// and `writable: false`, which is what an older or truncated save looks like.
pub fn list_locks(root: &RootObject) -> LocksSummary {
    let Some((_, property)) = unlocked_locks(root) else {
        return LocksSummary {
            unlocked: Vec::new(),
            writable: false,
        };
    };
    let PropertyValue::Set { elements, .. } = &property.value else {
        return LocksSummary {
            unlocked: Vec::new(),
            writable: false,
        };
    };
    let unlocked = elements
        .iter()
        .filter_map(|element| match element {
            PropertyValue::Name(value) | PropertyValue::Str(value) => Some(value.clone()),
            _ => None,
        })
        .collect();
    LocksSummary {
        unlocked,
        writable: set_is_writable(property),
    }
}

/// Stored spellings of `name` in the set, plus the path that addresses the
/// set. `None` for the path means the save has no such set at all.
///
/// Lock IDs compare case-insensitively, matching UE `FName` semantics and the
/// editor. Preserve each stored spelling for removal: string sets compare
/// case-sensitively and can hold multiple case variants of the same lock ID.
pub fn lock_snapshot(
    payload: &[u8],
    name: &str,
) -> Result<(Vec<String>, Option<Vec<properties::PathSeg>>), CoreError> {
    let root = properties::parse_private_root(payload)?;
    let Some((path, property)) = unlocked_locks(&root) else {
        return Ok((Vec::new(), None));
    };
    let PropertyValue::Set { elements, .. } = &property.value else {
        return Err(CoreError::Parse(format!(
            "{UNLOCKED_LOCKS_PROPERTY} is not a SetProperty"
        )));
    };
    if !set_is_writable(property) {
        return Err(CoreError::UnsupportedEdit(format!(
            "{UNLOCKED_LOCKS_PROPERTY} holds elements this build cannot splice"
        )));
    }
    let stored_names = elements
        .iter()
        .filter_map(|element| match element {
            PropertyValue::Name(value) | PropertyValue::Str(value)
                if value.eq_ignore_ascii_case(name) =>
            {
                Some(value.clone())
            }
            _ => None,
        })
        .collect();
    Ok((stored_names, Some(properties::parse_path(&path)?)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fstring(value: &str) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&((value.len() + 1) as i32).to_le_bytes());
        out.extend_from_slice(value.as_bytes());
        out.push(0);
        out
    }

    /// A private root carrying one `m_UnlockedLocks` set, built the way the
    /// property tests build theirs: raw bytes through the real parser, so the
    /// descriptor and element types are the ones a save would have.
    fn payload_with_locks(inner_type: &str, names: &[&str]) -> Vec<u8> {
        let mut set_body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        set_body.extend_from_slice(&(names.len() as u32).to_le_bytes());
        for name in names {
            set_body.extend_from_slice(&fstring(name));
        }

        let mut props = fstring(UNLOCKED_LOCKS_PROPERTY);
        props.extend_from_slice(&fstring("SetProperty"));
        props.extend_from_slice(&1u32.to_le_bytes()); // inner descriptor present
        props.extend_from_slice(&fstring(inner_type));
        props.extend_from_slice(&0u32.to_le_bytes()); // array_index
        props.extend_from_slice(&(set_body.len() as u32).to_le_bytes());
        props.push(0); // tag flags
        props.extend_from_slice(&set_body);

        let mut out = fstring("/Script/G1R.GothicLockSaveGameData");
        out.push(0); // object flag
        out.extend_from_slice(&props);
        out.extend_from_slice(&fstring("None"));
        out.extend_from_slice(&0u32.to_le_bytes()); // footer
        out
    }

    /// A `SetProperty`/`ArrayProperty` of names, tag and all.
    fn name_container(kind: &str, property: &str, names: &[&str]) -> Vec<u8> {
        let mut body = Vec::new();
        if kind == "SetProperty" {
            body.extend_from_slice(&0u32.to_le_bytes()); // num_to_remove
        }
        body.extend_from_slice(&(names.len() as u32).to_le_bytes());
        for name in names {
            body.extend_from_slice(&fstring(name));
        }
        let mut out = fstring(property);
        out.extend_from_slice(&fstring(kind));
        out.extend_from_slice(&1u32.to_le_bytes()); // inner descriptor present
        out.extend_from_slice(&fstring("NameProperty"));
        out.extend_from_slice(&0u32.to_le_bytes()); // array_index
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.push(0); // tag flags
        out.extend_from_slice(&body);
        out
    }

    fn root_of(props: Vec<u8>) -> Vec<u8> {
        let mut out = fstring("/Script/G1R.GameStateDataBaseSaveData");
        out.push(0);
        out.extend_from_slice(&props);
        out.extend_from_slice(&fstring("None"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out
    }

    /// A door state carrying the shape a real save has: the game appends to
    /// `m_DoorsOpen` without a uniqueness check, so one door occurs many times.
    fn door_payload(open: &[&str], closed: &[&str]) -> Vec<u8> {
        let mut props = name_container("ArrayProperty", DOORS_OPEN_PROPERTY, open);
        props.extend_from_slice(&name_container(
            "ArrayProperty",
            DOORS_CLOSED_PROPERTY,
            closed,
        ));
        root_of(props)
    }

    #[test]
    fn closing_a_door_strips_every_copy_of_its_name() {
        let payload = door_payload(
            &["OC_Santino_Door", "CV_Stash_Door", "OC_Santino_Door"],
            &[],
        );
        let root = properties::parse_private_root(&payload).unwrap();
        let plan = plan_close_door_leaf(&root, "OC_Santino_Door")
            .unwrap()
            .expect("the door is open");
        assert_eq!(
            plan.open_indices,
            [0, 2],
            "one removal would leave the door open"
        );
        assert_eq!(plan.closed_names, ["OC_Santino_Door"]);
        assert!(plan.closed_path.is_some());
    }

    #[test]
    fn a_door_already_in_the_closed_list_needs_no_second_entry() {
        let payload = door_payload(&["CV_Stash_Door"], &["CV_Stash_Door"]);
        let root = properties::parse_private_root(&payload).unwrap();
        let plan = plan_close_door_leaf(&root, "CV_Stash_Door")
            .unwrap()
            .expect("still listed as open");
        // m_DoorsOpen is scanned first and wins, so the stale open entry still
        // has to go — but the closed entry must not be duplicated.
        assert_eq!(plan.open_indices, [0]);
        assert!(plan.closed_names.is_empty());
    }

    #[test]
    fn a_chest_has_no_leaf_to_close() {
        let payload = door_payload(&["OC_Santino_Door"], &[]);
        let root = properties::parse_private_root(&payload).unwrap();
        assert!(
            plan_close_door_leaf(&root, "IO_OC_CHEST_DEXTER")
                .unwrap()
                .is_none(),
            "chests never appear in m_DoorsOpen"
        );
    }

    #[test]
    fn a_shared_class_bucket_is_refused_rather_than_shutting_them_all() {
        // These names stand for every door that has no unique name of its own;
        // one save holds 42 copies of GenericDoorSavedState alone.
        let payload = door_payload(&["GenericDoorSavedState", "CV_Stash_Door"], &[]);
        let root = properties::parse_private_root(&payload).unwrap();
        let error = plan_close_door_leaf(&root, "GenericDoorSavedState")
            .expect_err("a class bucket must be refused");
        assert!(
            error.to_string().contains("whole class of doors"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn closing_a_suffixed_lock_also_shuts_the_unsuffixed_leaf_name() {
        // The lock catalog and m_UnlockedLocks say OC_Guards_Cell_01_Door.
        // The leaf arrays in a real save say OC_Guards_Cell_01. Shutting only
        // the lock's spelling leaves every copy of the leaf name in
        // m_DoorsOpen, and IsDoorOpen returns true on the first hit.
        let payload = door_payload(
            &[
                "OC_Guards_Cell_01",
                "OC_Guards_Cell_02",
                "OC_Guards_Cell_01",
            ],
            &[],
        );
        let root = properties::parse_private_root(&payload).unwrap();
        let plan = plan_close_door_leaf(&root, "OC_Guards_Cell_01_Door")
            .unwrap()
            .expect("the unsuffixed leaf is still open");
        assert_eq!(plan.open_indices, [0, 2]);
        assert_eq!(plan.closed_names, ["OC_Guards_Cell_01"]);
    }

    #[test]
    fn a_door_alias_includes_the_level_actor_and_not_its_neighbor() {
        let cell = leaf_aliases("OC_Prison_Dungeons_Cell_06_Door");
        assert!(
            cell.iter()
                .any(|name| { name == "IO_OC_NormalDoor_C_UAID_2CF05D5C3CF171D501_1351102996" })
        );
        let neighbor = leaf_aliases("OC_Prison_Dungeons_Cell_23_Door");
        assert!(
            neighbor
                .iter()
                .all(|name| { name != "IO_OC_NormalDoor_C_UAID_2CF05D5C3CF171D501_1351102996" })
        );
        let fence = leaf_aliases("AbandonedMine_Fence_Door");
        assert!(fence.iter().any(|name| {
            name == "FenceDoor_Abandoned_C_UAID_E89C256C6855633902_1078604137"
        }));
        let altar = leaf_aliases("SunkenTower_Door_02_Altar_A");
        assert!(altar.iter().any(|name| {
            name == "SunkenTower_Door_02_Altar_C_UAID_2CF05D5C3CF14B7E02_1407505779"
        }));
        assert!(altar.iter().all(|name| name != "Sunken_Door_01_C_UAID_2CF05D5C3CF1EB7E02_1534383934"));
        let cell_05 = leaf_aliases("OC_Prison_Dungeons_Cell_05_Door");
        assert!(cell_05.iter().any(|name| {
            name == "IO_OC_NormalDoor_C_UAID_2CF05D5C3CF171D501_1308812995"
        }));
        assert!(cell_05.iter().all(|name| {
            name != "IO_OC_NormalDoor_C_UAID_2CF05D5C3CF171D501_1784882001"
        }));
    }

    #[test]
    fn closing_a_numbered_door_does_not_shut_the_door_it_is_named_after() {
        let payload = door_payload(&["OC_Cellar_Door", "OC_Cellar_Door_02"], &[]);
        let root = properties::parse_private_root(&payload).unwrap();
        let plan = plan_close_door_leaf(&root, "OC_Cellar_Door_02")
            .unwrap()
            .expect("door 02 is open");
        assert_eq!(plan.open_indices, [1]);
        assert_eq!(plan.closed_names, ["OC_Cellar_Door_02"]);
    }

    #[test]
    fn a_door_that_is_already_shut_is_left_alone() {
        let payload = door_payload(&["CV_Stash_Door"], &[]);
        let root = properties::parse_private_root(&payload).unwrap();
        assert!(
            plan_close_door_leaf(&root, "OC_Cellar_Door")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn a_save_without_the_door_arrays_is_not_an_error() {
        let payload = payload_with_locks("NameProperty", &["OC_Santino_Door"]);
        let root = properties::parse_private_root(&payload).unwrap();
        assert!(
            plan_close_door_leaf(&root, "OC_Santino_Door")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn lists_every_name_in_the_set() {
        let payload =
            payload_with_locks("NameProperty", &["IO_OC_CHEST_DEXTER", "OC_Santino_Door"]);
        let root = properties::parse_private_root(&payload).unwrap();
        let summary = list_locks(&root);
        assert_eq!(summary.unlocked, ["IO_OC_CHEST_DEXTER", "OC_Santino_Door"]);
        assert!(summary.writable);
    }

    #[test]
    fn an_empty_set_is_still_writable() {
        let payload = payload_with_locks("NameProperty", &[]);
        let root = properties::parse_private_root(&payload).unwrap();
        let summary = list_locks(&root);
        assert!(summary.unlocked.is_empty());
        assert!(
            summary.writable,
            "a fresh save carries the set with 0 elements"
        );
    }

    #[test]
    fn a_set_of_something_unsplicable_is_reported_read_only() {
        // An int set parses fine but SetAdd/SetRemove refuse it, so the panel
        // must be told the save is read-only here instead of offering a toggle.
        let payload = payload_with_locks("IntProperty", &[]);
        let root = properties::parse_private_root(&payload).unwrap();
        assert!(!list_locks(&root).writable);
    }

    #[test]
    fn membership_is_case_insensitive_like_fname() {
        let payload = payload_with_locks("NameProperty", &["IO_OC_CHEST_DEXTER"]);
        let (stored_names, path) = lock_snapshot(&payload, "io_oc_chest_dexter").unwrap();
        assert_eq!(stored_names, ["IO_OC_CHEST_DEXTER"]);
        assert!(path.is_some());
        let (absent, _) = lock_snapshot(&payload, "IO_OC_CHEST_STONE").unwrap();
        assert!(absent.is_empty());
    }

    #[test]
    fn relocking_removes_stored_case_variants_for_name_and_string_sets() {
        for inner_type in ["NameProperty", "StrProperty"] {
            let names: &[&str] = if inner_type == "StrProperty" {
                &["io_oc_chest_dexter", "IO_OC_CHEST_DEXTER", "OtherLock"]
            } else {
                &["io_oc_chest_dexter", "OtherLock"]
            };
            let mut payload = payload_with_locks(inner_type, names);
            let before = payload.clone();
            let apply = |payload: &mut Vec<u8>, unlocked| {
                crate::apply_private_lock_set_unlocked_to_payload(
                    payload,
                    &crate::PrivateLockSetUnlockedEdit {
                        lock: "IO_OC_CHEST_DEXTER".to_string(),
                        unlocked,
                    },
                )
                .unwrap();
            };
            apply(&mut payload, true);
            assert_eq!(payload, before, "already unlocked: {inner_type}");
            apply(&mut payload, false);
            let root = properties::parse_private_root(&payload).unwrap();
            assert_eq!(list_locks(&root).unlocked, ["OtherLock"]);
            let locked = payload.clone();
            apply(&mut payload, false);
            assert_eq!(payload, locked, "already locked: {inner_type}");
            apply(&mut payload, true);
            let root = properties::parse_private_root(&payload).unwrap();
            assert_eq!(
                list_locks(&root).unlocked,
                ["OtherLock", "IO_OC_CHEST_DEXTER"]
            );
            apply(&mut payload, false);
            assert_eq!(payload, locked, "roundtrip: {inner_type}");
        }
    }
}
