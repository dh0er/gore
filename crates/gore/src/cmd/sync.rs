//! `gore-cli sync` — ingest a runtime game-data dump (produced by the gore-dump
//! UE4SS Lua mod in-game) and emit gore-mod's bundled `model.json`, carrying the
//! real CDO default values.
//!
//! Unlike `gui-model` (which sources field schema from the C++ header reflection
//! model and has no values), `sync` is the post-release refresh path: drop in a
//! fresh `game_data.json` and regenerate the GUI's data — including the real
//! per-field defaults that only the running game can provide.
//!
//! The dump uses the same on-disk shape gore-mod consumes
//! (`{ "classes": { "<id>": { "fields": [ {"name","type","default"?,"enum_values"?} ] } } }`),
//! so `sync` validates it, restricts it to the item catalog, and re-emits it.

use anyhow::{bail, Context, Result};
use gore_core::catalog::parse_catalog;
use std::{fs, path::PathBuf};

use crate::cmd::gui_model::{GuiClass, GuiField, GuiModel};

pub fn run(dump_path: PathBuf, catalog_path: PathBuf, out: PathBuf) -> Result<()> {
    let dump_json = fs::read_to_string(&dump_path)
        .with_context(|| format!("reading dump '{}'", dump_path.display()))?;
    let dump: GuiModel = serde_json::from_str(&dump_json)
        .context("parsing game-data dump (expected {\"classes\":{...}})")?;

    let catalog_json = fs::read_to_string(&catalog_path)
        .with_context(|| format!("reading catalog '{}'", catalog_path.display()))?;
    let catalog = parse_catalog(&catalog_json).context("parsing item_catalog.json")?;

    let model = build_model(&dump, catalog.iter().map(|e| e.id.as_str()))?;

    let missing = catalog
        .iter()
        .filter(|e| !dump.classes.contains_key(&e.id))
        .count();
    if missing > 0 {
        eprintln!(
            "warning: {missing} catalog item(s) absent from the dump (kept out of the model); \
             the dump may be from a different game version or was taken before those items loaded"
        );
    }

    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let json = serde_json::to_string_pretty(&model)?;
    fs::write(&out, format!("{json}\n"))
        .with_context(|| format!("writing '{}'", out.display()))?;
    println!(
        "Synced GUI model for {} classes -> {}",
        model.classes.len(),
        out.display()
    );
    Ok(())
}

/// Build the output model from the dump, keeping only the classes named in
/// `catalog_ids` and validating every field.
fn build_model<'a>(
    dump: &GuiModel,
    catalog_ids: impl Iterator<Item = &'a str>,
) -> Result<GuiModel> {
    let mut model = GuiModel::default();
    for id in catalog_ids {
        let Some(class) = dump.classes.get(id) else {
            continue;
        };
        let fields = class
            .fields
            .iter()
            .map(|f| validate_field(id, f))
            .collect::<Result<Vec<_>>>()?;
        model.classes.insert(id.to_string(), GuiClass { fields });
    }
    Ok(model)
}

/// Validate a single dumped field and return it unchanged. Rejects field types
/// the GUI cannot render and defaults whose JSON type doesn't match the field.
fn validate_field(class: &str, f: &GuiField) -> Result<GuiField> {
    match f.field_type.as_str() {
        "int" | "float" | "bool" | "enum" => {}
        other => bail!(
            "{class}.{}: unsupported field type '{other}' (expected int/float/bool/enum)",
            f.name
        ),
    }
    if let Some(d) = &f.default {
        let ok = match f.field_type.as_str() {
            "int" => d.is_i64() || d.is_u64(),
            "float" => d.is_number(),
            "bool" => d.is_boolean(),
            // Enum default is the backing integer (the CDO discriminant the
            // dumper read), NOT a member index. When the backing values are
            // known we can verify it's one of them; when they're absent we
            // cannot cross-check (and must not assume 0..n-1 indices), so accept
            // any integer rather than reject a correct dump.
            "enum" => match d.as_i64() {
                Some(v) => f.enum_value_ints.is_empty() || f.enum_value_ints.contains(&v),
                None => false,
            },
            _ => false,
        };
        if !ok {
            bail!(
                "{class}.{}: default {d} is not a valid '{}' value",
                f.name,
                f.field_type
            );
        }
    }
    Ok(f.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn dump_json() -> String {
        json!({
            "classes": {
                "ItFo_Apple": {"fields": [
                    {"name": "m_Value", "type": "int", "default": 4},
                    {"name": "m_Weight", "type": "float", "default": 0.5},
                    {"name": "m_AutoTarget", "type": "bool", "default": false},
                    {"name": "m_Quality", "type": "enum", "enum_values": ["Low", "High"], "default": 1}
                ]},
                // Present in the dump but NOT in the catalog -> must be dropped.
                "SomeInternalThing": {"fields": [{"name": "x", "type": "int", "default": 0}]}
            }
        })
        .to_string()
    }

    fn catalog_json() -> &'static str {
        r#"[
          {"category":"food","id":"ItFo_Apple","path":"/Script/Angelscript.ItFo_Apple"},
          {"category":"misc","id":"ItMi_Gone","path":"/Script/Angelscript.ItMi_Gone"}
        ]"#
    }

    fn parse_dump(s: &str) -> GuiModel {
        serde_json::from_str(s).unwrap()
    }

    #[test]
    fn keeps_only_catalog_items_and_carries_defaults() {
        let dump = parse_dump(&dump_json());
        let catalog = parse_catalog(catalog_json()).unwrap();
        let model = build_model(&dump, catalog.iter().map(|e| e.id.as_str())).unwrap();

        // ItMi_Gone is in the catalog but absent from the dump -> not present.
        // SomeInternalThing is in the dump but not the catalog -> dropped.
        assert_eq!(model.classes.keys().collect::<Vec<_>>(), ["ItFo_Apple"]);

        let apple = &model.classes["ItFo_Apple"];
        let value = apple.fields.iter().find(|f| f.name == "m_Value").unwrap();
        assert_eq!(value.default, Some(json!(4)));
        let quality = apple.fields.iter().find(|f| f.name == "m_Quality").unwrap();
        assert_eq!(quality.default, Some(json!(1)));
        assert_eq!(quality.enum_values, ["Low", "High"]);
    }

    #[test]
    fn round_trips_to_the_gore_mod_model_shape() {
        let dump = parse_dump(&dump_json());
        let catalog = parse_catalog(catalog_json()).unwrap();
        let model = build_model(&dump, catalog.iter().map(|e| e.id.as_str())).unwrap();
        let out = serde_json::to_string(&model).unwrap();
        // The emitted default survives a round-trip and uses the GUI key names.
        assert!(out.contains(r#""name":"m_Value","type":"int","default":4"#));
        assert!(out.contains(r#""enum_values":["Low","High"]"#));
    }

    #[test]
    fn rejects_default_type_mismatch() {
        let bad = parse_dump(
            &json!({"classes": {"ItFo_Apple": {"fields": [
                {"name": "m_Value", "type": "int", "default": "lots"}
            ]}}})
            .to_string(),
        );
        let err = build_model(&bad, ["ItFo_Apple"].into_iter()).unwrap_err();
        assert!(err.to_string().contains("m_Value"), "{err}");
    }

    #[test]
    fn enum_default_not_assumed_to_be_an_index_without_backing_values() {
        // No enum_value_ints -> we can't cross-check; the value is a real CDO
        // discriminant, not an index, so it must be accepted (not index-range
        // checked). Rejection only happens when backing values are known
        // (see enum_default_must_match_a_backing_value).
        let d = parse_dump(
            &json!({"classes": {"ItFo_Apple": {"fields": [
                {"name": "m_Q", "type": "enum", "enum_values": ["A", "B"], "default": 5}
            ]}}})
            .to_string(),
        );
        assert!(build_model(&d, ["ItFo_Apple"].into_iter()).is_ok());
    }

    #[test]
    fn enum_default_must_match_a_backing_value() {
        let ok = parse_dump(
            &json!({"classes": {"C": {"fields": [
                {"name": "q", "type": "enum", "enum_values": ["Low", "Mid"], "enum_value_ints": [0, 5], "default": 5}
            ]}}})
            .to_string(),
        );
        assert!(build_model(&ok, ["C"].into_iter()).is_ok());

        let bad = parse_dump(
            &json!({"classes": {"C": {"fields": [
                {"name": "q", "type": "enum", "enum_values": ["Low", "Mid"], "enum_value_ints": [0, 5], "default": 2}
            ]}}})
            .to_string(),
        );
        // 2 is a member index but not a backing value -> rejected.
        assert!(build_model(&bad, ["C"].into_iter()).is_err());
    }

    #[test]
    fn member_less_enum_default_is_tolerated() {
        // gore-dump can write a numeric CDO value for an enum the model never
        // resolved (no members); sync must not fail the whole run over it.
        let d = parse_dump(
            &json!({"classes": {"C": {"fields": [
                {"name": "q", "type": "enum", "default": 7}
            ]}}})
            .to_string(),
        );
        assert!(build_model(&d, ["C"].into_iter()).is_ok());
    }

    #[test]
    fn missing_default_is_allowed() {
        let d = parse_dump(
            &json!({"classes": {"ItFo_Apple": {"fields": [
                {"name": "m_Value", "type": "int"}
            ]}}})
            .to_string(),
        );
        let model = build_model(&d, ["ItFo_Apple"].into_iter()).unwrap();
        assert_eq!(model.classes["ItFo_Apple"].fields[0].default, None);
    }
}
