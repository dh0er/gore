//! NPC saved-pose read + byte-level write roundtrip against a real save.
//! Requires a real GSAV save via GORE_SAVE; skips otherwise. The save is COPIED
//! into a temp dir first — the tests never touch the original.
//!
//! NOTE: the write tests below prove the BYTES change and read back, nothing
//! more. The game restores an NPC's placement from the level's WorldPointActor
//! and discards these records on load — a runtime probe read back the original
//! pre-edit values afterwards — so a moved pose here does not move an NPC in
//! game. See the `NpcPose` doc in `src/npc.rs`.
//!   GORE_SAVE='C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames\G1R-035.sav' \
//!     cargo test -p gore-save --test positions -- --nocapture
use serde_json::{Value, json};

fn exec(req: Value) -> Value {
    let resp: Value = serde_json::from_str(&gore_save::execute_json(&req.to_string())).unwrap();
    assert_eq!(resp["ok"], json!(true), "request failed: {resp}");
    resp["data"].clone()
}

/// The save under test: a COPY of `GORE_SAVE` in a temp dir, so an edit test can
/// never damage the user's real save. Returns `None` when GORE_SAVE is unset.
fn save_copy(name: &str) -> Option<(tempfile::TempDir, String)> {
    let source = std::env::var("GORE_SAVE").ok()?;
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join(name);
    std::fs::copy(&source, &target).unwrap();
    Some((dir, target.to_string_lossy().to_string()))
}

fn npc_ids(path: &str, limit: usize) -> Vec<String> {
    let data = exec(json!({
        "command": "private.npc.list",
        "payload": { "path": path, "limit": limit }
    }));
    data["npcs"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n["id"].as_str().map(str::to_string))
        .collect()
}

fn pose(path: &str, id: &str) -> Value {
    exec(json!({
        "command": "private.npc.position",
        "payload": { "path": path, "id": id }
    }))["pose"]
        .clone()
}

#[test]
fn npc_positions_read_for_every_listed_npc() {
    let Some((_dir, path)) = save_copy("G1R-positions-read.sav") else {
        eprintln!("GORE_SAVE not set; skipping");
        return;
    };
    let ids = npc_ids(&path, 200);
    assert!(!ids.is_empty(), "no NPCs listed");

    let mut at_origin = 0usize;
    let mut missing_location = 0usize;
    for id in &ids {
        let pose = pose(&path, id);
        assert!(
            pose["locationPath"]
                .as_array()
                .is_some_and(|p| !p.is_empty()),
            "{id} has no locationPath"
        );
        assert!(pose["rotationPath"].as_array().is_some());
        match pose["location"].as_object() {
            None => missing_location += 1,
            Some(loc) => {
                let zero = ["x", "y", "z"]
                    .iter()
                    .all(|k| loc[*k].as_f64().unwrap_or(f64::NAN) == 0.0);
                if zero {
                    at_origin += 1;
                }
            }
        }
        // A rotation is reported with the engine's component names, never x/y/z.
        if let Some(rot) = pose["rotation"].as_object() {
            assert!(rot.contains_key("pitch"), "{id} rotation: {rot:?}");
            assert!(!rot.contains_key("x"), "{id} rotation must not carry x");
        }
    }
    eprintln!(
        "positions: {} NPCs read, {at_origin} at (0,0,0), {missing_location} without CharacterLocation",
        ids.len()
    );
}

#[test]
fn npc_position_location_roundtrips_through_typed_set_value() {
    let Some((_dir, path)) = save_copy("G1R-positions-edit.sav") else {
        eprintln!("GORE_SAVE not set; skipping");
        return;
    };
    // Pick a listed NPC that actually has a location to nudge.
    let Some((id, before)) = npc_ids(&path, 50).into_iter().find_map(|id| {
        let p = pose(&path, &id);
        p["location"].as_object()?;
        Some((id, p))
    }) else {
        eprintln!("no NPC with a CharacterLocation; skipping asserts");
        return;
    };
    let location_path = before["locationPath"].clone();
    let (x, y, z) = (
        before["location"]["x"].as_f64().unwrap(),
        before["location"]["y"].as_f64().unwrap(),
        before["location"]["z"].as_f64().unwrap(),
    );

    let out = std::path::Path::new(&path)
        .with_file_name("G1R-positions-edit-out.sav")
        .to_string_lossy()
        .to_string();
    let started = std::time::Instant::now();
    exec(json!({
        "command": "write_save",
        "payload": {
            "path": path, "outputPath": out, "backup": false,
            "edits": [{ "path": "private.typed.setValue",
                        "value": { "path": location_path, "value": {"x": x + 1.0, "y": y, "z": z} } }],
        }
    }));
    // Printed next to the 10-NPC batch time so the two are comparable: the cost
    // is recompressing the private payload, not the edit itself.
    eprintln!(
        "single: 1 NPC position in one write_save took {:.3} s",
        started.elapsed().as_secs_f64()
    );

    let moved = pose(&out, &id);
    assert_eq!(moved["location"]["x"].as_f64().unwrap(), x + 1.0);
    assert_eq!(moved["location"]["y"].as_f64().unwrap(), y);
    assert_eq!(moved["location"]["z"].as_f64().unwrap(), z);

    // Write it back and confirm the original value returns.
    let restored_path = std::path::Path::new(&path)
        .with_file_name("G1R-positions-edit-back.sav")
        .to_string_lossy()
        .to_string();
    exec(json!({
        "command": "write_save",
        "payload": {
            "path": out, "outputPath": restored_path, "backup": false,
            "edits": [{ "path": "private.typed.setValue",
                        "value": { "path": location_path, "value": {"x": x, "y": y, "z": z} } }],
        }
    }));
    let restored = pose(&restored_path, &id);
    assert_eq!(restored["location"], before["location"]);
    eprintln!("roundtrip ok: {id} at ({x}, {y}, {z}) nudged +1 on x and back");
}

#[test]
fn ten_npc_positions_move_in_one_write() {
    let Some((_dir, path)) = save_copy("G1R-positions-batch.sav") else {
        eprintln!("GORE_SAVE not set; skipping");
        return;
    };
    // Each target keeps its real y/z — a batch must move points, not flatten them.
    let targets: Vec<(String, Value, Value)> = npc_ids(&path, 100)
        .into_iter()
        .filter_map(|id| {
            let p = pose(&path, &id);
            p["location"]["x"].as_f64()?;
            Some((id, p["locationPath"].clone(), p["location"].clone()))
        })
        .take(10)
        .collect();
    if targets.len() < 10 {
        eprintln!("fewer than 10 NPCs with a location; skipping asserts");
        return;
    }

    let edits: Vec<Value> = targets
        .iter()
        .map(|(_id, location_path, location)| {
            json!({ "path": "private.typed.setValue",
                    "value": { "path": location_path,
                               "value": { "x": location["x"].as_f64().unwrap() + 1.0,
                                          "y": location["y"], "z": location["z"] } } })
        })
        .collect();

    let out = std::path::Path::new(&path)
        .with_file_name("G1R-positions-batch-out.sav")
        .to_string_lossy()
        .to_string();
    let started = std::time::Instant::now();
    exec(json!({
        "command": "write_save",
        "payload": { "path": path, "outputPath": out, "backup": false, "edits": edits }
    }));
    let elapsed = started.elapsed();

    for (id, _location_path, location) in &targets {
        let after = pose(&out, id);
        assert_eq!(
            after["location"]["x"].as_f64().unwrap(),
            location["x"].as_f64().unwrap() + 1.0,
            "{id} did not move"
        );
        assert_eq!(after["location"]["y"], location["y"], "{id} y changed");
    }
    eprintln!(
        "batch: 10 NPC positions in ONE write_save took {:.3} s",
        elapsed.as_secs_f64()
    );
}
