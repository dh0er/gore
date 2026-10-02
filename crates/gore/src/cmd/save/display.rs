use super::*;
use base64::Engine;
use std::process::{Command, Stdio};

pub(super) fn print(data: &Value, o: &Options) -> Result<()> {
    let mut data = data.clone();
    if let Err(error) = localize(&mut data, o) {
        data["displayWarning"] = json!(error.to_string());
    }
    let text = serde_json::to_string_pretty(&data)?;
    println!("{text}");
    if o.copy && !o.dry_run {
        if let Err(error) = clipboard(&text) {
            eprintln!("Clipboard copy failed: {error}");
        }
    }
    Ok(())
}
pub(super) fn clipboard(text: &str) -> Result<()> {
    #[cfg(windows)]
    let mut command = Command::new("clip.exe");
    #[cfg(target_os = "macos")]
    let mut command = Command::new("pbcopy");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut c = Command::new("xclip");
        c.args(["-selection", "clipboard"]);
        c
    };
    let mut child = command
        .stdin(Stdio::piped())
        .spawn()
        .context("clipboard utility unavailable")?;
    child.stdin.take().unwrap().write_all(text.as_bytes())?;
    if !child.wait()?.success() {
        bail!("clipboard copy failed");
    }
    Ok(())
}
pub(super) fn open(path: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        // Explorer hands off to the shell; its exit code is not a launch result.
        Command::new("explorer.exe").arg(path).spawn()?;
    }
    #[cfg(not(windows))]
    {
        #[cfg(target_os = "macos")]
        let status = Command::new("open").arg(path).status()?;
        #[cfg(all(unix, not(target_os = "macos")))]
        let status = Command::new("xdg-open").arg(path).status()?;
        if !status.success() {
            bail!("could not open {}", path.display());
        }
    }
    Ok(())
}
pub(super) fn catalog(domain: &str) -> Result<Value> {
    macro_rules! asset {
        ($file:literal) => {
            include_str!(concat!(
                "../../../../../apps/save-editor/assets/",
                $file,
                ".json"
            ))
        };
    }
    let text = match domain {
        "item" | "items" => asset!("item_catalog"),
        "item-stats" => asset!("item_stats"),
        "npc" | "characters" => asset!("npc_catalog"),
        "knowledge" => asset!("knowledge_catalog"),
        "lock" | "locks" => asset!("lock_catalog"),
        "glossary" => asset!("glossary_npc_catalog"),
        "glossary-text" => asset!("glossary_segment_text_catalog"),
        "portraits" => asset!("glossary_images"),
        "location" | "locations" => asset!("location_catalog"),
        "story-semantics" => return Ok(super::presentation::metadata()["story"].clone()),
        "hero-attributes" => {
            return Ok(
                json!({"groups":super::presentation::metadata()["attributeGroups"],"hidden":super::presentation::metadata()["hiddenAttributes"]}),
            );
        }
        "ui-texts" => return Ok(super::presentation::metadata()["ui"].clone()),
        _ => bail!("unknown catalog domain {domain}"),
    };
    Ok(serde_json::from_str(text)?)
}
pub(super) fn item_path(id: &str) -> Result<String> {
    let entries = catalog("items")?;
    let row = entries
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["id"].as_str().is_some_and(|v| v.eq_ignore_ascii_case(id))
                || r["path"]
                    .as_str()
                    .is_some_and(|v| v.eq_ignore_ascii_case(id))
        })
        .context("item is not in the bundled catalog")?;
    Ok(row["path"].as_str().unwrap().into())
}
pub(super) fn existing_item_path(id: &str) -> Result<String> {
    if id.starts_with('/') && !id.contains(['\0', '\n', '\r']) {
        return Ok(id.into());
    }
    item_path(id)
}
pub(super) fn filter(data: &mut Value, key: &str, o: &Options) {
    let Some(rows) = data[key].as_array_mut() else {
        return;
    };
    rows.retain(|r| {
        o.query
            .as_ref()
            .is_none_or(|q| r.to_string().to_lowercase().contains(&q.to_lowercase()))
            && o.id.as_ref().is_none_or(|id| {
                r.as_object().is_some_and(|r| {
                    r.values()
                        .any(|v| v.as_str().is_some_and(|v| v.eq_ignore_ascii_case(id)))
                })
            })
            && o.category.as_ref().is_none_or(|c| {
                r["category"]
                    .as_str()
                    .is_some_and(|v| v.eq_ignore_ascii_case(c))
                    || r["a"].as_str() == Some(c)
            })
            && o.role.as_ref().is_none_or(|role| match role.as_str() {
                "teacher" => r["teacher"] == true,
                "trader" => r["isTrader"] == true,
                _ => r["roles"]
                    .as_array()
                    .is_some_and(|roles| roles.contains(&json!(role))),
            })
            && (key != "characters"
                || o.kind
                    .as_ref()
                    .is_none_or(|kind| r["category"].as_str() == Some(kind)))
            && (key != "locks"
                || o.kind.as_ref().is_none_or(|kind| {
                    r["k"]
                        .as_str()
                        .is_some_and(|k| k.eq_ignore_ascii_case(kind))
                }))
            && o.state.as_ref().is_none_or(|state| match state.as_str() {
                "dead" => r["isDead"] == true,
                "alive" => r["isDead"] == false,
                "locked" => r["unlocked"] == false,
                "unlocked" => r["unlocked"] == true,
                _ => true,
            })
    });
}
pub(super) fn paginate(data: &mut Value, key: &str, o: &Options) {
    if let Some(rows) = data[key].as_array_mut() {
        let total = rows.len();
        *rows = rows
            .iter()
            .skip(o.offset)
            .take(if o.all { usize::MAX } else { o.limit })
            .cloned()
            .collect();
        let count = rows.len();
        data["total"] = json!(total);
        data["count"] = json!(count);
        data["offset"] = json!(o.offset);
    }
}
pub(super) fn find_row<'a>(data: &'a Value, id: &str) -> Result<&'a Value> {
    fn matches(row: &Value, id: &str) -> bool {
        [
            "id",
            "questClass",
            "documentClass",
            "globalId",
            "uniqueName",
        ]
        .iter()
        .any(|key| {
            row[*key]
                .as_str()
                .is_some_and(|v| v.eq_ignore_ascii_case(id))
        })
    }
    fn find<'a>(data: &'a Value, id: &str) -> Option<&'a Value> {
        if matches(data, id) {
            return Some(data);
        }
        match data {
            Value::Array(rows) => rows.iter().find_map(|r| find(r, id)),
            Value::Object(map) => map.values().find_map(|r| find(r, id)),
            _ => None,
        }
    }
    find(data, id).context("entry was not found")
}
pub(super) fn localize(data: &mut Value, o: &Options) -> Result<()> {
    super::text::Texts::load_options(o)?.apply(data)
}

fn loc_payload(o: &Options) -> Result<Value> {
    let mut p = payload(o)?;
    if p["lcache"].is_null() {
        if let Some(hint) = o.game.clone().or_else(|| {
            gore_loc::config::load()
                .game_path
                .filter(|s| !s.trim().is_empty())
                .map(PathBuf::from)
        }) {
            p["lcache"] = json!(hint);
        }
    }
    Ok(p)
}

pub(super) fn dispatch(g: &str, v: &str, o: &Options) -> Result<Value> {
    match (g, v) {
        ("catalog" | "items" | "locations", _) => {
            let domain = if g == "catalog" {
                o.kind.as_deref().or(o.source.as_deref()).unwrap_or("items")
            } else {
                g
            };
            let mut data = catalog(domain)?;
            if domain == "ui-texts" {
                data = json!({"entries":data[&o.lang].as_object().context("unknown language")?.iter().map(|(key,text)|json!({"id":key,"text":text})).collect::<Vec<_>>()});
            }
            if matches!(domain, "item" | "items") {
                let stats = catalog("item-stats")?;
                for row in data.as_array_mut().unwrap() {
                    let id = row["id"].as_str().unwrap_or("").to_string();
                    row["stats"] = stats["items"][&id].clone();
                }
            }
            let key = if data.is_array() {
                "entries"
            } else if matches!(domain, "location" | "locations") {
                "spots"
            } else if matches!(domain, "lock" | "locks") {
                "locks"
            } else {
                "entries"
            };
            if data.is_array() {
                data = json!({key:data})
            }
            let mut options = o.clone();
            if options.id.is_none() {
                options.id = options.item.clone().or(options.location.clone());
            }
            filter(&mut data, key, &options);
            paginate(&mut data, key, o);
            localize(&mut data, o)?;
            Ok(data)
        }
        ("localization", "status") => call("loc_status", json!({})),
        ("localization", "find") => {
            if o.query.is_none() {
                return call("loc_find", loc_payload(o)?);
            }
            let catalog = read_json(&gore_loc::paths::loc_catalog_path())
                .context("prepare the localization catalog first")?;
            let query = o.query.as_deref().unwrap().to_lowercase();
            let mut rows = catalog
                .as_object()
                .context("invalid localization catalog")?
                .iter()
                .filter(|(id, texts)| {
                    id.contains(&query) || texts.to_string().to_lowercase().contains(&query)
                })
                .map(|(id, texts)| json!({"id":id,"languages":texts}))
                .collect::<Vec<_>>();
            rows.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
            let mut data = json!({"entries":rows});
            paginate(&mut data, "entries", o);
            Ok(data)
        }
        ("localization", "prepare") => {
            let p = loc_payload(o)?;
            if o.dry_run {
                let source =
                    gore_loc::loc_store::resolve_lcache(p["lcache"].as_str().map(Path::new))
                        .context("localization source was not found")?;
                let bytes = fs::read(&source)?;
                let hash = gore_loc::loc_store::sha256_hex(&bytes);
                let cache = gore_loc::loc::Lcache::decode(&bytes)?;
                let catalog = cache.export(false);
                if gore_loc::loc_store::sha256_hex(&fs::read(&source)?) != hash {
                    bail!("localization source changed during validation");
                }
                return Ok(
                    json!({"dryRun":true,"validated":true,"sourcePath":source,"idCount":catalog.len(),"sourceSha256":hash}),
                );
            }
            call("loc_extract", p)
        }
        ("assets", _) => assets(v, o),
        ("screenshot", "export") => {
            let data = call("inspect_save", payload(o)?)?;
            let screenshot = data["screenshot"]
                .as_object()
                .context("save has no screenshot")?;
            let bytes = base64::engine::general_purpose::STANDARD.decode(
                screenshot
                    .get("bytesBase64")
                    .and_then(Value::as_str)
                    .context("screenshot has no bytes")?,
            )?;
            let out = o.out.as_deref().context("--out required")?;
            if !o.dry_run {
                fs::write(out, &bytes)?;
                if o.open {
                    open(out)?
                }
            }
            Ok(
                json!({"path":out,"byteLength":bytes.len(),"mimeType":screenshot.get("mimeType"),"dryRun":o.dry_run}),
            )
        }
        ("", "about") => Ok(
            json!({"product":"gore","version":env!("CARGO_PKG_VERSION"),"saveCore":"gore-save", "repository":"https://github.com/dh0er/gore","license":"MIT","capabilities":api::capabilities()}),
        ),
        ("", "licenses") => Ok(
            json!({"project":include_str!("../../../../../LICENSE"),"fonts":{
            "Podkova":include_str!("../../../../../apps/save-editor/assets/licenses/Podkova-OFL.txt"),
            "NotoSerif":include_str!("../../../../../apps/save-editor/assets/licenses/NotoSerif-OFL.txt"),
            "NotoSerifJP":include_str!("../../../../../apps/save-editor/assets/licenses/NotoSerifJP-OFL.txt"),
            "NotoSerifSC":include_str!("../../../../../apps/save-editor/assets/licenses/NotoSerifSC-OFL.txt"),
            "NotoSerifTC":include_str!("../../../../../apps/save-editor/assets/licenses/NotoSerifTC-OFL.txt")}}),
        ),
        ("", "overview" | "statistics" | "report") => {
            let mut selected = o.clone();
            if v == "report" && o.with_assets && o.manifest.is_none() && !o.dry_run {
                match call("item_icons_prepare", payload(o)?) {
                    Ok(prepared) => {
                        selected.manifest = prepared["manifestPath"].as_str().map(PathBuf::from)
                    }
                    Err(error) => eprintln!("Optional images unavailable: {error}"),
                }
            }
            let o = &selected;
            let data = overview(o)?;
            if v != "report" {
                return Ok(data);
            }
            let out = o.out.as_deref().context("--out required")?;
            if o.format != "html" {
                bail!("report supports --format html");
            }
            let settings = administration::settings_read("ui")?;
            let body = super::report::html(&data, &settings, o)?;
            if !o.dry_run {
                fs::write(out, body)?;
                if o.open {
                    open(out)?
                }
            }
            Ok(json!({"path":out,"format":"html","dryRun":o.dry_run}))
        }
        _ => bail!("unknown display command"),
    }
}

fn assets(v: &str, o: &Options) -> Result<Value> {
    if o.kind.as_deref() == Some("portraits") {
        return portraits(v, o);
    }
    if v == "prepare" {
        if o.dry_run {
            return Ok(api::validate_item_icons(&payload(o)?)?);
        }
        return call("item_icons_prepare", payload(o)?);
    }
    if v == "status" {
        return call("item_icons_source_identity", payload(o)?);
    }
    let manifest = o
        .manifest
        .as_deref()
        .context("--manifest required (from assets prepare)")?;
    if v == "release" {
        if o.dry_run {
            return Ok(json!({"dryRun":true,"manifest":manifest}));
        }
        return call("item_icons_release", json!({"manifestPath":manifest}));
    }
    let data = serde_json::to_value(gore_tex::item_icons::verified_item_icon_manifest(manifest)?)?;
    if v == "list" {
        return Ok(data);
    }
    let id =
        o.id.as_deref()
            .or(o.item.as_deref())
            .context("--id or --item required")?;
    let relative = data["items"][id]
        .as_str()
        .context("asset not in manifest")?;
    let root = manifest
        .parent()
        .context("manifest has no parent")?
        .canonicalize()?;
    let file = root.join(relative).canonicalize()?;
    if !file.starts_with(&root) {
        bail!("asset is outside its cache");
    }
    if v == "open" {
        if !o.dry_run {
            open(&file)?
        }
        return Ok(json!({"path":file}));
    }
    let out = o.out.as_deref().context("--out required")?;
    if !o.dry_run {
        fs::copy(&file, out)?;
        if o.open {
            open(out)?
        }
    }
    Ok(json!({"path":out,"source":file,"dryRun":o.dry_run}))
}

pub(super) fn attach_artwork(data: &mut Value, o: &Options) {
    data["artwork"] =
        portrait_entries(o).unwrap_or_else(|e| json!({"available":false,"error":e.to_string()}));
}
fn portrait_entries(o: &Options) -> Result<Value> {
    let game =
        gore_loc::config::game_root(o.game.clone()).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let root = game.join("G1R/Story/Conversation/images/Glossary");
    let catalog = catalog("portraits")?;
    let mut rows = Vec::new();
    for key in ["images", "artwork"] {
        for (id, entry) in catalog[key]
            .as_object()
            .context("invalid artwork catalog")?
        {
            let kind = entry["kind"].as_str().context("artwork has no kind")?;
            let name = entry["name"].as_str().unwrap_or(id);
            let plain = |s: &str| {
                !s.is_empty() && s != "." && s != ".." && !s.contains(['/', '\\', ':', '\0'])
            };
            if !plain(kind) || !plain(name) {
                bail!("invalid artwork path segment");
            }
            for size in ["S", "M"] {
                let path = root
                    .join(kind)
                    .join(format!("T_GlossaryImage_{name}_{size}.png"));
                rows.push(json!({"id":id,"name":name,"kind":kind,"size":size,"path":path,"available":path.is_file()}));
            }
        }
    }
    Ok(json!({"sourceGamePath":game,"sourceRoot":root,"available":root.is_dir(),"entries":rows}))
}
fn portraits(v: &str, o: &Options) -> Result<Value> {
    let mut data = portrait_entries(o)?;
    if matches!(v, "prepare" | "status" | "list") {
        filter(&mut data, "entries", o);
        paginate(&mut data, "entries", o);
        return Ok(data);
    }
    if v == "release" {
        return Ok(
            json!({"released":false,"reason":"original loose artwork has no generated cache"}),
        );
    }
    let id =
        o.id.as_deref()
            .or(o.document.as_deref())
            .context("--id or --document required")?;
    let size = if o.details { "M" } else { "S" };
    let row = data["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["id"]
                .as_str()
                .is_some_and(|s| s.eq_ignore_ascii_case(id.rsplit('.').next().unwrap_or(id)))
                && r["size"] == size
        })
        .context("artwork not in the catalog")?;
    let path = Path::new(row["path"].as_str().unwrap())
        .canonicalize()
        .context("original artwork is missing")?;
    let root = Path::new(data["sourceRoot"].as_str().unwrap()).canonicalize()?;
    if !path.starts_with(root) {
        bail!("artwork is outside the game source");
    }
    if v == "open" {
        if !o.dry_run {
            open(&path)?;
        }
        return Ok(json!({"path":path,"dryRun":o.dry_run}));
    }
    let out = o.out.as_deref().context("--out required")?;
    if !o.dry_run {
        fs::copy(&path, out)?;
        if o.open {
            open(out)?;
        }
    }
    Ok(json!({"path":out,"source":path,"dryRun":o.dry_run}))
}

pub(super) fn clock_parts(row: &Value) -> Value {
    let seconds = row["editValue"]
        .as_f64()
        .or_else(|| row["value"].as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0.0);
    let whole = seconds.max(0.0).floor() as u64;
    json!({"totalSeconds":seconds,"day":whole/86400,"hour":whole%86400/3600,"minute":whole%3600/60,"second":whole%60,"path":row["path"]})
}

fn restock_forecast(activity: Option<f64>, world: Option<f64>, interval: Option<u64>) -> Value {
    let never = activity.is_some_and(|a| a <= -1000.0);
    let valid = activity.is_some_and(|a| a.is_finite() && a >= 0.0)
        && interval.is_some_and(|n| n > 0)
        && world.is_none_or(|w| w.is_finite() && w >= 0.0);
    let calendar = valid.then(|| {
        (activity.unwrap() / 86400.0).floor() * 86400.0 + interval.unwrap() as f64 * 86400.0
    });
    let elapsed = valid.then(|| activity.unwrap() + interval.unwrap() as f64 * 86400.0);
    let state = if never {
        "neverActive"
    } else if !valid || world.is_none() {
        "unavailable"
    } else if activity.unwrap() > world.unwrap() {
        "clockAhead"
    } else if world.unwrap() < calendar.unwrap() {
        "beforeWindow"
    } else if world.unwrap() < elapsed.unwrap() {
        "boundaryOnly"
    } else {
        "eligibleBoth"
    };
    json!({"activitySeconds":activity,"worldSeconds":world,"intervalDays":interval,"neverActive":never,"state":state,"calendarBoundarySeconds":calendar,"elapsedBoundarySeconds":elapsed,"maintenance":"performed lazily when the game processes the merchant"})
}
pub(super) fn timing(v: &str, o: &Options) -> Result<Value> {
    let data = call("private.traders.detail", payload(o)?)?;
    // A custom activity timestamp remains editable without a known difficulty
    // or clock. Only a forecast and Make due require those additional inputs.
    let seconds = if v == "set" {
        o.seconds.context("--seconds required")?
    } else {
        let world = clock(o)
            .ok()
            .and_then(|row| clock_parts(&row)["totalSeconds"].as_f64());
        let resources = administration::resources_level(o);
        let interval = resources
            .as_ref()
            .ok()
            .and_then(|level| match level.as_str() {
                "Novice" => Some(2),
                "Gothic" => Some(3),
                "Hard" => Some(5),
                _ => None,
            });
        if v == "show" {
            let mut forecast = restock_forecast(data["totalSeconds"].as_f64(), world, interval);
            forecast["path"] = data["totalSecondsPath"].clone();
            if let Err(error) = resources {
                forecast["resourcesWarning"] = json!(error.to_string());
            }
            return Ok(forecast);
        }
        let world = world.context("world clock is unavailable")?;
        let interval =
            interval.context("resources difficulty is unknown; supply --resources-level")?;
        let seconds = world - interval as f64 * 86400.0 - 1.0;
        if seconds < 0.0 {
            bail!("the current world day is too early to express an elapsed restock interval");
        }
        seconds
    };
    if !seconds.is_finite() {
        bail!("timestamp must be finite");
    }
    write(
        o,
        vec![edit(
            "private.typed.setValue",
            json!({"path":data["totalSecondsPath"],"value":seconds}),
        )],
        json!({}),
    )
}

pub(super) fn position(v: &str, o: &Options) -> Result<Value> {
    let hero = o.actor.eq_ignore_ascii_case("hero");
    let mut resolved = o.clone();
    if !hero {
        resolved.actor = npc_id(o)?;
    }
    let o = &resolved;
    let data = if hero {
        call(
            "inspect_save",
            json!({"path":save(o)?,"includePrivate":true}),
        )?
    } else {
        call(
            "private.npc.position",
            json!({"path":save(o)?,"id":npc_id(o)?}),
        )?
    };
    if matches!(v, "show" | "pin-status") {
        return Ok(data);
    }
    let pending = if let Some(file) = o.draft.as_deref().filter(|p| p.is_file()) {
        Some(read_json(file)?)
    } else {
        None
    };
    if hero {
        if v != "set" {
            bail!("this operation requires an NPC");
        }
        let original = &data["private"]["player"]["transform"];
        let mut transform = pending
            .as_ref()
            .and_then(|d| d["edits"].as_array())
            .and_then(|rows| {
                rows.iter()
                    .find(|e| e["path"] == "private.player.setTransform")
            })
            .map(|e| e["value"].clone())
            .unwrap_or_else(|| original.clone());
        fill_transform(&mut transform, o)?;
        return write(
            o,
            vec![edit("private.player.setTransform", transform)],
            json!({}),
        );
    }
    let pose = &data["pose"];
    let mut next = json!({"location":pose["location"],"rotation":pose["rotation"]});
    if let Some(rows) = pending.as_ref().and_then(|d| d["edits"].as_array()) {
        for edit in rows {
            for (leaf, key) in [("location", "locationPath"), ("rotation", "rotationPath")] {
                if edit["path"] == "private.typed.setValue" && edit["value"]["path"] == pose[key] {
                    next[leaf] = edit["value"]["value"].clone();
                }
            }
        }
    }
    let pending_note = pending
        .as_ref()
        .and_then(|d| d["placementNotes"].as_array())
        .and_then(|rows| rows.iter().find(|r| r["npc"] == o.actor))
        .map(|r| serde_json::from_value::<gore_save::placement::PlacementNote>(r["note"].clone()))
        .transpose()?;
    let mut extras = json!({});
    let mut edits = Vec::new();
    if v == "reset-to-spawn" {
        if pose["spawnLocation"].is_null()
            || ["x", "y", "z"]
                .iter()
                .all(|k| pose["spawnLocation"][*k].as_f64() == Some(0.0))
        {
            bail!("NPC has no spawn pose");
        }
        next["location"] = pose["spawnLocation"].clone();
        if !pose["spawnRotation"].is_null() {
            next["rotation"] = pose["spawnRotation"].clone();
        }
    } else if matches!(v, "undo" | "resume-routine") {
        let valid = if v == "undo" {
            "restorable"
        } else {
            "routineRestorable"
        };
        if data["undo"][valid] != true {
            bail!("placement undo is absent or stale");
        }
        if v == "undo" {
            next["location"] = data["undo"]["originalLocation"].clone();
            if !data["undo"]["originalRotation"].is_null() {
                next["rotation"] = data["undo"]["originalRotation"].clone();
            }
        }
        if !data["undo"]["originalRoutineClass"].is_null() {
            edits.push(edit("private.typed.setValue",json!({"path":data["routineClassPath"],"value":data["undo"]["originalRoutineClass"]})));
        }
        extras["clearPlacementNotes"] = json!([o.actor]);
    } else {
        fill_transform(&mut next, o)?;
    }
    for (leaf, path) in [("location", "locationPath"), ("rotation", "rotationPath")] {
        if next[leaf] != pose[leaf] {
            edits.push(edit(
                "private.typed.setValue",
                json!({"path":pose[path],"value":next[leaf]}),
            ));
        }
    }
    let pinned =
        data["routineClass"] == data["inertRoutineClass"] && !data["routineClass"].is_null();
    if o.stay || ((pinned || pending_note.is_some()) && matches!(v, "set" | "reset-to-spawn")) {
        if data["inertRoutineClass"].is_null()
            || pose["location"].is_null()
            || data["routineClassPath"]
                .as_array()
                .is_none_or(|p| p.is_empty())
        {
            bail!("NPC cannot be pinned");
        }
        if !pinned {
            edits.push(edit(
                "private.typed.setValue",
                json!({"path":data["routineClassPath"],"value":data["inertRoutineClass"]}),
            ));
        }
        let previous = gore_save::placement::read_notes(save(o)?).remove(&o.actor);
        if previous.is_some()
            && (data["undo"]["restorable"] != true || data["undo"]["routineRestorable"] != true)
        {
            bail!("placement undo is stale; inspect the current pose before pinning again");
        }
        if pinned && previous.is_none() {
            bail!("NPC is pinned without an original routine note");
        }
        let previous = pending_note.or(previous);
        let triplet = |point: &Value, keys: [&str; 3]| -> Result<[f64; 3]> {
            Ok([
                point[keys[0]].as_f64().context("missing coordinate")?,
                point[keys[1]].as_f64().context("missing coordinate")?,
                point[keys[2]].as_f64().context("missing coordinate")?,
            ])
        };
        let changed_rotation = next["rotation"] != pose["rotation"];
        let note = gore_save::placement::PlacementNote {
            original_location: match &previous {
                Some(n) => n.original_location,
                None => triplet(&pose["location"], ["x", "y", "z"])?,
            },
            original_routine_class: match &previous {
                Some(n) => n.original_routine_class.clone(),
                None => data["routineClass"].as_str().map(str::to_owned),
            },
            original_rotation: previous.as_ref().and_then(|n| n.original_rotation).or(
                if changed_rotation {
                    Some(triplet(&pose["rotation"], ["pitch", "yaw", "roll"])?)
                } else {
                    None
                },
            ),
            written_location: triplet(&next["location"], ["x", "y", "z"])?,
            written_rotation: if changed_rotation
                || previous
                    .as_ref()
                    .is_some_and(|n| n.original_rotation.is_some())
            {
                Some(triplet(&next["rotation"], ["pitch", "yaw", "roll"])?)
            } else {
                None
            },
            written_routine_class: data["inertRoutineClass"].as_str().map(str::to_owned),
        };
        extras["placementNotes"] = json!([{"npc":o.actor,"note":note}]);
    }
    if edits.is_empty() {
        return Ok(json!({"complete":true,"committed":[],"remaining":[],"unchanged":true}));
    }
    write(o, edits, extras)
}
fn fill_transform(next: &mut Value, o: &Options) -> Result<()> {
    if let Some(location) = &o.location {
        let catalog = gore_catalog::location::LocationCatalog::bundled()?;
        let spot = catalog
            .resolve(location)
            .context("unknown named location")?;
        next["location"] = json!({"x":spot.x,"y":spot.y,"z":spot.z});
        if o.apply_facing {
            next["rotation"] = json!({"pitch":0.0,"yaw":spot.w,"roll":0.0});
        }
    }
    for (group, key, val) in [
        ("location", "x", o.x),
        ("location", "y", o.y),
        ("location", "z", o.z),
        ("rotation", "pitch", o.pitch),
        ("rotation", "yaw", o.yaw),
        ("rotation", "roll", o.roll),
    ] {
        if let Some(val) = val {
            if !val.is_finite() {
                bail!("coordinate must be finite");
            }
            next[group][key] = json!(val);
        }
    }
    Ok(())
}

fn overview(o: &Options) -> Result<Value> {
    let inspection = call(
        "inspect_save",
        json!({"path":save(o)?,"includePrivate":true}),
    )?;
    let mut data = json!({"inspection":inspection});
    for (key, command) in [
        ("characters", "private.characters.list"),
        ("skills", "private.skills.list"),
        (
            "inventory",
            if o.actor.eq_ignore_ascii_case("hero") {
                "inspect_save"
            } else {
                "private.npc.inventory"
            },
        ),
        ("factions", "private.factions.list"),
        ("traders", "private.traders.list"),
    ] {
        data[key] = match call(command, payload(o)?) {
            Ok(v) => v,
            Err(e) => json!({"available":false,"error":e.to_string()}),
        };
    }
    data["worldTime"] = match clock(o) {
        Ok(row) => clock_parts(&row),
        Err(e) => json!({"available":false,"error":e.to_string()}),
    };
    data["quests"] = match super::progression(
        "quests",
        &Options {
            all: true,
            ..o.clone()
        },
    ) {
        Ok(v) => v,
        Err(e) => json!({"available":false,"error":e.to_string()}),
    };
    for section in ["tutorials", "glossary", "story", "knowledge", "events"] {
        data[section] = super::progression(
            section,
            &Options {
                all: true,
                include_unset: true,
                query: None,
                group: None,
                state: None,
                category: None,
                ..o.clone()
            },
        )
        .unwrap_or_else(|e| json!({"available":false,"error":e.to_string()}));
    }
    data["attributes"] = super::attributes(
        "list",
        &Options {
            all: true,
            ..o.clone()
        },
    )
    .unwrap_or_else(|e| json!({"available":false,"error":e.to_string()}));
    if o.with_assets {
        data["artwork"] = portrait_entries(o)
            .unwrap_or_else(|e| json!({"available":false,"error":e.to_string()}));
        if let Some(manifest) = &o.manifest {
            data["icons"] =
                serde_json::to_value(gore_tex::item_icons::verified_item_icon_manifest(manifest)?)?;
            data["icons"]["manifestPath"] = json!(manifest);
        } else {
            data["icons"] = json!({"available":false,"reason":"prepare an image manifest with gore save assets prepare, then pass --manifest"});
        }
    }
    data["statistics"] = super::presentation::statistics(&data, o)?;
    localize(&mut data, o)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn merchant_windows_match_calendar_elapsed_and_unknown_input_rules() {
        let activity = Some(12.0 * 3600.0);
        assert_eq!(
            restock_forecast(activity, Some(3.0 * 86400.0), Some(3))["state"],
            "boundaryOnly"
        );
        assert_eq!(
            restock_forecast(activity, Some(3.5 * 86400.0), Some(3))["state"],
            "eligibleBoth"
        );
        assert_eq!(
            restock_forecast(Some(-1000.0), None, None)["state"],
            "neverActive"
        );
        assert_eq!(
            restock_forecast(activity, None, Some(3))["state"],
            "unavailable"
        );
        assert_eq!(
            restock_forecast(activity, Some(0.0), Some(3))["state"],
            "clockAhead"
        );
        assert!(
            restock_forecast(activity, Some(86400.0), None)["calendarBoundarySeconds"].is_null()
        );
    }
}
