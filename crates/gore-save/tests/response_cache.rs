//! The read-response cache must never answer for a file that has changed.
//! Requires a real GSAV save via GORE_SAVE; skips otherwise.
//!   GORE_SAVE='C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames\G1R-011.sav' \
//!     cargo test --release -p gore-save --test response_cache -- --nocapture
use serde_json::{Value, json};

/// The caches under test are process-global and hold ONE save each, so two of
/// these tests running at once displace each other's state — harmless for the
/// content assertions, fatal for the timing ones. Every test takes this first,
/// which keeps the file correct whatever `--test-threads` is set to.
static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn serially() -> std::sync::MutexGuard<'static, ()> {
    ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner())
}

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
    let _serial = serially();
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
    let _serial = serially();
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
    let _serial = serially();
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

/// Returning to a save opened earlier gets its inspection from the response
/// cache, which means nothing reseeds the single-save decode and parse caches —
/// they still hold whichever save was opened in between. `warm_save` exists so
/// the background warm-up can put that right before the user clicks something
/// the response cache cannot answer, such as a per-NPC detail.
///
/// This is a timing test, because the property IS timing: both paths return the
/// same answer. It compares the two against each other rather than against a
/// fixed budget, so it does not depend on how fast the machine is; the gap it
/// guards was measured at roughly 250x.
#[test]
fn warming_a_returned_to_save_moves_the_reparse_off_the_next_read() {
    let _serial = serially();
    let Some((_dir_a, a)) = temp_copy("G1R-warm-a.sav") else {
        return;
    };
    let Some((_dir_b, b)) = temp_copy("G1R-warm-b.sav") else {
        return;
    };

    // A query whose answer is NOT in the response cache each time it is asked,
    // so it has to reach the parsed tree — as a freshly opened NPC panel does.
    let mut probe = 0;
    let mut read_needing_the_tree = |path: &str| {
        probe += 1;
        let started = std::time::Instant::now();
        exec(json!({
            "command": "search_typed_properties",
            "payload": {
                "path": path,
                "query": format!("GameTime {probe}"),
                "offset": 0,
                "limit": 10,
            },
        }));
        started.elapsed()
    };

    // A, away to B, back to A. The inspection comes back cached; the caches hold B.
    let _ = inspect(&a);
    let _ = read_needing_the_tree(&a);
    let _ = inspect(&b);
    let _ = read_needing_the_tree(&b);
    let _ = inspect(&a);
    let cold = read_needing_the_tree(&a);

    // Same again, with the warm-up step the prefetch performs.
    let _ = inspect(&b);
    let _ = read_needing_the_tree(&b);
    let _ = inspect(&a);
    exec(json!({ "command": "warm_save", "payload": { "path": a } }));
    let warmed = read_needing_the_tree(&a);

    assert!(
        warmed * 4 < cold,
        "warming did not move the reparse off the read: {warmed:?} against {cold:?}",
    );
}

/// `warm_save` must never be answered from the response cache: a stored "warmed"
/// would skip the seeding that is the entire point of the call.
#[test]
fn warming_is_never_answered_from_the_cache() {
    let _serial = serially();
    let Some((_dir_a, a)) = temp_copy("G1R-warm-cache-a.sav") else {
        return;
    };
    let Some((_dir_b, b)) = temp_copy("G1R-warm-cache-b.sav") else {
        return;
    };
    let warm = |path: &str| {
        let started = std::time::Instant::now();
        exec(json!({ "command": "warm_save", "payload": { "path": path } }));
        started.elapsed()
    };

    warm(&a);
    // Already this save: nothing to do beyond reading and hashing the file.
    let repeat = warm(&a);
    // Displace it, then ask again for the SAME request as the first call. A
    // cached answer would return just as fast as the repeat above did.
    warm(&b);
    let after_displacement = warm(&a);

    assert!(
        repeat * 4 < after_displacement,
        "warm_save was served from cache: {repeat:?} against {after_displacement:?}",
    );
}

/// `private.npc.position` reports the recorded placement undo, which lives in a
/// sidecar next to the save rather than inside it. The sidecar can change while
/// the save bytes stay exactly as they were — restoring a backup puts back
/// byte-identical bytes alongside that backup's placement notes — so the cache
/// key has to cover it.
#[test]
fn a_changed_placement_note_is_not_served_from_cache() {
    let _serial = serially();
    let Some((_dir, path)) = temp_copy("G1R-cache-placement.sav") else {
        return;
    };
    let save = std::path::Path::new(&path);

    // Any NPC the save actually knows about.
    let listed = exec(json!({
        "command": "private.npc.list",
        "payload": { "path": path, "offset": 0, "limit": 1 },
    }));
    let Some(npc) = listed["npcs"]
        .as_array()
        .and_then(|npcs| npcs.first())
        .and_then(|npc| npc["id"].as_str())
        .map(str::to_owned)
    else {
        eprintln!("save lists no NPCs; skipping");
        return;
    };

    let position = json!({
        "command": "private.npc.position",
        "payload": { "path": path, "id": npc },
    });
    assert!(
        exec(position.clone())["undo"].is_null(),
        "the fixture already carries a placement note for {npc}",
    );

    // Record a note. Only the sidecar changes; the save file is untouched.
    let before = std::fs::read(save).expect("read save");
    gore_save::placement::record(
        save,
        &[(
            npc.clone(),
            gore_save::placement::PlacementNote {
                original_location: [1.0, 2.0, 3.0],
                original_routine_class: None,
                original_rotation: None,
                written_location: [4.0, 5.0, 6.0],
                written_rotation: None,
                written_routine_class: None,
            },
        )],
    )
    .expect("record placement note");
    assert_eq!(
        std::fs::read(save).expect("re-read save"),
        before,
        "recording a note must not touch the save",
    );

    assert!(
        !exec(position.clone())["undo"].is_null(),
        "private.npc.position served its pre-note answer from cache",
    );

    // And back the other way: dropping the note must surface again.
    gore_save::placement::clear(save, std::slice::from_ref(&npc)).expect("clear placement note");
    assert!(
        exec(position)["undo"].is_null(),
        "private.npc.position served the removed note from cache",
    );
}

/// `list_backups` describes a directory, not the save, so it must not be cached:
/// removing a backup changes the answer while the save file is untouched.
#[test]
fn directory_listings_are_not_cached() {
    let _serial = serially();
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
