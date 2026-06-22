//! Dump quest rows of a GSAV save as JSON lines (questClass|id|group|name).
//!
//! Usage: dump_quests <save.sav>

use goresave_core::execute_json;
use serde_json::{Value, json};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: dump_quests <save.sav>");
        std::process::exit(2);
    }
    let save = &args[1];
    let mut offset = 0usize;
    loop {
        let request = json!({
            "command": "query_progression",
            "payload": { "section": "quests", "path": save, "offset": offset, "limit": 1000 }
        });
        let response = execute_json(&request.to_string());
        let value: Value = serde_json::from_str(&response).expect("invalid JSON");
        if value["ok"] != json!(true) {
            eprintln!("error: {}", value["error"]);
            std::process::exit(1);
        }
        let data = &value["data"];
        let rows = data["quests"].as_array().cloned().unwrap_or_default();
        for q in &rows {
            println!(
                "{}\t{}\t{}\t{}",
                q["questClass"].as_str().unwrap_or(""),
                q["id"].as_str().unwrap_or(""),
                q["group"].as_str().unwrap_or(""),
                q["name"].as_str().unwrap_or("")
            );
        }
        let total = data["total"].as_u64().unwrap_or(0) as usize;
        offset += rows.len();
        if rows.is_empty() || offset >= total {
            break;
        }
    }
}
