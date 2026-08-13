//! Scratch research driver: dump the full response of every read command the
//! save editor issues, so an optimized core can be diffed against the previous
//! one. Read-only; writes only the dump file. Not shipped.

fn main() {
    let path = std::env::args().nth(1).expect("usage: cmd_dump <save.sav> <out.txt>");
    let out = std::env::args().nth(2).expect("usage: cmd_dump <save.sav> <out.txt>");
    let esc = serde_json::to_string(&path).unwrap();

    let requests: Vec<String> = vec![
        format!(r#"{{"command":"inspect_save","payload":{{"path":{esc},"includePrivate":true}}}}"#),
        format!(r#"{{"command":"inspect_save","payload":{{"path":{esc},"includePrivate":true,"privateChunkLimit":4}}}}"#),
        format!(r#"{{"command":"inspect_save","payload":{{"path":{esc}}}}}"#),
        format!(r#"{{"command":"search_typed_properties","payload":{{"path":{esc},"query":"GameTime","offset":0,"limit":1000}}}}"#),
        format!(r#"{{"command":"search_typed_properties","payload":{{"path":{esc},"query":"AttributesByGlobalId {{Hero}}","offset":0,"limit":1000}}}}"#),
        format!(r#"{{"command":"search_typed_properties","payload":{{"path":{esc},"query":"","offset":900000,"limit":200}}}}"#),
        format!(r#"{{"command":"search_typed_properties","payload":{{"path":{esc},"query":"","offset":0,"limit":50,"includeNodes":true,"source":"private"}}}}"#),
        format!(r#"{{"command":"private.characters.list","payload":{{"path":{esc},"query":"","offset":0,"limit":100000}}}}"#),
        format!(r#"{{"command":"private.skills.list","payload":{{"path":{esc},"actor":"Hero"}}}}"#),
        format!(r#"{{"command":"private.npc.list","payload":{{"path":{esc},"query":"","offset":0,"limit":1000}}}}"#),
        format!(r#"{{"command":"private.factions.list","payload":{{"path":{esc}}}}}"#),
        format!(r#"{{"command":"query_progression","payload":{{"path":{esc},"section":"quests","query":"","offset":0,"limit":1000}}}}"#),
        format!(r#"{{"command":"query_progression","payload":{{"path":{esc},"section":"glossary","offset":0,"limit":1000}}}}"#),
        format!(r#"{{"command":"query_progression","payload":{{"path":{esc},"section":"tutorials","offset":0,"limit":1000}}}}"#),
        format!(r#"{{"command":"query_progression","payload":{{"path":{esc},"section":"story","query":"","offset":0,"limit":1000}}}}"#),
        format!(r#"{{"command":"query_progression","payload":{{"path":{esc},"section":"knowledge","character":"Hero","query":"","offset":0,"limit":1000}}}}"#),
        format!(r#"{{"command":"query_progression","payload":{{"path":{esc},"section":"events","character":"Hero","query":"","offset":0,"limit":1000}}}}"#),
    ];

    let mut text = String::new();
    for request in &requests {
        let response = gore_save::execute_json(request);
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        text.push_str(&format!("=== {request}\n"));
        text.push_str(&serde_json::to_string_pretty(&parsed).unwrap());
        text.push('\n');
    }
    std::fs::write(&out, &text).expect("write dump");
    println!("wrote {out} ({} bytes)", text.len());
}
