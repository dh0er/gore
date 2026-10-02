//! Shared planning and execution of a Save Editor's pending changes.
//! Ordinals always refer to the inspected input; splitting a conflicting pair
//! into separate writes cannot make that pair safe.
use crate::{CoreError, Edit, PrivateEdit, properties::PathSeg};
use serde_json::{Value, json};
use std::{fs, path::Path};

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidRequest(message.into())
}
fn pending(kind: &'static str, path: Option<&[PathSeg]>) -> CoreError {
    let path = path
        .unwrap_or(&[])
        .iter()
        .map(|s| match s {
            PathSeg::Name(s) => s.clone(),
            PathSeg::MapKey(s) => format!("{{{s}}}"),
            PathSeg::Index(i) => format!("[{i}]"),
        })
        .collect::<Vec<_>>()
        .join(" › ");
    CoreError::PlanConflict { kind, path }
}
fn conflict(message: impl Into<String>) -> CoreError {
    CoreError::UnsupportedEdit(message.into())
}
fn parse(raw: &[Value]) -> Result<Vec<Edit>, CoreError> {
    raw.iter()
        .cloned()
        .map(|v| serde_json::from_value(v).map_err(|e| invalid(e.to_string())))
        .collect()
}
fn is_array(edit: &Edit) -> bool {
    matches!(
        edit.path.as_str(),
        "private.typed.arrayRemove" | "private.typed.arrayDuplicate"
    )
}
fn actor(edit: &Edit) -> Option<&str> {
    edit.value
        .get("actorId")
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
}

/// NPC pose/routine edits whose bytes and placement sidecar form one action.
pub fn placement_actor(edit: &Value) -> Option<&str> {
    if edit["path"] != "private.typed.setValue" {
        return None;
    }
    edit["value"]["path"]
        .as_array()?
        .windows(3)
        .find_map(|parts| {
            if matches!(
                (parts[0].as_str()?, parts[2].as_str()?),
                (
                    "PositionByGlobalId",
                    "CharacterLocation" | "CharacterRotation"
                ) | ("DailyRoutineByGlobalId", "DailyRoutineClass")
            ) {
                parts[1].as_str()?.strip_prefix('{')?.strip_suffix('}')
            } else {
                None
            }
        })
}

/// Keep only sidecars whose placement action still has pending byte edits.
pub fn retain_pending_placement_sidecars(payload: &mut Value) {
    let actors = payload["edits"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(placement_actor)
        .map(str::to_lowercase)
        .collect::<Vec<_>>();
    for key in ["placementNotes", "clearPlacementNotes"] {
        if let Some(entries) = payload[key].as_array_mut() {
            entries.retain(|entry| {
                let npc = if key == "placementNotes" {
                    entry["npc"].as_str()
                } else {
                    entry.as_str()
                };
                npc.is_some_and(|npc| actors.contains(&npc.to_lowercase()))
            });
            if entries.is_empty() {
                payload.as_object_mut().unwrap().remove(key);
            }
        }
    }
}

fn placement_sidecar_groups(
    payload: &Value,
    raw: &[Value],
    groups: &[Vec<usize>],
) -> Result<Vec<Value>, CoreError> {
    let mut actions = vec![json!({}); groups.len()];
    for key in ["placementNotes", "clearPlacementNotes"] {
        for entry in payload[key].as_array().into_iter().flatten() {
            let npc = if key == "placementNotes" {
                entry["npc"].as_str()
            } else {
                entry.as_str()
            }
            .ok_or_else(|| invalid("placement action needs an NPC id"))?;
            let steps = groups
                .iter()
                .enumerate()
                .filter(|(_, group)| {
                    group.iter().any(|index| {
                        placement_actor(&raw[*index])
                            .is_some_and(|actor| actor.eq_ignore_ascii_case(npc))
                    })
                })
                .map(|(step, _)| step)
                .collect::<Vec<_>>();
            if steps.len() != 1 {
                return Err(invalid(format!(
                    "placement action for {npc} must belong to one write group"
                )));
            }
            let data = actions[steps[0]].as_object_mut().unwrap();
            data.entry(key)
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .unwrap()
                .push(entry.clone());
        }
    }
    Ok(actions)
}
fn typed(edit: &Edit) -> Result<Option<Vec<PathSeg>>, CoreError> {
    if !edit.path.starts_with("private.typed.") {
        return Ok(None);
    }
    let raw: Vec<String> =
        serde_json::from_value(edit.value["path"].clone()).map_err(|e| invalid(e.to_string()))?;
    Ok(Some(crate::properties::parse_path(&raw)?))
}
fn splicing(edit: &Edit) -> bool {
    matches!(
        edit.path.as_str(),
        "private.inventory.addItem"
            | "private.inventory.removeItem"
            | "private.inventory.reset"
            | "private.knowledge.addCharacter"
            | "private.knowledge.setEntry"
            | "private.typed.arrayRemove"
            | "private.typed.arrayDuplicate"
            | "private.glossary.setSegment"
            | "private.npc.revive"
            | "private.npc.setRelationship"
            | "private.traders.addItem"
            | "private.traders.removeItem"
            | "private.story.apply"
    )
}
fn exclusive(edit: &Edit) -> bool {
    matches!(
        edit.path.as_str(),
        "private.story.apply" | "private.inventory.reset"
    )
}

/// The pending registry replaces declarative intents while retaining every add.
pub fn replacement_key(raw: &Value) -> Option<String> {
    let edit: Edit = serde_json::from_value(raw.clone()).ok()?;
    if edit.path == "public.m_PlayerSaveName" {
        return Some(edit.path);
    }
    let spec = crate::parse_private_edit(&edit).ok()?;
    if let Some((kind, key)) = crate::structured_edit_target(&spec) {
        return Some(format!("{kind}:{key}"));
    }
    if edit.path == "private.typed.setValue" {
        return Some(format!("typed:{:?}", typed(&edit).ok()??));
    }
    match edit.path.as_str() {
        "private.player.setPlayerName"
        | "private.profile.setProfileName"
        | "private.player.setTransform"
        | "private.inventory.repairSlots" => Some(edit.path),
        "private.player.setAttribute" => Some(format!("{}:{}", edit.path, edit.value["id"])),
        "private.inventory.setItemCount" => Some(format!(
            "{}:{}:{}:{}:{}",
            edit.path,
            edit.value["actorId"],
            edit.value["containerType"],
            edit.value["slotId"],
            edit.value["path"]
        )),
        _ => None,
    }
}

/// Returns groups of original edit indices, preserving identity even for equal adds.
pub fn plan(raw: &[Value]) -> Result<Vec<Vec<usize>>, CoreError> {
    let edits = parse(raw)?;
    let specs = edits
        .iter()
        .map(|e| {
            if e.path.starts_with("private.") {
                crate::parse_private_edit(e).map(Some)
            } else if e.path == "public.m_PlayerSaveName" && e.value.is_string() {
                Ok(None)
            } else {
                Err(conflict(format!("unsupported edit {}", e.path)))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let paths = edits.iter().map(typed).collect::<Result<Vec<_>, _>>()?;
    let mut arrays: Vec<(Vec<PathSeg>, Vec<usize>)> = Vec::new();
    for (i, edit) in edits.iter().enumerate() {
        if !is_array(edit) {
            continue;
        }
        let path = paths[i]
            .clone()
            .ok_or_else(|| invalid("array path required"))?;
        let index = edit.value["index"]
            .as_u64()
            .ok_or_else(|| invalid("array index must be a non-negative integer"))?;
        if let Some((_, group)) = arrays.iter_mut().find(|(p, _)| *p == path) {
            if group
                .iter()
                .any(|j| edits[*j].value["index"].as_u64() == Some(index))
            {
                return Err(pending("structuralMultiple", Some(&path)));
            }
            group.push(i);
        } else {
            arrays.push((path, vec![i]));
        }
    }
    for (path, group) in &mut arrays {
        if group.len() > 1
            && group
                .iter()
                .any(|i| edits[*i].path == "private.typed.arrayDuplicate")
        {
            return Err(pending("structuralMultiple", Some(path)));
        }
        if paths.iter().enumerate().any(|(i, p)| {
            edits[i].path == "private.typed.setValue"
                && p.as_ref().is_some_and(|p| p.starts_with(path))
        }) {
            return Err(pending("structuralValue", Some(path)));
        }
        if crate::path_has_name(path, "MemorizedEvents")
            && edits.iter().any(|e| e.path == "private.npc.revive")
        {
            return Err(pending("structuralMultiple", Some(path)));
        }
        group.sort_by_key(|i| std::cmp::Reverse(edits[*i].value["index"].as_u64().unwrap()));
    }
    for (i, edit) in edits.iter().enumerate() {
        for j in i + 1..edits.len() {
            if edit.path == "private.typed.setValue"
                && edits[j].path == edit.path
                && paths[i] == paths[j]
            {
                return Err(pending("property", paths[i].as_deref()));
            }
        }
        if edit.path == "private.inventory.reset" {
            if edits.iter().enumerate().any(|(j, e)| {
                j != i
                    && ((e.path.starts_with("private.inventory.")
                        && e.path != "private.inventory.repairSlots"
                        && actor(e) == actor(edit))
                        || paths[j]
                            .as_ref()
                            .is_some_and(|p| crate::path_has_name(p, "m_Inventory")))
            }) {
                return Err(pending("inventoryReset", None));
            }
        }
        let Some(spec) = &specs[i] else {
            continue;
        };
        for (j, path) in paths.iter().enumerate() {
            let Some(path) = path else {
                continue;
            };
            match spec {
                PrivateEdit::NpcRelationship(_) | PrivateEdit::LockSetUnlocked(_) => {
                    if crate::structured_edit_rewrites(spec, path) {
                        return Err(pending(
                            if matches!(spec, PrivateEdit::NpcRelationship(_)) {
                                "relationship"
                            } else {
                                "property"
                            },
                            Some(path),
                        ));
                    }
                }
                PrivateEdit::InventoryAddItem(_) | PrivateEdit::InventoryRemoveItem(_) => {
                    if crate::path_reaches_inventory_slot(path) {
                        return Err(pending("inventorySlot", None));
                    }
                }
                PrivateEdit::InventoryRepairSlots => {
                    if matches!(path.as_slice(), [..,PathSeg::Name(slots),PathSeg::Index(_),PathSeg::Name(id)] if slots=="m_Slots" && id=="m_Id")
                    {
                        return Err(pending("inventorySlot", None));
                    }
                }
                PrivateEdit::GlossarySetSegment(_) => {
                    if crate::path_has_name(path, "MemorizedEvents")
                        && crate::path_has_key(path, "Hero")
                    {
                        return Err(pending("glossaryMemory", Some(path)));
                    }
                    let raw = edit
                        .value
                        .get("questStatePath")
                        .or_else(|| edit.value.get("statePath"));
                    if let Some(raw) = raw {
                        let strings: Vec<String> = serde_json::from_value(raw.clone())
                            .map_err(|e| invalid(e.to_string()))?;
                        if crate::properties::parse_path(&strings)? == *path {
                            return Err(pending("glossaryQuest", Some(path)));
                        }
                    }
                }
                PrivateEdit::SkillSet(skill) => {
                    if crate::path_has_name(path, "ActiveEffects")
                        && crate::path_has_key(path, &skill.actor)
                        && matches!(path.as_slice(),[..,PathSeg::Name(effect),PathSeg::Name(name)] if effect=="EffectSpec" && name == "Def")
                    {
                        return Err(pending("skillsEffect", None));
                    }
                }
                PrivateEdit::TraderSetStock(_)
                | PrivateEdit::TraderAddItem(_)
                | PrivateEdit::TraderRemoveItem(_) => {
                    if is_array(&edits[j]) && crate::path_targets_the_trader_array(path) {
                        return Err(pending("traderArray", None));
                    }
                }
                _ => {}
            }
        }
    }
    let mut fixed = Vec::new();
    let mut splice = Vec::new();
    let mut skills = Vec::new();
    let mut repair = Vec::new();
    let mut story = Vec::new();
    for (i, e) in edits.iter().enumerate() {
        match e.path.as_str() {
            "private.skills.set" => skills.push(i),
            "private.inventory.repairSlots" => repair.push(i),
            "private.story.apply" => story.push(i),
            _ if splicing(e) => splice.push(i),
            _ => fixed.push(i),
        }
    }
    fixed.sort_by_key(|i| {
        // A public rename is independent of private ordinals. Put it in the
        // first group so its paired profile backup uses the pristine save.
        (
            edits[*i].path != "public.m_PlayerSaveName",
            specs[*i]
                .as_ref()
                .is_some_and(crate::may_invalidate_caller_ordinals),
        )
    });
    // Pose and routine fields are independent of unrelated fixed edits. Keep
    // all fields for one NPC adjacent so an overlapping stock/attribute edit
    // cannot split a placement action across atomic writes.
    let mut grouped = Vec::with_capacity(fixed.len());
    let mut used = vec![false; raw.len()];
    for &index in &fixed {
        if used[index] {
            continue;
        }
        if let Some(npc) = placement_actor(&raw[index]) {
            for &sibling in &fixed {
                if !used[sibling]
                    && placement_actor(&raw[sibling])
                        .is_some_and(|actor| actor.eq_ignore_ascii_case(npc))
                {
                    used[sibling] = true;
                    grouped.push(sibling);
                }
            }
        } else {
            used[index] = true;
            grouped.push(index);
        }
    }
    fixed = grouped;
    for (_, group) in &arrays {
        let positions: Vec<_> = splice
            .iter()
            .enumerate()
            .filter(|(_, i)| group.contains(i))
            .map(|(pos, _)| pos)
            .collect();
        for (pos, i) in positions.into_iter().zip(group) {
            splice[pos] = *i;
        }
    }
    let positions: Vec<_> = splice
        .iter()
        .enumerate()
        .filter(|(_, i)| edits[**i].path == "private.glossary.setSegment")
        .map(|(pos, _)| pos)
        .collect();
    let mut glossary: Vec<_> = positions.iter().map(|p| splice[*p]).collect();
    glossary.sort_by_key(|i| edits[*i].value["unlocked"] != true);
    for (pos, i) in positions.into_iter().zip(glossary) {
        splice[pos] = i;
    }
    let ordered: Vec<_> = fixed
        .into_iter()
        .chain(splice)
        .chain(skills)
        .chain(repair)
        .chain(story)
        .collect();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    let mut moving = false;
    for i in ordered {
        let exclusive = exclusive(&edits[i]);
        let ordinal = specs[i].as_ref().is_some_and(crate::carries_caller_ordinal);
        let overlap = current.iter().any(|j| match (&specs[i], &specs[*j]) {
            (Some(a), Some(b)) => {
                (crate::structured_edit_target(a).is_some()
                    && crate::structured_edit_target(a) == crate::structured_edit_target(b))
                    || crate::raw_typed_path(a)
                        .is_some_and(|p| crate::structured_edit_rewrites(b, p))
                    || crate::raw_typed_path(b)
                        .is_some_and(|p| crate::structured_edit_rewrites(a, p))
            }
            _ => false,
        });
        if exclusive || (moving && ordinal) || overlap {
            if !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
            moving = false;
        }
        current.push(i);
        moving |= specs[i]
            .as_ref()
            .is_some_and(crate::may_invalidate_caller_ordinals);
        if exclusive {
            groups.push(std::mem::take(&mut current));
            moving = false;
        }
    }
    if !current.is_empty() {
        groups.push(current);
    }
    Ok(groups)
}

pub fn plan_request(payload: &Value) -> Result<Value, CoreError> {
    let edits = payload["edits"]
        .as_array()
        .ok_or_else(|| invalid("edits must be an array"))?;
    Ok(json!({"groups": plan(edits)?}))
}

/// Simulates every step using the real byte appliers without writing backups or sidecars.
pub fn simulate(
    path: &Path,
    raw: &[Value],
    groups: &[Vec<usize>],
) -> Result<Vec<String>, CoreError> {
    let edits = parse(raw)?;
    let mut bytes = fs::read(path)?;
    let backend = crate::codec_backend::KrakenBackend::default();
    let mut hashes = vec![crate::sha1_hex(&bytes)];
    for group in groups {
        let public: Vec<_> = group
            .iter()
            .map(|i| &edits[*i])
            .filter(|e| !e.path.starts_with("private."))
            .collect();
        for edit in public {
            crate::apply_public_edit(&mut bytes, edit)?;
        }
        let private: Vec<_> = group
            .iter()
            .map(|i| &edits[*i])
            .filter(|e| e.path.starts_with("private."))
            .collect();
        if !private.is_empty() {
            bytes = crate::apply_private_edits(&bytes, &private, Some(&backend))?;
        }
        crate::inspect_bytes(&bytes, None, false)?;
        if bytes.starts_with(b"GSAV") && crate::rebuild_gsav_preserving_stream(&bytes)? != bytes {
            return Err(CoreError::Validation(
                "edited GSAV does not rebuild byte-identically".into(),
            ));
        }
        hashes.push(crate::sha1_hex(&bytes));
    }
    Ok(hashes)
}

pub fn apply_request(payload: &Value) -> Result<Value, CoreError> {
    apply_with_progress(payload, |_| {})
}

pub fn apply_with_progress(
    payload: &Value,
    mut progress: impl FnMut(Value),
) -> Result<Value, CoreError> {
    let path = crate::required_path(payload)?;
    let raw = payload["edits"]
        .as_array()
        .ok_or_else(|| invalid("edits must be an array"))?;
    let groups = plan(raw)?;
    let hashes = simulate(&path, raw, &groups)?;
    if payload["expectedSha1"]
        .as_str()
        .is_some_and(|expected| expected != hashes[0])
    {
        return Err(CoreError::Validation(
            "save changed since the draft was created".into(),
        ));
    }
    let sync = payload["syncPersistentDataList"].as_bool().unwrap_or(false);
    if sync {
        if payload["outputPath"].is_string() {
            return Err(invalid(
                "profile synchronization cannot be combined with --out",
            ));
        }
        crate::prepare_persistent_data_list_sync(&path, &parse(raw)?)?;
        crate::api::check_persistent_snapshot(&path, payload)?;
    }
    if let Some(notes) = payload.get("placementNotes") {
        crate::placement::parse_records(notes)?;
    }
    if let Some(clears) = payload.get("clearPlacementNotes") {
        crate::placement::parse_clears(clears)?;
    }
    let sidecars = placement_sidecar_groups(payload, raw, &groups)?;
    if payload["dryRun"].as_bool().unwrap_or(false) {
        return Ok(
            json!({"dryRun":true,"groups":groups,"expectedSha1":hashes[0],"resultSha1":hashes.last(),"committed":[],"remaining":(0..raw.len()).collect::<Vec<_>>() }),
        );
    }
    let output = payload["outputPath"].as_str();
    let target = output.map(Path::new).unwrap_or(&path);
    let mut results = Vec::new();
    let mut committed = Vec::new();
    let mut error = None;
    for (step, group) in groups.iter().enumerate() {
        let source = if step == 0 { path.as_path() } else { target };
        let current = match fs::read(source) {
            Ok(bytes) => bytes,
            Err(err) => {
                error = Some(err.to_string());
                break;
            }
        };
        if crate::sha1_hex(&current) != hashes[step] {
            error = Some("save changed before the next write step".to_string());
            break;
        }
        let mut request = payload.clone();
        request["path"] = json!(source);
        request["edits"] = json!(group.iter().map(|i| &raw[*i]).collect::<Vec<_>>());
        request["expectedSha1"] = json!(hashes[step]);
        request["backup"] = json!(step == 0 && payload["backup"].as_bool().unwrap_or(true));
        request["syncPersistentDataList"] = json!(
            sync && group
                .iter()
                .any(|i| raw[*i]["path"] == "public.m_PlayerSaveName")
        );
        for key in ["placementNotes", "clearPlacementNotes"] {
            request.as_object_mut().unwrap().remove(key);
            if let Some(entries) = sidecars[step].get(key) {
                request[key] = entries.clone();
            }
        }
        if step > 0 {
            request.as_object_mut().unwrap().remove("outputPath");
        }
        match crate::api::execute(&crate::api::Request {
            command: "write_save".into(),
            payload: request,
        }) {
            Ok(result) => {
                results.push(result);
                committed.extend(group.iter().copied());
                progress(
                    json!({"step":step+1,"steps":groups.len(),"committed":committed,"remaining":raw.len()-committed.len()}),
                );
            }
            Err(err) => {
                error = Some(err.to_string());
                break;
            }
        }
    }
    let remaining: Vec<_> = (0..raw.len()).filter(|i| !committed.contains(i)).collect();
    Ok(
        json!({"complete":error.is_none(),"error":error,"committed":committed,"remaining":remaining,
        "results":results,"path":target,"sha1":fs::read(target).ok().map(|b|crate::sha1_hex(&b))}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn removal_indices_descend_and_keep_equal_adds_distinct() {
        let raw = vec![
            json!({"path":"private.typed.arrayRemove","value":{"path":["Events"],"index":1}}),
            json!({"path":"private.typed.arrayRemove","value":{"path":["Events"],"index":5}}),
        ];
        assert_eq!(plan(&raw).unwrap(), vec![vec![1], vec![0]]);
        let add = json!({"path":"private.inventory.addItem","value":{"path":"/Script/Angelscript.ItMi_Orenugget","count":1}});
        assert_eq!(
            plan(&[add.clone(), add]).unwrap().iter().flatten().count(),
            2
        );
    }
    #[test]
    fn moving_array_cannot_retarget_a_value_even_across_steps() {
        let edits = [
            json!({"path":"private.typed.arrayRemove","value":{"path":["Events"],"index":1}}),
            json!({"path":"private.typed.setValue","value":{"path":["Events","[2]","Magnitude"],"value":1}}),
        ];
        assert!(plan(&edits).is_err());
    }

    fn raw(path: &[&str]) -> Value {
        json!({"path":"private.typed.setValue","value":{"path":path,"value":1}})
    }
    #[test]
    fn aliases_and_rewrites_cannot_hide_conflicting_targets() {
        let a = raw(&["Events", "[04]", "Magnitude"]);
        let b = raw(&["Events", "[4]", "Magnitude"]);
        assert!(matches!(
            plan(&[a, b]),
            Err(CoreError::PlanConflict {
                kind: "property",
                ..
            })
        ));
        let repair = json!({"path":"private.inventory.repairSlots","value":{}});
        assert!(
            plan(&[
                repair.clone(),
                raw(&["m_Inventory", "m_Slots", "[1]", "m_Id"])
            ])
            .is_err()
        );
        assert_eq!(
            plan(&[
                repair,
                raw(&["m_Inventory", "m_Slots", "[1]", "m_Payload", "m_Id"])
            ])
            .unwrap()
            .len(),
            2
        );
        let add = json!({"path":"private.inventory.addItem","value":{"path":"/Script/Angelscript.ItMi_Orenugget","count":1}});
        assert!(plan(&[add, raw(&["m_Inventory", "m_Slots", "[1]", "m_Count"])]).is_err());
    }
    #[test]
    fn pending_targets_replace_scalars_but_keep_distinct_adds() {
        assert_eq!(
            replacement_key(&raw(&["Events", "[04]", "Magnitude"])),
            replacement_key(&raw(&["Events", "[4]", "Magnitude"]))
        );
        let add = json!({"path":"private.inventory.addItem","value":{"path":"/Script/Angelscript.ItMi_Orenugget","count":1}});
        assert_eq!(replacement_key(&add), None);
        assert_eq!(
            replacement_key(
                &json!({"path":"private.player.setAttribute","value":{"id":"Health","field":"BaseValue","value":4}})
            ),
            replacement_key(
                &json!({"path":"private.player.setAttribute","value":{"id":"Health","field":"CurrentValue","value":4}})
            )
        );
    }
    #[test]
    fn simulation_and_partial_commit_preserve_unconsumed_operation_identity() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("G1R-001.sav");
        let bytes = include_bytes!("../assets/start_saves/resources_gothic.sav");
        fs::write(&path, bytes).unwrap();
        let raw = json!([
            {"path":"public.m_PlayerSaveName","value":"Planned"},
            {"path":"private.story.apply","value":{"changes":[{"id":"CLI_Parity_Test","present":true,"rawValue":7,"expected":{"stored":false},"allowUnknownCreate":true}]}}
        ]);
        let mut request = json!({"path":path,"edits":raw,"dryRun":true});
        let simulated = apply_request(&request).unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert!(!temp.path().join("goresave_backups").exists());
        assert_eq!(simulated["groups"], json!([[0], [1]]));
        request["expectedSha1"] = json!("stale");
        assert!(apply_request(&request).is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        request.as_object_mut().unwrap().remove("expectedSha1");
        request["dryRun"] = json!(false);
        let result = apply_with_progress(&request, |progress| {
            assert_eq!(progress["step"], 1);
            fs::write(&path, b"external change").unwrap();
        })
        .unwrap();
        assert_eq!(result["complete"], false);
        assert_eq!(result["committed"], json!([0]));
        assert_eq!(result["remaining"], json!([1]));
        assert_eq!(fs::read(&path).unwrap(), b"external change");
        assert_eq!(
            crate::api::execute(&crate::api::Request {
                command: "list_backups".into(),
                payload: json!({"path":path})
            })
            .unwrap()["backups"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn interleaved_npc_pose_and_routine_edits_stay_in_one_write_group() {
        let stock = |count| json!({"path":"private.traders.setStock","value":{"index":0,"path":crate::traders::ORE_PATH,"count":count}});
        let edits = [
            stock(100),
            raw(&["PositionByGlobalId", "{NPC-A}", "CharacterLocation"]),
            stock(101),
            raw(&["DailyRoutineByGlobalId", "{NPC-A}", "DailyRoutineClass"]),
            raw(&["PositionByGlobalId", "{NPC-B}", "CharacterLocation"]),
        ];
        let groups = plan(&edits).unwrap();
        assert_eq!(groups.len(), 2);
        assert!(
            groups
                .iter()
                .any(|group| group.contains(&1) && group.contains(&3)),
            "another overlapping edit must not separate an NPC's pose and routine"
        );
    }

    #[test]
    fn partial_draft_retains_only_unconsumed_npc_placement_sidecars() {
        let mut payload = json!({"edits":[
            raw(&["PositionByGlobalId", "{npc-b}", "CharacterLocation"]),
            raw(&["_AttributeSet", "{NPC-A}", "Health"])
        ], "placementNotes":[{"npc":"NPC-A"},{"npc":"NPC-B"}],
            "clearPlacementNotes":["NPC-A","NPC-B"]});
        retain_pending_placement_sidecars(&mut payload);
        assert_eq!(payload["placementNotes"], json!([{"npc":"NPC-B"}]));
        assert_eq!(payload["clearPlacementNotes"], json!(["NPC-B"]));
        payload["edits"] = json!([]);
        retain_pending_placement_sidecars(&mut payload);
        assert!(payload.get("placementNotes").is_none());
        assert!(payload.get("clearPlacementNotes").is_none());
    }

    #[test]
    fn placement_sidecars_wait_for_the_associated_write_and_survive_partial_failure() {
        for clear in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("G1R-001.sav");
            let bytes = include_bytes!("../assets/start_saves/resources_gothic.sav");
            fs::write(&path, bytes).unwrap();
            let npc = "OC_STT_Diego-WP_EZ_START_DIEGO_SPAWN";
            let pose = crate::api::execute(&crate::api::Request {
                command: "private.npc.position".into(),
                payload: json!({"path":path,"id":npc}),
            })
            .unwrap()["pose"]
                .clone();
            let original = &pose["location"];
            let next = json!({"x":original["x"].as_f64().unwrap()+10.0,"y":original["y"],"z":original["z"]});
            let note = crate::placement::PlacementNote {
                original_location: [
                    original["x"].as_f64().unwrap(),
                    original["y"].as_f64().unwrap(),
                    original["z"].as_f64().unwrap(),
                ],
                written_location: [
                    next["x"].as_f64().unwrap(),
                    next["y"].as_f64().unwrap(),
                    next["z"].as_f64().unwrap(),
                ],
                original_rotation: None,
                written_rotation: None,
                original_routine_class: None,
                written_routine_class: None,
            };
            if clear {
                let previous = crate::placement::PlacementNote {
                    original_location: note.written_location,
                    written_location: note.original_location,
                    ..note.clone()
                };
                crate::placement::record(&path, &[(npc.into(), previous)]).unwrap();
            }
            let initial = crate::placement::read_notes(&path);
            let root = crate::decode_private_root_cached(
                &path,
                &crate::codec_backend::KrakenBackend::default(),
            )
            .unwrap();
            let index = crate::traders::list_traders(&root)
                .unwrap()
                .into_iter()
                .find(|trader| trader.ore.is_some() && !trader.placeholder)
                .unwrap()
                .index;
            drop(root);
            let raw = json!([
                {"path":"private.traders.setStock","value":{"index":index,"path":crate::traders::ORE_PATH,"count":100}},
                {"path":"private.traders.setStock","value":{"index":index,"path":crate::traders::ORE_PATH,"count":101}},
                {"path":"private.typed.setValue","value":{"path":pose["locationPath"],"value":next}}
            ]);
            let mut request = json!({"path":path,"edits":raw,"backup":false});
            if clear {
                request["clearPlacementNotes"] = json!([npc]);
            } else {
                request["placementNotes"] = json!([{"npc":npc,"note":note}]);
            }
            let interrupted = apply_with_progress(&request, |progress| {
                assert_eq!(progress["step"], 1);
                assert_eq!(
                    crate::placement::read_notes(&path),
                    initial,
                    "an unrelated committed group must not publish or clear the NPC's sidecar"
                );
                fs::write(&path, b"external change").unwrap();
            })
            .unwrap();
            assert_eq!(interrupted["committed"], json!([0]));
            assert_eq!(interrupted["remaining"], json!([1, 2]));
            assert_eq!(crate::placement::read_notes(&path), initial);
            let mut remaining = request.clone();
            remaining["edits"] = json!([raw[1], raw[2]]);
            retain_pending_placement_sidecars(&mut remaining);
            assert!(
                remaining
                    .get(if clear {
                        "clearPlacementNotes"
                    } else {
                        "placementNotes"
                    })
                    .is_some()
            );
            fs::write(&path, bytes).unwrap();
            let complete = apply_request(&request).unwrap();
            assert_eq!(complete["complete"], true, "{complete}");
            let notes = crate::placement::read_notes(&path);
            if clear {
                assert!(!notes.contains_key(npc));
            } else {
                assert_eq!(notes.get(npc), Some(&note));
            }
            let pose = crate::api::execute(&crate::api::Request {
                command: "private.npc.position".into(),
                payload: json!({"path":path,"id":npc}),
            })
            .unwrap();
            assert_eq!(pose["pose"]["location"], next);
        }
    }
}
