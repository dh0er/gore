//! Trader shop read + edit roundtrip over the JSON command surface.
//!
//! Runs against the embedded game-start save, so it needs no `GORE_SAVE` and no
//! game install: the shipped bytes already contain all 31 trader rows.
//!
//!   cargo test -p gore-save --test traders -- --nocapture

use serde_json::{Value, json};

fn exec(req: Value) -> Value {
    let resp: Value = serde_json::from_str(&gore_save::execute_json(&req.to_string())).unwrap();
    assert_eq!(resp["ok"], json!(true), "request failed: {resp}");
    resp["data"].clone()
}

fn exec_err(req: Value) -> String {
    let resp: Value = serde_json::from_str(&gore_save::execute_json(&req.to_string())).unwrap();
    assert_eq!(resp["ok"], json!(false), "request unexpectedly succeeded: {resp}");
    resp["error"].to_string()
}

/// Lay the embedded start save down as a real file so the command surface, which
/// works on paths, has something to open.
fn start_save(name: &str) -> String {
    let mut p = std::env::temp_dir();
    p.push(format!("gore_traders_{name}.sav"));
    std::fs::write(
        &p,
        gore_save::startsaves::start_save_bytes(gore_save::startsaves::ResourcesLevel::Gothic),
    )
    .expect("write temp save");
    p.to_string_lossy().to_string()
}

fn out_path(name: &str) -> String {
    let mut p = std::env::temp_dir();
    p.push(format!("gore_traders_{name}_out.sav"));
    p.to_string_lossy().to_string()
}

fn list(path: &str) -> Value {
    exec(json!({ "command": "private.traders.list", "payload": { "path": path } }))
}

fn detail(path: &str, index: u64) -> Value {
    exec(json!({
        "command": "private.traders.detail",
        "payload": { "path": path, "index": index }
    }))
}

fn write(path: &str, out: &str, edits: Value) -> Value {
    exec(json!({
        "command": "write_save",
        "payload": { "path": path, "outputPath": out, "backup": false, "edits": edits }
    }))
}

/// The first real (non-placeholder) trader that stocks something.
fn stocked_trader(data: &Value) -> (u64, String) {
    let t = data["traders"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["placeholder"] == json!(false) && t["itemCount"].as_u64().unwrap() > 0)
        .expect("a shipped trader stocks something");
    (
        t["index"].as_u64().unwrap(),
        t["uniqueName"].as_str().unwrap().to_string(),
    )
}

fn item_count(detail: &Value, path: &str) -> Option<i64> {
    detail["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["path"] == json!(path))
        .map(|i| i["count"].as_i64().unwrap())
}

const ORE: &str = "/Script/Angelscript.ItMi_Orenugget";

#[test]
fn list_reports_every_trader_and_the_ore_they_can_pay_with() {
    let path = start_save("list");
    let data = list(&path);
    let traders = data["traders"].as_array().unwrap();
    assert_eq!(traders.len(), 31, "every shipped save carries 31 trader rows");

    // Two rows are unnamed sentinels; that is why nothing may be addressed by
    // name alone.
    assert_eq!(
        traders
            .iter()
            .filter(|t| t["placeholder"] == json!(true))
            .count(),
        2
    );
    // Nobody has traded in a game-start save.
    assert!(traders.iter().all(|t| t["traded"] == json!(false)));
    // Ore is optional: a trader without the key reports null, not zero.
    assert!(traders.iter().any(|t| t["ore"].is_i64()));
    assert!(traders.iter().any(|t| t["ore"].is_null()));

    let writable: Vec<&str> = data["writable"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(writable.contains(&"private.traders.setStock"));
    assert!(writable.contains(&"private.traders.addItem"));
    assert!(writable.contains(&"private.traders.removeItem"));
}

#[test]
fn detail_resolves_by_index_and_by_unambiguous_name() {
    let path = start_save("detail");
    let data = list(&path);
    let (index, name) = stocked_trader(&data);

    let by_index = detail(&path, index);
    let by_name = exec(json!({
        "command": "private.traders.detail",
        "payload": { "path": &path, "uniqueName": &name }
    }));
    assert_eq!(by_index, by_name);
    assert_eq!(by_index["uniqueName"], json!(name));
    assert!(!by_index["items"].as_array().unwrap().is_empty());
    assert_eq!(
        by_index["totalSecondsPath"],
        json!([
            "m_GenericData",
            "{GameStateDataBase}",
            "m_Traders",
            format!("[{index}]"),
            "m_TotalSeconds"
        ])
    );
    // The staging map is empty in every shipped save; if this fires it needs modelling.
    assert_eq!(by_index["hasItemsByDifficulty"], json!(false));

    // The sentinel name is shared by two rows and must be refused, not guessed.
    let err = exec_err(json!({
        "command": "private.traders.detail",
        "payload": { "path": &path, "uniqueName": "None" }
    }));
    assert!(err.contains("ambiguous"), "{err}");
}

#[test]
fn set_stock_roundtrips_and_batches() {
    let path = start_save("setstock");
    let out = out_path("setstock");
    let data = list(&path);
    let (index, _) = stocked_trader(&data);

    // Two length-neutral edits in ONE write: this is exactly what the guard
    // allows and the reason setStock is not a splicing edit.
    let victim = detail(&path, index)["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["path"] != json!(ORE))
        .expect("something besides ore")
        .clone();
    let victim_path = victim["path"].as_str().unwrap().to_string();

    write(
        &path,
        &out,
        json!([
            { "path": "private.traders.setStock", "value": { "index": index, "path": ORE, "count": 4242 } },
            { "path": "private.traders.setStock", "value": { "index": index, "path": &victim_path, "count": 11 } },
        ]),
    );

    let after = detail(&out, index);
    assert_eq!(item_count(&after, ORE), Some(4242));
    assert_eq!(item_count(&after, &victim_path), Some(11));
}

#[test]
fn set_stock_targets_the_restock_baseline_when_asked() {
    let path = start_save("setstock_default");
    let out = out_path("setstock_default");
    let data = list(&path);
    let (index, _) = stocked_trader(&data);
    let before = detail(&path, index);
    let ore_before = item_count(&before, ORE);

    write(
        &path,
        &out,
        json!([
            { "path": "private.traders.setStock",
              "value": { "index": index, "path": ORE, "count": 5, "map": "default" } },
        ]),
    );

    let after = detail(&out, index);
    let default_ore = after["defaultItems"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["path"] == json!(ORE))
        .map(|i| i["count"].as_i64().unwrap());
    assert_eq!(default_ore, Some(5));
    assert_eq!(item_count(&after, ORE), ore_before, "m_Items untouched");
}

#[test]
fn add_and_remove_a_stock_line_through_the_command_surface() {
    let path = start_save("addremove");
    let added = out_path("addremove_added");
    let removed = out_path("addremove_removed");
    let data = list(&path);
    let (index, _) = stocked_trader(&data);

    // A catalogued item this trader does not already stock.
    let held: std::collections::HashSet<String> = detail(&path, index)["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["path"].as_str().unwrap().to_string())
        .collect();
    let fresh = ["ItFo_Cheese", "ItFo_Loaf", "ItKe_Lockpick", "ItAm_Bolt"]
        .iter()
        .map(|n| format!("/Script/Angelscript.{n}"))
        .find(|p| !held.contains(p))
        .expect("one of the probe items is unstocked");

    write(
        &path,
        &added,
        json!([
            { "path": "private.traders.addItem",
              "value": { "index": index, "path": &fresh, "count": 9 } },
        ]),
    );
    let after_add = detail(&added, index);
    assert_eq!(item_count(&after_add, &fresh), Some(9));
    assert_eq!(
        after_add["items"].as_array().unwrap().len(),
        held.len() + 1
    );

    write(
        &added,
        &removed,
        json!([
            { "path": "private.traders.removeItem",
              "value": { "index": index, "path": &fresh } },
        ]),
    );
    let after_remove = detail(&removed, index);
    assert_eq!(item_count(&after_remove, &fresh), None);
    assert_eq!(after_remove["items"].as_array().unwrap().len(), held.len());
}

#[test]
fn an_index_addressed_trader_edit_may_not_follow_a_structural_one() {
    // Every trader edit addresses its row by an index the caller read off a
    // list taken before the write. An insert earlier in the same write changes
    // how many entries a container holds, so the core refuses the pair rather
    // than let the index resolve against the changed layout.
    let path = start_save("guard");
    let out = out_path("guard");
    let data = list(&path);
    let (index, _) = stocked_trader(&data);

    let err = exec_err(json!({
        "command": "write_save",
        "payload": {
            "path": path,
            "outputPath": out,
            "backup": false,
            "edits": [
                { "path": "private.traders.addItem",
                  "value": { "index": index, "path": "/Script/Angelscript.ItFo_Cheese", "count": 1 } },
                { "path": "private.traders.setStock",
                  "value": { "index": index, "path": ORE, "count": 1 } },
            ]
        }
    }));
    assert!(err.contains("addresses an element by index"), "{err}");
    assert!(err.contains("private.traders.addItem"), "{err}");
    assert!(!std::path::Path::new(&out).exists(), "a refused write must not produce a save");
}

#[test]
fn the_same_pair_is_accepted_the_other_way_round() {
    // The refusal above is about ORDER, not about the two operations being
    // incompatible: with the index-addressed edit first, both resolve against
    // the layout they were read from and the write goes through.
    let path = start_save("order");
    let out = out_path("order");
    let data = list(&path);
    let (index, _) = stocked_trader(&data);
    let fresh = "/Script/Angelscript.ItFo_Cheese";
    assert!(
        item_count(&detail(&path, index), fresh).is_none(),
        "the probe item must not already be stocked"
    );

    write(
        &path,
        &out,
        json!([
            { "path": "private.traders.setStock", "value": { "index": index, "path": ORE, "count": 321 } },
            { "path": "private.traders.addItem", "value": { "index": index, "path": fresh, "count": 4 } },
        ]),
    );

    let after = detail(&out, index);
    assert_eq!(item_count(&after, ORE), Some(321));
    assert_eq!(item_count(&after, fresh), Some(4));
}

#[test]
fn a_trader_edit_and_an_m_traders_array_splice_are_refused_together() {
    // A trader edit is addressed by its row's position, and an array operation
    // on m_Traders renumbers the rows — so the pair is unsafe in either order,
    // and splitting it into two writes would not help: the index came from a
    // list read before either ran.
    let path = start_save("arrayconflict");
    let out = out_path("arrayconflict");
    let data = list(&path);
    let (index, _) = stocked_trader(&data);
    let traders_path = ["m_GenericData", "{GameStateDataBase}", "m_Traders"];

    for edits in [
        json!([
            { "path": "private.typed.arrayRemove",
              "value": { "path": traders_path, "index": 0 } },
            { "path": "private.traders.setStock",
              "value": { "index": index, "path": ORE, "count": 5 } },
        ]),
        // The reverse order is no safer, so it is refused too.
        json!([
            { "path": "private.traders.setStock",
              "value": { "index": index, "path": ORE, "count": 5 } },
            { "path": "private.typed.arrayRemove",
              "value": { "path": traders_path, "index": 0 } },
        ]),
    ] {
        let err = exec_err(json!({
            "command": "write_save",
            "payload": {
                "path": path, "outputPath": out, "backup": false, "edits": edits
            }
        }));
        assert!(err.contains("rewrites"), "{err}");
        assert!(!std::path::Path::new(&out).exists());
    }
}

#[test]
fn an_edit_inside_a_trader_row_is_not_a_renumbering_splice() {
    // Only an array operation ON m_Traders moves its rows. A value under one row
    // runs through the array without changing its length, so the pair is safe
    // and must go through.
    let path = start_save("insiderow");
    let out = out_path("insiderow");
    let data = list(&path);
    let (index, _) = stocked_trader(&data);

    write(
        &path,
        &out,
        json!([
            { "path": "private.traders.setStock",
              "value": { "index": index, "path": ORE, "count": 77 } },
            { "path": "private.typed.setValue",
              "value": {
                  "path": ["m_GenericData", "{GameStateDataBase}", "m_Traders",
                           format!("[{index}]"), "m_TotalSeconds"],
                  "value": 12345.5
              } },
        ]),
    );

    let after = detail(&out, index);
    assert_eq!(item_count(&after, ORE), Some(77));
    assert_eq!(after["totalSeconds"].as_f64(), Some(12345.5));
}

#[test]
fn a_set_and_a_removal_of_one_line_are_refused_together() {
    // The set would apply and the removal would then delete the line it lives
    // in, with the write reporting both as applied.
    let path = start_save("setremove");
    let out = out_path("setremove");
    let data = list(&path);
    let (index, _) = stocked_trader(&data);

    let err = exec_err(json!({
        "command": "write_save",
        "payload": {
            "path": path, "outputPath": out, "backup": false,
            "edits": [
                { "path": "private.traders.setStock",
                  "value": { "index": index, "path": ORE, "count": 9 } },
                { "path": "private.traders.removeItem",
                  "value": { "index": index, "path": ORE } },
            ]
        }
    }));
    assert!(err.contains("stock line of that trader"), "{err}");
    assert!(!std::path::Path::new(&out).exists());
}

#[test]
fn a_set_and_a_removal_of_different_lines_still_batch() {
    // Only the SAME line collides; unrelated lines are what a batch is for.
    let path = start_save("setremove_other");
    let out = out_path("setremove_other");
    let data = list(&path);
    let (index, _) = stocked_trader(&data);
    let other = detail(&path, index)["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["path"] != json!(ORE))
        .expect("something besides ore")["path"]
        .as_str()
        .unwrap()
        .to_string();

    write(
        &path,
        &out,
        json!([
            { "path": "private.traders.setStock",
              "value": { "index": index, "path": ORE, "count": 9 } },
            { "path": "private.traders.removeItem",
              "value": { "index": index, "path": &other } },
        ]),
    );

    let after = detail(&out, index);
    assert_eq!(item_count(&after, ORE), Some(9));
    assert_eq!(item_count(&after, &other), None);
}

#[test]
fn add_item_refuses_a_class_the_game_does_not_know() {
    let path = start_save("badclass");
    let out = out_path("badclass");
    let data = list(&path);
    let (index, _) = stocked_trader(&data);

    let err = exec_err(json!({
        "command": "write_save",
        "payload": {
            "path": path,
            "outputPath": out,
            "backup": false,
            "edits": [
                { "path": "private.traders.addItem",
                  "value": { "index": index, "path": "/Script/Angelscript.ItXx_Nope", "count": 1 } },
            ]
        }
    }));
    assert!(err.contains("not a known item class"), "{err}");
    assert!(!std::path::Path::new(&out).exists(), "a refused write must not produce a save");
}

#[test]
fn set_stock_rejects_a_zero_count() {
    // A sold-out line is deleted from the map, never held at zero, so setting a
    // count to 0 would write a record the game never produces. The refusal names
    // removeItem so the caller knows what to send instead.
    let path = start_save("setzero");
    let out = out_path("setzero");
    let data = list(&path);
    let (index, _) = stocked_trader(&data);

    let err = exec_err(json!({
        "command": "write_save",
        "payload": {
            "path": path,
            "outputPath": out,
            "backup": false,
            "edits": [
                { "path": "private.traders.setStock",
                  "value": { "index": index, "path": ORE, "count": 0 } },
            ]
        }
    }));
    assert!(err.contains("positive i32"), "{err}");
    assert!(err.contains("removeItem"), "{err}");
    assert!(!std::path::Path::new(&out).exists());
}

#[test]
fn add_item_rejects_a_zero_count() {
    // Sold-out lines are deleted, never left at zero, so a zero-count insert
    // would write a state the game never produces.
    let path = start_save("zerocount");
    let out = out_path("zerocount");
    let data = list(&path);
    let (index, _) = stocked_trader(&data);

    let err = exec_err(json!({
        "command": "write_save",
        "payload": {
            "path": path,
            "outputPath": out,
            "backup": false,
            "edits": [
                { "path": "private.traders.addItem",
                  "value": { "index": index, "path": "/Script/Angelscript.ItFo_Cheese", "count": 0 } },
            ]
        }
    }));
    assert!(err.contains("positive i32"), "{err}");
}
