//! Requires a real save via GORE_SAVE; skips otherwise.
//!   GORE_SAVE='C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames\G1R-021.sav' \
//!     cargo test -p gore-save --test characters_list -- --nocapture
use serde_json::{json, Value};

fn data(path: &str, command: &str, section: Option<&str>, offset: usize) -> Value {
    let mut payload = json!({ "path": path, "offset": offset, "limit": 1000 });
    if let Some(s) = section { payload["section"] = json!(s); }
    let req = json!({ "command": command, "payload": payload }).to_string();
    let resp: Value = serde_json::from_str(&gore_save::execute_json(&req)).unwrap();
    assert_eq!(resp["ok"], json!(true), "{command} failed: {resp}");
    resp["data"].clone()
}

#[test]
fn characters_list_matches_npc_and_knowledge_sets() {
    let Ok(path) = std::env::var("GORE_SAVE") else {
        eprintln!("GORE_SAVE not set; skipping"); return;
    };
    let chars = data(&path, "private.characters.list", None, 0);
    let rows = chars["characters"].as_array().unwrap();
    for r in rows {
        if r["globalId"].is_null() {
            assert_eq!(r["hasKnowledge"], json!(true), "orphan must have knowledge");
        }
    }
    let npc_total = data(&path, "private.npc.list", None, 0)["total"].as_u64().unwrap();
    let non_orphan = rows.iter().filter(|r| !r["globalId"].is_null()).count() as u64;
    assert_eq!(non_orphan, npc_total, "non-orphan rows must equal actor count");
    eprintln!("characters: {}, actors: {npc_total}", rows.len());
}
