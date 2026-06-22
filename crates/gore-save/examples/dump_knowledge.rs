//! Dump every dialog-knowledge entry string of a GSAV save, one per line.
//!
//! Usage: dump_knowledge <save.sav>

use gore_save::execute_json;
use serde_json::{Value, json};

fn query(save: &str, character: Option<&str>, offset: usize) -> Value {
    let mut payload = json!({
        "section": "knowledge",
        "path": save,
        "offset": offset,
        "limit": 1000,
    });
    if let Some(c) = character {
        payload["character"] = json!(c);
    }
    let request = json!({ "command": "query_progression", "payload": payload });
    let response = execute_json(&request.to_string());
    let value: Value = serde_json::from_str(&response).expect("core returned invalid JSON");
    if value["ok"] != json!(true) {
        eprintln!("error: {}", value["error"]);
        std::process::exit(1);
    }
    value["data"].clone()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: dump_knowledge <save.sav>");
        std::process::exit(2);
    }
    let save = &args[1];

    // 1) all characters
    let mut characters: Vec<String> = Vec::new();
    let mut offset = 0usize;
    loop {
        let data = query(save, None, offset);
        let page = data["characters"].as_array().cloned().unwrap_or_default();
        for c in &page {
            if let Some(name) = c["name"].as_str() {
                characters.push(name.to_string());
            }
        }
        let total = data["total"].as_u64().unwrap_or(0) as usize;
        offset += page.len();
        if page.is_empty() || offset >= total {
            break;
        }
    }
    eprintln!("characters: {}", characters.len());

    // 2) entries per character
    for ch in &characters {
        let mut offset = 0usize;
        loop {
            let data = query(save, Some(ch), offset);
            let entries = data["entries"].as_array().cloned().unwrap_or_default();
            for e in &entries {
                if let Some(s) = e.as_str() {
                    println!("{s}");
                }
            }
            let total = data["total"].as_u64().unwrap_or(0) as usize;
            offset += entries.len();
            if entries.is_empty() || offset >= total {
                break;
            }
        }
    }
}
