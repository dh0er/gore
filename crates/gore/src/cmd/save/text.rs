//! Save ids often differ from the game's localization keys.
use super::*;

fn snake(raw: &str) -> String {
    let mut out = String::new();
    let mut lower_or_digit = false;
    for c in raw.chars() {
        let last = out.chars().last();
        if matches!(c, '_' | '-') {
            if last != Some('_') {
                out.push('_');
            }
            lower_or_digit = false;
            continue;
        }
        if c.is_ascii_uppercase() {
            if lower_or_digit && last != Some('_') {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
            lower_or_digit = false;
        } else {
            if (c.is_ascii_digit() && last.is_some_and(|l| l.is_ascii_lowercase()))
                || (c.is_ascii_lowercase() && last.is_some_and(|l| l.is_ascii_digit()))
            {
                out.push('_');
            }
            out.push(c);
            lower_or_digit = c.is_ascii_lowercase() || c.is_ascii_digit();
        }
    }
    out.trim_matches('_').into()
}
pub(super) fn readable(raw: &str) -> String {
    snake(raw.rsplit(['/', '.']).next().unwrap_or(raw))
        .split('_')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut chars = s.chars();
            format!("{}{}", chars.next().unwrap().to_uppercase(), chars.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}
fn long_number(raw: &str) -> bool {
    let mut run = 0;
    raw.bytes().any(|c| {
        run = if c.is_ascii_digit() { run + 1 } else { 0 };
        run >= 4
    })
}
pub(super) struct Texts {
    catalog: Value,
    sets: Vec<&'static str>,
    lang: String,
}
pub(super) fn default_game_language(lang: &str) -> &'static str {
    presentation::metadata()["gameTextDefaults"][lang]
        .as_str()
        .unwrap_or("en")
}
impl Texts {
    pub(super) fn load_options(o: &Options) -> Result<Self> {
        let game = if o.game_lang.is_empty() || o.game_lang == "auto" {
            default_game_language(&o.lang)
        } else {
            &o.game_lang
        };
        Self::load_languages(&o.lang, game)
    }
    fn load_languages(lang: &str, game: &str) -> Result<Self> {
        if presentation::metadata()["ui"][lang].is_null() {
            bail!("unsupported interface language {lang}");
        }
        let sets = match game {
            "en" => vec![],
            "de" => vec!["german_new", "german"],
            "fr" => vec!["french"],
            "it" => vec!["italian"],
            "es" => vec!["spanish"],
            "pl" => vec!["polish"],
            "ru" => vec!["russian"],
            "ja" => vec!["japanese"],
            "zh-Hans" => vec!["schinese"],
            "pt-BR" => vec!["brazilian"],
            _ => bail!("unsupported game text language {game}"),
        };
        let path = gore_loc::paths::loc_catalog_path();
        let catalog = if path.is_file() {
            read_json(&path)?
        } else {
            json!({})
        };
        Ok(Self {
            catalog,
            sets,
            lang: lang.into(),
        })
    }
    pub(super) fn ui(&self, key: &str) -> String {
        presentation::metadata()["ui"][&self.lang][key]
            .as_str()
            .unwrap_or(key)
            .into()
    }
    pub(super) fn get(&self, id: &str) -> Option<String> {
        let entry = &self.catalog[id.to_lowercase()];
        self.sets
            .iter()
            .copied()
            .chain(["english_newer", "english_new", "english"])
            .find_map(|set| {
                entry[set]
                    .as_str()
                    .filter(|s| !s.trim().is_empty())
                    .map(str::to_owned)
            })
    }
    pub(super) fn character_name(&self, id: &str) -> String {
        let key = id.split('-').next().unwrap_or(id);
        if let Some(name) = self.get(key) {
            return name;
        }
        let mut rest = key;
        loop {
            let prefix = [
                "OC_",
                "OM_",
                "NPC_",
                "Creature_",
                "AM_",
                "PC_",
                "BL_",
                "VLK_",
                "KDF_",
                "STT_",
                "SLD_",
                "GRD_",
                "MIL_",
                "EBR_",
                "BAU_",
                "TPL_",
                "NOV_",
                "DJG_",
                "SFB_",
                "GUR_",
                "OUT_",
                "SUM_",
            ]
            .into_iter()
            .find(|prefix| {
                rest.len() > prefix.len()
                    && rest
                        .get(..prefix.len())
                        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
            });
            let Some(prefix) = prefix else {
                break;
            };
            rest = &rest[prefix.len()..];
        }
        rest = rest
            .trim_end_matches(|c: char| c.is_ascii_digit())
            .trim_end_matches(['_', '-']);
        if rest.is_empty() {
            rest = key;
        }
        readable(rest)
    }
    pub(super) fn key_name(&self, id: &str) -> String {
        if let Some(name) = self.get(id) {
            return name;
        }
        for prefix in [
            "ItMw_", "ItRw_", "ItAr_", "ItFo_", "ItMi_", "ItAt_", "ItWr_", "ItMs_", "ItKe_",
            "ItAm_",
        ] {
            if let Some(name) = id.strip_prefix(prefix) {
                let name = name.replace('_', " ");
                return if name.trim().is_empty() {
                    id.to_owned()
                } else {
                    name.trim().to_owned()
                };
            }
        }
        if id.starts_with("It") && id.as_bytes().get(2).is_some_and(u8::is_ascii_uppercase) {
            return readable(&id[2..]);
        }
        id.replace('_', " ").trim().to_owned()
    }
    fn knowledge(&self, id: &str, metadata: &Value) -> Option<String> {
        if let Some(value) = metadata["loc_key"].as_str().and_then(|key| self.get(key)) {
            return Some(value);
        }
        if let Some(caption) = metadata["caption"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
        {
            return Some(match caption.trim() {
                "[Forced Conversation]" => self.ui("knowledgeCaptionForcedConversation"),
                "[Followup Topic]" => self.ui("knowledgeCaptionFollowupTopic"),
                "[Fallback Topic]" => self.ui("knowledgeCaptionFallbackTopic"),
                _ => caption.into(),
            });
        }
        let lower = id.to_lowercase();
        if let Some(inner) = lower.strip_prefix("voiceline_") {
            return self.get(
                inner
                    .rsplit_once("_alkimialocalization")
                    .map(|(s, _)| s)
                    .unwrap_or(inner),
            );
        }
        if long_number(id) {
            return None;
        }
        let snake = snake(id);
        let body = snake
            .strip_prefix("choice_")
            .or_else(|| snake.strip_prefix("topic_"))
            .unwrap_or(&snake);
        for stem in [
            snake.clone(),
            format!("info_{body}"),
            format!("dia_{body}"),
            format!("info_{snake}"),
            format!("dia_{snake}"),
        ] {
            if let Some(value) = self.get(&stem) {
                return Some(value);
            }
            let prefix = format!("{stem}_");
            if let Some(key) = self
                .catalog
                .as_object()
                .and_then(|m| m.keys().find(|k| k.starts_with(&prefix)))
            {
                if let Some(value) = self.get(key) {
                    return Some(value);
                }
            }
        }
        None
    }
    pub(super) fn apply(&self, data: &mut Value) -> Result<()> {
        let knowledge = display::catalog("knowledge")?;
        let segments = display::catalog("glossary-text")?;
        fn visit(value: &mut Value, texts: &Texts, knowledge: &Value, segments: &Value) {
            match value {
                Value::Object(map) => {
                    for value in map.values_mut() {
                        visit(value, texts, knowledge, segments);
                    }
                    let snapshot = map.clone();
                    for (key, val) in &snapshot {
                        if let Some(id) = val.as_str() {
                            if let Some(text) = texts.get(id) {
                                map.insert(format!("{key}Text"), json!(text));
                            }
                        }
                    }
                    if let Some(segment) = snapshot.get("segmentClass").and_then(Value::as_str) {
                        if let Some(keys) = segments[segment].as_array() {
                            map.insert("textSegments".into(),json!(keys.iter().map(|key|json!({"id":key,"text":key.as_str().and_then(|id|texts.get(id))})).collect::<Vec<_>>()));
                        }
                    }
                    if let Some(id) = snapshot.get("questClass").and_then(Value::as_str) {
                        let id = id.rsplit('.').next().unwrap_or(id);
                        let body = id.strip_prefix("Quest_").unwrap_or(id).to_lowercase();
                        let label = texts
                            .get(&format!("quest-{body}-name"))
                            .unwrap_or_else(|| readable(id.strip_prefix("Quest_").unwrap_or(id)));
                        map.insert("label".into(), json!(label));
                        if let Some(description) = texts.get(&format!("quest-{body}-description")) {
                            map.insert("description".into(), json!(description));
                        }
                    }
                    if let Some(id) = snapshot.get("id").and_then(Value::as_str) {
                        if let Some(metadata) = knowledge.as_array().and_then(|rows| {
                            rows.iter().find(|r| {
                                r["id"].as_str().is_some_and(|v| v.eq_ignore_ascii_case(id))
                            })
                        }) {
                            if let Some(label) = texts.knowledge(id, metadata) {
                                map.insert("label".into(), json!(label));
                            }
                        }
                    }
                    if snapshot.contains_key("documentClass") && snapshot.contains_key("segments") {
                        let name = snapshot.get("name").and_then(Value::as_str).unwrap_or("");
                        let npc = snapshot.get("isNpc") == Some(&Value::Bool(true));
                        let label = snapshot
                            .get("uniqueName")
                            .and_then(Value::as_str)
                            .and_then(|id| texts.get(id))
                            .or_else(|| {
                                snapshot
                                    .get("id")
                                    .and_then(Value::as_str)
                                    .and_then(|id| texts.get(id))
                            })
                            .or_else(|| texts.get(name))
                            .unwrap_or_else(|| {
                                readable(if npc {
                                    name.rsplit('_').next().unwrap_or(name)
                                } else {
                                    name
                                })
                            });
                        map.insert("label".into(), json!(label));
                    }
                    if let Some(id) = snapshot
                        .get("attributeId")
                        .or_else(|| snapshot.get("key"))
                        .and_then(Value::as_str)
                        .filter(|_| {
                            snapshot.contains_key("base") || snapshot.contains_key("presentation")
                        })
                    {
                        let set = snapshot
                            .get("setClass")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .rsplit('.')
                            .next()
                            .unwrap_or("")
                            .strip_prefix("AttributeSet_")
                            .unwrap_or("")
                            .to_lowercase();
                        let exact = format!("attributeset_{set}_{}", id.to_lowercase());
                        let suffix = format!("_{}", id.to_lowercase());
                        let matching = texts
                            .catalog
                            .as_object()
                            .map(|m| {
                                m.keys()
                                    .filter(|k| {
                                        k.starts_with("attributeset_")
                                            && !k.ends_with("_description")
                                            && k.ends_with(&suffix)
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();
                        let key = if texts.get(&exact).is_some() {
                            Some(exact.as_str())
                        } else if matching.len() == 1 {
                            Some(matching[0].as_str())
                        } else {
                            None
                        };
                        if let Some(key) = key {
                            if let Some(label) = texts.get(key) {
                                map.insert("label".into(), json!(label));
                            }
                            if let Some(text) = texts.get(&format!("{key}_description")) {
                                map.insert("tooltip".into(), json!(text));
                            }
                        }
                    }
                    if snapshot.get("section").and_then(Value::as_str) == Some("knowledge") {
                        if let Some(entries) = map.get_mut("entries").and_then(Value::as_array_mut)
                        {
                            for entry in entries {
                                if let Some(id) = entry.as_str().map(str::to_owned) {
                                    let metadata = knowledge
                                        .as_array()
                                        .and_then(|rows| {
                                            rows.iter().find(|r| {
                                                r["id"]
                                                    .as_str()
                                                    .is_some_and(|v| v.eq_ignore_ascii_case(&id))
                                            })
                                        })
                                        .cloned()
                                        .unwrap_or(Value::Null);
                                    let category =
                                        metadata["category"].as_str().unwrap_or_else(|| {
                                            if id.to_lowercase().starts_with("voiceline_") {
                                                "voiceLine"
                                            } else if id.to_lowercase().starts_with("choice") {
                                                "choice"
                                            } else if id.to_lowercase().starts_with("topic") {
                                                "topic"
                                            } else if id.to_lowercase().starts_with("info") {
                                                "info"
                                            } else {
                                                "other"
                                            }
                                        });
                                    let label =
                                        texts.knowledge(&id, &metadata).unwrap_or_else(|| {
                                            if long_number(&id) {
                                                texts.ui(match category {
                                                    "choice" => "fallbackDialogChoice",
                                                    "topic" => "fallbackDialogTopic",
                                                    "info" => "fallbackDialogInformation",
                                                    _ => "fallbackDialogEntry",
                                                })
                                            } else {
                                                readable(&id)
                                            }
                                        });
                                    *entry = json!({"id":id,"label":label,"category":category,"catalog":metadata});
                                }
                            }
                        }
                    }
                }
                Value::Array(rows) => rows
                    .iter_mut()
                    .for_each(|r| visit(r, texts, knowledge, segments)),
                _ => {}
            }
        }
        visit(data, self, &knowledge, &segments);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn displayed_actor_and_key_names_resolve_catalog_ids_and_editor_fallbacks() {
        let texts = Texts {
            catalog: json!({"nc_org_wolf_855":{"german":"Rüstungsmeister"}, "itke_quentin_01":{"english":"Cave key"}}),
            sets: vec!["german"],
            lang: "en".into(),
        };
        assert_eq!(
            texts.character_name("NC_ORG_Wolf_855-WorldPointActor_wolf"),
            "Rüstungsmeister"
        );
        assert_eq!(texts.character_name("OC_VLK_Guard_01-1234"), "Guard");
        assert_eq!(texts.character_name("Creature_Meatbug"), "Meatbug");
        assert_eq!(texts.key_name("ItKe_Quentin_01"), "Cave key");
        assert_eq!(texts.key_name("ItKe_Quentin_02"), "Quentin 02");
        assert_eq!(texts.key_name("ItChestKey01"), "Chest Key 01");
    }

    #[test]
    fn quests_search_localized_titles_and_descriptions_before_pagination() {
        let texts = Texts {
            catalog: json!({
                "quest-oldcamp_first-name":{"german":"Übersetzter Titel"},
                "quest-oldcamp_second-name":{"german":"Anderer Titel"},
                "quest-oldcamp_first-description":{"german":"Gesuchte Beschreibung"},
                "quest-oldcamp_second-description":{"german":"Gesuchte Beschreibung"}
            }),
            sets: vec!["german"],
            lang: "en".into(),
        };
        let rows = json!({"section":"quests","quests":[
            {"questClass":"/Script/Angelscript.Quest_OldCamp_First","id":"Quest_OldCamp_First","group":"OldCamp","name":"First","currentState":"EQuestState::Running","statePath":["first"],"writable":true},
            {"questClass":"/Script/Angelscript.Quest_OldCamp_Second","id":"Quest_OldCamp_Second","group":"OldCamp","name":"Second","currentState":"EQuestState::Available","statePath":["second"],"writable":true},
            {"questClass":"/Script/Angelscript.Quest_NewCamp_Third","id":"Quest_NewCamp_Third","group":"NewCamp","name":"Third","currentState":null,"statePath":["third"],"writable":false}
        ]});
        let options = Options {
            query: Some("  ÜBERSETZTER TITEL  ".into()),
            limit: 1,
            ..Options::default()
        };
        let page = progression_page("quests", rows.clone(), &options, &texts).unwrap();
        assert_eq!(page["total"], 1);
        assert_eq!(page["quests"][0]["id"], "Quest_OldCamp_First");
        assert_eq!(page["quests"][0]["label"], "Übersetzter Titel");
        assert_eq!(page["quests"][0]["statePath"], json!(["first"]));

        let page = progression_page(
            "quests",
            rows.clone(),
            &Options {
                query: Some("gesuchte beschreibung".into()),
                offset: 1,
                ..options.clone()
            },
            &texts,
        )
        .unwrap();
        assert_eq!(page["total"], 2);
        assert_eq!(page["count"], 1);
        assert_eq!(page["offset"], 1);
        assert_eq!(page["limit"], 1);
        assert_eq!(page["quests"][0]["id"], "Quest_OldCamp_Second");
        assert_eq!(page["stateCounts"], json!({"Running":1,"Available":1}));
        assert_eq!(page["groupCounts"], json!({"OldCamp":2}));

        let page = progression_page(
            "quests",
            rows.clone(),
            &Options {
                query: Some("gesuchte beschreibung".into()),
                state: Some("equeststate::running".into()),
                group: Some("oldcamp".into()),
                ..options.clone()
            },
            &texts,
        )
        .unwrap();
        assert_eq!(page["total"], 1);
        assert_eq!(page["quests"][0]["id"], "Quest_OldCamp_First");
        assert_eq!(page["stateCounts"], json!({"Running":1,"Available":1}));
        assert_eq!(page["groupCounts"], json!({"OldCamp":1}));

        let page = progression_page(
            "quests",
            rows,
            &Options {
                query: Some("quest_newcamp_third".into()),
                ..options
            },
            &texts,
        )
        .unwrap();
        assert_eq!(page["total"], 1);
        assert_eq!(page["quests"][0]["writable"], false);
        assert_eq!(page["stateCounts"], json!({"unknown":1}));
    }

    #[test]
    fn glossary_search_and_category_facets_use_discovered_localized_documents() {
        let texts = Texts {
            catalog: json!({
                "scavenger":{"german":"Übersetzter Begriff A"},
                "bloodfly":{"german":"Übersetzter Begriff B"},
                "wolf":{"german":"Übersetzter Begriff C"}
            }),
            sets: vec!["german"],
            lang: "en".into(),
        };
        let mut rows = json!({"section":"glossary","categories":[
            {"id":"creatures","entries":[
                {"id":"BloodflyGlossary","name":"Bloodfly","category":"creatures","group":"CreaturesGlossary","documentClass":"Document_Bloodfly","currentState":null,"segments":[{"unlocked":true}]},
                {"id":"ScavengerGlossary","name":"Scavenger","category":"creatures","group":"CreaturesGlossary","documentClass":"Document_Scavenger","currentState":"EQuestState::NotAvailable","segments":[{"unlocked":true}]},
                {"id":"WolfGlossary","name":"Wolf","category":"creatures","group":"CreaturesGlossary","documentClass":"Document_Wolf","currentState":"EQuestState::Running","segments":[{"unlocked":false}]}
            ]}
        ]});
        annotate_glossary(&mut rows, &json!([]), &json!({"characters":[]})).unwrap();
        let options = Options {
            query: Some(" ÜBERSETZTER BEGRIFF ".into()),
            category: Some("CREATURE".into()),
            offset: 1,
            limit: 1,
            ..Options::default()
        };
        let page = progression_page("glossary", rows.clone(), &options, &texts).unwrap();
        assert_eq!(page["total"], 2);
        assert_eq!(page["count"], 1);
        assert_eq!(page["offset"], 1);
        assert_eq!(page["categoryCounts"], json!({"creatures":2}));
        assert_eq!(
            page["categories"][0]["entries"][0]["id"],
            "BloodflyGlossary"
        );
        assert_eq!(
            page["categories"][0]["entries"][0]["label"],
            "Übersetzter Begriff B"
        );

        let page = progression_page(
            "glossary",
            rows.clone(),
            &Options {
                include_unset: true,
                all: true,
                ..options.clone()
            },
            &texts,
        )
        .unwrap();
        assert_eq!(page["total"], 3);
        assert_eq!(page["count"], 2);
        assert_eq!(page["limit"], 2);

        let page = progression_page(
            "glossary",
            rows,
            &Options {
                entry: Some("document_wolf".into()),
                state: Some("locked".into()),
                offset: 0,
                ..options
            },
            &texts,
        )
        .unwrap();
        assert_eq!(page["total"], 1);
        assert_eq!(page["categories"][0]["entries"][0]["id"], "WolfGlossary");
    }

    #[test]
    fn catalog_search_matches_game_text_before_filtering_and_pagination() {
        let texts = Texts {
            catalog: json!({
                "itmi_orenugget":{"german":"Übersetzter Treffer: Erz", "english":"English ore"},
                "itar_rune_fireball":{"german":"Übersetzter Treffer: Feuer", "english":"English rune"}
            }),
            sets: vec!["german"],
            lang: "en".into(),
        };
        let options = Options {
            lang: "en".into(),
            game_lang: "de".into(),
            query: Some("übersetzter treffer".into()),
            offset: 1,
            limit: 1,
            ..Options::default()
        };
        let data = display::catalog_page("items", &options, &texts).unwrap();
        assert_eq!(data["total"], 2);
        assert_eq!(data["count"], 1);
        assert_eq!(data["offset"], 1);
        assert_eq!(data["entries"][0]["id"], "ItMi_Orenugget");
        assert_eq!(data["entries"][0]["idText"], "Übersetzter Treffer: Erz");

        let selected = display::catalog_page(
            "items",
            &Options {
                category: Some("material".into()),
                offset: 0,
                ..options.clone()
            },
            &texts,
        )
        .unwrap();
        assert_eq!(selected["total"], 1);
        assert_eq!(selected["entries"][0]["id"], "ItMi_Orenugget");

        let raw_id = display::catalog_page(
            "items",
            &Options {
                query: Some("ItMi_Orenugget".into()),
                offset: 0,
                ..options
            },
            &texts,
        )
        .unwrap();
        assert_eq!(raw_id["total"], 1);
        assert_eq!(raw_id["entries"][0]["id"], "ItMi_Orenugget");
    }

    #[test]
    fn progression_keys_resolve_exact_metadata_before_numeric_fallback() {
        let texts = Texts {
            catalog: json!({"text_exact":{"german":"Genau"},"quest-diego-name":{"english":"Diego's quest"},"info_exit_01":{"english":"Goodbye"}}),
            sets: vec!["german"],
            lang: "de".into(),
        };
        assert_eq!(
            texts
                .knowledge("Topic_Diego_123456", &json!({"loc_key":"TEXT_EXACT"}))
                .as_deref(),
            Some("Genau")
        );
        assert_eq!(texts.knowledge("Topic_Diego_123456", &Value::Null), None);
        assert_eq!(
            texts.knowledge("ChoiceExit", &Value::Null).as_deref(),
            Some("Goodbye")
        );
        let mut data = json!({"questClass":"/Script/Angelscript.Quest_Diego"});
        texts.apply(&mut data).unwrap();
        assert_eq!(data["label"], "Diego's quest");
    }
}
