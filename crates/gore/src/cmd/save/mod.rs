//! Save Editor operations exposed as native, scriptable CLI commands.
use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use gore_save::api::{self, Request};
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

mod administration;
mod display;
mod presentation;
mod report;
mod text;
mod updates;

/// Options are shared so an operation staged in a draft has the same arguments
/// and selection rules as its immediate counterpart.
#[derive(Debug, Clone, Args, Default)]
pub struct Options {
    /// Save file, or draft file for draft commands
    pub save: Option<PathBuf>,
    /// Snapshot taken before save-derived selectors and values are read.
    #[arg(skip)]
    pub inspected_sha1: Option<String>,
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub root: Option<PathBuf>,
    /// Internal zero-based profile ID
    #[arg(long)]
    pub profile: Option<i32>,
    #[arg(long)]
    pub other: bool,
    #[arg(long, default_value = "hero")]
    pub actor: String,
    #[arg(long)]
    pub id: Option<String>,
    #[arg(long)]
    pub item: Option<String>,
    #[arg(long)]
    pub count: Option<i32>,
    #[arg(long)]
    pub container: Option<String>,
    #[arg(long)]
    pub slot: Option<String>,
    #[arg(long)]
    pub index: Option<usize>,
    #[arg(long, default_value = "current")]
    pub map: String,
    #[arg(long)]
    pub query: Option<String>,
    #[arg(long)]
    pub state: Option<String>,
    #[arg(long)]
    pub group: Option<String>,
    #[arg(long)]
    pub category: Option<String>,
    #[arg(long)]
    pub kind: Option<String>,
    #[arg(long)]
    pub role: Option<String>,
    #[arg(long)]
    pub source: Option<String>,
    #[arg(long = "type")]
    pub type_filter: Option<String>,
    #[arg(long, action=clap::ArgAction::Set)]
    pub editable: Option<bool>,
    #[arg(long)]
    pub all: bool,
    #[arg(long, default_value_t = 0)]
    pub offset: usize,
    #[arg(long, default_value_t = 100)]
    pub limit: usize,
    #[arg(long)]
    pub include_unset: bool,
    #[arg(long)]
    pub semantic_type: Option<String>,
    #[arg(long)]
    pub set_class: Option<String>,
    #[arg(long)]
    pub attribute: Option<String>,
    #[arg(long, default_value = "current")]
    pub field: String,
    #[arg(long, allow_negative_numbers = true)]
    pub base: Option<f64>,
    #[arg(long, allow_negative_numbers = true)]
    pub current: Option<f64>,
    #[arg(long)]
    pub skill: Option<String>,
    #[arg(long)]
    pub tier: Option<String>,
    #[arg(long)]
    pub relationship: Option<String>,
    #[arg(long)]
    pub location: Option<String>,
    #[arg(long)]
    pub apply_facing: bool,
    #[arg(long)]
    pub stay: bool,
    #[arg(long, allow_negative_numbers = true)]
    pub x: Option<f64>,
    #[arg(long, allow_negative_numbers = true)]
    pub y: Option<f64>,
    #[arg(long, allow_negative_numbers = true)]
    pub z: Option<f64>,
    #[arg(long, allow_negative_numbers = true)]
    pub pitch: Option<f64>,
    #[arg(long, allow_negative_numbers = true)]
    pub yaw: Option<f64>,
    #[arg(long, allow_negative_numbers = true)]
    pub roll: Option<f64>,
    #[arg(long, allow_negative_numbers = true)]
    pub seconds: Option<f64>,
    #[arg(long)]
    pub day: Option<u32>,
    #[arg(long)]
    pub hour: Option<u8>,
    #[arg(long)]
    pub minute: Option<u8>,
    #[arg(long)]
    pub second: Option<u8>,
    #[arg(long)]
    pub preset: Option<String>,
    #[arg(long)]
    pub combat: Option<String>,
    #[arg(long)]
    pub resources: Option<String>,
    #[arg(long)]
    pub progression: Option<String>,
    #[arg(long, action=clap::ArgAction::Set)]
    pub flow_helper: Option<bool>,
    #[arg(long, action=clap::ArgAction::Set)]
    pub permadeath: Option<bool>,
    #[arg(long)]
    pub resources_level: Option<String>,
    #[arg(long)]
    pub document: Option<String>,
    #[arg(long)]
    pub segment: Option<String>,
    #[arg(long)]
    pub entry: Option<String>,
    #[arg(long)]
    pub character: Option<String>,
    #[arg(long)]
    pub guild: Option<String>,
    #[arg(long)]
    pub lock: Option<String>,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub clear_name: bool,
    #[arg(long)]
    pub path_file: Option<PathBuf>,
    #[arg(long, allow_hyphen_values = true)]
    pub value_json: Option<String>,
    #[arg(long)]
    pub request_file: Option<PathBuf>,
    #[arg(long)]
    pub payload_file: Option<PathBuf>,
    #[arg(long, allow_hyphen_values = true)]
    pub expect: Option<String>,
    #[arg(long)]
    pub allow_unknown_create: bool,
    #[arg(long)]
    pub backup: Option<PathBuf>,
    #[arg(long)]
    pub include_companions: bool,
    #[arg(long)]
    pub target: Option<PathBuf>,
    #[arg(long)]
    pub out: Option<PathBuf>,
    #[arg(long)]
    pub draft: Option<PathBuf>,
    #[arg(long)]
    pub operation: Option<usize>,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub yes: bool,
    #[arg(long)]
    pub private: bool,
    #[arg(long)]
    pub private_chunk_limit: Option<usize>,
    #[arg(long)]
    pub details: bool,
    #[arg(long)]
    pub with_assets: bool,
    #[arg(long)]
    pub game: Option<PathBuf>,
    #[arg(long)]
    pub manifest: Option<PathBuf>,
    #[arg(long)]
    pub lcache: Option<PathBuf>,
    #[arg(long, default_value = "auto")]
    pub lang: String,
    /// Game text language, independently of the interface language
    #[arg(long, default_value = "auto")]
    pub game_lang: String,
    #[arg(long)]
    pub show_ids: bool,
    #[arg(long)]
    pub copy: bool,
    #[arg(long)]
    pub open: bool,
    #[arg(long, default_value = "editor")]
    pub scope: String,
    #[arg(long)]
    pub key: Option<String>,
    #[arg(long, allow_hyphen_values = true)]
    pub value: Option<String>,
    #[arg(long, default_value = "html")]
    pub format: String,
    #[arg(long, default_value = "save-editor")]
    pub product: String,
    #[arg(long)]
    pub roundtrip: bool,
    #[arg(long)]
    pub codec_roundtrip: bool,
}

macro_rules! domain {
    ($name:ident { $($variant:ident => $command:literal),+ $(,)? }) => {
        #[derive(Debug, Subcommand)] pub enum $name { $(#[command(name=$command)] $variant(Options)),+ }
        impl $name { fn parts(self) -> (&'static str, Options) { match self { $(Self::$variant(o) => ($command,o)),+ } } }
    }
}
domain!(Profiles { List=>"list", Show=>"show" });
domain!(Profile { Assign=>"assign", Detach=>"detach" });
domain!(Characters { List=>"list", Show=>"show" });
domain!(Attributes { List=>"list", Show=>"show", Set=>"set" });
domain!(Skills { List=>"list", Show=>"show", Set=>"set" });
domain!(Inventory { List=>"list", Show=>"show", SetCount=>"set-count", Add=>"add", Remove=>"remove", Reset=>"reset", CheckSlots=>"check-slots", RepairSlots=>"repair-slots" });
domain!(Position { Show=>"show", Set=>"set", ResetToSpawn=>"reset-to-spawn", PinStatus=>"pin-status", ResumeRoutine=>"resume-routine", Undo=>"undo" });
domain!(Time { Show=>"show", Set=>"set" });
domain!(Difficulty { Show=>"show", Set=>"set" });
domain!(Quests { List=>"list", Show=>"show", SetState=>"set-state" });
domain!(Story { List=>"list", Show=>"show", Set=>"set", Unset=>"unset" });
domain!(Segment { Unlock=>"unlock", Lock=>"lock" });
domain!(Knowledge { List=>"list", Add=>"add", Remove=>"remove", CreateCharacter=>"create-character" });
domain!(Events { List=>"list", Show=>"show", Remove=>"remove", Duplicate=>"duplicate" });
domain!(Factions { List=>"list", Show=>"show", Forgive=>"forgive" });
domain!(Locks { List=>"list", Show=>"show", Unlock=>"unlock", Lock=>"lock" });
domain!(Stock { Set=>"set", Add=>"add", Remove=>"remove" });
domain!(Timing { Show=>"show", Set=>"set", MakeDue=>"make-due" });
domain!(Backups { List=>"list", Rename=>"rename", Restore=>"restore", Delete=>"delete" });
domain!(Recovery { List=>"list", Show=>"show", Restore=>"restore", Dismiss=>"dismiss", Repair=>"repair" });
domain!(Library { List=>"list", Add=>"add", Remove=>"remove", Hide=>"hide", Unhide=>"unhide" });
domain!(Settings { Show=>"show", Get=>"get", Set=>"set", Reset=>"reset" });
domain!(Localization { Status=>"status", Prepare=>"prepare", Find=>"find" });
domain!(Assets { Prepare=>"prepare", Status=>"status", List=>"list", Export=>"export", Open=>"open", Release=>"release" });
domain!(Data { Search=>"search", Show=>"show", Set=>"set", SetAdd=>"set-add", SetRemove=>"set-remove", ArrayRemove=>"array-remove", ArrayDuplicate=>"array-duplicate" });
domain!(Codec { Status=>"status" });
domain!(Core { Exec=>"exec", Capabilities=>"capabilities" });
domain!(Draft { Create=>"create", Show=>"show", Stage=>"stage", Remove=>"remove", Reset=>"reset", Validate=>"validate", Apply=>"apply" });
domain!(Catalog { List=>"list", Show=>"show", Search=>"search" });
domain!(Screenshot { Export=>"export" });
domain!(Updates { Check=>"check", OpenRelease=>"open-release", Install=>"install" });
domain!(Relationship { Show=>"show", Set=>"set" });

#[derive(Debug, Subcommand)]
pub enum Npc {
    List(Options),
    Show(Options),
    Revive(Options),
    #[command(subcommand)]
    Relationship(Relationship),
}
#[derive(Debug, Subcommand)]
pub enum Glossary {
    List(Options),
    Show(Options),
    SetState(Options),
    #[command(subcommand)]
    Segment(Segment),
}
#[derive(Debug, Subcommand)]
pub enum Traders {
    List(Options),
    Show(Options),
    #[command(subcommand)]
    Stock(Stock),
    #[command(subcommand)]
    Timing(Timing),
}
#[derive(Debug, Subcommand)]
pub enum SaveAction {
    List(Options),
    Inspect(Options),
    Refresh(Options),
    Rename(Options),
    Import(Options),
    Delete(Options),
    Overview(Options),
    Statistics(Options),
    Report(Options),
    About(Options),
    Licenses(Options),
    Validate(Options),
    #[command(subcommand)]
    Profiles(Profiles),
    #[command(subcommand)]
    Profile(Profile),
    #[command(subcommand)]
    Characters(Characters),
    #[command(subcommand)]
    Npc(Npc),
    #[command(subcommand)]
    Attributes(Attributes),
    #[command(subcommand)]
    Skills(Skills),
    #[command(subcommand)]
    Inventory(Inventory),
    #[command(subcommand)]
    Position(Position),
    #[command(subcommand)]
    Time(Time),
    #[command(subcommand)]
    Difficulty(Difficulty),
    #[command(subcommand)]
    Quests(Quests),
    #[command(subcommand)]
    Tutorials(Quests),
    #[command(subcommand)]
    Story(Story),
    #[command(subcommand)]
    Glossary(Glossary),
    #[command(subcommand)]
    Knowledge(Knowledge),
    #[command(subcommand)]
    Events(Events),
    #[command(subcommand)]
    Factions(Factions),
    #[command(subcommand)]
    Locks(Locks),
    #[command(subcommand)]
    Traders(Traders),
    #[command(subcommand)]
    Backups(Backups),
    #[command(subcommand)]
    Recovery(Recovery),
    #[command(subcommand)]
    Library(Library),
    #[command(subcommand)]
    Settings(Settings),
    #[command(subcommand)]
    Localization(Localization),
    #[command(subcommand)]
    Assets(Assets),
    #[command(subcommand)]
    Data(Data),
    #[command(subcommand)]
    Codec(Codec),
    #[command(subcommand)]
    Core(Core),
    #[command(subcommand)]
    Draft(Draft),
    #[command(subcommand)]
    Catalog(Catalog),
    #[command(subcommand)]
    Items(Catalog),
    #[command(subcommand)]
    Locations(Catalog),
    #[command(subcommand)]
    Screenshot(Screenshot),
    #[command(subcommand)]
    Updates(Updates),
}

impl SaveAction {
    fn parts(self) -> (&'static str, &'static str, Options) {
        use SaveAction::*;
        macro_rules! p {
            ($group:literal,$a:expr) => {{
                let (v, o) = $a.parts();
                ($group, v, o)
            }};
        }
        match self {
            List(o) => ("", "list", o),
            Inspect(o) => ("", "inspect", o),
            Refresh(o) => ("", "refresh", o),
            Rename(o) => ("", "rename", o),
            Import(o) => ("", "import", o),
            Delete(o) => ("", "delete", o),
            Overview(o) => ("", "overview", o),
            Statistics(o) => ("", "statistics", o),
            Report(o) => ("", "report", o),
            About(o) => ("", "about", o),
            Licenses(o) => ("", "licenses", o),
            Validate(o) => ("", "validate", o),
            Profiles(a) => p!("profiles", a),
            Profile(a) => p!("profile", a),
            Characters(a) => p!("characters", a),
            Attributes(a) => p!("attributes", a),
            Skills(a) => p!("skills", a),
            Inventory(a) => p!("inventory", a),
            Position(a) => p!("position", a),
            Time(a) => p!("time", a),
            Difficulty(a) => p!("difficulty", a),
            Quests(a) => p!("quests", a),
            Tutorials(a) => p!("tutorials", a),
            Story(a) => p!("story", a),
            Knowledge(a) => p!("knowledge", a),
            Events(a) => p!("events", a),
            Factions(a) => p!("factions", a),
            Locks(a) => p!("locks", a),
            Backups(a) => p!("backups", a),
            Recovery(a) => p!("recovery", a),
            Library(a) => p!("library", a),
            Settings(a) => p!("settings", a),
            Localization(a) => p!("localization", a),
            Assets(a) => p!("assets", a),
            Data(a) => p!("data", a),
            Codec(a) => p!("codec", a),
            Core(a) => p!("core", a),
            Draft(a) => p!("draft", a),
            Catalog(a) => p!("catalog", a),
            Items(a) => p!("items", a),
            Locations(a) => p!("locations", a),
            Screenshot(a) => p!("screenshot", a),
            Updates(a) => p!("updates", a),
            Npc(a) => match a {
                self::Npc::List(o) => ("npc", "list", o),
                self::Npc::Show(o) => ("npc", "show", o),
                self::Npc::Revive(o) => ("npc", "revive", o),
                self::Npc::Relationship(a) => p!("relationship", a),
            },
            Glossary(a) => match a {
                self::Glossary::List(o) => ("glossary", "list", o),
                self::Glossary::Show(o) => ("glossary", "show", o),
                self::Glossary::SetState(o) => ("glossary", "set-state", o),
                self::Glossary::Segment(a) => p!("segment", a),
            },
            Traders(a) => match a {
                self::Traders::List(o) => ("traders", "list", o),
                self::Traders::Show(o) => ("traders", "show", o),
                self::Traders::Stock(a) => p!("stock", a),
                self::Traders::Timing(a) => p!("timing", a),
            },
        }
    }
}

pub fn run(action: SaveAction) -> Result<()> {
    let (group, verb, mut o) = action.parts();
    let automatic_ui = o.lang == "auto";
    let preferences = administration::settings_read("ui").unwrap_or_default();
    if o.lang == "auto" {
        o.lang = preferences["appLocale"]
            .as_str()
            .unwrap_or("en")
            .to_string();
    }
    if o.game_lang == "auto" {
        o.game_lang = if automatic_ui {
            preferences["gameTextLocale"]
                .as_str()
                .filter(|s| !s.trim().is_empty())
        } else {
            None
        }
        .unwrap_or_else(|| text::default_game_language(&o.lang))
        .to_string();
    }
    let result = dispatch(group, verb, &o);
    match result {
        Ok(data) => {
            if o.json {
                let mut response = json!({"ok":data["complete"]!=false,"data":data});
                if data["complete"] == false {
                    response["error"] = json!({"code":"PARTIAL_APPLY","message":data["error"]});
                }
                println!("{response}");
                if o.copy && !o.dry_run {
                    if let Err(error) = display::clipboard(&response.to_string()) {
                        eprintln!("Clipboard copy failed: {error}");
                    }
                }
            } else {
                display::print(&data, &o)?;
            }
            if data["complete"] == false {
                bail!("save partially applied; inspect committed and remaining operations");
            }
            Ok(())
        }
        Err(error) => {
            if o.json {
                println!(
                    "{}",
                    json!({"ok":false,"error":error.downcast_ref::<gore_save::CoreError>().map(api::error_details).unwrap_or_else(||json!({"code":"INVALID_REQUEST","message":error.to_string()}))})
                );
            }
            Err(error)
        }
    }
}
pub(super) fn call(command: &str, payload: Value) -> Result<Value> {
    if command == "apply_edits" {
        return Ok(gore_save::workflow::apply_with_progress(
            &payload,
            |progress| {
                eprintln!(
                    "Save {}/{}: {} operations committed",
                    progress["step"],
                    progress["steps"],
                    progress["committed"].as_array().map(Vec::len).unwrap_or(0)
                )
            },
        )?);
    }
    api::execute(&Request {
        command: command.into(),
        payload,
    })
    .map_err(Into::into)
}
pub(super) fn required<'a>(value: &'a Option<String>, name: &str) -> Result<&'a str> {
    value
        .as_deref()
        .with_context(|| format!("--{name} is required"))
}
pub(super) fn save(o: &Options) -> Result<&Path> {
    o.save.as_deref().context("a save file is required")
}
pub(super) fn read_json(path: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(
        &fs::read(path).with_context(|| format!("reading {}", path.display()))?,
    )?)
}
fn value(o: &Options) -> Result<Value> {
    serde_json::from_str(required(&o.value_json, "value-json")?).context("invalid --value-json")
}
pub(super) fn payload(o: &Options) -> Result<Value> {
    let mut p = if let Some(file) = &o.payload_file {
        read_json(file)?
    } else {
        json!({})
    };
    if !p.is_object() {
        bail!("payload must be a JSON object");
    }
    macro_rules! insert {
        ($field:ident,$wire:literal) => {
            if let Some(v) = &o.$field {
                p[$wire] = json!(v);
            }
        };
    }
    insert!(save, "path");
    if o.save.is_none() {
        insert!(root, "path");
    }
    insert!(profile, "profileId");
    insert!(id, "id");
    insert!(query, "query");
    insert!(state, "state");
    insert!(group, "group");
    insert!(category, "category");
    insert!(kind, "kind");
    insert!(source, "source");
    insert!(type_filter, "type");
    insert!(editable, "editable");
    insert!(semantic_type, "semanticType");
    insert!(character, "character");
    insert!(index, "index");
    insert!(private_chunk_limit, "privateChunkLimit");
    insert!(game, "gamePath");
    insert!(lcache, "lcache");
    p["offset"] = json!(o.offset);
    p["limit"] = json!(o.limit);
    p["includeUnset"] = json!(o.include_unset);
    p["includePrivate"] = json!(o.private || o.details);
    p["actor"] = json!(if o.actor.eq_ignore_ascii_case("hero") {
        "Hero"
    } else {
        &o.actor
    });
    Ok(p)
}
pub(super) fn paged(command: &str, mut p: Value, o: &Options) -> Result<Value> {
    let mut result = call(command, p.clone())?;
    if !o.all {
        return Ok(result);
    }
    if let Some(mut categories) = result["categories"].as_array().cloned() {
        let count_entries = |categories: &[Value]| {
            categories
                .iter()
                .map(|category| category["entries"].as_array().map_or(0, Vec::len))
                .sum::<usize>()
        };
        let mut count = count_entries(&categories);
        let mut offset = o.offset;
        loop {
            offset += count;
            if count == 0 || offset >= result["total"].as_u64().unwrap_or(offset as u64) as usize {
                break;
            }
            p["offset"] = json!(offset);
            let page = call(command, p.clone())?;
            let incoming = page["categories"]
                .as_array()
                .context("paged categories are not an array")?;
            count = count_entries(incoming);
            for category in incoming {
                if let Some(existing) = categories.iter_mut().find(|c| c["id"] == category["id"]) {
                    existing["entries"]
                        .as_array_mut()
                        .context("category entries are not an array")?
                        .extend(
                            category["entries"]
                                .as_array()
                                .context("category entries are not an array")?
                                .iter()
                                .cloned(),
                        );
                } else {
                    categories.push(category.clone());
                }
            }
        }
        let count = count_entries(&categories);
        result["categories"] = json!(categories);
        result["offset"] = json!(o.offset);
        result["limit"] = json!(count);
        result["count"] = json!(count);
        return Ok(result);
    }
    let key = ["results", "quests", "npcs", "entries", "events", "values"]
        .into_iter()
        .find(|k| result[*k].is_array());
    let Some(key) = key else { return Ok(result) };
    let mut offset = o.offset;
    let mut rows = Vec::new();
    loop {
        let page = result[key]
            .as_array()
            .context("paged result is not an array")?;
        let count = page.len();
        rows.extend(page.iter().cloned());
        offset += count;
        if count == 0 || offset >= result["total"].as_u64().unwrap_or(offset as u64) as usize {
            break;
        }
        p["offset"] = json!(offset);
        result = call(command, p.clone())?;
    }
    result[key] = json!(rows);
    result["offset"] = json!(o.offset);
    result["limit"] = json!(rows.len());
    result["count"] = json!(rows.len());
    Ok(result)
}
fn progression(section: &str, o: &Options) -> Result<Value> {
    let mut p = payload(o)?;
    p["section"] = json!(section);
    if matches!(section, "knowledge" | "events") && p["character"].is_null() {
        p["character"] = json!(if section == "events" {
            actor_row(o)?["globalId"]
                .as_str()
                .context("character has no saved GlobalId")?
                .to_string()
        } else {
            character(o)?
        });
    }
    let knowledge_query = p["query"].as_str().map(str::to_owned);
    let mut data = if section == "knowledge" {
        // Knowledge categories and localized labels exist only after enrichment.
        // Read every core page before filtering and slicing the display rows.
        p.as_object_mut().unwrap().remove("query");
        p["offset"] = json!(0);
        p["limit"] = json!(1000);
        paged(
            "query_progression",
            p,
            &Options {
                all: true,
                offset: 0,
                limit: 1000,
                ..o.clone()
            },
        )?
    } else {
        paged("query_progression", p, o)?
    };
    if section == "story" {
        presentation::annotate_story(&mut data);
    }
    display::localize(&mut data, o)?;
    if section == "knowledge" {
        display::filter(
            &mut data,
            "entries",
            &Options {
                query: knowledge_query,
                ..o.clone()
            },
        );
        display::paginate(&mut data, "entries", o);
        data["limit"] = if o.all {
            data["count"].clone()
        } else {
            json!(o.limit)
        };
    }
    if section == "glossary" && o.with_assets {
        display::attach_artwork(&mut data, o);
    }
    Ok(data)
}
pub(super) fn actor_row(o: &Options) -> Result<Value> {
    let index = call("private.characters.list", json!({"path":save(o)?}))?;
    let rows = index["characters"]
        .as_array()
        .context("character index unavailable")?;
    let matches: Vec<_> = rows
        .iter()
        .filter(|r| {
            r["globalId"]
                .as_str()
                .is_some_and(|id| id.eq_ignore_ascii_case(&o.actor))
                || r["uniqueName"]
                    .as_str()
                    .is_some_and(|id| id.eq_ignore_ascii_case(&o.actor))
        })
        .collect();
    if matches.len() != 1 {
        bail!("actor must resolve to exactly one saved character; use its GlobalId");
    }
    Ok(matches[0].clone())
}
pub(super) fn character(o: &Options) -> Result<String> {
    if let Some(c) = &o.character {
        return Ok(c.clone());
    }
    Ok(actor_row(o)?["uniqueName"]
        .as_str()
        .context("character has no knowledge key")?
        .into())
}
pub(super) fn npc_id(o: &Options) -> Result<String> {
    Ok(actor_row(o)?["globalId"]
        .as_str()
        .context("character has no saved GlobalId")?
        .into())
}
fn typed_path(o: &Options) -> Result<Value> {
    let p = read_json(
        o.path_file
            .as_deref()
            .context("--path-file is required (JSON path segments)")?,
    )?;
    if !p.is_array() || p.as_array().unwrap().iter().any(|s| !s.is_string()) {
        bail!("path must be a JSON array of strings");
    }
    Ok(p)
}
pub(super) fn edit(path: &str, value: Value) -> Value {
    json!({"path":path,"value":value})
}
pub(super) fn write(o: &Options, edits: Vec<Value>, extras: Value) -> Result<Value> {
    let mut p = extras;
    p["path"] = json!(save(o)?);
    p["edits"] = json!(edits);
    p["dryRun"] = json!(o.dry_run);
    if let Some(hash) = &o.inspected_sha1 {
        if p["expectedSha1"]
            .as_str()
            .is_some_and(|expected| expected != hash)
        {
            bail!("save snapshot differs from the requested expectedSha1");
        }
        p["expectedSha1"] = json!(hash);
    }
    if let Some(out) = &o.out {
        p["outputPath"] = json!(out)
    }
    if let Some(draft) = &o.draft {
        return administration::stage(draft, &p, o.dry_run);
    }
    call("apply_edits", p)
}

fn capture_save_snapshot(o: &Options) -> Result<Options> {
    let mut guarded = o.clone();
    if guarded.inspected_sha1.is_none() {
        if let Some(path) = &guarded.save {
            guarded.inspected_sha1 = Some(api::file_sha1(path)?);
        }
    }
    Ok(guarded)
}

fn dispatch(g: &str, v: &str, o: &Options) -> Result<Value> {
    if matches!(
        g,
        "settings"
            | "library"
            | "draft"
            | "profiles"
            | "profile"
            | "backups"
            | "recovery"
            | "updates"
    ) || (g.is_empty() && matches!(v, "import" | "delete" | "list" | "refresh"))
    {
        return administration::dispatch(g, v, o);
    }
    let guarded = capture_save_snapshot(o)?;
    let o = &guarded;
    if matches!(
        g,
        "catalog" | "items" | "locations" | "assets" | "screenshot" | "localization"
    ) || (g.is_empty()
        && matches!(
            v,
            "overview" | "statistics" | "report" | "about" | "licenses"
        ))
    {
        return display::dispatch(g, v, o);
    }
    match (g, v) {
        ("", "inspect") => call("inspect_save", payload(o)?),
        ("", "rename") => write(
            o,
            vec![edit(
                "public.m_PlayerSaveName",
                json!(required(&o.name, "name")?),
            )],
            json!({"syncPersistentDataList":o.out.is_none()}),
        ),
        ("", "validate") => call(
            if o.codec_roundtrip {
                "validate_codec_roundtrip"
            } else {
                "validate_roundtrip"
            },
            payload(o)?,
        ),
        ("codec", "status") => call("check_codec", json!({})),
        ("core", "capabilities") => Ok(api::capabilities()),
        ("core", "exec") => {
            let request = if let Some(file) = &o.request_file {
                read_json(file)?
            } else {
                let mut s = String::new();
                io::stdin().read_to_string(&mut s)?;
                serde_json::from_str(&s)?
            };
            let request: Request = serde_json::from_value(request)?;
            if o.dry_run && api::mutates(&request.command) {
                bail!(
                    "raw mutating requests require a domain dry-run; no raw request was executed"
                );
            }
            api::execute(&request).map_err(Into::into)
        }
        ("characters", "show") => {
            let selected = Options {
                actor: o.id.clone().unwrap_or_else(|| o.actor.clone()),
                ..o.clone()
            };
            let mut row = actor_row(&selected)?;
            presentation::Characters::load()?.annotate(&mut row);
            Ok(row)
        }
        ("characters", _) => {
            let mut data = call("private.characters.list", payload(o)?)?;
            let classifier = presentation::Characters::load()?;
            if let Some(rows) = data["characters"].as_array_mut() {
                for row in rows {
                    classifier.annotate(row);
                }
            }
            display::filter(&mut data, "characters", o);
            display::paginate(&mut data, "characters", o);
            Ok(data)
        }
        ("npc", "list" | "show") => {
            let mut p = payload(o)?;
            if v == "show" {
                let selected = Options {
                    actor: o.id.clone().unwrap_or_else(|| o.actor.clone()),
                    ..o.clone()
                };
                p["query"] = json!(npc_id(&selected)?);
            }
            paged("private.npc.list", p, o)
        }
        ("npc", "revive") => write(
            o,
            vec![edit("private.npc.revive", json!({"id":npc_id(o)?}))],
            json!({}),
        ),
        ("relationship", "show") => paged(
            "private.npc.list",
            json!({"path":save(o)?,"query":npc_id(o)?,"limit":1000}),
            o,
        ),
        ("relationship", "set") => write(
            o,
            vec![edit(
                "private.npc.setRelationship",
                json!({"id":npc_id(o)?,"relationship":required(&o.relationship,"relationship")?}),
            )],
            json!({}),
        ),
        ("skills", "list" | "show") => {
            let mut p = payload(o)?;
            if !o.actor.eq_ignore_ascii_case("hero") {
                p["actor"] = json!(npc_id(o)?);
            }
            let mut data = call("private.skills.list", p)?;
            let mut filter = o.clone();
            filter.id = o.skill.clone().or(o.id.clone());
            display::filter(&mut data, "skills", &filter);
            if v == "show" {
                let base = filter.id.as_deref().context("--skill or --id required")?;
                return data["skills"]
                    .as_array()
                    .context("no skills")?
                    .iter()
                    .find(|row| {
                        row["base"]
                            .as_str()
                            .is_some_and(|id| id.eq_ignore_ascii_case(base))
                    })
                    .cloned()
                    .context("skill was not found");
            }
            display::paginate(&mut data, "skills", o);
            Ok(data)
        }
        ("skills", "set") => write(
            o,
            vec![edit(
                "private.skills.set",
                json!({"actor":if o.actor.eq_ignore_ascii_case("hero"){"Hero".to_string()}else{npc_id(o)?},"base":required(&o.skill,"skill")?,"tier":required(&o.tier,"tier")?}),
            )],
            json!({}),
        ),
        ("attributes", _) => attributes(v, o),
        ("inventory", _) => inventory(v, o),
        ("position", _) => display::position(v, o),
        ("time", _) => time(v, o),
        ("difficulty", _) => administration::difficulty(v, o),
        ("quests" | "tutorials" | "glossary", "list") => progression(g, o),
        ("quests" | "tutorials" | "glossary" | "story", "show") => {
            let id =
                o.id.as_deref()
                    .or(o.entry.as_deref())
                    .or(o.document.as_deref())
                    .context("--id, --entry or --document required")?;
            let data = progression(
                g,
                &Options {
                    query: Some(id.to_owned()),
                    offset: 0,
                    all: true,
                    include_unset: o.include_unset || g == "story",
                    ..o.clone()
                },
            )?;
            Ok(display::find_row(&data, id)?.clone())
        }
        ("quests" | "tutorials" | "glossary", "set-state") => {
            let data = progression(
                g,
                &Options {
                    all: true,
                    include_unset: true,
                    state: None,
                    offset: 0,
                    ..o.clone()
                },
            )?;
            let id =
                o.id.as_deref()
                    .or(o.entry.as_deref())
                    .or(o.document.as_deref())
                    .context("--id, --entry or --document required")?;
            let row = display::find_row(&data, id)?;
            let path = row
                .get("statePath")
                .context("entry has no writable state path")?
                .clone();
            let state = required(&o.state, "state")?;
            // Enum states remain strings; numeric CurrentState properties
            // have no enum label and require a JSON integer for their raw path.
            let state = if row["currentState"].is_null() {
                state
                    .parse::<i64>()
                    .map(|n| json!(n))
                    .unwrap_or_else(|_| json!(state))
            } else {
                json!(state)
            };
            write(
                o,
                vec![edit(
                    "private.typed.setValue",
                    json!({"path":path,"value":state}),
                )],
                json!({}),
            )
        }
        ("segment", _) => write(
            o,
            vec![edit(
                "private.glossary.setSegment",
                json!({"documentClass":required(&o.document,"document")?,"segmentClass":required(&o.segment,"segment")?,"unlocked":v=="unlock"}),
            )],
            json!({}),
        ),
        ("story", "list") => progression("story", o),
        ("story", "set" | "unset") => {
            let id = required(&o.id, "id")?;
            let page = progression(
                "story",
                &Options {
                    query: Some(id.into()),
                    include_unset: true,
                    all: true,
                    offset: 0,
                    ..o.clone()
                },
            )?;
            let row = page["entries"].as_array().and_then(|r| {
                r.iter()
                    .find(|r| r["id"].as_str().is_some_and(|s| s.eq_ignore_ascii_case(id)))
            });
            let expected = if let Some(expect) = &o.expect {
                serde_json::from_str(expect)?
            } else {
                match row {
                    Some(r) => json!({"stored":r["stored"],"rawValue":r["rawValue"]}),
                    None => json!({"stored":false}),
                }
            };
            let mut change = json!({"id":id,"present":v=="set","expected":expected,"allowUnknownCreate":o.allow_unknown_create});
            if v == "set" {
                change["rawValue"] = value(o)?;
            }
            write(
                o,
                vec![edit("private.story.apply", json!({"changes":[change]}))],
                json!({}),
            )
        }
        ("knowledge", "list") => progression("knowledge", o),
        ("knowledge", "create-character") => write(
            o,
            vec![edit(
                "private.knowledge.addCharacter",
                json!({"value":character(o)?}),
            )],
            json!({}),
        ),
        ("knowledge", "add" | "remove") => write(
            o,
            vec![edit(
                "private.knowledge.setEntry",
                json!({"character":character(o)?,"entry":required(&o.entry,"entry")?,"present":v=="add"}),
            )],
            json!({}),
        ),
        ("events", "list") => progression("events", o),
        ("events", "show") => {
            let index = o.index.context("--index required")?;
            let data = progression(
                "events",
                &Options {
                    offset: index,
                    limit: 1,
                    all: false,
                    query: None,
                    ..o.clone()
                },
            )?;
            let mut row = data["events"]
                .as_array()
                .context("no events")?
                .iter()
                .find(|row| row["index"].as_u64() == Some(index as u64))
                .cloned()
                .context("event was not found")?;
            row["arrayPath"] = data["arrayPath"].clone();
            row["character"] = data["character"].clone();
            Ok(row)
        }
        ("events", "remove" | "duplicate") => {
            let data = progression("events", o)?;
            write(
                o,
                vec![edit(
                    if v == "remove" {
                        "private.typed.arrayRemove"
                    } else {
                        "private.typed.arrayDuplicate"
                    },
                    json!({"path":data["arrayPath"],"index":o.index.context("--index required")?}),
                )],
                json!({}),
            )
        }
        ("factions", "list" | "show") => {
            let mut data = call("private.factions.list", payload(o)?)?;
            let mut filter = o.clone();
            filter.id = o.guild.clone().or(o.id.clone());
            display::filter(&mut data, "guilds", &filter);
            display::paginate(&mut data, "guilds", o);
            Ok(data)
        }
        ("factions", "forgive") => write(
            o,
            vec![edit(
                "private.factions.forgive",
                json!({"guild":required(&o.guild,"guild")?}),
            )],
            json!({}),
        ),
        ("locks", "list" | "show") => {
            let mut data = call("private.locks.list", payload(o)?)?;
            let catalog = display::catalog("locks")?;
            let unlocked = data["unlocked"].as_array().cloned().unwrap_or_default();
            let mut rows = catalog["locks"]
                .as_array()
                .context("invalid lock catalog")?
                .clone();
            for row in &mut rows {
                row["unlocked"] = json!(unlocked.iter().any(|id| id.as_str().is_some_and(|s| {
                    row["l"]
                        .as_str()
                        .is_some_and(|id| s.eq_ignore_ascii_case(id))
                })));
            }
            data["locks"] = json!(rows);
            let mut selected = o.clone();
            selected.id = selected.lock.clone().or(selected.id);
            display::filter(&mut data, "locks", &selected);
            display::paginate(&mut data, "locks", o);
            Ok(data)
        }
        ("locks", "lock" | "unlock") => write(
            o,
            vec![edit(
                "private.locks.setUnlocked",
                json!({"lock":required(&o.lock,"lock")?,"unlocked":v=="unlock"}),
            )],
            json!({}),
        ),
        ("traders", "list") => {
            let mut data = call("private.traders.list", payload(o)?)?;
            display::filter(&mut data, "traders", o);
            display::paginate(&mut data, "traders", o);
            Ok(data)
        }
        ("traders", "show") => call("private.traders.detail", payload(o)?),
        ("stock", _) => {
            let count = if v == "set" {
                o.count.context("--count required")?
            } else {
                o.count.unwrap_or(1)
            };
            let path = display::existing_item_path(required(&o.item, "item")?)?;
            write(
                o,
                vec![edit(
                    match v {
                        "set" => "private.traders.setStock",
                        "add" => "private.traders.addItem",
                        _ => "private.traders.removeItem",
                    },
                    json!({"index":o.index.context("--index required")?,"map":o.map,"path":path,"count":count}),
                )],
                json!({}),
            )
        }
        ("timing", _) => display::timing(v, o),
        ("data", "search" | "show") => {
            let mut p = payload(o)?;
            p["includeNodes"] = json!(true);
            if v == "show" {
                let path = typed_path(o)?;
                let mut data = paged(
                    "search_typed_properties",
                    p,
                    &Options {
                        all: true,
                        offset: 0,
                        ..o.clone()
                    },
                )?;
                data["results"]
                    .as_array_mut()
                    .context("no typed results")?
                    .retain(|r| r["path"] == path);
                data["total"] = json!(data["results"].as_array().unwrap().len());
                if data["total"] == 0 {
                    bail!("property path not found");
                }
                Ok(data)
            } else {
                paged("search_typed_properties", p, o)
            }
        }
        ("data", _) => {
            let op = match v {
                "set" => "private.typed.setValue",
                "set-add" => "private.typed.setAdd",
                "set-remove" => "private.typed.setRemove",
                "array-remove" => "private.typed.arrayRemove",
                _ => "private.typed.arrayDuplicate",
            };
            let mut data = json!({"path":typed_path(o)?});
            if matches!(v, "array-remove" | "array-duplicate") {
                data["index"] = json!(o.index.context("--index required")?);
            } else {
                data["value"] = value(o)?;
            }
            write(o, vec![edit(op, data)], json!({}))
        }
        _ => bail!("unsupported save command {g} {v}"),
    }
}

fn attribute_set_class(row: &Value) -> &str {
    row["path"]
        .as_array()
        .or_else(|| row["basePath"].as_array())
        .and_then(|path| {
            path.iter()
                .position(|s| s == "AttributeSetsByClass")
                .and_then(|i| path.get(i + 1))
        })
        .and_then(Value::as_str)
        .map(|s| s.trim_matches(['{', '}']))
        .unwrap_or("")
}

fn attribute_set_matches(row: &Value, class: Option<&str>) -> bool {
    class.is_none_or(|class| {
        let set = attribute_set_class(row);
        set == class || set.rsplit('.').next() == Some(class)
    })
}

fn attributes(v: &str, o: &Options) -> Result<Value> {
    let mut p = payload(o)?;
    let mut data = if o.actor.eq_ignore_ascii_case("hero") {
        p["query"] = json!("AttributesByGlobalId {Hero}");
        p["offset"] = json!(0);
        p["limit"] = json!(1000);
        let mut all = o.clone();
        all.all = true;
        all.offset = 0;
        all.limit = 1000;
        paged("search_typed_properties", p, &all)?
    } else {
        p["id"] = json!(npc_id(o)?);
        call("private.npc.attributes", p)?
    };
    if v != "set" {
        let key = if o.actor.eq_ignore_ascii_case("hero") {
            "results"
        } else {
            "attributes"
        };
        if let Some(rows) = data[key].as_array_mut() {
            for row in rows.iter_mut() {
                let path = row["path"]
                    .as_array()
                    .or_else(|| row["basePath"].as_array());
                let id = row["key"]
                    .as_str()
                    .or_else(|| {
                        path.and_then(|p| p.get(p.len().saturating_sub(2)))
                            .and_then(Value::as_str)
                            .map(|s| s.trim_matches(['{', '}']))
                    })
                    .unwrap_or("");
                let id = id.to_string();
                let set = attribute_set_class(row).to_string();
                row["presentation"] = presentation::attribute_info(&id, &set, &o.lang);
                row["attributeId"] = json!(id);
                row["setClass"] = json!(set);
            }
            rows.retain(|r| {
                (o.all || r["presentation"]["hidden"] != true)
                    && o.group
                        .as_ref()
                        .is_none_or(|g| r["presentation"]["group"] == *g)
                    && attribute_set_matches(r, o.set_class.as_deref())
            });
        }
        let mut filter = o.clone();
        filter.id = o.attribute.clone().or(o.id.clone());
        display::localize(&mut data, o)?;
        display::filter(&mut data, key, &filter);
        display::paginate(&mut data, key, o);
        return Ok(data);
    }
    let id = required(&o.attribute, "attribute")?;
    let mut edits = Vec::new();
    if o.actor.eq_ignore_ascii_case("hero") {
        let hits = data["results"].as_array().context("no attributes")?;
        for field in ["base", "current"] {
            let val = if field == "base" { o.base } else { o.current }.or(if field == o.field {
                o.value_json.as_deref().and_then(|s| s.parse().ok())
            } else {
                None
            });
            let Some(val) = val else { continue };
            if !val.is_finite() {
                bail!("attribute value must be finite");
            }
            let matches: Vec<_> = hits
                .iter()
                .filter(|r| {
                    r["path"].as_array().is_some_and(|p| {
                        p.iter().any(|s| s.as_str() == Some(&format!("{{{id}}}")))
                            && p.last().and_then(Value::as_str)
                                == Some(if field == "base" {
                                    "BaseValue"
                                } else {
                                    "CurrentValue"
                                })
                            && attribute_set_matches(r, o.set_class.as_deref())
                    })
                })
                .collect();
            if matches.len() != 1 {
                bail!("attribute must resolve uniquely; supply --set-class");
            }
            edits.push(edit(
                "private.typed.setValue",
                json!({"path":matches[0]["path"],"value":val}),
            ));
        }
    } else {
        let rows = data["attributes"].as_array().context("no NPC attributes")?;
        let matches: Vec<_> = rows
            .iter()
            .filter(|r| {
                r["key"].as_str() == Some(id) && attribute_set_matches(r, o.set_class.as_deref())
            })
            .collect();
        if matches.len() != 1 {
            bail!("attribute must resolve uniquely; supply --set-class");
        }
        let row = matches[0];
        for (field, path, val) in [
            ("base", "basePath", o.base),
            ("current", "currentPath", o.current),
        ] {
            let val = val.or(if field == o.field {
                o.value_json.as_deref().and_then(|s| s.parse::<f64>().ok())
            } else {
                None
            });
            if let Some(val) = val {
                if !val.is_finite() {
                    bail!("attribute value must be finite");
                }
                edits.push(edit(
                    "private.typed.setValue",
                    json!({"path":row[path],"value":val}),
                ));
            }
        }
    }
    if edits.is_empty() {
        bail!("provide --base, --current or --value-json");
    }
    write(o, edits, json!({}))
}
fn inventory(v: &str, o: &Options) -> Result<Value> {
    if matches!(v, "list" | "show" | "check-slots") {
        let mut data = if o.actor.eq_ignore_ascii_case("hero") {
            let mut summary = call(
                "inspect_save",
                json!({"path":save(o)?,"includePrivate":true}),
            )?["private"]["inventory"]
                .clone();
            // Complete typed rows carry container/slot identities and their own
            // equipment metadata. Keep the scan only when typed inventory is absent.
            if let Ok(typed) = call("private.inventory.list", payload(o)?) {
                if typed["writable"]
                    .as_array()
                    .is_some_and(|rows| !rows.is_empty())
                {
                    summary["itemStackCount"] = json!(typed["items"].as_array().map(Vec::len));
                    summary["items"] = typed["items"].clone();
                    for capability in typed["writable"].as_array().into_iter().flatten() {
                        if let Some(writable) = summary["writable"].as_array_mut() {
                            if !writable.contains(capability) {
                                writable.push(capability.clone());
                            }
                        }
                    }
                }
            }
            summary
        } else {
            let mut inventory = call(
                "private.npc.inventory",
                json!({"path":save(o)?,"id":npc_id(o)?}),
            )?;
            if v == "check-slots" {
                inventory["slotIntegrity"] = call(
                    "inspect_save",
                    json!({"path":save(o)?,"includePrivate":true}),
                )?["private"]["inventory"]["slotIntegrity"]
                    .clone();
            }
            inventory
        };
        let catalog = display::catalog("items")?;
        let stats = display::catalog("item-stats")?;
        if let Some(rows) = data["items"].as_array_mut() {
            for row in rows.iter_mut() {
                let id = row["id"].as_str().unwrap_or("").to_string();
                if let Some(item) = catalog.as_array().unwrap().iter().find(|i| i["id"] == id) {
                    row["category"] = item["category"].clone();
                    row["icon"] = item["icon"].clone();
                }
                if o.details {
                    row["stats"] = stats["items"][&id].clone();
                }
            }
            rows.retain(|r| {
                o.container
                    .as_ref()
                    .is_none_or(|c| r["containerType"] == *c)
                    && o.slot
                        .as_ref()
                        .is_none_or(|s| r["slotId"].as_i64() == s.parse::<i64>().ok())
            });
        }
        let mut filter = o.clone();
        filter.id = o.item.clone().or(o.id.clone());
        display::localize(&mut data, o)?;
        display::filter(&mut data, "items", &filter);
        display::paginate(&mut data, "items", o);
        return Ok(data);
    }
    let op = match v {
        "set-count" => "private.inventory.setItemCount",
        "add" => "private.inventory.addItem",
        "remove" => "private.inventory.removeItem",
        "reset" => "private.inventory.reset",
        _ => "private.inventory.repairSlots",
    };
    let mut data = json!({});
    if v != "repair-slots" && !o.actor.eq_ignore_ascii_case("hero") {
        data["actorId"] = json!(npc_id(o)?);
    }
    let item = o.item.as_ref().or(o.id.as_ref());
    if v == "remove"
        || (v == "set-count" && (item.is_none() || o.slot.is_some() || o.container.is_some()))
    {
        if item.is_none() && o.slot.is_none() && o.container.is_none() {
            bail!("--item or a container/slot selector required");
        }
        let rows = inventory(
            "list",
            &Options {
                all: true,
                offset: 0,
                query: None,
                category: None,
                state: None,
                role: None,
                item: None,
                id: None,
                ..o.clone()
            },
        )?;
        let rows = rows["items"]
            .as_array()
            .context("inventory is unavailable")?;
        let rows: Vec<_> = rows
            .iter()
            .filter(|row| {
                item.is_none_or(|item| {
                    ["id", "path"].iter().any(|key| {
                        row[*key]
                            .as_str()
                            .is_some_and(|value| value.eq_ignore_ascii_case(item))
                    })
                })
            })
            .collect();
        if rows.len() != 1 {
            bail!(
                "inventory selector must match exactly one stack; provide --container and --slot"
            );
        }
        data["path"] = json!(
            rows[0]["path"]
                .as_str()
                .context("selected stack has no item definition path")?
        );
        for key in ["slotId", "containerType"] {
            if !rows[0][key].is_null() {
                data[key] = rows[0][key].clone();
            }
        }
    } else if let Some(item) = item {
        data["path"] = json!(display::existing_item_path(item)?)
    }
    if let Some(count) = o.count {
        data["count"] = json!(count)
    } else if v == "add" {
        data["count"] = json!(1)
    }
    if let Some(slot) = &o.slot {
        data["slotId"] = json!(slot.parse::<i32>().context("--slot must be an integer")?)
    }
    if let Some(container) = &o.container {
        data["containerType"] = json!(container)
    }
    if v == "reset" {
        data.as_object_mut()
            .unwrap()
            .extend(reset_template(o)?.as_object().unwrap().clone());
    }
    write(o, vec![edit(op, data)], json!({}))
}

fn reset_template(o: &Options) -> Result<Value> {
    let mut template = json!({});
    if o.resources_level.is_none() {
        template = administration::resources_profile_snapshot(o)?;
    }
    template["resourcesLevel"] = json!(administration::resources_level(o)?);
    Ok(template)
}
pub(super) fn time_value(o: &Options) -> Result<f64> {
    if let Some(seconds) = o.seconds {
        if !seconds.is_finite() || seconds < 0.0 {
            bail!("seconds must be finite and non-negative");
        }
        return Ok(seconds);
    }
    let day = o.day.context("--seconds or --day required")?;
    let (hour, minute, second) = (
        o.hour.unwrap_or(0),
        o.minute.unwrap_or(0),
        o.second.unwrap_or(0),
    );
    if hour > 23 || minute > 59 || second > 59 {
        bail!("invalid clock time");
    }
    Ok(day as f64 * 86400.0 + hour as f64 * 3600.0 + minute as f64 * 60.0 + second as f64)
}
pub(super) fn clock(o: &Options) -> Result<Value> {
    let data = call(
        "search_typed_properties",
        json!({"path":save(o)?,"query":"GameTime CurrentTime TotalSeconds","limit":1000}),
    )?;
    let hits = data["results"]
        .as_array()
        .context("world clock unavailable")?;
    let row = hits
        .iter()
        .find(|r| {
            r["path"]
                .as_array()
                .is_some_and(|p| p.last().and_then(Value::as_str) == Some("TotalSeconds"))
        })
        .context("world clock unavailable")?;
    Ok(row.clone())
}
fn time(v: &str, o: &Options) -> Result<Value> {
    let row = clock(o)?;
    if v == "set" {
        write(
            o,
            vec![edit(
                "private.typed.setValue",
                json!({"path":row["path"],"value":time_value(o)?}),
            )],
            json!({}),
        )
    } else {
        Ok(display::clock_parts(&row))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_derived_reset_template_blocks_direct_writes_and_staging_after_profile_changes() {
        let temp = tempfile::tempdir().unwrap();
        let save = temp.path().join("G1R-001.sav");
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../gore-save/assets/start_saves/resources_gothic.sav"),
            &save,
        )
        .unwrap();
        let profile = temp.path().join("PersistentDataList.sav");
        fs::write(&profile, b"original profile").unwrap();
        let options = capture_save_snapshot(&Options {
            save: Some(save.clone()),
            ..Options::default()
        })
        .unwrap();
        let template = reset_template(&options).unwrap();
        assert_eq!(
            template["expectedPersistentSha1"],
            api::file_sha1(&profile).unwrap()
        );
        let edits = vec![edit("private.inventory.reset", template)];
        fs::write(&profile, b"concurrent profile change").unwrap();
        let save_hash = api::file_sha1(&save).unwrap();
        for dry_run in [true, false] {
            let direct = Options {
                dry_run,
                ..options.clone()
            };
            assert!(
                write(&direct, edits.clone(), json!({}))
                    .unwrap_err()
                    .to_string()
                    .contains("profile changed")
            );
            let draft = temp.path().join("stale-reset.json");
            let staged = Options {
                draft: Some(draft.clone()),
                ..direct
            };
            assert!(
                write(&staged, edits.clone(), json!({}))
                    .unwrap_err()
                    .to_string()
                    .contains("profile changed")
            );
            assert!(!draft.exists());
            assert_eq!(api::file_sha1(&save).unwrap(), save_hash);
            assert_eq!(fs::read(&profile).unwrap(), b"concurrent profile change");
            assert!(!temp.path().join("goresave_backups").exists());
        }
        let explicit = reset_template(&Options {
            resources_level: Some("Novice".into()),
            ..options
        })
        .unwrap();
        assert_eq!(explicit["resourcesLevel"], "Novice");
        assert!(explicit.get("expectedPersistentSha1").is_none());
    }

    #[test]
    fn inspected_save_hash_guards_direct_writes_and_draft_publication() {
        let temp = tempfile::tempdir().unwrap();
        let save = temp.path().join("G1R-001.sav");
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../gore-save/assets/start_saves/resources_gothic.sav"),
            &save,
        )
        .unwrap();
        let options = Options {
            save: Some(save.clone()),
            ..Options::default()
        };
        let inspected = capture_save_snapshot(&options).unwrap();
        call(
            "write_save",
            json!({"path":save,"backup":false,"edits":[
                edit("public.m_PlayerSaveName", json!("Concurrent save change"))
            ]}),
        )
        .unwrap();
        let current_hash = api::file_sha1(&save).unwrap();
        assert_ne!(
            inspected.inspected_sha1.as_deref(),
            Some(current_hash.as_str())
        );
        let stale_edits = vec![edit(
            "public.m_PlayerSaveName",
            json!("Stale inspected edit"),
        )];
        for dry_run in [false, true] {
            let guarded = Options {
                dry_run,
                ..inspected.clone()
            };
            let error = write(&guarded, stale_edits.clone(), json!({})).unwrap_err();
            assert!(error.to_string().contains("changed"), "{error}");
            assert_eq!(api::file_sha1(&save).unwrap(), current_hash);
        }
        assert!(!temp.path().join("goresave_backups").exists());

        let draft = temp.path().join("guarded.json");
        let guarded = Options {
            draft: Some(draft.clone()),
            ..inspected
        };
        for dry_run in [false, true] {
            let options = Options {
                dry_run,
                ..guarded.clone()
            };
            let error = write(&options, stale_edits.clone(), json!({})).unwrap_err();
            assert!(error.to_string().contains("changed"), "{error}");
            assert!(!draft.exists());
        }
        administration::stage(&draft, &json!({"path":save,"edits":[]}), false).unwrap();
        let original_draft = std::fs::read(&draft).unwrap();
        let error = write(&guarded, stale_edits, json!({})).unwrap_err();
        assert!(error.to_string().contains("changed"), "{error}");
        assert_eq!(std::fs::read(&draft).unwrap(), original_draft);
        assert_eq!(api::file_sha1(&save).unwrap(), current_hash);
    }
}
