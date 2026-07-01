//! Faction crime memory: list the player's crimes per guild and "forgive" a
//! guild so a revived enemy NPC of that guild becomes non-hostile.
//!
//! The crime state is a blob `m_GenericData["CrimeMemoryPersistentData"]` — an
//! `FInstancedStruct` of `actual_type` `GothicCrimeMemorySaveGameData` — whose
//! three collections are ordinary tagged properties (the typed parser handles
//! them):
//!   - `GlobalCrimeDataEntries`: `TArray<FGlobalCrimeDataEntry>` (AUTHORITATIVE).
//!     Each entry: `ID(IntProperty)`, `CrimeType(GameplayTag)`,
//!     `bIsForgiven(BoolProperty)`, `CriminalGlobalID(NameProperty)`,
//!     `CriminalGuild(GameplayTag)`, `VictimGlobalIDs(Array<Name>)`,
//!     `VictimGuilds(GameplayTagContainer)`, `TrueCriminalGlobalID(NameProperty)`.
//!   - `RelativeCrimeDataEntries`: `TMap<Name witness, { RelativeCrimes: [
//!     FRelativeCrimeDataEntry{ ID, bIsSuppressed(BoolProperty), ... } ] }>`. Each
//!     `ID` matches a global entry's `ID`.
//!   - `LocationCrimeDataEntries`: location memories keyed by GameplayTag (left
//!     untouched — once global-forgiven the severity is discounted).
//!
//! The player's criminal id is the FName `"Hero"`.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::CoreError;
use crate::properties::{
    PathSeg, Property, PropertyValue, RootObject, ScalarValue, StructValue, parse_path,
    patch_scalar, resolve_chain,
};

/// The player's criminal id in `CriminalGlobalID`.
const HERO: &str = "Hero";

/// The `m_GenericData` map key holding the crime memory instanced struct.
const CRIME_BLOB_KEY: &str = "CrimeMemoryPersistentData";

/// The "other / individual" pseudo-guild for crimes that implicate neither a
/// mappable guild tag nor a mappable victim prefix. Listed but not a guild.
const OTHER_BUCKET: &str = "Other";

/// Per-type counts of UN-FORGIVEN crimes for one guild. Each crime entry is
/// counted once, in its MOST SEVERE category (see [`classify_crime`]).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrimeBreakdown {
    pub murder: usize,
    pub assault: usize,
    pub theft: usize,
    pub trespassing: usize,
    pub threat: usize,
    pub other: usize,
}

/// One guild's crime tally for the player.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuildCrimes {
    /// The camp-level guild tag (e.g. `Guild.Human.OldCamp`), or [`OTHER_BUCKET`]
    /// for the individual/unmappable bucket.
    pub guild: String,
    /// Human-readable label (camp name, e.g. `OldCamp`).
    pub label: String,
    pub total: usize,
    pub forgiven: usize,
    pub unforgiven: usize,
    /// `true` when this guild is currently HOSTILE to the player — i.e. at least
    /// one of its member NPCs will attack on load. Reproduced from crime memory
    /// the way the game re-derives it on load (recent, un-forgiven, un-suppressed
    /// player-crime severity per witness, age-decayed; see [`compute_hostile_guilds`]).
    /// NOT a stored flag and NOT a crime-count heuristic.
    pub is_hostile: bool,
    /// Un-forgiven crime counts by most-severe type.
    pub crimes: CrimeBreakdown,
}

/// A crime category, in descending severity. A single entry may carry several
/// `Crime.*` tags; we classify by the MOST SEVERE present so each entry is
/// counted exactly once, in its worst category (murder beats assault beats
/// theft beats trespassing beats threat beats other).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CrimeKind {
    // Order matters: higher discriminant = more severe (used by `max`).
    Other,
    Threat,
    Trespassing,
    Theft,
    Assault,
    Murder,
}

/// Classify a single `Crime.*` tag by its TagName prefix.
fn kind_for_tag(tag: &str) -> CrimeKind {
    if tag.starts_with("Crime.Murder") {
        CrimeKind::Murder
    } else if tag.starts_with("Crime.Assault") {
        CrimeKind::Assault
    } else if tag.starts_with("Crime.Theft") {
        CrimeKind::Theft
    } else if tag.starts_with("Crime.Trespassing") {
        CrimeKind::Trespassing
    } else if tag.starts_with("Crime.DirectThreat") {
        CrimeKind::Threat
    } else {
        CrimeKind::Other
    }
}

/// The most-severe [`CrimeKind`] across all `Crime.*` tags a crime entry carries.
/// Reads the `CrimeType` standalone FGameplayTag (a struct with a `TagName`
/// NameProperty) and any `Crime.*` tags found in a `CrimeTags`/`Tags`
/// GameplayTagContainer if present. Falls back to [`CrimeKind::Other`].
fn classify_crime(entry_props: &[Property]) -> CrimeKind {
    let mut worst = CrimeKind::Other;
    // Primary: the `CrimeType` standalone FGameplayTag (struct → TagName name).
    if let Some(tag) = crime_type_tag(entry_props) {
        worst = worst.max(kind_for_tag(&tag));
    }
    // Defensive: a crime may also expose a tag container of all crime tags.
    for container_name in ["CrimeTags", "Tags"] {
        if let Some(p) = member(entry_props, container_name) {
            for tag in gameplay_tag_container(&p.value) {
                worst = worst.max(kind_for_tag(tag));
            }
        }
    }
    worst
}

/// The `TagName` of the `CrimeType` standalone FGameplayTag struct, if present.
fn crime_type_tag(entry_props: &[Property]) -> Option<String> {
    let p = member(entry_props, "CrimeType")?;
    match &p.value {
        // Standalone FGameplayTag parses as a struct with a TagName NameProperty.
        PropertyValue::Struct(StructValue::Properties(inner)) => {
            member(inner, "TagName").and_then(|tn| match &tn.value {
                PropertyValue::Name(s) | PropertyValue::Str(s) => Some(s.clone()),
                _ => None,
            })
        }
        PropertyValue::Name(s) | PropertyValue::Str(s) => Some(s.clone()),
        _ => None,
    }
}

impl CrimeBreakdown {
    /// Add one crime of `kind` (most-severe-wins already applied by the caller).
    fn add(&mut self, kind: CrimeKind) {
        match kind {
            CrimeKind::Murder => self.murder += 1,
            CrimeKind::Assault => self.assault += 1,
            CrimeKind::Theft => self.theft += 1,
            CrimeKind::Trespassing => self.trespassing += 1,
            CrimeKind::Threat => self.threat += 1,
            CrimeKind::Other => self.other += 1,
        }
    }
}

// ── tag / struct accessors ──────────────────────────────────────────────────

/// A `GameplayTagContainer`'s tags (e.g. `VictimGuilds`), empty if absent.
fn gameplay_tag_container(value: &PropertyValue) -> &[String] {
    match value {
        PropertyValue::Struct(StructValue::GameplayTagContainer(tags)) => tags.as_slice(),
        _ => &[],
    }
}

/// A member property of a struct-valued (Properties) value, by name.
fn member<'a>(props: &'a [Property], name: &str) -> Option<&'a Property> {
    props.iter().find(|p| p.name == name)
}

/// The properties of an array element that is itself a struct (`Properties`).
fn element_props(value: &PropertyValue) -> Option<&[Property]> {
    match value {
        PropertyValue::Struct(StructValue::Properties(props)) => Some(props),
        _ => None,
    }
}

// ── crime-blob navigation ────────────────────────────────────────────────────

/// Locate the `GothicCrimeMemorySaveGameData` instanced struct: find the
/// `m_GenericData` MapProperty anywhere in the tree, then the entry keyed by
/// `CrimeMemoryPersistentData` whose value is the instanced struct. Returns the
/// path of `m_GenericData` (RELATIVE to root, with `{key}`/`[i]` segments) and
/// the instanced struct's property list.
fn find_crime_blob(root: &RootObject) -> Option<(Vec<String>, &[Property])> {
    find_generic_instanced(root, CRIME_BLOB_KEY)
}

/// Generic: locate the InstancedStruct value stored under `m_GenericData[key]`
/// anywhere in the tree. Returns the path of `m_GenericData` (with `{key}`/`[i]`
/// segments) and the instanced struct's property list. Used for both the crime
/// blob and the `GameTime` subsystem.
fn find_generic_instanced<'a>(
    root: &'a RootObject,
    target_key: &str,
) -> Option<(Vec<String>, &'a [Property])> {
    // The nested `if`s are not collapsible: on a non-matching m_GenericData we
    // still fall through to recurse into it.
    #[allow(clippy::collapsible_if)]
    fn walk<'a>(
        props: &'a [Property],
        path: &mut Vec<String>,
        target_key: &str,
    ) -> Option<(Vec<String>, &'a [Property])> {
        for p in props {
            path.push(p.name.clone());
            if p.name == "m_GenericData" {
                if let PropertyValue::Map { entries, .. } = &p.value {
                    for (k, v) in entries {
                        let key = crate::map_key_string(k);
                        if key.as_deref() == Some(target_key) {
                            if let PropertyValue::Struct(StructValue::Instanced(Some(inst))) = v {
                                return Some((path.clone(), inst.properties.as_slice()));
                            }
                        }
                    }
                }
            }
            if let Some(found) = walk_value(&p.value, path, target_key) {
                return Some(found);
            }
            path.pop();
        }
        None
    }
    fn walk_value<'a>(
        value: &'a PropertyValue,
        path: &mut Vec<String>,
        target_key: &str,
    ) -> Option<(Vec<String>, &'a [Property])> {
        match value {
            PropertyValue::Struct(StructValue::Properties(inner)) => walk(inner, path, target_key),
            PropertyValue::Struct(StructValue::Instanced(Some(i))) => {
                walk(&i.properties, path, target_key)
            }
            PropertyValue::Map { entries, .. } => {
                for (k, v) in entries {
                    let Some(key) = crate::map_key_string(k) else {
                        continue;
                    };
                    path.push(format!("{{{key}}}"));
                    if let Some(found) = walk_value(v, path, target_key) {
                        return Some(found);
                    }
                    path.pop();
                }
                None
            }
            PropertyValue::Array { elements } | PropertyValue::Set { elements, .. } => {
                for (i, e) in elements.iter().enumerate() {
                    path.push(format!("[{i}]"));
                    if let Some(found) = walk_value(e, path, target_key) {
                        return Some(found);
                    }
                    path.pop();
                }
                None
            }
            _ => None,
        }
    }
    let mut path = Vec::new();
    walk(&root.properties, &mut path, target_key)
}

/// The `GlobalCrimeDataEntries` array of the crime blob, or `&[]` if absent.
fn global_entries(blob: &[Property]) -> &[PropertyValue] {
    match member(blob, "GlobalCrimeDataEntries").map(|p| &p.value) {
        Some(PropertyValue::Array { elements }) => elements.as_slice(),
        _ => &[],
    }
}

// ── guild mapping ────────────────────────────────────────────────────────────

/// Map a victim GlobalID FName prefix to a camp-level guild tag. Old-/New-/Swamp-
/// camp NPC ids share family prefixes (`OC_`/`OM_`/`OCR_`/`FM_` => OldCamp;
/// `NC_`/`NM_`/`KDF_`/`PSI_` => NewCamp; `SC_`/`SM_`/`TPL_`/`NOV_` => SwampCamp).
/// Used only when a crime's `VictimGuilds` is empty (fallback).
fn guild_for_victim_id(victim: &str) -> Option<&'static str> {
    let prefixes: &[(&str, &str)] = &[
        ("OC_", "Guild.Human.OldCamp"),
        ("OM_", "Guild.Human.OldCamp"),
        ("OCR_", "Guild.Human.OldCamp"),
        ("FM_", "Guild.Human.OldCamp"),
        ("NC_", "Guild.Human.NewCamp"),
        ("NM_", "Guild.Human.NewCamp"),
        ("KDF_", "Guild.Human.NewCamp"),
        ("PSI_", "Guild.Human.NewCamp"),
        ("SC_", "Guild.Human.SwampCamp"),
        ("SM_", "Guild.Human.SwampCamp"),
        ("TPL_", "Guild.Human.SwampCamp"),
        ("NOV_", "Guild.Human.SwampCamp"),
    ];
    // Longest prefix first so e.g. `OCR_` beats `OC_`.
    let mut sorted = prefixes.to_vec();
    sorted.sort_by_key(|(p, _)| std::cmp::Reverse(p.len()));
    sorted
        .into_iter()
        .find(|(p, _)| victim.starts_with(p))
        .map(|(_, g)| g)
}

/// Reduce a full guild tag (`Guild.Human.OldCamp.Guard`) to its camp-level tag
/// (`Guild.Human.OldCamp`). Tags shorter than 3 segments are returned as-is.
fn camp_level_tag(tag: &str) -> String {
    let parts: Vec<&str> = tag.split('.').collect();
    if parts.len() >= 3 && parts[0] == "Guild" {
        format!("{}.{}.{}", parts[0], parts[1], parts[2])
    } else {
        tag.to_string()
    }
}

/// Short human label for a camp-level guild tag (last segment), e.g.
/// `Guild.Human.OldCamp` => `OldCamp`. The other bucket keeps its name.
fn label_for(guild: &str) -> String {
    guild.rsplit('.').next().unwrap_or(guild).to_string()
}

/// The set of camp-level guild tags a single global crime entry implicates,
/// considering ONLY the player's crimes (`CriminalGlobalID == "Hero"`). Returns
/// an empty set for non-Hero crimes. A crime with neither a mappable guild tag
/// nor a mappable victim prefix maps to [`OTHER_BUCKET`].
fn entry_guilds(entry_props: &[Property]) -> Vec<String> {
    let is_hero = member(entry_props, "CriminalGlobalID")
        .map(|p| matches!(&p.value, PropertyValue::Name(s) | PropertyValue::Str(s) if s == HERO))
        .unwrap_or(false);
    if !is_hero {
        return Vec::new();
    }
    let mut guilds: Vec<String> = Vec::new();
    // Primary: VictimGuilds tag container.
    if let Some(p) = member(entry_props, "VictimGuilds") {
        for tag in gameplay_tag_container(&p.value) {
            let camp = camp_level_tag(tag);
            if !guilds.contains(&camp) {
                guilds.push(camp);
            }
        }
    }
    // Fallback: when VictimGuilds is empty, map by victim id prefix.
    if guilds.is_empty() {
        if let Some(PropertyValue::Array { elements }) =
            member(entry_props, "VictimGlobalIDs").map(|p| &p.value)
        {
            for e in elements {
                let (PropertyValue::Name(id) | PropertyValue::Str(id)) = e else {
                    continue;
                };
                if let Some(g) = guild_for_victim_id(id) {
                    let g = g.to_string();
                    if !guilds.contains(&g) {
                        guilds.push(g);
                    }
                }
            }
        }
    }
    if guilds.is_empty() {
        guilds.push(OTHER_BUCKET.to_string());
    }
    guilds
}

/// `true` when the entry's `bIsForgiven` BoolProperty is set.
fn is_forgiven(entry_props: &[Property]) -> bool {
    member(entry_props, "bIsForgiven")
        .map(|p| matches!(p.value, PropertyValue::Bool(true)))
        .unwrap_or(false)
}

/// `true` when the camp-level guild `target` is implicated by this entry per the
/// pacify recipe: a `VictimGuilds` tag whose camp prefix equals `target`, OR (when
/// VictimGuilds is empty) a victim id mapping to `target`. `OTHER_BUCKET` matches
/// entries that fall into the other bucket.
fn entry_matches_guild(entry_props: &[Property], target: &str) -> bool {
    entry_guilds(entry_props).iter().any(|g| g == target)
}

// ── hostility (load-time re-derivation, reproduced from the save) ──────────────
//
// The game does NOT store a per-guild "is hostile" flag; it RE-DERIVES hostility
// when an NPC senses the player after load (AngelScript
// `UCrimeProcessingSubsystem.EvaluateCrimesInMemoryForCharacter`): each WITNESS
// NPC accumulates the severity of the player's crimes it remembers, AGE-DECAYED by
// in-game time, and attacks once that accumulated severity crosses a threshold.
// Forgiven/suppressed crimes contribute nothing. A guild is hostile iff at least
// one of its member NPCs reaches the attack threshold.
//
// We reproduce that here from the persisted inputs (all present in the save):
//   - `RelativeCrimeDataEntries[witness].RelativeCrimes[]`: per-witness memory of
//     the player's crimes, each with `BaseSeverity` and `bIsSuppressed`.
//   - `GlobalCrimeDataEntries[id]`: `bIsForgiven`, `TimeCommitted.TotalSeconds`,
//     and `CriminalGlobalID` (only `Hero`'s crimes count).
//   - `m_GenericData["GameTime"].CurrentTime.TotalSeconds`: the clock for ageing.
// The witness's guild is its id prefix ([`guild_for_victim_id`]).
//
// WINDOW + THRESHOLD were derived and validated against real saves: a save where
// the Old-Mine guards are hostile (OldCamp reaches 10.3) vs an otherwise-identical
// save where they have been pacified (OldCamp 3.3, every other guild < 0.1). The
// game's exact decay curve lives in native code (not recoverable); a linear decay
// over a 24h recency window reproduces the observed hostile/friendly split with a
// wide margin. See `factions.rs` tests `hostility_*` and the real-save test.

/// Recency window for crime age-decay, in in-game seconds (24h). A crime older
/// than this contributes no hostility (its severity has decayed away).
const HOSTILITY_WINDOW_SECONDS: f64 = 24.0 * 3600.0;

/// Accumulated age-decayed severity (per witness) at/above which that witness is
/// "angry" — it would attack the player on sight. ~two fresh serious crimes
/// (BaseSeverity 3 each). Per-witness; friendly ceiling observed at ~3.6.
const HOSTILITY_SEVERITY_THRESHOLD: f64 = 6.0;

/// Minimum number of ANGRY members for the whole GUILD to count as hostile. One
/// (or two) angry NPCs is an isolated incident — the player can shoot a single
/// guard without the camp turning on them. A faction is "hostile" only when an
/// area-wide conflict is active. Real-save distribution is sharply bimodal:
/// friendly saves top out at 1–2 angry members, a hostile camp has ~64 (the
/// whole population) — so any cutoff in between is robust; 5 leaves wide margin.
const HOSTILITY_MIN_ANGRY_MEMBERS: usize = 5;

/// Read a `{ TotalSeconds }` time struct (e.g. `TimeCommitted`, `CurrentTime`)
/// as f64 seconds, tolerating the various numeric encodings.
fn total_seconds(prop: &Property) -> Option<f64> {
    let PropertyValue::Struct(StructValue::Properties(inner)) = &prop.value else {
        return None;
    };
    match &member(inner, "TotalSeconds")?.value {
        PropertyValue::Double(d) => Some(*d),
        PropertyValue::Float(f) => Some(*f as f64),
        PropertyValue::Int64(i) => Some(*i as f64),
        PropertyValue::Int(i) => Some(*i as f64),
        PropertyValue::UInt32(i) => Some(*i as f64),
        _ => None,
    }
}

/// Current in-game time in seconds from `m_GenericData["GameTime"].CurrentTime`.
fn game_time_seconds(root: &RootObject) -> Option<f64> {
    let (_, gt) = find_generic_instanced(root, "GameTime")?;
    total_seconds(member(gt, "CurrentTime")?)
}

/// Per-global-crime facts the hostility model needs, keyed by crime `ID`.
struct GlobalCrimeFacts {
    forgiven: bool,
    time_committed: Option<f64>,
    by_hero: bool,
}

/// Index the global crime entries by `ID` for the relative-crime join.
fn global_crime_facts(blob: &[Property]) -> BTreeMap<i32, GlobalCrimeFacts> {
    let mut out = BTreeMap::new();
    for entry in global_entries(blob) {
        let Some(props) = element_props(entry) else {
            continue;
        };
        let Some(id) = member(props, "ID").and_then(|p| match &p.value {
            PropertyValue::Int(v) => Some(*v),
            _ => None,
        }) else {
            continue;
        };
        let by_hero = member(props, "CriminalGlobalID")
            .map(|p| matches!(&p.value, PropertyValue::Name(s) | PropertyValue::Str(s) if s == HERO))
            .unwrap_or(false);
        out.insert(
            id,
            GlobalCrimeFacts {
                forgiven: is_forgiven(props),
                time_committed: member(props, "TimeCommitted").and_then(total_seconds),
                by_hero,
            },
        );
    }
    out
}

/// The accumulated age-decayed severity of the player's crimes that `witness`
/// (a relative-crime entry's `RelativeCrimes` list) currently holds against the
/// player, using `facts` (global crimes by id) and the game clock `now`.
fn witness_anger(rel_crimes: &[PropertyValue], facts: &BTreeMap<i32, GlobalCrimeFacts>, now: f64) -> f64 {
    let mut accum = 0.0_f64;
    for rel in rel_crimes {
        let Some(rprops) = element_props(rel) else {
            continue;
        };
        let Some(id) = member(rprops, "ID").and_then(|p| match &p.value {
            PropertyValue::Int(v) => Some(*v),
            _ => None,
        }) else {
            continue;
        };
        let Some(fact) = facts.get(&id) else {
            continue;
        };
        if fact.forgiven || !fact.by_hero {
            continue;
        }
        let suppressed = member(rprops, "bIsSuppressed")
            .map(|p| matches!(p.value, PropertyValue::Bool(true)))
            .unwrap_or(false);
        if suppressed {
            continue;
        }
        let Some(committed) = fact.time_committed else {
            continue;
        };
        let age = (now - committed).max(0.0);
        if age >= HOSTILITY_WINDOW_SECONDS {
            continue; // decayed away
        }
        let base = member(rprops, "BaseSeverity")
            .and_then(|p| match &p.value {
                PropertyValue::Float(f) => Some(*f as f64),
                PropertyValue::Double(d) => Some(*d),
                _ => None,
            })
            .unwrap_or(0.0);
        let decay = 1.0 - age / HOSTILITY_WINDOW_SECONDS; // linear, in [0,1)
        accum += base * decay;
    }
    accum
}

/// The set of camp-level guild tags currently HOSTILE to the player, reproduced
/// from crime memory (see the section comment). A guild is hostile when at least
/// [`HOSTILITY_MIN_ANGRY_MEMBERS`] of its member NPCs are "angry" (per-witness
/// accumulated severity ≥ [`HOSTILITY_SEVERITY_THRESHOLD`]). Empty when the crime
/// blob or the game clock is missing (hostility unknown → reported as not hostile).
fn compute_hostile_guilds(root: &RootObject) -> std::collections::BTreeSet<String> {
    let mut angry_per_guild: BTreeMap<String, usize> = BTreeMap::new();
    let Some((_, blob)) = find_crime_blob(root) else {
        return Default::default();
    };
    let Some(now) = game_time_seconds(root) else {
        return Default::default();
    };
    let facts = global_crime_facts(blob);

    let Some(PropertyValue::Map { entries, .. }) =
        member(blob, "RelativeCrimeDataEntries").map(|p| &p.value)
    else {
        return Default::default();
    };
    for (witness_key, witness_val) in entries {
        let Some(witness) = crate::map_key_string(witness_key) else {
            continue;
        };
        let Some(guild) = guild_for_victim_id(&witness) else {
            continue; // un-mappable witness (no camp) — never a "hostile guild"
        };
        let Some(wprops) = element_props(witness_val).or_else(|| match witness_val {
            PropertyValue::Struct(StructValue::Instanced(Some(i))) => Some(&i.properties),
            _ => None,
        }) else {
            continue;
        };
        let Some(PropertyValue::Array { elements }) =
            member(wprops, "RelativeCrimes").map(|p| &p.value)
        else {
            continue;
        };
        if witness_anger(elements, &facts, now) >= HOSTILITY_SEVERITY_THRESHOLD {
            *angry_per_guild.entry(guild.to_string()).or_default() += 1;
        }
    }
    angry_per_guild
        .into_iter()
        .filter(|(_, n)| *n >= HOSTILITY_MIN_ANGRY_MEMBERS)
        .map(|(g, _)| g)
        .collect()
}

// ── list ─────────────────────────────────────────────────────────────────────

/// Tally the player's crimes per camp-level guild, with the real per-guild
/// hostility flag ([`compute_hostile_guilds`]). Sorted hostile-first, then by
/// un-forgiven count, then name. The unmappable [`OTHER_BUCKET`] ("Other") bucket
/// is NOT listed — it confuses more than it informs (it is not a faction and can
/// never be hostile or pacified meaningfully).
pub fn list_guild_crimes(root: &RootObject) -> Vec<GuildCrimes> {
    let Some((_, blob)) = find_crime_blob(root) else {
        return Vec::new();
    };
    let hostile = compute_hostile_guilds(root);
    /// Per-guild accumulator while tallying.
    #[derive(Default)]
    struct Acc {
        total: usize,
        forgiven: usize,
        /// Un-forgiven counts by most-severe type.
        crimes: CrimeBreakdown,
    }
    let mut tally: BTreeMap<String, Acc> = BTreeMap::new();
    for entry in global_entries(blob) {
        let Some(props) = element_props(entry) else {
            continue;
        };
        let guilds = entry_guilds(props);
        if guilds.is_empty() {
            continue; // non-Hero
        }
        let forgiven = is_forgiven(props);
        // Each entry counts once in its single most-severe category.
        let kind = classify_crime(props);
        for g in guilds {
            let slot = tally.entry(g).or_default();
            slot.total += 1;
            if forgiven {
                slot.forgiven += 1;
            } else {
                slot.crimes.add(kind);
            }
        }
    }
    let mut rows: Vec<GuildCrimes> = tally
        .into_iter()
        // Drop the catch-all individuals bucket — not a faction.
        .filter(|(guild, _)| guild != OTHER_BUCKET)
        .map(|(guild, acc)| {
            let is_hostile = hostile.contains(&guild);
            GuildCrimes {
                label: label_for(&guild),
                unforgiven: acc.total - acc.forgiven,
                total: acc.total,
                forgiven: acc.forgiven,
                is_hostile,
                crimes: acc.crimes,
                guild,
            }
        })
        .collect();
    // Hostile guilds float to the top, then by un-forgiven count, then name.
    rows.sort_by(|a, b| {
        b.is_hostile
            .cmp(&a.is_hostile)
            .then_with(|| b.unforgiven.cmp(&a.unforgiven))
            .then_with(|| a.guild.cmp(&b.guild))
    });
    rows
}

// ── forgive ──────────────────────────────────────────────────────────────────

/// The typed paths of every bool flip a forgive must apply (rooted at
/// `m_GenericData/{key}/...`).
struct ForgivePlan {
    /// Path segments to each affected global entry `bIsForgiven` (only those
    /// currently false — already-forgiven entries are skipped).
    global_bool_paths: Vec<Vec<String>>,
    /// Path segments to each matching relative-entry `bIsSuppressed` (only those
    /// currently false).
    relative_bool_paths: Vec<Vec<String>>,
}

/// Build a forgive plan for `target` guild against the current parsed root. The
/// `base` path is the `m_GenericData` map path; entries are addressed as
/// `<base>/{CrimeMemoryPersistentData}/GlobalCrimeDataEntries/[i]/bIsForgiven`.
fn build_forgive_plan(root: &RootObject, target: &str) -> Result<ForgivePlan, CoreError> {
    let (base, blob) = find_crime_blob(root)
        .ok_or_else(|| CoreError::Parse(format!("{CRIME_BLOB_KEY} crime blob not found")))?;
    // The instanced-struct value is addressed by descending into the map entry.
    let prefix = |suffix: &[&str]| -> Vec<String> {
        let mut segs = base.clone();
        segs.push(format!("{{{CRIME_BLOB_KEY}}}"));
        segs.extend(suffix.iter().map(|s| s.to_string()));
        segs
    };

    let mut affected_ids: Vec<i32> = Vec::new();
    let mut global_bool_paths: Vec<Vec<String>> = Vec::new();
    for (i, entry) in global_entries(blob).iter().enumerate() {
        let Some(props) = element_props(entry) else {
            continue;
        };
        if !entry_matches_guild(props, target) {
            continue;
        }
        if let Some(id) = member(props, "ID").and_then(|p| match &p.value {
            PropertyValue::Int(v) => Some(*v),
            _ => None,
        }) {
            affected_ids.push(id);
        }
        if !is_forgiven(props) {
            let mut segs = prefix(&["GlobalCrimeDataEntries"]);
            segs.push(format!("[{i}]"));
            segs.push("bIsForgiven".to_string());
            global_bool_paths.push(segs);
        }
    }

    // Relative entries: for each affected ID, suppress matching FRelativeCrimeDataEntry.
    let mut relative_bool_paths: Vec<Vec<String>> = Vec::new();
    if let Some(PropertyValue::Map { entries, .. }) =
        member(blob, "RelativeCrimeDataEntries").map(|p| &p.value)
    {
        for (witness_key, witness_val) in entries {
            let Some(witness) = crate::map_key_string(witness_key) else {
                continue;
            };
            // Value is a struct with `RelativeCrimes: [FRelativeCrimeDataEntry]`.
            let Some(wprops) = element_props(witness_val).or_else(|| match witness_val {
                PropertyValue::Struct(StructValue::Instanced(Some(i))) => Some(&i.properties),
                _ => None,
            }) else {
                continue;
            };
            let Some(PropertyValue::Array { elements }) =
                member(wprops, "RelativeCrimes").map(|p| &p.value)
            else {
                continue;
            };
            for (j, rel) in elements.iter().enumerate() {
                let Some(rprops) = element_props(rel) else {
                    continue;
                };
                let Some(id) = member(rprops, "ID").and_then(|p| match &p.value {
                    PropertyValue::Int(v) => Some(*v),
                    _ => None,
                }) else {
                    continue;
                };
                if !affected_ids.contains(&id) {
                    continue;
                }
                let suppressed = member(rprops, "bIsSuppressed")
                    .map(|p| matches!(p.value, PropertyValue::Bool(true)))
                    .unwrap_or(false);
                if suppressed {
                    continue;
                }
                let mut segs = prefix(&["RelativeCrimeDataEntries"]);
                segs.push(format!("{{{witness}}}"));
                segs.push("RelativeCrimes".to_string());
                segs.push(format!("[{j}]"));
                segs.push("bIsSuppressed".to_string());
                relative_bool_paths.push(segs);
            }
        }
    }

    Ok(ForgivePlan {
        global_bool_paths,
        relative_bool_paths,
    })
}

/// Forgive every unforgiven player crime implicating `target` guild: set
/// `bIsForgiven=true` on matching `GlobalCrimeDataEntries` and `bIsSuppressed=true`
/// on matching `RelativeCrimeDataEntries`. SIZE-NEUTRAL bool-byte flips applied in
/// place via [`patch_scalar`]. Idempotent: already-forgiven/suppressed entries are
/// left alone. Re-parses the payload to prove byte-consistency before returning.
pub fn apply_forgive(payload: &mut Vec<u8>, target: &str) -> Result<(), CoreError> {
    let target = target.trim();
    if target.is_empty() {
        return Err(CoreError::InvalidRequest(
            "private.factions.forgive requires a non-empty guild tag".to_string(),
        ));
    }
    // Collect bool offsets via resolve_chain against the parsed root, then apply
    // each in place. Bool flips never change length, so all offsets stay valid
    // across the whole batch (no per-edit re-parse needed).
    let root = crate::properties::parse_private_root(payload)?;
    let plan = build_forgive_plan(&root, target)?;

    // Resolve every target property up front (immutable borrow of root), record
    // (property clone) for patching after the borrow ends.
    let mut targets: Vec<Property> = Vec::new();
    for segs in plan
        .global_bool_paths
        .iter()
        .chain(plan.relative_bool_paths.iter())
    {
        let path: Vec<PathSeg> = parse_path(segs)?;
        let chain = resolve_chain(&root.properties, &path)?;
        if chain.target.type_name != "BoolProperty" {
            return Err(CoreError::Parse(format!(
                "expected BoolProperty at {segs:?}, got {}",
                chain.target.type_name
            )));
        }
        targets.push(chain.target.clone());
    }
    drop(root);

    for target_prop in &targets {
        patch_scalar(payload, target_prop, ScalarValue::Bool(true))?;
    }

    // Prove byte-consistency: the payload must re-parse fully.
    let reparsed = crate::properties::parse_private_root(payload).map_err(|err| {
        CoreError::Parse(format!("forgive produced an inconsistent payload: {err}"))
    })?;
    // Sanity: every affected global entry is now forgiven.
    if let Some((_, blob)) = find_crime_blob(&reparsed) {
        for entry in global_entries(blob) {
            let Some(props) = element_props(entry) else {
                continue;
            };
            if entry_matches_guild(props, target) && !is_forgiven(props) {
                return Err(CoreError::Validation(format!(
                    "forgive left a {target} crime unforgiven"
                )));
            }
        }
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::properties::{TAG_FLAG_BOOL_TRUE, TAG_FLAG_NATIVE_SERIALIZE, parse_private_root};

    // ── byte builders (mirror the parser's wire format) ───────────────────────

    fn fstring(value: &str) -> Vec<u8> {
        let mut out = ((value.len() + 1) as i32).to_le_bytes().to_vec();
        out.extend_from_slice(value.as_bytes());
        out.push(0);
        out
    }

    fn tag(name: &str, type_name: &str) -> Vec<u8> {
        let mut out = fstring(name);
        out.extend_from_slice(&fstring(type_name));
        out
    }

    /// `[u32 array_index][u32 size][u8 flags]` value header.
    fn header(size: u32, flags: u8) -> Vec<u8> {
        let mut out = 0u32.to_le_bytes().to_vec();
        out.extend_from_slice(&size.to_le_bytes());
        out.push(flags);
        out
    }

    fn int_property(name: &str, value: i32) -> Vec<u8> {
        let mut out = tag(name, "IntProperty");
        out.extend_from_slice(&header(4, 0));
        out.extend_from_slice(&value.to_le_bytes());
        out
    }

    fn bool_property(name: &str, on: bool) -> Vec<u8> {
        let mut out = tag(name, "BoolProperty");
        out.extend_from_slice(&header(0, if on { TAG_FLAG_BOOL_TRUE } else { 0 }));
        out
    }

    fn name_property(name: &str, value: &str) -> Vec<u8> {
        let body = fstring(value);
        let mut out = tag(name, "NameProperty");
        out.extend_from_slice(&header(body.len() as u32, 0));
        out.extend_from_slice(&body);
        out
    }

    /// A standalone FGameplayTag StructProperty (`TagName: NameProperty`).
    fn gameplay_tag_property(name: &str, tag_name: &str) -> Vec<u8> {
        let mut body = name_property("TagName", tag_name);
        body.extend_from_slice(&fstring("None"));
        let mut out = tag(name, "StructProperty");
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("GameplayTag"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/GameplayTags"));
        out.extend_from_slice(&header(body.len() as u32, 0));
        out.extend_from_slice(&body);
        out
    }

    /// A native GameplayTagContainer StructProperty (VictimGuilds).
    fn tag_container_property(name: &str, tags: &[&str]) -> Vec<u8> {
        let mut body = (tags.len() as u32).to_le_bytes().to_vec();
        for t in tags {
            body.extend_from_slice(&fstring(t));
        }
        let mut out = tag(name, "StructProperty");
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("GameplayTagContainer"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/GameplayTags"));
        out.extend_from_slice(&header(body.len() as u32, TAG_FLAG_NATIVE_SERIALIZE));
        out.extend_from_slice(&body);
        out
    }

    /// An ArrayProperty<NameProperty> (VictimGlobalIDs).
    fn name_array_property(name: &str, names: &[&str]) -> Vec<u8> {
        let mut descriptor = 1u32.to_le_bytes().to_vec();
        descriptor.extend_from_slice(&fstring("NameProperty"));
        let mut body = (names.len() as u32).to_le_bytes().to_vec();
        for n in names {
            body.extend_from_slice(&fstring(n));
        }
        let mut out = tag(name, "ArrayProperty");
        out.extend_from_slice(&descriptor);
        out.extend_from_slice(&header(body.len() as u32, 0));
        out.extend_from_slice(&body);
        out
    }

    fn double_property(name: &str, value: f64) -> Vec<u8> {
        let mut out = tag(name, "DoubleProperty");
        out.extend_from_slice(&header(8, 0));
        out.extend_from_slice(&value.to_le_bytes());
        out
    }

    fn float_property(name: &str, value: f32) -> Vec<u8> {
        let mut out = tag(name, "FloatProperty");
        out.extend_from_slice(&header(4, 0));
        out.extend_from_slice(&value.to_le_bytes());
        out
    }

    /// An `FInGameTime`-shaped StructProperty `{ TotalSeconds: Double }` (the wire
    /// shape of `TimeCommitted` / `GameTime.CurrentTime`).
    fn ingametime_property(name: &str, total_seconds: f64) -> Vec<u8> {
        let mut body = double_property("TotalSeconds", total_seconds);
        body.extend_from_slice(&fstring("None"));
        let mut out = tag(name, "StructProperty");
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("InGameTime"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/G1R"));
        out.extend_from_slice(&header(body.len() as u32, 0));
        out.extend_from_slice(&body);
        out
    }

    /// One FGlobalCrimeDataEntry property list (no trailing "None"; the array
    /// builder appends it). Defaults the crime type to `Crime.Theft`; use
    /// [`global_entry_typed`] to set a specific `Crime.*` tag.
    fn global_entry(
        id: i32,
        forgiven: bool,
        criminal: &str,
        victim_guilds: &[&str],
        victim_ids: &[&str],
    ) -> Vec<u8> {
        global_entry_typed(id, forgiven, criminal, victim_guilds, victim_ids, "Crime.Theft")
    }

    /// Like [`global_entry`] but with an explicit `CrimeType` tag.
    fn global_entry_typed(
        id: i32,
        forgiven: bool,
        criminal: &str,
        victim_guilds: &[&str],
        victim_ids: &[&str],
        crime_type: &str,
    ) -> Vec<u8> {
        let mut out = int_property("ID", id);
        out.extend_from_slice(&gameplay_tag_property("CrimeType", crime_type));
        out.extend_from_slice(&bool_property("bIsForgiven", forgiven));
        out.extend_from_slice(&name_property("CriminalGlobalID", criminal));
        out.extend_from_slice(&gameplay_tag_property("CriminalGuild", "Guild.None"));
        out.extend_from_slice(&name_array_property("VictimGlobalIDs", victim_ids));
        out.extend_from_slice(&tag_container_property("VictimGuilds", victim_guilds));
        out.extend_from_slice(&name_property("TrueCriminalGlobalID", "None"));
        out
    }

    /// Like [`global_entry_typed`] plus a `TimeCommitted` timestamp — for the
    /// hostility model (which age-decays crimes against the game clock).
    fn global_entry_timed(
        id: i32,
        forgiven: bool,
        victim_guilds: &[&str],
        crime_type: &str,
        time_committed_secs: f64,
    ) -> Vec<u8> {
        let mut out = global_entry_typed(id, forgiven, "Hero", victim_guilds, &[], crime_type);
        out.extend_from_slice(&ingametime_property("TimeCommitted", time_committed_secs));
        out
    }

    /// ArrayProperty<StructProperty FGlobalCrimeDataEntry>.
    fn global_entries_array(entries: &[Vec<u8>]) -> Vec<u8> {
        let mut descriptor = 1u32.to_le_bytes().to_vec();
        descriptor.extend_from_slice(&fstring("StructProperty"));
        descriptor.extend_from_slice(&1u32.to_le_bytes());
        descriptor.extend_from_slice(&fstring("FGlobalCrimeDataEntry"));
        descriptor.extend_from_slice(&1u32.to_le_bytes());
        descriptor.extend_from_slice(&fstring("/Script/G1R"));
        let mut body = (entries.len() as u32).to_le_bytes().to_vec();
        for e in entries {
            body.extend_from_slice(e);
            body.extend_from_slice(&fstring("None"));
        }
        let mut out = tag("GlobalCrimeDataEntries", "ArrayProperty");
        out.extend_from_slice(&descriptor);
        out.extend_from_slice(&header(body.len() as u32, 0));
        out.extend_from_slice(&body);
        out
    }

    /// One FRelativeCrimeDataEntry property list (no trailing "None").
    fn relative_entry(id: i32, suppressed: bool) -> Vec<u8> {
        let mut out = int_property("ID", id);
        out.extend_from_slice(&bool_property("bIsSuppressed", suppressed));
        out
    }

    /// Like [`relative_entry`] plus a `BaseSeverity` — for the hostility model.
    fn relative_entry_sev(id: i32, suppressed: bool, base_severity: f32) -> Vec<u8> {
        let mut out = int_property("ID", id);
        out.extend_from_slice(&float_property("BaseSeverity", base_severity));
        out.extend_from_slice(&bool_property("bIsSuppressed", suppressed));
        out
    }

    /// ArrayProperty<StructProperty FRelativeCrimeDataEntry> named RelativeCrimes.
    fn relative_crimes_array(entries: &[Vec<u8>]) -> Vec<u8> {
        let mut descriptor = 1u32.to_le_bytes().to_vec();
        descriptor.extend_from_slice(&fstring("StructProperty"));
        descriptor.extend_from_slice(&1u32.to_le_bytes());
        descriptor.extend_from_slice(&fstring("FRelativeCrimeDataEntry"));
        descriptor.extend_from_slice(&1u32.to_le_bytes());
        descriptor.extend_from_slice(&fstring("/Script/G1R"));
        let mut body = (entries.len() as u32).to_le_bytes().to_vec();
        for e in entries {
            body.extend_from_slice(e);
            body.extend_from_slice(&fstring("None"));
        }
        let mut out = tag("RelativeCrimes", "ArrayProperty");
        out.extend_from_slice(&descriptor);
        out.extend_from_slice(&header(body.len() as u32, 0));
        out.extend_from_slice(&body);
        out
    }

    /// A witness-value (`{ RelativeCrimes: [...] }`) — the value of a
    /// RelativeCrimeDataEntries map entry, serialized inline as a struct map value
    /// (property list + "None", no per-value size header).
    fn witness_value(rel_entries: &[Vec<u8>]) -> Vec<u8> {
        let mut out = relative_crimes_array(rel_entries);
        out.extend_from_slice(&fstring("None"));
        out
    }

    /// RelativeCrimeDataEntries MapProperty<Name, StructProperty FRelativeCrimesContainer>.
    fn relative_map(witnesses: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut descriptor = 2u32.to_le_bytes().to_vec();
        descriptor.extend_from_slice(&fstring("NameProperty"));
        descriptor.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        descriptor.extend_from_slice(&fstring("StructProperty"));
        descriptor.extend_from_slice(&1u32.to_le_bytes());
        descriptor.extend_from_slice(&fstring("FRelativeCrimesContainer"));
        descriptor.extend_from_slice(&1u32.to_le_bytes());
        descriptor.extend_from_slice(&fstring("/Script/G1R"));
        let mut body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        body.extend_from_slice(&(witnesses.len() as u32).to_le_bytes());
        for (witness, value) in witnesses {
            body.extend_from_slice(&fstring(witness));
            body.extend_from_slice(value);
        }
        let mut out = tag("RelativeCrimeDataEntries", "MapProperty");
        out.extend_from_slice(&descriptor);
        out.extend_from_slice(&header(body.len() as u32, 0));
        out.extend_from_slice(&body);
        out
    }

    /// Build a full private payload: the crime blob (an InstancedStruct value of a
    /// `m_GenericData["CrimeMemoryPersistentData"]` map entry) plus a trailing int
    /// so a missed size fixup desyncs the strict re-parse.
    /// Wrap a struct body as an InstancedStruct: `[fstring actual_type][u32 size][body]`.
    fn instanced(actual_type: &str, struct_body: &[u8]) -> Vec<u8> {
        let mut out = fstring(actual_type);
        out.extend_from_slice(&(struct_body.len() as u32).to_le_bytes());
        out.extend_from_slice(struct_body);
        out
    }

    /// The `GothicCrimeMemorySaveGameData` InstancedStruct bytes (global + relative
    /// crime collections + an empty LocationCrimeDataEntries map).
    fn crime_blob_instanced(global: &[Vec<u8>], relative: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut struct_body = global_entries_array(global);
        struct_body.extend_from_slice(&relative_map(relative));
        // Empty LocationCrimeDataEntries map (GameplayTag key, struct value).
        let mut descriptor = 2u32.to_le_bytes().to_vec();
        descriptor.extend_from_slice(&fstring("StructProperty"));
        descriptor.extend_from_slice(&1u32.to_le_bytes());
        descriptor.extend_from_slice(&fstring("GameplayTag"));
        descriptor.extend_from_slice(&1u32.to_le_bytes());
        descriptor.extend_from_slice(&fstring("/Script/GameplayTags"));
        descriptor.extend_from_slice(&0u32.to_le_bytes()); // value_flags
        descriptor.extend_from_slice(&fstring("StructProperty"));
        descriptor.extend_from_slice(&1u32.to_le_bytes());
        descriptor.extend_from_slice(&fstring("FLocationMemoriesContainer"));
        descriptor.extend_from_slice(&1u32.to_le_bytes());
        descriptor.extend_from_slice(&fstring("/Script/G1R"));
        let mut mbody = 0u32.to_le_bytes().to_vec();
        mbody.extend_from_slice(&0u32.to_le_bytes()); // count 0
        struct_body.extend_from_slice(&tag("LocationCrimeDataEntries", "MapProperty"));
        struct_body.extend_from_slice(&descriptor);
        struct_body.extend_from_slice(&header(mbody.len() as u32, 0));
        struct_body.extend_from_slice(&mbody);
        struct_body.extend_from_slice(&fstring("None"));
        instanced("/Script/G1R.GothicCrimeMemorySaveGameData", &struct_body)
    }

    /// A `GameTimeSaveGameData` InstancedStruct with `CurrentTime` = `now_secs`.
    fn gametime_instanced(now_secs: f64) -> Vec<u8> {
        let mut struct_body = ingametime_property("CurrentTime", now_secs);
        struct_body.extend_from_slice(&fstring("None"));
        instanced("/Script/G1R.GameTimeSaveGameData", &struct_body)
    }

    /// Wrap `m_GenericData` entries (key → InstancedStruct bytes) into a full
    /// private payload (object envelope + trailing canary so a size desync fails
    /// the strict re-parse).
    fn payload_from_generic(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut map_body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        map_body.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for (key, value) in entries {
            map_body.extend_from_slice(&fstring(key));
            map_body.extend_from_slice(value);
        }
        let mut map_descriptor = 2u32.to_le_bytes().to_vec();
        map_descriptor.extend_from_slice(&fstring("NameProperty"));
        map_descriptor.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        map_descriptor.extend_from_slice(&fstring("StructProperty"));
        map_descriptor.extend_from_slice(&1u32.to_le_bytes());
        map_descriptor.extend_from_slice(&fstring("InstancedStruct"));
        map_descriptor.extend_from_slice(&1u32.to_le_bytes());
        map_descriptor.extend_from_slice(&fstring("/Script/StructUtils"));
        let mut generic = tag("m_GenericData", "MapProperty");
        generic.extend_from_slice(&map_descriptor);
        generic.extend_from_slice(&header(map_body.len() as u32, 0));
        generic.extend_from_slice(&map_body);

        let mut p = fstring("/Script/Angelscript.GothicFinalDataGame");
        p.push(0); // object flag
        p.extend_from_slice(&generic);
        p.extend_from_slice(&int_property("m_After", 7)); // trailing canary
        p.extend_from_slice(&fstring("None"));
        p.extend_from_slice(&0u32.to_le_bytes()); // footer
        p
    }

    /// Build a full private payload with just the crime blob (no GameTime → the
    /// hostility model reports nothing hostile; used by list/forgive tests).
    fn crime_payload(global: &[Vec<u8>], relative: &[(&str, Vec<u8>)]) -> Vec<u8> {
        payload_from_generic(&[(CRIME_BLOB_KEY, crime_blob_instanced(global, relative))])
    }

    /// Build a full private payload with the crime blob AND a `GameTime` clock —
    /// for the hostility model (needs `now` to age-decay crimes).
    fn crime_payload_h(
        global: &[Vec<u8>],
        relative: &[(&str, Vec<u8>)],
        now_secs: f64,
    ) -> Vec<u8> {
        payload_from_generic(&[
            (CRIME_BLOB_KEY, crime_blob_instanced(global, relative)),
            ("GameTime", gametime_instanced(now_secs)),
        ])
    }

    /// The standard synthetic fixture: 5 global entries + a relative map.
    fn fixture() -> Vec<u8> {
        let global = vec![
            global_entry(100, false, "Hero", &["Guild.Human.OldCamp.Guard"], &[]),
            global_entry(200, false, "Hero", &["Guild.Human.NewCamp"], &[]),
            global_entry(300, false, "OC_GRD_Guard1", &["Guild.Human.OldCamp"], &[]),
            global_entry(400, true, "Hero", &["Guild.Human.OldCamp"], &[]),
            global_entry(500, false, "Hero", &[], &["OC_EBR_Scar_101"]),
        ];
        let relative = vec![
            (
                "OC_STT_Diego",
                witness_value(&[relative_entry(100, false), relative_entry(200, false)]),
            ),
            ("OC_GRD_Guard14", witness_value(&[relative_entry(500, false)])),
        ];
        crime_payload(&global, &relative)
    }

    // ── list ──────────────────────────────────────────────────────────────────

    #[test]
    fn list_groups_and_counts_per_guild() {
        let payload = fixture();
        let root = parse_private_root(&payload).unwrap();
        assert_eq!(root.consumed, payload.len(), "fixture parses byte-clean");
        let rows = list_guild_crimes(&root);
        let by: BTreeMap<&str, &GuildCrimes> =
            rows.iter().map(|r| (r.guild.as_str(), r)).collect();

        // OldCamp: ids 100 (unf), 400 (forgiven), 500 (unf via victim prefix).
        let oc = by["Guild.Human.OldCamp"];
        assert_eq!(oc.total, 3);
        assert_eq!(oc.forgiven, 1);
        assert_eq!(oc.unforgiven, 2);
        assert_eq!(oc.label, "OldCamp");

        let nc = by["Guild.Human.NewCamp"];
        assert_eq!(nc.total, 1);
        assert_eq!(nc.unforgiven, 1);

        // Non-Hero crime (id 300) excluded; no Other bucket here.
        assert!(!by.contains_key("Other"), "rows: {rows:?}");
        // Sorted by unforgiven desc.
        assert_eq!(rows[0].guild, "Guild.Human.OldCamp");
    }

    #[test]
    fn list_omits_unmappable_other_bucket() {
        // A crime against an unmappable individual lands in OTHER_BUCKET internally
        // but must NOT surface as a faction row (the UI dropped "Other").
        let global = vec![global_entry(1, false, "Hero", &[], &["Lizard-WP_SPAWN_01"])];
        let payload = crime_payload(&global, &[]);
        let root = parse_private_root(&payload).unwrap();
        let rows = list_guild_crimes(&root);
        assert!(
            !rows.iter().any(|r| r.guild == "Other"),
            "Other bucket must not be listed: {rows:?}"
        );
    }

    // ── hostility (real model: fresh, un-forgiven player-crime severity) ─────────

    /// Anchor time for the synthetic hostility fixtures (arbitrary game clock).
    const NOW: f64 = 1_000_000.0;
    fn hours_ago(h: f64) -> f64 {
        NOW - h * 3600.0
    }

    /// Build `n_witnesses` members of `guild` (witness ids `{prefix}{i}`), each
    /// remembering `crimes_each` Hero crimes of BaseSeverity `base` committed at
    /// `committed`, optionally `forgiven`/`suppressed`. Crime ids start at
    /// `start_id`. Returns (global entries, owned (witness-key, value) pairs).
    #[allow(clippy::too_many_arguments)]
    fn angry_members(
        guild: &str,
        prefix: &str,
        n_witnesses: usize,
        crimes_each: usize,
        base: f32,
        forgiven: bool,
        suppressed: bool,
        committed: f64,
        start_id: i32,
    ) -> (Vec<Vec<u8>>, Vec<(String, Vec<u8>)>) {
        let mut globals = Vec::new();
        let mut witnesses = Vec::new();
        let mut id = start_id;
        for w in 0..n_witnesses {
            let mut rels = Vec::new();
            for _ in 0..crimes_each {
                globals.push(global_entry_timed(
                    id,
                    forgiven,
                    &[guild],
                    "Crime.Assault.Weapon.Melee",
                    committed,
                ));
                rels.push(relative_entry_sev(id, suppressed, base));
                id += 1;
            }
            witnesses.push((format!("{prefix}{w}"), witness_value(&rels)));
        }
        (globals, witnesses)
    }

    /// Assemble a hostility payload from several [`angry_members`] groups.
    fn hostility_payload(
        groups: Vec<(Vec<Vec<u8>>, Vec<(String, Vec<u8>)>)>,
        now: f64,
    ) -> Vec<u8> {
        let mut globals = Vec::new();
        let mut owned: Vec<(String, Vec<u8>)> = Vec::new();
        for (g, w) in groups {
            globals.extend(g);
            owned.extend(w);
        }
        let relative: Vec<(&str, Vec<u8>)> =
            owned.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
        crime_payload_h(&globals, &relative, now)
    }

    #[test]
    fn area_conflict_of_angry_members_is_hostile() {
        // OldCamp: 6 members, each 3 FRESH un-forgiven assaults (accum ≈ 8.25 ≥ 6)
        //   → 6 angry ≥ 5 members → HOSTILE.
        // NewCamp: 6 members, fresh assaults but FORGIVEN → 0 angry → friendly.
        // SwampCamp: 6 members, OLD (100h, decayed) crimes → 0 angry → friendly.
        let oc = "Guild.Human.OldCamp";
        let nc = "Guild.Human.NewCamp";
        let sc = "Guild.Human.SwampCamp";
        let fresh = hours_ago(2.0);
        let old = hours_ago(100.0);
        let payload = hostility_payload(
            vec![
                angry_members(oc, "OC_GRD_Guard", 6, 3, 3.0, false, false, fresh, 1),
                angry_members(nc, "NC_GRD_Guard", 6, 3, 3.0, true, false, fresh, 1_000),
                angry_members(sc, "SC_NOV_Nov", 6, 3, 3.0, false, false, old, 2_000),
            ],
            NOW,
        );
        let root = parse_private_root(&payload).unwrap();
        assert_eq!(root.consumed, payload.len(), "fixture parses byte-clean");
        let hostile = compute_hostile_guilds(&root);
        assert!(hostile.contains(oc), "6 angry OldCamp members → hostile");
        assert!(!hostile.contains(nc), "forgiven → friendly");
        assert!(!hostile.contains(sc), "old/decayed → friendly");

        let rows = list_guild_crimes(&root);
        let by: BTreeMap<&str, &GuildCrimes> =
            rows.iter().map(|r| (r.guild.as_str(), r)).collect();
        assert!(by[oc].is_hostile);
        assert!(!by[nc].is_hostile);
        assert!(!by[sc].is_hostile);
        assert_eq!(rows[0].guild, oc, "hostile guild floats to top: {rows:?}");
    }

    #[test]
    fn few_angry_members_is_not_hostile() {
        // The G1R-001 case: only 2 OldCamp members are angry (each well above the
        // per-witness threshold) — an isolated incident, NOT an area conflict.
        // Below HOSTILITY_MIN_ANGRY_MEMBERS (5) → guild is NOT hostile.
        let oc = "Guild.Human.OldCamp";
        let fresh = hours_ago(2.0);
        let payload = hostility_payload(
            vec![angry_members(oc, "OC_GRD_Guard", 2, 3, 3.0, false, false, fresh, 1)],
            NOW,
        );
        let root = parse_private_root(&payload).unwrap();
        assert!(
            !compute_hostile_guilds(&root).contains(oc),
            "2 angry members is below the area-conflict threshold → friendly"
        );
    }

    #[test]
    fn per_witness_below_threshold_is_not_angry() {
        // 6 OldCamp members, but each remembers only ONE fresh assault (accum ≈ 3 <
        // 6) → nobody is "angry" → friendly, even though there are many members.
        let oc = "Guild.Human.OldCamp";
        let fresh = hours_ago(1.0);
        let payload = hostility_payload(
            vec![angry_members(oc, "OC_GRD_Guard", 6, 1, 3.0, false, false, fresh, 1)],
            NOW,
        );
        let root = parse_private_root(&payload).unwrap();
        assert!(
            !compute_hostile_guilds(&root).contains(oc),
            "one crime per member is below the per-witness threshold → friendly"
        );
    }

    #[test]
    fn suppressed_relative_crimes_are_not_hostile() {
        // 6 members with fresh un-forgiven globals, but their relative memories are
        // SUPPRESSED → 0 angry → not hostile.
        let oc = "Guild.Human.OldCamp";
        let fresh = hours_ago(1.0);
        let payload = hostility_payload(
            vec![angry_members(oc, "OC_GRD_Guard", 6, 3, 3.0, false, true, fresh, 1)],
            NOW,
        );
        let root = parse_private_root(&payload).unwrap();
        assert!(
            !compute_hostile_guilds(&root).contains(oc),
            "suppressed memories → not hostile"
        );
    }

    #[test]
    fn forgiving_clears_hostility() {
        // 6 angry members → hostile; forgiving OldCamp clears every contribution.
        let oc = "Guild.Human.OldCamp";
        let fresh = hours_ago(2.0);
        let mut payload = hostility_payload(
            vec![angry_members(oc, "OC_GRD_Guard", 6, 3, 3.0, false, false, fresh, 1)],
            NOW,
        );
        let root = parse_private_root(&payload).unwrap();
        assert!(compute_hostile_guilds(&root).contains(oc), "starts hostile");

        apply_forgive(&mut payload, oc).unwrap();
        let root2 = parse_private_root(&payload).unwrap();
        assert_eq!(root2.consumed, payload.len(), "forgive re-parses byte-clean");
        assert!(
            !compute_hostile_guilds(&root2).contains(oc),
            "all crimes forgiven → friendly"
        );
        let r = list_guild_crimes(&root2)
            .into_iter()
            .find(|r| r.guild == oc)
            .unwrap();
        assert!(!r.is_hostile);
        assert_eq!(r.unforgiven, 0);
    }

    #[test]
    fn most_severe_tag_wins_per_entry() {
        // A single entry carrying both a murder and a theft tag counts once,
        // as a murder (the most severe).
        let oc = "Guild.Human.OldCamp";
        let mut entry = int_property("ID", 1);
        entry.extend_from_slice(&gameplay_tag_property("CrimeType", "Crime.Theft.Item"));
        entry.extend_from_slice(&bool_property("bIsForgiven", false));
        entry.extend_from_slice(&name_property("CriminalGlobalID", "Hero"));
        entry.extend_from_slice(&gameplay_tag_property("CriminalGuild", "Guild.None"));
        entry.extend_from_slice(&name_array_property("VictimGlobalIDs", &[]));
        entry.extend_from_slice(&tag_container_property("VictimGuilds", &[oc]));
        // A CrimeTags container that also lists a murder tag → most severe wins.
        entry.extend_from_slice(&tag_container_property(
            "CrimeTags",
            &["Crime.Theft.Item", "Crime.Murder.Hot"],
        ));
        entry.extend_from_slice(&name_property("TrueCriminalGlobalID", "None"));
        let payload = crime_payload(&[entry], &[]);
        let root = parse_private_root(&payload).unwrap();
        let r = &list_guild_crimes(&root)[0];
        assert_eq!(r.crimes.murder, 1, "classified as murder");
        assert_eq!(r.crimes.theft, 0, "not double-counted as theft");
    }

    // ── forgive ─────────────────────────────────────────────────────────────────

    #[test]
    fn forgive_flips_only_matching_global_and_relative_bools() {
        let mut payload = fixture();

        apply_forgive(&mut payload, "Guild.Human.OldCamp").unwrap();

        let root = parse_private_root(&payload).unwrap();
        assert_eq!(root.consumed, payload.len(), "re-parses byte-clean");

        let rows = list_guild_crimes(&root);
        let oc = rows.iter().find(|r| r.guild == "Guild.Human.OldCamp").unwrap();
        assert_eq!(oc.unforgiven, 0, "all OldCamp Hero crimes forgiven");
        assert_eq!(oc.total, 3);

        let nc = rows.iter().find(|r| r.guild == "Guild.Human.NewCamp").unwrap();
        assert_eq!(nc.unforgiven, 1, "NewCamp untouched");

        let (_, blob) = find_crime_blob(&root).unwrap();
        let forgiven_ids: Vec<i32> = global_entries(blob)
            .iter()
            .filter_map(element_props)
            .filter(|p| is_forgiven(p))
            .filter_map(|p| {
                member(p, "ID").and_then(|x| match &x.value {
                    PropertyValue::Int(v) => Some(*v),
                    _ => None,
                })
            })
            .collect();
        assert!(forgiven_ids.contains(&100));
        assert!(forgiven_ids.contains(&400));
        assert!(forgiven_ids.contains(&500));
        assert!(!forgiven_ids.contains(&200), "NewCamp crime untouched");
        assert!(!forgiven_ids.contains(&300), "non-Hero crime untouched");

        let suppressed = relative_suppressed_ids(&root);
        assert!(suppressed.contains(&100), "rel 100 suppressed");
        assert!(suppressed.contains(&500), "rel 500 suppressed");
        assert!(!suppressed.contains(&200), "rel 200 (NewCamp) NOT suppressed");
    }

    #[test]
    fn forgive_is_idempotent() {
        let mut payload = fixture();
        apply_forgive(&mut payload, "Guild.Human.OldCamp").unwrap();
        let snapshot = payload.clone();
        apply_forgive(&mut payload, "Guild.Human.OldCamp").unwrap();
        assert_eq!(payload, snapshot, "second forgive is a byte-identical no-op");
    }

    #[test]
    fn forgive_unknown_guild_is_clean_noop() {
        let mut payload = fixture();
        let before = payload.clone();
        apply_forgive(&mut payload, "Guild.Human.Nonexistent").unwrap();
        assert_eq!(payload, before, "no matching crimes => byte-identical");
    }

    fn relative_suppressed_ids(root: &RootObject) -> Vec<i32> {
        let mut out = Vec::new();
        let Some((_, blob)) = find_crime_blob(root) else {
            return out;
        };
        if let Some(PropertyValue::Map { entries, .. }) =
            member(blob, "RelativeCrimeDataEntries").map(|p| &p.value)
        {
            for (_k, v) in entries {
                let Some(props) = element_props(v) else {
                    continue;
                };
                if let Some(PropertyValue::Array { elements }) =
                    member(props, "RelativeCrimes").map(|p| &p.value)
                {
                    for rel in elements {
                        if let Some(rp) = element_props(rel) {
                            let suppressed = member(rp, "bIsSuppressed")
                                .map(|p| matches!(p.value, PropertyValue::Bool(true)))
                                .unwrap_or(false);
                            if suppressed {
                                if let Some(id) = member(rp, "ID").and_then(|p| match &p.value {
                                    PropertyValue::Int(v) => Some(*v),
                                    _ => None,
                                }) {
                                    out.push(id);
                                }
                            }
                        }
                    }
                }
            }
        }
        out
    }

    // ── real fixture ──────────────────────────────────────────────────────────

    #[test]
    #[ignore = "needs GORESAVE_G1R014_FRESH=<G1R-014 decompressed blob>"]
    fn real_g1r014_list_and_forgive_oldcamp() {
        let path = std::env::var("GORESAVE_G1R014_FRESH")
            .expect("set GORESAVE_G1R014_FRESH to the fresh G1R-014 decompressed blob");
        let mut payload = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));

        let root = parse_private_root(&payload).unwrap();
        let rows = list_guild_crimes(&root);
        let oc = rows
            .iter()
            .find(|r| r.guild == "Guild.Human.OldCamp")
            .expect("OldCamp present");
        eprintln!(
            "OldCamp: hostile={} total={} unforgiven={} crimes={:?}",
            oc.is_hostile, oc.total, oc.unforgiven, oc.crimes
        );
        // The breakdown must account for every un-forgiven crime exactly once.
        let bd = &oc.crimes;
        assert_eq!(
            bd.murder + bd.assault + bd.theft + bd.trespassing + bd.threat + bd.other,
            oc.unforgiven,
            "breakdown sums to unforgiven"
        );
        assert!(
            (310..=320).contains(&oc.total),
            "OldCamp total ~315, got {}",
            oc.total
        );
        assert!(
            (295..=305).contains(&oc.unforgiven),
            "OldCamp unforgiven ~301, got {}",
            oc.unforgiven
        );

        apply_forgive(&mut payload, "Guild.Human.OldCamp").unwrap();
        let root2 = parse_private_root(&payload).unwrap();
        assert_eq!(root2.consumed, payload.len(), "byte-clean re-parse");
        let rows2 = list_guild_crimes(&root2);
        let oc2 = rows2
            .iter()
            .find(|r| r.guild == "Guild.Human.OldCamp")
            .unwrap();
        assert_eq!(oc2.unforgiven, 0, "OldCamp unforgiven drops to 0");
        assert_eq!(oc2.total, oc.total, "total unchanged");

        let nc_before = rows.iter().find(|r| r.guild == "Guild.Human.NewCamp").unwrap();
        let nc_after = rows2.iter().find(|r| r.guild == "Guild.Human.NewCamp").unwrap();
        assert_eq!(nc_before.unforgiven, nc_after.unforgiven, "NewCamp untouched");

        let snap = payload.clone();
        apply_forgive(&mut payload, "Guild.Human.OldCamp").unwrap();
        assert_eq!(payload, snap, "second forgive byte-identical");
    }

    /// Decompress a raw GSAV `.sav` into its private payload (GSAV header →
    /// compressed stream → Oodle decode), mirroring `inspect_bytes`.
    fn load_sav_payload(path: &str) -> Vec<u8> {
        let data = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert!(data.starts_with(b"GSAV"), "{path}: not GSAV");
        let public_size = u32::from_le_bytes(data[9..13].try_into().unwrap()) as usize;
        let stream = crate::parse_compressed_stream(&data, 13 + public_size).unwrap();
        let backend = crate::codec_backend::KrakenBackend::default();
        crate::decompress_private_payload(&data, &stream, &backend).unwrap()
    }

    /// END-TO-END hostility validation against two REAL saves from the same
    /// playthrough minutes apart: in the HOSTILE save the Old-Mine guards attack
    /// (OldCamp hostile, no other camp); in the FRIENDLY save the player has
    /// pacified them (NO camp hostile). This is the ground truth the model was
    /// derived from — it must classify both correctly.
    #[test]
    #[ignore = "needs GORESAVE_HOSTILE_SAV (G1R-004) + GORESAVE_FRIENDLY_SAV (G1R-015)"]
    fn real_hostility_oldmine_vs_friendly() {
        let hostile_path = std::env::var("GORESAVE_HOSTILE_SAV").expect("set GORESAVE_HOSTILE_SAV");
        let friendly_path =
            std::env::var("GORESAVE_FRIENDLY_SAV").expect("set GORESAVE_FRIENDLY_SAV");

        let hostile_root = parse_private_root(&load_sav_payload(&hostile_path)).unwrap();
        let h = compute_hostile_guilds(&hostile_root);
        eprintln!("hostile save → hostile guilds: {h:?}");
        assert!(
            h.contains("Guild.Human.OldCamp"),
            "Old-Mine guards are hostile → OldCamp must be hostile, got {h:?}"
        );
        assert!(
            !h.contains("Guild.Human.NewCamp") && !h.contains("Guild.Human.SwampCamp"),
            "only OldCamp should be hostile, got {h:?}"
        );

        let friendly_root = parse_private_root(&load_sav_payload(&friendly_path)).unwrap();
        let f = compute_hostile_guilds(&friendly_root);
        eprintln!("friendly save → hostile guilds: {f:?}");
        assert!(f.is_empty(), "pacified save must have NO hostile guild, got {f:?}");

        // Optional: G1R-001 — only ONE Old-Mine NPC is angry (isolated incident,
        // the user confirmed no NPC attacks). Must NOT be reported hostile.
        if let Ok(few_path) = std::env::var("GORESAVE_FEWANGRY_SAV") {
            let few_root = parse_private_root(&load_sav_payload(&few_path)).unwrap();
            let few = compute_hostile_guilds(&few_root);
            eprintln!("few-angry save → hostile guilds: {few:?}");
            assert!(
                few.is_empty(),
                "a lone angry NPC must NOT make the camp hostile, got {few:?}"
            );
        }
    }
}
