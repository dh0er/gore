//! Undo notes for NPC placement edits.
//!
//! Moving an NPC and making him STAY takes two writes — `CharacterLocation` and
//! `DailyRoutineByGlobalId{id}.DailyRoutineClass = DailyRoutine_Empty` — and the
//! second one destroys information the save no longer holds anywhere: the
//! routine he was on. The class name is not derivable, because the current
//! routine is story state (`..._Collapsed`, `..._WaitYard`, `..._TcWait`), not
//! the `..._Start` the naming convention would suggest. So the editor writes it
//! down before overwriting it.
//!
//! **Beside the save, not inside it.** An in-save marker would travel with the
//! file, but the game re-serializes the whole save from live state the next time
//! the player saves in-game: the marker would be gone while
//! `DailyRoutine_Empty`, being live state by then, survives — losing the undo
//! exactly in the case it exists for. A sidecar survives that, and changes no
//! byte the game has to parse.
//!
//! The file lives next to the backups and is written the same way as
//! [`crate::backup_names_path`]'s label map: staged and swapped, never written
//! in place, and only published while the file is still what was read, so two
//! editors on one save folder cannot drop each other's notes.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{CoreError, FileSnapshot, ScratchFile, begin_replace_if_unchanged, snapshot_file};

/// What one NPC's placement edit replaced, and what it put there.
///
/// Both halves are kept. The `original_*` fields are what a restore writes back;
/// the `written_*` fields are what the save must still hold for that restore to
/// be honest — if the game or another tool has moved the NPC since, the note
/// describes a state that no longer exists and restoring it would silently
/// discard whatever happened in between.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacementNote {
    /// `CharacterLocation` before the move.
    pub original_location: [f64; 3],
    /// `DailyRoutineClass` before the move. `None` when the NPC had no
    /// `DailyRoutineByGlobalId` entry at all — restoring then writes nothing
    /// there, rather than inventing a routine the save never had.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_routine_class: Option<String>,
    /// `CharacterLocation` the move wrote.
    pub written_location: [f64; 3],
    /// `DailyRoutineClass` the move wrote. `None` when the move deliberately
    /// left the routine alone (the NPC was moved but not pinned).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub written_routine_class: Option<String>,
}

/// Notes for one save file: NPC GlobalId to note. `BTreeMap` so the published
/// JSON has a stable key order and a re-write with no change is a no-op file.
pub type SaveNotes = BTreeMap<String, PlacementNote>;

/// Every save in one folder: save file name to that save's notes.
///
/// Keyed by file NAME rather than full path, so the notes survive the folder
/// being moved — the same reason the backup-label map is keyed that way.
pub type FolderNotes = BTreeMap<String, SaveNotes>;

/// Where the notes live: one JSON object in the save folder's
/// `goresave_backups` directory, beside `backup_names.json`.
pub fn notes_path(save_path: &Path) -> PathBuf {
    save_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("goresave_backups")
        .join("npc_placements.json")
}

fn save_key(save_path: &Path) -> String {
    save_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Read every note in `save_path`'s folder. A missing or unreadable file yields
/// an empty map: a note is an undo affordance, never a reason to fail a read.
pub fn read_folder_notes(save_path: &Path) -> FolderNotes {
    let Ok(text) = fs::read_to_string(notes_path(save_path)) else {
        return FolderNotes::new();
    };
    serde_json::from_str::<FolderNotes>(&text).unwrap_or_default()
}

/// The notes recorded for one save.
pub fn read_notes(save_path: &Path) -> SaveNotes {
    read_folder_notes(save_path)
        .remove(&save_key(save_path))
        .unwrap_or_default()
}

/// The notes, with an existing-but-unreadable file reported as an error.
///
/// Anything that REWRITES the file goes through this. The forgiving reader would
/// hand a mutation an empty map for a file it could not parse, and publishing
/// that back would discard every note the file still held.
fn read_folder_notes_strict(save_path: &Path) -> Result<FolderNotes, CoreError> {
    let path = notes_path(save_path);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(FolderNotes::new()),
        Err(err) => return Err(err.into()),
    };
    serde_json::from_str::<FolderNotes>(&text).map_err(|err| {
        CoreError::Parse(format!(
            "{} is not readable NPC-placement JSON: {err}",
            path.display()
        ))
    })
}

fn publish(path: &Path, expected: &FileSnapshot, notes: &FolderNotes) -> Result<(), CoreError> {
    let notes: FolderNotes = notes
        .iter()
        .filter(|(_, save)| !save.is_empty())
        .map(|(key, save)| (key.clone(), save.clone()))
        .collect();
    if notes.is_empty() {
        return remove(path, expected);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(&notes)
        .map_err(|err| CoreError::Parse(format!("cannot encode NPC placements: {err}")))?;
    let staged = ScratchFile::create(path, "tmp-placements", text.as_bytes())?;
    begin_replace_if_unchanged(path, staged.path(), expected)?.commit();
    Ok(())
}

fn remove(path: &Path, expected: &FileSnapshot) -> Result<(), CoreError> {
    if matches!(expected, FileSnapshot::Missing) {
        return Ok(());
    }
    let empty = ScratchFile::create(path, "tmp-placements", &[])?;
    let pending = begin_replace_if_unchanged(path, empty.path(), expected)?;
    pending.commit();
    let _ = fs::remove_file(path);
    Ok(())
}

/// Apply `mutate` to this save's notes and publish the result.
///
/// Read, change, write is a lost update with a second editor open on the same
/// folder, so the publish only goes through while the file is still exactly what
/// was read; a change underneath restarts the sequence on the newer map.
pub fn mutate_notes<F>(save_path: &Path, mutate: F) -> Result<(), CoreError>
where
    F: Fn(&mut SaveNotes),
{
    let path = notes_path(save_path);
    let key = save_key(save_path);
    let mut last_conflict = None;
    for _ in 0..8 {
        let before = snapshot_file(&path)?;
        let mut folder = read_folder_notes_strict(save_path)?;
        let unchanged = folder.clone();
        mutate(folder.entry(key.clone()).or_default());
        if folder == unchanged {
            return Ok(());
        }
        match publish(&path, &before, &folder) {
            Ok(()) => return Ok(()),
            Err(err) => last_conflict = Some(err),
        }
    }
    Err(last_conflict.unwrap_or_else(|| {
        CoreError::Update(format!(
            "NPC placements at {} kept changing underneath",
            path.display()
        ))
    }))
}

/// Record the notes in `records`, replacing any note for the same NPC.
pub fn record(save_path: &Path, records: &[(String, PlacementNote)]) -> Result<(), CoreError> {
    if records.is_empty() {
        return Ok(());
    }
    mutate_notes(save_path, |notes| {
        for (npc, note) in records {
            notes.insert(npc.clone(), note.clone());
        }
    })
}

/// Drop the notes for `npcs`. Dropping one that is not there is not an error —
/// a restore whose note was already cleared has nothing left to do.
pub fn clear(save_path: &Path, npcs: &[String]) -> Result<(), CoreError> {
    if npcs.is_empty() {
        return Ok(());
    }
    mutate_notes(save_path, |notes| {
        for npc in npcs {
            notes.remove(npc);
        }
    })
}

/// Parse the `placementNotes` array of a `write_save` payload.
///
/// Shape: `[{"npc": "<GlobalId>", "note": {...}}]`. A malformed entry is a
/// request error rather than a silently skipped note: a caller that believes it
/// recorded an undo and did not would offer a restore button that cannot work.
pub fn parse_records(value: &Value) -> Result<Vec<(String, PlacementNote)>, CoreError> {
    let Some(entries) = value.as_array() else {
        return Err(CoreError::InvalidRequest(
            "placementNotes must be an array".to_string(),
        ));
    };
    entries
        .iter()
        .map(|entry| {
            let npc = entry
                .get("npc")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    CoreError::InvalidRequest("placementNotes entry needs npc".to_string())
                })?
                .to_string();
            let note = entry.get("note").ok_or_else(|| {
                CoreError::InvalidRequest("placementNotes entry needs note".to_string())
            })?;
            let note: PlacementNote = serde_json::from_value(note.clone()).map_err(|err| {
                CoreError::InvalidRequest(format!("placementNotes entry for {npc} is invalid: {err}"))
            })?;
            // No finite-coordinate guard here on purpose: `serde_json` refuses
            // `Infinity`/`NaN` while parsing the request, so a non-finite
            // coordinate cannot reach this point and a check for one would be
            // unreachable code pretending to be a safety net.
            Ok((npc, note))
        })
        .collect()
}

/// Parse the `clearPlacementNotes` array of a `write_save` payload.
pub fn parse_clears(value: &Value) -> Result<Vec<String>, CoreError> {
    let Some(entries) = value.as_array() else {
        return Err(CoreError::InvalidRequest(
            "clearPlacementNotes must be an array".to_string(),
        ));
    };
    entries
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| {
                    CoreError::InvalidRequest(
                        "clearPlacementNotes entries must be NPC GlobalId strings".to_string(),
                    )
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn note() -> PlacementNote {
        PlacementNote {
            original_location: [1.0, 2.0, 3.0],
            original_routine_class: Some("/Script/Angelscript.DailyRoutine_A_Start".to_string()),
            written_location: [4.0, 5.0, 6.0],
            written_routine_class: Some(
                "/Script/Angelscript.DailyRoutine_Empty".to_string(),
            ),
        }
    }

    #[test]
    fn a_recorded_note_reads_back_for_that_save_only() {
        let dir = tempdir().unwrap();
        let save = dir.path().join("G1R-001.sav");
        let other = dir.path().join("G1R-002.sav");
        record(&save, &[("Npc-A".to_string(), note())]).unwrap();

        assert_eq!(read_notes(&save).get("Npc-A"), Some(&note()));
        assert!(read_notes(&other).is_empty());
    }

    #[test]
    fn clearing_the_last_note_removes_the_file_rather_than_leaving_an_empty_object() {
        let dir = tempdir().unwrap();
        let save = dir.path().join("G1R-001.sav");
        record(&save, &[("Npc-A".to_string(), note())]).unwrap();
        clear(&save, &["Npc-A".to_string()]).unwrap();

        assert!(read_notes(&save).is_empty());
        assert!(!notes_path(&save).exists());
    }

    #[test]
    fn one_save_s_notes_survive_another_save_s_write() {
        let dir = tempdir().unwrap();
        let first = dir.path().join("G1R-001.sav");
        let second = dir.path().join("G1R-002.sav");
        record(&first, &[("Npc-A".to_string(), note())]).unwrap();
        record(&second, &[("Npc-B".to_string(), note())]).unwrap();

        assert_eq!(read_notes(&first).keys().collect::<Vec<_>>(), ["Npc-A"]);
        assert_eq!(read_notes(&second).keys().collect::<Vec<_>>(), ["Npc-B"]);
    }

    #[test]
    fn a_note_belongs_to_the_file_the_bytes_landed_in() {
        // An export writes the moved bytes to `outputPath` and leaves the source
        // alone, so the note belongs beside the export. Recording it against the
        // source would leave the exported save with no undo and hand an
        // untouched file a note for a move it does not contain.
        let dir = tempdir().unwrap();
        let source = dir.path().join("G1R-001.sav");
        let export = dir.path().join("exported").join("G1R-009.sav");
        fs::create_dir_all(export.parent().unwrap()).unwrap();
        record(&export, &[("Npc-A".to_string(), note())]).unwrap();

        assert_eq!(read_notes(&export).get("Npc-A"), Some(&note()));
        assert!(read_notes(&source).is_empty());
        assert!(!notes_path(&source).exists());
    }

    #[test]
    fn an_unreadable_file_blocks_a_write_instead_of_being_overwritten() {
        let dir = tempdir().unwrap();
        let save = dir.path().join("G1R-001.sav");
        let path = notes_path(&save);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"{ this is not json").unwrap();

        let err = record(&save, &[("Npc-A".to_string(), note())]).unwrap_err();
        assert!(
            format!("{err}").contains("not readable NPC-placement JSON"),
            "unexpected error: {err}"
        );
        // The damaged file is still there to be recovered by hand.
        assert_eq!(fs::read(&path).unwrap(), b"{ this is not json");
    }

    #[test]
    fn a_note_without_a_routine_class_round_trips_as_absent() {
        let dir = tempdir().unwrap();
        let save = dir.path().join("G1R-001.sav");
        let bare = PlacementNote {
            original_routine_class: None,
            written_routine_class: None,
            ..note()
        };
        record(&save, &[("Npc-A".to_string(), bare.clone())]).unwrap();

        let text = fs::read_to_string(notes_path(&save)).unwrap();
        assert!(!text.contains("routine_class"), "{text}");
        assert_eq!(read_notes(&save).get("Npc-A"), Some(&bare));
    }

    #[test]
    fn parse_records_refuses_an_entry_it_cannot_fully_read() {
        // A note the caller believes it recorded but that was silently dropped
        // would leave a restore button that cannot work, so a broken entry is a
        // request error rather than a skipped one.
        let missing_npc = serde_json::json!([{ "note": {
            "original_location": [1.0, 2.0, 3.0], "written_location": [4.0, 5.0, 6.0] } }]);
        assert!(parse_records(&missing_npc).is_err());

        let missing_note = serde_json::json!([{ "npc": "Npc-A" }]);
        assert!(parse_records(&missing_note).is_err());

        let bad_shape = serde_json::json!([{ "npc": "Npc-A", "note": { "written_location": [4.0] } }]);
        assert!(parse_records(&bad_shape).is_err());

        assert!(parse_records(&serde_json::json!("not an array")).is_err());
    }

    #[test]
    fn parse_clears_takes_ids_and_refuses_anything_else() {
        assert_eq!(
            parse_clears(&serde_json::json!(["Npc-A", "Npc-B"])).unwrap(),
            ["Npc-A", "Npc-B"]
        );
        assert!(parse_clears(&serde_json::json!([1])).is_err());
        assert!(parse_clears(&serde_json::json!({})).is_err());
    }
}
