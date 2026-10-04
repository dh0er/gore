//! Save-backed presentation rules shared with the Editor's statistics.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

pub(super) fn metadata() -> &'static Value {
    static DATA: OnceLock<Value> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("editor_presentation.json"))
            .expect("generated Editor presentation metadata")
    })
}

pub(super) fn annotate_story(data: &mut Value) {
    if let Some(rows) = data["entries"].as_array_mut() {
        for row in rows {
            let id = row["id"].as_str().unwrap_or("").to_lowercase();
            if let Some(info) = metadata()["story"].get(&id) {
                row["integerSemantics"] = info.clone();
            }
        }
    }
}

pub(super) fn attribute_info(id: &str, set: &str, lang: &str) -> Value {
    let set = set
        .rsplit('.')
        .next()
        .unwrap_or(set)
        .strip_prefix("AttributeSet_")
        .unwrap_or(set);
    let qualified = format!("{set}_{id}");
    let key = if matches!(
        id,
        "FillRatio" | "FillRatioPeriod" | "MaxThresholdIndex" | "RecoveryRatePerHourOfSleep"
    ) {
        qualified.as_str()
    } else {
        id
    };
    let group = metadata()["attributeGroups"]
        .as_object()
        .and_then(|groups| {
            groups
                .iter()
                .find(|(_, ids)| ids.as_array().is_some_and(|a| a.contains(&json!(key))))
        })
        .map(|(name, _)| name.as_str())
        .unwrap_or("advanced");
    let selects = &metadata()["selects"][lang];
    json!({"key":key,"group":group,"hidden":metadata()["hiddenAttributes"].as_array().is_some_and(|a|a.contains(&json!(key)) || a.contains(&json!(id))),
        "label":selects["attributeManualFallbackLabel"][key].as_str().unwrap_or(id),"tooltip":selects["attributeManualTooltip"][key].as_str().filter(|s|*s!="?")})
}

pub(super) struct Characters {
    categories: BTreeMap<String, String>,
    stripped: BTreeMap<String, String>,
    compact: BTreeMap<String, String>,
    unnumbered: BTreeMap<String, String>,
    roles: BTreeMap<String, BTreeSet<String>>,
}
fn fold(s: &str) -> String {
    s.chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}
fn unnumber(s: &str) -> Option<&str> {
    let (head, tail) = s.rsplit_once('_')?;
    let digits = tail.strip_suffix('n').unwrap_or(tail);
    (!digits.is_empty() && digits.bytes().all(|c| c.is_ascii_digit())).then_some(head)
}
fn candidates(raw: &str) -> Vec<String> {
    let mut name = raw
        .trim()
        .replace('\'', "")
        .rsplit('/')
        .next()
        .unwrap_or("")
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_string();
    if name.to_lowercase().ends_with("_c") {
        name.truncate(name.len() - 2);
    }
    let mut values = vec![name.clone()];
    let lower = name.to_lowercase();
    if let Some(at) = lower.find("-worldpointactor_") {
        values.push(name[..at].to_owned());
        values.push(name[at + 17..].to_owned());
    }
    if let Some(at) = lower.find("-wp_") {
        values.push(name[..at].to_owned());
    }
    // GlobalIds have a trailing GUID; never interpret it as part of a definition.
    let pieces: Vec<_> = name.rsplitn(6, '-').collect();
    if pieces.len() == 6
        && pieces[..5]
            .iter()
            .zip([12, 4, 4, 4, 8])
            .all(|(p, n)| p.len() == n && p.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        values.push(pieces[5].to_owned());
    }
    if let Some((head, _)) = name.split_once('-') {
        values.push(head.to_owned());
    }
    values
}
impl Characters {
    pub fn load() -> Result<Self> {
        let mut this = Self {
            categories: BTreeMap::new(),
            stripped: BTreeMap::new(),
            compact: BTreeMap::new(),
            unnumbered: BTreeMap::new(),
            roles: BTreeMap::new(),
        };
        for row in display::catalog("npc")?
            .as_array()
            .context("invalid character catalog")?
        {
            if let (Some(id), Some(kind)) = (row["id"].as_str(), row["category"].as_str()) {
                this.categories.insert(id.to_lowercase(), kind.into());
            }
        }
        for (index, mode) in [
            (&mut this.stripped, 0),
            (&mut this.compact, 1),
            (&mut this.unnumbered, 2),
        ] {
            let mut ambiguous = BTreeSet::new();
            for (name, kind) in &this.categories {
                let Some((_, tail)) = name.split_once('_') else {
                    continue;
                };
                let key = match mode {
                    1 => fold(tail),
                    2 => match unnumber(tail) {
                        Some(key) => key.to_owned(),
                        None => continue,
                    },
                    _ => tail.into(),
                };
                if index.get(&key).is_some_and(|old| old != kind) {
                    ambiguous.insert(key);
                } else {
                    index.insert(key, kind.clone());
                }
            }
            for key in ambiguous {
                index.remove(&key);
            }
        }
        for row in display::catalog("glossary")?
            .as_array()
            .context("invalid role catalog")?
        {
            let roles: BTreeSet<String> = row["segments"]
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|segment| segment["roles"].as_array().into_iter().flatten())
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect();
            for key in ["id", "uniqueName"] {
                if let Some(id) = row[key].as_str().filter(|id| !id.is_empty()) {
                    this.roles
                        .entry(id.to_lowercase())
                        .or_default()
                        .extend(roles.iter().cloned());
                }
            }
        }
        Ok(this)
    }
    pub fn category(&self, raw: &str) -> Option<&str> {
        for candidate in candidates(raw) {
            let key = candidate.to_lowercase();
            if let Some(category) = self
                .categories
                .get(&key)
                .or_else(|| self.stripped.get(&key))
                .or_else(|| self.compact.get(&fold(&key)))
            {
                return Some(category);
            }
            if let Some((_, tail)) = key.split_once('_') {
                if let Some(key) = unnumber(tail) {
                    if let Some(category) = self.unnumbered.get(key) {
                        return Some(category);
                    }
                }
            }
        }
        None
    }
    pub fn teacher(&self, raw: &str) -> bool {
        self.roles(raw).contains("teacher")
    }
    fn roles(&self, raw: &str) -> BTreeSet<String> {
        candidates(raw)
            .iter()
            .filter_map(|name| self.roles.get(&name.to_lowercase()))
            .flatten()
            .cloned()
            .collect()
    }
    pub fn annotate(&self, row: &mut Value) {
        let name = row["uniqueName"]
            .as_str()
            .or(row["id"].as_str())
            .unwrap_or("")
            .to_string();
        row["category"] = json!(self.category(&name));
        let roles = self.roles(&name);
        row["teacher"] = json!(roles.contains("teacher"));
        row["roles"] = json!(roles);
    }
}

pub(super) fn statistics(data: &Value, o: &Options) -> Result<Value> {
    let inspected = &data["inspection"];
    let private = &inspected["private"];
    let verified = private["typedParse"]["status"] == "ok" && private["preview"] != true;
    let mut attrs = BTreeMap::new();
    if let Some(rows) = private["player"]["attributes"].as_array() {
        for row in rows {
            if let Some(id) = row["id"].as_str() {
                attrs.insert(
                    id.to_owned(),
                    row["currentValue"].as_f64().or(row["baseValue"].as_f64()),
                );
            }
        }
    }
    let typed = super::attributes(
        "list",
        &Options {
            actor: "hero".into(),
            all: true,
            ..o.clone()
        },
    )
    .ok();
    if let Some(rows) = typed.as_ref().and_then(|v| v["results"].as_array()) {
        for row in rows {
            let Some(path) = row["path"].as_array() else {
                continue;
            };
            if path.last().and_then(Value::as_str) != Some("CurrentValue") || path.len() < 2 {
                continue;
            }
            if let Some(id) = path[path.len() - 2]
                .as_str()
                .and_then(|s| s.strip_prefix('{'))
                .and_then(|s| s.strip_suffix('}'))
            {
                attrs.insert(
                    id.to_owned(),
                    row["value"].as_str().and_then(|s| s.parse::<f64>().ok()),
                );
            }
        }
    }
    let pool = |a: &str, b: &str| json!({"current":attrs.get(a).copied().flatten(),"maximum":attrs.get(b).copied().flatten()});
    let inventory = &private["inventory"];
    let items = inventory["items"].as_array();
    let complete = verified
        && inventory["itemScope"] == "player_inventory_region"
        && items.is_some_and(|rows| {
            inventory["itemStackCount"].as_u64() == Some(rows.len() as u64)
                && rows.iter().all(|r| r["count"].is_i64())
        });
    let item_count = items.map(|rows| {
        rows.iter()
            .map(|r| r["count"].as_i64().unwrap_or(0))
            .sum::<i64>()
    });
    let ore = items.map(|rows| {
        rows.iter()
            .filter(|r| {
                r["id"]
                    .as_str()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains("orenugget")
                    || r["path"]
                        .as_str()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains("orenugget")
            })
            .map(|r| r["count"].as_i64().unwrap_or(0))
            .sum::<i64>()
    });
    let classifier = Characters::load()?;
    let rows = data["characters"]["characters"].as_array();
    let humans = rows.map(|rows| {
        rows.iter()
            .filter(|r| {
                r["uniqueName"].as_str().is_some_and(|name| {
                    name.to_lowercase() != "hero" && classifier.category(name) == Some("human")
                })
            })
            .collect::<Vec<_>>()
    });
    let hero = rows
        .and_then(|rows| {
            rows.iter().find(|r| {
                r["uniqueName"]
                    .as_str()
                    .is_some_and(|s| s.eq_ignore_ascii_case("hero"))
            })
        })
        .and_then(|r| r["globalId"].as_str());
    let events = hero.and_then(|id| {
        super::progression(
            "events",
            &Options {
                character: Some(id.into()),
                all: true,
                offset: 0,
                ..o.clone()
            },
        )
        .ok()
    });
    let mut monster_kills = 0;
    let mut npc_kills = 0;
    let mut defeats = 0;
    let mut guild = None;
    let mut last_guild_index = None;
    if let Some(rows) = events.as_ref().and_then(|v| v["events"].as_array()) {
        for event in rows {
            let tags = event["tags"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_lowercase)
                        .collect::<BTreeSet<_>>()
                })
                .unwrap_or_default();
            let killed = tags.contains("memory.character.defeated.kill")
                || tags.contains("memory.execution");
            let kind = if tags.iter().any(|t| t.starts_with("species.creature.")) {
                Some("creature")
            } else {
                classifier.category(event["affected"].as_str().unwrap_or(""))
            };
            if killed && kind == Some("creature") {
                monster_kills += 1;
            } else if killed && kind == Some("human") {
                npc_kills += 1;
            } else if tags.contains("memory.character.defeated") && kind == Some("human") {
                defeats += 1;
            }
            if tags.contains("memory.guild.joined") || tags.contains("memory.guild.expelled") {
                let index = event["index"].as_u64().unwrap_or(0);
                if last_guild_index.is_none_or(|last| index >= last) {
                    last_guild_index = Some(index);
                    guild = if tags.contains("memory.guild.expelled") {
                        None
                    } else {
                        event["tags"]
                            .as_array()
                            .and_then(|a| {
                                a.iter().filter_map(Value::as_str).find(|s| {
                                    s.to_lowercase().contains("guild")
                                        && !s.eq_ignore_ascii_case("memory.guild.joined")
                                })
                            })
                            .or(event["optionalClass1"].as_str())
                            .or(event["optionalClass2"].as_str())
                            .map(str::to_owned)
                    };
                }
            }
        }
    }
    let mut quests = json!({});
    let quest_available = verified && private["progression"]["status"] == "ok";
    for state in ["Succeeded", "Failed", "Running", "Available"] {
        quests[state] = if quest_available {
            json!(private["progression"]["questStates"].as_object().map(|m| {
                m.iter()
                    .filter(|(k, _)| {
                        k.rsplit("::")
                            .next()
                            .is_some_and(|k| k.eq_ignore_ascii_case(state))
                    })
                    .map(|(_, v)| v.as_u64().unwrap_or(0))
                    .sum::<u64>()
            }))
        } else {
            Value::Null
        };
    }
    let skills = &data["skills"];
    let learned = if skills["found"] == true {
        skills["skills"]
            .as_array()
            .map(|rows| rows.iter().filter(|s| s["learned"] == true).count())
    } else {
        None
    };
    let save_path = save(o)?.canonicalize()?;
    let parent = save(o)?
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let metadata = call("scan_save_dir_readonly", json!({"path":parent})).ok();
    let meta = metadata
        .as_ref()
        .and_then(|v| v["saves"].as_array())
        .and_then(|rows| {
            rows.iter().find(|r| {
                r["path"]
                    .as_str()
                    .and_then(|p| Path::new(p).canonicalize().ok())
                    .as_ref()
                    == Some(&save_path)
            })
        });
    Ok(
        json!({"progress":{"chapter":meta.map(|m|&m["chapterId"]),"playedSeconds":meta.map(|m|&m["timePlayedSeconds"]),"worldTime":data["worldTime"]},
        "character":{"level":attrs.get("Level").copied().flatten(),"experience":attrs.get("Experience").copied().flatten(),"learningPoints":attrs.get("SkillPoints").copied().flatten(),"guild":guild,"health":pool("Health","MaxHealth"),"mana":pool("Mana","MaxMana")},
        "quests":quests,"encounters":{"killedMonsters":events.as_ref().map(|_|monster_kills),"defeatedNpcs":events.as_ref().map(|_|defeats),"killedNpcs":events.as_ref().map(|_|npc_kills),"knownNpcs":humans.as_ref().map(Vec::len),"traders":humans.as_ref().map(|r|r.iter().filter(|r|r["isTrader"]==true).count()),"knownTeachers":humans.as_ref().map(|r|r.iter().filter(|r|classifier.teacher(r["uniqueName"].as_str().unwrap_or(""))).count()),"openCrimes":if verified{private["factions"]["openCrimes"].clone()}else{Value::Null}},
        "inventory":{"learnedSkills":learned,"items":complete.then_some(item_count).flatten(),"ore":complete.then_some(ore).flatten()},"unknownValue":null}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn categories_follow_spawn_ids_without_confusing_the_man_named_wolf() {
        let catalog = Characters::load().unwrap();
        assert_eq!(catalog.category("Wolf-WP_SPAWN_01-1"), Some("creature"));
        assert_eq!(
            catalog.category("NC_ORG_Wolf_855-WorldPointActor_wolf"),
            Some("human")
        );
        assert_eq!(
            catalog.category("LizardFire-WP_SPAWN_01-1"),
            Some("creature")
        );
        assert_eq!(catalog.category("unknown-species"), None);
        let roles = BTreeSet::from_iter(
            ["armorer", "dead", "portrait", "teacher", "trader"].map(String::from),
        );
        for identity in [
            "NC_ORG_WOLF",
            "NC_ORG_Wolf_855",
            "NC_ORG_Wolf_855-WorldPointActor_wolf",
        ] {
            assert_eq!(catalog.roles(identity), roles);
            assert!(catalog.teacher(identity));
        }
        assert!(catalog.roles("Wolf-WP_SPAWN_01-1").is_empty());
        assert!(catalog.roles("unknown-character").is_empty());
    }
    #[test]
    fn generated_tables_remain_identical_to_the_editor_sources() {
        use sha2::{Digest, Sha256};
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for (file, hash) in metadata()["sources"].as_object().unwrap() {
            let source = fs::read_to_string(root.join(file))
                .unwrap()
                .replace("\r\n", "\n");
            assert_eq!(
                format!("{:x}", Sha256::digest(source.as_bytes())),
                hash.as_str().unwrap(),
                "regenerate CLI presentation metadata after changing {file}"
            );
        }
        assert_eq!(metadata()["story"].as_object().unwrap().len(), 419);
        assert_eq!(metadata()["ui"].as_object().unwrap().len(), 16);
        let game_languages = [
            "en", "de", "fr", "it", "es", "pl", "ru", "ja", "zh-Hans", "pt-BR",
        ];
        for default in metadata()["gameTextDefaults"].as_object().unwrap().values() {
            assert!(game_languages.contains(&default.as_str().unwrap()));
        }
        assert_eq!(metadata()["gameTextDefaults"]["pt-BR"], "pt-BR");
        assert_eq!(
            attribute_info(
                "RecoveryRatePerHourOfSleep",
                "/Script/G1R.AttributeSet_Health",
                "en"
            )["group"],
            "sleep"
        );
    }
}
