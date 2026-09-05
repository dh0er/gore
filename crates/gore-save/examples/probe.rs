//! Eine Sonde in einen Spielstand, für Fragen, die nur er beantwortet.
//!
//! `gore-save` ist eine Bibliothek für den Editor und hängt bewusst nicht an der `gore`-CLI. Für
//! eine einzelne Messung — steht diese Figur im Spielstand, und wo — ist der Editor zu umständlich
//! und ein neues Kommando zu viel. Dieses Beispiel reicht eine Anfrage durch und druckt die
//! Antwort.
//!
//! ```text
//! cargo run -p gore-save --example probe -- <save.sav> <command> [suchtext]
//! cargo run -p gore-save --example probe -- G1R-003.sav private.characters.list GORE_TEST
//! ```

use std::env;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let (Some(path), Some(command)) = (args.first(), args.get(1)) else {
        eprintln!("usage: probe <save.sav> <command> [needle]");
        std::process::exit(2);
    };
    let needle = args.get(2).map(String::as_str);

    // Ein Zusatzargument der Form `schluessel=wert`, weil die interessanten Lesebefehle eine Id
    // brauchen: `private.npc.position` etwa fragt nach genau einer Figur.
    let mut payload = serde_json::json!({ "path": path });
    if let Some(extra) = args.get(3) {
        if let Some((key, value)) = extra.split_once('=') {
            payload[key] = serde_json::json!(value);
        }
    }

    let request = serde_json::json!({
        "command": command,
        "payload": payload,
    })
    .to_string();

    let response = gore_save::execute_json(&request);
    let parsed: serde_json::Value = match serde_json::from_str(&response) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("the response was not JSON: {error}\n{response}");
            std::process::exit(1);
        }
    };

    if parsed["ok"] != serde_json::json!(true) {
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&parsed).unwrap_or(response)
        );
        std::process::exit(1);
    }

    // Ohne Suchtext das Ganze; mit Suchtext nur die Zeilen, die ihn tragen — eine
    // Charakterliste hat über tausend Einträge und der Rest ist hier Rauschen.
    let text = serde_json::to_string_pretty(&parsed["data"]).unwrap_or_default();
    match needle {
        None => println!("{text}"),
        Some(needle) => {
            let mut hits = 0;
            for line in text.lines() {
                if line.contains(needle) {
                    println!("{}", line.trim_end());
                    hits += 1;
                }
            }
            println!("--- {hits} line(s) carrying {needle:?} ---");
        }
    }
}
