//! Round-trip probe: add or remove an inventory item on a save copy, then
//! re-inspect.
//!
//! Usage: try_add_item <save.sav> <codec_host.exe> <game.exe> <item_path> <count> [add|remove]

use goresave_core::execute_json;
use serde_json::{json, Value};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 6 {
        eprintln!(
            "usage: try_add_item <save.sav> <codec_host.exe> <game.exe> <item_path> <count> [add|remove]"
        );
        std::process::exit(2);
    }
    let host = json!({ "helperPath": args[2], "exePath": args[3] });
    let count: i64 = args[5].parse().expect("count");
    let op = args.get(6).map(String::as_str).unwrap_or("add");
    let edit = if op == "remove" {
        json!({ "path": "private.inventory.removeItem",
                "value": { "path": args[4] } })
    } else {
        json!({ "path": "private.inventory.addItem",
                "value": { "path": args[4], "count": count } })
    };

    let write = json!({
        "command": "write_save",
        "payload": {
            "path": args[1],
            "backup": false,
            "edits": [edit],
            "binaryHost": host,
        }
    });
    let write_out = execute_json(&write.to_string());
    let wv: Value = serde_json::from_str(&write_out).unwrap();
    eprintln!(
        "WRITE ok={} editsApplied={} bytesChanged={}",
        wv["ok"], wv["data"]["editsApplied"], wv["data"]["bytesChanged"]
    );

    let inspect = json!({
        "command": "inspect_save",
        "payload": { "path": args[1], "includePrivate": true, "binaryHost": host }
    });
    let insp_out = execute_json(&inspect.to_string());
    let iv: Value = serde_json::from_str(&insp_out).unwrap();
    let inv = &iv["data"]["private"]["inventory"];
    let items = inv["items"].as_array().cloned().unwrap_or_default();
    let found = items
        .iter()
        .any(|it| it["path"].as_str() == Some(args[4].as_str()));
    eprintln!(
        "INSPECT scope={} itemStackCount={} returnedItems={} contains_added={}",
        inv["itemScope"], inv["itemStackCount"], items.len(), found
    );
    if let Some(it) = items
        .iter()
        .find(|it| it["path"].as_str() == Some(args[4].as_str()))
    {
        eprintln!("ADDED ITEM: {it}");
    }
}
