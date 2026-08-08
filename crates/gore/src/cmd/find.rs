//! `gore find` — one lookup across both layers: the bundled catalogs, which say
//! what exists, and the effect register, which says what it does in game.
//!
//! # Why one command rather than a `find` per family
//!
//! A person arrives with a thing, not with a namespace. "The healing potion",
//! "the main menu logo", "Diego". Which of `gore texture`, `gore loc`, `gore as`
//! or `gore audio` owns that thing is the answer, not the question — so asking
//! it as a precondition puts the whole toolkit behind a guess. This searches
//! every namespace at once and says which one each hit came from.
//!
//! Both layers are compiled into the binary, so this answers with no game
//! installation, no dump and no generation step — the same reason
//! [`gore_catalog::location`] embeds its catalog, and the same ~1 MB cost.
//!
//! # The name problem, which this command must not paper over
//!
//! The bundled catalogs carry class ids, categories and asset paths. They do
//! **not** carry display names. `ItFo_Potion_Health_01` is "Essenz heilender
//! Kraft" only inside the game's encrypted localization cache, which is not
//! ours to ship — and "find me the healing potion" is exactly what the
//! blind-user test showed people type.
//!
//! So the display names come from the shared loc catalog, which exists only
//! once a user has run `gore loc extract`. When it is there, names are searched
//! and the report says so. When it is not, the report says *that*, in a line
//! that names the command that would fix it, and it says it whether the search
//! found something or nothing. A search that quietly omitted the name index
//! would answer "no such item" about an item that is right there — the one
//! failure this command exists to prevent, and the one a confident-looking
//! empty result hides best.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Result};
use gore_catalog::register::{Entry, Outcome, Register, Status};
use gore_loc::paths;
use serde::de::{DeserializeSeed, IgnoredAny, MapAccess, Visitor};
use serde::Deserialize;

// ─── Bundled catalogs ────────────────────────────────────────────────────────

/// The three catalogs `gore catalog` generates, compiled in.
///
/// Read out of the save editor's asset directory rather than copied next to
/// this file: that is where `gore catalog --kind …` writes them and where
/// `gore-save` already `include_str!`s them from, and a second copy would be a
/// second thing to regenerate after a game patch. (The effect register is the
/// other way round — it lives in `gore-catalog/register/` because no app owns
/// it — see `gore_catalog::register`.)
const ITEM_CATALOG_JSON: &str = include_str!("../../../../apps/save-editor/assets/item_catalog.json");
const NPC_CATALOG_JSON: &str = include_str!("../../../../apps/save-editor/assets/npc_catalog.json");
const KNOWLEDGE_CATALOG_JSON: &str =
    include_str!("../../../../apps/save-editor/assets/knowledge_catalog.json");

/// The domains a bundled catalog covers. The register's domains are its own and
/// come from [`Register::domains`]; both are one vocabulary to `--domain`,
/// because a domain names an id namespace rather than a command.
const CATALOG_DOMAINS: &[&str] = &["item", "npc", "knowledge"];

/// Default for `--max`. Lower than `location list`'s 200 because a hit here is a
/// block of several lines rather than one, so 50 already fills a screen twice.
pub const DEFAULT_MAX: usize = 50;

/// Localization columns to show a name from when nothing better is known,
/// newest first.
///
/// The same chain the save editor uses (`kEnglishLocSets` in
/// `apps/save-editor/lib/loc/game_lang.dart`): the game ships three English
/// columns and later ones override earlier ones. A hit that matched a *specific*
/// language is shown in that language instead — somebody who typed German is
/// answered in German without a flag.
const PREFERRED_LANGUAGES: &[&str] = &["english_newer", "english_new", "english"];

/// One row of a bundled catalog, reduced to the shape this command searches.
///
/// The three catalogs have three different schemas; folding them here rather
/// than matching per-schema is what keeps one query able to cross all of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    pub domain: &'static str,
    pub id: String,
    pub category: String,
    /// The fully-qualified name the game resolves: an item's `path`, an NPC's
    /// `class`. `None` for knowledge, which has no class of its own.
    pub class: Option<String>,
    /// Knowledge only: the conversation module it is declared in.
    pub module: Option<String>,
    /// Knowledge only: the localization key its text lives under. Knowledge ids
    /// have no loc entry of their own, so this is the only route to a name for
    /// them.
    pub loc_key: Option<String>,
    /// Knowledge only, and only for some: a caption the catalog generator
    /// recovered from the script cache. Bundled text, available with no loc
    /// catalog at all.
    pub caption: Option<String>,
}

#[derive(Deserialize)]
struct ItemRow {
    id: String,
    category: String,
    path: String,
}

#[derive(Deserialize)]
struct NpcRow {
    id: String,
    category: String,
    class: String,
}

#[derive(Deserialize)]
struct KnowledgeRow {
    id: String,
    category: String,
    #[serde(default)]
    module: Option<String>,
    #[serde(default)]
    loc_key: Option<String>,
    #[serde(default)]
    caption: Option<String>,
}

/// Every bundled catalog row, item then NPC then knowledge.
///
/// Parsed on every run rather than lazily cached: ~5,800 rows out of ~1 MB of
/// JSON is a few milliseconds, and a cache would be a second thing that can be
/// stale.
pub fn bundled_catalog() -> Result<Vec<CatalogEntry>> {
    let items: Vec<ItemRow> =
        serde_json::from_str(ITEM_CATALOG_JSON).map_err(|error| catalog_error("item", error))?;
    let npcs: Vec<NpcRow> =
        serde_json::from_str(NPC_CATALOG_JSON).map_err(|error| catalog_error("npc", error))?;
    let knowledge: Vec<KnowledgeRow> = serde_json::from_str(KNOWLEDGE_CATALOG_JSON)
        .map_err(|error| catalog_error("knowledge", error))?;

    let mut entries = Vec::with_capacity(items.len() + npcs.len() + knowledge.len());
    entries.extend(items.into_iter().map(|row| CatalogEntry {
        domain: "item",
        id: row.id,
        category: row.category,
        class: Some(row.path),
        module: None,
        loc_key: None,
        caption: None,
    }));
    entries.extend(npcs.into_iter().map(|row| CatalogEntry {
        domain: "npc",
        id: row.id,
        category: row.category,
        class: Some(row.class),
        module: None,
        loc_key: None,
        caption: None,
    }));
    entries.extend(knowledge.into_iter().map(|row| CatalogEntry {
        domain: "knowledge",
        id: row.id,
        category: row.category,
        class: None,
        module: row.module,
        loc_key: row.loc_key,
        caption: row.caption,
    }));
    Ok(entries)
}

fn catalog_error(kind: &str, error: serde_json::Error) -> anyhow::Error {
    anyhow::anyhow!(
        "the bundled {kind} catalog compiled into this binary is not readable: {error}. \
         Regenerate it with `gore catalog --kind {kind}` and rebuild"
    )
}

// ─── The display-name index ──────────────────────────────────────────────────

/// One localized spelling of an id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    /// The loc catalog's own column name — `german`, `english_new`, and so on.
    /// Printed with the name, so nobody reads a German answer as English.
    pub language: String,
    pub text: String,
}

/// Display names for the ids this command can annotate.
#[derive(Debug, Clone, Default)]
pub struct NameIndex {
    /// Lowercased loc id -> its non-empty columns, in catalog order.
    names: HashMap<String, Vec<Name>>,
    /// Ids in the shared catalog as a whole, not just the kept ones.
    total_ids: usize,
    languages: BTreeSet<String>,
}

impl NameIndex {
    /// Build one from `(id, language, text)` triples, without a shared loc
    /// catalog on disk. The real loader is [`load_name_index`]; this exists so a
    /// test can pin the degradation without depending on whether the machine
    /// running it has ever run `gore loc extract`.
    #[cfg(test)]
    fn from_pairs(pairs: &[(&str, &str, &str)]) -> Self {
        let mut index = NameIndex::default();
        for (id, language, text) in pairs {
            index.languages.insert((*language).to_string());
            index
                .names
                .entry(id.to_lowercase())
                .or_default()
                .push(Name { language: (*language).to_string(), text: (*text).to_string() });
        }
        index.total_ids = index.names.len();
        index
    }

    fn get(&self, id: &str) -> Option<&[Name]> {
        self.names.get(&id.to_lowercase()).map(Vec::as_slice)
    }

    /// How many CATALOG ROWS this index can name — the honest coverage number,
    /// which is well short of all of them (knowledge ids have no loc entry at
    /// all, and some 47 item classes are unnamed in every language).
    ///
    /// Rows, not ids, because the sentence this feeds says "of N catalog
    /// entries" and N is `catalog.len()`. Counting the deduplicated `wanted` set
    /// instead answered a different question in both directions at once: 232
    /// knowledge rows share `text_dialog_end` and counted once between them,
    /// while register-only ids counted although they are not catalog rows at
    /// all.
    fn covering(&self, catalog: &[CatalogEntry]) -> usize {
        catalog
            .iter()
            .filter(|entry| self.names.contains_key(&entry.loc_id().to_lowercase()))
            .count()
    }
}

/// Whether display names could be searched, and why not when they could not.
#[derive(Debug, Clone)]
pub enum NameIndexState {
    /// Loaded; names were searched.
    Ready(NameIndex),
    /// No shared loc catalog on this machine yet.
    Absent,
    /// One is there but could not be read. Deliberately distinct from
    /// [`NameIndexState::Absent`]: "run `gore loc extract`" is the wrong advice
    /// for a catalog that exists and is broken, and reporting a read failure as
    /// an absence would hide it forever.
    Unreadable { path: PathBuf, detail: String },
}

impl NameIndexState {
    fn index(&self) -> Option<&NameIndex> {
        match self {
            NameIndexState::Ready(index) => Some(index),
            _ => None,
        }
    }

    /// The one line every report carries about the name index, hit or no hit.
    ///
    /// Takes the catalog rather than a count and a set that have to agree: the
    /// numerator and the denominator are now read off the same slice, so they
    /// cannot drift apart again.
    fn notice(&self, catalog: &[CatalogEntry]) -> String {
        let catalog_entries = catalog.len();
        match self {
            NameIndexState::Ready(index) => format!(
                "display names: searched — {} of {catalog_entries} catalog entries have one \
                 (shared loc catalog: {} ids, {} languages)",
                index.covering(catalog),
                index.total_ids,
                index.languages.len()
            ),
            NameIndexState::Absent => format!(
                "display names: NOT searched — the bundled catalogs carry class ids and \
                 categories, not names. Run `gore loc extract` once to search names too; until \
                 then a word that appears in no id cannot match ({} is not there yet)",
                paths::loc_catalog_path().display()
            ),
            NameIndexState::Unreadable { path, detail } => format!(
                "display names: NOT searched — the shared loc catalog at {} could not be read: \
                 {detail}. Re-run `gore loc extract` to rebuild it",
                path.display()
            ),
        }
    }

    fn searched(&self) -> bool {
        matches!(self, NameIndexState::Ready(_))
    }
}

/// Read the shared loc catalog, keeping only the ids in `wanted`.
///
/// The catalog is ~28 MB — 43,851 ids in 14 columns — and this command can name
/// at most the ~9,700 ids the catalogs and the register between them mention.
/// Materializing the other 34,000 would be by far the most expensive thing a
/// `gore find` does, so the value of every unwanted key is skipped with
/// [`IgnoredAny`] instead of being built and dropped.
///
/// A missing or unreadable catalog is not an error: it is the degraded mode this
/// command is designed to report rather than fail on.
pub fn load_name_index(wanted: &HashSet<String>) -> NameIndexState {
    load_name_index_at(paths::loc_catalog_path(), wanted)
}

/// [`load_name_index`] against a named path, so the three states can be exercised without
/// depending on whether the machine running the tests has ever run `gore loc extract`.
fn load_name_index_at(path: PathBuf, wanted: &HashSet<String>) -> NameIndexState {
    // Read first and classify the failure, rather than asking `catalog_present()` — which is
    // `Path::is_file()`, false both for "no catalog yet" and for "could not tell". A catalog behind
    // an unreadable directory was reported as never extracted, and the advice that follows from
    // that is `gore loc extract`, which will fail for the same reason nobody has mentioned.
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return NameIndexState::Absent;
        }
        Err(error) => {
            return NameIndexState::Unreadable { path, detail: error.to_string() };
        }
    };
    match parse_name_index(&text, wanted) {
        Ok(index) => NameIndexState::Ready(index),
        Err(error) => NameIndexState::Unreadable { path, detail: error.to_string() },
    }
}

/// The parse behind [`load_name_index`], separated from the path so it can be exercised directly.
///
/// `end()` is the reason this is not a one-liner. `serde_json::from_str` checks that nothing
/// follows the value it read; driving a [`DeserializeSeed`] over a `Deserializer` by hand does not,
/// so a catalog consisting of one good object and then garbage — a truncated rewrite, two files
/// concatenated — deserialized happily and `gore find` reported that display names had been
/// searched. Reading half a catalog and calling it a catalog is the same failure this command
/// reports everywhere else, one layer down.
fn parse_name_index(text: &str, wanted: &HashSet<String>) -> serde_json::Result<NameIndex> {
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let index = (WantedNames { wanted }).deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(index)
}

/// Deserialization seed that keeps only the ids it was asked for.
struct WantedNames<'a> {
    wanted: &'a HashSet<String>,
}

impl<'de> DeserializeSeed<'de> for WantedNames<'_> {
    type Value = NameIndex;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for WantedNames<'_> {
    type Value = NameIndex;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the shared loc catalog, as {id: {language: text}}")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut index = NameIndex::default();
        while let Some(id) = map.next_key::<String>()? {
            index.total_ids += 1;
            // Folded the same way the save editor folds it, and the same way the
            // wanted set was built: the catalog's own keys are lowercase today,
            // but nothing in the extractor promises that they stay so.
            let id = id.to_lowercase();
            if !self.wanted.contains(&id) {
                map.next_value::<IgnoredAny>()?;
                continue;
            }
            let columns: BTreeMap<String, String> = map.next_value()?;
            let names: Vec<Name> = columns
                .into_iter()
                // An empty column is a translation nobody wrote. Keeping it would
                // let a hit print a blank name and claim it as the answer.
                .filter(|(_, text)| !text.trim().is_empty())
                .map(|(language, text)| {
                    index.languages.insert(language.clone());
                    Name { language, text }
                })
                .collect();
            if !names.is_empty() {
                index.names.insert(id, names);
            }
        }
        Ok(index)
    }
}

// ─── Matching ────────────────────────────────────────────────────────────────

/// Where a hit's query terms were found. Printed so a result that does not
/// visibly contain the query still explains itself.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Matched {
    Id,
    Category,
    Class,
    Module,
    LocKey,
    Caption,
    /// A display name, in the named language.
    Name(String),
    /// The register's `id`, `effect` or `note` — never its `witness`, which is
    /// verbatim speech about something else as often as not.
    Register,
}

impl Matched {
    fn label(&self) -> String {
        match self {
            Matched::Id => "id".to_string(),
            Matched::Category => "category".to_string(),
            Matched::Class => "class".to_string(),
            Matched::Module => "module".to_string(),
            Matched::LocKey => "loc key".to_string(),
            Matched::Caption => "caption".to_string(),
            Matched::Name(language) => format!("display name ({language})"),
            Matched::Register => "register text".to_string(),
        }
    }
}

/// One id, everything both layers know about it, and why it is in the result.
#[derive(Debug, Clone)]
pub struct Hit<'a> {
    pub domain: String,
    pub id: String,
    /// The bundled catalog row, when one carries this id.
    pub catalog: Option<&'a CatalogEntry>,
    /// Register entries for this id, in provenance order. Empty for the
    /// overwhelming majority — an id nobody has looked at yet, which is not the
    /// same as an id that does nothing.
    pub register: Vec<&'a Entry>,
    /// The name to print, when one is known.
    pub name: Option<Name>,
    /// Where the name came from, for the label beside it.
    pub name_source: NameSource,
    pub matched: Vec<Matched>,
    /// The query is this id, spelled exactly. Sorted to the front.
    pub exact: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameSource {
    None,
    /// From the shared loc catalog.
    Localization,
    /// From the knowledge catalog's own `caption` field — bundled text, which
    /// is why it survives with no loc catalog at all.
    CatalogCaption,
}

/// Case-insensitive substring test, folded with `to_lowercase`.
///
/// Past ASCII on purpose, and for the reason `cmd::contains_case_insensitive`
/// gives: the text being searched here is German, Russian and Japanese item
/// names, and `--filter MÜLLER` matching nothing in a catalog that has `Müller`
/// is a false negative nobody can debug from the outside. `needle` arrives
/// already folded, once per run.
fn contains(haystack: &str, folded_needle: &str) -> bool {
    haystack.to_lowercase().contains(folded_needle)
}

/// Every term must match somewhere in the entry, so a second word narrows the
/// result instead of widening it.
///
/// The alternative — one needle, matched whole — is what `Register::search`
/// takes, and it answers `gore find healing potion` with nothing while
/// `gore find healing` answers with twelve items, because no name contains the
/// two words in that order. "Find me the healing potion" is the sentence this
/// command was built for.
fn match_catalog(
    entry: &CatalogEntry,
    terms: &[String],
    index: Option<&NameIndex>,
) -> Option<Vec<Matched>> {
    let names = index.and_then(|index| index.get(entry.loc_id())).unwrap_or(&[]);
    let mut found: BTreeSet<Matched> = BTreeSet::new();
    for term in terms {
        let mut here: Vec<Matched> = Vec::new();
        if contains(&entry.id, term) {
            here.push(Matched::Id);
        }
        if contains(&entry.category, term) {
            here.push(Matched::Category);
        }
        // Only when the id did *not* already answer for this term. An item's
        // class is `/Script/Angelscript.<id>` and an NPC's is
        // `CharacterDefinition_<kind>_<id>`, so the class contains the id and
        // reporting both would put "class" on the explanation line of nearly
        // every hit while explaining nothing. It still earns its place on its
        // own: `CharacterDefinition_Human_` matches no id at all.
        if !here.contains(&Matched::Id)
            && entry.class.as_deref().is_some_and(|class| contains(class, term))
        {
            here.push(Matched::Class);
        }
        if entry.module.as_deref().is_some_and(|module| contains(module, term)) {
            here.push(Matched::Module);
        }
        if entry.loc_key.as_deref().is_some_and(|key| contains(key, term)) {
            here.push(Matched::LocKey);
        }
        if entry.caption.as_deref().is_some_and(|caption| contains(caption, term)) {
            here.push(Matched::Caption);
        }
        for name in names {
            if contains(&name.text, term) {
                here.push(Matched::Name(name.language.clone()));
            }
        }
        if here.is_empty() {
            return None;
        }
        found.extend(here);
    }
    Some(found.into_iter().collect())
}

impl CatalogEntry {
    /// The localization id this entry's name lives under.
    ///
    /// The id itself for items and NPCs (`ItFo_Apple` -> `itfo_apple`), and the
    /// `loc_key` for knowledge, whose ids (`Choice62749`) are generated and
    /// appear nowhere in the localization at all.
    fn loc_id(&self) -> &str {
        match (self.domain, &self.loc_key) {
            ("knowledge", Some(key)) => key,
            _ => &self.id,
        }
    }
}

// ─── The command ─────────────────────────────────────────────────────────────

pub fn run(query: Vec<String>, domain: Option<String>, max: usize, json: bool) -> Result<()> {
    // Several words are one query, exactly as `gore guide search` takes them, so
    // a phrase needs no quoting on any shell.
    let query = query.join(" ");
    let terms = query_terms(&query);
    if terms.is_empty() {
        bail!("nothing to search for — pass one or more words, e.g. `gore find healing potion`");
    }

    let register = Register::bundled()?;
    let catalog = bundled_catalog()?;

    if let Some(domain) = domain.as_deref() {
        known_domain(domain, &register)?;
    }
    let catalog = catalog_in_domain(catalog, domain.as_deref());

    let wanted = wanted_loc_ids(&catalog, &register);
    let name_index = load_name_index(&wanted);

    let hits = search(&catalog, &register, &terms, name_index.index(), domain.as_deref());
    let report = Report {
        query: &query,
        domain: domain.as_deref(),
        catalog: &catalog,
        register: &register,
        name_index: &name_index,
        listed: hits.len().min(max),
        hits: &hits,
    };
    println!("{}", if json { report.json()? } else { report.text() });
    // A search that matched nothing is a real answer to a real question, not a
    // failure — the same posture `location list` takes, and the one that keeps
    // the "display names were not searched" line as the last word rather than
    // burying it under `error:`.
    Ok(())
}

/// The words of a query, lowercased, with surrounding punctuation taken off.
///
/// Every term has to match, so one stray character removes a result entirely: `gore find healing
/// potion?` searched for `potion?`, which is in no id and no display name, and answered that
/// nothing matches. Trimmed only at the edges — punctuation inside a word belongs to it, and an id
/// like `/Game/UI/T_Logo` or `ItFo_Apple` is mostly punctuation.
fn query_terms(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|c: char| c.is_ascii_punctuation() && c != '_')
                .to_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect()
}

/// The catalog rows a search can reach, which with `--domain` is not all of them.
///
/// Scoped once, here, rather than only inside `search`: every number the report prints is read off
/// this slice, and `--domain texture` used to claim it had searched all ~9,700 bundled entries
/// while skipping every item, NPC and knowledge row in them.
fn catalog_in_domain(mut catalog: Vec<CatalogEntry>, domain: Option<&str>) -> Vec<CatalogEntry> {
    if let Some(domain) = domain {
        catalog.retain(|entry| entry.domain.eq_ignore_ascii_case(domain));
    }
    catalog
}

/// Every localization id this run could possibly need a name for.
///
/// Built before the loc catalog is opened, because it is what makes opening it
/// cheap: see [`load_name_index`].
fn wanted_loc_ids(catalog: &[CatalogEntry], register: &Register) -> HashSet<String> {
    let mut wanted: HashSet<String> = catalog
        .iter()
        .map(|entry| entry.loc_id().to_lowercase())
        .collect();
    wanted.extend(register.entries().map(|entry| entry.id.to_lowercase()));
    wanted
}

/// Refuse a `--domain` nothing carries, naming the ones that exist.
///
/// A domain filter nobody has matches nothing, which reads exactly like a
/// namespace that happens to be empty — the same trap `location list --area`
/// falls into, answered the same way.
fn known_domain(domain: &str, register: &Register) -> Result<()> {
    let mut known: BTreeSet<&str> = CATALOG_DOMAINS.iter().copied().collect();
    known.extend(register.domains());
    if known.iter().any(|known| known.eq_ignore_ascii_case(domain)) {
        return Ok(());
    }
    bail!(
        "no domain '{domain}' — the bundled catalogs and the effect register carry: {}",
        known.into_iter().collect::<Vec<_>>().join(", ")
    );
}

/// Match both layers and merge them by id.
///
/// One id is one hit, however many layers carry it: the register annotates the
/// catalog rather than shadowing it, and printing the same id twice under two
/// headings would read as two things.
///
/// Ordering is by evidence strength (see [`rank`]), then domain, then id.
/// Register entries inside a hit keep the provenance order `Register::lookup`
/// returns them in, and each one prints its own provenance — so sorting the
/// *hits* never blends two sources into one unlabelled list.
pub fn search<'a>(
    catalog: &'a [CatalogEntry],
    register: &'a Register,
    terms: &[String],
    index: Option<&NameIndex>,
    domain: Option<&str>,
) -> Vec<Hit<'a>> {
    let in_scope = |candidate: &str| match domain {
        Some(wanted) => candidate.eq_ignore_ascii_case(wanted),
        None => true,
    };

    // `matching_register` indexes `terms[0]`, and a caller outside this module
    // could hand over an empty query. An empty query selects nothing, which is
    // also what `run` refuses one call earlier.
    if terms.is_empty() {
        return Vec::new();
    }

    let mut hits: Vec<Hit<'a>> = Vec::new();
    let mut seen: HashMap<(String, String), usize> = HashMap::new();

    for entry in catalog.iter().filter(|entry| in_scope(entry.domain)) {
        let Some(matched) = match_catalog(entry, terms, index) else {
            continue;
        };
        let key = (entry.domain.to_string(), entry.id.to_lowercase());
        seen.insert(key, hits.len());
        hits.push(hit_for(entry, register, index, matched, terms));
    }

    // The register is annotation, so an entry that matched on its own text still
    // belongs to whatever catalog row carries the same id: merged into that hit
    // when there is one, its own hit when there is not (textures, loc keys and
    // FMOD samples have no bundled catalog at all).
    for entry in matching_register(register, terms, domain, index) {
        let why = register_match(entry, terms, register, index);
        let carried = catalog
            .iter()
            .find(|row| row.domain == entry.domain && row.id.eq_ignore_ascii_case(&entry.id));
        // Two keys, because two different questions are being asked. Against a catalog row the
        // fold is right: the register may spell an id differently from the catalog and still mean
        // that row. Between register entries it is wrong — `Register::lookup` says in as many
        // words that two ids differing only by case are two ids, and folding them here dropped the
        // second one's id and its observations while keeping only its match reasons.
        let key = match carried {
            Some(row) => (row.domain.to_string(), row.id.to_lowercase()),
            None => (entry.domain.clone(), entry.id.clone()),
        };
        if let Some(position) = seen.get(&key) {
            let hit = &mut hits[*position];
            for reason in why {
                if !hit.matched.contains(&reason) {
                    hit.matched.push(reason);
                }
            }
            continue;
        }
        seen.insert(key, hits.len());
        match carried {
            Some(row) => hits.push(hit_for(row, register, index, why, terms)),
            None => {
                let name = index
                    .and_then(|index| index.get(&entry.id))
                    .and_then(|names| choose_name(names, terms))
                    .cloned();
                hits.push(Hit {
                    domain: entry.domain.clone(),
                    id: entry.id.clone(),
                    catalog: None,
                    register: register.lookup(&entry.id, Some(&entry.domain)),
                    name_source: match name {
                        Some(_) => NameSource::Localization,
                        None => NameSource::None,
                    },
                    name,
                    matched: why,
                    exact: terms.len() == 1 && entry.id.eq_ignore_ascii_case(&terms[0]),
                });
            }
        }
    }

    hits.sort_by(|left, right| {
        rank(left)
            .cmp(&rank(right))
            .then_with(|| left.domain.cmp(&right.domain))
            .then_with(|| left.id.cmp(&right.id))
    });
    hits
}

/// How strong a hit's best evidence is, lowest first.
///
/// Searching every shipped language is what makes "find me the healing potion"
/// work, and it is also what makes `gore find logo` drag in five Portuguese
/// dialog lines that happen to contain the word — while the texture actually
/// called `T_Logo` sorts under `t` and lands below them. A name match in a line
/// of dialogue is the weakest signal there is; an id somebody typed almost
/// exactly is the strongest. Nothing is dropped, only ordered, so a
/// `--max`-clipped result still shows the likely answer first.
fn rank(hit: &Hit<'_>) -> u8 {
    if hit.exact {
        return 0;
    }
    if hit.matched.contains(&Matched::Id) {
        return 1;
    }
    // An id is in the register because a person looked at it, which is a
    // stronger reason to show it than any lexical accident.
    if hit.matched.contains(&Matched::Register) {
        return 2;
    }
    if hit
        .matched
        .iter()
        .any(|matched| !matches!(matched, Matched::Name(_)))
    {
        return 3;
    }
    4
}

/// Register entries matching every term.
///
/// Intersected across `Register::search` calls rather than re-implemented, so
/// which fields the register searches — id, effect and note, never witness —
/// stays decided in one place. Entries are compared by address because they all
/// come from the same `Register`, and two sources may legitimately hold entries
/// that are equal in every field.
/// Which of a register entry's fields answered for the query.
///
/// `Register::search` looks in id, effect and note and does not say which one it
/// found something in — right for the register, which owes the caller entries
/// rather than an explanation. Here it matters twice: an id match is what makes
/// `/Game/UI/Textures/Common/T_Logo` outrank a subtitle containing the word
/// "logo", and "matched: register text" printed under an id the query is
/// visibly inside would explain nothing.
fn register_match(
    entry: &Entry,
    terms: &[String],
    register: &Register,
    index: Option<&NameIndex>,
) -> Vec<Matched> {
    // Every reason, not the first one that fits. `matching_register` admits an entry when each
    // term is answered by its id, its register text OR a display name, so a multi-word query can be
    // split across all three — and naming only one of them both drops most of the explanation and
    // costs the entry its rank, which is what decides who survives `--max`.
    let mut reasons = Vec::new();
    if terms.iter().any(|term| contains(&entry.id, term)) {
        reasons.push(Matched::Id);
    }

    // Only the terms the id did not already answer for. `Register::search` looks in the id as well
    // as the effect and the note, so without this every id match would also be labelled "register
    // text" — which explains the wrong thing about the strongest kind of hit there is.
    let rest: Vec<String> = terms
        .iter()
        .filter(|term| !contains(&entry.id, term))
        .cloned()
        .collect();

    let answered_by_register = rest.iter().any(|term| {
        register
            .search(term, Some(&entry.domain))
            .iter()
            .any(|other| std::ptr::eq(*other, entry))
    });
    if answered_by_register {
        reasons.push(Matched::Register);
    }
    if let Some(name) = index
        .and_then(|index| index.get(&entry.id))
        .and_then(|names| matching_name(names, &rest))
    {
        reasons.push(Matched::Name(name.language.clone()));
    }
    if reasons.is_empty() {
        // Unreachable through `matching_register`, which admits an entry only for one of the two
        // reasons above. Keeps the function total without inventing a language nobody matched in.
        reasons.push(Matched::Register);
    }
    reasons
}

/// Register entries matching every term, against the entry's own text OR its display name.
///
/// The name index has to be consulted here rather than only when a hit is rendered. An id that
/// exists in the register and in no bundled catalog — a texture path, an FMOD sample, a loc key —
/// is reachable only through this function, so searching just id, effect and note meant
/// `gore find <localized text>` answered "nothing matches" while the same run reported that display
/// names had been searched. Saying a name index was used and then not using it is worse than not
/// having one.
fn matching_register<'a>(
    register: &'a Register,
    terms: &[String],
    domain: Option<&str>,
    index: Option<&NameIndex>,
) -> Vec<&'a Entry> {
    let named = |entry: &Entry, term: &str| {
        index
            .and_then(|index| index.get(&entry.id))
            .is_some_and(|names| {
                names
                    .iter()
                    .any(|name| super::contains_case_insensitive(&name.text, &term.to_lowercase()))
            })
    };

    let mut matched: Vec<&Entry> = register
        .in_domain(domain)
        .into_iter()
        .filter(|entry| {
            terms.iter().all(|term| {
                register
                    .search(term, domain)
                    .iter()
                    .any(|other| std::ptr::eq(*entry, *other))
                    || named(entry, term)
            })
        })
        .collect();
    matched.dedup_by(|a, b| std::ptr::eq(*a, *b));
    matched
}

fn hit_for<'a>(
    entry: &'a CatalogEntry,
    register: &'a Register,
    index: Option<&NameIndex>,
    matched: Vec<Matched>,
    terms: &[String],
) -> Hit<'a> {
    let names = index.and_then(|index| index.get(entry.loc_id()));
    let (name, name_source) = match names.and_then(|names| choose_name(names, terms)) {
        Some(name) => (Some(name.clone()), NameSource::Localization),
        // A knowledge caption is bundled text: it is the only name most of the
        // knowledge catalog can ever have without a loc catalog.
        None => match &entry.caption {
            Some(caption) => (
                Some(Name { language: String::new(), text: caption.clone() }),
                NameSource::CatalogCaption,
            ),
            None => (None, NameSource::None),
        },
    };
    Hit {
        domain: entry.domain.to_string(),
        id: entry.id.clone(),
        catalog: Some(entry),
        register: register.lookup(&entry.id, Some(entry.domain)),
        name,
        name_source,
        matched,
        exact: terms.len() == 1 && entry.id.eq_ignore_ascii_case(&terms[0]),
    }
}

/// The name to print: the language the query matched, else the newest English
/// column, else whatever exists.
///
/// Answering in the language somebody typed costs no flag and is almost always
/// what they meant — a German search for `Apfel` that came back "Apple" would
/// leave them checking whether it is even the same item.
/// The name a term is actually inside, or nothing.
///
/// [`choose_name`] falls back to a preferred language so a hit always has something to print, which
/// makes it the wrong question to ask when deciding WHY an entry matched: it answers even when no
/// name did.
fn matching_name<'a>(names: &'a [Name], terms: &[String]) -> Option<&'a Name> {
    names
        .iter()
        .find(|name| terms.iter().any(|term| contains(&name.text, term)))
}

fn choose_name<'a>(names: &'a [Name], terms: &[String]) -> Option<&'a Name> {
    if let Some(name) = matching_name(names, terms) {
        return Some(name);
    }
    for preferred in PREFERRED_LANGUAGES {
        if let Some(name) = names.iter().find(|name| name.language == *preferred) {
            return Some(name);
        }
    }
    names.first()
}

// ─── Reports ─────────────────────────────────────────────────────────────────

/// Label column of a hit's detail lines. The longest label is `register` (8).
const LABEL: usize = 9;

/// One finished search, ready to be written out either way.
///
/// A struct rather than eight arguments repeated across two renderers, so the
/// text and the JSON cannot end up describing different searches — which is
/// exactly the drift the name-index notice must not suffer.
pub struct Report<'a> {
    pub query: &'a str,
    pub domain: Option<&'a str>,
    pub catalog: &'a [CatalogEntry],
    pub register: &'a Register,
    pub name_index: &'a NameIndexState,
    pub hits: &'a [Hit<'a>],
    /// How many of `hits` are printed. Never more than `hits.len()`.
    pub listed: usize,
}

impl Report<'_> {
    /// Register entries this search could reach: the whole register, or one domain of it.
    ///
    /// The catalog arrives already scoped (see `run`), the register does not — it is shared by
    /// reference and filtered per match. Counting all of it beside a scoped catalog would describe
    /// two different searches in one sentence.
    fn register_searched(&self) -> usize {
        self.register.in_domain(self.domain).len()
    }

    fn text(&self) -> String {
        let query = self.query;
        let from_catalog = self.hits.iter().filter(|hit| hit.catalog.is_some()).count();
        let annotated = self.hits.iter().filter(|hit| !hit.register.is_empty()).count();

        let mut out = if self.hits.is_empty() {
            format!(
                "nothing matches {query:?} — searched {} bundled catalog entries and {} \
                 effect-register entries\n",
                self.catalog.len(),
                self.register_searched()
            )
        } else {
            format!(
                "{} hit(s) for {query:?} — {from_catalog} in the bundled catalogs, {annotated} \
                 annotated by the effect register\n",
                self.hits.len()
            )
        };
        // Always, hit or no hit: an empty result read as exhaustive is the
        // failure this line exists to prevent, and that is exactly the case
        // where a line about what was *not* searched is easiest to leave out.
        out.push_str(&self.name_index.notice(self.catalog));
        out.push('\n');
        if self.register.is_empty() {
            out.push_str(
                "effect register: empty in this build — no id has an observed effect recorded \
                 yet\n",
            );
        }

        for hit in &self.hits[..self.listed] {
            out.push('\n');
            out.push_str(&hit_block(hit));
        }

        if self.listed < self.hits.len() {
            // The same marker `location list` and the MCP server append to a
            // clipped result, so one learned habit covers all three.
            let narrow = match self.domain {
                Some(_) => "Narrow the query",
                None => "Narrow the query or add --domain",
            };
            out.push_str(&format!(
                "\n… [truncated: {} hits matched and only the first {} are shown. {narrow}, or \
                 raise --max]\n",
                self.hits.len(),
                self.listed
            ));
        }
        out
    }

    fn json(&self) -> Result<String> {
        let hits: Vec<serde_json::Value> =
            self.hits[..self.listed].iter().map(json_hit).collect();
        let document = serde_json::json!({
            "query": self.query,
            "domain": self.domain,
            "catalog_entries": self.catalog.len(),
            "register_entries": self.register_searched(),
            "matched_count": self.hits.len(),
            "listed_count": self.listed,
            "truncated": self.listed < self.hits.len(),
            "name_index": {
                "searched": self.name_index.searched(),
                // The same sentence the text report prints, so a client that
                // renders only the JSON cannot show a search that looks
                // exhaustive when the name index was missing.
                "notice": self.name_index.notice(self.catalog),
            },
            "hits": hits,
        });
        Ok(serde_json::to_string_pretty(&document)?)
    }
}

fn hit_block(hit: &Hit<'_>) -> String {
    let mut out = format!("{}\n", hit.id);

    match hit.catalog {
        Some(entry) if entry.category.is_empty() => {
            line(&mut out, "from", &format!("bundled catalog · {}", entry.domain));
        }
        Some(entry) => line(
            &mut out,
            "from",
            &format!("bundled catalog · {} · {}", entry.domain, entry.category),
        ),
        // Said outright rather than left to inference: textures, loc keys, FMOD
        // samples and voice lines have no bundled catalog at all, so "not in a
        // catalog" here does not mean the id is unknown to the game.
        None => line(
            &mut out,
            "from",
            &format!(
                "effect register only ({}) — no bundled catalog covers this namespace",
                hit.domain
            ),
        ),
    }

    if let Some(name) = &hit.name {
        let label = match hit.name_source {
            NameSource::Localization => format!("{} ({})", name.text, name.language),
            NameSource::CatalogCaption => format!("{} (catalog caption)", name.text),
            NameSource::None => name.text.clone(),
        };
        line(&mut out, "name", &label);
    }
    if let Some(entry) = hit.catalog {
        if let Some(class) = &entry.class {
            line(&mut out, "class", class);
        }
        if let Some(module) = &entry.module {
            line(&mut out, "module", module);
        }
        if let Some(key) = &entry.loc_key {
            line(&mut out, "loc key", key);
        }
    }

    for entry in &hit.register {
        line(
            &mut out,
            "register",
            &format!("{} · {} · {}", entry.provenance, entry.domain, standing(entry)),
        );
        match &entry.effect {
            Some(effect) => prose(&mut out, &format!("effect: {effect}")),
            // `effect` is null when every observation refutes it, which is a
            // finding rather than a gap.
            None => {
                if entry.status() == Status::Refuted {
                    continuation(&mut out, "effect: none — every observation refutes it");
                }
            }
        }
        if let Some(note) = &entry.note {
            prose(&mut out, &format!("note: {note}"));
        }
        let degraded = entry.degraded();
        if !degraded.is_empty() {
            // The witness rule, made visible. "Somebody claimed this and showed
            // nothing" and "nobody has tried it" are different states, and a
            // reader who cannot tell them apart cannot weigh either.
            let claimed: BTreeSet<String> = degraded
                .iter()
                .filter_map(|observation| observation.degraded_from)
                .map(|outcome| outcome.to_string())
                .collect();
            continuation(
                &mut out,
                &format!(
                    "{} claiming {} with no witness — recorded, not counted",
                    count(degraded.len(), "observation"),
                    claimed.into_iter().collect::<Vec<_>>().join("/")
                ),
            );
        }
    }

    if let Some(explained) = explanation(&hit.matched) {
        line(&mut out, "matched", &explained);
    }
    out
}

/// Why this hit is in the result, when that is not obvious.
///
/// An id match is self-evident: the query is visibly inside the id printed one
/// line up, so saying "matched id" is noise. Everything else is not obvious, and
/// a hit nobody can explain reads as noise even when it is the right answer —
/// `ItFo_Potion_Health_01` contains none of "Essenz heilender Kraft".
///
/// The languages of a name match are collapsed into one entry and capped:
/// "potion" is a word in six of the shipped languages, and six near-identical
/// lines would bury the one fact the line is for.
fn explanation(matched: &[Matched]) -> Option<String> {
    const LANGUAGES_SHOWN: usize = 3;

    let mut parts: Vec<String> = Vec::new();
    let mut languages: Vec<&str> = Vec::new();
    for entry in matched {
        match entry {
            Matched::Id => {}
            Matched::Name(language) => languages.push(language),
            other => parts.push(other.label()),
        }
    }
    if !languages.is_empty() {
        let hidden = languages.len().saturating_sub(LANGUAGES_SHOWN);
        languages.truncate(LANGUAGES_SHOWN);
        let more = if hidden > 0 {
            format!(", +{hidden} more")
        } else {
            String::new()
        };
        parts.push(format!("display name ({}{more})", languages.join(", ")));
    }
    (!parts.is_empty()).then(|| parts.join(", "))
}

fn line(out: &mut String, label: &str, value: &str) {
    out.push_str(&format!("  {label:<LABEL$} {value}\n"));
}

fn continuation(out: &mut String, value: &str) {
    out.push_str(&format!("  {:<LABEL$} {value}\n", ""));
}

/// How wide a wrapped `effect` or `note` is allowed to get, including the label
/// column. Below the 80 an unresized console still has, because these lines are
/// the ones worth reading rather than skimming.
const WRAP: usize = 78;

/// A continuation line, wrapped on word boundaries under the same indent.
///
/// The register's prose is written to be read: the seeded `note` on the health
/// potion is a full paragraph about how `m_Value` reaches a trader's price. A
/// terminal soft-wraps it into the left margin, which puts the second half of
/// the sentence where the *ids* are and makes a block of four facts look like
/// eight. Wrapping it here keeps the hit shaped like a hit.
fn prose(out: &mut String, value: &str) {
    let width = WRAP.saturating_sub(LABEL + 3).max(20);
    let mut current = String::new();
    for word in value.split_whitespace() {
        // A single word longer than the budget goes on its own line rather than
        // being cut: these are asset paths and identifiers, and half of one is
        // worse than a long line.
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > width {
            continuation(out, &current);
            current.clear();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        continuation(out, &current);
    }
}

/// An entry's standing in one phrase, with the corroboration behind it.
///
/// `disputed` names both camps and resolves neither: it is usually informative
/// rather than wrong (a patch, a language, a display scale), and picking a side
/// in the arithmetic would throw the finding away.
fn standing(entry: &Entry) -> String {
    let corroboration = entry.corroboration();
    match entry.status() {
        Status::Confirmed => format!(
            "confirmed by {}{}",
            count(corroboration.confirmed.observations, "observation"),
            builds(corroboration.confirmed.builds)
        ),
        Status::Refuted => format!(
            "refuted by {}{}",
            count(corroboration.refuted.observations, "observation"),
            builds(corroboration.refuted.builds)
        ),
        Status::Disputed => format!(
            "disputed — {} confirm, {} refute{}",
            corroboration.tally(Outcome::Confirmed).observations,
            corroboration.tally(Outcome::Refuted).observations,
            builds(corroboration.builds)
        ),
        Status::Unconfirmed => format!(
            "unconfirmed — {}, nobody has checked this in game",
            count(corroboration.observations(), "observation")
        ),
    }
}

fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("{n} {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// ` across 2 builds`, or nothing when no observation recorded a build.
fn builds(n: usize) -> String {
    if n == 0 {
        String::new()
    } else {
        format!(" across {}", count(n, "build"))
    }
}

fn json_hit(hit: &Hit<'_>) -> serde_json::Value {
    let register: Vec<serde_json::Value> = hit
        .register
        .iter()
        .map(|entry| {
            let mut value = serde_json::to_value(entry).unwrap_or(serde_json::Value::Null);
            if let Some(object) = value.as_object_mut() {
                // Derived, never stored — so it is computed here rather than
                // taken off the wire, exactly as `Entry::status` insists.
                object.insert("status".into(), serde_json::json!(entry.status().as_str()));
                object.insert(
                    "corroboration".into(),
                    serde_json::to_value(entry.corroboration()).unwrap_or(serde_json::Value::Null),
                );
            }
            value
        })
        .collect();
    serde_json::json!({
        "domain": hit.domain,
        "id": hit.id,
        "source": if hit.catalog.is_some() { "bundled catalog" } else { "effect register" },
        "name": hit.name.as_ref().map(|name| serde_json::json!({
            "text": name.text,
            "language": (!name.language.is_empty()).then(|| name.language.clone()),
            "from": match hit.name_source {
                NameSource::Localization => "shared loc catalog",
                NameSource::CatalogCaption => "bundled catalog caption",
                NameSource::None => "",
            },
        })),
        "category": hit.catalog.map(|entry| entry.category.clone()),
        "class": hit.catalog.and_then(|entry| entry.class.clone()),
        "module": hit.catalog.and_then(|entry| entry.module.clone()),
        "loc_key": hit.catalog.and_then(|entry| entry.loc_key.clone()),
        "matched": hit.matched.iter().map(Matched::label).collect::<Vec<_>>(),
        "register": register,
    })
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use gore_catalog::register::{Provenance, RegisterSource};

    fn terms(query: &str) -> Vec<String> {
        query.split_whitespace().map(str::to_lowercase).collect()
    }

    /// A register file with one entry, so a test can vary the one thing it is about.
    fn register_with(domain: &str, id: &str, observations: &str, effect: &str) -> Register {
        let json = format!(
            r#"{{"format": 1, "domain": "{domain}", "entries": [
                {{"id": "{id}", "effect": "{effect}",
                  "note": "the cheapest smoke test there is",
                  "observations": [{observations}]}}
            ]}}"#
        );
        let source = RegisterSource::parse(&json, Provenance::Bundled, "test fixture")
            .expect("the fixture is a valid register");
        let mut register = Register::default();
        register.push(source);
        register
    }

    fn observation(outcome: &str, witness: Option<&str>, build: &str) -> String {
        let witness = match witness {
            Some(text) => format!(r#""witness": "{text}", "#),
            None => String::new(),
        };
        format!(
            r#"{{"date": "2026-08-07", "build": "{build}", "gore": "90940340",
                 "outcome": "{outcome}", {witness}"by": "maintainer"}}"#
        )
    }

    fn apple() -> CatalogEntry {
        CatalogEntry {
            domain: "item",
            id: "ItFo_Apple".into(),
            category: "food".into(),
            class: Some("/Script/Angelscript.ItFo_Apple".into()),
            module: None,
            loc_key: None,
            caption: None,
        }
    }

    fn potion() -> CatalogEntry {
        CatalogEntry {
            domain: "item",
            id: "ItFo_Potion_Health_01".into(),
            category: "food".into(),
            class: Some("/Script/Angelscript.ItFo_Potion_Health_01".into()),
            module: None,
            loc_key: None,
            caption: None,
        }
    }

    fn names() -> NameIndex {
        NameIndex::from_pairs(&[
            ("itfo_apple", "english", "Apple"),
            ("itfo_apple", "german", "Apfel"),
            ("itfo_potion_health_01", "english", "Essence of Healing"),
            ("itfo_potion_health_01", "german", "Essenz heilender Kraft"),
        ])
    }

    /// The whole search, rendered as a user reads it.
    fn report(
        catalog: &[CatalogEntry],
        register: &Register,
        query: &str,
        state: &NameIndexState,
    ) -> String {
        let hits = search(catalog, register, &terms(query), state.index(), None);
        Report {
            query,
            domain: None,
            catalog,
            register,
            name_index: state,
            listed: hits.len(),
            hits: &hits,
        }
        .text()
    }

    #[test]
    fn an_item_whose_id_contains_none_of_the_query_is_still_found_by_its_display_name() {
        // The blind-user finding this command exists for: people arrive with
        // "the healing potion", and `ItFo_Potion_Health_01` does not contain the
        // word "Essenz", "healing" or "Kraft" anywhere a substring search over
        // ids could see.
        let catalog = vec![apple(), potion()];
        let register = Register::default();
        let index = names();
        let hits = search(&catalog, &register, &terms("essenz"), Some(&index), None);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "ItFo_Potion_Health_01");
        assert_eq!(
            hits[0].matched,
            vec![Matched::Name("german".into())],
            "and the hit says why it is here, since the id shows nothing"
        );
        // Answered in the language it was asked in, without a flag.
        assert_eq!(hits[0].name.as_ref().unwrap().text, "Essenz heilender Kraft");
        assert_eq!(hits[0].name.as_ref().unwrap().language, "german");
    }

    #[test]
    fn without_the_shared_loc_catalog_the_same_query_finds_nothing_and_the_report_says_why() {
        // The degradation the spec forbids papering over. What must never happen
        // is the first half of this test without the second: an empty result
        // that reads as "no such item".
        let catalog = vec![apple(), potion()];
        let register = Register::default();
        let hits = search(&catalog, &register, &terms("essenz"), None, None);
        assert!(hits.is_empty());

        let text = report(&catalog, &register, "essenz", &NameIndexState::Absent);
        assert!(text.contains("NOT searched"), "{text}");
        assert!(
            text.contains("gore loc extract"),
            "the notice must name the command that fixes it: {text}"
        );
        assert!(
            text.contains("nothing matches"),
            "and it must not read as an exhaustive answer: {text}"
        );
    }

    #[test]
    fn a_loc_catalog_that_exists_but_cannot_be_read_is_not_reported_as_an_absent_one() {
        // "Run `gore loc extract`" is the wrong advice for a catalog that is
        // already there and broken, and folding the two cases together would
        // hide a corrupt file behind advice that changes nothing.
        let catalog = vec![apple()];
        let register = Register::default();
        let state = NameIndexState::Unreadable {
            path: PathBuf::from("C:/gore/loc_catalog.json"),
            detail: "expected value at line 1 column 1".into(),
        };
        let text = report(&catalog, &register, "apfel", &state);
        assert!(text.contains("could not be read"), "{text}");
        assert!(text.contains("expected value at line 1"), "{text}");
        assert!(text.contains("loc_catalog.json"), "{text}");
    }

    #[test]
    fn the_name_index_notice_is_printed_whether_or_not_anything_matched() {
        let catalog = vec![apple()];
        let register = Register::default();
        for query in ["apfel", "nothing-like-this"] {
            let text = report(&catalog, &register, query, &NameIndexState::Absent);
            assert!(
                text.lines().any(|line| line.starts_with("display names:")),
                "{query}: {text}"
            );
        }
    }

    #[test]
    fn every_hit_says_which_layer_it_came_from() {
        // Provenance is the whole point of the two-layer split: a bundled fact
        // and a stranger's observation must never be printed identically.
        let catalog = vec![apple()];
        let register = register_with(
            "texture",
            "/Game/UI/Textures/Common/T_LogoRemake",
            &observation("confirmed", Some("nr 1 magenta"), "24340829"),
            "Main menu wordmark",
        );
        let state = NameIndexState::Ready(names());

        let from_register = report(&catalog, &register, "logoremake", &state);
        assert!(from_register.contains("effect register only"), "{from_register}");
        assert!(from_register.contains("bundled · texture ·"), "{from_register}");

        let from_catalog = report(&catalog, &register, "apfel", &state);
        assert!(
            from_catalog.contains("bundled catalog · item · food"),
            "{from_catalog}"
        );
    }

    #[test]
    fn a_register_entry_annotates_the_catalog_row_instead_of_replacing_it() {
        let catalog = vec![apple()];
        let register = register_with(
            "item",
            "ItFo_Apple",
            &observation("confirmed", Some("heilt 5 LP"), "24340829"),
            "eaten, it restores a little health",
        );
        let index = names();
        let hits = search(&catalog, &register, &terms("apple"), Some(&index), None);
        assert_eq!(hits.len(), 1, "one id, one hit, not one per layer");
        assert!(hits[0].catalog.is_some());
        assert_eq!(hits[0].register.len(), 1);

        let text = report(&catalog, &register, "apple", &NameIndexState::Ready(names()));
        assert!(text.contains("bundled catalog · item · food"), "{text}");
        assert!(text.contains("register  bundled · item · confirmed by 1 observation"), "{text}");
        assert!(text.contains("effect: eaten, it restores a little health"), "{text}");
    }

    #[test]
    fn an_id_the_register_has_never_seen_is_still_a_perfectly_good_hit() {
        // The overwhelming majority. An id absent from the register is one
        // nobody has looked at, not one that does nothing, so nothing in the
        // output may read as a warning.
        let catalog = vec![apple()];
        let register = Register::default();
        let hits = search(&catalog, &register, &terms("itfo_apple"), None, None);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].register.is_empty());
        let block = hit_block(&hits[0]);
        assert!(!block.contains("register"), "{block}");
        assert!(block.contains("bundled catalog · item · food"), "{block}");
    }

    #[test]
    fn a_disputed_entry_is_surfaced_as_disputed_rather_than_resolved() {
        // Both observations are real. Newest-wins, majority-wins or
        // bundled-wins would each throw away the finding.
        let register = register_with(
            "texture",
            "/Game/UI/T_LogoRemake",
            &format!(
                "{},{}",
                observation("confirmed", Some("magenta"), "24340829"),
                observation("refuted", Some("nichts passiert"), "24169431")
            ),
            "Main menu wordmark",
        );
        let text = report(&[], &register, "logoremake", &NameIndexState::Absent);
        assert!(text.contains("disputed — 1 confirm, 1 refute across 2 builds"), "{text}");
    }

    #[test]
    fn a_claim_with_no_witness_is_shown_as_recorded_rather_than_counted() {
        // The witness rule made visible. "Somebody said they saw this and showed
        // nothing" is a different state from "nobody has tried it", and a reader
        // who cannot tell them apart cannot weigh either.
        let register = register_with(
            "texture",
            "/Game/UI/T_LogoRemake",
            &observation("confirmed", None, "24340829"),
            "Main menu wordmark",
        );
        let text = report(&[], &register, "logoremake", &NameIndexState::Absent);
        assert!(text.contains("unconfirmed"), "{text}");
        assert!(text.contains("no witness"), "{text}");
        assert!(text.contains("recorded, not counted"), "{text}");
    }

    #[test]
    fn two_words_narrow_the_result_instead_of_widening_it() {
        // One needle matched whole would answer `healing potion` with nothing,
        // because no name contains the two words in that order — and answering
        // nothing to the sentence people actually type is the failure this
        // command was built for.
        let catalog = vec![apple(), potion()];
        let index = names();
        let register = Register::default();
        assert_eq!(
            search(&catalog, &register, &terms("essence healing"), Some(&index), None).len(),
            1
        );
        assert_eq!(
            search(&catalog, &register, &terms("healing essence"), Some(&index), None).len(),
            1,
            "order must not matter"
        );
        assert!(
            search(&catalog, &register, &terms("essence apple"), Some(&index), None).is_empty(),
            "every term has to match, or two words would widen the result"
        );
    }

    #[test]
    fn an_exact_id_is_listed_before_everything_it_is_a_prefix_of() {
        let catalog = vec![
            potion(),
            CatalogEntry { id: "ItFo_Potion_Health_01_Broken".into(), ..potion() },
        ];
        let register = Register::default();
        let hits = search(&catalog, &register, &terms("itfo_potion_health_01"), None, None);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id, "ItFo_Potion_Health_01");
        assert!(hits[0].exact);
    }

    #[test]
    fn an_id_match_outranks_a_word_that_only_turned_up_in_a_line_of_dialogue() {
        // `gore find logo` really did list five Portuguese dialogue lines above
        // `/Game/UI/Textures/Common/T_Logo`, because `knowledge` sorts before
        // `texture` and every hit was equal until then. Searching all eighteen
        // shipped languages is what makes a display-name search work at all;
        // ranking is what keeps it from burying the answer.
        let dialogue = CatalogEntry {
            domain: "knowledge",
            id: "ChoiceSaturasExit".into(),
            category: "choice".into(),
            class: None,
            module: None,
            loc_key: Some("TEXT_DDIAZ_20231219_132411".into()),
            caption: None,
        };
        let catalog = vec![dialogue];
        let register = register_with(
            "texture",
            "/Game/UI/Textures/Common/T_Logo",
            &observation("refuted", Some("nicht vorhanden"), "24340829"),
            "nothing observed",
        );
        let index = NameIndex::from_pairs(&[("text_ddiaz_20231219_132411", "brazilian", "Até logo.")]);

        let hits = search(&catalog, &register, &terms("logo"), Some(&index), None);
        assert_eq!(hits.len(), 2);
        assert_eq!(
            hits[0].id, "/Game/UI/Textures/Common/T_Logo",
            "the id somebody typed comes before the word that happened to appear in a subtitle"
        );
        assert_eq!(rank(&hits[0]), 1);
        assert_eq!(rank(&hits[1]), 4);
    }

    #[test]
    fn a_domain_nobody_carries_is_named_rather_than_answered_with_nothing() {
        // An unknown filter matching nothing reads exactly like an empty
        // namespace, and the only way forward from that is another guess.
        let register = Register::bundled().expect("the bundled registers are valid");
        let error = known_domain("textures", &register).unwrap_err().to_string();
        assert!(error.contains("no domain 'textures'"), "{error}");
        assert!(error.contains("texture"), "the real spellings must be listed: {error}");
        assert!(error.contains("knowledge"), "including the catalogs': {error}");
        assert!(known_domain("ITEM", &register).is_ok(), "domains fold case");
    }

    #[test]
    fn a_domain_filter_keeps_the_search_inside_one_namespace() {
        let catalog = vec![apple(), CatalogEntry { domain: "npc", ..apple() }];
        let register = Register::default();
        assert_eq!(search(&catalog, &register, &terms("apple"), None, None).len(), 2);
        assert_eq!(
            search(&catalog, &register, &terms("apple"), None, Some("npc")).len(),
            1
        );
    }

    #[test]
    fn a_truncated_result_says_how_much_is_missing_and_how_to_see_it() {
        // A listing that stopped silently lets a reader take the first page for
        // the whole answer and conclude a thing does not exist.
        let catalog: Vec<CatalogEntry> = (0..5)
            .map(|n| CatalogEntry { id: format!("ItFo_Apple_{n}"), ..apple() })
            .collect();
        let register = Register::default();
        let hits = search(&catalog, &register, &terms("apple"), None, None);
        let clipped = Report {
            query: "apple",
            domain: None,
            catalog: &catalog,
            register: &register,
            name_index: &NameIndexState::Absent,
            listed: 2,
            hits: &hits,
        };
        let text = clipped.text();
        assert!(text.contains("truncated: 5 hits matched and only the first 2"), "{text}");
        assert!(text.contains("--max"), "{text}");
        assert!(
            text.contains("add --domain"),
            "a query with no domain filter is told about one: {text}"
        );

        // …and one that already has a domain is not told to add the flag it used.
        let narrowed = Report { domain: Some("item"), ..clipped };
        assert!(!narrowed.text().contains("add --domain"), "{}", narrowed.text());
    }

    #[test]
    fn a_knowledge_entry_is_named_by_its_loc_key_and_falls_back_to_its_bundled_caption() {
        // Knowledge ids are generated (`Choice62749`) and appear nowhere in the
        // localization: looking their names up by id finds 0 of 3,913. The
        // `loc_key` is the only route, and the caption is the only name the
        // 740 rows that carry one have without a loc catalog at all.
        let keyed = CatalogEntry {
            domain: "knowledge",
            id: "Choice62749".into(),
            category: "choice".into(),
            class: None,
            module: Some("Story.G1R.Conversation.Conversation_UNITTEST".into()),
            loc_key: Some("TEXT_ANDRE_20220118_145939".into()),
            caption: None,
        };
        let captioned = CatalogEntry {
            id: "ChoiceAsghan144609".into(),
            loc_key: None,
            caption: Some("[Forced Conversation]".into()),
            ..keyed.clone()
        };
        let catalog = vec![keyed, captioned];
        let index = NameIndex::from_pairs(&[("text_andre_20220118_145939", "german", "Wer bist du?")]);
        let register = Register::default();

        let hits = search(&catalog, &register, &terms("wer bist"), Some(&index), None);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name.as_ref().unwrap().text, "Wer bist du?");
        assert_eq!(hits[0].name_source, NameSource::Localization);

        let hits = search(&catalog, &register, &terms("asghan"), Some(&index), None);
        assert_eq!(hits[0].name.as_ref().unwrap().text, "[Forced Conversation]");
        assert_eq!(hits[0].name_source, NameSource::CatalogCaption);
    }

    #[test]
    fn the_json_document_carries_the_same_name_index_notice_as_the_text() {
        // A client that renders only the JSON must not be able to show a search
        // that looks exhaustive when the name index was missing.
        let catalog = vec![apple()];
        let register = Register::default();
        let hits = search(&catalog, &register, &terms("apple"), None, None);
        let json = Report {
            query: "apple",
            domain: None,
            catalog: &catalog,
            register: &register,
            name_index: &NameIndexState::Absent,
            listed: hits.len(),
            hits: &hits,
        }
        .json()
        .expect("serializes");
        let document: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(document["name_index"]["searched"], serde_json::json!(false));
        assert!(document["name_index"]["notice"]
            .as_str()
            .expect("a notice")
            .contains("gore loc extract"));
        assert_eq!(document["hits"][0]["source"], serde_json::json!("bundled catalog"));
        assert_eq!(document["hits"][0]["id"], serde_json::json!("ItFo_Apple"));
    }

    #[test]
    fn the_coverage_count_counts_rows_and_not_deduplicated_ids() {
        // Three catalog rows, all named, and the name they share is one id. The count belongs to
        // the sentence "of N catalog entries", so it has to say 3 — counting the deduplicated set
        // said 1 for the rows and then added a register id that is no row at all.
        let shared = |n: u8| CatalogEntry {
            domain: "knowledge",
            id: format!("Info_Diego_{n}"),
            category: "dialog".into(),
            class: None,
            module: None,
            loc_key: Some("text_dialog_end".into()),
            caption: None,
        };
        let catalog = vec![shared(1), shared(2), shared(3)];
        let register = register_with(
            "loc",
            "ui_main_newgame",
            &observation("confirmed", Some("NEUES SPIEL"), "24539464"),
            "Main menu entry",
        );
        let index = NameIndex::from_pairs(&[
            ("text_dialog_end", "german", "Bis dann."),
            // In the index and in the register, but not a catalog row: it must not inflate the
            // numerator of a fraction whose denominator counts rows.
            ("ui_main_newgame", "german", "NEUES SPIEL"),
        ]);

        let text = report(
            &catalog,
            &register,
            "nothing-matches-this",
            &NameIndexState::Ready(index),
        );
        assert!(text.contains("3 of 3 catalog entries have one"), "{text}");
    }

    #[test]
    fn scoping_the_catalog_keeps_the_rows_of_one_domain_and_all_of_them_without_one() {
        let dialogue = CatalogEntry {
            domain: "knowledge",
            id: "ChoiceSaturasExit".into(),
            category: "choice".into(),
            class: None,
            module: None,
            loc_key: Some("TEXT_DDIAZ_20231219_132411".into()),
            caption: None,
        };
        let catalog = vec![apple(), potion(), dialogue];
        assert_eq!(catalog_in_domain(catalog.clone(), None).len(), 3);

        let items = catalog_in_domain(catalog.clone(), Some("item"));
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|entry| entry.domain == "item"), "{items:?}");

        // Case-insensitively, like every other domain comparison in this command.
        assert_eq!(catalog_in_domain(catalog, Some("KNOWLEDGE")).len(), 1);
    }

    #[test]
    fn a_domain_filter_scopes_the_totals_and_not_just_the_hits() {
        // The totals are what tells a reader an empty result was exhaustive. Reporting the whole
        // catalog and the whole register beside a search that skipped most of both says the
        // opposite of what happened.
        let catalog = vec![apple(), potion()];
        let mut register = register_with(
            "texture",
            "/Game/UI/T_LogoRemake",
            &observation("confirmed", Some("magenta"), "24539464"),
            "Main menu wordmark",
        );
        let item_source = format!(
            r#"{{"format": 1, "domain": "item", "entries": [
                 {{"id": "ItFo_Apple", "effect": "an apple",
                   "note": "second domain, so the filter has something to leave out",
                   "observations": [{}]}}
               ]}}"#,
            observation("confirmed", Some("Apfel"), "24539464")
        );
        register.push(
            RegisterSource::parse(&item_source, Provenance::Bundled, "test fixture")
                .expect("the fixture is a valid register"),
        );

        // Rows are dropped where `run` drops them, so the totals below count what was searched.
        let scoped = catalog_in_domain(catalog, Some("texture"));
        assert!(scoped.is_empty(), "the fixture catalog carries no texture rows");

        let hits = search(&scoped, &register, &terms("logoremake"), None, Some("texture"));
        let report = Report {
            query: "logoremake",
            domain: Some("texture"),
            catalog: &scoped,
            register: &register,
            name_index: &NameIndexState::Absent,
            listed: hits.len(),
            hits: &hits,
        };

        let json = report.json().expect("serializes");
        let document: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(document["catalog_entries"], serde_json::json!(0));
        assert_eq!(
            document["register_entries"],
            serde_json::json!(1),
            "only the texture entry was searchable: {json}"
        );

        // And the same numbers in the text, which is the half a person actually reads.
        let empty = Report { query: "nothing-matches-this", hits: &[], listed: 0, ..report };
        let text = empty.text();
        assert!(
            text.contains("searched 0 bundled catalog entries and 1 effect-register entries"),
            "{text}"
        );
    }

    #[test]
    fn a_catalog_with_anything_after_the_object_is_not_a_catalog() {
        let wanted: HashSet<String> = ["itfo_apple".to_string()].into_iter().collect();
        let good = r#"{"itfo_apple": {"german": "Apfel"}}"#;
        assert!(parse_name_index(good, &wanted).is_ok());

        // One valid object and then anything at all: a truncated rewrite, two files concatenated,
        // a stray brace. The seed stops at the end of the first value and would have reported a
        // catalog it only half read.
        for tail in ["{\"itfo_apple\": {}}", "garbage", "]"] {
            let text = format!("{good}\n{tail}");
            assert!(
                parse_name_index(&text, &wanted).is_err(),
                "trailing {tail:?} must not pass as a catalog"
            );
        }
    }

    #[test]
    fn a_query_split_between_register_text_and_a_display_name_keeps_both_reasons() {
        // `matching_register` admits an entry when every term is answered by its register text OR
        // by a display name, so the two can split a multi-word query. Reporting only the name lost
        // half the explanation and dropped the entry to the weakest tier, where `--max` cuts first.
        let register = register_with(
            "texture",
            "/Game/UI/Textures/Common/T_Wordmark",
            &observation("confirmed", Some("magenta"), "24539464"),
            "turns magenta in the main menu",
        );
        let index = NameIndex::from_pairs(&[(
            "/Game/UI/Textures/Common/T_Wordmark",
            "german",
            "Logo Remake",
        )]);

        // "logo" is in the display name only; "magenta" is in the register text only.
        let hits = search(&[], &register, &terms("logo magenta"), Some(&index), None);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(
            hits[0].matched.contains(&Matched::Register),
            "the register answered for 'magenta': {:?}",
            hits[0].matched
        );
        assert!(
            hits[0]
                .matched
                .iter()
                .any(|matched| matches!(matched, Matched::Name(_))),
            "the display name answered for 'logo': {:?}",
            hits[0].matched
        );
        // Tier 2 is the register's, and it is what keeps an observed entry ahead of lexical
        // accidents when the list is truncated.
        assert_eq!(rank(&hits[0]), 2);
    }

    #[test]
    fn a_catalog_that_is_missing_and_one_that_cannot_be_read_are_different_answers() {
        // `catalog_present()` is `Path::is_file()`, false for both — so a catalog behind an
        // unreadable directory was reported as never extracted, and the advice that follows from
        // that is `gore loc extract`, which fails for the same unmentioned reason.
        let dir = tempfile::tempdir().unwrap();
        let wanted: HashSet<String> = ["itfo_apple".to_string()].into_iter().collect();

        let absent = load_name_index_at(dir.path().join("nothing-here.json"), &wanted);
        assert!(matches!(absent, NameIndexState::Absent), "{absent:?}");

        // A directory where a file is expected: reading it fails with something other than
        // NotFound on every platform, without needing permissions the suite may not have.
        let unreadable = load_name_index_at(dir.path().to_path_buf(), &wanted);
        assert!(
            matches!(unreadable, NameIndexState::Unreadable { .. }),
            "{unreadable:?}"
        );
    }

    #[test]
    fn an_id_that_answered_for_part_of_the_query_still_ranks_as_an_id_match() {
        // `logo main`: the id carries "logo", the effect carries "main". Calling that register
        // text alone dropped the entry from tier 1 to tier 2, which is where `--max` starts
        // cutting — and an id match is the strongest reason this command has.
        let register = register_with(
            "texture",
            "/Game/UI/Textures/Common/T_Logo",
            &observation("confirmed", Some("magenta"), "24539464"),
            "the wordmark on the main menu",
        );

        let hits = search(&[], &register, &terms("logo main"), None, None);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(
            hits[0].matched.contains(&Matched::Id),
            "the id answered for 'logo': {:?}",
            hits[0].matched
        );
        assert!(
            hits[0].matched.contains(&Matched::Register),
            "the effect answered for 'main': {:?}",
            hits[0].matched
        );
        assert_eq!(rank(&hits[0]), 1);

        // And an entry the id answers for entirely is still an id match and nothing else: the
        // register searches ids too, so this is the case that would wrongly gain "register text".
        let only_id = search(&[], &register, &terms("t_logo"), None, None);
        assert_eq!(only_id[0].matched, vec![Matched::Id], "{:?}", only_id[0].matched);
    }

    #[test]
    fn a_word_with_punctuation_stuck_to_it_is_still_that_word() {
        // Every term must match, so one stray character removed the result entirely: `gore find
        // healing potion?` searched for `potion?`, which is in no id and no name anywhere.
        assert_eq!(query_terms("healing potion?"), vec!["healing", "potion"]);
        assert_eq!(query_terms("\"Apfel\","), vec!["apfel"]);

        // Only at the edges. Ids are mostly punctuation, and an underscore is part of the word.
        assert_eq!(query_terms("/Game/UI/T_Logo"), vec!["game/ui/t_logo"]);
        assert_eq!(query_terms("ItFo_Apple"), vec!["itfo_apple"]);

        // A term that was nothing but punctuation is not a term.
        assert!(query_terms("?? --").is_empty());
    }

    #[test]
    fn two_register_ids_differing_only_by_case_are_two_hits() {
        // `Register::lookup` states the contract: exact spellings win, and two ids that differ only
        // by case are two ids. Keying the result by a lowercased id broke it — the second entry
        // reached the merge branch, contributed its match reasons, and had its own id and every
        // observation on it dropped from the report.
        let mut register = register_with(
            "texture",
            "/Game/UI/T_Logo",
            &observation("confirmed", Some("magenta"), "24539464"),
            "the wordmark",
        );
        let second = format!(
            r#"{{"format": 1, "domain": "texture", "entries": [
                 {{"id": "/game/ui/t_logo", "effect": "a different asset entirely",
                   "note": "same letters, different id",
                   "observations": [{}]}}
               ]}}"#,
            observation("refuted", Some("nicht vorhanden"), "24539464")
        );
        register.push(
            RegisterSource::parse(&second, Provenance::Bundled, "test fixture")
                .expect("the fixture is a valid register"),
        );

        let hits = search(&[], &register, &terms("t_logo"), None, None);
        assert_eq!(hits.len(), 2, "{hits:?}");

        let ids: Vec<&str> = hits.iter().map(|hit| hit.id.as_str()).collect();
        assert!(ids.contains(&"/Game/UI/T_Logo"), "{ids:?}");
        assert!(ids.contains(&"/game/ui/t_logo"), "{ids:?}");

        // And each keeps its own observations rather than borrowing the other's.
        for hit in &hits {
            assert_eq!(hit.register.len(), 1, "{:?}", hit.register);
        }
    }

    #[test]
    fn the_json_register_block_carries_the_derived_status_and_its_corroboration() {
        let register = register_with(
            "texture",
            "/Game/UI/T_LogoRemake",
            &observation("confirmed", Some("magenta"), "24340829"),
            "Main menu wordmark",
        );
        let hits = search(&[], &register, &terms("logoremake"), None, None);
        let json = Report {
            query: "logoremake",
            domain: None,
            catalog: &[],
            register: &register,
            name_index: &NameIndexState::Absent,
            listed: hits.len(),
            hits: &hits,
        }
        .json()
        .expect("serializes");
        let document: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        let entry = &document["hits"][0]["register"][0];
        assert_eq!(entry["provenance"], serde_json::json!("bundled"));
        assert_eq!(entry["status"], serde_json::json!("confirmed"));
        assert_eq!(entry["corroboration"]["confirmed"]["observations"], serde_json::json!(1));
        assert_eq!(document["hits"][0]["source"], serde_json::json!("effect register"));
    }

    #[test]
    fn the_bundled_catalogs_parse_and_hold_what_this_command_claims_to_search() {
        // The counts are a claim about the shipped assets, and a regenerated
        // catalog that lost a schema field would otherwise only surface as
        // silently missing hits.
        let catalog = bundled_catalog().expect("the bundled catalogs are valid");
        assert_eq!(catalog.iter().filter(|entry| entry.domain == "item").count(), 831);
        assert_eq!(catalog.iter().filter(|entry| entry.domain == "npc").count(), 1095);
        assert_eq!(catalog.iter().filter(|entry| entry.domain == "knowledge").count(), 3913);

        let apple = catalog
            .iter()
            .find(|entry| entry.id == "ItFo_Apple")
            .expect("a class every cook has");
        assert_eq!(apple.category, "food");
        assert_eq!(apple.class.as_deref(), Some("/Script/Angelscript.ItFo_Apple"));
        assert_eq!(apple.loc_id(), "ItFo_Apple");

        // Knowledge is the schema most likely to lose a field, because two of
        // its four are optional.
        assert!(catalog
            .iter()
            .any(|entry| entry.domain == "knowledge" && entry.loc_key.is_some()));
        assert!(catalog
            .iter()
            .any(|entry| entry.domain == "knowledge" && entry.caption.is_some()));
    }

    #[test]
    fn a_search_over_the_real_catalogs_finds_the_class_a_person_would_be_looking_for() {
        let catalog = bundled_catalog().expect("the bundled catalogs are valid");
        let register = Register::bundled().expect("the bundled registers are valid");
        let hits = search(&catalog, &register, &terms("itfo_potion_health"), None, None);
        assert!(
            hits.iter().any(|hit| hit.id == "ItFo_Potion_Health_01"),
            "an id substring must find the family"
        );
        // A category is one of the three things the spec says is always
        // searched, and it is the only handle somebody has with no loc catalog.
        let runes = search(&catalog, &register, &terms("rune"), None, Some("item"));
        assert!(runes.len() > 20, "found {} rune-ish item classes", runes.len());
        assert!(runes.iter().all(|hit| hit.domain == "item"));
    }

    #[test]
    fn the_wanted_set_covers_every_id_a_name_could_be_needed_for() {
        // This set is what makes opening a 28 MB catalog cheap; an id missing
        // from it is an id that silently loses its name.
        let catalog = bundled_catalog().expect("the bundled catalogs are valid");
        let register = Register::bundled().expect("the bundled registers are valid");
        let wanted = wanted_loc_ids(&catalog, &register);
        for entry in &catalog {
            assert!(
                wanted.contains(&entry.loc_id().to_lowercase()),
                "{} would never be named",
                entry.id
            );
        }
        for entry in register.entries() {
            assert!(wanted.contains(&entry.id.to_lowercase()), "{}", entry.id);
        }
    }
}
