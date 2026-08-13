//! Scratch research driver: replay the exact command sequence the save editor's
//! tabs issue and time each one through the public FFI entry point. Read-only.
//! Not shipped.

use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).expect("usage: tab_timer <save.sav>");
    let esc = serde_json::to_string(&path).unwrap();

    // (label, command, payload-json-without-path)
    let steps: Vec<(&str, String)> = vec![
        (
            "inspect_save (initial load)",
            format!(r#"{{"command":"inspect_save","payload":{{"path":{esc},"includePrivate":true}}}}"#),
        ),
        (
            "check_codec",
            format!(r#"{{"command":"check_codec","payload":{{"path":{esc}}}}}"#),
        ),
        // Overview tab
        (
            "OVERVIEW loadGameTime (search 'GameTime')",
            format!(r#"{{"command":"search_typed_properties","payload":{{"path":{esc},"query":"GameTime","offset":0,"limit":1000}}}}"#),
        ),
        // Characters tab
        (
            "CHARACTERS loadAllCharacters",
            format!(r#"{{"command":"private.characters.list","payload":{{"path":{esc},"query":"","offset":0,"limit":100000}}}}"#),
        ),
        (
            "CHARACTERS loadHeroAttributes (search)",
            format!(r#"{{"command":"search_typed_properties","payload":{{"path":{esc},"query":"AttributesByGlobalId {{Hero}}","offset":0,"limit":1000}}}}"#),
        ),
        (
            "CHARACTERS loadSkills (Hero)",
            format!(r#"{{"command":"private.skills.list","payload":{{"path":{esc},"actor":"Hero"}}}}"#),
        ),
        (
            "CHARACTERS loadAllNpcActors",
            format!(r#"{{"command":"private.npc.list","payload":{{"path":{esc},"query":"","offset":0,"limit":100000}}}}"#),
        ),
        // World tab
        (
            "WORLD loadProgressionQuests",
            format!(r#"{{"command":"query_progression","payload":{{"path":{esc},"section":"quests","query":"","offset":0,"limit":100}}}}"#),
        ),
        (
            "WORLD loadGlossary",
            format!(r#"{{"command":"query_progression","payload":{{"path":{esc},"section":"glossary","offset":0,"limit":1000}}}}"#),
        ),
        (
            "WORLD loadProgressionTutorials",
            format!(r#"{{"command":"query_progression","payload":{{"path":{esc},"section":"tutorials","offset":0,"limit":100}}}}"#),
        ),
        (
            "WORLD loadStoryState",
            format!(r#"{{"command":"query_progression","payload":{{"path":{esc},"section":"story","query":"","offset":0,"limit":1000}}}}"#),
        ),
        (
            "WORLD loadFactions",
            format!(r#"{{"command":"private.factions.list","payload":{{"path":{esc}}}}}"#),
        ),
        (
            "WORLD loadKnowledgeEntries (Hero)",
            format!(r#"{{"command":"query_progression","payload":{{"path":{esc},"section":"knowledge","character":"Hero","query":"","offset":0,"limit":200}}}}"#),
        ),
        (
            "WORLD loadMemoryEvents (Hero)",
            format!(r#"{{"command":"query_progression","payload":{{"path":{esc},"section":"events","character":"Hero","query":"","offset":0,"limit":200}}}}"#),
        ),
        // All data tab
        (
            "ALLDATA browse (includeNodes)",
            format!(r#"{{"command":"search_typed_properties","payload":{{"path":{esc},"query":"","offset":0,"limit":50,"includeNodes":true,"source":"private"}}}}"#),
        ),
        // Trade sub-tab
        (
            "TRADE loadTraders",
            format!(r#"{{"command":"private.traders.list","payload":{{"path":{esc}}}}}"#),
        ),
        (
            "TRADE loadTraderDetail(0)",
            format!(r#"{{"command":"private.traders.detail","payload":{{"path":{esc},"index":0}}}}"#),
        ),
        (
            "TRADE loadTraderDetail(7)",
            format!(r#"{{"command":"private.traders.detail","payload":{{"path":{esc},"index":7}}}}"#),
        ),
        (
            "BACKUPS list_backups",
            format!(r#"{{"command":"list_backups","payload":{{"path":{esc}}}}}"#),
        ),
    ];

    // This box is rarely idle (a running game, a browser), and background load
    // inflates every sample. Repeat each step and report the MINIMUM, which is
    // the sample least contaminated by contention; the median is printed beside
    // it so a wide spread is visible rather than hidden.
    let runs: usize = std::env::args()
        .nth(2)
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);

    println!("save: {path}   ({runs} runs per step, reporting min)\n");
    println!("{:<44} {:>10} {:>10} {:>10}", "step", "min", "median", "resp KB");
    println!("{}", "-".repeat(78));

    let mut min_total = 0.0f64;
    for (label, request) in &steps {
        let mut samples = Vec::with_capacity(runs);
        let mut response = String::new();
        for _ in 0..runs {
            let t = Instant::now();
            response = gore_save::execute_json(request);
            samples.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let min = samples[0];
        let median = samples[samples.len() / 2];

        let ok = response.contains(r#""ok":true"#);
        min_total += min;
        println!(
            "{:<44} {:>8.1}ms {:>8.1}ms {:>10.0} {}",
            label,
            min,
            median,
            response.len() as f64 / 1024.0,
            if ok { "" } else { "  <-- FAILED" },
        );
        if !ok {
            let short: String = response.chars().take(160).collect();
            println!("      {short}");
        }
    }
    println!("{}", "-".repeat(78));
    println!("{:<44} {:>8.1}ms", "TOTAL (min)", min_total);
}
