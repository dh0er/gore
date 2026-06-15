//! Apply one private.typed.setValue edit to a save (use a throwaway copy).
//!
//! Usage: try_typed_edit <save.sav> <codec_host.exe> <game.exe> <path-json> <value>

use goresave_core::execute_json;
use serde_json::{Value, json};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 6 {
        eprintln!(
            "usage: try_typed_edit <save.sav> <codec_host.exe> <game.exe> <path-json> <value>"
        );
        std::process::exit(2);
    }
    let path_segments: Value = serde_json::from_str(&args[4]).expect("path-json must be JSON");
    let value: Value = serde_json::from_str(&args[5]).expect("value must be JSON");
    let request = json!({
        "command": "write_save",
        "payload": {
            "path": args[1],
            "backup": false,
            "edits": [
                {
                    "path": "private.typed.setValue",
                    "value": { "path": path_segments, "value": value },
                }
            ],
            "binaryHost": { "helperPath": args[2], "exePath": args[3] },
        }
    });
    println!("{}", execute_json(&request.to_string()));
}
