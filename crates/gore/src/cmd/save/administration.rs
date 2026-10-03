use super::*;
use std::io::IsTerminal;

fn settings_path(scope: &str) -> Result<PathBuf> {
    let name = match scope {
        "editor" => "settings.json",
        "ui" => "ui_settings.json",
        _ => bail!("--scope must be editor or ui"),
    };
    Ok(gore_loc::paths::shared_data_dir()
        .join("gore-save")
        .join(name))
}
pub(super) fn settings_read(scope: &str) -> Result<Value> {
    let path = settings_path(scope)?;
    let legacy = path
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(|base| {
            base.join("gore-tools/gore-save")
                .join(path.file_name().unwrap())
        });
    let old_root = if cfg!(windows) {
        std::env::var_os("APPDATA")
            .or_else(|| std::env::var_os("LOCALAPPDATA"))
            .map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|p| PathBuf::from(p).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))
    };
    let old = old_root
        .unwrap_or_else(|| PathBuf::from("."))
        .join("goresave")
        .join(path.file_name().unwrap());
    let source = if path.exists() {
        Some(path)
    } else {
        legacy
            .filter(|p| p.exists())
            .or_else(|| old.exists().then_some(old))
    };
    if let Some(path) = source {
        let value = read_json(&path)?;
        if !value.is_object() {
            bail!("settings must be a JSON object");
        }
        Ok(value)
    } else {
        Ok(json!({}))
    }
}
fn root(o: &Options) -> Result<PathBuf> {
    if let Some(root) = &o.root {
        return Ok(root.clone());
    }
    if let Some(root) = settings_read("editor")?["saveDir"].as_str() {
        return Ok(root.into());
    }
    if let Some(file) = o.save.as_deref().or(o.target.as_deref()) {
        if let Some(parent) = file.parent().filter(|p| !p.as_os_str().is_empty()) {
            return Ok(parent.to_owned());
        }
        return Ok(std::env::current_dir()?);
    }
    Ok(PathBuf::from(
        call("scan_save_dir_readonly", json!({}))?["saveRoot"]
            .as_str()
            .context("save root unavailable")?,
    ))
}
fn profile_path(o: &Options) -> Result<PathBuf> {
    Ok(root(o)?.join("PersistentDataList.sav"))
}
fn save_context(o: &Options) -> Result<Options> {
    let mut context = o.clone();
    if let Some(path) = &o.save {
        // Detaching a stale registry entry must also work after its save vanished.
        let path = normalized_path(path);
        let parent = path
            .parent()
            .context("save has no parent directory")?
            .to_owned();
        if o.root
            .as_ref()
            .is_some_and(|root| !same_path(&root.to_string_lossy(), &parent.to_string_lossy()))
        {
            bail!("--root must be the save's own directory; use save import to move it");
        }
        context.save = Some(path);
        context.root = Some(parent);
    } else if let Some(root) = &o.root {
        context.root = Some(normalized_path(root));
    }
    Ok(context)
}
fn scan(o: &Options) -> Result<Value> {
    let mut data = call("scan_save_dir_readonly", json!({"path":root(o)?}))?;
    let settings = settings_read("editor")?;
    let hidden = settings["hiddenOtherSavePaths"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(rows) = data["saves"].as_array_mut() {
        rows.retain(|r| {
            !r["persistentProfileId"].is_null()
                || !hidden.iter().any(|p| {
                    p.as_str()
                        .zip(r["path"].as_str())
                        .is_some_and(|(a, b)| same_path(a, b))
                })
        });
    }
    for external in settings["externalSavePaths"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        if hidden
            .iter()
            .any(|p| p.as_str().is_some_and(|p| same_path(p, external)))
            || data["saves"].as_array().is_some_and(|rows| {
                rows.iter()
                    .any(|r| r["path"].as_str().is_some_and(|p| same_path(p, external)))
            })
        {
            continue;
        }
        let row = match call("inspect_save", json!({"path":external})) {
            Ok(mut row) => {
                row["external"] = json!(true);
                row["path"] = json!(external);
                row
            }
            Err(e) => {
                json!({"path":external,"external":true,"available":false,"error":e.to_string()})
            }
        };
        data["saves"]
            .as_array_mut()
            .context("invalid scan saves")?
            .push(row);
    }
    Ok(data)
}

fn has_difficulty(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|m| m.values().any(|v| !v.is_null()))
}
fn resolved_resources(difficulty: &Value) -> Result<String> {
    if !has_difficulty(difficulty) {
        return Ok("Gothic".into());
    }
    if let Some(preset) = difficulty["preset"].as_str() {
        if !preset.ends_with("_Custom") && preset != "Custom" {
            return normalize_resources(preset);
        }
    }
    difficulty["resources"]
        .as_str()
        .map(normalize_resources)
        .unwrap_or_else(|| Ok("Gothic".into()))
}
fn profile_resource_settings(profile: &Value) -> Value {
    json!({"preset":profile["difficultyPreset"],"resources":profile["customResourcesSettings"]})
}
pub(super) fn resources_level(o: &Options) -> Result<String> {
    if let Some(level) = &o.resources_level {
        return normalize_resources(level);
    }
    let context = save_context(o)?;
    let listing = scan(&context)?;
    let selected = context.save.as_deref().and_then(|p| p.canonicalize().ok());
    let id = listing["saves"]
        .as_array()
        .and_then(|rows| {
            rows.iter().find(|r| {
                r["path"]
                    .as_str()
                    .and_then(|p| Path::new(p).canonicalize().ok())
                    .is_some_and(|p| Some(p) == selected)
            })
        })
        .and_then(|r| r["persistentProfileId"].as_i64());
    let profiles = listing["profiles"].as_array();
    let attached = profiles
        .and_then(|rows| {
            rows.iter()
                .find(|r| id.is_some() && r["profileId"].as_i64() == id)
        })
        .map(profile_resource_settings);
    if let Some(difficulty) = attached.filter(has_difficulty) {
        return resolved_resources(&difficulty);
    }
    if let Some(save) = &selected {
        let inspection = call("inspect_save", json!({"path":save}))?;
        if has_difficulty(&inspection["difficulty"]) {
            return resolved_resources(&inspection["difficulty"]);
        }
    }
    let active = profiles.and_then(|rows| {
        rows.iter()
            .find(|r| r["profileId"] == listing["activeProfileId"])
    });
    resolved_resources(&active.map(profile_resource_settings).unwrap_or(Value::Null))
}
pub(super) fn resources_profile_snapshot(o: &Options) -> Result<Value> {
    let profile = profile_path(&save_context(o)?)?;
    let hash = if profile.exists() {
        Some(api::file_sha1(&profile)?)
    } else {
        None
    };
    Ok(json!({"persistentPath":profile,"expectedPersistentSha1":hash}))
}
fn normalize_resources(raw: &str) -> Result<String> {
    let last = raw
        .rsplit([':', '.', '_'])
        .find(|s| !s.is_empty())
        .unwrap_or(raw)
        .to_lowercase();
    Ok(match last.as_str() {
        "novice" | "easy" => "Novice",
        "gothic" | "standard" => "Gothic",
        "hard" => "Hard",
        _ => bail!(
            "unknown resources difficulty {raw}; supply --resources-level Novice, Gothic or Hard"
        ),
    }
    .into())
}
fn normalized_path(path: &Path) -> PathBuf {
    if let Ok(path) = path.canonicalize() {
        return path;
    }
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_owned());
    if let (Some(parent), Some(name)) = (absolute.parent(), absolute.file_name()) {
        if let Ok(parent) = parent.canonicalize() {
            return parent.join(name);
        }
    }
    absolute
}

fn same_path(a: &str, b: &str) -> bool {
    let a = normalized_path(Path::new(a));
    let b = normalized_path(Path::new(b));
    if cfg!(windows) {
        use std::path::{Component, Prefix};
        let parts = |path: &Path| -> Vec<String> {
            path.components()
                .filter(|part| !matches!(part, Component::CurDir))
                .map(|part| match part {
                    Component::Prefix(prefix) => match prefix.kind() {
                        Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                            (letter as char).to_ascii_lowercase().to_string()
                        }
                        Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => {
                            format!("{}/{}", server.to_string_lossy(), share.to_string_lossy())
                                .to_lowercase()
                        }
                        _ => part.as_os_str().to_string_lossy().to_lowercase(),
                    },
                    _ => part.as_os_str().to_string_lossy().to_lowercase(),
                })
                .collect()
        };
        parts(&a) == parts(&b)
    } else {
        a == b
    }
}
fn confirm(o: &Options, message: &str) -> Result<()> {
    if o.yes || o.dry_run {
        return Ok(());
    }
    if !io::stdin().is_terminal() {
        bail!("{message}; pass --yes to confirm");
    }
    eprint!("{message} [y/N] ");
    io::stderr().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    if !line.trim().eq_ignore_ascii_case("y") {
        bail!("cancelled");
    }
    Ok(())
}
fn settings(v: &str, o: &Options) -> Result<Value> {
    let path = settings_path(&o.scope)?;
    // A full reset must also recover unreadable or malformed preferences.
    if v == "reset" && o.key.is_none() {
        let data = if o.dry_run {
            json!({})
        } else {
            api::reset_json_file(&path)?
        };
        return Ok(json!({"path":path,"settings":data,"dryRun":o.dry_run}));
    }
    let mut data = settings_read(&o.scope)?;
    if v == "show" {
        return Ok(json!({"path":path,"settings":data}));
    }
    let key = o.key.as_deref().unwrap_or("saveDir");
    let key = if key == "save-root" { "saveDir" } else { key };
    if v == "get" {
        return Ok(json!({"key":key,"value":data[key]}));
    }
    if v == "set" {
        let raw = o
            .value
            .as_deref()
            .or(o.value_json.as_deref())
            .or(o.root.as_deref().and_then(Path::to_str))
            .context("--value required")?;
        let value = serde_json::from_str(raw).unwrap_or_else(|_| json!(raw));
        validate_setting(key, &value)?;
        data[key] = value;
        if o.scope == "ui" && key == "appLocale" {
            data["gameTextLocale"] = json!(super::text::default_game_language(
                data[key].as_str().unwrap()
            ));
        }
    } else if o.key.is_some() {
        data.as_object_mut().unwrap().remove(key);
    } else {
        data = json!({});
    }
    if !o.dry_run {
        let proposed = data.clone();
        api::update_json_file(&path, |current| {
            let mut current = if current.as_object().is_some_and(|m| m.is_empty()) {
                settings_read(&o.scope).map_err(|e| gore_save::CoreError::Parse(e.to_string()))?
            } else {
                current
            };
            if !current.is_object() {
                return Err(gore_save::CoreError::Parse(
                    "settings are not an object".into(),
                ));
            }
            if v == "reset" && o.key.is_none() {
                return Ok(json!({}));
            }
            if let Some(value) = proposed.get(key) {
                current[key] = value.clone();
                if o.scope == "ui" && key == "appLocale" {
                    current["gameTextLocale"] = proposed["gameTextLocale"].clone();
                }
            } else {
                current.as_object_mut().unwrap().remove(key);
            }
            Ok(current)
        })?;
    }
    Ok(json!({"path":path,"settings":data,"dryRun":o.dry_run}))
}
fn validate_setting(key: &str, value: &Value) -> Result<()> {
    match key {
        "themeMode" => {
            if !matches!(value.as_str(), Some("light" | "dark" | "system")) {
                bail!("invalid themeMode");
            }
        }
        "uiFontFamily" => {
            if !matches!(value.as_str(), Some("system" | "podkova" | "notoSerif")) {
                bail!("invalid font");
            }
        }
        "uiScale" => {
            if value.as_f64().is_none_or(|v| !(0.5..=2.0).contains(&v)) {
                bail!("uiScale must be 0.5..2.0");
            }
        }
        "appLocale" => {
            if value
                .as_str()
                .is_none_or(|lang| super::presentation::metadata()["ui"][lang].is_null())
            {
                bail!("invalid locale");
            }
        }
        "gameTextLocale" => {
            if value.as_str().is_none_or(|lang| {
                !super::presentation::metadata()["gameTextDefaults"]
                    .as_object()
                    .unwrap()
                    .values()
                    .any(|v| v == lang)
            }) {
                bail!("invalid game text locale");
            }
        }
        "windowWidth" | "windowHeight" => {
            if value.as_f64().is_none_or(|v| v <= 0.0) {
                bail!("window size must be positive");
            }
        }
        "windowMaximized" | "autoUpdateCheck" | "gameDataSourceNoticeShown" | "showObjectIds" => {
            if !value.is_boolean() {
                bail!("setting requires a boolean");
            }
        }
        "saveDir" => {
            if !value.is_string() {
                bail!("saveDir requires a path string");
            }
        }
        "externalSavePaths" | "hiddenOtherSavePaths" => {
            if value
                .as_array()
                .is_none_or(|a| a.iter().any(|v| !v.is_string()))
            {
                bail!("setting requires an array of paths");
            }
        }
        _ => {}
    }
    Ok(())
}
fn library(v: &str, o: &Options) -> Result<Value> {
    if v == "list" {
        return settings_read("editor");
    }
    let input = save(o)?;
    if v == "add" {
        if !input.is_file() {
            bail!("library add requires an existing save file");
        }
        let inspection = call("inspect_save", json!({"path":input}))?;
        if inspection["format"] != "GSAV" {
            bail!("library add requires an inspectable Gothic GSAV save");
        }
    }
    let file = normalized_path(input).to_string_lossy().into_owned();
    let key = if matches!(v, "hide" | "unhide") {
        "hiddenOtherSavePaths"
    } else {
        "externalSavePaths"
    };
    let change = |mut data: Value| -> Result<Value, gore_save::CoreError> {
        if data.as_object().is_some_and(|m| m.is_empty()) {
            data =
                settings_read("editor").map_err(|e| gore_save::CoreError::Parse(e.to_string()))?;
        }
        if !data.is_object() {
            return Err(gore_save::CoreError::Parse("invalid settings".into()));
        }
        let mut rows = data[key].as_array().cloned().unwrap_or_default();
        rows.retain(|p| p.as_str().is_none_or(|p| !same_path(p, &file)));
        if matches!(v, "add" | "hide") {
            rows.push(json!(file));
        }
        data[key] = json!(rows);
        Ok(data)
    };
    if o.dry_run {
        Ok(change(settings_read("editor")?)?)
    } else {
        Ok(api::update_json_file(&settings_path("editor")?, change)?)
    }
}

pub(super) fn stage(file: &Path, payload: &Value, dry_run: bool) -> Result<Value> {
    let path = PathBuf::from(payload["path"].as_str().context("draft has no save path")?)
        .canonicalize()?;
    let hash = api::file_sha1(&path)?;
    if let Some(output) = payload["outputPath"].as_str() {
        api::validate_output_path(&path, Path::new(output))?;
    }
    api::check_edit_persistent_snapshots(
        &path,
        payload["edits"].as_array().context("edits required")?,
    )?;
    if payload["expectedSha1"]
        .as_str()
        .is_some_and(|expected| expected != hash)
    {
        return Err(gore_save::CoreError::Validation(
            "save changed since command inspection".into(),
        )
        .into());
    }
    let change = |mut current: Value| -> std::result::Result<Value, gore_save::CoreError> {
        if current.as_object().is_some_and(|m| m.is_empty()) {
            current = json!({"format":"gore.save.draft.v1","path":path,"expectedSha1":hash,"expectedPersistentSha1":api::file_sha1(&path.parent().unwrap().join("PersistentDataList.sav")).ok(),"edits":[]});
        }
        if current["format"] != "gore.save.draft.v1"
            || current["path"] != json!(path)
            || current["expectedSha1"] != hash
        {
            return Err(gore_save::CoreError::Validation(
                "draft belongs to another save or a changed input".into(),
            ));
        }
        let edits = payload["edits"]
            .as_array()
            .ok_or_else(|| gore_save::CoreError::InvalidRequest("edits required".into()))?;
        let list = current["edits"]
            .as_array_mut()
            .ok_or_else(|| gore_save::CoreError::Parse("invalid draft edits".into()))?;
        for incoming in edits {
            if incoming["path"] == "private.player.setAttribute" {
                let key = gore_save::workflow::replacement_key(incoming);
                let mut merged = list
                    .iter()
                    .find(|e| key.is_some() && gore_save::workflow::replacement_key(e) == key)
                    .cloned()
                    .unwrap_or_else(|| incoming.clone());
                let normalize = |value: &mut Value| {
                    if let Some(shared) = value.get("value").cloned() {
                        value["baseValue"] = shared.clone();
                        value["currentValue"] = shared;
                        value.as_object_mut().unwrap().remove("value");
                    }
                };
                normalize(&mut merged["value"]);
                let mut incoming = incoming.clone();
                normalize(&mut incoming["value"]);
                for field in ["baseValue", "currentValue"] {
                    if let Some(value) = incoming["value"].get(field) {
                        merged["value"][field] = value.clone();
                    }
                }
                list.retain(|e| key.is_none() || gore_save::workflow::replacement_key(e) != key);
                list.push(merged);
            } else if incoming["path"] == "private.story.apply" {
                let changes = incoming["value"]["changes"].as_array().ok_or_else(|| {
                    gore_save::CoreError::InvalidRequest("story changes required".into())
                })?;
                let existing = list.iter().position(|e| e["path"] == "private.story.apply");
                let index = existing.unwrap_or_else(|| {
                    list.push(json!({"path":"private.story.apply","value":{"changes":[]}}));
                    list.len() - 1
                });
                let pending = list[index]["value"]["changes"]
                    .as_array_mut()
                    .ok_or_else(|| {
                        gore_save::CoreError::Parse("invalid pending story changes".into())
                    })?;
                for change in changes {
                    let id = change["id"].as_str().ok_or_else(|| {
                        gore_save::CoreError::InvalidRequest("story id required".into())
                    })?;
                    let mut change = change.clone();
                    if let Some(old) = pending.iter().find(|e| {
                        e["id"]
                            .as_str()
                            .is_some_and(|old| old.eq_ignore_ascii_case(id))
                    }) {
                        if let Some(expected) = old.get("expected") {
                            change["expected"] = expected.clone();
                        }
                    }
                    pending.retain(|e| {
                        e["id"]
                            .as_str()
                            .is_none_or(|old| !old.eq_ignore_ascii_case(id))
                    });
                    pending.push(change);
                }
            } else {
                if let Some(key) = gore_save::workflow::replacement_key(incoming) {
                    list.retain(|existing| {
                        gore_save::workflow::replacement_key(existing).as_ref() != Some(&key)
                    });
                }
                list.push(incoming.clone());
            }
        }
        for key in ["placementNotes", "clearPlacementNotes"] {
            if let Some(entries) = payload[key].as_array() {
                let mut values = current[key].as_array().cloned().unwrap_or_default();
                for entry in entries {
                    let npc = if key == "placementNotes" {
                        entry["npc"].as_str()
                    } else {
                        entry.as_str()
                    };
                    let matches = |value: &Value, records: bool| {
                        let name = if records {
                            value["npc"].as_str()
                        } else {
                            value.as_str()
                        };
                        name.zip(npc)
                            .is_some_and(|(name, npc)| name.eq_ignore_ascii_case(npc))
                    };
                    values.retain(|v| !matches(v, key == "placementNotes"));
                    let opposite = if key == "placementNotes" {
                        "clearPlacementNotes"
                    } else {
                        "placementNotes"
                    };
                    if let Some(rows) = current.get_mut(opposite).and_then(Value::as_array_mut) {
                        rows.retain(|v| !matches(v, opposite == "placementNotes"));
                        if rows.is_empty() {
                            current.as_object_mut().unwrap().remove(opposite);
                        }
                    }
                    values.push(entry.clone());
                }
                current[key] = json!(values);
            }
        }
        if payload["syncPersistentDataList"] == true {
            current["syncPersistentDataList"] = json!(true);
            let profile = path.parent().unwrap().join("PersistentDataList.sav");
            let hash = if profile.exists() {
                Some(api::file_sha1(&profile)?)
            } else {
                None
            };
            if let Some(expected) = current.get("expectedPersistentSha1") {
                if expected != &json!(hash) {
                    return Err(gore_save::CoreError::Validation(
                        "profile changed since draft creation".into(),
                    ));
                }
            } else {
                current["expectedPersistentSha1"] = json!(hash);
            }
        }
        if let Some(output) = payload.get("outputPath") {
            current["outputPath"] = output.clone();
        }
        if let Some(output) = current["outputPath"].as_str() {
            api::validate_output_path(&path, Path::new(output))?;
        }
        api::check_edit_persistent_snapshots(
            &path,
            current["edits"]
                .as_array()
                .ok_or_else(|| gore_save::CoreError::Parse("invalid draft edits".into()))?,
        )?;
        Ok(current)
    };
    let result = if dry_run {
        change(if file.exists() {
            read_json(file)?
        } else {
            json!({})
        })?
    } else {
        api::update_json_file(file, change)?
    };
    Ok(
        json!({"draft":file,"pending":result["edits"].as_array().map(Vec::len),"data":result,"dryRun":dry_run}),
    )
}
fn placement_edit_npc(edit: &Value) -> Option<&str> {
    gore_save::workflow::placement_actor(edit)
}

fn draft(v: &str, o: &Options) -> Result<Value> {
    let file = save(o)?;
    if v == "create" {
        let target = o
            .target
            .as_deref()
            .context("--target is the save file for a new draft")?
            .canonicalize()?;
        if file.exists() {
            bail!("draft already exists");
        }
        return stage(file, &json!({"path":target,"edits":[]}), o.dry_run);
    }
    let mut data = read_json(file)?;
    if data["format"] != "gore.save.draft.v1" {
        bail!("unknown draft format");
    }
    if v == "show" {
        return Ok(data);
    }
    let original = data.clone();
    if matches!(v, "remove" | "reset") {
        if v == "remove" && o.operation.is_none() {
            bail!("--operation required for draft remove");
        }
        if let Some(index) = o.operation {
            let list = data["edits"].as_array_mut().context("invalid draft")?;
            if index >= list.len() {
                bail!("operation out of range");
            }
            let removed = list.remove(index);
            let npc = placement_edit_npc(&removed).map(str::to_owned);
            let has_action = npc.as_deref().is_some_and(|npc| {
                ["placementNotes", "clearPlacementNotes"].iter().any(|key| {
                    data[*key].as_array().is_some_and(|entries| {
                        entries.iter().any(|entry| {
                            let id = if *key == "placementNotes" {
                                entry["npc"].as_str()
                            } else {
                                entry.as_str()
                            };
                            id.is_some_and(|id| id.eq_ignore_ascii_case(npc))
                        })
                    })
                })
            });
            let list = data["edits"].as_array_mut().context("invalid draft")?;
            if has_action {
                list.retain(|edit| {
                    !placement_edit_npc(edit)
                        .zip(npc.as_deref())
                        .is_some_and(|(id, npc)| id.eq_ignore_ascii_case(npc))
                });
            }
            let empty = list.is_empty();
            // A pin/undo/resume combines pose and routine changes with one
            // sidecar. Discard that whole action when any of its edits is removed.
            for key in ["placementNotes", "clearPlacementNotes"] {
                if empty {
                    data.as_object_mut().unwrap().remove(key);
                } else if let Some(entries) = data.get_mut(key).and_then(Value::as_array_mut) {
                    entries.retain(|entry| {
                        let npc = if key == "placementNotes" {
                            entry["npc"].as_str()
                        } else {
                            entry.as_str()
                        };
                        !placement_edit_npc(&removed)
                            .zip(npc)
                            .is_some_and(|(id, npc)| id.eq_ignore_ascii_case(npc))
                    });
                    if entries.is_empty() {
                        data.as_object_mut().unwrap().remove(key);
                    }
                }
            }
            if removed["path"] == "public.m_PlayerSaveName"
                && data["edits"].as_array().is_some_and(|edits| {
                    !edits
                        .iter()
                        .any(|edit| edit["path"] == "public.m_PlayerSaveName")
                })
            {
                data.as_object_mut()
                    .unwrap()
                    .remove("syncPersistentDataList");
            }
        } else {
            data["edits"] = json!([]);
            let save = Path::new(data["path"].as_str().context("draft has no source")?);
            let hash = api::file_sha1(save)?;
            let profile_hash = api::file_sha1(
                &save
                    .parent()
                    .unwrap_or(Path::new("."))
                    .join("PersistentDataList.sav"),
            )
            .ok();
            data["expectedSha1"] = json!(hash);
            data["expectedPersistentSha1"] = json!(profile_hash);
            for key in [
                "placementNotes",
                "clearPlacementNotes",
                "syncPersistentDataList",
            ] {
                data.as_object_mut().unwrap().remove(key);
            }
        }
        if !o.dry_run {
            api::update_json_file(file, |current| {
                if current != original {
                    return Err(gore_save::CoreError::Validation(
                        "draft changed while the command was running".into(),
                    ));
                }
                Ok(data.clone())
            })?;
        }
        return Ok(data);
    }
    if v == "stage" {
        let p = read_json(
            o.payload_file
                .as_deref()
                .context("--payload-file required")?,
        )?;
        return stage(file, &p, o.dry_run);
    }
    data["dryRun"] = json!(o.dry_run || v == "validate");
    let mut result = call("apply_edits", data.clone())?;
    if !o.dry_run && v == "apply" {
        data = draft_after_apply(data, &result)?;
        let publication = api::update_json_file(file, |current| {
            if current != original {
                return Err(gore_save::CoreError::Validation(
                    "draft changed while the command was running".into(),
                ));
            }
            Ok(data.clone())
        });
        if let Err(error) = publication {
            result["draftUpdated"] = json!(false);
            result["draftWarning"] = json!(format!(
                "save result is committed, but draft could not be updated: {error}"
            ));
        } else {
            result["draftUpdated"] = json!(true);
        }
    }
    Ok(result)
}

fn draft_after_apply(mut data: Value, result: &Value) -> Result<Value> {
    let source = PathBuf::from(data["path"].as_str().context("draft has no source")?);
    let committed: Vec<usize> = serde_json::from_value(result["committed"].clone())?;
    let list = data["edits"].as_array().context("invalid draft")?;
    let consumed_sync = committed.iter().any(|index| {
        list.get(*index)
            .is_some_and(|edit| edit["path"] == "public.m_PlayerSaveName")
    });
    data["edits"] = json!(
        list.iter()
            .enumerate()
            .filter(|(i, _)| !committed.contains(i))
            .map(|(_, e)| e)
            .collect::<Vec<_>>()
    );
    if let Some(results) = result["results"].as_array() {
        for write in results {
            api::refresh_edit_persistent_snapshots(
                &source,
                data["edits"].as_array_mut().unwrap(),
                write,
            );
        }
    }
    let has_pending = data["edits"]
        .as_array()
        .is_some_and(|edits| !edits.is_empty());
    if !committed.is_empty() {
        if let Some(hash) = result["sha1"].as_str() {
            data["expectedSha1"] = json!(hash);
        }
        let committed_path = PathBuf::from(
            result["path"]
                .as_str()
                .unwrap_or(data["path"].as_str().unwrap_or("")),
        );
        let committed_path = committed_path.canonicalize().unwrap_or(committed_path);
        let profile = committed_path
            .parent()
            .unwrap_or(Path::new("."))
            .join("PersistentDataList.sav");
        data["path"] = json!(committed_path);
        if consumed_sync || !has_pending || data["syncPersistentDataList"] != true {
            data["expectedPersistentSha1"] = json!(api::file_sha1(&profile).ok());
        }
        data.as_object_mut().unwrap().remove("outputPath");
    }
    data.as_object_mut().unwrap().remove("dryRun");
    if !committed.is_empty() {
        gore_save::workflow::retain_pending_placement_sidecars(&mut data);
        if consumed_sync || !has_pending {
            data.as_object_mut()
                .unwrap()
                .remove("syncPersistentDataList");
        }
    }
    Ok(data)
}

pub(super) fn difficulty(v: &str, o: &Options) -> Result<Value> {
    let context = save_context(o)?;
    let o = &context;
    let id = o.profile.context("--profile required")?;
    if v == "show" {
        let data = scan(o)?;
        return Ok(data["profiles"]
            .as_array()
            .and_then(|rows| rows.iter().find(|r| r["profileId"] == id))
            .context("profile not found")?
            .clone());
    }
    let mut difficulty = json!({});
    for (key, value) in [
        ("preset", &o.preset),
        ("combat", &o.combat),
        ("resources", &o.resources),
        ("progression", &o.progression),
    ] {
        if let Some(value) = value {
            difficulty[key] = json!(value);
        }
    }
    if let Some(value) = o.flow_helper {
        difficulty["flowHelper"] = json!(value)
    }
    if let Some(value) = o.permadeath {
        difficulty["permadeath"] = json!(value)
    }
    let p = json!({"difficulty":difficulty,"profile":{"path":profile_path(o)?,"profileId":id},"backup":true});
    admin_write("write_difficulty", p, o)
}

fn copy_admin_files(
    src: &Path,
    dst: &Path,
    backup: bool,
    remap: &impl Fn(&mut Value),
) -> Result<()> {
    fs::create_dir_all(dst)?;
    if !src.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let to = dst.join(entry.file_name());
        let ty = entry.file_type()?;
        if ty.is_symlink() {
            bail!(
                "cannot simulate transactions through a symlink: {}",
                path.display()
            );
        }
        if ty.is_dir() && (backup || entry.file_name() == "goresave_backups") {
            copy_admin_files(&path, &to, true, remap)?;
        } else if ty.is_file()
            && (backup
                || path.extension().is_some_and(|e| e == "sav")
                || entry.file_name().to_str().is_some_and(|name| {
                    let name = name.to_ascii_lowercase();
                    name.contains(".sav.assign-final-goresave-") || name.contains(".sav.bak.")
                }))
        {
            if path.extension().is_some_and(|e| e == "json") {
                let bytes = fs::read(&path)?;
                if let Ok(mut value) = serde_json::from_slice::<Value>(&bytes) {
                    remap(&mut value);
                    fs::write(to, serde_json::to_vec_pretty(&value)?)?;
                } else {
                    fs::write(to, bytes)?;
                }
            } else {
                fs::copy(path, to)?;
            }
        }
    }
    Ok(())
}

/// Run the actual administrative core transaction against copies, including
/// backups, manifests and placement notes. Validation is identical to a write.
fn admin_write(command: &str, p: Value, o: &Options) -> Result<Value> {
    if !o.dry_run {
        return call(command, p);
    }
    let temporary = tempfile::tempdir()?;
    let absolute = |p: &Path| -> Result<PathBuf> {
        Ok(if p.is_absolute() {
            p.to_owned()
        } else {
            std::env::current_dir()?.join(p)
        })
    };
    let live = absolute(&root(o)?)?;
    let mut mappings = vec![(live, temporary.path().join("saves"))];
    fn visit_request_paths(
        value: &mut Value,
        visit: &mut impl FnMut(&mut String) -> Result<()>,
    ) -> Result<()> {
        match value {
            Value::Object(fields) => {
                for (key, value) in fields {
                    if matches!(
                        key.as_str(),
                        "path" | "persistentPath" | "backupPath" | "destinationPath"
                    ) {
                        if let Value::String(path) = value {
                            visit(path)?;
                            continue;
                        }
                    }
                    visit_request_paths(value, visit)?;
                }
            }
            Value::Array(values) => {
                for value in values {
                    visit_request_paths(value, visit)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    let mut request = p.clone();
    visit_request_paths(&mut request, &mut |raw| {
        let path = absolute(Path::new(raw))?;
        if !mappings.iter().any(|(src, _)| path.starts_with(src)) {
            let parent = path.parent().context("path has no parent")?.to_owned();
            mappings.push((
                parent,
                temporary
                    .path()
                    .join(format!("external-{}", mappings.len())),
            ));
        }
        Ok(())
    })?;
    fn mapped(raw: &str, mappings: &[(PathBuf, PathBuf)]) -> Option<PathBuf> {
        let path = Path::new(raw);
        let path = if path.is_absolute() {
            path.to_owned()
        } else {
            std::env::current_dir().ok()?.join(path)
        };
        mappings
            .iter()
            .filter_map(|(src, dst)| {
                path.strip_prefix(src)
                    .ok()
                    .map(|tail| (src.components().count(), dst.join(tail)))
            })
            .max_by_key(|(n, _)| *n)
            .map(|(_, p)| p)
    }
    fn remap(value: &mut Value, mappings: &[(PathBuf, PathBuf)]) {
        match value {
            Value::String(s) => {
                if let Some(p) = mapped(s, mappings) {
                    *s = p.to_string_lossy().into_owned();
                }
            }
            Value::Array(a) => a.iter_mut().for_each(|v| remap(v, mappings)),
            Value::Object(m) => m.values_mut().for_each(|v| remap(v, mappings)),
            _ => {}
        }
    }
    for (src, dst) in &mappings {
        copy_admin_files(src, dst, false, &|value| remap(value, &mappings))?;
    }
    visit_request_paths(&mut request, &mut |raw| {
        if let Some(path) = mapped(raw, &mappings) {
            *raw = path.to_string_lossy().into_owned();
        }
        Ok(())
    })?;
    let result = call(command, request)?;
    Ok(json!({"dryRun":true,"command":command,"request":p,"validated":true,"simulation":result}))
}

fn selected_recovery<'a>(data: &'a Value, o: &Options) -> Result<&'a Value> {
    data["recoveries"]
        .as_array()
        .context("no recovery")?
        .iter()
        .rev()
        .find(|r| {
            !r.is_null()
                && o.backup
                    .as_ref()
                    .is_none_or(|p| r["backupPath"] == json!(p))
        })
        .context("no matching recovery")
}

fn draft_is_stale(draft: &Value) -> bool {
    let Some(path) = draft["path"].as_str().map(Path::new) else {
        return true;
    };
    if !api::file_sha1(path).is_ok_and(|hash| draft["expectedSha1"] == hash) {
        return true;
    }
    if draft["syncPersistentDataList"].as_bool().unwrap_or(false)
        && api::check_persistent_snapshot(path, draft).is_err()
    {
        return true;
    }
    draft["edits"]
        .as_array()
        .is_none_or(|edits| api::check_edit_persistent_snapshots(path, edits).is_err())
}

pub(super) fn dispatch(g: &str, v: &str, o: &Options) -> Result<Value> {
    match (g, v) {
        ("settings", _) => settings(v, o),
        ("library", _) => library(v, o),
        ("draft", _) => draft(v, o),
        ("", "list" | "refresh") => {
            let mut data = scan(o)?;
            if let Some(rows) = data["saves"].as_array_mut() {
                rows.retain(|r| {
                    if o.other {
                        r["persistentProfileId"].is_null()
                    } else {
                        o.profile.is_none_or(|id| r["persistentProfileId"] == id)
                    }
                });
            }
            if let Some(file) = &o.draft {
                let draft = read_json(file)?;
                let stale = draft_is_stale(&draft);
                data["draft"] = json!({"path":file,"pending":draft["edits"].as_array().map(Vec::len),"stale":stale,"preserved":true});
            }
            display::filter(&mut data, "saves", o);
            display::paginate(&mut data, "saves", o);
            Ok(data)
        }
        ("profiles", _) => {
            let context = save_context(o)?;
            let data = scan(&context)?;
            if v == "show" {
                return difficulty("show", &context);
            }
            Ok(json!({"profiles":data["profiles"],"activeProfileId":data["activeProfileId"]}))
        }
        ("profile", "assign") | ("", "import") => {
            let mut input = save(o)?.to_owned();
            let mut destination = o.clone();
            if v == "import" {
                // The external source is never a hint for the destination.
                destination.save = None;
                destination.target = None;
            } else {
                input = input.canonicalize()?;
                destination.save = Some(input.clone());
                destination = save_context(&destination)?;
            }
            let root = root(&destination)?;
            destination.root = Some(root.clone());
            let id = o.profile.context("--profile required")?;
            let mut p = json!({"path":input,"persistentPath":root.join("PersistentDataList.sav"),"profileId":id,"backup":true});
            if v == "import" {
                let listing = scan(&destination)?;
                let used = listing["saves"]
                    .as_array()
                    .context("save listing unavailable")?;
                let available = |slot: &str| {
                    !root.join(format!("{slot}.sav")).exists()
                        && !used.iter().any(|r| {
                            r["slot"]
                                .as_str()
                                .is_some_and(|s| s.eq_ignore_ascii_case(slot))
                        })
                };
                let stem = input
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_uppercase();
                let conventional = stem
                    .strip_prefix("G1R-")
                    .filter(|s| s.len() == 3)
                    .and_then(|s| s.parse::<u32>().ok())
                    .is_some_and(|n| (1..=999).contains(&n));
                let slot = if conventional && available(&stem) {
                    stem
                } else {
                    (1..=999)
                        .map(|i| format!("G1R-{i:03}"))
                        .find(|s| available(s))
                        .context("no free save slot")?
                };
                p["destinationPath"] = json!(root.join(format!("{slot}.sav")));
            }
            admin_write("assign_save_profile", p, &destination)
        }
        ("profile", "detach") | ("", "delete") => {
            let context = save_context(o)?;
            let o = &context;
            let input = o.save.as_deref();
            let slot = o
                .slot
                .as_deref()
                .or_else(|| input.and_then(|p| p.file_stem()).and_then(|s| s.to_str()))
                .context("--slot or a save required")?;
            let id = o.profile.context("--profile required")?;
            let input = match input {
                Some(path) => path.to_owned(),
                None => root(o)?.join(format!("{slot}.sav")),
            };
            let p = json!({"path":input,"slot":slot,"profileId":id,"persistentPath":profile_path(o)?,"backup":true});
            admin_write(
                if v == "delete" {
                    "delete_save"
                } else {
                    "remove_save_from_profile"
                },
                p,
                o,
            )
        }
        ("backups", "list") => {
            let mut data = call("list_backups", payload(o)?)?;
            if !o.include_companions {
                data.as_object_mut().unwrap().remove("companionBackups");
            }
            Ok(data)
        }
        ("backups", _) => {
            let path = o
                .target
                .as_deref()
                .or(o.save.as_deref())
                .context("save target required")?;
            let backup = o.backup.as_deref().context("--backup required")?;
            if v == "rename" {
                if o.clear_name == o.name.is_some() {
                    bail!("specify exactly one of --name or --clear-name");
                }
                if o.name.as_deref().is_some_and(|name| name.trim().is_empty()) {
                    bail!("--name must not be blank; use --clear-name to remove the label");
                }
            }
            if v == "delete" {
                confirm(o, "Permanently delete this backup")?;
            }
            let p = json!({"path":path,"backupPath":backup,"name":if o.clear_name{""}else{o.name.as_deref().unwrap_or("")}});
            admin_write(
                match v {
                    "rename" => "rename_backup",
                    "delete" => "delete_backup",
                    _ => "restore_backup",
                },
                p,
                o,
            )
        }
        ("recovery", "list") => call("recovery_status", json!({"path":root(o)?})),
        ("recovery", "show") => {
            let data = call("recovery_status", json!({"path":root(o)?}))?;
            Ok(selected_recovery(&data, o)?.clone())
        }
        ("recovery", "repair") => admin_write("scan_save_dir", json!({"path":root(o)?}), o),
        ("recovery", "restore" | "dismiss") => {
            let data = call("recovery_status", json!({"path":root(o)?}))?;
            let recovery = selected_recovery(&data, o)?;
            let p = json!({"path":recovery["targetPath"],"backupPath":recovery["backupPath"],"expectedPersistentSha1":recovery["persistentPostDeleteSha1"],"expectedSaveSha1":recovery["deletedSaveSha1"],"expectedPersistentBackupSha1":recovery["deletedPersistentSha1"]});
            admin_write(
                if v == "restore" {
                    "restore_deleted_save"
                } else {
                    "dismiss_deleted_save_recovery"
                },
                p,
                o,
            )
        }
        ("updates", _) => super::updates::run(v, o),
        _ => bail!("unknown administrative command"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_staleness_checks_only_consumed_profile_snapshots() {
        let temp = tempfile::tempdir().unwrap();
        let save = temp.path().join("G1R-001.sav");
        fs::write(&save, b"source").unwrap();
        let profile = temp.path().join("PersistentDataList.sav");
        fs::write(&profile, b"profile before").unwrap();
        let initial = json!({"path":save,"expectedSha1":api::file_sha1(&save).unwrap(),
            "expectedPersistentSha1":api::file_sha1(&profile).unwrap(),"edits":[]});
        let mut synced = initial.clone();
        synced["syncPersistentDataList"] = json!(true);
        let mut derived = initial.clone();
        derived["edits"] = json!([{"path":"private.inventory.reset","value":{
            "resourcesLevel":"Hard","expectedPersistentSha1":initial["expectedPersistentSha1"]}}]);
        for draft in [&initial, &synced, &derived] {
            assert!(!draft_is_stale(draft));
        }
        fs::write(&profile, b"profile after").unwrap();
        assert!(!draft_is_stale(&initial));
        assert!(draft_is_stale(&synced));
        assert!(draft_is_stale(&derived));

        let external = tempfile::tempdir().unwrap();
        let external_profile = external.path().join("PersistentDataList.sav");
        fs::write(&external_profile, b"independent profile").unwrap();
        derived["edits"][0]["value"]["persistentPath"] = json!(external_profile);
        derived["edits"][0]["value"]["expectedPersistentSha1"] =
            json!(api::file_sha1(&external_profile).unwrap());
        assert!(!draft_is_stale(&derived));
        fs::write(&external_profile, b"external change").unwrap();
        assert!(draft_is_stale(&derived));
        derived["edits"][0]["value"]
            .as_object_mut()
            .unwrap()
            .remove("expectedPersistentSha1");
        assert!(!draft_is_stale(&derived));

        fs::remove_file(&profile).unwrap();
        synced["expectedPersistentSha1"] = Value::Null;
        assert!(!draft_is_stale(&synced));
        fs::write(&profile, b"new profile").unwrap();
        assert!(draft_is_stale(&synced));
        fs::write(&save, b"source after").unwrap();
        assert!(draft_is_stale(&initial));
        fs::remove_file(&save).unwrap();
        assert!(draft_is_stale(&initial));
    }

    #[test]
    fn staging_placement_actions_replaces_opposite_and_same_case_variants_for_one_npc() {
        for key in ["placementNotes", "clearPlacementNotes"] {
            let temp = tempfile::tempdir().unwrap();
            let save = temp.path().join("G1R-001.sav");
            let file = temp.path().join("draft.json");
            fs::write(&save, b"source").unwrap();
            let position = |npc: &str| {
                json!({"path":"private.typed.setValue","value":{
                    "path":["PositionByGlobalId",format!("{{{npc}}}"),"CharacterLocation"],"value":1
                }})
            };
            let action = |key: &str, npc: &str, version: u32| {
                if key == "placementNotes" {
                    json!({"npc":npc,"note":{"version":version}})
                } else {
                    json!(npc)
                }
            };
            let opposite = if key == "placementNotes" {
                "clearPlacementNotes"
            } else {
                "placementNotes"
            };
            let mut initial = json!({"path":save,"edits":[position("NPC-A"),position("NPC-B")]});
            initial[opposite] = json!([action(opposite, "NPC-A", 0), action(opposite, "NPC-B", 0)]);
            stage(&file, &initial, false).unwrap();
            let before = fs::read(&file).unwrap();
            let mut next = json!({"path":save,"edits":[]});
            next[key] = json!([action(key, "npc-a", 1)]);
            let preview = stage(&file, &next, true).unwrap()["data"].clone();
            assert_eq!(fs::read(&file).unwrap(), before);
            assert_eq!(preview[opposite], json!([action(opposite, "NPC-B", 0)]));
            let staged = stage(&file, &next, false).unwrap()["data"].clone();
            assert_eq!(staged, preview);
            next[key] = json!([action(key, "NPC-a", 2)]);
            let replaced = stage(&file, &next, false).unwrap()["data"].clone();
            assert_eq!(replaced[key], next[key]);
            assert_eq!(replaced[opposite], preview[opposite]);
            assert_eq!(replaced["edits"], initial["edits"]);
        }
    }

    #[test]
    fn removing_any_placement_component_discards_the_whole_action_and_keeps_other_edits() {
        for key in ["placementNotes", "clearPlacementNotes"] {
            for operation in 0..3 {
                let temp = tempfile::tempdir().unwrap();
                let save = temp.path().join("G1R-001.sav");
                let file = temp.path().join("draft.json");
                fs::write(&save, b"source").unwrap();
                let change = |map: &str, npc: &str, member: &str| {
                    json!({"path":"private.typed.setValue","value":{
                        "path":[map,format!("{{{npc}}}"),member],"value":1
                    }})
                };
                let attribute = change("_AttributeSet", "NPC-A", "Health");
                let other_npc = change("PositionByGlobalId", "NPC-B", "CharacterLocation");
                let mut payload = json!({"path":save,"edits":[
                    change("PositionByGlobalId", "NPC-A", "CharacterLocation"),
                    change("PositionByGlobalId", "NPC-A", "CharacterRotation"),
                    change("DailyRoutineByGlobalId", "NPC-A", "DailyRoutineClass"),
                    attribute, other_npc
                ]});
                payload[key] = if key == "placementNotes" {
                    json!([{"npc":"npc-a","note":{}},{"npc":"NPC-B","note":{}}])
                } else {
                    json!(["npc-a", "NPC-B"])
                };
                stage(&file, &payload, false).unwrap();
                let before = fs::read(&file).unwrap();
                let mut options = Options {
                    save: Some(file.clone()),
                    operation: Some(operation),
                    dry_run: true,
                    ..Default::default()
                };
                let simulated = draft("remove", &options).unwrap();
                assert_eq!(fs::read(&file).unwrap(), before);
                options.dry_run = false;
                let removed = draft("remove", &options).unwrap();
                assert_eq!(removed, simulated);
                assert_eq!(removed["edits"], json!([attribute, other_npc]));
                assert_eq!(removed[key], json!([payload[key][1]]));
            }
        }
    }

    #[test]
    fn removing_draft_operations_discards_their_placement_actions_only() {
        for key in ["placementNotes", "clearPlacementNotes"] {
            let temp = tempfile::tempdir().unwrap();
            let save = temp.path().join("G1R-001.sav");
            let file = temp.path().join("draft.json");
            fs::write(&save, b"source").unwrap();
            let action = |npc: &str| {
                if key == "placementNotes" {
                    json!({"npc":npc,"note":{
                        "original_location":[1.0,2.0,3.0],
                        "written_location":[4.0,5.0,6.0]
                    }})
                } else {
                    json!(npc)
                }
            };
            let position = |npc: &str| {
                json!({"path":"private.typed.setValue","value":{
                    "path":["PositionByGlobalId",format!("{{{npc}}}"),"CharacterLocation"],
                    "value":{"x":4.0,"y":5.0,"z":6.0}
                }})
            };
            let rename = json!({"path":"public.m_PlayerSaveName","value":"Unrelated"});
            let mut payload =
                json!({"path":save,"edits":[position("NPC-A"),position("NPC-B"),rename]});
            payload[key] = json!([action("npc-a"), action("NPC-B")]);
            stage(&file, &payload, false).unwrap();

            for (map, member) in [
                ("_AttributeSet", "Health"),
                ("_Inventory", "InventoryItems"),
                ("PositionByGlobalId", "SpawnLocation"),
            ] {
                let unrelated = json!({"path":"private.typed.setValue","value":{
                    "path":["CharacterState",map,"{NPC-A}",member],"value":1
                }});
                stage(&file, &json!({"path":save,"edits":[unrelated]}), false).unwrap();
                let removed = draft(
                    "remove",
                    &Options {
                        save: Some(file.clone()),
                        operation: Some(3),
                        ..Default::default()
                    },
                )
                .unwrap();
                assert_eq!(
                    removed[key], payload[key],
                    "unrelated changes to the same NPC keep its undo"
                );
            }

            let mut options = Options {
                save: Some(file.clone()),
                operation: Some(2),
                ..Default::default()
            };
            let unchanged = draft("remove", &options).unwrap();
            assert_eq!(
                unchanged[key], payload[key],
                "an unrelated removal keeps NPC actions"
            );

            options.operation = Some(0);
            let removed = draft("remove", &options).unwrap();
            assert_eq!(removed[key], json!([action("NPC-B")]));
            stage(&file, &json!({"path":save,"edits":[rename]}), false).unwrap();
            let before = fs::read(&file).unwrap();
            options.dry_run = true;
            assert!(draft("remove", &options).unwrap().get(key).is_none());
            assert_eq!(fs::read(&file).unwrap(), before);
            options.dry_run = false;
            assert!(draft("remove", &options).unwrap().get(key).is_none());

            stage(&file, &json!({"path":save,"edits":[rename]}), false).unwrap();
            assert!(
                read_json(&file).unwrap().get(key).is_none(),
                "later edits must not revive removed sidecars"
            );
        }
    }

    #[test]
    fn simulated_repair_copies_and_retires_assignment_claims_without_touching_the_source() {
        let source = tempfile::tempdir().unwrap();
        let simulation = tempfile::tempdir().unwrap();
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../gore-save/assets/start_saves/resources_gothic.sav");
        fs::copy(fixture, source.path().join("G1R-001.sav")).unwrap();
        let claim = "G1R-001.sav.assign-final-goresave-1-2-3";
        fs::write(source.path().join(claim), b"an older assignment").unwrap();
        let live_before = fs::read(source.path().join("G1R-001.sav")).unwrap();

        copy_admin_files(source.path(), simulation.path(), false, &|_| {}).unwrap();
        assert_eq!(
            fs::read(simulation.path().join(claim)).unwrap(),
            b"an older assignment"
        );
        call("scan_save_dir", json!({"path":simulation.path()})).unwrap();
        assert!(!simulation.path().join(claim).exists());
        assert_eq!(
            fs::read(simulation.path().join("G1R-001.sav")).unwrap(),
            live_before
        );
        assert_eq!(
            fs::read(source.path().join(claim)).unwrap(),
            b"an older assignment"
        );
        assert_eq!(
            fs::read(source.path().join("G1R-001.sav")).unwrap(),
            live_before
        );
    }

    #[test]
    fn draft_registry_merges_story_and_partial_attributes_and_guards_profile_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let save = temp.path().join("G1R-001.sav");
        let draft = temp.path().join("draft.json");
        fs::write(&save, b"source").unwrap();
        stage(&draft, &json!({"path":save,"edits":[]}), false).unwrap();
        let story = |id: &str, value: i32| json!({"path":save,"edits":[{"path":"private.story.apply","value":{"changes":[{"id":id,"rawValue":value,"present":true,"expected":{"stored":false}}]}}]});
        stage(&draft, &story("First", 1), false).unwrap();
        stage(&draft, &story("Second", 2), false).unwrap();
        stage(&draft, &story("FIRST", 3), false).unwrap();
        for value in [
            json!({"id":"Health","baseValue":10}),
            json!({"id":"Health","currentValue":20}),
        ] {
            stage(&draft,&json!({"path":save,"edits":[{"path":"private.player.setAttribute","value":value}]}),false).unwrap();
        }
        let data = read_json(&draft).unwrap();
        let edits = data["edits"].as_array().unwrap();
        assert_eq!(edits.len(), 2);
        assert_eq!(edits[0]["value"]["changes"].as_array().unwrap().len(), 2);
        assert_eq!(edits[1]["value"]["baseValue"], 10);
        assert_eq!(edits[1]["value"]["currentValue"], 20);
        let before = fs::read(&draft).unwrap();
        stage(&draft, &story("Third", 4), true).unwrap();
        assert_eq!(before, fs::read(&draft).unwrap());
        fs::write(temp.path().join("PersistentDataList.sav"), b"new profile").unwrap();
        assert!(
            stage(
                &draft,
                &json!({"path":save,"edits":[],"syncPersistentDataList":true}),
                false
            )
            .is_err()
        );
        assert_eq!(before, fs::read(&draft).unwrap());
        assert_eq!(fs::read(save).unwrap(), b"source");
    }
    #[test]
    fn removing_an_unrelated_draft_edit_preserves_an_independent_profile_guard() {
        let temp = tempfile::tempdir().unwrap();
        let save = temp.path().join("G1R-001.sav");
        let file = temp.path().join("draft.json");
        let profile = temp.path().join("PersistentDataList.sav");
        let bytes = include_bytes!("../../../../gore-save/assets/start_saves/resources_gothic.sav");
        fs::write(&save, bytes).unwrap();
        fs::write(&profile, b"profile snapshot").unwrap();
        let attribute =
            json!({"path":"private.player.setAttribute","value":{"id":"Health","baseValue":300}});
        let other =
            json!({"path":"private.player.setAttribute","value":{"id":"Mana","currentValue":20}});
        stage(
            &file,
            &json!({"path":save,"edits":[attribute,other],"syncPersistentDataList":true}),
            false,
        )
        .unwrap();
        let before = fs::read(&file).unwrap();
        let mut options = Options {
            save: Some(file.clone()),
            operation: Some(1),
            dry_run: true,
            ..Default::default()
        };
        let simulated = draft("remove", &options).unwrap();
        assert_eq!(simulated["syncPersistentDataList"], true);
        assert_eq!(fs::read(&file).unwrap(), before);
        options.dry_run = false;
        let removed = draft("remove", &options).unwrap();
        assert_eq!(removed["syncPersistentDataList"], true);
        assert_eq!(
            removed["expectedPersistentSha1"],
            simulated["expectedPersistentSha1"]
        );
        assert_eq!(removed["edits"], json!([attribute]));
        draft("validate", &options).unwrap();
        fs::write(&profile, b"changed profile difficulty").unwrap();
        assert!(
            draft("validate", &options)
                .unwrap_err()
                .to_string()
                .contains("profile changed since draft creation")
        );
        assert_eq!(fs::read(&save).unwrap(), bytes);
    }

    #[test]
    fn partial_draft_apply_preserves_an_independent_profile_snapshot_for_retry() {
        let temp = tempfile::tempdir().unwrap();
        let save = temp.path().join("G1R-001.sav");
        let file = temp.path().join("draft.json");
        let profile = temp.path().join("PersistentDataList.sav");
        let bytes = include_bytes!("../../../../gore-save/assets/start_saves/resources_gothic.sav");
        fs::write(&save, bytes).unwrap();
        fs::write(&profile, b"initial profile").unwrap();
        let reset = json!({"path":"private.inventory.reset","value":{}});
        let story = json!({"path":"private.story.apply","value":{"changes":[{
            "id":"CLI_Profile_Guard_Test","present":true,"rawValue":1,
            "expected":{"stored":false},"allowUnknownCreate":true
        }]}});
        stage(
            &file,
            &json!({"path":save,"edits":[reset,story],"syncPersistentDataList":true}),
            false,
        )
        .unwrap();
        let original = read_json(&file).unwrap();
        let result = gore_save::workflow::apply_with_progress(&original, |progress| {
            assert_eq!(progress["committed"], json!([0]));
            // A concurrent writer invalidates the second group and changes the
            // profile before the remaining draft is published.
            api::execute(&api::Request {
                command: "write_save".into(),
                payload: json!({"path":save,"backup":false,
                    "edits":[{"path":"public.m_PlayerSaveName","value":"Concurrent save change"}]}),
            })
            .unwrap();
            fs::write(&profile, b"changed profile difficulty").unwrap();
        })
        .unwrap();
        assert_eq!(result["complete"], false);
        assert_eq!(result["committed"], json!([0]));
        let remaining = draft_after_apply(original.clone(), &result).unwrap();
        assert_eq!(remaining["edits"], json!([story]));
        assert_eq!(remaining["syncPersistentDataList"], true);
        assert_eq!(
            remaining["expectedPersistentSha1"],
            original["expectedPersistentSha1"]
        );
        api::update_json_file(&file, |_| Ok(remaining)).unwrap();
        let before_retry = api::file_sha1(&save).unwrap();
        let options = Options {
            save: Some(file),
            ..Default::default()
        };
        assert!(
            draft("validate", &options)
                .unwrap_err()
                .to_string()
                .contains("profile changed since draft creation")
        );
        assert!(
            draft("apply", &options)
                .unwrap_err()
                .to_string()
                .contains("profile changed since draft creation")
        );
        assert_eq!(api::file_sha1(&save).unwrap(), before_retry);
        assert_eq!(fs::read(&profile).unwrap(), b"changed profile difficulty");
    }

    #[test]
    fn completed_guarded_apply_refreshes_the_profile_snapshot_for_new_draft_edits() {
        let temp = tempfile::tempdir().unwrap();
        let save = temp.path().join("G1R-001.sav");
        let file = temp.path().join("draft.json");
        let profile = temp.path().join("PersistentDataList.sav");
        fs::write(
            &save,
            include_bytes!("../../../../gore-save/assets/start_saves/resources_gothic.sav"),
        )
        .unwrap();
        fs::write(&profile, b"initial profile").unwrap();
        stage(
            &file,
            &json!({"path":save,"syncPersistentDataList":true,"edits":[{
                "path":"private.player.setAttribute","value":{"id":"Health","baseValue":300}
            }]}),
            false,
        )
        .unwrap();
        let original = read_json(&file).unwrap();
        let result = gore_save::workflow::apply_with_progress(&original, |_| {
            fs::write(&profile, b"profile changed after the final write").unwrap();
        })
        .unwrap();
        assert_eq!(result["complete"], true);
        assert_eq!(result["committed"], json!([0]));
        let completed = draft_after_apply(original.clone(), &result).unwrap();
        assert_eq!(completed["edits"], json!([]));
        assert!(completed.get("syncPersistentDataList").is_none());
        assert_ne!(
            completed["expectedPersistentSha1"],
            original["expectedPersistentSha1"]
        );
        assert_eq!(
            completed["expectedPersistentSha1"],
            api::file_sha1(&profile).unwrap()
        );
        api::update_json_file(&file, |_| Ok(completed)).unwrap();
        let staged = stage(
            &file,
            &json!({"path":save,"syncPersistentDataList":true,
                "edits":[{"path":"public.m_PlayerSaveName","value":"Next draft name"}]
            }),
            false,
        )
        .unwrap();
        assert_eq!(staged["pending"], 1);
        assert_eq!(staged["data"]["syncPersistentDataList"], true);
    }

    #[test]
    fn resource_presets_dominate_sublevels_and_unknown_settings_remain_unknown() {
        assert_eq!(resolved_resources(&json!({"preset":"DifficultyPreset_Hard","resources":"ResourcesDifficultySettings_Standard"})).unwrap(),"Hard");
        assert_eq!(resolved_resources(&json!({"preset":"DifficultyPreset_Custom","resources":"ResourcesDifficultySettings_Easy"})).unwrap(),"Novice");
        assert_eq!(resolved_resources(&Value::Null).unwrap(), "Gothic");
        assert!(resolved_resources(&json!({"preset":"DifficultyPreset_Future","resources":"ResourcesDifficultySettings_Standard"})).is_err());
    }
}
#[test]
fn partial_apply_keeps_the_reset_guard_from_the_committed_profile_write() {
    let temp = tempfile::tempdir().unwrap();
    let save = temp.path().join("G1R-001.sav");
    fs::write(&save, b"committed save").unwrap();
    let profile = temp.path().join("PersistentDataList.sav");
    fs::write(&profile, b"concurrent profile change").unwrap();
    let before = json!({"path":save,"syncPersistentDataList":true,"edits":[
        {"path":"public.m_PlayerSaveName","value":"Committed rename"},
        {"path":"private.inventory.reset","value":{"resourcesLevel":"Hard","persistentPath":profile,"expectedPersistentSha1":"original-hash"}}
    ]});
    let result = json!({"complete":false,"path":save,"sha1":api::file_sha1(&save).unwrap(),"committed":[0],"remaining":[1],"results":[{"persistentPath":profile,"persistentOriginalSha1":"original-hash","persistentWrittenSha1":"self-written-hash"}]});
    let remaining = draft_after_apply(before, &result).unwrap();
    assert_eq!(
        remaining["edits"][0]["value"]["expectedPersistentSha1"],
        "self-written-hash"
    );
    assert!(
        api::check_edit_persistent_snapshots(&save, remaining["edits"].as_array().unwrap())
            .is_err()
    );
    assert_eq!(fs::read(profile).unwrap(), b"concurrent profile change");
}
