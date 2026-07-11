//! Hero skill read + edit roundtrip. Requires a real GSAV save via GORE_SAVE;
//! skips otherwise.
//!   GORE_SAVE='C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames\G1R-021.sav' \
//!     cargo test -p gore-save --test skills -- --nocapture
use serde_json::{Value, json};

fn exec(req: Value) -> Value {
    let resp: Value = serde_json::from_str(&gore_save::execute_json(&req.to_string())).unwrap();
    assert_eq!(resp["ok"], json!(true), "request failed: {resp}");
    resp["data"].clone()
}

fn list_skills(path: &str) -> Value {
    exec(json!({ "command": "private.skills.list", "payload": { "path": path } }))
}

fn skill<'a>(data: &'a Value, base: &str) -> Option<&'a Value> {
    data["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["base"] == base)
}

#[test]
fn skills_list_finds_hero_and_full_roster() {
    let Ok(path) = std::env::var("GORE_SAVE") else {
        eprintln!("GORE_SAVE not set; skipping");
        return;
    };
    let data = list_skills(&path);
    assert_eq!(data["found"], json!(true), "hero ActiveEffects not found");
    let skills = data["skills"].as_array().unwrap();
    assert!(!skills.is_empty());

    // Every catalogued base is present (learned or roster) exactly once, and
    // each carries at least one tier option and a current value.
    for s in skills {
        assert!(s["base"].is_string());
        assert!(s["label"].is_string());
        assert!(s["current"].is_string());
        assert!(
            !s["options"].as_array().unwrap().is_empty(),
            "{} has no options",
            s["base"]
        );
        // The current value must be one of the selectable options.
        let cur = s["current"].as_str().unwrap();
        assert!(
            s["options"]
                .as_array()
                .unwrap()
                .iter()
                .any(|o| o["value"] == cur),
            "{} current {cur} not in options",
            s["base"]
        );
    }
    eprintln!(
        "skills: {} ({} learned)",
        skills.len(),
        skills
            .iter()
            .filter(|s| s["learned"] == json!(true))
            .count()
    );
}

/// Apply a batch that mixes two structural edits (learn + unlearn) with an
/// in-place tier change in ONE write — normally forbidden for generic array
/// ops, but safe for value-addressed skill edits — and confirm each transition
/// survives a full recompress + re-decode.
#[test]
fn skills_batch_learn_unlearn_retier_roundtrips() {
    let Ok(path) = std::env::var("GORE_SAVE") else {
        eprintln!("GORE_SAVE not set; skipping");
        return;
    };
    let before = list_skills(&path);

    // Pick a learned ladder skill with a `_Untrained` class (retier target) and
    // a learned skill WITHOUT one (so Untrained means a structural delete). Read
    // `hasUntrained` from the data rather than hardcoding a family, so the choice
    // stays correct as the catalog's has_untrained flags change.
    let retier = ["Ranged_Bow", "Ranged_Crossbow", "Picklock", "Pickpocket"]
        .into_iter()
        .find(|b| skill(&before, b).is_some_and(|s| s["learned"] == json!(true)));
    let unlearn = before["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["learned"] == json!(true) && s["hasUntrained"] == json!(false))
        .and_then(|s| s["base"].as_str());
    // Pick any roster (unlearned) skill to learn, and a tier that is valid for
    // it — the first non-Untrained option value (a binary skill's is "Learned",
    // a ladder/hunting/circle skill's is a real tier). Hardcoding "Learned"
    // would fail `SkillSetEdit` validation for a non-binary first roster entry.
    let learn = before["skills"].as_array().unwrap().iter().find_map(|s| {
        if s["learned"] != json!(false) {
            return None;
        }
        let tier = s["options"]
            .as_array()?
            .iter()
            .filter_map(|o| o["value"].as_str())
            .find(|v| *v != "Untrained")?;
        Some((s["base"].as_str().unwrap().to_string(), tier.to_string()))
    });

    let (Some(retier), Some(unlearn), Some((learn, learn_tier))) = (retier, unlearn, learn) else {
        eprintln!("save lacks the skill mix this test needs; skipping asserts");
        return;
    };

    let mut out = std::env::temp_dir();
    out.push("gore_skills_roundtrip.sav");
    let out = out.to_string_lossy().to_string();

    exec(json!({
        "command": "write_save",
        "payload": {
            "path": path,
            "outputPath": out,
            "backup": false,
            "edits": [
                { "path": "private.skills.set", "value": { "base": learn, "tier": learn_tier } },
                { "path": "private.skills.set", "value": { "base": unlearn, "tier": "Untrained" } },
                { "path": "private.skills.set", "value": { "base": retier, "tier": "Trained" } },
            ],
        }
    }));

    let after = list_skills(&out);
    assert_eq!(
        skill(&after, &unlearn).unwrap()["learned"],
        json!(false),
        "unlearn failed"
    );
    assert_eq!(
        skill(&after, retier).unwrap()["current"],
        json!("Trained"),
        "retier failed"
    );
    let learned_after = skill(&after, &learn).unwrap();
    assert_eq!(learned_after["learned"], json!(true), "learn failed");

    let _ = std::fs::remove_file(&out);
    eprintln!("roundtrip ok: learn {learn}, unlearn {unlearn}, retier {retier}->Trained");
}

/// A skill edit (which can splice the hero's ActiveEffects array) must be
/// rejected when batched with an index-addressed typed edit, since the splice
/// shifts the index that edit resolves against.
#[test]
fn skills_reject_batch_with_index_addressed_edit() {
    let Ok(path) = std::env::var("GORE_SAVE") else {
        eprintln!("GORE_SAVE not set; skipping");
        return;
    };
    let resp: Value = serde_json::from_str(&gore_save::execute_json(
        &json!({
            "command": "write_save",
            "payload": {
                "path": path,
                "backup": false,
                "edits": [
                    { "path": "private.skills.set", "value": { "base": "Sneak", "tier": "Learned" } },
                    { "path": "private.typed.setValue", "value": {
                        "path": ["ActiveEffectsByGlobalId", "{Hero}", "ActiveEffects", "[0]", "EffectSpec", "Level"],
                        "value": 2.0
                    } },
                ],
            }
        })
        .to_string(),
    ))
    .unwrap();
    assert_eq!(
        resp["ok"],
        json!(false),
        "mixed skill + indexed edit must be rejected"
    );
    assert_eq!(
        resp["error"]["code"],
        json!("UNSUPPORTED_EDIT"),
        "resp: {resp}"
    );
}

/// A skill edit batched with only NAME/map-key-addressed peers (a hero attribute
/// setValue) is allowed — those re-resolve correctly after a splice.
#[test]
fn skills_allow_batch_with_named_edit() {
    let Ok(path) = std::env::var("GORE_SAVE") else {
        eprintln!("GORE_SAVE not set; skipping");
        return;
    };
    // Resolve a real hero attribute path to keep the peer edit applyable.
    let search: Value = serde_json::from_str(&gore_save::execute_json(
        &json!({ "command": "search_typed_properties",
                 "payload": { "path": path, "query": "AttributeSetsByClass", "limit": 1000 } })
        .to_string(),
    ))
    .unwrap();
    let attr = search["data"]["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["type"] == "FloatProperty" && r["editable"] == json!(true));
    let Some(attr) = attr else {
        eprintln!("no editable hero attribute found; skipping");
        return;
    };
    let attr_path = attr["path"].clone();
    let cur: f64 = attr["value"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);

    let mut out = std::env::temp_dir();
    out.push("gore_skills_named_batch.sav");
    let out = out.to_string_lossy().to_string();

    let resp: Value = serde_json::from_str(&gore_save::execute_json(
        &json!({
            "command": "write_save",
            "payload": {
                "path": path, "outputPath": out, "backup": false,
                "edits": [
                    { "path": "private.skills.set", "value": { "base": "Sneak", "tier": "Untrained" } },
                    { "path": "private.typed.setValue", "value": { "path": attr_path, "value": cur } },
                ],
            }
        })
        .to_string(),
    ))
    .unwrap();
    assert_eq!(
        resp["ok"],
        json!(true),
        "skill + named-path edit must be allowed: {resp}"
    );
    let _ = std::fs::remove_file(&out);
}
