//! Validate quest-name loc mapping: quest-<lower(id minus Quest_)>-name
//!
//! Usage: quest_coverage <loc_catalog.json> <quests_dump.tsv>

use std::collections::HashSet;
use std::fs;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cat_json = fs::read_to_string(&args[1]).expect("read catalog");
    let value: serde_json::Value = serde_json::from_str(&cat_json).expect("parse catalog");
    let keys: HashSet<String> = value.as_object().unwrap().keys().cloned().collect();

    let dump = fs::read_to_string(&args[2]).expect("read dump");
    let (mut name_hit, mut name_miss, mut desc_hit) = (0u32, 0u32, 0u32);
    let mut total = 0u32;
    let mut miss_samples: Vec<String> = Vec::new();
    for line in dump.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 2 {
            continue;
        }
        let id = cols[1];
        total += 1;
        let body = id.strip_prefix("Quest_").unwrap_or(id).to_lowercase();
        let name_key = format!("quest-{body}-name");
        let desc_key = format!("quest-{body}-description");
        if keys.contains(&name_key) {
            name_hit += 1;
        } else {
            name_miss += 1;
            if miss_samples.len() < 25 {
                miss_samples.push(format!("{id}  -> {name_key}"));
            }
        }
        if keys.contains(&desc_key) {
            desc_hit += 1;
        }
    }
    println!("=== quest coverage over {total} quests ===");
    println!(
        "name hit={name_hit}  miss={name_miss}  ({:.1}%)",
        100.0 * name_hit as f64 / total as f64
    );
    println!("description present = {desc_hit}");
    println!("\n--- name miss samples ---");
    for s in &miss_samples {
        println!("  {s}");
    }
}
