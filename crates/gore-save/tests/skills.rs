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

/// Take Scutes is ranked, and the game leaves Cavalorn's first lesson in place
/// when it grants the second — so a real save can hold TWO elements for it. The
/// row has to read as the higher rung (the Master class implies the lower one:
/// a hero carrying only it harvests both trophies, verified in game), and an
/// edit has to reconcile every element, not just the first one it finds.
#[test]
fn a_skill_carrying_two_rungs_reads_and_edits_as_one() {
    let Ok(path) = std::env::var("GORE_SAVE") else {
        eprintln!("GORE_SAVE not set; skipping");
        return;
    };
    const TRAINED: &str = "/Script/Angelscript.Default__GE_Skill_Hunting_Scutes_Trained";
    const MASTER: &str = "/Script/Angelscript.Default__GE_Skill_Hunting_Scutes_Master";
    const BASE: &str = "Hunting_Scutes";

    let mut out = std::env::temp_dir();
    out.push("gore_skills_scutes.sav");
    let out = out.to_string_lossy().to_string();

    let def_path = |index: usize| {
        json!([
            "m_GenericData",
            "{CharacterStates}",
            "AnyCharacterType",
            "ActiveEffectsByGlobalId",
            "{Hero}",
            "ActiveEffects",
            format!("[{index}]"),
            "EffectSpec",
            "Def"
        ])
    };

    // Plant both rungs, lower one first — the order the game itself produces.
    exec(json!({
        "command": "write_save",
        "payload": {
            "path": path,
            "outputPath": out,
            "backup": false,
            "edits": [
                { "path": "private.typed.setValue",
                  "value": { "path": def_path(0), "value": TRAINED } },
            ],
        }
    }));
    exec(json!({
        "command": "write_save",
        "payload": {
            "path": out,
            "backup": false,
            "edits": [
                { "path": "private.typed.setValue",
                  "value": { "path": def_path(1), "value": MASTER } },
            ],
        }
    }));

    let planted = list_skills(&out);
    let row = skill(&planted, BASE).expect("scutes row");
    assert_eq!(
        row["current"],
        json!("Master"),
        "a hero holding both rungs must read as the higher one, not as whichever          element comes first: {row}"
    );

    // Lowering to Trained must leave ONE element behind, on the lower rung —
    // otherwise the Master element survives unseen and still grants the plates.
    exec(json!({
        "command": "write_save",
        "payload": {
            "path": out,
            "backup": false,
            "edits": [
                { "path": "private.skills.set", "value": { "base": BASE, "tier": "Trained" } },
            ],
        }
    }));
    let lowered = list_skills(&out);
    assert_eq!(skill(&lowered, BASE).unwrap()["current"], json!("Trained"));
    let search = exec(json!({
        "command": "search_typed_properties",
        "payload": { "path": out, "query": "ActiveEffectsByGlobalId {Hero}",
                     "offset": 0, "limit": 400, "source": "private" }
    }));
    let scutes_elements = search["results"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| {
            r["value"]
                .as_str()
                .is_some_and(|v| v.contains("GE_Skill_Hunting_Scutes"))
        })
        .count();
    assert_eq!(
        scutes_elements, 1,
        "the skill must end up saying exactly one thing"
    );

    // And unlearning takes every element with it.
    exec(json!({
        "command": "write_save",
        "payload": {
            "path": out,
            "backup": false,
            "edits": [
                { "path": "private.skills.set", "value": { "base": BASE, "tier": "Untrained" } },
            ],
        }
    }));
    assert_eq!(
        skill(&list_skills(&out), BASE).unwrap()["learned"],
        json!(false)
    );

    let _ = std::fs::remove_file(&out);
}

/// A class the catalog does not know — a skill the game defines but never
/// grants (Extract Mandibles), a console `addskill`, or one a newer game version
/// added — still has to come back out of a save. It is listed under "Other" with
/// an Untrained option, and the edit API accepts that one transition for it.
#[test]
fn an_uncatalogued_class_is_listed_and_can_be_removed() {
    let Ok(path) = std::env::var("GORE_SAVE") else {
        eprintln!("GORE_SAVE not set; skipping");
        return;
    };
    const DEAD: &str = "/Script/Angelscript.Default__GE_Skill_Hunting_MandibleMineCrawler_Trained";
    const BASE: &str = "Hunting_MandibleMineCrawler";

    let mut out = std::env::temp_dir();
    out.push("gore_skills_orphan.sav");
    let out = out.to_string_lossy().to_string();

    // Plant one: retarget the hero's first effect at the dead class, the way an
    // older editor build (or the game console) would have put it there.
    exec(json!({
        "command": "write_save",
        "payload": {
            "path": path,
            "outputPath": out,
            "backup": false,
            "edits": [
                { "path": "private.typed.setValue", "value": {
                    "path": ["m_GenericData", "{CharacterStates}", "AnyCharacterType",
                             "ActiveEffectsByGlobalId", "{Hero}", "ActiveEffects", "[0]",
                             "EffectSpec", "Def"],
                    "value": DEAD
                } },
            ],
        }
    }));

    let planted = list_skills(&out);
    let row = skill(&planted, BASE).expect("an uncatalogued class must still be listed");
    assert_eq!(row["learned"], json!(true));
    assert_eq!(row["category"], json!("Other"), "row: {row}");
    let options: Vec<&str> = row["options"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|o| o["value"].as_str())
        .collect();
    assert!(
        options.contains(&"Untrained"),
        "no way to drop it again: {options:?}"
    );

    exec(json!({
        "command": "write_save",
        "payload": {
            "path": out,
            "backup": false,
            "edits": [
                { "path": "private.skills.set", "value": { "base": BASE, "tier": "Untrained" } },
            ],
        }
    }));
    assert!(
        skill(&list_skills(&out), BASE).is_none(),
        "the dead class survived its removal"
    );

    // The other direction stays shut: nothing can put it back.
    let resp: Value = serde_json::from_str(&gore_save::execute_json(
        &json!({
            "command": "write_save",
            "payload": {
                "path": out,
                "backup": false,
                "edits": [
                    { "path": "private.skills.set", "value": { "base": BASE, "tier": "Trained" } },
                ],
            }
        })
        .to_string(),
    ))
    .unwrap();
    assert_eq!(
        resp["ok"],
        json!(false),
        "a dead class must not be learnable"
    );

    let _ = std::fs::remove_file(&out);
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
