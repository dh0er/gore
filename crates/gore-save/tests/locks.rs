//! Lock read + edit roundtrip over the JSON command surface.
//!
//! Runs against the embedded game-start save, so it needs no `GORE_SAVE` and no
//! game install: the shipped bytes already carry `m_UnlockedLocks` as an empty
//! `Set<NameProperty>`, which is exactly the state a new game starts in.
//!
//!   cargo test -p gore-save --test locks -- --nocapture

use serde_json::{Value, json};

fn exec(req: Value) -> Value {
    let resp: Value = serde_json::from_str(&gore_save::execute_json(&req.to_string())).unwrap();
    assert_eq!(resp["ok"], json!(true), "request failed: {resp}");
    resp["data"].clone()
}

fn exec_err(req: Value) -> String {
    let resp: Value = serde_json::from_str(&gore_save::execute_json(&req.to_string())).unwrap();
    assert_eq!(
        resp["ok"],
        json!(false),
        "request unexpectedly succeeded: {resp}"
    );
    resp["error"].to_string()
}

fn start_save(name: &str) -> String {
    let mut p = std::env::temp_dir();
    p.push(format!("gore_locks_{name}.sav"));
    std::fs::write(
        &p,
        gore_save::startsaves::start_save_bytes(gore_save::startsaves::ResourcesLevel::Gothic),
    )
    .expect("write temp save");
    p.to_string_lossy().to_string()
}

fn out_path(name: &str) -> String {
    let mut p = std::env::temp_dir();
    p.push(format!("gore_locks_{name}_out.sav"));
    p.to_string_lossy().to_string()
}

fn list(path: &str) -> Value {
    exec(json!({ "command": "private.locks.list", "payload": { "path": path } }))
}

fn unlocked(path: &str) -> Vec<String> {
    list(path)["unlocked"]
        .as_array()
        .expect("unlocked array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

fn write(path: &str, out: &str, edits: Vec<Value>) -> Value {
    exec(json!({
        "command": "write_save",
        "payload": { "path": path, "outputPath": out, "edits": edits },
    }))
}

fn set_unlocked(lock: &str, unlocked: bool) -> Value {
    json!({
        "path": "private.locks.setUnlocked",
        "value": { "lock": lock, "unlocked": unlocked },
    })
}

#[test]
fn a_fresh_save_has_no_unlocked_locks_but_is_writable() {
    let path = start_save("fresh");
    let data = list(&path);
    assert_eq!(data["unlocked"], json!([]));
    assert_eq!(data["writable"], json!(["private.locks.setUnlocked"]));
}

#[test]
fn unlocking_a_lock_adds_its_name_to_the_set() {
    let path = start_save("unlock");
    let out = out_path("unlock");
    write(&path, &out, vec![set_unlocked("IO_OC_CHEST_DEXTER", true)]);
    assert_eq!(unlocked(&out), ["IO_OC_CHEST_DEXTER"]);
}

#[test]
fn locking_it_again_removes_the_name() {
    let path = start_save("relock");
    let opened = out_path("relock_opened");
    let closed = out_path("relock_closed");
    write(
        &path,
        &opened,
        vec![set_unlocked("OC_Guards_Cell_01_Door", true)],
    );
    assert_eq!(unlocked(&opened), ["OC_Guards_Cell_01_Door"]);

    write(
        &opened,
        &closed,
        vec![set_unlocked("OC_Guards_Cell_01_Door", false)],
    );
    assert!(unlocked(&closed).is_empty());
}

#[test]
fn chest_and_door_locks_batch_into_one_write() {
    let path = start_save("batch");
    let out = out_path("batch");
    write(
        &path,
        &out,
        vec![
            set_unlocked("IO_OC_CHEST_DEXTER", true),
            set_unlocked("OC_Santino_Door", true),
            set_unlocked("IO_NC_CHEST_RICELORD", true),
        ],
    );
    let mut names = unlocked(&out);
    names.sort();
    assert_eq!(
        names,
        [
            "IO_NC_CHEST_RICELORD",
            "IO_OC_CHEST_DEXTER",
            "OC_Santino_Door"
        ]
    );
}

#[test]
fn an_intent_that_already_holds_is_a_no_op() {
    let path = start_save("idempotent");
    let once = out_path("idempotent_once");
    let twice = out_path("idempotent_twice");
    write(&path, &once, vec![set_unlocked("IO_OC_CHEST_STONE", true)]);
    // Re-applying the same desired state must not fail on a duplicate insert,
    // which is what makes a retry after a partial write safe.
    write(&once, &twice, vec![set_unlocked("IO_OC_CHEST_STONE", true)]);
    assert_eq!(unlocked(&twice), ["IO_OC_CHEST_STONE"]);

    // Locking something that was never unlocked is equally a no-op.
    let never = out_path("idempotent_never");
    write(&twice, &never, vec![set_unlocked("IO_SW_CHEST_01", false)]);
    assert_eq!(unlocked(&never), ["IO_OC_CHEST_STONE"]);
}

#[test]
fn two_intents_for_the_same_lock_are_refused() {
    let path = start_save("conflict");
    let out = out_path("conflict");
    let error = exec_err(json!({
        "command": "write_save",
        "payload": {
            "path": path,
            "outputPath": out,
            "edits": [
                set_unlocked("IO_OC_CHEST_DEXTER", true),
                set_unlocked("IO_OC_CHEST_DEXTER", false),
            ],
        },
    }));
    assert!(
        error.contains("lock state of that chest or door"),
        "unexpected error: {error}"
    );
    assert!(
        !std::path::Path::new(&out).exists(),
        "a refused write must leave no output file"
    );
}

#[test]
fn relocking_and_raw_door_array_edits_are_refused_in_both_orders() {
    let path = start_save("door_conflict");
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("refused.sav");
    let lock = set_unlocked("CV_Stash_Door", false);
    let raw = json!({
        "path": "private.typed.arrayRemove",
        "value": {"path": ["m_DoorsOpen"], "index": 0},
    });
    for edits in [vec![lock.clone(), raw.clone()], vec![raw, lock]] {
        let error = exec_err(json!({
            "command": "write_save",
            "payload": {"path": path, "outputPath": out, "edits": edits},
        }));
        assert!(
            error.contains("rewrites as a whole"),
            "unexpected error: {error}"
        );
        assert!(!out.exists());
    }
}

#[test]
fn a_missing_or_malformed_value_is_refused() {
    let path = start_save("malformed");
    let out = out_path("malformed");
    for value in [
        json!({ "unlocked": true }),
        json!({ "lock": "   ", "unlocked": true }),
        json!({ "lock": "IO_OC_CHEST_DEXTER" }),
    ] {
        let error = exec_err(json!({
            "command": "write_save",
            "payload": {
                "path": path,
                "outputPath": out,
                "edits": [{ "path": "private.locks.setUnlocked", "value": value }],
            },
        }));
        assert!(
            error.contains("private.locks.setUnlocked"),
            "unexpected error: {error}"
        );
    }
}
