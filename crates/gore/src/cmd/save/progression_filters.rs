//! Editor progression joins and filters, applied to the complete localized list.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn normalized(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_ascii_lowercase)
}

fn short_state(row: &Value) -> &str {
    row["currentState"]
        .as_str()
        .map(|s| s.rsplit("::").next().unwrap_or(s))
        .unwrap_or("unknown")
}

fn matches_state(row: &Value, state: Option<&str>) -> bool {
    state.is_none_or(|state| {
        let full = row["currentState"].as_str().unwrap_or("");
        // Keep the core predicate exact, including its treatment of null states.
        full.eq_ignore_ascii_case(state)
            || full
                .rsplit("::")
                .next()
                .unwrap_or(full)
                .eq_ignore_ascii_case(state)
    })
}

fn matches_query(row: &Value, query: &str, fields: &[&str]) -> bool {
    query.is_empty()
        || fields.iter().any(|field| {
            row[*field]
                .as_str()
                .is_some_and(|s| s.to_lowercase().contains(query))
        })
}

pub(super) fn filter_quests(data: &mut Value, o: &Options) {
    let query = o.query.as_deref().unwrap_or("").trim().to_lowercase();
    let state = normalized(&o.state);
    let group = normalized(&o.group);
    let Some(rows) = data["quests"].as_array_mut() else {
        return;
    };
    let query_matches = |row: &Value| {
        matches_query(
            row,
            &query,
            &["questClass", "id", "name", "group", "label", "description"],
        )
    };
    let group_matches = |row: &Value| {
        group.as_deref().is_none_or(|group| {
            row["group"]
                .as_str()
                .unwrap_or("")
                .eq_ignore_ascii_case(group)
        })
    };
    let mut states = BTreeMap::<String, usize>::new();
    let mut groups = BTreeMap::<String, usize>::new();
    for row in rows.iter().filter(|row| query_matches(row)) {
        if group_matches(row) {
            *states.entry(short_state(row).into()).or_default() += 1;
        }
        if matches_state(row, state.as_deref()) {
            *groups
                .entry(row["group"].as_str().unwrap_or("").into())
                .or_default() += 1;
        }
    }
    rows.retain(|row| {
        query_matches(row) && group_matches(row) && matches_state(row, state.as_deref())
    });
    data["stateCounts"] = json!(states);
    data["groupCounts"] = json!(groups);
}

pub(super) fn annotate_glossary(
    data: &mut Value,
    catalog: &Value,
    characters: &Value,
) -> Result<()> {
    // Unlock events, rather than quest states or viewed events, reveal a segment.
    let unlocks: BTreeMap<_, _> = data["segmentUnlocks"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| {
            (
                (
                    row["documentClass"].as_str().unwrap_or("").to_lowercase(),
                    row["segmentClass"].as_str().unwrap_or("").to_lowercase(),
                ),
                row,
            )
        })
        .collect();
    let characters: BTreeMap<_, _> = characters["characters"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| (row["uniqueName"].as_str().unwrap_or("").to_lowercase(), row))
        .collect();
    let writable = data["writable"]
        .as_array()
        .is_some_and(|operations| operations.contains(&json!("private.glossary.setSegment")));
    let memory_path = data["heroMemoryArrayPath"].clone();
    let mut npc_categories = BTreeMap::<String, Vec<Value>>::new();
    for entry in catalog.as_array().context("invalid NPC glossary catalog")? {
        let document = entry["documentClass"].as_str().unwrap_or("");
        let unique_name = entry["uniqueName"].as_str().unwrap_or("");
        let character = characters
            .get(&unique_name.to_lowercase())
            .copied()
            .unwrap_or(&Value::Null);
        let mut roles = BTreeSet::<String>::new();
        let segments: Vec<_> = entry["segments"].as_array().into_iter().flatten().map(|segment| {
            let class = segment["class"].as_str().unwrap_or("");
            let memory = unlocks.get(&(document.to_lowercase(), class.to_lowercase())).copied().unwrap_or(&Value::Null);
            let indices = memory["unlockedEventIndices"].as_array().cloned().unwrap_or_default();
            let unlocked = !indices.is_empty();
            if unlocked {
                roles.extend(segment["roles"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_owned));
            }
            let paths: Vec<_> = indices.iter().filter_map(|index| {
                let mut path = memory_path.as_array()?.clone();
                path.push(json!(format!("[{}]", index.as_u64()?)));
                Some(Value::Array(path))
            }).collect();
            json!({
                "id":segment["id"], "name":segment["label"], "segmentClass":class,
                "documentClass":document, "roles":segment["roles"],
                "unlocked":unlocked, "writable":writable,
                "eventIndex":indices.first(), "eventIndices":indices,
                "eventPath":paths.first(), "eventPaths":paths,
                "viewedEventIndices":memory["viewedEventIndices"].as_array().cloned().unwrap_or_default()
            })
        }).collect();
        let unlocked_count = segments
            .iter()
            .filter(|segment| segment["unlocked"] == true)
            .count();
        let camp = entry["camp"].as_str().unwrap_or("outsiders");
        npc_categories.entry(camp.into()).or_default().push(json!({
            "id":entry["id"], "name":entry["id"], "uniqueName":unique_name,
            "globalId":character["globalId"], "documentClass":document,
            "category":camp, "group":camp, "isNpc":true,
            "personalRelationship":character["personalRelationship"],
            "roles":roles, "segments":segments, "segmentCount":segments.len(),
            "unlockedSegmentCount":unlocked_count, "unlocked":unlocked_count > 0,
            "writable":writable
        }));
    }
    let categories = data["categories"]
        .as_array_mut()
        .context("glossary categories unavailable")?;
    for category in categories.iter_mut() {
        if let Some(entries) = category["entries"].as_array_mut() {
            for entry in entries {
                let count = entry["segments"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|s| s["unlocked"] == true)
                    .count();
                entry["unlocked"] = json!(count > 0);
                entry["unlockedSegmentCount"] = json!(count);
            }
        }
    }
    for camp in ["oldCamp", "newCamp", "swampCamp", "outsiders"] {
        categories.push(json!({"id":camp, "group":camp, "entries":npc_categories.remove(camp).unwrap_or_default()}));
    }
    Ok(())
}

fn has_role(row: &Value, role: &str) -> bool {
    row["isNpc"] == true
        && row["roles"].as_array().is_some_and(|roles| {
            roles
                .iter()
                .any(|r| r.as_str().is_some_and(|s| s.eq_ignore_ascii_case(role)))
        })
}

fn hostile(row: &Value) -> bool {
    row["isNpc"] == true
        && row["personalRelationship"]
            .as_str()
            .is_some_and(|s| s.eq_ignore_ascii_case("Enemy"))
}

pub(super) fn filter_glossary(data: &mut Value, o: &Options) {
    let query = o.query.as_deref().unwrap_or("").trim().to_lowercase();
    let state = normalized(&o.state);
    let role = normalized(&o.role);
    let relationship = normalized(&o.relationship);
    let selected =
        o.id.as_deref()
            .or(o.entry.as_deref())
            .or(o.document.as_deref());
    // Core accepts singular category names and uses group as a category alias.
    let category = normalized(&o.category).or_else(|| normalized(&o.group));
    let Some(categories) = data["categories"].as_array() else {
        return;
    };
    let mut rows: Vec<_> = categories
        .iter()
        .flat_map(|category| category["entries"].as_array().into_iter().flatten())
        .filter(|row| {
            let visible = row["unlocked"] == true;
            let state_matches = match state.as_deref() {
                None => visible || o.include_unset,
                Some("unlocked" | "discovered") => visible,
                Some("locked") => !visible,
                Some("dead") => has_role(row, "dead"),
                Some("alive") => visible && !has_role(row, "dead"),
                Some("hostile") => visible && hostile(row),
                Some(state) => (visible || o.include_unset) && matches_state(row, Some(state)),
            };
            state_matches
                && selected.is_none_or(|id| {
                    [
                        "id",
                        "name",
                        "documentClass",
                        "questClass",
                        "uniqueName",
                        "globalId",
                    ]
                    .iter()
                    .any(|field| {
                        row[*field]
                            .as_str()
                            .is_some_and(|s| s.eq_ignore_ascii_case(id))
                    })
                })
                && role.as_deref().is_none_or(|role| {
                    if role == "hostile" {
                        hostile(row)
                    } else {
                        has_role(row, role)
                    }
                })
                && relationship.as_deref().is_none_or(|relationship| {
                    row["isNpc"] == true
                        && row["personalRelationship"]
                            .as_str()
                            .is_some_and(|s| s.eq_ignore_ascii_case(relationship))
                })
                && (matches_query(
                    row,
                    &query,
                    &[
                        "id",
                        "name",
                        "label",
                        "uniqueName",
                        "documentClass",
                        "questClass",
                    ],
                ) || row["segments"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|segment| {
                        matches_query(
                            segment,
                            &query,
                            &["id", "name", "label", "questClass", "segmentClass"],
                        ) || segment["textSegments"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .any(|text| matches_query(text, &query, &["id", "text"]))
                    }))
        })
        .cloned()
        .collect();
    let mut counts = BTreeMap::<String, usize>::new();
    for row in &rows {
        *counts
            .entry(row["category"].as_str().unwrap_or("").into())
            .or_default() += 1;
    }
    rows.retain(|row| {
        category.as_deref().is_none_or(|category| {
            let alias = match category {
                "creature" => "creatures",
                "location" => "locations",
                _ => category,
            };
            ((category == "npc" || category == "npcs") && row["isNpc"] == true)
                || row["category"]
                    .as_str()
                    .is_some_and(|s| s.eq_ignore_ascii_case(alias))
                || row["group"]
                    .as_str()
                    .is_some_and(|s| s.eq_ignore_ascii_case(category))
        })
    });
    // Preserve category order; sort document names within each Editor section.
    let rank: BTreeMap<_, _> = categories
        .iter()
        .enumerate()
        .map(|(i, category)| (category["id"].as_str().unwrap_or("").to_string(), i))
        .collect();
    rows.sort_by(|a, b| {
        rank.get(a["category"].as_str().unwrap_or(""))
            .cmp(&rank.get(b["category"].as_str().unwrap_or("")))
            .then_with(|| {
                a["label"]
                    .as_str()
                    .or(a["name"].as_str())
                    .unwrap_or("")
                    .to_lowercase()
                    .cmp(
                        &b["label"]
                            .as_str()
                            .or(b["name"].as_str())
                            .unwrap_or("")
                            .to_lowercase(),
                    )
            })
    });
    let page_options = Options {
        limit: o.limit.clamp(1, 1000),
        ..o.clone()
    };
    let mut page = json!({"entries":rows});
    display::paginate(&mut page, "entries", &page_options);
    for category in data["categories"].as_array_mut().unwrap() {
        let id = category["id"].as_str().unwrap_or("");
        let entries: Vec<_> = page["entries"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["category"].as_str() == Some(id))
            .cloned()
            .collect();
        let total = counts.get(id).copied().unwrap_or(0);
        category["entries"] = json!(entries);
        category["total"] = json!(total);
    }
    for key in ["total", "count", "offset"] {
        data[key] = page[key].clone();
    }
    data["limit"] = if o.all {
        page["count"].clone()
    } else {
        json!(page_options.limit)
    };
    data["categoryCounts"] = json!(counts);
}
