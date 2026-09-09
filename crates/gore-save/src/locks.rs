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

/// The event whose magnitude carries the door's section: 1 opens, 0 shuts.
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
    /// Whether the name still has to be appended there.
    pub closed_needs_entry: bool,
    /// Paths of every saved-message magnitude that still says "open".
    pub open_message_magnitudes: Vec<Vec<String>>,
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
    if SHARED_DOOR_BUCKETS
        .iter()
        .any(|bucket| bucket.eq_ignore_ascii_case(name))
    {
        return Err(CoreError::UnsupportedEdit(format!(
            "{name:?} names a whole class of doors, not one door; refusing to \
             shut every door that shares it"
        )));
    }
    let Some((open_path, open_elements)) = array_elements(root, DOORS_OPEN_PROPERTY) else {
        return Ok(None);
    };
    let open_indices: Vec<usize> = open_elements
        .iter()
        .enumerate()
        .filter(|(_, element)| element_is(element, name))
        .map(|(index, _)| index)
        .collect();
    if open_indices.is_empty() {
        return Ok(None);
    }

    let (closed_path, closed_needs_entry) = match array_elements(root, DOORS_CLOSED_PROPERTY) {
        Some((path, elements)) => {
            let present = elements.iter().any(|element| element_is(element, name));
            (Some(path), !present)
        }
        None => (None, false),
    };

    // A door that carries a saved verb message would have it replayed on load;
    // if that message still says section 1, the leaf swings back open however
    // the arrays read. Zero it for this door only.
    let mut open_message_magnitudes = Vec::new();
    if let (Some((_, names)), Some((structs_path, structs))) = (
        array_elements(root, DOOR_MESSAGE_NAMES_PROPERTY),
        array_elements(root, DOOR_MESSAGE_STRUCTS_PROPERTY),
    ) {
        for (index, entry) in names.iter().enumerate() {
            if !element_is(entry, name) {
                continue;
            }
            // The two arrays are parallel; a save whose lengths disagree is not
            // one this can reason about, so it is left alone.
            let Some(message) = structs.get(index).and_then(struct_properties) else {
                continue;
            };
            let is_section_event = message
                .iter()
                .any(|p| p.name.as_str() == "m_Event" && element_is(&p.value, DOOR_SECTION_EVENT));
            if !is_section_event {
                continue;
            }
            let still_open = message.iter().any(|p| {
                p.name.as_str() == "m_Magnitude"
                    && matches!(p.value, PropertyValue::Double(value) if value != 0.0)
            });
            if !still_open {
                continue;
            }
            let mut path = structs_path.clone();
            path.push(format!("[{index}]"));
            path.push("m_Magnitude".to_string());
            open_message_magnitudes.push(path);
        }
    }

    Ok(Some(DoorLeafPlan {
        open_path,
        open_indices,
        closed_path,
        closed_needs_entry,
        open_message_magnitudes,
    }))
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
        assert!(plan.closed_needs_entry);
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
        assert!(!plan.closed_needs_entry);
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
