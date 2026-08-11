//! The read-response cache must never answer for a file that has changed.
//! Requires a real GSAV save via GORE_SAVE; skips otherwise.
//!   GORE_SAVE='C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames\G1R-011.sav' \
//!     cargo test --release -p gore-save --test response_cache -- --nocapture
use serde_json::{Value, json};

fn exec(req: Value) -> Value {
    let resp: Value = serde_json::from_str(&gore_save::execute_json(&req.to_string())).unwrap();
    assert_eq!(resp["ok"], json!(true), "request failed: {resp}");
    resp["data"].clone()
}

fn source_save() -> Option<String> {
    match std::env::var("GORE_SAVE") {
        Ok(path) => Some(path),
        Err(_) => {
            eprintln!("GORE_SAVE not set; skipping");
            None
        }
    }
}

/// Copy the save so a test can edit it without touching the user's file.
fn temp_copy(name: &str) -> Option<(tempfile::TempDir, String)> {
    let source = source_save()?;
    let dir = tempfile::tempdir().expect("temp dir");
    let target = dir.path().join(name);
    std::fs::copy(&source, &target).expect("copy save");
    Some((dir, target.to_string_lossy().to_string()))
}

fn inspect(path: &str) -> Value {
    exec(json!({
        "command": "inspect_save",
        "payload": { "path": path, "includePrivate": true },
    }))
}

/// The typed path of the hero's first attribute base value, and its current
/// value — a scalar that `private.typed.setValue` can nudge in place.
fn first_hero_attribute(path: &str) -> (Vec<Value>, f64) {
    let found = exec(json!({
        "command": "search_typed_properties",
        "payload": {
            "path": path,
            "query": "AttributesByGlobalId {Hero}",
            "offset": 0,
            "limit": 1000,
        },
    }));
    let hit = found["results"]
        .as_array()
        .expect("results")
        .iter()
        .find(|hit| hit["type"] == "FloatProperty" && hit["editable"] == json!(true))
        .expect("no editable float attribute on the hero");
    let value: f64 = hit["value"].as_str().unwrap().parse().unwrap();
    (hit["path"].as_array().unwrap().clone(), value)
}

/// A repeat of the same request must return exactly what the first one did —
/// the cache is a memo, not an approximation.
#[test]
fn repeated_reads_return_the_same_answer() {
    let Some((_dir, path)) = temp_copy("G1R-cache-repeat.sav") else {
        return;
    };

    for request in [
        json!({ "command": "inspect_save", "payload": { "path": path, "includePrivate": true } }),
        json!({ "command": "private.characters.list", "payload": { "path": path } }),
        json!({ "command": "private.skills.list", "payload": { "path": path, "actor": "Hero" } }),
        json!({ "command": "private.factions.list", "payload": { "path": path } }),
        json!({
            "command": "query_progression",
            "payload": { "path": path, "section": "quests", "offset": 0, "limit": 100 },
        }),
        json!({
            "command": "search_typed_properties",
            "payload": { "path": path, "query": "GameTime", "offset": 0, "limit": 1000 },
        }),
    ] {
        let first = exec(request.clone());
        let second = exec(request.clone());
        assert_eq!(first, second, "second read differs for {request}");
    }
}

/// The whole point of keying on content: once the save has been written, the
/// next read must reflect the new bytes rather than the memo of the old ones.
#[test]
fn a_write_is_never_served_a_stale_read() {
    let Some((_dir, path)) = temp_copy("G1R-cache-write.sav") else {
        return;
    };

    // Read first, so the pre-write answers are in the cache.
    let (attribute_path, before) = first_hero_attribute(&path);
    let _ = inspect(&path);

    // A real edit through the same entry point the editor uses, written back
    // over the same file.
    exec(json!({
        "command": "write_save",
        "payload": {
            "path": path,
            "outputPath": path,
            "backup": false,
            "edits": [{
                "path": "private.typed.setValue",
                "value": { "path": attribute_path, "value": before + 1.0 },
            }],
        },
    }));

    let (_, after) = first_hero_attribute(&path);
    assert_eq!(
        after,
        before + 1.0,
        "the typed search served its pre-write value",
    );
}

/// A save replaced behind the editor's back (a cloud sync, the game saving over
/// the slot) runs no write command, so only the content fingerprint can catch
/// it.
#[test]
fn an_external_replacement_is_not_served_from_cache() {
    let Some((_dir, path)) = temp_copy("G1R-cache-external.sav") else {
        return;
    };
    let Some((_other_dir, other)) = temp_copy("G1R-cache-external-source.sav") else {
        return;
    };

    let (attribute_path, before) = first_hero_attribute(&path);

    // Edit the OTHER copy, then move its bytes over the read one without going
    // through any command that touches `path`.
    exec(json!({
        "command": "write_save",
        "payload": {
            "path": other,
            "outputPath": other,
            "backup": false,
            "edits": [{
                "path": "private.typed.setValue",
                "value": { "path": attribute_path, "value": before + 2.0 },
            }],
        },
    }));
    std::fs::copy(&other, &path).expect("replace the save behind the core's back");

    let (_, after) = first_hero_attribute(&path);
    assert_eq!(
        after,
        before + 2.0,
        "the typed search served the replaced file's cached response",
    );
}

/// `list_backups` describes a directory, not the save, so it must not be cached:
/// removing a backup changes the answer while the save file is untouched.
#[test]
fn directory_listings_are_not_cached() {
    let Some((dir, path)) = temp_copy("G1R-cache-backups.sav") else {
        return;
    };

    let backup_dir = dir.path().join("goresave_backups");
    std::fs::create_dir_all(&backup_dir).expect("backup dir");
    std::fs::copy(&path, backup_dir.join("G1R-cache-backups.sav.bak.100")).expect("backup");

    let listed = exec(json!({ "command": "list_backups", "payload": { "path": path } }));
    let backups = listed["backups"].as_array().cloned().unwrap_or_default();
    assert!(!backups.is_empty(), "no backup was created to test with");

    // Remove the backups outright; the save file itself is unchanged, so a
    // save-keyed cache would happily serve the old listing.
    std::fs::remove_dir_all(&backup_dir).expect("drop backups");

    let relisted = exec(json!({ "command": "list_backups", "payload": { "path": path } }));
    assert!(
        relisted["backups"]
            .as_array()
            .is_none_or(|backups| backups.is_empty()),
        "list_backups served a cached listing after the backups were removed",
    );
}
