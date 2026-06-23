//! Measure how many dialog-knowledge entries resolve to a loc_catalog string.
//!
//! Usage: loc_coverage <loc_catalog.json> <knowledge_dump.txt>
//!   knowledge_dump.txt = one entry per line (output of dump_knowledge)

use std::collections::HashSet;
use std::fs;

/// CamelCase / mixed → snake_case, collapsing existing separators.
fn to_snake(s: &str) -> String {
    let mut out = String::new();
    let mut prev_lower_or_digit = false;
    for ch in s.chars() {
        if ch == '_' || ch == '-' {
            if !out.ends_with('_') {
                out.push('_');
            }
            prev_lower_or_digit = false;
            continue;
        }
        if ch.is_ascii_uppercase() {
            if prev_lower_or_digit && !out.ends_with('_') {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            prev_lower_or_digit = false;
        } else if ch.is_ascii_digit() {
            // split letter<->digit boundary: catalog writes `stt_311`, not `stt311`
            if out.chars().last().is_some_and(|c| c.is_ascii_lowercase()) {
                out.push('_');
            }
            out.push(ch);
            prev_lower_or_digit = true;
        } else {
            // letter following a digit also gets a split: `311fisk` -> `311_fisk`
            if ch.is_ascii_lowercase() && out.chars().last().is_some_and(|c| c.is_ascii_digit()) {
                out.push('_');
            }
            out.push(ch);
            prev_lower_or_digit = ch.is_ascii_lowercase();
        }
    }
    out.trim_matches('_').to_string()
}

fn has_long_number(s: &str) -> bool {
    let mut run = 0;
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            run += 1;
            if run >= 4 {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

struct Catalog {
    exact: HashSet<String>,
    sorted: Vec<String>,
}

impl Catalog {
    fn has_exact(&self, k: &str) -> bool {
        self.exact.contains(k)
    }
    /// any key == prefix OR starts with `prefix_`
    fn has_prefix(&self, prefix: &str) -> bool {
        if self.exact.contains(prefix) {
            return true;
        }
        let needle = format!("{prefix}_");
        match self.sorted.binary_search(&needle) {
            Ok(_) => true,
            Err(i) => self
                .sorted
                .get(i)
                .map(|k| k.starts_with(&needle))
                .unwrap_or(false),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: loc_coverage <loc_catalog.json> <knowledge_dump.txt>");
        std::process::exit(2);
    }
    let cat_json = fs::read_to_string(&args[1]).expect("read catalog");
    let value: serde_json::Value = serde_json::from_str(&cat_json).expect("parse catalog");
    let obj = value.as_object().expect("catalog is object");
    let mut sorted: Vec<String> = obj.keys().cloned().collect();
    sorted.sort();
    let exact: HashSet<String> = sorted.iter().cloned().collect();
    let cat = Catalog { exact, sorted };
    eprintln!("catalog ids: {}", cat.sorted.len());

    let dump = fs::read_to_string(&args[2]).expect("read dump");
    let mut uniq: HashSet<String> = HashSet::new();
    for line in dump.lines() {
        let t = line.trim();
        if !t.is_empty() {
            uniq.insert(t.to_string());
        }
    }
    eprintln!("unique entries: {}", uniq.len());

    let (mut vl_hit, mut vl_miss) = (0u32, 0u32);
    let (mut camel_hit, mut camel_miss) = (0u32, 0u32);
    let (mut numeric, mut other_hit, mut other_miss) = (0u32, 0u32, 0u32);
    let mut miss_samples: Vec<String> = Vec::new();
    let mut camel_hit_samples: Vec<String> = Vec::new();

    for e in &uniq {
        let lower = e.to_lowercase();
        // 1) Voiceline_<id>_AlkimiaLocalization
        if lower.starts_with("voiceline_") {
            let mut inner = lower.trim_start_matches("voiceline_").to_string();
            if let Some(p) = inner.rfind("_alkimialocalization") {
                inner = inner[..p].to_string();
            }
            if cat.has_exact(&inner) {
                vl_hit += 1;
            } else {
                vl_miss += 1;
                if miss_samples.len() < 20 {
                    miss_samples.push(format!("VL  {e}  -> {inner}"));
                }
            }
            continue;
        }
        // 2) Topic_* / Choice* with semantic name (no long number) -> info_<snake>
        let is_topic = e.starts_with("Topic_") || e.starts_with("Topic");
        let is_choice = e.starts_with("Choice");
        if is_topic || is_choice {
            if has_long_number(e) {
                numeric += 1;
                continue;
            }
            let mut body = e.as_str();
            for p in ["Topic_", "Topic", "Choice_", "Choice"] {
                if let Some(s) = body.strip_prefix(p) {
                    body = s;
                    break;
                }
            }
            let snake = to_snake(body);
            let cand = format!("info_{snake}");
            // try info_ prefix, then bare snake, then dia_ prefix
            if cat.has_prefix(&cand) || cat.has_prefix(&snake) || cat.has_prefix(&format!("dia_{snake}"))
            {
                camel_hit += 1;
                if camel_hit_samples.len() < 15 {
                    camel_hit_samples.push(format!("{e}  -> {cand}*"));
                }
            } else {
                camel_miss += 1;
                if miss_samples.len() < 20 {
                    miss_samples.push(format!("TC  {e}  -> {cand}"));
                }
            }
            continue;
        }
        // 3) anything else
        if has_long_number(e) {
            numeric += 1;
            continue;
        }
        let snake = to_snake(e);
        // snake-form choice/topic entries map to info_<rest> too
        let body = snake
            .strip_prefix("choice_")
            .or_else(|| snake.strip_prefix("topic_"))
            .unwrap_or(&snake)
            .to_string();
        if cat.has_prefix(&snake)
            || cat.has_prefix(&format!("info_{body}"))
            || cat.has_prefix(&format!("dia_{body}"))
            || cat.has_prefix(&format!("info_{snake}"))
            || cat.has_prefix(&format!("dia_{snake}"))
        {
            other_hit += 1;
        } else {
            other_miss += 1;
            if miss_samples.len() < 20 {
                miss_samples.push(format!("OT  {e}  -> {snake}"));
            }
        }
    }

    let total = uniq.len() as u32;
    let resolvable = vl_hit + camel_hit + other_hit;
    println!("=== coverage over {total} unique entries ===");
    println!("Voiceline_*  hit={vl_hit:5}  miss={vl_miss:5}");
    println!("Topic/Choice hit={camel_hit:5}  miss={camel_miss:5}  (semantic-name)");
    println!("other        hit={other_hit:5}  miss={other_miss:5}");
    println!("numeric (no text, skipped) = {numeric}");
    println!(
        "RESOLVABLE = {resolvable} / {total}  ({:.1}%)",
        100.0 * resolvable as f64 / total as f64
    );
    println!("\n--- camel/topic hit samples ---");
    for s in &camel_hit_samples {
        println!("  {s}");
    }
    println!("\n--- miss samples ---");
    for s in &miss_samples {
        println!("  {s}");
    }
}
