//! Dump every typed property of a GSAV save as JSON lines.
//!
//! Usage: dump_typed <save.sav> <codec_host.exe> <game.exe> [query]

use goresave_core::execute_json;
use serde_json::{Value, json};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("usage: dump_typed <save.sav> <codec_host.exe> <game.exe> [query]");
        std::process::exit(2);
    }
    let save = &args[1];
    let helper = &args[2];
    let game = &args[3];
    let query = args.get(4).cloned().unwrap_or_default();

    let mut offset = 0usize;
    loop {
        let request = json!({
            "command": "search_typed_properties",
            "payload": {
                "path": save,
                "query": query,
                "offset": offset,
                "limit": 1000,
                "binaryHost": { "helperPath": helper, "exePath": game },
            }
        });
        let response = execute_json(&request.to_string());
        let value: Value = serde_json::from_str(&response).expect("core returned invalid JSON");
        if value["ok"] != json!(true) {
            eprintln!("error: {}", value["error"]);
            std::process::exit(1);
        }
        let data = &value["data"];
        let results = data["results"].as_array().expect("results array");
        for hit in results {
            println!("{hit}");
        }
        let total = data["total"].as_u64().unwrap_or(0) as usize;
        offset += results.len();
        if offset >= total || results.is_empty() {
            break;
        }
    }
}
