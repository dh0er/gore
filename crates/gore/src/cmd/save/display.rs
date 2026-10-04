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

fn same_export_file(left: &fs::File, right: &fs::File) -> Result<bool> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let left = left.metadata()?;
        let right = right.metadata()?;
        Ok((left.dev(), left.ino()) == (right.dev(), right.ino()))
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
        };
        let identity = |file: &fs::File| -> Result<_> {
            let mut info = unsafe { std::mem::zeroed::<BY_HANDLE_FILE_INFORMATION>() };
            // SAFETY: the file owns a valid handle and info is a writable Win32 buffer.
            if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
                return Err(io::Error::last_os_error().into());
            }
            Ok((
                info.dwVolumeSerialNumber,
                info.nFileIndexHigh,
                info.nFileIndexLow,
            ))
        };
        Ok(identity(left)? == identity(right)?)
    }
    #[cfg(not(any(unix, windows)))]
    {
        bail!("export file identity checks are unsupported on this platform")
    }
}

fn export_output(
    source: &fs::File,
    out: &Path,
    dry_run: bool,
    source_label: &str,
) -> Result<Option<fs::File>> {
    let output = if dry_run {
        match fs::File::open(out) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let parent = out
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                    .unwrap_or(Path::new("."));
                if !fs::metadata(parent)?.is_dir() {
                    bail!("export output parent must be a directory");
                }
                return Ok(None);
            }
            Err(error) => return Err(error.into()),
        }
    } else {
        fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(out)?
    };
    if !output.metadata()?.is_file() {
        bail!("export output must be a regular file");
    }
    if same_export_file(source, &output)? {
        bail!("export output must not refer to the source {source_label}");
    }
    Ok(Some(output))
}

fn write_save_export(o: &Options, out: &Path, bytes: &[u8]) -> Result<()> {
    let p = payload(o)?;
    let source = fs::File::open(p["path"].as_str().context("a save file is required")?)?;
    let output = export_output(&source, out, o.dry_run, "save")?;
    if !o.dry_run {
        let mut output = output.context("export output was not opened")?;
        // Validate the opened file before truncating, and write through that same handle.
        output.set_len(0)?;
        output.write_all(bytes)?;
    }
    Ok(())
}

fn copy_artwork_export(source: &Path, out: &Path, dry_run: bool) -> Result<()> {
    let mut source = fs::File::open(source)?;
    let output = export_output(&source, out, dry_run, "artwork")?;
    if !dry_run {
        let mut output = output.context("export output was not opened")?;
        output.set_len(0)?;
        io::copy(&mut source, &mut output)?;
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

fn item_category(id: &str, stats: &Value, filters: &[&Value]) -> &'static str {
    if stats.is_null() {
        use gore_catalog::ItemCategory;
        return match gore_catalog::item_category_from_id(id) {
            ItemCategory::MeleeWeapon => "meleeWeapon",
            ItemCategory::RangedWeapon | ItemCategory::Ammunition => "rangedWeapon",
            ItemCategory::Rune | ItemCategory::Scroll => "magic",
            ItemCategory::Armor | ItemCategory::Amulet | ItemCategory::Ring => "wearable",
            ItemCategory::Food
                if id.starts_with("ItFo_Potion_") || id.starts_with("ItFo_Booze") =>
            {
                "potion"
            }
            ItemCategory::Food => "food",
            ItemCategory::Trophy => "material",
            ItemCategory::Writing => "document",
            ItemCategory::Misc => "misc",
            ItemCategory::Mission | ItemCategory::Key => "artefact",
            _ => "other",
        };
    }
    let claims = |filter: &Value, tag: &str| {
        filter["itemTags"].as_array().is_some_and(|tags| {
            tags.iter().filter_map(Value::as_str).any(|parent| {
                tag == parent
                    || tag
                        .strip_prefix(parent)
                        .is_some_and(|tail| tail.starts_with('_'))
            })
        })
    };
    let category = |filter: &Value| {
        super::presentation::metadata()["itemCategories"][filter["id"].as_str().unwrap_or("")]
            .as_str()
            .unwrap_or("other")
    };
    let mut by_type = None;
    for &filter in filters {
        // Property matches outrank type matches, including later filters (forge stock).
        if stats["specs"].as_array().is_some_and(|specs| {
            specs
                .iter()
                .filter_map(Value::as_str)
                .any(|tag| claims(filter, tag))
        }) {
            return category(filter);
        }
        if by_type.is_none()
            && stats["itemType"]
                .as_str()
                .is_some_and(|tag| !tag.is_empty() && claims(filter, tag))
        {
            by_type = Some(filter);
        }
    }
    // Known items no filter claims stay in Other; only unknown ids use prefixes.
    by_type.map(category).unwrap_or("other")
}

pub(super) fn annotate_items(rows: &mut [Value], details: bool) -> Result<()> {
    let stats = catalog("item-stats")?;
    let by_id = stats["items"]
        .as_object()
        .context("invalid item stats catalog")?
        .iter()
        .map(|(id, stats)| (id.trim().to_lowercase(), stats))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut filters = stats["filters"]
        .as_array()
        .context("invalid inventory filters")?
        .iter()
        .filter(|filter| filter["id"].as_str().is_some_and(|id| !id.is_empty()))
        .collect::<Vec<_>>();
    filters.sort_by_key(|filter| filter["sortOrder"].as_i64().unwrap_or(1 << 20));
    for row in rows {
        let id = row["id"].as_str().unwrap_or("").to_string();
        let info = by_id
            .get(&id.trim().to_lowercase())
            .copied()
            .unwrap_or(&Value::Null);
        row["category"] = json!(item_category(&id, info, &filters));
        if details {
            row["stats"] = info.clone();
        }
    }
    Ok(())
}

pub(super) fn filter(data: &mut Value, key: &str, o: &Options) {
    filter_rows(data, key, o, false);
}

pub(super) fn filter_items(data: &mut Value, key: &str, o: &Options) {
    filter_rows(data, key, o, true);
}

fn filter_rows(data: &mut Value, key: &str, o: &Options, items: bool) {
    let Some(rows) = data[key].as_array_mut() else {
        return;
    };
    rows.retain(|r| {
        o.query.as_ref().is_none_or(|q| {
            let query = q.to_lowercase();
            if items {
                ["id", "path", "idText"].iter().any(|field| {
                    r[*field]
                        .as_str()
                        .is_some_and(|value| value.to_lowercase().contains(&query))
                })
            } else {
                r.to_string().to_lowercase().contains(&query)
            }
        }) && o.id.as_ref().is_none_or(|id| {
            if items {
                ["id", "path"].iter().any(|field| {
                    r[*field]
                        .as_str()
                        .is_some_and(|value| value.eq_ignore_ascii_case(id))
                })
            } else {
                r.as_object().is_some_and(|r| {
                    r.values().any(|value| {
                        value
                            .as_str()
                            .is_some_and(|value| value.eq_ignore_ascii_case(id))
                    })
                })
            }
        }) && o.category.as_ref().is_none_or(|c| {
            r["category"]
                .as_str()
                .is_some_and(|v| v.eq_ignore_ascii_case(c))
                || r["a"].as_str() == Some(c)
        }) && o.role.as_ref().is_none_or(|role| match role.as_str() {
            "teacher" => r["teacher"] == true,
            "trader" => r["isTrader"] == true,
            _ => r["roles"]
                .as_array()
                .is_some_and(|roles| roles.contains(&json!(role))),
        }) && (key != "characters"
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

pub(super) fn catalog_page(domain: &str, o: &Options, texts: &super::text::Texts) -> Result<Value> {
    let mut data = catalog(domain)?;
    let dictionary = matches!(
        domain,
        "story-semantics" | "glossary-text" | "portraits" | "hero-attributes"
    );
    // Normalize only the entry dictionaries in each bundled schema. Wrapper
    // metadata (schema, filters, hidden attributes, etc.) is not an entry.
    match domain {
        "story-semantics" => {
            data = json!({"entries":data.as_object().context("invalid story semantics")?.values().collect::<Vec<_>>()});
        }
        "glossary-text" => {
            data = json!({"entries":data.as_object().context("invalid glossary texts")?.iter().map(|(id,text_ids)|json!({"id":id,"segmentClass":id,"textIds":text_ids})).collect::<Vec<_>>()});
        }
        "portraits" => {
            let mut rows = Vec::new();
            for source in ["images", "artwork"] {
                let entries = data
                    .as_object_mut()
                    .context("invalid portrait catalog")?
                    .remove(source)
                    .context("portrait catalog has no entry dictionary")?;
                for (id, entry) in entries.as_object().context("invalid portrait entries")? {
                    let mut row = entry.clone();
                    row["id"] = json!(id);
                    row["source"] = json!(source);
                    rows.push(row);
                }
            }
            data["entries"] = json!(rows);
        }
        "hero-attributes" => {
            let groups = data
                .as_object_mut()
                .context("invalid hero attribute catalog")?
                .remove("groups")
                .context("hero attribute catalog has no groups")?;
            data["entries"] = json!(
                groups
                    .as_object()
                    .context("invalid attribute groups")?
                    .iter()
                    .map(|(id, attributes)| json!({"id":id,"attributes":attributes}))
                    .collect::<Vec<_>>()
            );
        }
        _ => {}
    }
    if domain == "item-stats" {
        let items = data
            .as_object_mut()
            .context("invalid item stats catalog")?
            .remove("items")
            .context("item stats have no items")?;
        let mut rows = items
            .as_object()
            .context("invalid item stats")?
            .iter()
            .map(|(id, stats)| {
                let mut row = stats.clone();
                row["id"] = json!(id);
                row
            })
            .collect::<Vec<_>>();
        annotate_items(&mut rows, false)?;
        data["entries"] = json!(rows);
    }
    if domain == "ui-texts" {
        data = json!({"entries":data[&o.lang].as_object().context("unknown language")?.iter().map(|(key,text)|json!({"id":key,"text":text})).collect::<Vec<_>>()});
    }
    if matches!(domain, "item" | "items") {
        annotate_items(data.as_array_mut().context("invalid item catalog")?, true)?;
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
    texts.apply(&mut data)?;
    if dictionary {
        if let Some(id) = options.id.take() {
            // The dictionary key (or its canonical id) identifies a row;
            // content such as a portrait name or semantic kind does not.
            data["entries"]
                .as_array_mut()
                .context("invalid dictionary catalog entries")?
                .retain(|row| {
                    row["id"]
                        .as_str()
                        .is_some_and(|value| value.eq_ignore_ascii_case(&id))
                });
        }
    }
    if matches!(domain, "item" | "items" | "item-stats") {
        filter_items(&mut data, key, &options);
    } else {
        filter(&mut data, key, &options);
    }
    paginate(&mut data, key, o);
    Ok(data)
}

pub(super) fn dispatch(g: &str, v: &str, o: &Options) -> Result<Value> {
    match (g, v) {
        ("catalog" | "items" | "locations", _) => {
            let mut options = o.clone();
            let domain = if g == "catalog" {
                if let Some(source) = o.source.as_deref() {
                    source
                } else {
                    options.kind = None;
                    o.kind.as_deref().unwrap_or("items")
                }
            } else {
                g
            };
            catalog_page(domain, &options, &super::text::Texts::load_options(o)?)
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
                    id.to_lowercase().contains(&query)
                        || texts.to_string().to_lowercase().contains(&query)
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
            let p = payload(o)?;
            let source = Path::new(p["path"].as_str().context("a save file is required")?);
            let screenshot =
                gore_save::screenshot_for_save(source)?.context("save has no screenshot")?;
            let bytes =
                base64::engine::general_purpose::STANDARD.decode(&screenshot.bytes_base64)?;
            let out = o.out.as_deref().context("--out required")?;
            write_save_export(o, out, &bytes)?;
            if !o.dry_run {
                if o.open {
                    open(out)?
                }
            }
            Ok(
                json!({"path":out,"byteLength":bytes.len(),"mimeType":screenshot.mime_type,"dryRun":o.dry_run}),
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
            let settings = administration::settings_read("ui").unwrap_or_default();
            let body = super::report::html(&data, &settings, o)?;
            write_save_export(o, out, body.as_bytes())?;
            if !o.dry_run {
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
        return retain_cli_assets(call("item_icons_prepare", payload(o)?)?);
    }
    if v == "status" {
        return call("item_icons_source_identity", payload(o)?);
    }
    let manifest_path = std::path::absolute(
        o.manifest
            .as_deref()
            .context("--manifest required (from assets prepare)")?,
    )?;
    let manifest = manifest_path.as_path();
    if v == "release" {
        if o.dry_run {
            let would_release =
                gore_tex::item_icons::preview_release_item_icon_cache_for_cli(manifest)?;
            return Ok(json!({"dryRun":true,"manifest":manifest,"wouldRelease":would_release}));
        }
        let released = gore_tex::item_icons::release_item_icon_cache_for_cli(manifest)?;
        return Ok(json!({"released":released}));
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
    copy_artwork_export(&file, out, o.dry_run)?;
    if !o.dry_run {
        if o.open {
            open(out)?
        }
    }
    Ok(json!({"path":out,"source":file,"dryRun":o.dry_run}))
}

fn retain_cli_assets(mut prepared: Value) -> Result<Value> {
    let manifest = prepared["manifestPath"]
        .as_str()
        .context("item icon preparation returned no manifest path")?;
    let manifest = Path::new(manifest);
    gore_tex::item_icons::retain_item_icon_cache_for_cli(manifest)?;
    gore_tex::item_icons::release_item_icon_cache(manifest)?;
    prepared["leaseScope"] = json!("cli");
    Ok(prepared)
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
    copy_artwork_export(&path, out, o.dry_run)?;
    if !o.dry_run {
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
    if hero && !matches!(v, "show" | "set") {
        bail!("this operation requires an NPC");
    }
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
        if hero {
            return data["private"]["player"]
                .get("transform")
                .filter(|transform| transform.is_object())
                .cloned()
                .context("player transform is unavailable");
        }
        return Ok(data);
    }
    let pending = if let Some(file) = o.draft.as_deref().filter(|p| p.is_file()) {
        Some(read_json(file)?)
    } else {
        None
    };
    if hero {
        let original = &data["private"]["player"]["transform"];
        let mut transform = original.clone();
        for staged in pending
            .as_ref()
            .and_then(|d| d["edits"].as_array())
            .into_iter()
            .flatten()
            .filter(|e| e["path"] == "private.player.setTransform")
            .map(|e| &e["value"])
        {
            let fields = staged
                .as_object()
                .context("pending transform must be an object")?;
            transform
                .as_object_mut()
                .context("player transform is unavailable")?
                .extend(fields.clone());
        }
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
        .and_then(|rows| {
            rows.iter().find(|r| {
                r["npc"]
                    .as_str()
                    .is_some_and(|npc| npc.eq_ignore_ascii_case(&o.actor))
            })
        })
        .map(|r| serde_json::from_value::<gore_save::placement::PlacementNote>(r["note"].clone()))
        .transpose()?;
    let previous_note = gore_save::placement::read_notes(save(o)?).remove(&o.actor);
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
        let original_routine = data["undo"]["originalRoutineClass"]
            .as_str()
            .filter(|class| !class.trim().is_empty());
        if original_routine.is_none()
            && (v == "resume-routine"
                || previous_note
                    .as_ref()
                    .is_some_and(|note| note.written_routine_class.is_some()))
        {
            bail!("the original NPC routine is unavailable; placement note retained");
        }
        if v == "undo" {
            next["location"] = data["undo"]["originalLocation"].clone();
            if !data["undo"]["originalRotation"].is_null() {
                next["rotation"] = data["undo"]["originalRotation"].clone();
            }
        }
        if let Some(class) = original_routine {
            edits.push(edit(
                "private.typed.setValue",
                json!({"path":data["routineClassPath"],"value":class}),
            ));
        }
        extras["clearPlacementNotes"] = json!([o.actor]);
    } else {
        fill_transform(&mut next, o)?;
    }
    for (leaf, path) in [("location", "locationPath"), ("rotation", "rotationPath")] {
        let staged = pending
            .as_ref()
            .and_then(|draft| draft["edits"].as_array())
            .is_some_and(|edits| {
                edits.iter().any(|edit| {
                    edit["path"] == "private.typed.setValue" && edit["value"]["path"] == pose[path]
                })
            });
        if next[leaf] != pose[leaf] || staged {
            edits.push(edit(
                "private.typed.setValue",
                json!({"path":pose[path],"value":next[leaf]}),
            ));
        }
    }
    let pinned =
        data["routineClass"] == data["inertRoutineClass"] && !data["routineClass"].is_null();
    let pending_routine = pending
        .as_ref()
        .and_then(|draft| draft["edits"].as_array())
        .and_then(|edits| {
            edits.iter().find(|edit| {
                edit["path"] == "private.typed.setValue"
                    && edit["value"]["path"] == data["routineClassPath"]
            })
        })
        .map(|edit| &edit["value"]["value"]);
    // A queued restore takes precedence over the routine still stored on disk.
    let effective_routine = pending_routine.unwrap_or(&data["routineClass"]);
    let effective_pinned =
        *effective_routine == data["inertRoutineClass"] && !effective_routine.is_null();
    if o.stay
        || ((effective_pinned || pending_note.is_some()) && matches!(v, "set" | "reset-to-spawn"))
    {
        if data["inertRoutineClass"].is_null()
            || pose["location"].is_null()
            || data["routineClassPath"]
                .as_array()
                .is_none_or(|p| p.is_empty())
        {
            bail!("NPC cannot be pinned");
        }
        if !pinned || pending_routine.is_some_and(|class| *class != data["inertRoutineClass"]) {
            edits.push(edit(
                "private.typed.setValue",
                json!({"path":data["routineClassPath"],"value":data["inertRoutineClass"]}),
            ));
        }
        let previous = previous_note;
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
    data["inspection"]["screenshot"] =
        serde_json::to_value(gore_save::screenshot_for_save(save(o)?)?)?;
    for (key, command) in [
        ("characters", "private.characters.list"),
        ("factions", "private.factions.list"),
        ("traders", "private.traders.list"),
    ] {
        data[key] = match call(command, payload(o)?) {
            Ok(v) => v,
            Err(e) => json!({"available":false,"error":e.to_string()}),
        };
    }
    let selected = Options {
        all: true,
        offset: 0,
        ..o.clone()
    };
    for section in ["inventory", "skills"] {
        data[section] = super::dispatch(section, "list", &selected)
            .unwrap_or_else(|error| json!({"available":false,"error":error.to_string()}));
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

    fn catalog_options(domain: &str) -> Options {
        Options {
            kind: Some(domain.into()),
            lang: "en".into(),
            game_lang: "en".into(),
            limit: 1,
            ..Options::default()
        }
    }

    #[test]
    fn dictionary_catalog_search_and_exact_id_reject_missing_entries() {
        for domain in [
            "story-semantics",
            "glossary-text",
            "portraits",
            "hero-attributes",
        ] {
            let options = catalog_options(domain);
            for selected in [
                Options {
                    query: Some("PR123_NO_SUCH_QUEST_987654".into()),
                    ..options.clone()
                },
                Options {
                    id: Some("PR123_NO_SUCH_QUEST_987654".into()),
                    ..options.clone()
                },
            ] {
                let page = dispatch("catalog", "search", &selected).unwrap();
                assert_eq!(page["entries"], json!([]), "{domain}");
                assert_eq!(page["total"], 0, "{domain}");
                assert_eq!(page["count"], 0, "{domain}");
                assert_eq!(page["offset"], 0, "{domain}");
            }
        }
    }

    #[test]
    fn dictionary_catalog_story_rows_retain_semantics_and_canonical_ids() {
        let raw = catalog("story-semantics").unwrap();
        let options = catalog_options("story-semantics");
        let selected = dispatch(
            "catalog",
            "show",
            &Options {
                id: Some("BAALLUKOR_BRINGPARCHMENT".into()),
                ..options.clone()
            },
        )
        .unwrap();
        assert_eq!(selected["total"], 1);
        assert_eq!(selected["entries"][0], raw["baallukor_bringparchment"]);
        assert_eq!(selected["entries"][0]["id"], "BaalLukor_BringParchment");
        let expected = raw
            .as_object()
            .unwrap()
            .values()
            .filter(|row| row["kind"] == "finiteState")
            .cloned()
            .collect::<Vec<_>>();
        let searched = dispatch(
            "catalog",
            "search",
            &Options {
                query: Some("FINITESTATE".into()),
                offset: 1,
                ..options.clone()
            },
        )
        .unwrap();
        assert!(expected.len() > 1);
        assert_eq!(searched["total"], expected.len());
        assert_eq!(searched["count"], 1);
        assert_eq!(searched["entries"][0], expected[1]);
        let by_value = dispatch(
            "catalog",
            "show",
            &Options {
                id: Some("finiteState".into()),
                ..options
            },
        )
        .unwrap();
        assert_eq!(by_value["total"], 0, "a semantic kind is not a row id");
    }

    #[test]
    fn dictionary_catalog_glossary_rows_retain_ordered_text_ids() {
        let raw = catalog("glossary-text").unwrap();
        let (id, text_ids) = raw
            .as_object()
            .unwrap()
            .iter()
            .find(|(_, ids)| ids.as_array().unwrap().len() > 1)
            .unwrap();
        let options = catalog_options("glossary-text");
        let selected = dispatch(
            "catalog",
            "show",
            &Options {
                id: Some(id.to_uppercase()),
                ..options.clone()
            },
        )
        .unwrap();
        assert_eq!(selected["total"], 1);
        assert_eq!(selected["entries"][0]["id"], *id);
        assert_eq!(selected["entries"][0]["segmentClass"], *id);
        assert_eq!(selected["entries"][0]["textIds"], *text_ids);
        let query = text_ids[0].as_str().unwrap().to_lowercase();
        let expected = raw
            .as_object()
            .unwrap()
            .iter()
            .filter(|(_, ids)| ids.to_string().to_lowercase().contains(&query))
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        let searched = dispatch(
            "catalog",
            "search",
            &Options {
                query: Some(query.clone()),
                all: true,
                ..options.clone()
            },
        )
        .unwrap();
        assert_eq!(searched["total"], expected.len());
        assert_eq!(
            searched["entries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            expected
        );
        let by_text_id = dispatch(
            "catalog",
            "show",
            &Options {
                id: Some(query),
                ..options
            },
        )
        .unwrap();
        assert_eq!(by_text_id["total"], 0);
    }

    #[test]
    fn dictionary_catalog_portraits_preserve_both_sources_and_schema() {
        let raw = catalog("portraits").unwrap();
        let options = catalog_options("portraits");
        let page = dispatch(
            "catalog",
            "list",
            &Options {
                all: true,
                ..options.clone()
            },
        )
        .unwrap();
        assert_eq!(page["schema"], raw["schema"]);
        let rows = page["entries"].as_array().unwrap();
        let mut expected_total = 0;
        for source in ["images", "artwork"] {
            let entries = raw[source].as_object().unwrap();
            expected_total += entries.len();
            for (id, content) in entries {
                let row = rows
                    .iter()
                    .find(|row| row["id"] == *id && row["source"] == source)
                    .unwrap();
                for (key, value) in content.as_object().unwrap() {
                    assert_eq!(row[key], *value, "{source}/{id}/{key}");
                }
            }
        }
        assert_eq!(page["total"], expected_total);
        assert_eq!(rows.len(), expected_total);
        let selected = dispatch(
            "catalog",
            "show",
            &Options {
                id: Some("abandonedmine".into()),
                ..options.clone()
            },
        )
        .unwrap();
        assert_eq!(
            selected["total"], 1,
            "an image name is not its dictionary id"
        );
        assert_eq!(selected["entries"][0]["id"], "AbandonedMine");
        assert_eq!(selected["entries"][0]["source"], "artwork");
        let searched = dispatch(
            "catalog",
            "search",
            &Options {
                query: Some("AbandonedMine".into()),
                all: true,
                ..options
            },
        )
        .unwrap();
        assert_eq!(searched["total"], 2);
        assert_eq!(searched["schema"], raw["schema"]);
    }

    #[test]
    fn dictionary_catalog_attribute_groups_keep_hidden_metadata_out_of_rows() {
        let raw = catalog("hero-attributes").unwrap();
        let options = catalog_options("hero-attributes");
        let page = dispatch(
            "catalog",
            "list",
            &Options {
                all: true,
                ..options.clone()
            },
        )
        .unwrap();
        let groups = raw["groups"].as_object().unwrap();
        assert_eq!(page["hidden"], raw["hidden"]);
        assert_eq!(page["total"], groups.len());
        for row in page["entries"].as_array().unwrap() {
            assert_eq!(row["attributes"], groups[row["id"].as_str().unwrap()]);
        }
        let searched = dispatch(
            "catalog",
            "search",
            &Options {
                query: Some("Health".into()),
                offset: 1,
                ..options.clone()
            },
        )
        .unwrap();
        assert_eq!(searched["total"], 2);
        assert_eq!(searched["count"], 1);
        assert_eq!(searched["entries"][0]["id"], "sleep");
        assert_eq!(searched["hidden"], raw["hidden"]);
        let metadata_only = dispatch(
            "catalog",
            "search",
            &Options {
                query: Some("ToughnessA".into()),
                ..options
            },
        )
        .unwrap();
        assert_eq!(metadata_only["total"], 0);
        assert_eq!(metadata_only["hidden"], raw["hidden"]);
    }

    #[test]
    fn dictionary_catalog_pages_support_offsets_zero_limits_and_all() {
        for domain in [
            "story-semantics",
            "glossary-text",
            "portraits",
            "hero-attributes",
        ] {
            let options = catalog_options(domain);
            let whole = dispatch(
                "catalog",
                "list",
                &Options {
                    all: true,
                    ..options.clone()
                },
            )
            .unwrap();
            let rows = whole["entries"].as_array().unwrap();
            assert!(rows.len() > 1, "{domain}");
            let first = dispatch("catalog", "list", &options).unwrap();
            let by_source = dispatch(
                "catalog",
                "list",
                &Options {
                    kind: None,
                    source: Some(domain.into()),
                    ..options.clone()
                },
            )
            .unwrap();
            assert_eq!(by_source, first, "{domain}");
            let selected = dispatch(
                "catalog",
                "show",
                &Options {
                    id: Some(rows[0]["id"].as_str().unwrap().to_uppercase()),
                    ..options.clone()
                },
            )
            .unwrap();
            assert_eq!(selected["entries"], json!([rows[0]]), "{domain}");
            assert_eq!(selected["total"], 1, "{domain}");
            let second = dispatch(
                "catalog",
                "list",
                &Options {
                    offset: 1,
                    ..options.clone()
                },
            )
            .unwrap();
            assert_eq!(first["total"], rows.len(), "{domain}");
            assert_eq!(first["count"], 1, "{domain}");
            assert_eq!(first["entries"], json!([rows[0]]), "{domain}");
            assert_eq!(second["total"], rows.len(), "{domain}");
            assert_eq!(second["count"], 1, "{domain}");
            assert_eq!(second["offset"], 1, "{domain}");
            assert_eq!(second["entries"], json!([rows[1]]), "{domain}");
            for selected in [
                Options {
                    limit: 0,
                    ..options.clone()
                },
                Options {
                    offset: rows.len() + 1,
                    ..options.clone()
                },
            ] {
                let empty = dispatch("catalog", "list", &selected).unwrap();
                assert_eq!(empty["entries"], json!([]), "{domain}");
                assert_eq!(empty["total"], rows.len(), "{domain}");
                assert_eq!(empty["count"], 0, "{domain}");
                assert_eq!(empty["offset"], selected.offset, "{domain}");
            }
            let rest = dispatch(
                "catalog",
                "list",
                &Options {
                    all: true,
                    offset: 1,
                    ..options
                },
            )
            .unwrap();
            assert_eq!(rest["entries"], json!(rows[1..]), "{domain}");
            assert_eq!(rest["total"], rows.len(), "{domain}");
            assert_eq!(rest["count"], rows.len() - 1, "{domain}");
        }
    }

    #[test]
    fn catalog_pages_preserve_existing_array_and_wrapper_metadata() {
        for (domain, key, metadata) in [
            ("npc", "entries", &[][..]),
            ("knowledge", "entries", &[][..]),
            ("glossary", "entries", &[][..]),
            ("locations", "spots", &["version", "areas"][..]),
            ("locks", "locks", &["version"][..]),
            ("item-stats", "entries", &["schema", "filters"][..]),
        ] {
            let options = catalog_options(domain);
            let mut raw = catalog(domain).unwrap();
            // Shared metadata is localized too when a game-text cache exists.
            super::super::text::Texts::load_options(&options)
                .unwrap()
                .apply(&mut raw)
                .unwrap();
            let page = dispatch("catalog", "list", &options).unwrap();
            let expected = if raw.is_array() {
                raw.as_array().unwrap().len()
            } else if domain == "item-stats" {
                raw["items"].as_object().unwrap().len()
            } else {
                raw[key].as_array().unwrap().len()
            };
            assert_eq!(page["total"], expected, "{domain}");
            assert_eq!(page["count"], 1, "{domain}");
            assert_eq!(page[key].as_array().unwrap().len(), 1, "{domain}");
            for field in metadata {
                assert_eq!(page[*field], raw[*field], "{domain}/{field}");
            }
        }
        let raw = catalog("ui-texts").unwrap();
        let page = dispatch("catalog", "list", &catalog_options("ui-texts")).unwrap();
        assert_eq!(page["total"], raw["en"].as_object().unwrap().len());
        assert_eq!(page["count"], 1);
        let row = &page["entries"][0];
        assert_eq!(row["text"], raw["en"][row["id"].as_str().unwrap()]);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn dictionary_catalog_localizes_before_search_and_pagination() {
        const CHILD: &str = "GORE_TEST_DICTIONARY_LOCALIZATION_CHILD";
        const TEST: &str =
            "cmd::save::display::tests::dictionary_catalog_localizes_before_search_and_pagination";
        let raw = catalog("glossary-text").unwrap();
        let segments = raw.as_object().unwrap().iter().take(2).collect::<Vec<_>>();
        if std::env::var_os(CHILD).is_none() {
            // A child harness isolates its localization cache without mutating the
            // environment or extracted game texts used by concurrent unit tests.
            let temp = tempfile::tempdir().unwrap();
            let mut localized = json!({});
            for (_, ids) in &segments {
                for id in ids.as_array().unwrap() {
                    localized[id.as_str().unwrap().to_lowercase()] =
                        json!({"german":"PR123 Übersetzter Glossartreffer"});
                }
            }
            fs::create_dir(temp.path().join("gore")).unwrap();
            fs::write(
                temp.path().join("gore/loc_catalog.json"),
                serde_json::to_vec(&localized).unwrap(),
            )
            .unwrap();
            let output = Command::new(std::env::current_exe().unwrap())
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
        let options = Options {
            query: Some("pr123 übersetzter glossartreffer".into()),
            game_lang: "de".into(),
            offset: 1,
            ..catalog_options("glossary-text")
        };
        let page = dispatch("catalog", "search", &options).unwrap();
        assert_eq!(page["total"], 2);
        assert_eq!(page["count"], 1);
        assert_eq!(page["offset"], 1);
        assert_eq!(page["entries"][0]["id"], *segments[1].0);
        assert_eq!(page["entries"][0]["textIds"], *segments[1].1);
        assert_eq!(
            page["entries"][0]["textSegments"][0]["text"],
            "PR123 Übersetzter Glossartreffer"
        );
    }

    #[test]
    fn item_stat_catalog_supports_selection_search_and_pagination() {
        let options = Options {
            kind: Some("item-stats".into()),
            lang: "en".into(),
            game_lang: "en".into(),
            limit: 1,
            ..Options::default()
        };
        let empty = dispatch(
            "catalog",
            "search",
            &Options {
                query: Some("definitely_does_not_exist".into()),
                ..options.clone()
            },
        )
        .unwrap();
        assert_eq!(empty["total"], 0);
        let selected = dispatch(
            "catalog",
            "show",
            &Options {
                id: Some("ItMi_Orenugget".into()),
                ..options.clone()
            },
        )
        .unwrap();
        assert_eq!(selected["total"], 1);
        assert_eq!(selected["entries"][0]["id"], "ItMi_Orenugget");
        assert_eq!(
            selected["entries"][0]["maxStack"],
            catalog("item-stats").unwrap()["items"]["ItMi_Orenugget"]["maxStack"]
        );
        let first = dispatch("catalog", "list", &options).unwrap();
        let second = dispatch(
            "catalog",
            "list",
            &Options {
                offset: 1,
                ..options
            },
        )
        .unwrap();
        assert_eq!(first["total"], 867);
        assert_eq!(first["count"], 1);
        assert_eq!(second["total"], first["total"]);
        assert_ne!(first["entries"][0]["id"], second["entries"][0]["id"]);
    }

    #[test]
    fn lock_catalog_selection_is_separate_from_lock_kind_filter() {
        let options = Options {
            all: true,
            lang: "en".into(),
            game_lang: "en".into(),
            ..Options::default()
        };
        let source = dispatch(
            "catalog",
            "list",
            &Options {
                source: Some("locks".into()),
                ..options.clone()
            },
        )
        .unwrap();
        assert!(source["total"].as_u64().unwrap() > 0);
        let kind = dispatch(
            "catalog",
            "list",
            &Options {
                kind: Some("locks".into()),
                ..options.clone()
            },
        )
        .unwrap();
        assert_eq!(kind, source);
        for lock_kind in ["chest", "door"] {
            let filtered = dispatch(
                "catalog",
                "list",
                &Options {
                    source: Some("locks".into()),
                    kind: Some(lock_kind.into()),
                    ..options.clone()
                },
            )
            .unwrap();
            let expected = source["locks"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| {
                    row["k"]
                        .as_str()
                        .is_some_and(|kind| kind.eq_ignore_ascii_case(lock_kind))
                })
                .count();
            assert!(expected > 0);
            assert_eq!(filtered["total"], expected);
        }
    }

    #[test]
    fn catalog_items_use_editor_inventory_categories() {
        for (id, category) in [
            ("ItMi_Orenugget", "material"),
            ("ItAr_Rune_FireBall", "magic"),
            ("ItAm_Arrow", "rangedWeapon"),
            ("ItKe_Lockpick", "misc"),
            ("ItMi_Smith_1H_Axe_01", "material"),
            ("ItMw_2H_Mace_Orc_01_vOrc", "other"),
        ] {
            let data = dispatch(
                "catalog",
                "search",
                &Options {
                    kind: Some("items".into()),
                    id: Some(id.into()),
                    category: Some(category.to_uppercase()),
                    lang: "en".into(),
                    game_lang: "en".into(),
                    limit: 1,
                    ..Options::default()
                },
            )
            .unwrap();
            assert_eq!(data["total"], 1, "{id} belongs to {category}");
            assert_eq!(data["entries"][0]["id"], id);
            assert_eq!(data["entries"][0]["category"], category);
        }
    }

    #[test]
    fn item_categories_use_case_insensitive_stats_and_native_fallbacks() {
        let mut rows = vec![
            json!({"id":" itmi_orenugget ","category":"misc"}),
            json!({"id":"ItMw_NewWeapon"}),
            json!({"id":"ItAr_Scroll_NewSpell"}),
            json!({"id":"ItFo_Potion_NewPotion"}),
            json!({"id":"ItFo_BoozeNewDrink"}),
            json!({"id":"ItAt_NewTrophy"}),
            json!({"id":"Org_Armor_NewPiece"}),
            json!({"id":"UnknownItem"}),
        ];
        annotate_items(&mut rows, false).unwrap();
        assert_eq!(
            rows.iter()
                .map(|row| row["category"].as_str().unwrap())
                .collect::<Vec<_>>(),
            [
                "material",
                "meleeWeapon",
                "magic",
                "potion",
                "potion",
                "material",
                "wearable",
                "other"
            ]
        );
        assert!(rows.iter().all(|row| row.get("stats").is_none()));
    }

    #[test]
    fn item_categories_prioritize_properties_and_match_whole_tag_segments() {
        let filters = json!([
            {"id":"G1R_All","itemTags":[]},
            {"id":"G1R_MeleeWeapons","itemTags":["Item_Weapon_Sword"]},
            {"id":"G1R_Magic","itemTags":["Item_Weapon_Rune"]},
            {"id":"G1R_Materials","itemTags":["Item_Property_Forge"]}
        ]);
        let filters = filters.as_array().unwrap().iter().collect::<Vec<_>>();
        for (stats, expected) in [
            (
                json!({"itemType":"Item_Weapon_Sword_OneHand","specs":["Item_Property_Forge_Head"]}),
                "material",
            ),
            (json!({"itemType":"Item_Weapon_Rune_FireBall"}), "magic"),
            (json!({"itemType":"Item_Weapon_RuneFake"}), "other"),
            (json!({}), "other"),
        ] {
            assert_eq!(item_category("ItMw_KnownItem", &stats, &filters), expected);
        }
    }

    #[test]
    fn export_previews_require_a_valid_parent_and_file_target() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = temp.path().join("source");
        fs::write(&source_path, b"original source").unwrap();
        let source = fs::File::open(&source_path).unwrap();
        let directory = temp.path().join("directory");
        fs::create_dir(&directory).unwrap();
        for out in [
            temp.path().join("missing/child"),
            source_path.join("child"),
            directory,
        ] {
            for dry in [false, true] {
                assert!(export_output(&source, &out, dry, "fixture").is_err());
            }
        }
        let fresh = temp.path().join("fresh");
        assert!(
            export_output(&source, &fresh, true, "fixture")
                .unwrap()
                .is_none()
        );
        assert!(!fresh.exists());
        let mut output = export_output(&source, &fresh, false, "fixture")
            .unwrap()
            .unwrap();
        output.write_all(b"exported").unwrap();
        assert_eq!(fs::read(fresh).unwrap(), b"exported");
        assert_eq!(fs::read(source_path).unwrap(), b"original source");
        assert!(!temp.path().join("missing").exists());
    }

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
