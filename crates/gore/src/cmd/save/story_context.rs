//! Editor glossary context for story rows, resolved before search/pagination.
use super::*;
use std::collections::{HashMap, HashSet};

struct GlossaryLink {
    npc_catalog_id: String,
    unique_name: String,
    segment_id: String,
    segment_label: String,
    segment_class: String,
    lower_class: String,
    text_ids: Vec<String>,
}

/// Attach context without replacing any story identifiers, values or paths.
/// The caller must collect unqueried rows first and filter/page them afterward.
pub(super) fn annotate(data: &mut Value, o: &Options) -> Result<()> {
    let links = catalog_links(
        &display::catalog("glossary")?,
        &display::catalog("glossary-text")?,
    )?;
    annotate_links(data, &links, &text::Texts::load_options(o)?)
}

fn catalog_links(npcs: &Value, segment_texts: &Value) -> Result<Vec<GlossaryLink>> {
    // Match the Editor loaders: retain nonblank string text ids in order and
    // normalize class lookup keys, while leaving the original ids untouched.
    let mut text_ids = HashMap::new();
    for (class, ids) in segment_texts
        .as_object()
        .context("invalid glossary segment text catalog")?
    {
        let Some(ids) = ids.as_array() else {
            continue;
        };
        let ids = ids
            .iter()
            .filter_map(Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if !class.is_empty() && !ids.is_empty() {
            text_ids.insert(class.to_lowercase(), ids);
        }
    }

    let mut links = Vec::new();
    for npc in npcs.as_array().context("invalid NPC glossary catalog")? {
        let unique_name = npc["uniqueName"].as_str().unwrap_or("");
        let document_class = npc["documentClass"].as_str().unwrap_or("");
        let Some(segments) = npc["segments"].as_array() else {
            continue;
        };
        if document_class.is_empty() || unique_name.is_empty() {
            continue;
        }
        for segment in segments.iter().filter(|segment| segment.is_object()) {
            let class = segment["class"].as_str().unwrap_or("");
            let lower_class = class.to_lowercase();
            links.push(GlossaryLink {
                npc_catalog_id: npc["id"].as_str().unwrap_or("").into(),
                unique_name: unique_name.into(),
                segment_id: segment["id"].as_str().unwrap_or("").into(),
                segment_label: segment["label"].as_str().unwrap_or("").into(),
                segment_class: class.into(),
                text_ids: text_ids.get(&lower_class).cloned().unwrap_or_default(),
                lower_class,
            });
        }
    }
    Ok(links)
}

fn find_story_glossary_link<'a>(
    story_id: &str,
    links: &'a [GlossaryLink],
) -> Option<&'a GlossaryLink> {
    // Port findStoryGlossaryLink: only the complete underscore-delimited
    // technical suffix supplies evidence. A second match always refuses a link,
    // including duplicate segments within the same NPC.
    let suffix = format!("_{}", story_id.to_lowercase());
    let mut matches = links
        .iter()
        .filter(|link| link.lower_class.ends_with(&suffix));
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

fn annotate_links(data: &mut Value, links: &[GlossaryLink], texts: &text::Texts) -> Result<()> {
    let entries = data["entries"]
        .as_array_mut()
        .context("invalid story entries")?;
    for entry in entries {
        let Some(link) = entry["id"]
            .as_str()
            .and_then(|id| find_story_glossary_link(id, links))
        else {
            continue;
        };
        // StoryGlossaryLink uses uniqueName, then npcCatalogId, then a
        // humanized final id component. The broader character_name heuristic
        // has different fallbacks and must not supply story glossary context.
        let npc_name = texts
            .get(&link.unique_name)
            .or_else(|| texts.get(&link.npc_catalog_id))
            .unwrap_or_else(|| {
                humanize_story_id(link.npc_catalog_id.rsplit('_').next().unwrap_or(""))
            });
        let mut seen = HashSet::new();
        let paragraphs = link
            .text_ids
            .iter()
            .filter_map(|id| texts.get(id))
            .map(|paragraph| paragraph.trim().to_owned())
            .filter(|paragraph| !paragraph.is_empty() && seen.insert(paragraph.clone()))
            .collect::<Vec<_>>();
        entry["glossaryLink"] = json!({
            "npcCatalogId": link.npc_catalog_id,
            "uniqueName": link.unique_name,
            "segmentId": link.segment_id,
            "segmentLabel": link.segment_label,
            "segmentClass": link.segment_class,
            "textIds": link.text_ids,
            "npcName": npc_name,
            "paragraphs": paragraphs
        });
    }
    Ok(())
}

// Editor humanizeStoryId preserves acronyms and digits; text::readable would
// change both. This is only the NPC-name fallback, never a linking heuristic.
fn humanize_story_id(value: &str) -> String {
    let mut spaced = String::new();
    let mut previous = None::<char>;
    for c in value.chars() {
        if c == '_' {
            spaced.push(' ');
        } else {
            if c.is_ascii_uppercase()
                && previous.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit())
            {
                spaced.push(' ');
            }
            spaced.push(c);
        }
        previous = Some(c);
    }
    let spaced = spaced.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = spaced.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => value.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real_links() -> Vec<GlossaryLink> {
        catalog_links(
            &display::catalog("glossary").unwrap(),
            &display::catalog("glossary-text").unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn stone_story_ids_link_to_actual_npc_segments_and_ordered_text_ids() {
        let links = real_links();
        for (story_id, segment, label, text_id) in [
            (
                "Stone_OreArmor",
                "OreArmor",
                "Ore Armor",
                "TEXT_WIP_TMNINLL_20250515_115330",
            ),
            (
                "Stone_Teacher",
                "Teacher",
                "Teacher",
                "TEXT_WIP_TMNINLL_20250515_113030",
            ),
        ] {
            let link = find_story_glossary_link(story_id, &links).unwrap();
            assert_eq!(link.npc_catalog_id, "OCR_GRD_STONE");
            assert_eq!(link.unique_name, "OCR_GRD_STONE_219");
            assert_eq!(link.segment_id, segment);
            assert_eq!(link.segment_label, label);
            assert_eq!(
                link.segment_class,
                format!("/Script/Angelscript.DocumentSegment_Glossary_OCR_GRD_STONE_{segment}")
            );
            assert_eq!(link.text_ids, [text_id]);
        }
    }

    #[test]
    fn suffix_matching_accepts_case_changes_without_rewriting_underscores() {
        let links = real_links();
        let link = find_story_glossary_link("sToNe_oReArMoR", &links).unwrap();
        assert_eq!(link.segment_id, "OreArmor");
        for unrelated in [
            "Stone",
            "StoneOreArmor",
            "Stone_Ore_Armor",
            "Stone__OreArmor",
            "_Stone_OreArmor",
            "tone_OreArmor",
            "Stone_OreArmor_Day",
            "Stone_OreArmor ",
        ] {
            assert!(
                find_story_glossary_link(unrelated, &links).is_none(),
                "must not infer a link for {unrelated}"
            );
        }
    }

    #[test]
    fn multiple_matching_segments_refuse_links_even_within_one_npc() {
        let mut npcs = display::catalog("glossary").unwrap();
        let texts = display::catalog("glossary-text").unwrap();
        let stone = npcs
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|npc| npc["id"] == "OCR_GRD_STONE")
            .unwrap();
        let ore_armor = stone["segments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|segment| segment["id"] == "OreArmor")
            .unwrap()
            .clone();
        stone["segments"]
            .as_array_mut()
            .unwrap()
            .push(ore_armor.clone());
        let links = catalog_links(&npcs, &texts).unwrap();
        assert!(find_story_glossary_link("Stone_OreArmor", &links).is_none());
        assert!(find_story_glossary_link("Stone_Teacher", &links).is_some());

        let mut npcs = display::catalog("glossary").unwrap();
        npcs.as_array_mut().unwrap().push(json!({
            "id": "OtherNpc", "uniqueName": "OtherNpc_1",
            "documentClass": "Document_OtherNpc", "segments": [ore_armor]
        }));
        let links = catalog_links(&npcs, &texts).unwrap();
        assert!(find_story_glossary_link("Stone_OreArmor", &links).is_none());
        assert!(find_story_glossary_link("Stone_Teacher", &links).is_some());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn localization_makes_linked_context_searchable_before_pagination() {
        const CHILD: &str = "GORE_TEST_STORY_CONTEXT_LOCALIZATION_CHILD";
        const TEST: &str = "cmd::save::story_context::tests::localization_makes_linked_context_searchable_before_pagination";
        if std::env::var_os(CHILD).is_none() {
            // Isolate the cache in a child harness so parallel tests and the
            // user's extracted game catalog cannot affect these assertions.
            let temp = tempfile::tempdir().unwrap();
            fs::create_dir(temp.path().join("gore")).unwrap();
            fs::write(
                temp.path().join("gore/loc_catalog.json"),
                serde_json::to_vec(&json!({
                    "ocr_grd_stone_219": {
                        "german_new": "  ", "german": "Prüfschmied",
                        "english_newer": "Verification smith"
                    },
                    "ocr_grd_stone": {"german": "Katalogschmied"},
                    "text_wip_tmninll_20250515_115330": {
                        "german": "  Prüfgeschichte über die Erzrüstung \n",
                        "english": "Verification lore about ore armor"
                    },
                    "text_wip_tmninll_20250515_113030": {
                        "german": "  ", "english_newer": "Teacher lore fallback",
                        "english": "Older teacher lore"
                    },
                    "same_paragraph": {"german": "Prüfgeschichte über die Erzrüstung"},
                    "extra_paragraph": {"english_new": " Extra lore fallback "},
                    "blank_paragraph": {"german": "  ", "english": "\n"}
                }))
                .unwrap(),
            )
            .unwrap();
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", TEST, "--nocapture"])
                .env(CHILD, "1")
                .env("XDG_DATA_HOME", temp.path())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }

        let o = Options {
            lang: "en".into(),
            game_lang: "de".into(),
            limit: 1,
            ..Options::default()
        };
        let original = json!({"section": "story", "total": 3, "entries": [
            {"id": "Stone_OreArmor", "stored": true, "rawValue": 1767047,
             "path": ["StoryPropertyValues", "{Stone_OreArmor}"], "declaredType": "FInGameTime"},
            {"id": "Stone_Teacher", "stored": false, "rawValue": null, "path": []},
            {"id": "StoneOreArmor", "stored": false, "rawValue": null}
        ]});
        let mut data = original.clone();
        annotate(&mut data, &o).unwrap();
        assert_eq!(data["total"], 3);
        assert_eq!(data["entries"].as_array().unwrap().len(), 3);
        for (row, raw) in data["entries"]
            .as_array()
            .unwrap()
            .iter()
            .zip(original["entries"].as_array().unwrap())
        {
            let mut without_context = row.clone();
            without_context
                .as_object_mut()
                .unwrap()
                .remove("glossaryLink");
            assert_eq!(&without_context, raw);
        }
        assert_eq!(data["entries"][2], original["entries"][2]);
        assert_eq!(data["entries"][0]["glossaryLink"]["npcName"], "Prüfschmied");
        assert_eq!(
            data["entries"][0]["glossaryLink"]["paragraphs"],
            json!(["Prüfgeschichte über die Erzrüstung"])
        );
        assert_eq!(
            data["entries"][1]["glossaryLink"]["paragraphs"],
            json!(["Teacher lore fallback"])
        );

        let mut by_name = data.clone();
        let search = Options {
            query: Some("PRÜFSCHMIED".into()),
            offset: 1,
            ..o.clone()
        };
        display::filter(&mut by_name, "entries", &search);
        display::paginate(&mut by_name, "entries", &search);
        assert_eq!(by_name["total"], 2);
        assert_eq!(by_name["count"], 1);
        assert_eq!(by_name["entries"][0]["id"], "Stone_Teacher");
        for query in ["Erzrüstung", "Stone_OreArmor"] {
            let mut page = data.clone();
            let search = Options {
                query: Some(query.into()),
                ..o.clone()
            };
            display::filter(&mut page, "entries", &search);
            display::paginate(&mut page, "entries", &search);
            assert_eq!(page["total"], 1);
            assert_eq!(page["entries"][0]["id"], "Stone_OreArmor");
        }

        let mut english = original.clone();
        annotate(
            &mut english,
            &Options {
                game_lang: "en".into(),
                ..o.clone()
            },
        )
        .unwrap();
        assert_eq!(
            english["entries"][0]["glossaryLink"]["npcName"],
            "Verification smith"
        );

        let npcs = display::catalog("glossary").unwrap();
        let mut segment_texts = display::catalog("glossary-text").unwrap();
        let class = "/Script/Angelscript.DocumentSegment_Glossary_OCR_GRD_STONE_OreArmor";
        segment_texts.as_object_mut().unwrap().remove(class);
        segment_texts[class.to_lowercase()] = json!([
            "TEXT_WIP_TMNINLL_20250515_115330",
            "same_paragraph",
            "extra_paragraph",
            "blank_paragraph",
            "missing_paragraph"
        ]);
        let links = catalog_links(&npcs, &segment_texts).unwrap();
        let mut deduplicated = original.clone();
        annotate_links(
            &mut deduplicated,
            &links,
            &text::Texts::load_options(&o).unwrap(),
        )
        .unwrap();
        assert_eq!(
            deduplicated["entries"][0]["glossaryLink"]["textIds"],
            segment_texts[class.to_lowercase()]
        );
        assert_eq!(
            deduplicated["entries"][0]["glossaryLink"]["paragraphs"],
            json!(["Prüfgeschichte über die Erzrüstung", "Extra lore fallback"])
        );

        let path = gore_loc::paths::loc_catalog_path();
        let mut cache = read_json(&path).unwrap();
        cache.as_object_mut().unwrap().remove("ocr_grd_stone_219");
        fs::write(&path, serde_json::to_vec(&cache).unwrap()).unwrap();
        let mut fallback = original.clone();
        annotate(&mut fallback, &o).unwrap();
        assert_eq!(
            fallback["entries"][0]["glossaryLink"]["npcName"],
            "Katalogschmied"
        );

        fs::remove_file(path).unwrap();
        annotate(&mut fallback, &o).unwrap();
        assert_eq!(fallback["entries"][0]["glossaryLink"]["npcName"], "STONE");
        assert_eq!(
            fallback["entries"][0]["glossaryLink"]["paragraphs"],
            json!([])
        );
    }
}
