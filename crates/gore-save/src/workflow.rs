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
pub(super) fn reject_duplicate_public_renames(edits: &[Edit]) -> Result<(), CoreError> {
    if edits
        .iter()
        .filter(|edit| edit.path == "public.m_PlayerSaveName")
        .take(2)
        .count()
        == 2
    {
        let path = [
            PathSeg::Name("public".into()),
            PathSeg::Name("m_PlayerSaveName".into()),
        ];
        return Err(pending("property", Some(&path)));
    }
    Ok(())
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
        if let Some(entries) = payload.get_mut(key).and_then(Value::as_array_mut) {
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
    plan_with_root(raw, None)
}

/// Only case-only opposing set values need the source's element descriptor.
pub fn plan_for_save(path: &Path, raw: &[Value]) -> Result<Vec<Vec<usize>>, CoreError> {
    let set_specs = parse(raw)?
        .iter()
        .filter(|edit| {
            matches!(
                edit.path.as_str(),
                "private.typed.setAdd" | "private.typed.setRemove"
            )
        })
        .map(crate::parse_private_edit)
        .collect::<Result<Vec<_>, _>>()?;
    if crate::case_only_opposing_set_paths(&set_specs).is_empty() {
        return plan(raw);
    }
    let root =
        crate::decode_private_root_cached(path, &crate::codec_backend::KrakenBackend::default())?;
    plan_with_root(raw, Some(&root))
}

fn plan_with_root(
    raw: &[Value],
    root: Option<&crate::properties::RootObject>,
) -> Result<Vec<Vec<usize>>, CoreError> {
    let edits = parse(raw)?;
    reject_duplicate_public_renames(&edits)?;
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
    let targets = specs
        .iter()
        .map(|spec| spec.as_ref().and_then(crate::structured_edit_target))
        .collect::<Vec<_>>();
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
        if paths
            .iter()
            .enumerate()
            .any(|(i, p)| !group.contains(&i) && p.as_ref().is_some_and(|p| p.starts_with(path)))
        {
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
            if let Some((path, value)) = specs[i]
                .as_ref()
                .zip(specs[j].as_ref())
                .and_then(|(first, second)| crate::opposing_set_element(first, second, true))
            {
                let exact =
                    specs[i]
                        .as_ref()
                        .zip(specs[j].as_ref())
                        .is_some_and(|(first, second)| {
                            crate::opposing_set_element(first, second, false).is_some()
                        });
                if exact || root.is_none() || crate::set_path_folds_case(root.unwrap(), path)? {
                    let mut path = path.to_vec();
                    path.push(PathSeg::MapKey(value.into()));
                    return Err(pending("property", Some(&path)));
                }
            }
            let overlap = if targets[i].is_some() && targets[i] == targets[j] {
                targets[i].clone()
            } else {
                specs[i]
                    .as_ref()
                    .zip(specs[j].as_ref())
                    .and_then(|(first, second)| crate::player_field_overlap(first, second))
            };
            if let Some((description, key)) = overlap {
                let key = key
                    .split(['\u{1f}', '\u{1e}'])
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
                    .join(" › ");
                let path = [PathSeg::Name(description.into()), PathSeg::Name(key)];
                return Err(pending("property", Some(&path)));
            }
            if specs[i]
                .as_ref()
                .zip(specs[j].as_ref())
                .is_some_and(|(first, second)| {
                    crate::inventory_count_removal_conflict(first, second)
                })
            {
                return Err(pending("inventorySlot", None));
            }
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
                        || paths[j].as_ref().is_some_and(|p| {
                            specs[i]
                                .as_ref()
                                .is_some_and(|spec| crate::structured_edit_rewrites(spec, p))
                        }))
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
                PrivateEdit::PlayerName(_)
                | PrivateEdit::ProfileName(_)
                | PrivateEdit::PlayerAttribute(_)
                | PrivateEdit::PlayerTransform(_)
                | PrivateEdit::InventoryItemCount(_)
                | PrivateEdit::StoryApply(_)
                | PrivateEdit::NpcRelationship(_)
                | PrivateEdit::LockSetUnlocked(_)
                | PrivateEdit::NpcRevive(_)
                | PrivateEdit::KnowledgeAddCharacter(_)
                | PrivateEdit::KnowledgeSetEntry(_)
                | PrivateEdit::FactionsForgive(_) => {
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
                    if crate::structured_edit_rewrites(spec, path) {
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
                        .or_else(|| edit.value.get("statePath"))
                        .filter(|path| !path.is_null());
                    if let Some(raw) = raw {
                        let strings: Vec<String> = serde_json::from_value(raw.clone())
                            .map_err(|e| invalid(e.to_string()))?;
                        if crate::properties::parse_path(&strings)? == *path {
                            return Err(pending("glossaryQuest", Some(path)));
                        }
                    } else if crate::path_is_a_quest_current_state(path) {
                        return Err(pending("glossaryQuest", Some(path)));
                    }
                }
                PrivateEdit::SkillSet(_) => {
                    if crate::structured_edit_rewrites(spec, path) {
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
                crate::raw_typed_path(a).is_some_and(|p| crate::structured_edit_rewrites(b, p))
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
    let groups = if let Some(path) = payload["path"].as_str() {
        plan_for_save(Path::new(path), edits)?
    } else {
        plan(edits)?
    };
    if let Some(notes) = payload.get("placementNotes") {
        crate::placement::parse_records(notes)?;
    }
    if let Some(clears) = payload.get("clearPlacementNotes") {
        crate::placement::parse_clears(clears)?;
    }
    let sidecars = placement_sidecar_groups(payload, edits, &groups)?;
    Ok(json!({"groups":groups,"sidecars":sidecars}))
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
    if let Some(output) = payload["outputPath"].as_str() {
        crate::api::validate_output_path(&path, Path::new(output))?;
    }
    crate::api::check_edit_persistent_snapshots(&path, raw)?;
    let groups = plan_for_save(&path, raw)?;
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
    let mut operations = raw.clone();
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
        request["edits"] = json!(group.iter().map(|i| &operations[*i]).collect::<Vec<_>>());
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
                crate::api::refresh_edit_persistent_snapshots(&path, &mut operations, &result);
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
    fn opposing_set_elements_conflict_but_independent_elements_batch() {
        let add = json!({"path":"private.typed.setAdd","value":{"path":["Events","[01]","Knowledge"],"value":"ChoiceB"}});
        let remove = json!({"path":"private.typed.setRemove","value":{"path":["Events","[1]","Knowledge"],"value":"ChoiceB"}});
        for edits in [
            vec![add.clone(), remove.clone()],
            vec![remove.clone(), add.clone()],
        ] {
            assert!(
                matches!(
                    plan(&edits),
                    Err(CoreError::PlanConflict {
                        kind: "property",
                        ..
                    })
                ),
                "{edits:?}"
            );
        }
        let mut case_alias = remove.clone();
        case_alias["value"]["value"] = json!("choiceb");
        assert!(matches!(
            plan(&[add.clone(), case_alias]),
            Err(CoreError::PlanConflict {
                kind: "property",
                ..
            })
        ));
        let mut other_element = remove.clone();
        other_element["value"]["value"] = json!("ChoiceA");
        assert!(plan(&[add.clone(), other_element]).is_ok());
        let mut other_set = remove.clone();
        other_set["value"]["path"] = json!(["Other"]);
        assert!(plan(&[add.clone(), other_set]).is_ok());
        assert!(plan(&[add.clone(), add]).is_ok());
        assert!(plan(&[remove.clone(), remove]).is_ok());
    }
    #[test]
    fn repeated_private_names_are_refused_in_either_order() {
        for path in [
            "private.player.setPlayerName",
            "private.profile.setProfileName",
        ] {
            let first = json!({"path":path,"value":"First name"});
            let second = json!({"path":path,"value":{"name":"Second name"}});
            for edit in [&first, &second] {
                assert!(plan(&[edit.clone()]).is_ok());
            }
            for edits in [
                vec![first.clone(), second.clone()],
                vec![second, first.clone()],
                vec![first.clone(), first],
            ] {
                assert!(
                    matches!(
                        plan(&edits),
                        Err(CoreError::PlanConflict {
                            kind: "property",
                            ..
                        })
                    ),
                    "{edits:?}"
                );
            }
        }
        let public = json!({"path":"public.m_PlayerSaveName","value":"Save name"});
        let player = json!({"path":"private.player.setPlayerName","value":"Player name"});
        let profile = json!({"path":"private.profile.setProfileName","value":"Profile name"});
        assert!(plan(&[public, player, profile]).is_ok());
    }
    #[test]
    fn player_components_conflict_only_when_fields_overlap() {
        let base =
            json!({"path":"private.player.setAttribute","value":{"id":"Strength","baseValue":20}});
        let current = json!({"path":"private.player.setAttribute","value":{"id":"Strength","currentValue":21}});
        let both =
            json!({"path":"private.player.setAttribute","value":{"id":"Strength","value":22}});
        let location =
            json!({"path":"private.player.setTransform","value":{"location":{"x":1,"y":2,"z":3}}});
        let rotation = json!({"path":"private.player.setTransform","value":{"rotation":{"pitch":4,"yaw":5,"roll":6}}});
        let transform = json!({"path":"private.player.setTransform","value":{"location":{"x":7,"y":8,"z":9},"rotation":{"pitch":10,"yaw":11,"roll":12}}});
        for (first, second) in [
            (base.clone(), base.clone()),
            (current.clone(), current.clone()),
            (base.clone(), both.clone()),
            (current.clone(), both),
            (location.clone(), location.clone()),
            (rotation.clone(), rotation.clone()),
            (location.clone(), transform.clone()),
            (rotation.clone(), transform),
        ] {
            assert!(plan(&[first.clone()]).is_ok());
            assert!(plan(&[second.clone()]).is_ok());
            for edits in [vec![first.clone(), second.clone()], vec![second, first]] {
                assert!(
                    matches!(
                        plan(&edits),
                        Err(CoreError::PlanConflict {
                            kind: "property",
                            ..
                        })
                    ),
                    "{edits:?}"
                );
            }
        }
        assert!(plan(&[base.clone(), current]).is_ok());
        assert!(plan(&[location.clone(), rotation]).is_ok());
        let other =
            json!({"path":"private.player.setAttribute","value":{"id":"Dexterity","baseValue":23}});
        assert!(plan(&[base, other, location]).is_ok());
    }
    #[test]
    fn repeated_public_renames_are_refused_before_grouping() {
        let first = json!({"path":"public.m_PlayerSaveName","value":"First name"});
        let second = json!({"path":"public.m_PlayerSaveName","value":"Second name"});
        for edit in [&first, &second] {
            assert_eq!(plan(&[edit.clone()]).unwrap(), vec![vec![0]]);
        }
        for edits in [
            vec![first.clone(), second.clone()],
            vec![second, first.clone()],
            vec![first.clone(), first],
        ] {
            assert!(matches!(
                plan(&edits),
                Err(CoreError::PlanConflict { kind: "property", path })
                    if path == "public › m_PlayerSaveName"
            ));
        }
        let unrelated = json!({"path":"private.npc.setRelationship","value":{"id":"NPC-A","relationship":"friend"}});
        let rename = json!({"path":"public.m_PlayerSaveName","value":"Allowed name"});
        assert!(plan(&[unrelated, rename]).is_ok());
    }
    #[test]
    fn repeated_structured_targets_are_refused_before_grouping() {
        let relationship = json!({"path":"private.npc.setRelationship","value":{"id":"NPC-A","relationship":"friend"}});
        let skill = json!({"path":"private.skills.set","value":{"actor":"Hero","base":"Melee_OneHanded","tier":"Trained"}});
        let knowledge = json!({"path":"private.knowledge.setEntry","value":{"character":"Hero","entry":"Info_Test","present":true}});
        let lock = json!({"path":"private.locks.setUnlocked","value":{"lock":"Lock_Test","unlocked":true}});
        let glossary = json!({"path":"private.glossary.setSegment","value":{
            "documentClass":"/Script/Angelscript.Document_Glossary_Bloodfly",
            "segmentClass":"/Script/Angelscript.DocumentSegment_Glossary_Bloodfly_01","unlocked":true
        }});
        let stock = json!({"path":"private.traders.setStock","value":{"index":0,"path":crate::traders::ORE_PATH,"count":100}});
        let removal = json!({"path":"private.traders.removeItem","value":{"index":0,"path":crate::traders::ORE_PATH}});
        for (first, value_field, second_value, target_field, other_target, kind) in [
            (
                relationship,
                "relationship",
                json!("enemy"),
                "id",
                json!("NPC-B"),
                "property",
            ),
            (
                skill,
                "tier",
                json!("Master"),
                "base",
                json!("Ranged_Bow"),
                "property",
            ),
            (
                knowledge,
                "present",
                json!(false),
                "entry",
                json!("Info_Other"),
                "property",
            ),
            (
                lock,
                "unlocked",
                json!(false),
                "lock",
                json!("Lock_Other"),
                "property",
            ),
            (
                glossary,
                "unlocked",
                json!(false),
                "segmentClass",
                json!("/Script/Angelscript.DocumentSegment_Glossary_Bloodfly_02"),
                "property",
            ),
            (
                stock.clone(),
                "count",
                json!(101),
                "index",
                json!(1),
                "property",
            ),
            (removal, "index", json!(0), "index", json!(1), "property"),
        ] {
            let mut second = first.clone();
            second["value"][value_field] = second_value;
            assert!(plan(&[first.clone()]).is_ok(), "{first}");
            assert!(plan(&[second.clone()]).is_ok(), "{second}");
            for edits in [
                vec![first.clone(), second.clone()],
                vec![second.clone(), first.clone()],
            ] {
                assert!(
                    matches!(plan(&edits),Err(CoreError::PlanConflict{kind:actual,..}) if actual==kind),
                    "{edits:?}"
                );
            }
            let mut alias = second.clone();
            if matches!(target_field, "id" | "entry" | "lock") {
                let value = alias["value"][target_field].as_str().unwrap();
                alias["value"][target_field] = json!(value.to_ascii_lowercase());
                assert!(plan(&[alias.clone()]).is_ok());
                assert!(matches!(
                    plan(&[first.clone(), alias]),
                    Err(CoreError::PlanConflict {
                        kind: "property",
                        ..
                    })
                ));
            }
            second["value"][target_field] = other_target;
            assert!(
                plan(&[first.clone(), second.clone()]).is_ok(),
                "{first}, {second}"
            );
        }
        let removal = json!({"path":"private.traders.removeItem","value":{"index":0,"path":crate::traders::ORE_PATH}});
        for edits in [vec![stock.clone(), removal.clone()], vec![removal, stock]] {
            assert!(matches!(
                plan(&edits),
                Err(CoreError::PlanConflict {
                    kind: "property",
                    ..
                })
            ));
        }
    }
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

    #[test]
    fn structural_arrays_refuse_all_typed_container_descendants() {
        for parent in ["private.typed.arrayRemove", "private.typed.arrayDuplicate"] {
            let structural = json!({"path":parent,"value":{"path":["Events"],"index":1}});
            for operation in [
                "private.typed.setAdd",
                "private.typed.setRemove",
                "private.typed.arrayRemove",
                "private.typed.arrayDuplicate",
            ] {
                let field = if operation.starts_with("private.typed.array") {
                    "Notes"
                } else {
                    "Knowledge"
                };
                let descendant = json!({"path":operation,"value":{"path":["Events","[01]",field],"value":"ChoiceB","index":0}});
                assert!(plan(&[structural.clone()]).is_ok());
                assert!(plan(&[descendant.clone()]).is_ok());
                for edits in [
                    vec![structural.clone(), descendant.clone()],
                    vec![descendant.clone(), structural.clone()],
                ] {
                    assert!(
                        matches!(
                            plan(&edits),
                            Err(CoreError::PlanConflict {
                                kind: "structuralValue",
                                ..
                            })
                        ),
                        "{edits:?}"
                    );
                }
                let mut sibling = descendant;
                sibling["value"]["path"] = json!(["OtherEvents", "[1]", "Knowledge"]);
                assert!(plan(&[structural.clone(), sibling]).is_ok());
            }
        }
        let unrelated =
            json!({"path":"private.typed.arrayRemove","value":{"path":["Elsewhere"],"index":1}});
        let ordered = vec![
            json!({"path":"private.typed.arrayRemove","value":{"path":["Events"],"index":1}}),
            json!({"path":"private.typed.arrayRemove","value":{"path":["Events"],"index":5}}),
            unrelated,
        ];
        assert_eq!(plan(&ordered).unwrap().iter().flatten().count(), 3);
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
    fn inventory_slot_conflicts_are_scoped_to_the_structured_edits_actor() {
        let player = raw(&[
            "m_SavedPlayers",
            "[0]",
            "m_Inventory",
            "m_Slots",
            "[1]",
            "m_Count",
        ]);
        let npc = |actor: &str| {
            raw(&[
                "CharacterState",
                "_Inventory",
                &format!("{{{actor}}}"),
                "m_Slots",
                "[1]",
                "m_Count",
            ])
        };
        for operation in ["private.inventory.addItem", "private.inventory.removeItem"] {
            let hero = json!({"path":operation,"value":{
                "path":"/Script/Angelscript.ItMi_Orenugget","count":1
            }});
            let mut diego = hero.clone();
            diego["value"]["actorId"] = json!("NPC-Diego");
            for (edit, same, other) in [
                (hero, player.clone(), npc("NPC-Diego")),
                (diego, npc("NPC-Diego"), player.clone()),
            ] {
                assert!(matches!(
                    plan(&[edit.clone(), same]),
                    Err(CoreError::PlanConflict {
                        kind: "inventorySlot",
                        ..
                    })
                ));
                let groups = plan(&[edit.clone(), other]).unwrap();
                assert_eq!(
                    groups.iter().flatten().copied().collect::<Vec<_>>(),
                    vec![1, 0]
                );
                assert!(plan(&[edit, npc("NPC-Gorn")]).is_ok());
            }
        }
    }

    #[test]
    fn inventory_counts_conflict_with_removal_only_for_overlapping_stack_selectors() {
        for actor in [Value::Null, json!("NPC-Diego")] {
            let count = json!({"path":"private.inventory.setItemCount","value":{
                "actorId":actor,"path":"/Script/Angelscript.ItMi_Orenugget",
                "containerType":"MainContainer","slotId":7,"count":2
            }});
            let remove = json!({"path":"private.inventory.removeItem","value":{
                "actorId":actor,"path":"/Script/Angelscript.ItMi_Orenugget",
                "containerType":"EInventoryTypes::MainContainer","slotId":7
            }});
            for (omit_count_slot, omit_remove_slot, id_only) in [
                (false, false, false),
                (true, false, false),
                (false, true, false),
                (true, true, false),
                (false, false, true),
            ] {
                let mut count = count.clone();
                let mut remove = remove.clone();
                if omit_count_slot {
                    count["value"].as_object_mut().unwrap().remove("slotId");
                }
                if omit_remove_slot {
                    remove["value"].as_object_mut().unwrap().remove("slotId");
                }
                if id_only {
                    count["value"].as_object_mut().unwrap().remove("path");
                    count["value"]["id"] = json!("ItMi_Orenugget");
                }
                for edits in [vec![count.clone(), remove.clone()], vec![remove, count]] {
                    assert!(matches!(
                        plan(&edits),
                        Err(CoreError::PlanConflict {
                            kind: "inventorySlot",
                            ..
                        })
                    ));
                }
            }
            for (key, value) in [
                ("actorId", json!("NPC-Other")),
                ("containerType", json!("Pouch")),
                ("slotId", json!(8)),
                ("path", json!("/Script/Angelscript.ItFo_Loaf")),
            ] {
                let mut other = remove.clone();
                other["value"][key] = value;
                for edits in [
                    vec![count.clone(), other.clone()],
                    vec![other, count.clone()],
                ] {
                    assert!(plan(&edits).is_ok());
                }
            }
            assert_ne!(replacement_key(&count), replacement_key(&remove));
            let add = json!({"path":"private.inventory.addItem","value":{
                "actorId":actor,"path":"/Script/Angelscript.ItMi_Orenugget","count":1
            }});
            assert!(plan(&[count, add]).is_ok());
        }
        let legacy = json!({"path":"private.inventory.setItemCount","value":{
            "id":"ItMi_Orenugget","count":2
        }});
        let pouch = json!({"path":"private.inventory.removeItem","value":{
            "path":"/Script/Angelscript.ItMi_Orenugget","containerType":"Pouch","slotId":7
        }});
        for edits in [vec![legacy.clone(), pouch.clone()], vec![pouch, legacy]] {
            assert!(matches!(
                plan(&edits),
                Err(CoreError::PlanConflict {
                    kind: "inventorySlot",
                    ..
                })
            ));
        }
    }

    #[test]
    fn inventory_resets_guard_the_selected_actor_entire_inventory() {
        let hero = json!({"path":"private.inventory.reset","value":{}});
        let npc = json!({"path":"private.inventory.reset","value":{"actorId":"NPC-Diego"}});
        let hero_path = raw(&["m_SavedPlayers", "[0]", "m_Inventory", "m_Keys"]);
        let npc_path = |actor: &str, map: &str, field: &str| {
            raw(&[map, &format!("{{{actor}}}"), "InventoryItems", field])
        };
        for (reset, paths) in [
            (
                hero.clone(),
                vec![hero_path.clone(), raw(&["m_Inventory", "m_Slots"])],
            ),
            (
                npc.clone(),
                vec![
                    npc_path("NPC-Diego", "InventoryByGlobalId", "m_Keys"),
                    npc_path(
                        "NPC-Diego",
                        "CharacterStateSaveGameData_Inventory",
                        "m_Values",
                    ),
                    npc_path("NPC-Diego", "_Inventory", "m_Slots"),
                    raw(&["InventoryByGlobalId"]),
                ],
            ),
        ] {
            for raw in paths {
                for edits in [vec![reset.clone(), raw.clone()], vec![raw, reset.clone()]] {
                    assert!(matches!(
                        plan(&edits),
                        Err(CoreError::PlanConflict {
                            kind: "inventoryReset",
                            ..
                        })
                    ));
                }
            }
        }
        for (reset, other) in [
            (hero, npc_path("NPC-Diego", "InventoryByGlobalId", "m_Keys")),
            (npc.clone(), hero_path),
            (
                npc.clone(),
                npc_path("NPC-Gorn", "InventoryByGlobalId", "m_Keys"),
            ),
            (
                npc,
                raw(&["AttributesByGlobalId", "{NPC-Diego}", "BaseValue"]),
            ),
        ] {
            assert!(plan(&[reset.clone(), other.clone()]).is_ok());
            assert!(plan(&[other, reset]).is_ok());
        }
    }

    #[test]
    fn skill_transitions_guard_effect_descendants_and_structural_edits() {
        let skill = json!({"path":"private.skills.set","value":{"actor":"Hero","base":"Skill_Bow","tier":"Untrained"}});
        for raw in [
            raw(&[
                "ActiveEffectsByGlobalId",
                "{Hero}",
                "ActiveEffects",
                "[2]",
                "EffectSpec",
                "Duration",
            ]),
            raw(&[
                "ActiveEffectsByGlobalId",
                "{Hero}",
                "ActiveEffects",
                "[2]",
                "StackCount",
            ]),
            json!({"path":"private.typed.arrayRemove","value":{"path":["ActiveEffectsByGlobalId","{Hero}","ActiveEffects"],"index":2}}),
        ] {
            for edits in [vec![skill.clone(), raw.clone()], vec![raw, skill.clone()]] {
                assert!(matches!(
                    plan(&edits),
                    Err(CoreError::PlanConflict {
                        kind: "skillsEffect",
                        ..
                    })
                ));
            }
        }
        let peer = raw(&[
            "ActiveEffectsByGlobalId",
            "{NPC-Diego}",
            "ActiveEffects",
            "[2]",
            "EffectSpec",
            "Duration",
        ]);
        assert!(plan(&[skill.clone(), peer.clone()]).is_ok());
        assert!(plan(&[peer, skill]).is_ok());
    }

    #[test]
    fn faction_forgiveness_guards_crime_flags_but_allows_other_fields() {
        let forgive =
            json!({"path":"private.factions.forgive","value":{"guild":"Guild.Human.OldCamp"}});
        for path in [
            vec![
                "m_GenericData",
                "{CrimeMemoryPersistentData}",
                "GlobalCrimeDataEntries",
                "[0]",
                "bIsForgiven",
            ],
            vec![
                "m_GenericData",
                "{CrimeMemoryPersistentData}",
                "RelativeCrimeDataEntries",
                "{OC_STT_Diego}",
                "RelativeCrimes",
                "[0]",
                "bIsSuppressed",
            ],
        ] {
            let raw = raw(&path);
            for edits in [
                vec![forgive.clone(), raw.clone()],
                vec![raw, forgive.clone()],
            ] {
                assert!(matches!(
                    plan(&edits),
                    Err(CoreError::PlanConflict {
                        kind: "property",
                        ..
                    })
                ));
            }
        }
        for path in [
            vec![
                "m_GenericData",
                "{CrimeMemoryPersistentData}",
                "GlobalCrimeDataEntries",
                "[0]",
                "ID",
            ],
            vec![
                "m_GenericData",
                "{CrimeMemoryPersistentData}",
                "RelativeCrimeDataEntries",
                "{OC_STT_Diego}",
                "RelativeCrimes",
                "[0]",
                "BaseSeverity",
            ],
            vec![
                "m_GenericData",
                "{OtherMemory}",
                "GlobalCrimeDataEntries",
                "[0]",
                "bIsForgiven",
            ],
            vec!["Unrelated", "bIsSuppressed"],
        ] {
            let raw = raw(&path);
            assert!(plan(&[forgive.clone(), raw.clone()]).is_ok());
            assert!(plan(&[raw, forgive.clone()]).is_ok());
        }
    }

    #[test]
    fn revive_and_knowledge_rewrites_reject_raw_collisions_in_either_order() {
        let temp = tempfile::tempdir().unwrap();
        let save = temp.path().join("G1R-001.sav");
        fs::write(&save, b"guarded source").unwrap();
        let revive = json!({"path":"private.npc.revive","value":{"id":"NPC-A"}});
        let add = json!({"path":"private.knowledge.addCharacter","value":{"value":"Hero"}});
        let entry = json!({"path":"private.knowledge.setEntry","value":{
            "character":"Hero","entry":"Info_Test","present":true
        }});
        for (structured, paths) in [
            (
                revive,
                vec![
                    vec![
                        "LongTermMemoryByGlobalId",
                        "{NPC-A}",
                        "MemorizedEvents",
                        "[0]",
                        "Value",
                    ],
                    vec!["LooseTagsByGlobalId", "{NPC-A}"],
                    vec!["m_SavedInventories", "{Character_NPC-A}", "Items"],
                    vec!["m_SavedInventories", "{Character_NPC-A_123}", "Items"],
                    vec!["LooseTagsByGlobalId"],
                    vec!["m_SavedInventories"],
                ],
            ),
            (
                add,
                vec![vec![
                    "CharacterKnowledgeByUniqueName",
                    "{Hero}",
                    "Knowledge",
                ]],
            ),
            (
                entry,
                vec![vec![
                    "CharacterKnowledgeByUniqueName",
                    "{Hero}",
                    "Knowledge",
                    "Info_Test",
                ]],
            ),
        ] {
            for path in paths {
                let raw = raw(&path);
                for edits in [
                    vec![structured.clone(), raw.clone()],
                    vec![raw.clone(), structured.clone()],
                ] {
                    assert!(matches!(
                        plan(&edits),
                        Err(CoreError::PlanConflict {
                            kind: "property",
                            ..
                        })
                    ));
                    for dry_run in [false, true] {
                        assert!(matches!(
                            apply_request(&json!({"path":save,"edits":edits,"dryRun":dry_run})),
                            Err(CoreError::PlanConflict {
                                kind: "property",
                                ..
                            })
                        ));
                        assert_eq!(fs::read(&save).unwrap(), b"guarded source");
                        assert!(!temp.path().join("goresave_backups").exists());
                    }
                }
            }
            assert!(plan(&[structured, raw(&["Unrelated", "Value"])]).is_ok());
        }
        for edit in [
            json!({"path":"private.knowledge.addCharacter","value":{"value":"Hero"}}),
            json!({"path":"private.knowledge.setEntry","value":{"character":"Hero","entry":"Info_Test","present":true}}),
        ] {
            assert!(
                plan(&[
                    edit,
                    raw(&["CharacterKnowledgeByUniqueName", "{Diego}", "Knowledge"])
                ])
                .is_ok()
            );
        }
    }

    #[test]
    fn revive_health_conflicts_cover_the_same_actor_and_preserve_other_attributes() {
        let revive = json!({"path":"private.npc.revive","value":{"id":"NPC-A"}});
        for map in ["AttributesByGlobalId", "AttributesMap", "_Attributes"] {
            for class in ["/Script/G1R.AttributeSet_Health", "AttributeSet_Health"] {
                for field in ["BaseValue", "CurrentValue"] {
                    let path = [
                        map,
                        "{npc-a}",
                        "AttributeSetsByClass",
                        &format!("{{{class}}}"),
                        "Attributes",
                        "{Health}",
                        field,
                    ];
                    for length in 1..=path.len() {
                        let raw = raw(&path[..length]);
                        for edits in [
                            vec![revive.clone(), raw.clone()],
                            vec![raw.clone(), revive.clone()],
                        ] {
                            assert!(
                                matches!(
                                    plan(&edits),
                                    Err(CoreError::PlanConflict {
                                        kind: "property",
                                        ..
                                    })
                                ),
                                "{edits:?}"
                            );
                        }
                    }
                }
            }
            for (actor, attribute) in [
                ("NPC-B", "Health"),
                ("Hero", "Health"),
                ("NPC-A", "MaxHealth"),
                ("NPC-A", "Strength"),
            ] {
                let raw = raw(&[
                    map,
                    &format!("{{{actor}}}"),
                    "Attributes",
                    &format!("{{{attribute}}}"),
                    "CurrentValue",
                ]);
                assert!(plan(&[revive.clone(), raw.clone()]).is_ok());
                assert!(plan(&[raw, revive.clone()]).is_ok());
            }
        }
    }

    #[test]
    fn revive_tag_and_corpse_conflicts_allow_other_npcs_but_guard_all_memory_owners() {
        let revive = json!({"path":"private.npc.revive","value":{"id":"NPC-A"}});
        for path in [
            vec!["LooseTagsByGlobalId", "{NPC-B}"],
            vec!["m_SavedInventories", "{Character_NPC-B}", "Items"],
            vec!["m_SavedInventories", "{Character_NPC-B_123}", "Items"],
        ] {
            let raw = raw(&path);
            for edits in [vec![revive.clone(), raw.clone()], vec![raw, revive.clone()]] {
                assert!(plan(&edits).is_ok());
            }
        }
        for owner in ["NPC-A", "NPC-B", "Hero"] {
            let path = raw(&[
                "LongTermMemoryByGlobalId",
                &format!("{{{owner}}}"),
                "MemorizedEvents",
                "[0]",
                "Time",
            ]);
            assert!(matches!(
                plan(&[revive.clone(), path]),
                Err(CoreError::PlanConflict {
                    kind: "property",
                    ..
                })
            ));
        }
    }

    #[test]
    fn player_transform_conflicts_follow_requested_components_and_guard_the_player_array() {
        let location = json!({"x":1.0,"y":2.0,"z":3.0});
        let rotation = json!({"pitch":4.0,"yaw":5.0,"roll":6.0});
        for value in [
            json!({"location":location}),
            json!({"rotation":rotation}),
            json!({"location":location,"rotation":rotation}),
        ] {
            let transform = json!({"path":"private.player.setTransform","value":value});
            for (leaf, key) in [("m_Location", "location"), ("m_Rotation", "rotation")] {
                let path = raw(&["m_SavedPlayers", "[0]", leaf]);
                for edits in [
                    vec![transform.clone(), path.clone()],
                    vec![path, transform.clone()],
                ] {
                    if value.get(key).is_some() {
                        assert!(matches!(
                            plan(&edits),
                            Err(CoreError::PlanConflict {
                                kind: "property",
                                ..
                            })
                        ));
                    } else {
                        assert!(plan(&edits).is_ok());
                    }
                }
            }
            let array = json!({"path":"private.typed.arrayDuplicate","value":{"path":["m_SavedPlayers"],"index":0}});
            assert!(matches!(
                plan(&[transform.clone(), array]),
                Err(CoreError::PlanConflict {
                    kind: "property",
                    ..
                })
            ));
            assert!(
                plan(&[
                    transform.clone(),
                    raw(&["PositionByGlobalId", "{NPC-A}", "CharacterLocation"])
                ])
                .is_ok()
            );
            assert!(plan(&[transform, raw(&["NpcData", "m_Location"])]).is_ok());
        }
    }

    #[test]
    fn fixed_structured_edits_guard_names_attributes_counts_and_story_values() {
        let attribute =
            json!({"path":"private.player.setAttribute","value":{"id":"Health","baseValue":1.0}});
        let count = json!({"path":"private.inventory.setItemCount","value":{"path":"/Script/Angelscript.ItMi_Orenugget","actorId":"NPC-A","count":2}});
        let story = json!({"path":"private.story.apply","value":{"changes":[{"id":"Planner_Test","present":true,"rawValue":2,"expected":{"stored":true,"rawValue":0},"allowUnknownCreate":true}]}});
        for (structured, same, other) in [
            (
                json!({"path":"private.player.setPlayerName","value":{"name":"Hero"}}),
                vec!["PlayerData", "m_PlayerName"],
                vec!["PlayerData", "m_Title"],
            ),
            (
                json!({"path":"private.profile.setProfileName","value":{"name":"Profile"}}),
                vec!["PlayerData", "m_ProfileName"],
                vec!["PlayerData", "m_PlayerName"],
            ),
            (
                attribute.clone(),
                vec![
                    "AttributesByGlobalId",
                    "{Hero}",
                    "AttributeSetsByClass",
                    "{/Script/G1R.AttributeSet_Health}",
                    "Attributes",
                    "{Health}",
                    "BaseValue",
                ],
                vec![
                    "AttributesByGlobalId",
                    "{NPC-A}",
                    "AttributeSetsByClass",
                    "{/Script/G1R.AttributeSet_Health}",
                    "Attributes",
                    "{Health}",
                    "BaseValue",
                ],
            ),
            (
                count,
                vec![
                    "CharacterState",
                    "_Inventory",
                    "{NPC-A}",
                    "m_Slots",
                    "[1]",
                    "m_SlotData",
                    "m_ItemCount",
                ],
                vec![
                    "CharacterState",
                    "_Inventory",
                    "{NPC-B}",
                    "m_Slots",
                    "[1]",
                    "m_SlotData",
                    "m_ItemCount",
                ],
            ),
            (
                story,
                vec!["StoryPropertyValues", "{Planner_Test}"],
                vec!["StoryPropertyValues", "{Another_Test}"],
            ),
        ] {
            let same = raw(&same);
            for edits in [
                vec![structured.clone(), same.clone()],
                vec![same, structured.clone()],
            ] {
                assert!(matches!(
                    plan(&edits),
                    Err(CoreError::PlanConflict {
                        kind: "property",
                        ..
                    })
                ));
            }
            assert!(plan(&[structured, raw(&other)]).is_ok());
        }
        assert!(
            plan(&[
                attribute,
                raw(&[
                    "AttributesByGlobalId",
                    "{Hero}",
                    "AttributeSetsByClass",
                    "{/Script/G1R.AttributeSet_Health}",
                    "Attributes",
                    "{Health}",
                    "CurrentValue"
                ])
            ])
            .is_ok()
        );
    }

    #[test]
    fn derived_glossary_quest_states_conflict_before_grouping_and_explicit_paths_stay_scoped() {
        let mut segment = json!({"path":"private.glossary.setSegment","value":{
            "documentClass":"/Script/Angelscript.Document_Glossary_Bloodfly",
            "segmentClass":"/Script/Angelscript.DocumentSegment_Glossary_Bloodfly_01",
            "unlocked":true
        }});
        let path = [
            "QuestDataByClass",
            "{Quest_Glossary_Bloodfly_01}",
            "CurrentState",
        ];
        let state = raw(&path);
        for null in [false, true] {
            if null {
                segment["value"]["questStatePath"] = Value::Null;
            }
            for edits in [
                vec![segment.clone(), state.clone()],
                vec![state.clone(), segment.clone()],
            ] {
                assert!(matches!(
                    plan(&edits),
                    Err(CoreError::PlanConflict {
                        kind: "glossaryQuest",
                        ..
                    })
                ));
            }
        }
        segment["value"]["questStatePath"] = json!(path);
        assert!(matches!(
            plan(&[segment.clone(), state]),
            Err(CoreError::PlanConflict {
                kind: "glossaryQuest",
                ..
            })
        ));
        assert!(
            plan(&[
                segment,
                raw(&[
                    "QuestDataByClass",
                    "{Quest_Glossary_Wolf_01}",
                    "CurrentState"
                ])
            ])
            .is_ok()
        );
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
        let stock = |index, count| json!({"path":"private.traders.setStock","value":{"index":index,"path":crate::traders::ORE_PATH,"count":count}});
        let edits = [
            stock(0, 100),
            raw(&["PositionByGlobalId", "{NPC-A}", "CharacterLocation"]),
            stock(1, 101),
            raw(&["DailyRoutineByGlobalId", "{NPC-A}", "DailyRoutineClass"]),
            raw(&["PositionByGlobalId", "{NPC-B}", "CharacterLocation"]),
            json!({"path":"private.inventory.reset","value":{"resourcesLevel":"Gothic"}}),
        ];
        let groups = plan(&edits).unwrap();
        assert_eq!(groups.len(), 2);
        assert!(
            groups
                .iter()
                .any(|group| group.contains(&1) && group.contains(&3)),
            "an unrelated declarative edit must not separate an NPC's pose and routine"
        );
    }

    #[test]
    fn planning_assigns_placement_sidecars_to_their_npcs_write_group() {
        let stock = |count| json!({"path":"private.traders.setStock","value":{"index":0,"path":crate::traders::ORE_PATH,"count":count}});
        let edits = json!([
            stock(100),
            {"path":"private.inventory.reset","value":{"resourcesLevel":"Gothic"}},
            raw(&["PositionByGlobalId", "{NPC-A}", "CharacterLocation"])
        ]);
        let notes = json!([{"npc":"npc-a","note":{
            "original_location":[0.0,0.0,0.0],"written_location":[1.0,2.0,3.0]
        }}]);
        let payload = json!({"edits":edits,"placementNotes":notes,"clearPlacementNotes":["NPC-A"]});
        let planned = plan_request(&payload).unwrap();
        assert_eq!(planned["groups"], json!([[0, 2], [1]]));
        assert_eq!(
            planned["sidecars"],
            json!([{
                "placementNotes":notes,"clearPlacementNotes":["NPC-A"]
            }, {}])
        );
        let mut orphan = payload;
        orphan["clearPlacementNotes"] = json!(["NPC-B"]);
        assert!(plan_request(&orphan).is_err());
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
        payload["edits"] = json!([raw(&["Events", "[0]", "Magnitude"])]);
        retain_pending_placement_sidecars(&mut payload);
        assert!(payload.get("placementNotes").is_none());
        assert!(payload.get("clearPlacementNotes").is_none());
    }

    #[test]
    fn placement_sidecars_commit_with_their_write_before_a_later_group_fails() {
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
                {"path":"private.inventory.reset","value":{"resourcesLevel":"Gothic"}},
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
                    crate::placement::read_notes(&path).get(npc),
                    if clear { None } else { Some(&note) },
                    "the committed NPC group must publish or clear its own sidecar"
                );
                fs::write(&path, b"external change").unwrap();
            })
            .unwrap();
            assert_eq!(interrupted["committed"], json!([0, 2]));
            assert_eq!(interrupted["remaining"], json!([1]));
            assert_ne!(crate::placement::read_notes(&path), initial);
            let mut remaining = request.clone();
            remaining["edits"] = json!([raw[1]]);
            retain_pending_placement_sidecars(&mut remaining);
            assert!(
                remaining
                    .get(if clear {
                        "clearPlacementNotes"
                    } else {
                        "placementNotes"
                    })
                    .is_none()
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
