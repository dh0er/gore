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
pub(super) fn resources_level(o: &Options) -> Result<String> {
    if let Some(level) = &o.resources_level {
        return normalize_resources(level);
    }
    let listing = scan(o)?;
    let selected = o.save.as_deref().and_then(|p| p.canonicalize().ok());
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
    let attached = profiles.and_then(|rows| {
        rows.iter()
            .find(|r| id.is_some() && r["profileId"].as_i64() == id)
    });
    if let Some(profile) = attached.filter(|p| has_difficulty(&p["difficulty"])) {
        return resolved_resources(&profile["difficulty"]);
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
    resolved_resources(
        &active
            .map(|p| p["difficulty"].clone())
            .unwrap_or(Value::Null),
    )
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
fn same_path(a: &str, b: &str) -> bool {
    let a = Path::new(a)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(a));
    let b = Path::new(b)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(b));
    if cfg!(windows) {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
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
            if !matches!(
                value.as_str(),
                Some("en" | "de" | "fr" | "it" | "es" | "pl" | "ru" | "ja" | "zh-Hans" | "pt-BR")
            ) {
                bail!("invalid locale");
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
    let file = input
        .canonicalize()
        .unwrap_or_else(|_| input.to_owned())
        .to_string_lossy()
        .into_owned();
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
                    values.retain(|v| {
                        if key == "placementNotes" {
                            v["npc"] != entry["npc"]
                        } else {
                            v != entry
                        }
                    });
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
        if let Some(index) = o.operation {
            let list = data["edits"].as_array_mut().context("invalid draft")?;
            if index >= list.len() {
                bail!("operation out of range");
            }
            list.remove(index);
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
        let committed: Vec<usize> = serde_json::from_value(result["committed"].clone())?;
        let list = data["edits"].as_array().context("invalid draft")?;
        data["edits"] = json!(
            list.iter()
                .enumerate()
                .filter(|(i, _)| !committed.contains(i))
                .map(|(_, e)| e)
                .collect::<Vec<_>>()
        );
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
            data["expectedPersistentSha1"] = json!(api::file_sha1(&profile).ok());
            data.as_object_mut().unwrap().remove("outputPath");
        }
        data.as_object_mut().unwrap().remove("dryRun");
        if !committed.is_empty() {
            for key in [
                "placementNotes",
                "clearPlacementNotes",
                "syncPersistentDataList",
            ] {
                data.as_object_mut().unwrap().remove(key);
            }
        }
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

pub(super) fn difficulty(v: &str, o: &Options) -> Result<Value> {
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
    for key in ["path", "persistentPath", "backupPath", "destinationPath"] {
        if let Some(raw) = p[key].as_str() {
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
        }
    }
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
    fn copy(src: &Path, dst: &Path, backup: bool, mappings: &[(PathBuf, PathBuf)]) -> Result<()> {
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
                copy(&path, &to, true, mappings)?;
            } else if ty.is_file() && (backup || path.extension().is_some_and(|e| e == "sav")) {
                if path.extension().is_some_and(|e| e == "json") {
                    let bytes = fs::read(&path)?;
                    if let Ok(mut value) = serde_json::from_slice::<Value>(&bytes) {
                        remap(&mut value, mappings);
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
    for (src, dst) in &mappings {
        copy(src, dst, false, &mappings)?;
    }
    let mut request = p.clone();
    remap(&mut request, &mappings);
    let result = call(command, request)?;
    Ok(json!({"dryRun":true,"command":command,"request":p,"validated":true,"simulation":result}))
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
                let stale = draft["path"]
                    .as_str()
                    .map(Path::new)
                    .and_then(|p| api::file_sha1(p).ok())
                    .is_none_or(|h| draft["expectedSha1"] != h);
                data["draft"] = json!({"path":file,"pending":draft["edits"].as_array().map(Vec::len),"stale":stale,"preserved":true});
            }
            display::filter(&mut data, "saves", o);
            display::paginate(&mut data, "saves", o);
            Ok(data)
        }
        ("profiles", _) => {
            let data = scan(o)?;
            if v == "show" {
                return difficulty("show", o);
            }
            Ok(json!({"profiles":data["profiles"],"activeProfileId":data["activeProfileId"]}))
        }
        ("profile", "assign") | ("", "import") => {
            let input = save(o)?;
            let mut destination = o.clone();
            if v == "import" {
                // The external source is never a hint for the destination.
                destination.save = None;
                destination.target = None;
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
            let input = o.save.as_deref();
            let slot = o
                .slot
                .as_deref()
                .or_else(|| input.and_then(|p| p.file_stem()).and_then(|s| s.to_str()))
                .context("--slot or a save required")?;
            let id = o.profile.context("--profile required")?;
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
        ("backups", "list") => call("list_backups", payload(o)?),
        ("backups", _) => {
            let path = o
                .target
                .as_deref()
                .or(o.save.as_deref())
                .context("save target required")?;
            let backup = o.backup.as_deref().context("--backup required")?;
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
        ("recovery", "list" | "show") => call("recovery_status", json!({"path":root(o)?})),
        ("recovery", "repair") => admin_write("scan_save_dir", json!({"path":root(o)?}), o),
        ("recovery", "restore" | "dismiss") => {
            let data = call("recovery_status", json!({"path":root(o)?}))?;
            let rows = data["recoveries"].as_array().context("no recovery")?;
            let recovery = rows
                .iter()
                .rev()
                .find(|r| {
                    !r.is_null()
                        && o.backup
                            .as_ref()
                            .is_none_or(|p| r["backupPath"] == json!(p))
                })
                .context("no matching recovery")?;
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
    fn resource_presets_dominate_sublevels_and_unknown_settings_remain_unknown() {
        assert_eq!(resolved_resources(&json!({"preset":"DifficultyPreset_Hard","resources":"ResourcesDifficultySettings_Standard"})).unwrap(),"Hard");
        assert_eq!(resolved_resources(&json!({"preset":"DifficultyPreset_Custom","resources":"ResourcesDifficultySettings_Easy"})).unwrap(),"Novice");
        assert_eq!(resolved_resources(&Value::Null).unwrap(), "Gothic");
        assert!(resolved_resources(&json!({"preset":"DifficultyPreset_Future","resources":"ResourcesDifficultySettings_Standard"})).is_err());
    }
}
