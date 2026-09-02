//! `gore dialog` — read the game's dialog trees.
//!
//! Everything here works offline and only ever reads the installation. The tree comes out of the
//! shipping script cache; the text comes out of the shared localization catalog that `gore loc
//! extract` writes. Nothing here launches the game, writes into the install, touches a save, or
//! deploys; the commands that produce something write only where they are pointed.
//!
//! `checkout`/`check`/`stage` prepare an edit to a shipped conversation module and say offline
//! whether the current default-regeneration and remap contract can represent it. They stop at the
//! compiler's door: producing the mini-cache is `gore as compile-module`, and shipping it is
//! `gore mod`.
//!
//! What the cache declares is not the same as what a player sees: a topic's rules and its
//! `IsVisible` override decide that per save state, and this command deliberately reports both
//! rather than pretending to evaluate them.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use gore_as::cache::dialog::{
    self, Arg, Caption, CheckSource, Conversation, DialogGraph, Rule, RuleKind, Step, StepKind,
    Topic, Visibility,
};

use super::find::{load_name_index, Name, NameIndexState};

/// Localization columns to read a line from, newest first, per language family.
const GERMAN_COLUMNS: &[&str] = &["german_new", "german"];
const ENGLISH_COLUMNS: &[&str] = &["english_newer", "english_new", "english"];

#[derive(Subcommand)]
pub enum DialogAction {
    /// List the conversations the game ships, with their participants and size
    List {
        /// Keep only conversations whose participant or module contains this text
        filter: Option<String>,
        /// Exact script cache to read. Defaults to the one in the resolved game install
        #[arg(long)]
        cache: Option<PathBuf>,
        /// Game install root (the folder containing `G1R/`)
        #[arg(long)]
        game: Option<PathBuf>,
        /// Emit one JSON document instead of the human-readable listing
        #[arg(long)]
        json: bool,
    },
    /// Print one NPC's complete dialog tree
    Tree {
        /// Participant identifier (`om_stt_viper_302`), part of one, or a module name
        npc: String,
        /// Localization column, or a language family (`german`, `english`)
        #[arg(long, default_value = "english")]
        lang: String,
        /// Stop after this much sub-dialog nesting
        #[arg(long)]
        depth: Option<usize>,
        /// Print class names and localization keys next to the text
        #[arg(long)]
        ids: bool,
        /// Emit one JSON document instead of the human-readable tree
        #[arg(long)]
        json: bool,
        #[arg(long)]
        cache: Option<PathBuf>,
        #[arg(long)]
        game: Option<PathBuf>,
    },
    /// Print one topic in full: caption, rules, visibility, and body
    Show {
        /// Topic class name, with or without the generated `U` prefix
        topic: String,
        #[arg(long, default_value = "english")]
        lang: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        cache: Option<PathBuf>,
        #[arg(long)]
        game: Option<PathBuf>,
    },
    /// Write one NPC's dialog text as a `gore loc import` edits document
    Text {
        /// Participant identifier (`om_stt_viper_302`), part of one, or a module name
        npc: String,
        /// Localization column, or a language family (`german`, `english`)
        #[arg(long, default_value = "german")]
        lang: String,
        /// Output edits JSON, ready for `gore loc import --edits`
        #[arg(short = 'o', long)]
        out: PathBuf,
        #[arg(long)]
        cache: Option<PathBuf>,
        #[arg(long)]
        game: Option<PathBuf>,
    },
    /// Scaffold a new dialog option inside one NPC's checked-out conversation module
    NewTopic {
        /// Participant identifier (`om_stt_viper_302`), part of one, or a module name
        npc: String,
        /// The menu text, as an untranslated literal
        #[arg(
            long,
            conflicts_with = "caption_key",
            required_unless_present = "caption_key"
        )]
        caption: Option<String>,
        /// The menu text's localization key, for a translatable option
        #[arg(long)]
        caption_key: Option<String>,
        /// AngelScript class name for the new option
        #[arg(long)]
        class: Option<String>,
        /// Existing parent topic whose one Subdialog call should receive the new option
        #[arg(long, value_name = "TOPIC")]
        subdialog_of: Option<String>,
        /// 1-based position among populated entries; default keeps a trailing TEXT_BACK last
        #[arg(
            long,
            value_name = "N",
            requires = "subdialog_of",
            value_parser = clap::value_parser!(usize)
        )]
        subdialog_position: Option<usize>,
        /// Menu-order rank. Roots choose an automatic rank before a recognized final End/Back entry, otherwise before the trailing rank group; sub-topics default to 0 and keep equal-rank order from --subdialog-position. Pass a value to override it; -1 has the game's forced-topic semantics and is never chosen automatically
        #[arg(long, value_name = "N", allow_negative_numbers = true)]
        priority_rank: Option<i32>,
        /// Mod name, used for the default class name and the staged bundle
        #[arg(long, default_value = "MyDialogMod")]
        mod_name: String,
        /// Output directory for source, pristine copy, and edit manifest
        #[arg(short = 'o', long)]
        out: PathBuf,
        #[arg(long)]
        cache: Option<PathBuf>,
        #[arg(long)]
        game: Option<PathBuf>,
    },
    /// Scaffold the first option and private topic base for an NPC conversation
    NewConversation {
        /// Exact NPC identifier with one loaded per-NPC conversation-settings module; a separate unbound add module is refused
        npc: String,
        /// The first menu option's text, as an untranslated literal
        #[arg(
            long,
            conflicts_with = "caption_key",
            required_unless_present = "caption_key"
        )]
        caption: Option<String>,
        /// The first menu option's localization key, for a translatable option
        #[arg(long)]
        caption_key: Option<String>,
        /// AngelScript class name for the first option
        #[arg(long)]
        class: Option<String>,
        /// Menu-order rank for the first option. Pass -1 only when intentionally authoring a forced topic
        #[arg(
            long,
            value_name = "N",
            default_value_t = 2,
            allow_negative_numbers = true
        )]
        priority_rank: i32,
        /// Mod name, used for the default class name and staged bundle
        #[arg(long, default_value = "MyDialogMod")]
        mod_name: String,
        /// Output directory for source, pristine copy, and edit manifest
        #[arg(short = 'o', long)]
        out: PathBuf,
        #[arg(long)]
        cache: Option<PathBuf>,
        #[arg(long)]
        game: Option<PathBuf>,
    },
    /// Take one conversation's AngelScript, including class defaults, out for editing
    Checkout {
        /// Participant identifier (`om_stt_viper_302`), part of one, or a module name
        npc: String,
        /// Empty working directory for the source, its pristine copy, and the manifest
        #[arg(short = 'o', long)]
        out: PathBuf,
        #[arg(long)]
        cache: Option<PathBuf>,
        #[arg(long)]
        game: Option<PathBuf>,
    },
    /// Check an edited conversation against the current compile/remap contract
    Check {
        /// The directory `checkout` wrote
        dir: PathBuf,
        /// Emit one JSON document instead of the human-readable verdict
        #[arg(long)]
        json: bool,
        #[arg(long)]
        cache: Option<PathBuf>,
        #[arg(long)]
        game: Option<PathBuf>,
    },
    /// Write the build spec for a checked edit and print the commands that compile it
    Stage {
        /// The directory `checkout` wrote
        dir: PathBuf,
        /// Portable single-component mod name for the bundle this edit ships in
        #[arg(long, default_value = "MyDialogEdit")]
        mod_name: String,
        #[arg(long)]
        cache: Option<PathBuf>,
        #[arg(long)]
        game: Option<PathBuf>,
    },
    /// Write every conversation to a directory, one JSON file each
    Export {
        /// Output directory. Created if absent; existing files are overwritten
        #[arg(short = 'o', long)]
        out: PathBuf,
        #[arg(long)]
        cache: Option<PathBuf>,
        #[arg(long)]
        game: Option<PathBuf>,
    },
}

pub fn run(action: DialogAction) -> Result<()> {
    match action {
        DialogAction::List {
            filter,
            cache,
            game,
            json,
        } => list(filter.as_deref(), cache, game, json),
        DialogAction::Tree {
            npc,
            lang,
            depth,
            ids,
            json,
            cache,
            game,
        } => tree(&npc, &lang, depth, ids, json, cache, game),
        DialogAction::Show {
            topic,
            lang,
            json,
            cache,
            game,
        } => show(&topic, &lang, json, cache, game),
        DialogAction::Text {
            npc,
            lang,
            out,
            cache,
            game,
        } => text_edits(&npc, &lang, &out, cache, game),
        DialogAction::NewTopic {
            npc,
            caption,
            caption_key,
            class,
            subdialog_of,
            subdialog_position,
            priority_rank,
            mod_name,
            out,
            cache,
            game,
        } => new_topic(NewTopicRequest {
            npc,
            caption,
            caption_key,
            class,
            subdialog_of,
            subdialog_position,
            priority_rank,
            mod_name,
            out,
            cache,
            game,
        }),
        DialogAction::NewConversation {
            npc,
            caption,
            caption_key,
            class,
            priority_rank,
            mod_name,
            out,
            cache,
            game,
        } => new_conversation(NewConversationRequest {
            npc,
            caption,
            caption_key,
            class,
            priority_rank,
            mod_name,
            out,
            cache,
            game,
        }),
        DialogAction::Checkout {
            npc,
            out,
            cache,
            game,
        } => checkout(&npc, &out, cache, game),
        DialogAction::Check {
            dir,
            json,
            cache,
            game,
        } => check(&dir, json, cache, game),
        DialogAction::Stage {
            dir,
            mod_name,
            cache,
            game,
        } => stage(&dir, &mod_name, cache, game),
        DialogAction::Export { out, cache, game } => export(&out, cache, game),
    }
}

// ─── Reading the cache ───────────────────────────────────────────────────────

/// The script cache to read: the one named, else the one in the resolved install.
fn cache_path(cache: Option<PathBuf>, game: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(cache) = cache {
        return Ok(cache);
    }
    let root = gore_loc::config::game_root(game).context("resolving the game path")?;
    let paths = gore_mod::resolve_game_paths(&root);
    if !paths.script_cache.is_file() {
        bail!(
            "no script cache at {}. Pass --cache to read one directly",
            paths.script_cache.display()
        );
    }
    Ok(paths.script_cache)
}

fn read_cache(cache: Option<PathBuf>, game: Option<PathBuf>) -> Result<(PathBuf, Vec<u8>)> {
    let path = cache_path(cache, game)?;
    let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    Ok((path, bytes))
}

fn read_graph(cache: Option<PathBuf>, game: Option<PathBuf>) -> Result<DialogGraph> {
    let (path, bytes) = read_cache(cache, game)?;
    dialog::build(&bytes).with_context(|| format!("reading dialog from {}", path.display()))
}

/// Every conversation whose participant or module contains `needle`, case-insensitively.
fn containing<'a>(graph: &'a DialogGraph, needle: &str) -> Vec<&'a Conversation> {
    let needle = needle.to_lowercase();
    graph
        .conversations
        .iter()
        .filter(|conversation| {
            conversation
                .participants
                .iter()
                .any(|participant| participant.to_lowercase().contains(&needle))
                || conversation.module.to_lowercase().contains(&needle)
        })
        .collect()
}

/// Prefer exact participant/module matches when resolving one conversation, then fall back to
/// the broad substring lookup used by read commands.
fn matching<'a>(graph: &'a DialogGraph, needle: &str) -> Vec<&'a Conversation> {
    let folded = needle.to_lowercase();
    let exact: Vec<&Conversation> = graph
        .conversations
        .iter()
        .filter(|conversation| {
            conversation
                .npc_participants()
                .any(|participant| participant.to_lowercase() == folded)
                || conversation.module.to_lowercase() == folded
        })
        .collect();
    if !exact.is_empty() {
        return exact;
    }
    containing(graph, needle)
}

/// Exact participant lookup for commands that may scaffold into a loaded settings anchor.
///
/// Reusing [`matching`] here would be unsafe: its documented partial/module fallback is useful for
/// read commands, but a unique substring could silently turn a requested new NPC into an edit of a
/// different shipped conversation.
fn exact_npc_matches<'a>(graph: &'a DialogGraph, npc: &str) -> Vec<&'a Conversation> {
    graph
        .conversations
        .iter()
        .filter(|conversation| {
            conversation
                .npc_participants()
                .any(|participant| participant.eq_ignore_ascii_case(npc))
        })
        .collect()
}

// ─── Localization ────────────────────────────────────────────────────────────

/// Resolved text for the localization keys one tree needs.
struct Text {
    lines: BTreeMap<String, String>,
    /// Why no text is available, when that is the case.
    note: Option<String>,
}

impl Text {
    fn get(&self, key: &str) -> Option<&str> {
        self.lines.get(&key.to_lowercase()).map(String::as_str)
    }
}

fn columns_for(lang: &str) -> Vec<String> {
    match lang.to_lowercase().as_str() {
        "german" | "de" | "deutsch" => GERMAN_COLUMNS.iter().map(|c| (*c).to_owned()).collect(),
        "english" | "en" => ENGLISH_COLUMNS.iter().map(|c| (*c).to_owned()).collect(),
        other => vec![other.to_owned()],
    }
}

fn name_in_columns<'a>(names: &'a [Name], columns: &[String]) -> Option<&'a Name> {
    columns.iter().find_map(|column| {
        names
            .iter()
            .find(|name| name.language.eq_ignore_ascii_case(column))
    })
}

/// Load the text for `keys` in the first populated column of `lang`'s chain.
fn load_text(keys: &HashSet<String>, lang: &str) -> Text {
    let columns = columns_for(lang);
    match load_name_index(keys) {
        NameIndexState::Ready(index) => {
            let mut lines = BTreeMap::new();
            for key in keys {
                let Some(names) = index.names_for(key) else {
                    continue;
                };
                let chosen = name_in_columns(names, &columns);
                if let Some(name) = chosen {
                    lines.insert(key.to_lowercase(), name.text.clone());
                }
            }
            Text { lines, note: None }
        }
        NameIndexState::Absent => Text {
            lines: BTreeMap::new(),
            note: Some(
                "no localization catalog yet — run `gore loc extract` to see the spoken text"
                    .to_owned(),
            ),
        },
        NameIndexState::Unreadable { path, detail } => Text {
            lines: BTreeMap::new(),
            note: Some(format!(
                "localization catalog at {} could not be read: {detail}",
                path.display()
            )),
        },
        NameIndexState::Obstructed { path } => Text {
            lines: BTreeMap::new(),
            note: Some(format!(
                "{} is not a file, so no text could be read",
                path.display()
            )),
        },
    }
}

/// Every localization key one conversation refers to, folded the way the shared catalog's
/// loader wants them: it keeps only ids that match its lowercase `wanted` set.
fn rule_text_keys(rule: &Rule) -> impl Iterator<Item = &str> {
    let carries_localized_line = matches!(
        rule.kind,
        RuleKind::RequireCharacterHasListenedTo | RuleKind::RequireCharacterHasNotListenedTo
    );
    rule.args.iter().filter_map(move |arg| {
        if !carries_localized_line {
            return None;
        }
        match arg {
            Arg::Text { value } => Some(value.as_str()),
            _ => None,
        }
    })
}

fn keys_of(conversation: &Conversation) -> HashSet<String> {
    let mut keys = HashSet::new();
    for topic in &conversation.topics {
        if let Some(key) = topic.caption.loc_key() {
            keys.insert(key.to_lowercase());
        }
        for step in &topic.act {
            if let StepKind::Say {
                loc_key: Some(key), ..
            } = &step.kind
            {
                keys.insert(key.to_lowercase());
            }
        }
        for rule in &topic.rules {
            for key in rule_text_keys(rule) {
                keys.insert(key.to_lowercase());
            }
        }
    }
    keys
}

// ─── list ────────────────────────────────────────────────────────────────────

fn list(
    filter: Option<&str>,
    cache: Option<PathBuf>,
    game: Option<PathBuf>,
    json: bool,
) -> Result<()> {
    let graph = read_graph(cache, game)?;
    let selected: Vec<&Conversation> = match filter {
        Some(needle) => containing(&graph, needle),
        None => graph.conversations.iter().collect(),
    };

    if json {
        let rows: Vec<_> = selected
            .iter()
            .map(|conversation| {
                serde_json::json!({
                    "module": conversation.module,
                    "participants": conversation.participants,
                    "topics": conversation.topics.len(),
                    "roots": conversation.roots.len(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    if selected.is_empty() {
        println!("no conversation matched");
        return Ok(());
    }

    let width = selected
        .iter()
        .flat_map(|conversation| conversation.npc_participants())
        .map(str::len)
        .max()
        .unwrap_or(0)
        .min(46);
    for conversation in &selected {
        let who = participant_label(conversation);
        println!(
            "{who:<width$}  {:>3} topics  {:>3} root  {}",
            conversation.topics.len(),
            conversation.roots.len(),
            conversation.module,
        );
    }
    println!();
    println!("{} conversation(s)", selected.len());
    Ok(())
}

// ─── tree ────────────────────────────────────────────────────────────────────

/// The one conversation `npc` names, or a refusal that lists the candidates.
fn resolve_one<'a>(graph: &'a DialogGraph, npc: &str) -> Result<&'a Conversation> {
    let selected = matching(graph, npc);
    match selected.len() {
        0 => bail!("no conversation matched {npc:?}. Try `gore dialog list`"),
        1 => Ok(selected[0]),
        _ => {
            let names: Vec<String> = selected
                .iter()
                .map(|conversation| participant_label(conversation))
                .collect();
            bail!(
                "{npc:?} matched {} conversations: {}. Name one exactly",
                selected.len(),
                names.join(", ")
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn tree(
    npc: &str,
    lang: &str,
    depth: Option<usize>,
    ids: bool,
    json: bool,
    cache: Option<PathBuf>,
    game: Option<PathBuf>,
) -> Result<()> {
    let graph = read_graph(cache, game)?;
    let conversation = resolve_one(&graph, npc)?;

    if json {
        println!("{}", serde_json::to_string_pretty(conversation)?);
        return Ok(());
    }

    let text = load_text(&keys_of(conversation), lang);
    print_conversation(conversation, &text, depth, ids);
    Ok(())
}

fn print_conversation(conversation: &Conversation, text: &Text, depth: Option<usize>, ids: bool) {
    println!("{}", participant_label(conversation));
    println!("{}", conversation.module);
    if conversation.topics.is_empty() {
        println!("this conversation declares character settings and no topics");
        return;
    }
    println!(
        "{} topics, {} root option(s)",
        conversation.topics.len(),
        conversation.roots.len()
    );
    if let Some(note) = &text.note {
        println!("note: {note}");
    }
    println!();

    let mut printed = BTreeSet::new();
    for root in &conversation.roots {
        if let Some(topic) = conversation.topic(root) {
            print_topic(conversation, topic, text, 0, 0, depth, ids, &mut printed);
        }
    }

    let unreached: Vec<&Topic> = conversation
        .topics
        .iter()
        .filter(|topic| !printed.contains(&topic.class))
        .collect();
    if !unreached.is_empty() {
        println!();
        println!(
            "{} topic(s) not reached from a root option:",
            unreached.len()
        );
        for topic in unreached {
            println!("  {} {}", caption_of(topic, text), topic.class);
        }
    }

    let coverage = &conversation.coverage;
    println!();
    print!(
        "read {} step(s), {} of them typed",
        coverage.steps, coverage.steps_typed
    );
    if coverage.ambient_topics > 0 {
        print!("; {} ambient topic(s)", coverage.ambient_topics);
    }
    if coverage.topics_without_caption > 0 {
        print!(
            "; {} chooseable topic(s) without a caption",
            coverage.topics_without_caption
        );
    }
    if coverage.says_incomplete > 0 {
        print!(
            "; {} line(s) missing a speaker or key",
            coverage.says_incomplete
        );
    }
    if coverage.calls_unresolved > 0 {
        print!("; {} unresolved operation(s)", coverage.calls_unresolved);
    }
    if coverage.dangling_children > 0 {
        print!(
            "; {} sub-topic(s) declared elsewhere",
            coverage.dangling_children
        );
    }
    println!();
}

fn caption_of(topic: &Topic, text: &Text) -> String {
    match &topic.caption {
        Caption::LocKey { key } => match text.get(key) {
            Some(line) => format!("\"{line}\""),
            None => format!("<{key}>"),
        },
        Caption::Literal { text } => format!("\"{text}\""),
        // An ambient topic plays on its own, so having no menu entry is what it is, not a gap.
        Caption::Unresolved if topic.flags.is_ambient => "(ambient)".to_owned(),
        Caption::Unresolved => "<no caption>".to_owned(),
    }
}

#[allow(clippy::too_many_arguments)]
fn print_topic(
    conversation: &Conversation,
    topic: &Topic,
    text: &Text,
    level: usize,
    menu_depth: usize,
    depth: Option<usize>,
    ids: bool,
    printed: &mut BTreeSet<String>,
) {
    if !printed.insert(topic.class.clone()) {
        println!(
            "{}- {} (already shown)",
            pad(level),
            caption_of(topic, text)
        );
        return;
    }
    let indent = pad(level);
    let mut header = format!("{indent}- {}", caption_of(topic, text));
    if ids {
        header.push_str(&format!("   [{}", topic.class));
        if let Some(key) = topic.caption.loc_key() {
            header.push_str(&format!(" · {key}"));
        }
        header.push(']');
    }
    println!("{header}");

    let body = pad(level + 1);
    for line in condition_lines(topic, text) {
        println!("{body}{line}");
    }

    for step in &topic.act {
        print_step(
            conversation,
            step,
            text,
            level + 1,
            menu_depth,
            depth,
            ids,
            printed,
        );
    }
}

/// The rules and visibility override rendered as human sentences.
fn condition_lines(topic: &Topic, text: &Text) -> Vec<String> {
    let mut lines = Vec::new();
    let owner = class_without_object_prefix(&topic.class);
    for rule in &topic.rules {
        lines.push(format!("? {}", rule_sentence(rule, Some(owner), text)));
    }
    if let Visibility::Scripted { checks } = &topic.visibility {
        let shown: Vec<String> = checks
            .iter()
            .map(|check| match check.source {
                CheckSource::Field => check.name.clone(),
                CheckSource::Call => format!("{}({})", check.name, render_args(&check.args)),
            })
            .collect();
        if shown.is_empty() {
            lines.push("? visible only when its script says so".to_owned());
        } else {
            lines.push(format!("? visible when: {}", shown.join(", ")));
        }
    }
    if let Some(character) = &topic.flags.for_character {
        lines.push(format!("· spoken by {character}"));
    }
    if topic.flags.is_ambient {
        lines.push("· ambient: played without being chosen".to_owned());
    }
    if topic.flags.is_followup {
        lines.push("· follow-up: offered right after its predecessor".to_owned());
    }
    lines
}

fn rule_sentence(rule: &Rule, owner: Option<&str>, text: &Text) -> String {
    let first_class = rule.args.iter().find_map(|arg| match arg {
        Arg::Class { name } => Some(class_without_object_prefix(name).to_owned()),
        _ => None,
    });
    let first_text = rule.args.iter().find_map(|arg| match arg {
        Arg::Text { value } => Some(value.clone()),
        _ => None,
    });
    let first_name = rule.args.iter().find_map(|arg| match arg {
        Arg::Name { value } => Some(value.clone()),
        _ => None,
    });
    let line = |key: &Option<String>| match key {
        Some(key) => match text.get(key) {
            Some(line) => format!("\"{line}\""),
            None => key.clone(),
        },
        None => "?".to_owned(),
    };

    match rule.kind {
        RuleKind::HideIfKnows | RuleKind::HideIfKnowsId => match first_class {
            // The overwhelmingly common form: a topic that hides itself once it has been picked.
            Some(class) if Some(class.as_str()) == owner => "asked only once".to_owned(),
            Some(class) => format!("hidden once {class} is known"),
            None => "hidden once it is known".to_owned(),
        },
        RuleKind::AllowIfCharacterHasKnowledgeOf | RuleKind::AllowIfCharacterHasKnowledgeOfId => {
            format!(
                "only after {} is known",
                first_class.unwrap_or_else(|| "another topic".to_owned())
            )
        }
        RuleKind::RequireCharacterHasListenedTo => format!(
            "only once {} heard {}",
            first_name.unwrap_or_else(|| "the character".to_owned()),
            line(&first_text)
        ),
        RuleKind::RequireCharacterHasNotListenedTo => format!(
            "only while {} has not heard {}",
            first_name.unwrap_or_else(|| "the character".to_owned()),
            line(&first_text)
        ),
        RuleKind::RequireCharacterCloseToWaypoint => {
            let who = first_name.unwrap_or_else(|| "the character".to_owned());
            let where_ = rule
                .args
                .iter()
                .filter_map(|arg| match arg {
                    Arg::Name { value } => Some(value.clone()),
                    _ => None,
                })
                .nth(1)
                .unwrap_or_else(|| "a waypoint".to_owned());
            let radius = rule.args.iter().find_map(|arg| match arg {
                Arg::Float { value } => Some(*value),
                _ => None,
            });
            match radius {
                Some(radius) => format!("only while {who} is within {radius} of {where_}"),
                None => format!("only while {who} is near {where_}"),
            }
        }
        RuleKind::Add => format!("rule {}", render_args(&rule.args)),
        RuleKind::Other { ref name } => format!("{name}({})", render_args(&rule.args)),
    }
}

#[allow(clippy::too_many_arguments)]
fn print_step(
    conversation: &Conversation,
    step: &Step,
    text: &Text,
    level: usize,
    menu_depth: usize,
    depth: Option<usize>,
    ids: bool,
    printed: &mut BTreeSet<String>,
) {
    let indent = pad(level);
    let mark = if step.guard.conditional { "? " } else { "  " };
    match &step.kind {
        StepKind::Say {
            speaker,
            loc_key,
            expression,
        } => {
            let who = speaker.clone().unwrap_or_else(|| "?".to_owned());
            let spoken = match loc_key {
                Some(key) => match text.get(key) {
                    Some(line) => format!("\"{line}\""),
                    None => format!("<{key}>"),
                },
                None => "<no key>".to_owned(),
            };
            let mut line = format!("{indent}{mark}{who}: {spoken}");
            if ids {
                if let Some(key) = loc_key {
                    line.push_str(&format!("   [{key}]"));
                }
                if let Some(expression) = expression {
                    line.push_str(&format!(" [{expression}]"));
                }
            }
            println!("{line}");
        }
        StepKind::Subdialog { children } => {
            println!("{indent}{mark}opens a sub-menu:");
            let next_level = level + 1;
            if !subdialog_within_limit(menu_depth, depth) {
                println!(
                    "{}… {} option(s) not shown",
                    pad(next_level),
                    children.len()
                );
                return;
            }
            let next_menu_depth = menu_depth.saturating_add(1);
            for child in children {
                match conversation.topic(child) {
                    Some(topic) => print_topic(
                        conversation,
                        topic,
                        text,
                        next_level,
                        next_menu_depth,
                        depth,
                        ids,
                        printed,
                    ),
                    None => println!("{}- {child} (declared elsewhere)", pad(next_level)),
                }
            }
        }
        StepKind::ReturnToLastSelection => println!("{indent}{mark}back to the previous menu"),
        StepKind::EndConversation => println!("{indent}{mark}ends the conversation"),
        StepKind::Call { name, args } => {
            let mut line = format!("{indent}{mark}{name}({})", render_args(args));
            if step.guard.conditional && !step.guard.hints.is_empty() {
                line.push_str(&format!("   when {}", step.guard.hints.join(", ")));
            }
            println!("{line}");
        }
    }
}

fn subdialog_within_limit(menu_depth: usize, limit: Option<usize>) -> bool {
    limit.map_or(true, |limit| menu_depth < limit)
}

fn render_args(args: &[Arg]) -> String {
    args.iter()
        .filter(|arg| !matches!(arg, Arg::Null))
        .map(render_arg)
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_arg(arg: &Arg) -> String {
    match arg {
        Arg::Class { name } => class_without_object_prefix(name).to_owned(),
        Arg::Name { value } => format!("n\"{value}\""),
        Arg::Symbol { name } => name.clone(),
        Arg::Text { value } => format!("\"{value}\""),
        Arg::Int { value } => value.to_string(),
        Arg::Float { value } => format!("{value}"),
        Arg::Null => "null".to_owned(),
        Arg::Opaque => "…".to_owned(),
    }
}

/// How a conversation is named to a person: its non-hero participants, else its module.
fn participant_label(conversation: &Conversation) -> String {
    let who: Vec<&str> = conversation.npc_participants().collect();
    if who.is_empty() {
        conversation.module.clone()
    } else {
        who.join(" + ")
    }
}

fn pad(level: usize) -> String {
    "  ".repeat(level)
}

// ─── show ────────────────────────────────────────────────────────────────────

fn show(
    topic: &str,
    lang: &str,
    json: bool,
    cache: Option<PathBuf>,
    game: Option<PathBuf>,
) -> Result<()> {
    let graph = read_graph(cache, game)?;
    let wanted = class_without_object_prefix(topic).to_lowercase();
    let found = graph.conversations.iter().find_map(|conversation| {
        conversation
            .topics
            .iter()
            .find(|candidate| {
                class_without_object_prefix(&candidate.class).to_lowercase() == wanted
            })
            .map(|candidate| (conversation, candidate))
    });
    let Some((conversation, found)) = found else {
        bail!("no topic class matched {topic:?}");
    };

    if json {
        println!("{}", serde_json::to_string_pretty(found)?);
        return Ok(());
    }

    let text = load_text(&keys_of(conversation), lang);
    println!("{}", found.class);
    println!("in {}", conversation.module);
    if let Some(note) = &text.note {
        println!("note: {note}");
    }
    println!();
    let mut printed = BTreeSet::new();
    print_topic(
        conversation,
        found,
        &text,
        0,
        0,
        Some(1),
        true,
        &mut printed,
    );
    Ok(())
}

// ─── text ────────────────────────────────────────────────────────────────────

/// Every localization key this conversation uses, in the order the tree prints them.
fn ordered_keys(conversation: &Conversation) -> Vec<String> {
    let mut keys = Vec::new();
    let mut seen = BTreeSet::new();
    let mut visited = BTreeSet::new();

    fn walk(
        conversation: &Conversation,
        class: &str,
        keys: &mut Vec<String>,
        seen: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
    ) {
        if !visited.insert(class.to_owned()) {
            return;
        }
        let Some(topic) = conversation.topic(class) else {
            return;
        };
        fn push(key: &str, keys: &mut Vec<String>, seen: &mut BTreeSet<String>) {
            if seen.insert(key.to_lowercase()) {
                keys.push(key.to_owned());
            }
        }
        if let Some(key) = topic.caption.loc_key() {
            push(key, keys, seen);
        }
        for rule in &topic.rules {
            for key in rule_text_keys(rule) {
                push(key, keys, seen);
            }
        }
        for step in &topic.act {
            match &step.kind {
                StepKind::Say {
                    loc_key: Some(key), ..
                } => push(key, keys, seen),
                StepKind::Subdialog { children } => {
                    for child in children {
                        walk(conversation, child, keys, seen, visited);
                    }
                }
                _ => {}
            }
        }
    }

    for root in &conversation.roots {
        walk(conversation, root, &mut keys, &mut seen, &mut visited);
    }
    // Topics no root reaches still carry text somebody may want to change.
    for topic in &conversation.topics {
        walk(
            conversation,
            &topic.class,
            &mut keys,
            &mut seen,
            &mut visited,
        );
    }
    keys
}

/// Write the conversation's text as a `gore loc import` edits document.
fn text_edits(
    npc: &str,
    lang: &str,
    out: &PathBuf,
    cache: Option<PathBuf>,
    game: Option<PathBuf>,
) -> Result<()> {
    let graph = read_graph(cache, game)?;
    let conversation = resolve_one(&graph, npc)?;
    let keys = ordered_keys(conversation);
    if keys.is_empty() {
        bail!(
            "{} has no localized dialog text to export",
            participant_label(conversation)
        );
    }

    let columns = columns_for(lang);
    let wanted: HashSet<String> = keys.iter().map(|key| key.to_lowercase()).collect();
    let index = match load_name_index(&wanted) {
        NameIndexState::Ready(index) => index,
        NameIndexState::Absent => bail!(
            "no localization catalog yet — run `gore loc extract` first, otherwise there is no \
             text to edit"
        ),
        NameIndexState::Unreadable { path, detail } => {
            bail!(
                "localization catalog at {} is unreadable: {detail}",
                path.display()
            )
        }
        NameIndexState::Obstructed { path } => {
            bail!("{} is not a file, so no text could be read", path.display())
        }
    };

    // The game reads the newest populated column of a language, so an edit written to an older
    // one is a silent no-op. Pick the column each id is actually served from.
    let fallback = columns.first().cloned().unwrap_or_else(|| lang.to_owned());
    let mut rows = Vec::with_capacity(keys.len());
    let mut empty = 0usize;
    let mut used: BTreeMap<String, usize> = BTreeMap::new();
    for key in &keys {
        let names = index.names_for(key).unwrap_or(&[]);
        let chosen = name_in_columns(names, &columns);
        let (column, value) = match chosen {
            Some(name) => (name.language.clone(), name.text.clone()),
            None => {
                empty += 1;
                (fallback.clone(), String::new())
            }
        };
        *used.entry(column.clone()).or_default() += 1;
        rows.push((key.clone(), column, value));
    }

    let mut document = String::from("{\n");
    for (position, (key, column, value)) in rows.iter().enumerate() {
        let comma = if position + 1 == rows.len() { "" } else { "," };
        document.push_str(&format!(
            "  {}: {{ {}: {} }}{comma}\n",
            serde_json::to_string(key)?,
            serde_json::to_string(column)?,
            serde_json::to_string(value)?
        ));
    }
    document.push_str("}\n");
    fs::write(out, document).with_context(|| format!("writing {}", out.display()))?;

    let columns_used: Vec<String> = used
        .iter()
        .map(|(column, count)| format!("{column} ×{count}"))
        .collect();
    println!(
        "wrote {} line(s) for {} to {} ({})",
        rows.len(),
        participant_label(conversation),
        out.display(),
        columns_used.join(", ")
    );
    if empty > 0 {
        println!("{empty} of them have no text in this language yet");
    }
    println!(
        "edit the file, then: gore loc import --edits {}",
        out.display()
    );
    Ok(())
}

// ─── checkout / check / stage ────────────────────────────────────────────────

/// What `checkout` records so `check` and `stage` need no arguments beyond the directory.
#[derive(serde::Serialize, serde::Deserialize)]
struct EditManifest {
    /// Existing workspaces predate this field and are ordinary module edits.
    #[serde(default)]
    operation: DialogModuleOperation,
    /// The Modules TMap key, for `compile-module --module`.
    module: String,
    /// Where the compiler expects the file, for `compile-module --rel-path`.
    relative_path: String,
    /// The editable file, relative to the working directory.
    source_file: String,
    /// The untouched copy, relative to the working directory.
    pristine_file: String,
    participant: String,
    /// The exact cache this edit is bound to. A game update invalidates the whole contract.
    cache_sha256: String,
    /// `new-conversation` workspaces edit a loaded per-NPC settings anchor and must retain the
    /// private root plus at least one topic. Ordinary/legacy checkouts default to false.
    #[serde(default, skip_serializing_if = "is_false")]
    requires_topic_scaffold: bool,
    /// Explicit root-topic registrations for the bundle spec. Subdialog topics are wired by the
    /// authored `Subdialog` call and need no transient root registration.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    dialog_topics: Vec<DialogTopicRegistration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum DialogModuleOperation {
    Edit,
    Add,
}

impl Default for DialogModuleOperation {
    fn default() -> Self {
        Self::Edit
    }
}

impl DialogModuleOperation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Edit => "edit",
            Self::Add => "add",
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct DialogTopicRegistration {
    id: String,
    participant_name: String,
    topic_class: String,
    sentinel_class: String,
    #[serde(default, skip_serializing_if = "is_false")]
    allow_hidden: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn reflected_topic_path(class: &str) -> String {
    format!("/Script/Angelscript.{}", class_without_object_prefix(class))
}

fn registered_topic_class(path: &str) -> Result<String> {
    let leaf = path
        .strip_prefix("/Script/Angelscript.")
        .filter(|leaf| is_angelscript_identifier(leaf))
        .with_context(|| format!("invalid dialog topic class path {path:?}"))?;
    Ok(format!("U{leaf}"))
}

fn class_debug_ids(class: &dialog::ClassOutline) -> Result<Vec<i64>> {
    class
        .defaults
        .iter()
        .filter(|default| default.target == "DebugId")
        .map(|default| {
            let compact = default
                .statement
                .chars()
                .filter(|character| !character.is_whitespace())
                .collect::<String>();
            compact
                .strip_prefix("defaultDebugId=")
                .and_then(|value| value.strip_suffix(';'))
                .and_then(|value| value.parse::<i64>().ok())
                .with_context(|| {
                    format!(
                        "new topic {} has a DebugId that is not one signed 64-bit integer literal: {}",
                        class.name, default.statement
                    )
                })
        })
        .collect()
}

fn member_method_name(member: &str) -> Option<&str> {
    let tokens = member.split_whitespace().collect::<Vec<_>>();
    let mut index = 0usize;
    if tokens.first() == Some(&"UFUNCTION") && tokens.get(1) == Some(&"(") {
        let mut depth = 1usize;
        index = 2;
        while index < tokens.len() && depth != 0 {
            match tokens[index] {
                "(" => depth += 1,
                ")" => depth -= 1,
                _ => {}
            }
            index += 1;
        }
        if depth != 0 {
            return None;
        }
    }
    let parameters = tokens[index..].iter().position(|token| *token == "(")? + index;
    parameters
        .checked_sub(1)
        .and_then(|name| tokens.get(name).copied())
}

/// A new class has no shipped function record whose Unreal override metadata can be overlaid.
fn validate_new_topic_source_contract(class: &dialog::ClassOutline) -> Result<()> {
    const VISIBILITY: &str = "UFUNCTION ( BlueprintOverride ) bool IsVisible ( ) const";
    const ACT: &str = "UFUNCTION ( BlueprintOverride ) void Act ( )";

    for (hook, required) in [("IsVisible", VISIBILITY), ("Act", ACT)] {
        let implementation = format!("{hook}_Implementation");
        let declarations = class
            .members
            .iter()
            .filter(|member| {
                member_method_name(member)
                    .is_some_and(|name| name == hook || name == implementation)
            })
            .collect::<Vec<_>>();
        if declarations.len() != 1 || declarations[0].as_str() != required {
            bail!(
                "new topic {} must declare exactly one `{required}` and no other `{hook}` or `{implementation}` overload; found {:?}. New classes have no shipped Unreal function metadata to inherit",
                class.name,
                declarations,
            );
        }
    }

    let debug_ids = class_debug_ids(class)?;
    let [debug_id] = debug_ids.as_slice() else {
        bail!(
            "new topic {} must author exactly one nonzero `default DebugId = <int64>;`; found {}",
            class.name,
            debug_ids.len()
        );
    };
    if *debug_id == 0 {
        bail!("new topic {} may not use the unset DebugId 0", class.name);
    }
    Ok(())
}

fn literal_bool_default(class: &dialog::ClassOutline, target: &str) -> Result<Option<bool>> {
    let defaults = class
        .defaults
        .iter()
        .filter(|default| default.target == target)
        .collect::<Vec<_>>();
    let ([] | [_]) = defaults.as_slice() else {
        bail!(
            "new topic {} must not declare more than one `default {target}`; found {}",
            class.name,
            defaults.len()
        );
    };
    let Some(default) = defaults.first() else {
        return Ok(None);
    };
    let tokens = code_tokens(&default.statement)?;
    let spelling = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    if spelling == ["default", target, "=", "true", ";"] {
        Ok(Some(true))
    } else if spelling == ["default", target, "=", "false", ";"] {
        Ok(Some(false))
    } else {
        bail!(
            "new topic {} must spell `default {target}` as one literal true/false assignment; found {}",
            class.name,
            default.statement
        )
    }
}

fn validate_new_topic_placement(class: &dialog::ClassOutline, is_subdialog: bool) -> Result<()> {
    let declares_subtopic = literal_bool_default(class, "bIsSubTopic")?.unwrap_or(false);
    if declares_subtopic != is_subdialog {
        let expected = if is_subdialog {
            "declare `default bIsSubTopic = true;`"
        } else {
            "not declare `default bIsSubTopic = true;`"
        };
        bail!(
            "new topic {} is wired as a {} and must {expected}",
            class.name,
            if is_subdialog { "Subdialog" } else { "root" }
        );
    }
    Ok(())
}

fn outline_class_identity(class: &dialog::ClassOutline) -> String {
    if class.namespace.is_empty() {
        class.name.clone()
    } else {
        format!("{}::{}", class.namespace, class.name)
    }
}

fn resolve_outline_parent<'a>(
    classes: &'a BTreeMap<String, &'a dialog::ClassOutline>,
    class: &dialog::ClassOutline,
) -> Option<&'a dialog::ClassOutline> {
    let parent = class.super_class.as_deref()?;
    let parent = parent.strip_prefix("::").unwrap_or(parent);
    let identity = if parent.contains("::") || class.namespace.is_empty() {
        parent.to_owned()
    } else {
        format!("{}::{parent}", class.namespace)
    };
    classes.get(&identity).copied()
}

/// Return the supported direct topic additions and reject an indirect new-topic hierarchy.
///
/// The runtime contract below validates each new topic's own methods, defaults, and placement.
/// Letting `class B : A` slip past merely because only `A` directly names the shipped root would
/// leave `B` completely unchecked. Follow parents through the authored overlay so that shape fails
/// closed until indirect topic inheritance has an equally complete runtime contract.
fn direct_added_topic_classes(
    outline: &dialog::SourceOutline,
    added_classes: &[String],
    root: &dialog::ClassOutline,
) -> Result<BTreeSet<String>> {
    let classes = outline
        .classes
        .iter()
        .map(|class| (outline_class_identity(class), class))
        .collect::<BTreeMap<_, _>>();
    let added = added_classes
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let root_identity = outline_class_identity(root);
    let mut direct = BTreeSet::new();

    for class in outline
        .classes
        .iter()
        .filter(|class| added.contains(class.name.as_str()))
    {
        let original = class;
        let mut cursor = class;
        let mut chain = Vec::new();
        let mut seen = BTreeSet::new();

        loop {
            let identity = outline_class_identity(cursor);
            if !seen.insert(identity.clone()) {
                chain.push(identity);
                bail!(
                    "new-class inheritance cycle while checking {}: {}. The dialog pipeline cannot determine this overlay's topic ancestry",
                    original.name,
                    chain.join(" -> ")
                );
            }
            chain.push(identity);

            let Some(parent) = resolve_outline_parent(&classes, cursor) else {
                break;
            };
            let parent_identity = outline_class_identity(parent);
            if parent_identity == root_identity {
                if chain.len() == 1 {
                    direct.insert(original.name.clone());
                } else {
                    chain.push(parent_identity);
                    bail!(
                        "new topic {} derives indirectly from the shipped conversation root {}: {}. Indirect new-topic inheritance is unsupported; derive each new topic directly from {}",
                        original.name,
                        root.name,
                        chain.join(" -> "),
                        root.name
                    );
                }
                break;
            }
            cursor = parent;
        }
    }

    Ok(direct)
}

const CONVERSATION_NAMESPACE: &str = "G1R::Conversation";

/// Native conversation parents are admitted only in their exact direct, unqualified class form.
/// Looking only at the final `::Leaf` would let an authored surrogate impersonate a native base.
fn directly_derives_native(class: &dialog::ClassOutline, parent: &str) -> bool {
    class.kind == "class" && class.super_class.as_deref() == Some(parent)
}

fn expected_topic_root(participant: &str) -> String {
    format!("UTopic_Hero__{participant}")
}

fn literal_name_default(class: &dialog::ClassOutline, target: &str) -> Result<Option<String>> {
    let defaults = class
        .defaults
        .iter()
        .filter(|default| default.target == target)
        .collect::<Vec<_>>();
    let ([] | [_]) = defaults.as_slice() else {
        bail!(
            "{} must not declare more than one `default {target}`; found {}",
            class.name,
            defaults.len()
        );
    };
    let Some(default) = defaults.first() else {
        return Ok(None);
    };
    let invalid = || {
        anyhow::anyhow!(
            "{} must spell `default {target}` as one FName literal; found {}",
            class.name,
            default.statement
        )
    };
    let mut rest = default.statement.trim_start();
    rest = rest.strip_prefix("default").ok_or_else(&invalid)?;
    if !rest.chars().next().is_some_and(char::is_whitespace) {
        return Err(invalid());
    }
    rest = rest.trim_start();
    rest = rest.strip_prefix(target).ok_or_else(&invalid)?;
    if rest
        .chars()
        .next()
        .is_some_and(|character| !character.is_whitespace() && character != '=')
    {
        return Err(invalid());
    }
    rest = rest.trim_start();
    rest = rest.strip_prefix('=').ok_or_else(&invalid)?.trim_start();
    rest = rest.strip_prefix('n').ok_or_else(&invalid)?.trim_start();
    if !rest.starts_with('"') {
        return Err(invalid());
    }

    let bytes = rest.as_bytes();
    let mut escaped = false;
    let mut end = None;
    for (index, byte) in bytes.iter().enumerate().skip(1) {
        if escaped {
            escaped = false;
        } else if *byte == b'\\' {
            escaped = true;
        } else if *byte == b'"' {
            end = Some(index + 1);
            break;
        }
    }
    let end = end.ok_or_else(&invalid)?;
    if rest[end..].trim() != ";" {
        return Err(invalid());
    }
    serde_json::from_str(&rest[..end])
        .map(Some)
        .with_context(|| format!("reading {}::default {target}", class.name))
}

fn validate_topicless_settings<'a>(
    outline: &'a dialog::SourceOutline,
    report: &dialog::EditReport,
    participant: &str,
) -> Result<&'a dialog::ClassOutline> {
    let settings = validate_settings_anchor_source(outline, participant)?;
    if report.added_classes.contains(&settings.name) {
        bail!(
            "a conversation-settings anchor edit may not replace its shipped settings class {}",
            settings.name
        );
    }
    Ok(settings)
}

/// The shipped per-NPC settings declaration that makes a newly added conversation discoverable.
/// Its authored source must remain present and bound to the exact participant; the runtime-proven
/// path only appends topic declarations in another namespace of this already loaded module.
fn validate_settings_anchor_source<'a>(
    outline: &'a dialog::SourceOutline,
    participant: &str,
) -> Result<&'a dialog::ClassOutline> {
    let settings = outline
        .classes
        .iter()
        .filter(|class| directly_derives_native(class, "UConversationCharacterSettings"))
        .collect::<Vec<_>>();
    let [settings] = settings.as_slice() else {
        bail!(
            "a conversation-settings anchor must retain exactly one direct UConversationCharacterSettings class; found {}",
            settings.len()
        );
    };
    if literal_name_default(settings, "ForCharacter")?.as_deref() != Some(participant) {
        bail!(
            "{} must retain `default ForCharacter = n{};`",
            settings.name,
            serde_json::to_string(participant)?
        );
    }
    Ok(settings)
}

fn validate_settings_anchor_declarations_unchanged(report: &dialog::EditReport) -> Result<()> {
    let added = report
        .added_classes
        .iter()
        .map(|class| class.as_str())
        .collect::<BTreeSet<_>>();
    let changed_bodies = report
        .changed
        .iter()
        .filter(|change| !added.contains(change.class.rsplit("::").next().unwrap_or(&change.class)))
        .map(|change| format!("{}::{}", change.class, change.member))
        .collect::<Vec<_>>();
    let changed_defaults = report
        .changed_defaults
        .iter()
        .filter(|change| !added.contains(change.class.rsplit("::").next().unwrap_or(&change.class)))
        .map(|change| format!("{}::default {}", change.class, change.target))
        .collect::<Vec<_>>();
    if !changed_bodies.is_empty() || !changed_defaults.is_empty() {
        bail!(
            "a new-conversation settings-anchor edit must leave every shipped declaration unchanged; changed: {}",
            changed_bodies
                .into_iter()
                .chain(changed_defaults)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(())
}

fn validate_new_conversation_base<'a>(
    outline: &'a dialog::SourceOutline,
    report: &dialog::EditReport,
    participant: &str,
    require_new_settings: bool,
) -> Result<&'a dialog::ClassOutline> {
    let expected_root = expected_topic_root(participant);
    let native_roots = outline
        .classes
        .iter()
        .filter(|class| {
            report.added_classes.contains(&class.name)
                && directly_derives_native(class, "UG1RDialogTopic")
        })
        .collect::<Vec<_>>();
    let [root] = native_roots.as_slice() else {
        bail!(
            "a new conversation must declare exactly one new private topic base derived directly from UG1RDialogTopic; found {}",
            native_roots.len()
        );
    };
    if root.name != expected_root {
        bail!(
            "the private topic base for {participant} must be named {expected_root}, not {}",
            root.name
        );
    }
    if !require_new_settings {
        validate_topicless_settings(outline, report, participant)?;
    }
    let expected_namespace = CONVERSATION_NAMESPACE;
    if root.namespace != expected_namespace {
        bail!(
            "the private topic base {} must stay in namespace `{expected_namespace}`, not `{}`",
            root.name,
            root.namespace
        );
    }
    if literal_name_default(root, "ForCharacter")?.as_deref() != Some(participant) {
        bail!(
            "{expected_root} must declare `default ForCharacter = n{};`",
            serde_json::to_string(participant)?
        );
    }
    if literal_name_default(root, "WithCharacter")?.as_deref() != Some("Hero") {
        bail!("{expected_root} must declare `default WithCharacter = n\"Hero\";`");
    }

    if require_new_settings {
        let expected_settings = format!("UConversationCharacterSettings_G1R_{participant}");
        let settings = outline
            .classes
            .iter()
            .filter(|class| {
                report.added_classes.contains(&class.name)
                    && directly_derives_native(class, "UConversationCharacterSettings")
            })
            .collect::<Vec<_>>();
        let [settings] = settings.as_slice() else {
            bail!(
                "a new conversation module must declare exactly one new UConversationCharacterSettings class; found {}",
                settings.len()
            );
        };
        if settings.name != expected_settings {
            bail!(
                "the settings class for {participant} must be named {expected_settings}, not {}",
                settings.name
            );
        }
        if settings.namespace != CONVERSATION_NAMESPACE {
            bail!(
                "the settings class {expected_settings} must be declared in namespace `{CONVERSATION_NAMESPACE}`, not `{}`",
                settings.namespace
            );
        }
        if literal_name_default(settings, "ForCharacter")?.as_deref() != Some(participant) {
            bail!(
                "{expected_settings} must declare `default ForCharacter = n{};`",
                serde_json::to_string(participant)?
            );
        }
    }
    Ok(root)
}

/// Qualify a complete graph of newly authored topics. Every new topic is either a native root or
/// has exactly one Subdialog parent, and a Subdialog owned by a new topic may contain only other
/// topics from the same new conversation. This makes all-new multi-level trees explicit instead
/// of treating only shipped-parent insertion as safe.
fn validate_new_topic_tree(
    authored: &str,
    outline: &dialog::SourceOutline,
    report: &dialog::EditReport,
    root: &dialog::ClassOutline,
    added_topics: &BTreeSet<String>,
    registrations: &BTreeMap<String, usize>,
) -> Result<()> {
    if outline
        .functions
        .iter()
        .any(|declaration| free_function_has_name(declaration, "Say"))
    {
        bail!(
            "an all-new topic tree may not declare a module-local free function named `Say`; the qualified `::Say(...)` separator must resolve to the shipped dialog function"
        );
    }

    let mut debug_ids = BTreeMap::<i64, String>::new();
    for class_name in added_topics {
        let class = outline
            .classes
            .iter()
            .find(|class| &class.name == class_name)
            .with_context(|| {
                format!("new topic {class_name} disappeared from the source outline")
            })?;
        if class.kind != "class" {
            bail!(
                "new topic {class_name} must be declared as a class, not a {}",
                class.kind
            );
        }
        if class.namespace != root.namespace {
            bail!(
                "new topic {class_name} must be declared beside its private root {} in namespace `{}`, not `{}`",
                root.name,
                root.namespace,
                class.namespace
            );
        }
        validate_new_topic_source_contract(class)?;
        let debug_id = class_debug_ids(class)?[0];
        if let Some(other) = debug_ids.insert(debug_id, class_name.clone()) {
            bail!(
                "new topics {other} and {class_name} duplicate DebugId {debug_id}; every new topic needs a unique nonzero DebugId"
            );
        }
    }

    let (calls, call_shapes) = inventory_fixed_subdialog_calls(authored)?;
    let direct_act_calls = direct_act_subdialog_call_counts(authored)?;
    let mut parents = BTreeMap::<String, Vec<String>>::new();
    let mut children = BTreeMap::<String, Vec<String>>::new();
    let added_classes = report
        .added_classes
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    for (owner, owner_calls) in &calls {
        let owner_is_new_topic = added_topics.contains(owner);
        let references_new = owner_calls
            .iter()
            .flatten()
            .flatten()
            .any(|child| added_topics.contains(child));
        if !owner_is_new_topic && !references_new {
            continue;
        }
        if owner_is_new_topic
            && call_shapes
                .get(owner)
                .into_iter()
                .flatten()
                .any(|shape| *shape != SubdialogCallShape::Global)
        {
            bail!(
                "new topic {owner} must open its new children with the compiler-qualified global `::Subdialog(this, ...)` form; `this.Subdialog(...)` does not bind newly appended child classes on the qualified all-new-tree path"
            );
        }
        let [slots] = owner_calls.as_slice() else {
            bail!(
                "{owner} participates in the new topic tree but has {} source-level Subdialog calls; exactly one is required",
                owner_calls.len()
            );
        };
        if direct_act_calls.get(owner).copied().unwrap_or(0) != 1 {
            bail!(
                "the Subdialog call in {owner} that owns new topics must appear directly in `Act` or `Act_Implementation`; helper-indirect menu transitions are not runtime-qualified"
            );
        }
        if !owner_is_new_topic && added_classes.contains(owner.as_str()) {
            bail!(
                "new topic(s) are referenced by added non-topic class {owner}; a Subdialog parent must itself be a topic"
            );
        }

        let mut saw_hole = false;
        let mut unique = BTreeSet::new();
        for child in slots {
            let Some(child) = child else {
                saw_hole = true;
                continue;
            };
            if saw_hole {
                bail!(
                    "the Subdialog call in {owner} has populated topic {child} after an empty slot"
                );
            }
            if !unique.insert(child.as_str()) {
                bail!("the Subdialog call in {owner} duplicates topic {child}");
            }
            if owner_is_new_topic && !added_topics.contains(child) {
                bail!(
                    "new topic {owner} references foreign Subdialog child {child}; an all-new tree may contain only topics declared for this conversation"
                );
            }
            if added_topics.contains(child) {
                parents
                    .entry(child.clone())
                    .or_default()
                    .push(owner.clone());
                if owner_is_new_topic {
                    children
                        .entry(owner.clone())
                        .or_default()
                        .push(child.clone());
                }
            }
        }
    }

    for class in added_topics {
        let topic_parents = parents.get(class).map(Vec::as_slice).unwrap_or(&[]);
        if topic_parents.len() > 1 {
            bail!(
                "new topic {class} has multiple Subdialog parents: {}",
                topic_parents.join(", ")
            );
        }
        let registered_roots = registrations.get(class).copied().unwrap_or(0);
        if registered_roots > 1 || (registered_roots == 1 && !topic_parents.is_empty()) {
            bail!(
                "new topic {class} must have one placement, but has {registered_roots} root registration(s) and Subdialog parent(s) {:?}",
                topic_parents
            );
        }
        let class_outline = outline
            .classes
            .iter()
            .find(|candidate| candidate.name == *class)
            .expect("added topic was resolved above");
        validate_new_topic_placement(class_outline, !topic_parents.is_empty())?;
    }

    // `Subdialog` is a blocking ability task. The live all-new-tree oracle soft-locked when one
    // topic's otherwise-empty Act selected a child whose otherwise-empty Act immediately opened
    // the next menu. The same bytecode and new-symbol references work once either transition has
    // an unconditional `Say` first, and the shipped corpus contains no consecutive actionless
    // pair. Only that spoken-line form is runtime-qualified: declarations, assignments, empty
    // blocks and control-flow wrappers must not accidentally bypass this fail-closed guard.
    let immediate_subdialog_acts = immediate_subdialog_acts(authored)?;
    for (child, owners) in &parents {
        for owner in owners {
            if immediate_subdialog_acts.contains(owner) && immediate_subdialog_acts.contains(child)
            {
                bail!(
                    "new topic tree edge {owner} -> {child} chains two actionless `Subdialog` Acts; this menu-to-menu transition soft-locks at runtime. Add an unconditional top-level `::Say(...)` before either Subdialog call"
                );
            }
        }
    }

    // Kahn's algorithm over new-to-new edges gives one deterministic cycle refusal.
    let mut internal_indegree = added_topics
        .iter()
        .map(|class| (class.clone(), 0usize))
        .collect::<BTreeMap<_, _>>();
    for descendants in children.values() {
        for child in descendants {
            *internal_indegree
                .get_mut(child)
                .expect("new edge targets a new topic") += 1;
        }
    }
    let mut ready = internal_indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(class, _)| class.clone())
        .collect::<Vec<_>>();
    let mut visited = 0usize;
    while let Some(class) = ready.pop() {
        visited += 1;
        for child in children.get(&class).map(Vec::as_slice).unwrap_or(&[]) {
            let degree = internal_indegree
                .get_mut(child)
                .expect("new edge targets a new topic");
            *degree -= 1;
            if *degree == 0 {
                ready.push(child.clone());
            }
        }
    }
    if visited != added_topics.len() {
        let cycle = internal_indegree
            .into_iter()
            .filter(|(_, degree)| *degree != 0)
            .map(|(class, _)| class)
            .collect::<Vec<_>>();
        bail!(
            "new Subdialog tree contains a cycle involving {}",
            cycle.join(", ")
        );
    }

    let mut reachable = BTreeSet::new();
    let mut pending = added_topics
        .iter()
        .filter(|class| {
            registrations.get(*class).copied().unwrap_or(0) == 1
                || parents.get(*class).is_none_or(Vec::is_empty)
                || parents.get(*class).is_some_and(|owners| {
                    owners
                        .first()
                        .is_some_and(|owner| !added_topics.contains(owner))
                })
        })
        .cloned()
        .collect::<Vec<_>>();
    while let Some(class) = pending.pop() {
        if !reachable.insert(class.clone()) {
            continue;
        }
        pending.extend(children.get(&class).into_iter().flatten().cloned());
    }
    let orphaned = added_topics
        .difference(&reachable)
        .cloned()
        .collect::<Vec<_>>();
    if !orphaned.is_empty() {
        bail!(
            "new Subdialog tree contains unreachable/orphan topic(s): {}",
            orphaned.join(", ")
        );
    }
    Ok(())
}

fn free_function_has_name(declaration: &str, expected: &str) -> bool {
    let tokens = declaration.split_whitespace().collect::<Vec<_>>();
    tokens
        .windows(2)
        .any(|pair| pair[0] == expected && pair[1] == "(")
}

/// Refuse the pre-anchor first-conversation shape independently of a mutable manifest flag.
///
/// Some shipped Story modules contain settings but no topic root. They can be checked out like a
/// normal conversation, but the live oracle proved that adding a private root there does not make
/// the game discover it. A first conversation must instead come from `new-conversation`, which
/// edits the NPC's already-loaded AI settings anchor.
fn validate_topicless_module_is_not_scaffolded(
    conversation: Option<&Conversation>,
    outline: &dialog::SourceOutline,
    report: &dialog::EditReport,
) -> Result<()> {
    let Some(conversation) = conversation.filter(|item| item.root_class.is_none()) else {
        return Ok(());
    };
    let adds_private_root = outline.classes.iter().any(|class| {
        report.added_classes.contains(&class.name)
            && directly_derives_native(class, "UG1RDialogTopic")
    });
    if adds_private_root {
        bail!(
            "{} is a shipped topicless conversation module, but that module shape is not discovered as a first conversation at runtime; use `gore dialog new-conversation` so the exact loaded per-NPC settings anchor is edited instead",
            conversation.module
        );
    }
    Ok(())
}

/// Bind each new topic to one native root, legacy adapter registration, or one Subdialog parent.
fn validate_topic_registrations(
    manifest: &EditManifest,
    report: &dialog::EditReport,
    authored: &str,
    cache: &[u8],
) -> Result<()> {
    validate_manifest_settings_anchor(cache, manifest)?;
    let graph =
        dialog::build(cache).context("re-reading the base dialog for registration checks")?;
    let matches = graph
        .conversations
        .iter()
        .filter(|conversation| conversation.module == manifest.module)
        .collect::<Vec<_>>();
    let conversation = match manifest.operation {
        DialogModuleOperation::Edit => match matches.as_slice() {
            [conversation] => Some(*conversation),
            [] if manifest.requires_topic_scaffold => None,
            _ => bail!(
                "the edit module maps to {} base conversations; exactly one is required",
                matches.len()
            ),
        },
        DialogModuleOperation::Add => {
            if !matches.is_empty() {
                bail!("an added conversation must not already exist in the base dialog graph");
            }
            None
        }
    };
    let outline = dialog::read_outline(authored)
        .map_err(|reason| anyhow::anyhow!("inventorying new topic classes: {reason}"))?;
    validate_topicless_module_is_not_scaffolded(conversation, &outline, report)?;
    if manifest.requires_topic_scaffold {
        validate_settings_anchor_declarations_unchanged(report)?;
    }

    let added_settings = outline
        .classes
        .iter()
        .filter(|class| {
            report.added_classes.contains(&class.name)
                && directly_derives_native(class, "UConversationCharacterSettings")
        })
        .collect::<Vec<_>>();
    if manifest.operation == DialogModuleOperation::Edit && !added_settings.is_empty() {
        bail!(
            "an edited conversation may not add another UConversationCharacterSettings class: {}",
            added_settings
                .iter()
                .map(|class| class.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    if report.added_classes.is_empty() && manifest.dialog_topics.is_empty() {
        if manifest.operation == DialogModuleOperation::Add || manifest.requires_topic_scaffold {
            bail!(
                "a new conversation workspace must retain its private topic base and at least one direct topic option"
            );
        }
        if let Some(conversation) = conversation.filter(|item| item.root_class.is_none()) {
            let participants = conversation.npc_participants().collect::<Vec<_>>();
            let [participant] = participants.as_slice() else {
                bail!(
                    "a topicless conversation must name exactly one NPC participant; found {}",
                    participants.len()
                );
            };
            validate_topicless_settings(&outline, report, participant)?;
        }
        return Ok(());
    }

    let (root_outline, participant) = match conversation.and_then(|item| {
        item.root_class
            .as_deref()
            .map(|root| (item, root.to_owned()))
    }) {
        Some((conversation, root_class)) => {
            let native_roots = outline
                .classes
                .iter()
                .filter(|class| {
                    report.added_classes.contains(&class.name)
                        && directly_derives_native(class, "UG1RDialogTopic")
                })
                .collect::<Vec<_>>();
            if !native_roots.is_empty() {
                bail!(
                    "{} already has private topic base {root_class}; a same-module edit may not add another UG1RDialogTopic root",
                    manifest.module
                );
            }
            let roots = outline
                .classes
                .iter()
                .filter(|class| class.name == root_class)
                .collect::<Vec<_>>();
            let [root] = roots.as_slice() else {
                bail!(
                    "the private root class {root_class} has {} source identities; exactly one is required",
                    roots.len()
                );
            };
            let participant = conversation
                .npc_participants()
                .next()
                .context("the base conversation names no NPC participant")?;
            (*root, participant.to_owned())
        }
        None => {
            let participant = match conversation {
                Some(conversation) => {
                    let participants = conversation.npc_participants().collect::<Vec<_>>();
                    let [participant] = participants.as_slice() else {
                        bail!(
                            "a topicless conversation must name exactly one NPC participant; found {}",
                            participants.len()
                        );
                    };
                    (*participant).to_owned()
                }
                None => manifest.participant.clone(),
            };
            let root = validate_new_conversation_base(
                &outline,
                report,
                &participant,
                manifest.operation == DialogModuleOperation::Add,
            )?;
            (root, participant)
        }
    };
    let added_topics = direct_added_topic_classes(&outline, &report.added_classes, root_outline)?;
    if conversation.is_none() && added_topics.is_empty() {
        bail!("a new conversation module must declare at least one direct topic option");
    }
    if conversation.is_some_and(|item| item.root_class.is_none()) && added_topics.is_empty() {
        bail!("a topicless conversation edit must add at least one direct topic option");
    }

    let participants = conversation
        .map(|item| {
            item.npc_participants()
                .map(|value| value.to_ascii_lowercase())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_else(|| BTreeSet::from([participant.to_ascii_lowercase()]));
    let sentinel = if manifest.dialog_topics.is_empty() {
        None
    } else {
        let conversation = conversation.context(
            "dialog_topics cannot register a new or previously topicless conversation; native same-module discovery is required",
        )?;
        Some(
            sentinel_of(conversation)
                .map(|topic| reflected_topic_path(&topic.class))
                .context("the base conversation has no root sentinel for registration")?,
        )
    };
    let mut registrations = BTreeMap::<String, usize>::new();
    for registration in &manifest.dialog_topics {
        let class = registered_topic_class(&registration.topic_class)?;
        if !added_topics.contains(&class) {
            bail!(
                "dialog_topics registers {}, but that is not one newly added direct topic class in {}",
                registration.topic_class,
                manifest.module
            );
        }
        if !participants.contains(&registration.participant_name.to_ascii_lowercase()) {
            bail!(
                "dialog_topics participant {:?} does not belong to {}",
                registration.participant_name,
                manifest.module
            );
        }
        if registration.sentinel_class != sentinel.as_deref().expect("resolved above") {
            bail!(
                "dialog_topics sentinel {:?} is not this conversation's checked sentinel {:?}",
                registration.sentinel_class,
                sentinel.as_deref().expect("resolved above")
            );
        }
        *registrations.entry(class).or_default() += 1;
    }

    validate_new_topic_tree(
        authored,
        &outline,
        report,
        root_outline,
        &added_topics,
        &registrations,
    )
}

const MANIFEST_NAME: &str = "gore-dialog-edit.json";

fn ensure_empty_dialog_workspace(out: &Path) -> Result<()> {
    if !out.exists() {
        return Ok(());
    }
    let mut entries = fs::read_dir(out).with_context(|| format!("reading {}", out.display()))?;
    if entries
        .next()
        .transpose()
        .with_context(|| format!("reading {}", out.display()))?
        .is_some()
    {
        bail!(
            "{} is not empty; choose an empty --out directory so no stale spec or source survives",
            out.display()
        );
    }
    Ok(())
}

fn digest_of(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The Binds cache beside the script cache, resolved the way the compiler resolves it, so an
/// emitted checkout is byte-identical to the tree the compiler will build.
fn native_api(cache_path: &std::path::Path) -> Option<gore_as::cache::binds::NativeApi> {
    let path = match std::env::var_os("GORE_AS_BINDS") {
        Some(path) => PathBuf::from(path),
        None => cache_path.parent()?.join("Binds.Cache"),
    };
    gore_as::cache::binds::NativeApi::load(&path)
}

fn checkout(npc: &str, out: &PathBuf, cache: Option<PathBuf>, game: Option<PathBuf>) -> Result<()> {
    ensure_empty_dialog_workspace(out)?;
    let (path, bytes) = read_cache(cache, game)?;
    let graph = dialog::build(&bytes).context("reading dialog from the script cache")?;
    let conversation = resolve_one(&graph, npc)?;

    let taken = dialog::checkout(&bytes, &conversation.module, native_api(&path))
        .with_context(|| format!("taking {} out of the cache", conversation.module))?;
    let leaf = taken
        .relative_path
        .rsplit('/')
        .next()
        .unwrap_or("module.as")
        .to_owned();

    fs::create_dir_all(out.join("pristine"))
        .with_context(|| format!("creating {}", out.display()))?;
    let source_file = out.join(&leaf);
    let pristine_file = out.join("pristine").join(&leaf);
    fs::write(&source_file, &taken.source)
        .with_context(|| format!("writing {}", source_file.display()))?;
    fs::write(&pristine_file, &taken.source)
        .with_context(|| format!("writing {}", pristine_file.display()))?;

    let manifest = EditManifest {
        operation: DialogModuleOperation::Edit,
        module: taken.module.clone(),
        relative_path: taken.relative_path.clone(),
        source_file: leaf.clone(),
        pristine_file: format!("pristine/{leaf}"),
        participant: participant_label(conversation),
        cache_sha256: digest_of(&bytes),
        requires_topic_scaffold: false,
        dialog_topics: Vec::new(),
    };
    let manifest_path = out.join(MANIFEST_NAME);
    fs::write(
        &manifest_path,
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )
    .with_context(|| format!("writing {}", manifest_path.display()))?;

    println!("{} — {}", manifest.participant, taken.module);
    println!("edit    {}", source_file.display());
    println!("as-was  {}", pristine_file.display());
    println!();
    println!("you may change: method bodies and reconstructed class defaults — Caption,");
    println!("                PriorityRank, Rules and topic flags included");
    println!("you may add:    a same-module topic class, its own methods/defaults, and new");
    println!("                string/text ids; stage then selects --allow-new-symbols");
    if conversation.topics.is_empty() {
        println!("this module has no topics yet: use `gore dialog new-conversation` to scaffold");
        println!("its private topic base and first option without replacing its settings");
    }
    println!("you may not:    remove shipped defaults, classes, methods or fields, or change");
    println!("                the layout/signature of an existing class");
    println!();
    println!("then: gore dialog check {}", out.display());
    Ok(())
}

/// Read the manifest and re-derive everything the check needs from the same cache.
fn open_edit(
    dir: &PathBuf,
    cache: Option<PathBuf>,
    game: Option<PathBuf>,
) -> Result<(EditManifest, dialog::EditReport, PathBuf)> {
    let manifest_path = dir.join(MANIFEST_NAME);
    let manifest: EditManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path)
            .with_context(|| format!("reading {}", manifest_path.display()))?,
    )
    .with_context(|| format!("parsing {}", manifest_path.display()))?;

    let (path, bytes) = read_cache(cache, game)?;
    let digest = digest_of(&bytes);
    if digest != manifest.cache_sha256 {
        bail!(
            "this edit was taken from a different script cache ({}…, now {}…). A game update \
             changes every identity the edit is checked against; take a fresh checkout",
            &manifest.cache_sha256[..12.min(manifest.cache_sha256.len())],
            &digest[..12.min(digest.len())]
        );
    }
    validate_manifest_settings_anchor(&bytes, &manifest)
        .context("the new-conversation manifest is not bound to one loaded NPC settings module")?;

    let taken = match manifest.operation {
        DialogModuleOperation::Edit => {
            let taken = dialog::checkout(&bytes, &manifest.module, native_api(&path))
                .with_context(|| format!("re-reading {}", manifest.module))?;
            if manifest.relative_path != taken.relative_path {
                bail!(
                    "the edit manifest redirects {} from its cache path {} to {}; take a fresh checkout",
                    manifest.module,
                    taken.relative_path,
                    manifest.relative_path
                );
            }
            taken
        }
        DialogModuleOperation::Add => {
            let (expected_module, expected_relative_path) =
                new_conversation_module_names(&manifest.participant)?;
            if manifest.module != expected_module
                || manifest.relative_path != expected_relative_path
            {
                bail!(
                    "an added conversation for {:?} must use module {} at {}; the manifest names {} at {}",
                    manifest.participant,
                    expected_module,
                    expected_relative_path,
                    manifest.module,
                    manifest.relative_path
                );
            }
            let modules = gore_as::cache::model::parse_modules(&bytes)
                .map_err(|error| anyhow::anyhow!("parsing the script cache: {error}"))?;
            gore_as::cache::emit_all::validate_add_module_target(
                &modules,
                &manifest.module,
                &manifest.relative_path,
            )
            .map_err(|reason| anyhow::anyhow!("invalid add-module target: {reason}"))?;
            dialog::Checkout {
                module: manifest.module.clone(),
                relative_path: manifest.relative_path.clone(),
                source: String::new(),
                default_classes: BTreeSet::new(),
                unsupported_generated_methods: Vec::new(),
            }
        }
    };
    let source_path = dir.join(&manifest.source_file);
    let authored = fs::read_to_string(&source_path)
        .with_context(|| format!("reading {}", source_path.display()))?;
    let known = dialog::known_names(&bytes).context("collecting the cache's names")?;
    let report = dialog::verify(&taken, &authored, &known);
    if report.is_carryable() {
        validate_saturated_subdialog_edits(&taken.source, &authored, &report.added_classes)
            .context("the edited Subdialog shape is not runtime-qualified")?;
        validate_topic_registrations(&manifest, &report, &authored, &bytes)
            .context("the dialog registration manifest is not bound to this checked source")?;
    }
    Ok((manifest, report, source_path))
}

fn check(dir: &PathBuf, json: bool, cache: Option<PathBuf>, game: Option<PathBuf>) -> Result<()> {
    let (manifest, report, source_path) = match open_edit(dir, cache, game) {
        Ok(opened) => opened,
        Err(error) if json => {
            let document = serde_json::json!({
                "module": serde_json::Value::Null,
                "participant": serde_json::Value::Null,
                "unchanged": false,
                "safe": false,
                "requires_new_symbols": false,
                "changed": [],
                "changed_defaults": [],
                "added_classes": [],
                "added_functions": [],
                "new_strings": [],
                "new_static_names": [],
                "violations": [format!("{error:#}")],
            });
            println!("{}", serde_json::to_string_pretty(&document)?);
            bail!("1 problem(s)");
        }
        Err(error) => return Err(error),
    };

    if json {
        let document = serde_json::json!({
            "operation": manifest.operation.as_str(),
            "module": manifest.module,
            "participant": manifest.participant,
            "unchanged": report.unchanged,
            "safe": report.is_carryable(),
            "requires_new_symbols": report.requires_new_symbols(),
            "changed": report.changed.iter().map(|body| {
                serde_json::json!({ "class": body.class, "member": body.member })
            }).collect::<Vec<_>>(),
            "changed_defaults": report.changed_defaults.iter().map(|change| {
                serde_json::json!({ "class": change.class, "target": change.target })
            }).collect::<Vec<_>>(),
            "added_classes": report.added_classes,
            "added_functions": report.added_functions,
            "new_strings": report.new_strings,
            "new_static_names": report.new_static_names,
            "violations": report.violations.iter().map(|violation| violation.explain())
                .collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&document)?);
        return if report.is_carryable() {
            Ok(())
        } else {
            bail!("{} problem(s)", report.violations.len())
        };
    }

    println!("{} — {}", manifest.participant, manifest.module);
    println!("{}", source_path.display());
    println!();

    if !report.violations.is_empty() {
        println!("this edit is not safe to compile:");
        for violation in &report.violations {
            println!("  - {}", violation.explain());
        }
        println!();
        bail!(
            "{} problem(s); nothing was compiled",
            report.violations.len()
        );
    }

    if report.unchanged {
        println!("nothing changed yet — the file is still the one the game ships");
        return Ok(());
    }

    println!("this edit is safe for the current compile/remap contract. Rewritten bodies:");
    for body in &report.changed {
        println!("  - {}::{}", body.class, body.member);
    }
    if !report.changed_defaults.is_empty() {
        println!("rewritten defaults:");
        for change in &report.changed_defaults {
            println!("  - {}::default {}", change.class, change.target);
        }
    }
    if report.requires_new_symbols() {
        println!("new symbols: --allow-new-symbols is required");
    }
    println!();
    println!("checked offline against the shipped module. The compiler still has the last word");
    println!(
        "on whether the source parses; run it with `gore dialog stage {}`",
        dir.display()
    );
    Ok(())
}

fn stage(
    dir: &PathBuf,
    mod_name: &str,
    cache: Option<PathBuf>,
    game: Option<PathBuf>,
) -> Result<()> {
    gore_mod::validate_mod_name(mod_name).context("invalid --mod-name")?;
    let compiler_game_arg = game.clone();
    let (manifest, report, source_path) = open_edit(dir, cache, game)?;
    if !report.is_carryable() {
        for violation in &report.violations {
            println!("  - {}", violation.explain());
        }
        bail!(
            "{} problem(s); run `gore dialog check {}` for the detail",
            report.violations.len(),
            dir.display()
        );
    }
    if report.unchanged {
        bail!("the file is unchanged, so there is nothing to build");
    }
    let compiler_game = compiler_game_for(&manifest, compiler_game_arg)?;

    let mini = format!("{mod_name}.mini.Cache");
    let mut spec = serde_json::json!({
        "meta": { "name": mod_name, "version": "0.1.0", "author": "" },
        "scripts": [{
            "op": manifest.operation.as_str(),
            "module_name": manifest.module,
            "mini_cache": mini,
        }],
    });
    if !manifest.dialog_topics.is_empty() {
        spec["dialog_topics"] = serde_json::to_value(&manifest.dialog_topics)?;
    }
    let spec_path = dir.join("spec.json");
    fs::write(
        &spec_path,
        format!("{}\n", serde_json::to_string_pretty(&spec)?),
    )
    .with_context(|| format!("writing {}", spec_path.display()))?;

    let work_dir = dir.join(".gore-as-work");
    fs::create_dir_all(&work_dir).with_context(|| format!("creating {}", work_dir.display()))?;
    let mini_path = dir.join(&mini);

    println!("wrote {}", spec_path.display());
    println!();
    println!("rewritten method bodies:");
    for body in &report.changed {
        println!("  - {}::{}", body.class, body.member);
    }
    for change in &report.changed_defaults {
        println!("  - {}::default {}", change.class, change.target);
    }
    for class in &report.added_classes {
        println!("  - new class {class}");
    }
    for function in &report.added_functions {
        println!("  - new function {function}");
    }
    println!();
    println!("next:");
    println!(
        "  {}",
        compile_module_command(
            &manifest,
            &source_path,
            &work_dir,
            &mini_path,
            &compiler_game,
            report.requires_new_symbols(),
        )
    );
    println!(
        "  gore mod build --spec {} -o {}",
        powershell_quote(&spec_path),
        powershell_quote(&dir.join("build")),
    );
    println!();
    if report.requires_new_symbols() {
        println!("`--allow-new-symbols` is required because this edit introduces names, strings,");
        println!("functions or classes absent from the pristine cache.");
    } else {
        println!("strict remapping is sufficient because this edit introduces no new symbols.");
    }
    println!("Complete authored defaults are regenerated by the compiler; byte-for-byte default");
    println!("carry is only the fallback when no existing class authors defaults.");
    println!();
    println!("Building writes only below this edit directory. Deployment is a separate,");
    println!("installation-writing step and is intentionally not run or suggested as automatic.");
    Ok(())
}

/// `compile-module` targets a resolved game installation, not an arbitrary cache file. Bind the
/// printed command to an installation whose current script cache is exactly the checkout base.
fn compiler_game_for(manifest: &EditManifest, game: Option<PathBuf>) -> Result<PathBuf> {
    let root = gore_loc::config::game_root(game).context("resolving compiler game path")?;
    let script_cache = gore_mod::resolve_game_paths(&root).script_cache;
    let bytes = fs::read(&script_cache).with_context(|| {
        format!(
            "reading compiler base cache {}. `compile-module` cannot target an arbitrary --cache file",
            script_cache.display()
        )
    })?;
    if digest_of(&bytes) != manifest.cache_sha256 {
        bail!(
            "the game cache at {} is not the cache this edit was checked out from. `stage` cannot \
             print a safe compile command for a different or arbitrary --cache file",
            script_cache.display()
        );
    }
    Ok(root)
}

fn powershell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "''"))
}

fn powershell_quote_text(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn compile_module_command(
    manifest: &EditManifest,
    source: &Path,
    work_dir: &Path,
    out: &Path,
    game: &Path,
    allow_new_symbols: bool,
) -> String {
    let allow_new = if allow_new_symbols {
        " --allow-new-symbols"
    } else {
        ""
    };
    format!(
        "gore as compile-module --backend standalone --op {} --module {} --rel-path {} \
         --source {} --work-dir {}{} -o {} --game {}",
        manifest.operation.as_str(),
        powershell_quote_text(&manifest.module),
        powershell_quote_text(&manifest.relative_path),
        powershell_quote(source),
        powershell_quote(work_dir),
        allow_new,
        powershell_quote(out),
        powershell_quote(game),
    )
}

// ─── new-topic ───────────────────────────────────────────────────────────────

pub struct NewTopicRequest {
    pub npc: String,
    pub caption: Option<String>,
    pub caption_key: Option<String>,
    pub class: Option<String>,
    pub subdialog_of: Option<String>,
    pub subdialog_position: Option<usize>,
    pub priority_rank: Option<i32>,
    pub mod_name: String,
    pub out: PathBuf,
    pub cache: Option<PathBuf>,
    pub game: Option<PathBuf>,
}

pub struct NewConversationRequest {
    pub npc: String,
    pub caption: Option<String>,
    pub caption_key: Option<String>,
    pub class: Option<String>,
    pub priority_rank: i32,
    pub mod_name: String,
    pub out: PathBuf,
    pub cache: Option<PathBuf>,
    pub game: Option<PathBuf>,
}

fn caption_default_line(caption: Option<&str>, caption_key: Option<&str>) -> Result<String> {
    match (caption, caption_key) {
        (Some(text), _) => Ok(format!(
            "    default Caption = FText::FromString(n{}.ToString());",
            serde_json::to_string(text)?
        )),
        (_, Some(key)) => Ok(format!(
            "    default Caption = LocText({});",
            serde_json::to_string(key)?
        )),
        _ => bail!("pass --caption or --caption-key"),
    }
}

fn new_topic_caption_line(caption: Option<&str>, caption_key: Option<&str>) -> Result<String> {
    caption_default_line(caption, caption_key)
}

fn new_conversation_caption_line(
    caption: Option<&str>,
    caption_key: Option<&str>,
) -> Result<String> {
    caption_default_line(caption, caption_key)
}

fn new_conversation_module_names(participant: &str) -> Result<(String, String)> {
    if participant.eq_ignore_ascii_case("Hero") || !is_angelscript_identifier(participant) {
        bail!(
            "a new conversation NPC must be one exact AngelScript-safe identifier such as `NC_SLD_GORN_699FM`, not {participant:?}"
        );
    }
    let leaf = format!("Conversation_{participant}");
    Ok((
        format!("Story.G1R.Conversation.{leaf}"),
        format!("Story/G1R/Conversation/{leaf}.as"),
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConversationSettingsAnchor {
    module: String,
    participant: String,
}

/// Recognize only the per-NPC AI settings module shape observed to be loaded by the runtime.
/// Merely finding some `UConversationCharacterSettings` subclass is not enough: those classes
/// occur in unrelated modules too, and a fuzzy participant match could bind a new conversation to
/// the wrong NPC.
fn conversation_settings_anchor_identity(module: &str) -> Option<ConversationSettingsAnchor> {
    let parts = module.split('.').collect::<Vec<_>>();
    let [ai, agent, human, config, participant, leaf] = parts.as_slice() else {
        return None;
    };
    if !ai.eq_ignore_ascii_case("AI")
        || !agent.eq_ignore_ascii_case("AIAgent")
        || !human.eq_ignore_ascii_case("Human")
        || !config.eq_ignore_ascii_case("Config")
    {
        return None;
    }
    let suffix = leaf.strip_prefix("ConversationCharacterSettings_")?;
    if !suffix.eq_ignore_ascii_case(participant) {
        return None;
    }
    Some(ConversationSettingsAnchor {
        module: module.to_owned(),
        participant: (*participant).to_owned(),
    })
}

fn select_exact_settings_anchor<'a>(
    requested_participant: &str,
    candidates: &'a [ConversationSettingsAnchor],
) -> Result<&'a ConversationSettingsAnchor> {
    let matches = candidates
        .iter()
        .filter(|anchor| {
            anchor
                .participant
                .eq_ignore_ascii_case(requested_participant)
        })
        .collect::<Vec<_>>();
    let [anchor] = matches.as_slice() else {
        if matches.is_empty() {
            bail!(
                "no loaded per-NPC conversation-settings anchor exactly matches {requested_participant:?}; a separate new Story.G1R.Conversation module compiles but is not discovered by the game, so it cannot be staged safely"
            );
        }
        bail!(
            "{requested_participant:?} has {} matching conversation-settings anchors: {}. Refusing an ambiguous NPC binding",
            matches.len(),
            matches
                .iter()
                .map(|anchor| anchor.module.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    };
    Ok(*anchor)
}

/// Resolve one exact, already present settings anchor. A standalone Add module compiles, but the
/// live game does not discover it; therefore absence and ambiguity both fail closed here.
fn resolve_conversation_settings_anchor(
    cache: &[u8],
    requested_participant: &str,
) -> Result<ConversationSettingsAnchor> {
    new_conversation_module_names(requested_participant)?;
    let modules = gore_as::cache::model::parse_modules(cache)
        .map_err(|error| anyhow::anyhow!("parsing the script cache: {error}"))?;
    let candidates = modules
        .iter()
        .filter_map(|module| conversation_settings_anchor_identity(&module.name))
        .collect::<Vec<_>>();
    let anchor = select_exact_settings_anchor(requested_participant, &candidates)?.clone();
    let module = modules
        .iter()
        .find(|module| module.name == anchor.module)
        .expect("the selected anchor came from this module list");
    let settings_count = module
        .classes
        .iter()
        .filter(|class| class.super_class.as_deref() == Some("UConversationCharacterSettings"))
        .count();
    if settings_count != 1 {
        bail!(
            "loaded conversation-settings anchor {} declares {settings_count} direct UConversationCharacterSettings classes; exactly one is required",
            anchor.module
        );
    }
    Ok(anchor)
}

fn validate_manifest_settings_anchor(cache: &[u8], manifest: &EditManifest) -> Result<()> {
    if manifest.operation == DialogModuleOperation::Add {
        bail!(
            "this legacy dialog workspace uses --op add, but a separate conversation module is not discovered by the game; create a fresh new-conversation workspace for an NPC with one loaded conversation-settings anchor"
        );
    }
    if !manifest.requires_topic_scaffold {
        if conversation_settings_anchor_identity(&manifest.module).is_some() {
            bail!(
                "a dialog workspace targeting a loaded per-NPC conversation-settings module must retain `requires_topic_scaffold = true`; refusing a manifest that disables its anchor checks"
            );
        }
        return Ok(());
    }
    let anchor = resolve_conversation_settings_anchor(cache, &manifest.participant)?;
    if anchor.module != manifest.module {
        bail!(
            "the new conversation for {:?} must edit its loaded settings anchor {}, not {}",
            manifest.participant,
            anchor.module,
            manifest.module
        );
    }
    Ok(())
}

/// Keep only the characters an AngelScript identifier may carry.
fn identifier(text: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            if capitalize {
                out.extend(character.to_uppercase());
                capitalize = false;
            } else {
                out.push(character);
            }
        } else {
            capitalize = true;
        }
    }
    out
}

/// The vanilla topic that proves the live topic set belongs to this conversation.
///
/// The exit option is the right one: every conversation has it, it is always in the root menu,
/// and it is never conditional. Falling back to the first root keeps a conversation with an
/// unusual exit usable, and no conversation at all means there is nothing to attach to.
fn sentinel_of(conversation: &Conversation) -> Option<&Topic> {
    let roots: Vec<&Topic> = conversation
        .roots
        .iter()
        .filter_map(|class| conversation.topic(class))
        .collect();
    roots
        .iter()
        .find(|topic| {
            topic.act.len() == 1
                && matches!(topic.act[0].kind, StepKind::EndConversation)
                && matches!(topic.visibility, Visibility::Always)
                && topic.rules.is_empty()
        })
        .or_else(|| roots.first())
        .copied()
}

/// Every class the cache already declares, for a collision check on the authored name.
fn declared_classes(bytes: &[u8]) -> Result<BTreeSet<String>> {
    let modules = gore_as::cache::model::parse_modules(bytes)
        .map_err(|error| anyhow::anyhow!("parsing the script cache: {error}"))?;
    Ok(modules
        .iter()
        .flat_map(|module| module.classes.iter())
        .map(|class| class.name.to_lowercase())
        .collect())
}

fn is_angelscript_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn class_without_object_prefix(name: &str) -> &str {
    name.strip_prefix('U').unwrap_or(name)
}

/// Give a generated topic its own stable story-debug identity.
fn generated_topic_debug_id(module: &str, class: &str, source: &str) -> i64 {
    use sha2::{Digest, Sha256};

    let used = source
        .lines()
        .filter_map(|line| {
            let compact = line
                .chars()
                .filter(|character| !character.is_whitespace())
                .collect::<String>();
            compact
                .strip_prefix("defaultDebugId=")
                .and_then(|value| value.strip_suffix(';'))
                .and_then(|value| value.parse::<i64>().ok())
        })
        .collect::<BTreeSet<_>>();
    for nonce in 0u64.. {
        let digest = Sha256::digest(format!("gore-dialog-topic\0{module}\0{class}\0{nonce}"));
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&digest[..8]);
        let candidate = (u64::from_le_bytes(bytes) & i64::MAX as u64) as i64;
        if candidate != 0 && !used.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!("the finite DebugId space cannot be exhausted by one conversation module")
}

fn resolve_topic_in<'a>(conversation: &'a Conversation, wanted: &str) -> Result<&'a Topic> {
    let wanted = class_without_object_prefix(wanted).to_ascii_lowercase();
    let matches = conversation
        .topics
        .iter()
        .filter(|topic| class_without_object_prefix(&topic.class).eq_ignore_ascii_case(&wanted))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [topic] => Ok(topic),
        [] => bail!(
            "no topic {wanted:?} belongs to {}. Use `gore dialog tree {} --ids`",
            participant_label(conversation),
            participant_label(conversation),
        ),
        _ => bail!("topic {wanted:?} is not unique in {}", conversation.module),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CodeToken {
    text: String,
    start: usize,
    end: usize,
}

/// Lex only code punctuation and identifiers. Comments and quoted literals deliberately vanish,
/// so a scaffold never patches a word that merely looks like `class` or `Subdialog` in prose.
fn code_tokens(source: &str) -> Result<Vec<CodeToken>> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            index += 2;
            while index < bytes.len() && !matches!(bytes[index], b'\r' | b'\n') {
                index += 1;
            }
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            index += 2;
            let mut closed = false;
            while index < bytes.len() {
                if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    index += 2;
                    closed = true;
                    break;
                }
                index += 1;
            }
            if !closed {
                bail!("authored source has an unterminated block comment");
            }
            continue;
        }
        if matches!(bytes[index], b'\'' | b'\"') {
            let quote = bytes[index];
            index += 1;
            let mut closed = false;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                    continue;
                }
                if bytes[index] == quote {
                    index += 1;
                    closed = true;
                    break;
                }
                index += 1;
            }
            if !closed {
                bail!("authored source has an unterminated quoted literal");
            }
            continue;
        }
        if bytes[index].is_ascii_alphabetic() || bytes[index] == b'_' {
            let start = index;
            index += 1;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
            {
                index += 1;
            }
            tokens.push(CodeToken {
                text: source[start..index].to_owned(),
                start,
                end: index,
            });
            continue;
        }

        let start = index;
        let character = source[index..]
            .chars()
            .next()
            .context("reading authored source")?;
        index += character.len_utf8();
        tokens.push(CodeToken {
            text: character.to_string(),
            start,
            end: index,
        });
    }
    Ok(tokens)
}

fn matching_close(tokens: &[CodeToken], open: usize, left: &str, right: &str) -> Result<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        if token.text == left {
            depth += 1;
        } else if token.text == right {
            depth = depth
                .checked_sub(1)
                .with_context(|| format!("unmatched {right} in authored source"))?;
            if depth == 0 {
                return Ok(index);
            }
        }
    }
    bail!("unclosed {left} in authored source")
}

fn namespace_name(tokens: &[CodeToken]) -> Result<String> {
    if tokens.is_empty() {
        bail!("namespace declaration has no name");
    }
    let mut parts = Vec::new();
    let mut index = 0usize;
    while index < tokens.len() {
        let name = &tokens[index].text;
        if !is_angelscript_identifier(name) {
            bail!("namespace declaration contains an invalid name {name:?}");
        }
        parts.push(name.as_str());
        index += 1;
        if index == tokens.len() {
            break;
        }
        if tokens.get(index).is_none_or(|token| token.text != ":")
            || tokens.get(index + 1).is_none_or(|token| token.text != ":")
        {
            bail!("namespace declaration is not a `Name::Name` path");
        }
        index += 2;
    }
    Ok(parts.join("::"))
}

/// Put generated declarations in the namespace that owns `class`, immediately before the
/// innermost namespace's closing brace. Moving the new topic to global scope would change both
/// its compiler identity and whether it can resolve the module-private conversation base.
fn append_to_class_namespace(source: &str, class: &str, addition: &str) -> Result<String> {
    let tokens = code_tokens(source)?;
    let mut declarations = Vec::new();
    for index in 0..tokens.len().saturating_sub(1) {
        if !matches!(tokens[index].text.as_str(), "class" | "struct")
            || tokens[index + 1].text != class
        {
            continue;
        }
        let Some(open) = ((index + 2)..tokens.len())
            .find(|candidate| matches!(tokens[*candidate].text.as_str(), "{" | ";"))
        else {
            bail!("class {class} has no body");
        };
        if tokens[open].text == "{" {
            declarations.push(index);
        }
    }
    let [class_at] = declarations.as_slice() else {
        bail!(
            "class {class} has {} source bodies; exactly one is required",
            declarations.len()
        );
    };

    let mut scopes = Vec::<(usize, usize, String)>::new();
    for index in 0..tokens.len() {
        if tokens[index].text != "namespace" {
            continue;
        }
        let Some(open) = ((index + 1)..tokens.len())
            .find(|candidate| matches!(tokens[*candidate].text.as_str(), "{" | ";"))
        else {
            bail!("namespace at byte {} has no body", tokens[index].start);
        };
        if tokens[open].text == ";" {
            continue;
        }
        let close = matching_close(&tokens, open, "{", "}")?;
        if open < *class_at && *class_at < close {
            scopes.push((open, close, namespace_name(&tokens[index + 1..open])?));
        }
    }
    scopes.sort_by_key(|(open, _, _)| *open);

    let mut edited = source.to_owned();
    let addition = addition.trim_end();
    if let Some((_, close, _)) = scopes.last() {
        let insert_at = tokens[*close].start;
        let prefix = if source[..insert_at].ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        };
        edited.insert_str(insert_at, &format!("{prefix}{addition}\n"));
    } else {
        if !edited.ends_with('\n') {
            edited.push('\n');
        }
        edited.push('\n');
        edited.push_str(addition);
        edited.push('\n');
    }
    Ok(edited)
}

/// Append declarations in an explicit namespace without rewriting any byte of the shipped source
/// prefix. The live new-conversation oracle used this exact arrangement: the global per-NPC
/// settings class remains untouched, while its new private root and topics live under the normal
/// `G1R::Conversation` identity inside the same already-loaded module.
fn append_conversation_namespace(source: &str, addition: &str) -> String {
    let mut edited = source.to_owned();
    if !edited.ends_with('\n') {
        edited.push('\n');
    }
    edited.push_str("\nnamespace G1R::Conversation\n{\n");
    edited.push_str(addition.trim_end());
    edited.push_str("\n}\n");
    edited
}

const SUBDIALOG_TOPIC_SLOTS: usize = 20;
const EMPTY_SUBDIALOG_SLOT: &str = "TSubclassOf<UConversationTopic>(nullptr)";
const BACK_CAPTION_KEY: &str = "TEXT_BACK";
const END_CAPTION_KEY: &str = "TEXT_DIALOG_END";
const DEFAULT_ROOT_PRIORITY_RANK: i32 = 2;
const DEFAULT_SUBDIALOG_PRIORITY_RANK: i32 = 0;

fn is_closing_caption(caption: &Caption) -> bool {
    matches!(
        caption,
        Caption::LocKey { key }
            if key.eq_ignore_ascii_case(END_CAPTION_KEY)
                || key.eq_ignore_ascii_case(BACK_CAPTION_KEY)
    )
}

fn rank_before(anchor: i32) -> Result<i32> {
    let candidate = anchor
        .checked_sub(1)
        .context("the trailing root topic already uses the smallest supported PriorityRank")?;
    if candidate == -1 {
        // Rank -1 has native forced-topic semantics. It is available as an explicit override, but
        // an ordinary root scaffold must never opt into it merely because the trailing row is 0.
        return anchor
            .checked_sub(2)
            .context("no ordinary PriorityRank exists before the trailing root topic");
    }
    Ok(candidate)
}

/// Pick an ordinary root-menu rank that leaves a closing row after the new option.
///
/// Prefer the first rank before every root End/Back row the cache identifies. Older or unusual
/// conversations sometimes use a custom caption for their final row; there the graph's already
/// ordered last root is the conservative anchor. The fixed rank 2 is only for the otherwise
/// impossible no-root fallback (a normal topicless conversation uses `new-conversation`).
fn automatic_root_priority_rank(conversation: &Conversation) -> Result<i32> {
    let closing_rank = conversation
        .roots
        .iter()
        .filter_map(|class| conversation.topic(class))
        .filter(|topic| is_closing_caption(&topic.caption))
        .map(|topic| topic.priority.unwrap_or(0))
        .min();
    let anchor = match closing_rank {
        Some(rank) => Some(rank),
        None => conversation
            .roots
            .last()
            .and_then(|class| conversation.topic(class))
            .map(|topic| topic.priority.unwrap_or(0)),
    };
    let Some(anchor) = anchor else {
        return Ok(DEFAULT_ROOT_PRIORITY_RANK);
    };
    let anchor = i32::try_from(anchor).with_context(|| {
        format!("the trailing root topic has unsupported PriorityRank {anchor}")
    })?;
    rank_before(anchor)
}

fn new_topic_priority_rank(
    conversation: &Conversation,
    subdialog: bool,
    requested: Option<i32>,
) -> Result<i32> {
    match requested {
        Some(rank) => Ok(rank),
        None if subdialog => Ok(DEFAULT_SUBDIALOG_PRIORITY_RANK),
        None => automatic_root_priority_rank(conversation),
    }
}

type FixedSubdialogCalls = BTreeMap<String, Vec<Vec<Option<String>>>>;
type FixedSubdialogShapes = BTreeMap<String, Vec<SubdialogCallShape>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SubdialogCallShape {
    Instance,
    Global,
}

/// Bind a `Subdialog` identifier to one of the two source shapes emitted by the compiler.
/// Merely finding the method name is not enough: rewriting another receiver, a free call, or a
/// namespace-owned helper would silently edit code whose runtime meaning we have not qualified.
fn subdialog_call_shape(
    tokens: &[CodeToken],
    call: usize,
    class_open: usize,
) -> Result<SubdialogCallShape> {
    if call >= class_open + 3 && tokens[call - 2].text == "this" && tokens[call - 1].text == "." {
        return Ok(SubdialogCallShape::Instance);
    }

    if call >= class_open + 3 && tokens[call - 2].text == ":" && tokens[call - 1].text == ":" {
        // `Owner::Subdialog` is not the global `::Subdialog` form. The decompiler emits the
        // supported call as a standalone statement, so an identifier immediately before the
        // two colons proves that this is a different callee rather than global qualification.
        if call >= class_open + 4 && is_angelscript_identifier(&tokens[call - 3].text) {
            bail!(
                "the Subdialog call is namespace- or owner-qualified; only `::Subdialog(this, 20 topic slots)` is supported"
            );
        }
        return Ok(SubdialogCallShape::Global);
    }

    bail!(
        "the Subdialog call has an unsupported receiver; only `this.Subdialog(20 topic slots)` or `::Subdialog(this, 20 topic slots)` is supported"
    )
}

fn call_argument_ranges(
    tokens: &[CodeToken],
    open: usize,
    close: usize,
) -> Result<Vec<(usize, usize)>> {
    if open + 1 == close {
        return Ok(Vec::new());
    }
    let mut ranges = Vec::new();
    let mut start = open + 1;
    let mut nested = 0usize;
    for index in (open + 1)..close {
        match tokens[index].text.as_str() {
            "(" | "[" | "{" => nested += 1,
            ")" | "]" | "}" => {
                nested = nested
                    .checked_sub(1)
                    .context("unmatched delimiter inside Subdialog arguments")?;
            }
            "," if nested == 0 => {
                if start == index {
                    bail!("the Subdialog call contains an empty argument");
                }
                ranges.push((start, index - 1));
                start = index + 1;
            }
            _ => {}
        }
    }
    if nested != 0 {
        bail!("the Subdialog call contains an unclosed argument expression");
    }
    if start == close {
        bail!("the Subdialog call has a trailing empty argument");
    }
    ranges.push((start, close - 1));
    Ok(ranges)
}

fn is_empty_subdialog_slot(tokens: &[CodeToken], range: (usize, usize)) -> bool {
    let expected = [
        "TSubclassOf",
        "<",
        "UConversationTopic",
        ">",
        "(",
        "nullptr",
        ")",
    ];
    let (start, end) = range;
    end - start + 1 == expected.len()
        && expected
            .iter()
            .enumerate()
            .all(|(offset, expected)| tokens[start + offset].text == *expected)
}

/// Accept the bare or namespace-qualified class values emitted for a populated topic slot.
fn subdialog_class_name(tokens: &[CodeToken], range: (usize, usize)) -> Option<&str> {
    let (start, end) = range;
    let mut index = start;
    if !is_angelscript_identifier(&tokens[index].text) {
        return None;
    }
    let mut name = tokens[index].text.as_str();
    index += 1;
    while index <= end {
        if index + 2 > end
            || tokens[index].text != ":"
            || tokens[index + 1].text != ":"
            || !is_angelscript_identifier(&tokens[index + 2].text)
        {
            return None;
        }
        name = tokens[index + 2].text.as_str();
        index += 3;
    }
    Some(name)
}

/// Inventory the fixed-width calls in each topic class without assigning meaning to their body.
///
/// `dialog check` already proves that bare class identities are unambiguous before this helper is
/// called. Keeping the bare owner here therefore matches `subdialog_class_name`, which deliberately
/// accepts both bare and namespace-qualified child values but returns the class leaf.
fn inventory_fixed_subdialog_calls(
    source: &str,
) -> Result<(FixedSubdialogCalls, FixedSubdialogShapes)> {
    let tokens = code_tokens(source)?;
    let mut calls = BTreeMap::<String, Vec<Vec<Option<String>>>>::new();
    let mut shapes = BTreeMap::<String, Vec<SubdialogCallShape>>::new();

    for declaration in 0..tokens.len().saturating_sub(1) {
        if tokens[declaration].text != "class" {
            continue;
        }
        let owner = tokens[declaration + 1].text.clone();
        let Some(class_open) = ((declaration + 2)..tokens.len())
            .find(|candidate| matches!(tokens[*candidate].text.as_str(), "{" | ";"))
        else {
            bail!("class {owner} has no body or forward declaration terminator");
        };
        if tokens[class_open].text == ";" {
            continue;
        }
        let class_close = matching_close(&tokens, class_open, "{", "}")?;

        for call in (class_open + 1)..class_close.saturating_sub(1) {
            if tokens[call].text != "Subdialog" || tokens[call + 1].text != "(" {
                continue;
            }
            let open = call + 1;
            let close = matching_close(&tokens, open, "(", ")")?;
            if close >= class_close {
                bail!("the Subdialog call in {owner} leaves its class body");
            }
            let shape = subdialog_call_shape(&tokens, call, class_open)
                .with_context(|| format!("binding the Subdialog call in {owner}"))?;
            let arguments = call_argument_ranges(&tokens, open, close)?;
            let slots = match (shape, arguments.as_slice()) {
                (SubdialogCallShape::Global, [owner_argument, slots @ ..])
                    if arguments.len() == SUBDIALOG_TOPIC_SLOTS + 1 =>
                {
                    let (start, end) = *owner_argument;
                    if start != end || tokens[start].text != "this" {
                        bail!(
                            "the Subdialog call in {owner} has 21 arguments but its first argument is not `this`"
                        );
                    }
                    slots
                }
                (SubdialogCallShape::Instance, slots)
                    if arguments.len() == SUBDIALOG_TOPIC_SLOTS =>
                {
                    slots
                }
                (SubdialogCallShape::Global, _) => bail!(
                    "the global Subdialog call in {owner} must have `this` followed by exactly {SUBDIALOG_TOPIC_SLOTS} topic slots; found {} arguments",
                    arguments.len()
                ),
                (SubdialogCallShape::Instance, _) => bail!(
                    "the instance Subdialog call in {owner} must have exactly {SUBDIALOG_TOPIC_SLOTS} topic slots; found {} arguments",
                    arguments.len()
                ),
            };

            let children = slots
                .iter()
                .map(|range| {
                    if is_empty_subdialog_slot(&tokens, *range) {
                        Ok(None)
                    } else {
                        subdialog_class_name(&tokens, *range)
                            .map(|name| Some(name.to_owned()))
                            .with_context(|| {
                                format!(
                                    "the Subdialog call in {owner} contains a populated slot that is not a bare or namespace-qualified topic class"
                                )
                            })
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            shapes.entry(owner.clone()).or_default().push(shape);
            calls.entry(owner.clone()).or_default().push(children);
        }
    }

    Ok((calls, shapes))
}

fn fixed_subdialog_calls(source: &str) -> Result<FixedSubdialogCalls> {
    Ok(inventory_fixed_subdialog_calls(source)?.0)
}

/// Count source-level Subdialog calls lexically inside each class's Act override.
///
/// Class-wide inventory is still used to validate the fixed call itself, but an edge into a newly
/// authored topic must not hide that call in a synchronous helper. That shape would evade the
/// actionless-transition guard without providing the runtime-qualified `Say` separator.
fn direct_act_subdialog_call_counts(source: &str) -> Result<BTreeMap<String, usize>> {
    let tokens = code_tokens(source)?;
    let mut counts = BTreeMap::new();

    for declaration in 0..tokens.len().saturating_sub(1) {
        if tokens[declaration].text != "class" {
            continue;
        }
        let owner = tokens[declaration + 1].text.clone();
        let Some(class_open) = ((declaration + 2)..tokens.len())
            .find(|candidate| matches!(tokens[*candidate].text.as_str(), "{" | ";"))
        else {
            bail!("class {owner} has no body or forward declaration terminator");
        };
        if tokens[class_open].text == ";" {
            continue;
        }
        let class_close = matching_close(&tokens, class_open, "{", "}")?;
        let mut item_start = class_open + 1;
        let mut index = item_start;
        while index < class_close {
            match tokens[index].text.as_str() {
                "{" => {
                    let body_close = matching_close(&tokens, index, "{", "}")?;
                    let declaration_tokens = &tokens[item_start..index];
                    let is_act = declaration_tokens.windows(2).any(|pair| {
                        matches!(pair[0].text.as_str(), "Act" | "Act_Implementation")
                            && pair[1].text == "("
                    });
                    if is_act {
                        let count = (index + 1..body_close.saturating_sub(1))
                            .filter(|call| {
                                tokens[*call].text == "Subdialog" && tokens[*call + 1].text == "("
                            })
                            .count();
                        *counts.entry(owner.clone()).or_default() += count;
                    }
                    index = body_close + 1;
                    item_start = index;
                }
                ";" => {
                    index += 1;
                    item_start = index;
                }
                _ => index += 1,
            }
        }
    }

    Ok(counts)
}

/// Topic Acts that open a Subdialog before an unconditional, top-level `::Say(...)`.
///
/// `Say` is the transition separator proven by the live three-level oracle. Debug markers,
/// declarations, assignments, empty blocks and control flow are not allowed to masquerade as that
/// proof. A `Say` nested in a branch is likewise insufficient because it may not execute. Anything
/// after `Subdialog` is too late to prevent the re-entrant start.
fn immediate_subdialog_acts(source: &str) -> Result<BTreeSet<String>> {
    let tokens = code_tokens(source)?;
    let mut acts = BTreeSet::new();

    for declaration in 0..tokens.len().saturating_sub(1) {
        if tokens[declaration].text != "class" {
            continue;
        }
        let owner = tokens[declaration + 1].text.clone();
        let Some(class_open) = ((declaration + 2)..tokens.len())
            .find(|candidate| matches!(tokens[*candidate].text.as_str(), "{" | ";"))
        else {
            bail!("class {owner} has no body or forward declaration terminator");
        };
        if tokens[class_open].text == ";" {
            continue;
        }
        let class_close = matching_close(&tokens, class_open, "{", "}")?;
        let mut item_start = class_open + 1;
        let mut index = item_start;
        while index < class_close {
            match tokens[index].text.as_str() {
                "{" => {
                    let body_close = matching_close(&tokens, index, "{", "}")?;
                    let declaration_tokens = &tokens[item_start..index];
                    let is_act = declaration_tokens.windows(2).any(|pair| {
                        matches!(pair[0].text.as_str(), "Act" | "Act_Implementation")
                            && pair[1].text == "("
                    });
                    if is_act && immediate_subdialog_body(&tokens, index, body_close)? {
                        acts.insert(owner.clone());
                    }
                    index = body_close + 1;
                    item_start = index;
                }
                ";" => {
                    index += 1;
                    item_start = index;
                }
                _ => index += 1,
            }
        }
    }

    Ok(acts)
}

fn immediate_subdialog_body(
    tokens: &[CodeToken],
    body_open: usize,
    body_close: usize,
) -> Result<bool> {
    let mut index = body_open + 1;
    let mut block_depth = 0usize;
    let mut statement_start = true;
    let mut saw_qualified_say = false;
    while index < body_close {
        let global_subdialog = tokens.get(index).is_some_and(|token| token.text == ":")
            && tokens.get(index + 1).is_some_and(|token| token.text == ":")
            && tokens
                .get(index + 2)
                .is_some_and(|token| token.text == "Subdialog")
            && tokens.get(index + 3).is_some_and(|token| token.text == "(");
        let instance_subdialog = tokens.get(index).is_some_and(|token| token.text == "this")
            && tokens.get(index + 1).is_some_and(|token| token.text == ".")
            && tokens
                .get(index + 2)
                .is_some_and(|token| token.text == "Subdialog")
            && tokens.get(index + 3).is_some_and(|token| token.text == "(");
        if global_subdialog || instance_subdialog {
            let open = index + 3;
            let close = matching_close(tokens, open, "(", ")")?;
            if close >= body_close || tokens.get(close + 1).is_none_or(|token| token.text != ";") {
                return Ok(false);
            }
            return Ok(!saw_qualified_say);
        }

        let direct_global_say = block_depth == 0
            && statement_start
            && tokens.get(index).is_some_and(|token| token.text == ":")
            && tokens.get(index + 1).is_some_and(|token| token.text == ":")
            && tokens
                .get(index + 2)
                .is_some_and(|token| token.text == "Say")
            && tokens.get(index + 3).is_some_and(|token| token.text == "(");
        if direct_global_say {
            let close = matching_close(tokens, index + 3, "(", ")")?;
            if close < body_close && tokens.get(close + 1).is_some_and(|token| token.text == ";") {
                saw_qualified_say = true;
                index = close + 2;
                statement_start = true;
                continue;
            }
        }

        match tokens[index].text.as_str() {
            "{" => {
                block_depth += 1;
                statement_start = true;
            }
            "}" => {
                block_depth = block_depth.saturating_sub(1);
                statement_start = block_depth == 0;
            }
            ";" if block_depth == 0 => statement_start = true,
            _ if block_depth == 0 => statement_start = false,
            _ => {}
        }
        index += 1;
    }
    Ok(false)
}

/// Refuse the saturated structural shape that failed the current live runtime oracle.
///
/// A checked-out call that is already full may still have its topics' methods, defaults and ranks
/// edited; changing or removing its 20-slot structure is refused. A smaller call may still grow to
/// the proven Stage-A 20-sibling shape, but newly declared children must occur in source declaration
/// order. Stage A also proves replacing the smaller call's old children entirely; old children that
/// are retained may not be duplicated or relatively reordered, and every authored child must be
/// either pristine or newly declared. Visible ordering remains freely controllable through the
/// separately runtime-qualified `PriorityRank` path. This admits Stage A/B and ordinary insertion
/// while rejecting the reversed, replaced Stage-C call which compiled cleanly but failed in game
/// with `ArrayNum exceeds ArrayMax`.
fn validate_saturated_subdialog_edits(
    pristine: &str,
    authored: &str,
    added_classes: &[String],
) -> Result<()> {
    let pristine_calls = fixed_subdialog_calls(pristine)?;
    let authored_calls = fixed_subdialog_calls(authored)?;
    let added = added_classes
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let outline = dialog::read_outline(authored)
        .map_err(|reason| anyhow::anyhow!("inventorying authored class order: {reason}"))?;
    let declaration_order = outline
        .classes
        .iter()
        .map(|class| class.name.as_str())
        .filter(|class| added.contains(class))
        .collect::<Vec<_>>();

    let owners = pristine_calls
        .keys()
        .chain(authored_calls.keys())
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    for owner in owners {
        let pristine = pristine_calls.get(owner).map(Vec::as_slice).unwrap_or(&[]);
        let edited = authored_calls.get(owner).map(Vec::as_slice).unwrap_or(&[]);

        if pristine.iter().any(|call| call.iter().all(Option::is_some)) {
            if pristine != edited {
                bail!(
                    "{owner} structurally changes or removes an already full 20-child Subdialog. That saturated reshape failed the current live runtime oracle; edit child defaults/methods or PriorityRank instead"
                );
            }
            continue;
        }

        if pristine == edited || !edited.iter().any(|call| call.iter().all(Option::is_some)) {
            continue;
        }
        let [edited_call] = edited else {
            bail!(
                "{owner} has a structurally changed saturated Subdialog plus {} source-level calls; exactly one call is required",
                edited.len()
            );
        };
        let [pristine_call] = pristine else {
            bail!(
                "{owner} changes to a saturated 20-child Subdialog, but its pristine source has {} calls; exactly one base call is required",
                pristine.len()
            );
        };
        let pristine_order = pristine_call
            .iter()
            .filter_map(Option::as_deref)
            .collect::<Vec<_>>();
        let pristine_children = pristine_order.iter().copied().collect::<BTreeSet<_>>();
        let edited_children = edited_call
            .iter()
            .filter_map(Option::as_deref)
            .collect::<Vec<_>>();

        if let Some(foreign) = edited_children
            .iter()
            .copied()
            .find(|class| !pristine_children.contains(class) && !added.contains(class))
        {
            bail!(
                "{owner} fills a 20-child Subdialog with {foreign}, which is neither a pristine child nor a newly authored class"
            );
        }

        let edited_pristine_order = edited_children
            .iter()
            .copied()
            .filter(|class| pristine_children.contains(class))
            .collect::<Vec<_>>();
        let retained_pristine = edited_pristine_order
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if retained_pristine.len() != edited_pristine_order.len() {
            bail!(
                "{owner} fills a 20-child Subdialog while duplicating a retained pristine child; each retained child may appear only once"
            );
        }
        let expected_pristine_order = pristine_order
            .iter()
            .copied()
            .filter(|class| retained_pristine.contains(class))
            .collect::<Vec<_>>();
        if edited_pristine_order != expected_pristine_order {
            bail!(
                "{owner} fills a 20-child Subdialog while reordering retained pristine children; keep their original relative order or replace them with newly authored topics"
            );
        }

        let edited_new_order = edited_call
            .iter()
            .filter_map(Option::as_deref)
            .filter(|class| added.contains(class))
            .collect::<Vec<_>>();
        let referenced = edited_new_order.iter().copied().collect::<BTreeSet<_>>();
        let expected_new_order = declaration_order
            .iter()
            .copied()
            .filter(|class| referenced.contains(class))
            .collect::<Vec<_>>();
        if edited_new_order != expected_new_order {
            bail!(
                "{owner} fills a 20-child Subdialog while permuting newly authored topics. The runtime-qualified saturated path keeps new children in declaration order; use PriorityRank for visible ordering or keep the call below 20 children"
            );
        }
    }

    Ok(())
}

/// Add one class argument to the one fixed-width `Subdialog` call in an existing topic class.
///
/// The source call is bound back to the graph's exact current child order before any rewrite.
/// Existing children are shifted, never dropped. With no explicit position, a trailing topic whose
/// caption is the language-independent `TEXT_BACK` key remains last; otherwise the child appends.
/// This is intentionally narrower than a general source rewriter: ambiguity is a refusal, and the
/// final `dialog check` independently verifies that no shipped declaration/default target was lost.
fn wire_subdialog(
    source: &str,
    parent: &str,
    child: &str,
    expected_children: &[String],
    trailing_back: Option<&str>,
    requested_position: Option<usize>,
) -> Result<String> {
    let tokens = code_tokens(source)?;
    let mut class_bodies = Vec::new();
    for index in 0..tokens.len().saturating_sub(1) {
        if tokens[index].text != "class" || tokens[index + 1].text != parent {
            continue;
        }
        let Some(open) = ((index + 2)..tokens.len())
            .find(|candidate| matches!(tokens[*candidate].text.as_str(), "{" | ";"))
        else {
            bail!("class {parent} has no body");
        };
        if tokens[open].text == ";" {
            continue;
        }
        class_bodies.push((open, matching_close(&tokens, open, "{", "}")?));
    }
    let [(class_open, class_close)] = class_bodies.as_slice() else {
        bail!(
            "class {parent} has {} source bodies; exactly one is required",
            class_bodies.len()
        );
    };

    let calls = ((*class_open + 1)..*class_close)
        .filter(|index| {
            tokens[*index].text == "Subdialog"
                && tokens
                    .get(*index + 1)
                    .is_some_and(|token| token.text == "(")
        })
        .collect::<Vec<_>>();
    let [call] = calls.as_slice() else {
        bail!(
            "{parent} contains {} source-level Subdialog calls; exactly one is required",
            calls.len()
        );
    };
    let open = *call + 1;
    let close = matching_close(&tokens, open, "(", ")")?;
    if close >= *class_close {
        bail!("the Subdialog call in {parent} leaves its class body");
    }

    let call_shape = subdialog_call_shape(&tokens, *call, *class_open)
        .with_context(|| format!("binding the Subdialog call in {parent}"))?;
    let arguments = call_argument_ranges(&tokens, open, close)?;
    let slot_ranges = match (call_shape, arguments.as_slice()) {
        // Decompiled global calls carry the owning topic as their first argument.
        (SubdialogCallShape::Global, [owner, slots @ ..])
            if arguments.len() == SUBDIALOG_TOPIC_SLOTS + 1 =>
        {
            let (start, end) = *owner;
            if start != end || tokens[start].text != "this" {
                bail!(
                    "the Subdialog call in {parent} has 21 arguments but its first argument is not `this`"
                );
            }
            slots
        }
        (SubdialogCallShape::Instance, slots)
            if arguments.len() == SUBDIALOG_TOPIC_SLOTS =>
        {
            slots
        }
        (SubdialogCallShape::Global, _) => bail!(
            "the global Subdialog call in {parent} must have `this` followed by exactly {SUBDIALOG_TOPIC_SLOTS} topic slots; found {} arguments",
            arguments.len()
        ),
        (SubdialogCallShape::Instance, _) => bail!(
            "the instance Subdialog call in {parent} must have exactly {SUBDIALOG_TOPIC_SLOTS} topic slots; found {} arguments",
            arguments.len()
        ),
    };

    let mut populated = Vec::<(String, String)>::new();
    let mut saw_empty = false;
    for range in slot_ranges {
        if is_empty_subdialog_slot(&tokens, *range) {
            saw_empty = true;
            continue;
        }
        if saw_empty {
            bail!(
                "the Subdialog call in {parent} has a populated topic after an empty slot; refusing to guess its runtime order"
            );
        }
        let Some(name) = subdialog_class_name(&tokens, *range) else {
            bail!(
                "the Subdialog call in {parent} contains a populated slot that is not a bare or namespace-qualified topic class"
            );
        };
        let raw = source[tokens[range.0].start..tokens[range.1].end].to_owned();
        populated.push((name.to_owned(), raw));
    }

    let source_children = populated
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>();
    let graph_children = expected_children
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if source_children != graph_children {
        bail!(
            "the source-level Subdialog children in {parent} do not match the checked cache graph; refusing a stale or ambiguous rewrite"
        );
    }
    if populated.len() == SUBDIALOG_TOPIC_SLOTS {
        bail!("the Subdialog call in {parent} has no empty topic slot");
    }

    let insert_at = match requested_position {
        Some(position) if position == 0 || position > populated.len() + 1 => bail!(
            "--subdialog-position for {parent} must be between 1 and {}, inclusive; got {position}",
            populated.len() + 1
        ),
        Some(position) => position - 1,
        None => match trailing_back {
            Some(back) => {
                if populated.last().is_none_or(|(name, _)| name != back) {
                    bail!(
                        "the checked cache identifies {back} as the trailing Back option in {parent}, but the source call does not end with it"
                    );
                }
                populated.len() - 1
            }
            None => populated.len(),
        },
    };

    let mut rewritten = populated
        .into_iter()
        .map(|(_, raw)| raw)
        .collect::<Vec<_>>();
    rewritten.insert(insert_at, child.to_owned());
    rewritten.resize(SUBDIALOG_TOPIC_SLOTS, EMPTY_SUBDIALOG_SLOT.to_owned());

    let mut edited = source.to_owned();
    for (range, replacement) in slot_ranges.iter().zip(rewritten.iter()).rev() {
        edited.replace_range(tokens[range.0].start..tokens[range.1].end, replacement);
    }
    Ok(edited)
}

fn new_conversation(request: NewConversationRequest) -> Result<()> {
    let participant_input = request.npc.trim();
    let (_new_module, _new_relative_path) = new_conversation_module_names(participant_input)?;
    let (cache_path, bytes) = read_cache(request.cache, request.game)?;
    let graph = dialog::build(&bytes).context("reading dialog from the script cache")?;
    let selected = exact_npc_matches(&graph, participant_input);
    match selected.as_slice() {
        [] => {}
        [conversation] if conversation.root_class.is_none() && conversation.topics.is_empty() => {}
        [conversation] => bail!(
            "{} already has a conversation with topics; use `gore dialog new-topic` to add an option",
            participant_label(conversation)
        ),
        conversations => {
            let names = conversations
                .iter()
                .map(|conversation| participant_label(conversation))
                .collect::<Vec<_>>();
            bail!(
                "{participant_input:?} matched {} conversations: {}. Name the NPC exactly",
                conversations.len(),
                names.join(", ")
            )
        }
    };
    let anchor = resolve_conversation_settings_anchor(&bytes, participant_input)?;
    let participant = anchor.participant.clone();

    let slug = identifier(&request.mod_name);
    if slug.is_empty() {
        bail!("--mod-name has to contain at least one letter or digit");
    }
    let class = request
        .class
        .clone()
        .unwrap_or_else(|| format!("UChoice{slug}"));
    if !class.starts_with('U') {
        bail!("an AngelScript topic class name has to start with `U`, unlike {class:?}");
    }
    if !is_angelscript_identifier(&class) {
        bail!("{class:?} is not an AngelScript identifier");
    }

    let declared = declared_classes(&bytes)?;
    let root_class = expected_topic_root(&participant);
    for generated in [&class, &root_class] {
        if declared.contains(&generated.to_ascii_lowercase()) {
            bail!(
                "the cache already declares a class called {generated:?}; choose another NPC or --class"
            );
        }
    }

    let caption_line =
        new_conversation_caption_line(request.caption.as_deref(), request.caption_key.as_deref())?;
    let priority_rank = request.priority_rank;

    let taken = dialog::checkout(&bytes, &anchor.module, native_api(&cache_path))
        .with_context(|| format!("taking {} out of the cache", anchor.module))?;
    let outline = dialog::read_outline(&taken.source).map_err(|reason| {
        anyhow::anyhow!("inventorying the conversation-settings anchor: {reason}")
    })?;
    validate_settings_anchor_source(&outline, &participant)
        .with_context(|| format!("validating {}", anchor.module))?;
    let operation = DialogModuleOperation::Edit;
    let module = taken.module;
    let relative_path = taken.relative_path;
    let pristine = taken.source;
    let default_classes = taken.default_classes;
    let unsupported_generated_methods = taken.unsupported_generated_methods;
    let debug_id = generated_topic_debug_id(&module, &class, &pristine);
    let getter = format!("Get{}", identifier(&participant));
    let root_source = format!(
        "class {root_class} : UG1RDialogTopic\n\
         {{\n\
         \x20   default ForCharacter = n{};\n\
         \x20   default WithCharacter = n\"Hero\";\n\
         \n\
         \x20   {root_class}()\n\
         \x20   {{\n\
         \x20       super();\n\
         \x20       return;\n\
         \x20   }}\n\
         \x20   AGothicCharacterState GetHero() const\n\
         \x20   {{\n\
         \x20       return this.GetCharacter(n\"Hero\");\n\
         \x20   }}\n\
         \x20   AGothicCharacterState {getter}() const\n\
         \x20   {{\n\
         \x20       return this.GetSelf();\n\
         \x20   }}\n\
         }}",
        serde_json::to_string(&participant)?
    );
    let topic_source = format!(
        "class {class} : {root_class}\n\
         {{\n\
         \x20   default DebugId = {debug_id};\n\
         {caption_line}\n\
         \x20   default PriorityRank = {priority_rank};\n\
         \n\
         \x20   UFUNCTION(BlueprintOverride)\n\
         \x20   bool IsVisible() const\n\
         \x20   {{\n\
         \x20       return true;\n\
         \x20   }}\n\
         \n\
         \x20   UFUNCTION(BlueprintOverride)\n\
         \x20   void Act()\n\
         \x20   {{\n\
         \x20       this.EndConversation();\n\
         \x20   }}\n\
         }}"
    );
    let generated = format!(
        "// Generated by `gore dialog new-conversation` for {participant}.\n\
         // Add more direct choices beside the first one. A choice may open a fixed-width\n\
         // Subdialog containing other newly declared choices from this same module; use the\n\
         // global ::Subdialog(this, UChildClass, ...) form for new-to-new edges.\n\
         \n\
         {root_source}\n\n{topic_source}"
    );
    let source = append_conversation_namespace(&pristine, &generated);

    let manifest = EditManifest {
        operation,
        module,
        relative_path,
        source_file: String::new(),
        pristine_file: String::new(),
        participant: participant.clone(),
        cache_sha256: digest_of(&bytes),
        requires_topic_scaffold: true,
        dialog_topics: Vec::new(),
    };
    let known = dialog::known_names(&bytes).context("collecting the cache's names")?;
    let checkout = dialog::Checkout {
        module: manifest.module.clone(),
        relative_path: manifest.relative_path.clone(),
        source: pristine.clone(),
        default_classes,
        unsupported_generated_methods,
    };
    let report = dialog::verify(&checkout, &source, &known);
    if !report.is_carryable() {
        let reasons = report
            .violations
            .iter()
            .map(|violation| violation.explain())
            .collect::<Vec<_>>()
            .join("; ");
        bail!("generated conversation did not pass the dialog edit contract: {reasons}");
    }
    validate_saturated_subdialog_edits(&pristine, &source, &report.added_classes)
        .context("the generated Subdialog shape is not runtime-qualified")?;
    validate_topic_registrations(&manifest, &report, &source, &bytes)
        .context("the generated conversation tree is not safely connected")?;
    if !report.requires_new_symbols() {
        bail!("generated conversation was not recognized as a new-symbol edit");
    }

    ensure_empty_dialog_workspace(&request.out)?;
    fs::create_dir_all(request.out.join("pristine"))
        .with_context(|| format!("creating {}", request.out.display()))?;
    let leaf = manifest
        .relative_path
        .rsplit('/')
        .next()
        .unwrap_or("module.as")
        .to_owned();
    let source_path = request.out.join(&leaf);
    let pristine_path = request.out.join("pristine").join(&leaf);
    fs::write(&source_path, &source)
        .with_context(|| format!("writing {}", source_path.display()))?;
    fs::write(&pristine_path, &pristine)
        .with_context(|| format!("writing {}", pristine_path.display()))?;
    let manifest = EditManifest {
        source_file: leaf.clone(),
        pristine_file: format!("pristine/{leaf}"),
        ..manifest
    };
    let manifest_path = request.out.join(MANIFEST_NAME);
    fs::write(
        &manifest_path,
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )
    .with_context(|| format!("writing {}", manifest_path.display()))?;

    println!("{} — {}", manifest.participant, manifest.module);
    println!("wrote     {}", source_path.display());
    println!("as-was    {}", pristine_path.display());
    println!("manifest  {}", manifest_path.display());
    println!();
    println!("loaded per-NPC settings module retained; private root and first choice added");
    println!("root      {root_class}");
    println!("choice    {class}");
    println!("priority  {priority_rank}");
    println!(
        "stage     --op {} --allow-new-symbols",
        manifest.operation.as_str()
    );
    println!();
    println!("next:");
    println!("  gore dialog check {}", powershell_quote(&request.out));
    println!(
        "  gore dialog stage {} --mod-name {}",
        powershell_quote(&request.out),
        powershell_quote_text(&request.mod_name),
    );
    println!("Compilation, packaging, deployment and runtime proof remain separate steps.");
    Ok(())
}

fn new_topic(request: NewTopicRequest) -> Result<()> {
    if request.subdialog_position.is_some() && request.subdialog_of.is_none() {
        bail!("--subdialog-position requires --subdialog-of");
    }
    if let Some(position) = request.subdialog_position {
        if !(1..=SUBDIALOG_TOPIC_SLOTS).contains(&position) {
            bail!(
                "--subdialog-position must be between 1 and {SUBDIALOG_TOPIC_SLOTS}; got {position}"
            );
        }
    }
    let (cache_path, bytes) = read_cache(request.cache, request.game)?;
    let graph = dialog::build(&bytes).context("reading dialog from the script cache")?;
    let conversation = resolve_one(&graph, &request.npc)?;

    let Some(root_class) = conversation.root_class.clone() else {
        bail!(
            "{} declares no dialog topics yet; use `gore dialog new-conversation` to add its private topic base and first option",
            participant_label(conversation)
        );
    };
    let Some(participant) = conversation.npc_participants().next() else {
        bail!("{} names no NPC participant", conversation.module);
    };

    let slug = identifier(&request.mod_name);
    if slug.is_empty() {
        bail!("--mod-name has to contain at least one letter or digit");
    }
    let class = match &request.class {
        Some(name) => name.clone(),
        None => format!("UChoice{slug}"),
    };
    if !class.starts_with('U') {
        bail!("an AngelScript topic class name has to start with `U`, unlike {class:?}");
    }
    if !is_angelscript_identifier(&class) {
        bail!("{class:?} is not an AngelScript identifier");
    }
    let declared = declared_classes(&bytes)?;
    if declared.contains(&class.to_lowercase()) {
        bail!("the cache already declares a class called {class:?}. Pass a different --class");
    }

    let taken = dialog::checkout(&bytes, &conversation.module, native_api(&cache_path))
        .with_context(|| format!("taking {} out of the cache", conversation.module))?;
    let debug_id = generated_topic_debug_id(&taken.module, &class, &taken.source);

    let caption_line =
        new_topic_caption_line(request.caption.as_deref(), request.caption_key.as_deref())?;

    let is_subdialog = request.subdialog_of.is_some();
    let subtopic_default = if is_subdialog {
        "    default bIsSubTopic = true;\n"
    } else {
        ""
    };
    // Shipped sub-topics use rank 0 and are ordered by their fixed Subdialog argument slots.
    // Giving a newly wired child a root-menu rank can move it past a trailing Back row even when
    // the source call places it correctly.
    let priority_rank = new_topic_priority_rank(conversation, is_subdialog, request.priority_rank)?;
    let topic_source = format!(
        "// Generated by `gore dialog new-topic` for {participant}.\n\
         //\n\
         // This class stays in the conversation's own module and namespace. Keep it there: a\n\
         // separate add-module cannot derive from the module-private topic base. Spoken lines, conditions\n\
         // and effects are yours to add; `gore dialog show <topic>` displays shipped examples.\n\
         \n\
         class {class} : {root_class}\n\
         {{\n\
         \x20   default DebugId = {debug_id};\n\
         {caption_line}\n\
         \x20   default PriorityRank = {priority_rank};\n\
         {subtopic_default}\
         \n\
         \x20   UFUNCTION(BlueprintOverride)\n\
         \x20   bool IsVisible() const\n\
         \x20   {{\n\
         \x20       return true;\n\
         \x20   }}\n\
         \n\
         \x20   UFUNCTION(BlueprintOverride)\n\
         \x20   void Act()\n\
         \x20   {{\n\
         \x20       this.EndConversation();\n\
         \x20   }}\n\
         }}\n"
    );

    let mut source = if let Some(parent_name) = request.subdialog_of.as_deref() {
        let parent = resolve_topic_in(conversation, parent_name)?;
        let subdialogs = parent
            .act
            .iter()
            .filter_map(|step| match &step.kind {
                StepKind::Subdialog { children } => Some(children),
                _ => None,
            })
            .collect::<Vec<_>>();
        let [children] = subdialogs.as_slice() else {
            bail!(
                "{} has {} compiled Subdialog calls; --subdialog-of requires exactly one",
                parent.class,
                subdialogs.len()
            );
        };
        let trailing_back = children.last().and_then(|last| {
            conversation
                .topic(last)
                .and_then(|topic| match &topic.caption {
                    Caption::LocKey { key } if key.eq_ignore_ascii_case(BACK_CAPTION_KEY) => {
                        Some(last.as_str())
                    }
                    _ => None,
                })
        });
        wire_subdialog(
            &taken.source,
            &parent.class,
            &class,
            children,
            trailing_back,
            request.subdialog_position,
        )
        .with_context(|| format!("wiring the new topic into {}", parent.class))?
    } else {
        taken.source.clone()
    };
    source = append_to_class_namespace(&source, &root_class, &topic_source)
        .with_context(|| format!("placing {class} beside its conversation base {root_class}"))?;

    // Same-module roots are discovered natively by the current game. Keep `dialog_topics` empty:
    // that low-level adapter surface remains only for legacy or explicitly hand-authored specs.
    let dialog_topics = Vec::new();

    let known = dialog::known_names(&bytes).context("collecting the cache's names")?;
    let report = dialog::verify(&taken, &source, &known);
    if !report.is_carryable() {
        let reasons = report
            .violations
            .iter()
            .map(|violation| violation.explain())
            .collect::<Vec<_>>()
            .join("; ");
        bail!("generated topic did not pass the dialog edit contract: {reasons}");
    }
    if !report.added_classes.iter().any(|added| added == &class) || !report.requires_new_symbols() {
        bail!("generated topic was not recognized as a same-module new-symbol edit");
    }

    ensure_empty_dialog_workspace(&request.out)?;
    fs::create_dir_all(request.out.join("pristine"))
        .with_context(|| format!("creating {}", request.out.display()))?;
    let leaf = taken
        .relative_path
        .rsplit('/')
        .next()
        .unwrap_or("module.as")
        .to_owned();
    let source_path = request.out.join(&leaf);
    let pristine_path = request.out.join("pristine").join(&leaf);
    fs::write(&source_path, &source)
        .with_context(|| format!("writing {}", source_path.display()))?;
    fs::write(&pristine_path, &taken.source)
        .with_context(|| format!("writing {}", pristine_path.display()))?;
    let manifest = EditManifest {
        operation: DialogModuleOperation::Edit,
        module: taken.module,
        relative_path: taken.relative_path,
        source_file: leaf.clone(),
        pristine_file: format!("pristine/{leaf}"),
        participant: participant_label(conversation),
        cache_sha256: digest_of(&bytes),
        requires_topic_scaffold: false,
        dialog_topics,
    };
    let manifest_path = request.out.join(MANIFEST_NAME);
    fs::write(
        &manifest_path,
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )
    .with_context(|| format!("writing {}", manifest_path.display()))?;

    println!("{} — {}", manifest.participant, manifest.module);
    println!("wrote     {}", source_path.display());
    println!("as-was    {}", pristine_path.display());
    println!("manifest  {}", manifest_path.display());
    println!();
    println!("class     {class} : {root_class}");
    match (request.priority_rank, is_subdialog) {
        (Some(_), _) => println!("priority  {priority_rank} (explicit)"),
        (None, true) => println!(
            "priority  {priority_rank} (subdialog default; equal ranks follow slot position)"
        ),
        (None, false) => println!("priority  {priority_rank} (automatic root rank)"),
    }
    if let Some(parent) = request.subdialog_of.as_deref() {
        println!("subdialog {parent} (same module; no root registration)");
        match request.subdialog_position {
            Some(position) => println!("position  {position} (1-based; existing entries shifted)"),
            None => {
                println!("position  before trailing TEXT_BACK, otherwise after existing entries")
            }
        }
    } else {
        println!("native root (same module; no adapter registration)");
    }
    if request.caption_key.is_some() {
        println!("localization key recorded in source only; add its localized row separately");
    } else {
        println!("caption is an untranslated literal; localized rows remain a separate payload");
    }
    println!();
    println!("next:");
    println!("  gore dialog check {}", powershell_quote(&request.out));
    println!(
        "  gore dialog stage {} --mod-name {}",
        powershell_quote(&request.out),
        powershell_quote_text(&request.mod_name),
    );
    println!("stage uses one same-module edit with allow-new-symbols.");
    println!("It does not compile, package, deploy, launch the game, or prove runtime behavior.");
    Ok(())
}

// ─── export ──────────────────────────────────────────────────────────────────

fn export_file_name(position: usize, module: &str) -> String {
    let stem: String = module
        .chars()
        .take(96)
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-') {
                character
            } else {
                '_'
            }
        })
        .collect();
    format!("{:06}_{stem}.json", position + 1)
}

fn write_export_files(out: &Path, graph: &DialogGraph) -> Result<usize> {
    fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;
    for (position, conversation) in graph.conversations.iter().enumerate() {
        // The ordinal makes names unique even on case-insensitive filesystems and when two module
        // identities sanitize to the same display stem. The complete identity remains in JSON.
        let path = out.join(export_file_name(position, &conversation.module));
        let json = serde_json::to_string_pretty(conversation)?;
        fs::write(&path, json).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(graph.conversations.len())
}

fn export(out: &PathBuf, cache: Option<PathBuf>, game: Option<PathBuf>) -> Result<()> {
    let graph = read_graph(cache, game)?;
    let written = write_export_files(out, &graph)?;
    println!("wrote {} conversation(s) to {}", written, out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_language_family_expands_to_its_columns_newest_first() {
        assert_eq!(columns_for("german"), vec!["german_new", "german"]);
        assert_eq!(
            columns_for("english"),
            vec!["english_newer", "english_new", "english"]
        );
    }

    #[test]
    fn an_exact_column_is_used_as_given() {
        assert_eq!(columns_for("polish"), vec!["polish"]);
    }

    #[test]
    fn tree_depth_counts_submenus_instead_of_visual_indentation() {
        assert!(!subdialog_within_limit(0, Some(0)));
        assert!(subdialog_within_limit(0, Some(1)));
        assert!(!subdialog_within_limit(1, Some(1)));
        assert!(subdialog_within_limit(1, Some(2)));
        assert!(subdialog_within_limit(usize::MAX, None));
    }

    #[test]
    fn localized_names_never_fall_back_to_another_language() {
        let names = vec![
            Name {
                language: "german".to_owned(),
                text: "Deutsch".to_owned(),
            },
            Name {
                language: "english".to_owned(),
                text: "Old".to_owned(),
            },
            Name {
                language: "english_new".to_owned(),
                text: "New".to_owned(),
            },
        ];

        assert_eq!(
            name_in_columns(&names, &columns_for("english")).map(|name| name.text.as_str()),
            Some("New")
        );
        assert!(name_in_columns(&names, &columns_for("polish")).is_none());
        assert!(name_in_columns(&names, &["english_newer".to_owned()]).is_none());
    }

    fn empty_conversation(module: &str, participant: &str) -> Conversation {
        Conversation {
            module: module.to_owned(),
            root_class: None,
            participants: vec!["Hero".to_owned(), participant.to_owned()],
            topics: Vec::new(),
            roots: Vec::new(),
            coverage: Default::default(),
        }
    }

    #[test]
    fn list_filter_keeps_all_substring_matches_while_single_resolution_prefers_exact() {
        let graph = DialogGraph {
            conversations: vec![
                empty_conversation("Story.Exact", "DIEGO"),
                empty_conversation("Story.Guard", "DIEGO_GUARD"),
                empty_conversation("Story.Diego.Extra", "OTHER"),
            ],
        };

        assert_eq!(containing(&graph, "DiEgO").len(), 3);
        assert_eq!(matching(&graph, "DiEgO").len(), 1);
        assert_eq!(matching(&graph, "DiEgO")[0].module, "Story.Exact");
    }

    #[test]
    fn export_names_cannot_overwrite_sanitization_or_case_collisions() {
        let graph = DialogGraph {
            conversations: vec![
                empty_conversation("Story.A_B", "ONE"),
                empty_conversation("Story_A.B", "TWO"),
                empty_conversation("story.a_b", "THREE"),
            ],
        };
        let temp = tempfile::tempdir().unwrap();

        assert_eq!(write_export_files(temp.path(), &graph).unwrap(), 3);
        let mut modules = Vec::new();
        let mut names = Vec::new();
        for entry in fs::read_dir(temp.path()).unwrap() {
            let entry = entry.unwrap();
            names.push(entry.file_name().to_string_lossy().to_lowercase());
            let value: serde_json::Value =
                serde_json::from_slice(&fs::read(entry.path()).unwrap()).unwrap();
            modules.push(value["module"].as_str().unwrap().to_owned());
        }
        names.sort();
        names.dedup();
        modules.sort();

        assert_eq!(names.len(), 3);
        assert!(names.iter().all(|name| {
            name.len() < 120 && !name.contains('/') && !name.contains('\\') && !name.contains(':')
        }));
        assert_eq!(
            modules,
            vec![
                "Story.A_B".to_owned(),
                "Story_A.B".to_owned(),
                "story.a_b".to_owned()
            ]
        );
    }

    fn topic(class: &str, caption: &str, act: Vec<StepKind>) -> gore_as::cache::dialog::Topic {
        gore_as::cache::dialog::Topic {
            class: class.to_owned(),
            super_class: None,
            caption: Caption::LocKey {
                key: caption.to_owned(),
            },
            priority: None,
            flags: Default::default(),
            rules: Vec::new(),
            settings: Vec::new(),
            visibility: Visibility::Always,
            act: act
                .into_iter()
                .map(|kind| Step {
                    guard: Default::default(),
                    kind,
                })
                .collect(),
        }
    }

    fn say(key: &str) -> StepKind {
        StepKind::Say {
            speaker: Some("Hero".to_owned()),
            loc_key: Some(key.to_owned()),
            expression: None,
        }
    }

    #[test]
    fn text_keys_follow_the_tree_and_repeat_nothing() {
        let mut root = topic(
            "URootTopic",
            "CAP_ROOT",
            vec![
                say("LINE_ONE"),
                StepKind::Subdialog {
                    children: vec!["UChild".to_owned()],
                },
                say("LINE_ONE"),
            ],
        );
        root.rules.push(Rule {
            kind: RuleKind::RequireCharacterHasListenedTo,
            args: vec![
                Arg::Name {
                    value: "Hero".to_owned(),
                },
                Arg::Text {
                    value: "RULE_ONLY_LINE".to_owned(),
                },
            ],
        });
        root.rules.push(Rule {
            kind: RuleKind::Other {
                name: "CustomStringRule".to_owned(),
            },
            args: vec![Arg::Text {
                value: "NOT_A_LOCALIZATION_KEY".to_owned(),
            }],
        });
        let conversation = Conversation {
            module: "M".to_owned(),
            root_class: Some("URoot".to_owned()),
            participants: vec!["Hero".to_owned(), "NPC".to_owned()],
            topics: vec![
                root,
                topic("UChild", "CAP_CHILD", vec![say("LINE_TWO")]),
                topic("UOrphan", "CAP_ORPHAN", vec![]),
            ],
            roots: vec!["URootTopic".to_owned()],
            coverage: Default::default(),
        };

        assert_eq!(
            ordered_keys(&conversation),
            vec![
                "CAP_ROOT".to_owned(),
                "RULE_ONLY_LINE".to_owned(),
                "LINE_ONE".to_owned(),
                "CAP_CHILD".to_owned(),
                "LINE_TWO".to_owned(),
                "CAP_ORPHAN".to_owned(),
            ],
            "a sub-menu's text follows the line that opens it, and nothing appears twice"
        );
    }

    #[test]
    fn a_sub_menu_cycle_does_not_recurse_forever() {
        let conversation = Conversation {
            module: "M".to_owned(),
            root_class: Some("URoot".to_owned()),
            participants: vec!["Hero".to_owned()],
            topics: vec![
                topic(
                    "UA",
                    "CAP_A",
                    vec![StepKind::Subdialog {
                        children: vec!["UB".to_owned()],
                    }],
                ),
                topic(
                    "UB",
                    "CAP_B",
                    vec![StepKind::Subdialog {
                        children: vec!["UA".to_owned()],
                    }],
                ),
            ],
            roots: vec!["UA".to_owned()],
            coverage: Default::default(),
        };

        assert_eq!(
            ordered_keys(&conversation),
            vec!["CAP_A".to_owned(), "CAP_B".to_owned()]
        );
    }

    #[test]
    fn an_identifier_keeps_only_what_angelscript_accepts() {
        assert_eq!(identifier("My Dialog Mod"), "MyDialogMod");
        assert_eq!(identifier("viper-work_2"), "ViperWork2");
        assert_eq!(
            identifier("für"),
            "FR",
            "non-ascii is dropped, not transliterated"
        );
        assert_eq!(identifier("---"), "");
    }

    #[test]
    fn a_root_scaffold_stays_before_the_closing_rank_without_implicitly_forcing() {
        let ranked = |class: &str, caption: &str, priority: Option<i64>| {
            let mut topic = topic(class, caption, Vec::new());
            topic.priority = priority;
            topic
        };
        let conversation = |topics: Vec<Topic>, roots: Vec<&str>| Conversation {
            module: "M".to_owned(),
            root_class: Some("URoot".to_owned()),
            participants: vec!["Hero".to_owned(), "NPC".to_owned()],
            topics,
            roots: roots.into_iter().map(str::to_owned).collect(),
            coverage: Default::default(),
        };

        let end_at_five = conversation(
            vec![
                ranked("UQuestion", "QUESTION", Some(1)),
                ranked("UEnd", END_CAPTION_KEY, Some(5)),
            ],
            vec!["UQuestion", "UEnd"],
        );
        assert_eq!(automatic_root_priority_rank(&end_at_five).unwrap(), 4);

        let custom_trailing_row = conversation(
            vec![ranked("UCustomExit", "CUSTOM_EXIT", Some(3))],
            vec!["UCustomExit"],
        );
        assert_eq!(
            automatic_root_priority_rank(&custom_trailing_row).unwrap(),
            2
        );

        let negative_end = conversation(
            vec![ranked("UNegativeEnd", END_CAPTION_KEY, Some(-100))],
            vec!["UNegativeEnd"],
        );
        assert_eq!(automatic_root_priority_rank(&negative_end).unwrap(), -101);

        let engine_default_end =
            conversation(vec![ranked("UEnd", END_CAPTION_KEY, None)], vec!["UEnd"]);
        assert_eq!(
            automatic_root_priority_rank(&engine_default_end).unwrap(),
            -2,
            "automatic selection must skip rank -1's forced-topic semantics"
        );
        assert_eq!(
            new_topic_priority_rank(&end_at_five, true, None).unwrap(),
            DEFAULT_SUBDIALOG_PRIORITY_RANK
        );
        assert_eq!(
            new_topic_priority_rank(&end_at_five, false, Some(-1)).unwrap(),
            -1,
            "an explicit expert override remains exact"
        );
    }

    #[test]
    fn new_topic_plain_caption_is_an_inline_default_expression() {
        let request = NewTopicRequest {
            npc: "NPC".to_owned(),
            caption: Some("Sag \"Los\\geht's\"\nJetzt".to_owned()),
            caption_key: None,
            class: None,
            subdialog_of: None,
            subdialog_position: None,
            priority_rank: None,
            mod_name: "Test".to_owned(),
            out: PathBuf::new(),
            cache: None,
            game: None,
        };

        assert_eq!(
            new_topic_caption_line(request.caption.as_deref(), request.caption_key.as_deref())
                .unwrap(),
            r#"    default Caption = FText::FromString(n"Sag \"Los\\geht's\"\nJetzt".ToString());"#
        );
    }

    #[test]
    fn new_conversation_plain_caption_is_an_inline_default_expression() {
        let request = NewConversationRequest {
            npc: "NEW_NPC".to_owned(),
            caption: Some("Neue Unterhaltung".to_owned()),
            caption_key: None,
            class: None,
            priority_rank: DEFAULT_ROOT_PRIORITY_RANK,
            mod_name: "Test".to_owned(),
            out: PathBuf::new(),
            cache: None,
            game: None,
        };

        assert_eq!(
            new_conversation_caption_line(
                request.caption.as_deref(),
                request.caption_key.as_deref(),
            )
            .unwrap(),
            r#"    default Caption = FText::FromString(n"Neue Unterhaltung".ToString());"#
        );
    }

    #[test]
    fn a_new_conversation_never_turns_a_partial_or_module_match_into_an_edit() {
        let graph = DialogGraph {
            conversations: vec![Conversation {
                module: "Story.G1R.Conversation.Conversation_NC_SLD_GORN_699FM".to_owned(),
                root_class: None,
                participants: vec!["Hero".to_owned(), "NC_SLD_GORN_699FM".to_owned()],
                topics: Vec::new(),
                roots: Vec::new(),
                coverage: Default::default(),
            }],
        };

        assert_eq!(
            matching(&graph, "GORN").len(),
            1,
            "the read helper is intentionally broad"
        );
        assert!(exact_npc_matches(&graph, "GORN").is_empty());
        assert!(exact_npc_matches(&graph, &graph.conversations[0].module).is_empty());
        assert_eq!(exact_npc_matches(&graph, "nc_sld_gorn_699fm").len(), 1);
    }

    #[test]
    fn only_one_unreal_object_prefix_is_removed() {
        assert_eq!(class_without_object_prefix("UChoice"), "Choice");
        assert_eq!(class_without_object_prefix("UUFoo"), "UFoo");
        assert_eq!(class_without_object_prefix("Choice"), "Choice");
        assert_eq!(
            registered_topic_class(&reflected_topic_path("UUFoo")).unwrap(),
            "UUFoo"
        );
    }

    #[test]
    fn the_exit_option_is_the_registration_sentinel() {
        let conversation = Conversation {
            module: "M".to_owned(),
            root_class: Some("URoot".to_owned()),
            participants: vec!["Hero".to_owned(), "NPC".to_owned()],
            topics: vec![
                topic("UTalk", "CAP_TALK", vec![say("LINE")]),
                topic("UExit", "CAP_EXIT", vec![StepKind::EndConversation]),
            ],
            roots: vec!["UTalk".to_owned(), "UExit".to_owned()],
            coverage: Default::default(),
        };
        assert_eq!(
            sentinel_of(&conversation).map(|topic| topic.class.as_str()),
            Some("UExit")
        );
    }

    #[test]
    fn without_an_exit_option_the_first_root_stands_in() {
        let conversation = Conversation {
            module: "M".to_owned(),
            root_class: Some("URoot".to_owned()),
            participants: vec!["Hero".to_owned(), "NPC".to_owned()],
            topics: vec![topic("UTalk", "CAP_TALK", vec![say("LINE")])],
            roots: vec!["UTalk".to_owned()],
            coverage: Default::default(),
        };
        assert_eq!(
            sentinel_of(&conversation).map(|topic| topic.class.as_str()),
            Some("UTalk")
        );
    }

    #[test]
    fn a_conversation_without_topics_has_no_sentinel() {
        let conversation = Conversation {
            module: "M".to_owned(),
            root_class: None,
            participants: vec!["NPC".to_owned()],
            topics: Vec::new(),
            roots: Vec::new(),
            coverage: Default::default(),
        };
        assert!(sentinel_of(&conversation).is_none());
    }

    #[test]
    fn arguments_render_without_their_class_prefix_and_without_padding() {
        let args = vec![
            Arg::Class {
                name: "UTopic_Brannok_136200".to_owned(),
            },
            Arg::Null,
            Arg::Name {
                value: "Hero".to_owned(),
            },
        ];
        assert_eq!(render_args(&args), "Topic_Brannok_136200, n\"Hero\"");
    }

    fn fixed_subdialog_arguments(children: &[&str]) -> String {
        children
            .iter()
            .copied()
            .map(str::to_owned)
            .chain(
                std::iter::repeat(EMPTY_SUBDIALOG_SLOT.to_owned())
                    .take(SUBDIALOG_TOPIC_SLOTS - children.len()),
            )
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn graph_children(children: &[&str]) -> Vec<String> {
        children.iter().map(|child| (*child).to_owned()).collect()
    }

    #[test]
    fn a_subdialog_child_appends_when_there_is_no_trailing_back() {
        let parent_arguments = fixed_subdialog_arguments(&["UFirst"]);
        let other_arguments = fixed_subdialog_arguments(&["UUntouched"]);
        let source = format!(
            r#"
// class UParent {{ Subdialog(UBogus::StaticClass()); }}
class UParent : UBase
{{
    void Act_Implementation()
    {{
        FString Example = "Subdialog(also not code)";
        this.Subdialog({parent_arguments});
    }}
}}

class UOther : UBase
{{
    void Act_Implementation()
    {{
        this.Subdialog({other_arguments});
    }}
}}
"#
        );
        let edited = wire_subdialog(
            &source,
            "UParent",
            "UNewChild",
            &graph_children(&["UFirst"]),
            None,
            None,
        )
        .unwrap();
        assert!(edited.contains("this.Subdialog(UFirst, UNewChild,"));
        assert!(edited.contains(&format!("this.Subdialog({other_arguments});")));
        assert_eq!(edited.matches("UNewChild").count(), 1);
        let calls = fixed_subdialog_calls(&edited).unwrap();
        assert!(calls["UParent"]
            .iter()
            .flatten()
            .flatten()
            .any(|child| child == "UNewChild"));
    }

    #[test]
    fn default_subdialog_placement_keeps_a_trailing_back_last() {
        let arguments = fixed_subdialog_arguments(&["UFirst", "UBack"]);
        let source = format!(
            "class UParent : UBase {{ void Act_Implementation() {{ ::Subdialog(this, {arguments}); }} }}"
        );
        let edited = wire_subdialog(
            &source,
            "UParent",
            "UNewChild",
            &graph_children(&["UFirst", "UBack"]),
            Some("UBack"),
            None,
        )
        .unwrap();
        assert!(edited.contains("::Subdialog(this, UFirst, UNewChild, UBack,"));
    }

    #[test]
    fn an_explicit_one_based_position_can_place_the_child_anywhere() {
        let arguments = fixed_subdialog_arguments(&["UFirst", "USecond", "UBack"]);
        let source = format!(
            "class UParent : UBase {{ void Act_Implementation() {{ this.Subdialog({arguments}); }} }}"
        );
        let children = graph_children(&["UFirst", "USecond", "UBack"]);

        let first = wire_subdialog(
            &source,
            "UParent",
            "UNewChild",
            &children,
            Some("UBack"),
            Some(1),
        )
        .unwrap();
        assert!(first.contains("this.Subdialog(UNewChild, UFirst, USecond, UBack,"));

        let after_back = wire_subdialog(
            &source,
            "UParent",
            "UNewChild",
            &children,
            Some("UBack"),
            Some(4),
        )
        .unwrap();
        assert!(after_back.contains("this.Subdialog(UFirst, USecond, UBack, UNewChild,"));
    }

    #[test]
    fn an_out_of_range_subdialog_position_fails_closed() {
        let arguments = fixed_subdialog_arguments(&["UFirst", "UBack"]);
        let source = format!(
            "class UParent : UBase {{ void Act_Implementation() {{ this.Subdialog({arguments}); }} }}"
        );
        let error = wire_subdialog(
            &source,
            "UParent",
            "UNewChild",
            &graph_children(&["UFirst", "UBack"]),
            Some("UBack"),
            Some(4),
        )
        .unwrap_err();
        assert!(error.to_string().contains("between 1 and 3"), "{error}");
    }

    #[test]
    fn ambiguous_subdialog_wiring_fails_closed() {
        let first = fixed_subdialog_arguments(&["UA"]);
        let second = fixed_subdialog_arguments(&["UB"]);
        let source = format!(
            "class UParent : UBase {{ void Act_Implementation() {{ this.Subdialog({first}); this.Subdialog({second}); }} }}"
        );
        let error = wire_subdialog(
            &source,
            "UParent",
            "UNewChild",
            &graph_children(&["UA"]),
            None,
            None,
        )
        .unwrap_err();
        assert!(error.to_string().contains("exactly one"), "{error}");
    }

    #[test]
    fn wrong_subdialog_receivers_and_free_calls_fail_closed() {
        let arguments = fixed_subdialog_arguments(&["UA"]);
        for invocation in [
            format!("other.Subdialog({arguments})"),
            format!("Subdialog({arguments})"),
            format!("Helpers::Subdialog(this, {arguments})"),
        ] {
            let source = format!(
                "class UParent : UBase {{ void Act_Implementation() {{ {invocation}; }} }}"
            );
            let error = wire_subdialog(
                &source,
                "UParent",
                "UNewChild",
                &graph_children(&["UA"]),
                None,
                None,
            )
            .unwrap_err();
            let message = format!("{error:#}");
            assert!(
                message.contains("unsupported receiver")
                    || message.contains("namespace- or owner-qualified"),
                "{invocation}: {message}"
            );
        }
    }

    #[test]
    fn a_global_subdialog_call_requires_this_as_its_owner() {
        let arguments = fixed_subdialog_arguments(&["UA"]);
        let source = format!(
            "class UParent : UBase {{ void Act_Implementation() {{ ::Subdialog(other, {arguments}); }} }}"
        );
        let error = wire_subdialog(
            &source,
            "UParent",
            "UNewChild",
            &graph_children(&["UA"]),
            None,
            None,
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("first argument is not `this`"),
            "{error}"
        );
    }

    #[test]
    fn a_full_subdialog_call_fails_closed() {
        let names = (0..SUBDIALOG_TOPIC_SLOTS)
            .map(|index| format!("UTopic{index}"))
            .collect::<Vec<_>>();
        let refs = names.iter().map(String::as_str).collect::<Vec<_>>();
        let arguments = fixed_subdialog_arguments(&refs);
        let source = format!(
            "class UParent : UBase {{ void Act_Implementation() {{ this.Subdialog({arguments}); }} }}"
        );
        let error =
            wire_subdialog(&source, "UParent", "UNewChild", &names, None, None).unwrap_err();
        assert!(error.to_string().contains("no empty topic slot"), "{error}");
    }

    fn subdialog_source(arguments: &str, declarations: &[String]) -> String {
        format!(
            "class UParent : UBase {{ void Act_Implementation() {{ this.Subdialog({arguments}); }} }}\n{}",
            declarations
                .iter()
                .map(|name| format!("class {name} : UBase {{}}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    #[test]
    fn a_new_saturated_subdialog_keeps_new_topics_in_declaration_order() {
        let names = (1..=SUBDIALOG_TOPIC_SLOTS)
            .map(|index| format!("UNew{index:02}"))
            .collect::<Vec<_>>();
        let refs = names.iter().map(String::as_str).collect::<Vec<_>>();
        let pristine = subdialog_source(&fixed_subdialog_arguments(&["UExisting"]), &[]);
        let authored = subdialog_source(&fixed_subdialog_arguments(&refs), &names);

        validate_saturated_subdialog_edits(&pristine, &authored, &names).unwrap();
    }

    #[test]
    fn a_new_saturated_subdialog_permutation_fails_closed() {
        let names = (1..=SUBDIALOG_TOPIC_SLOTS)
            .map(|index| format!("UNew{index:02}"))
            .collect::<Vec<_>>();
        let reversed = names.iter().rev().map(String::as_str).collect::<Vec<_>>();
        let pristine = subdialog_source(&fixed_subdialog_arguments(&["UExisting"]), &[]);
        let authored = subdialog_source(&fixed_subdialog_arguments(&reversed), &names);

        let error = validate_saturated_subdialog_edits(&pristine, &authored, &names).unwrap_err();
        assert!(error.to_string().contains("permuting"), "{error}");
        assert!(error.to_string().contains("PriorityRank"), "{error}");
    }

    #[test]
    fn a_saturated_subdialog_cannot_duplicate_a_retained_pristine_child() {
        let names = (1..=SUBDIALOG_TOPIC_SLOTS - 2)
            .map(|index| format!("UNew{index:02}"))
            .collect::<Vec<_>>();
        let mut authored_refs = vec!["UFirst", "UFirst"];
        authored_refs.extend(names.iter().map(String::as_str));
        let pristine = subdialog_source(&fixed_subdialog_arguments(&["UFirst", "USecond"]), &[]);
        let authored = subdialog_source(&fixed_subdialog_arguments(&authored_refs), &names);

        let error = validate_saturated_subdialog_edits(&pristine, &authored, &names).unwrap_err();
        assert!(error.to_string().contains("duplicating"), "{error}");
        assert!(error.to_string().contains("only once"), "{error}");
    }

    #[test]
    fn a_saturated_subdialog_cannot_reorder_retained_pristine_children() {
        let names = (1..=SUBDIALOG_TOPIC_SLOTS - 2)
            .map(|index| format!("UNew{index:02}"))
            .collect::<Vec<_>>();
        let mut authored_refs = vec!["USecond", "UFirst"];
        authored_refs.extend(names.iter().map(String::as_str));
        let pristine = subdialog_source(&fixed_subdialog_arguments(&["UFirst", "USecond"]), &[]);
        let authored = subdialog_source(&fixed_subdialog_arguments(&authored_refs), &names);

        let error = validate_saturated_subdialog_edits(&pristine, &authored, &names).unwrap_err();
        assert!(error.to_string().contains("reordering"), "{error}");
        assert!(error.to_string().contains("relative order"), "{error}");
    }

    #[test]
    fn a_saturated_subdialog_rejects_an_unqualified_child() {
        let names = (1..=SUBDIALOG_TOPIC_SLOTS - 2)
            .map(|index| format!("UNew{index:02}"))
            .collect::<Vec<_>>();
        let mut authored_refs = vec!["UFirst", "UForeign"];
        authored_refs.extend(names.iter().map(String::as_str));
        let pristine = subdialog_source(&fixed_subdialog_arguments(&["UFirst"]), &[]);
        let authored = subdialog_source(&fixed_subdialog_arguments(&authored_refs), &names);

        let error = validate_saturated_subdialog_edits(&pristine, &authored, &names).unwrap_err();
        assert!(error.to_string().contains("UForeign"), "{error}");
        assert!(error.to_string().contains("neither"), "{error}");
    }

    #[test]
    fn changing_an_already_saturated_subdialog_fails_closed() {
        let names = (1..=SUBDIALOG_TOPIC_SLOTS)
            .map(|index| format!("UTopic{index:02}"))
            .collect::<Vec<_>>();
        let pristine_refs = names.iter().map(String::as_str).collect::<Vec<_>>();
        let mut edited_refs = pristine_refs.clone();
        edited_refs.swap(0, 1);
        let pristine = subdialog_source(&fixed_subdialog_arguments(&pristine_refs), &[]);
        let authored = subdialog_source(&fixed_subdialog_arguments(&edited_refs), &[]);

        let error = validate_saturated_subdialog_edits(&pristine, &authored, &[]).unwrap_err();
        assert!(error.to_string().contains("already full"), "{error}");
        assert!(error.to_string().contains("PriorityRank"), "{error}");
    }

    #[test]
    fn shrinking_an_already_saturated_subdialog_fails_closed() {
        let names = (1..=SUBDIALOG_TOPIC_SLOTS)
            .map(|index| format!("UTopic{index:02}"))
            .collect::<Vec<_>>();
        let pristine_refs = names.iter().map(String::as_str).collect::<Vec<_>>();
        let partial_refs = pristine_refs[..SUBDIALOG_TOPIC_SLOTS - 1].to_vec();
        let pristine = subdialog_source(&fixed_subdialog_arguments(&pristine_refs), &[]);
        let authored = subdialog_source(&fixed_subdialog_arguments(&partial_refs), &[]);

        let error = validate_saturated_subdialog_edits(&pristine, &authored, &[]).unwrap_err();
        assert!(error.to_string().contains("already full"), "{error}");
        assert!(error.to_string().contains("removes"), "{error}");
    }

    #[test]
    fn removing_an_already_saturated_subdialog_call_fails_closed() {
        let names = (1..=SUBDIALOG_TOPIC_SLOTS)
            .map(|index| format!("UTopic{index:02}"))
            .collect::<Vec<_>>();
        let pristine_refs = names.iter().map(String::as_str).collect::<Vec<_>>();
        let pristine = subdialog_source(&fixed_subdialog_arguments(&pristine_refs), &[]);
        let authored = "class UParent : UBase { void Act_Implementation() { /* removed */ } }";

        let error = validate_saturated_subdialog_edits(&pristine, authored, &[]).unwrap_err();
        assert!(error.to_string().contains("already full"), "{error}");
        assert!(error.to_string().contains("removes"), "{error}");
    }

    #[test]
    fn ordinary_subdialog_insertion_remains_runtime_qualified() {
        let pristine = subdialog_source(&fixed_subdialog_arguments(&["UFirst", "UBack"]), &[]);
        let authored = subdialog_source(
            &fixed_subdialog_arguments(&["UFirst", "UNew", "UBack"]),
            &["UNew".to_owned()],
        );

        validate_saturated_subdialog_edits(&pristine, &authored, &["UNew".to_owned()]).unwrap();
    }

    #[test]
    fn a_new_topic_is_inserted_inside_the_conversation_namespace() {
        let source = r#"namespace G1R::Conversation
{
class UTopic_Hero__Npc : UConversationTopic
{
}
}
"#;
        let edited = append_to_class_namespace(
            source,
            "UTopic_Hero__Npc",
            "class UChoiceNew : UTopic_Hero__Npc\n{\n}",
        )
        .unwrap();

        let topic = edited.find("class UChoiceNew").unwrap();
        let namespace_close = edited.rfind('}').unwrap();
        assert!(topic < namespace_close);
        assert!(edited[topic..namespace_close].contains("UTopic_Hero__Npc"));
        assert!(!edited[namespace_close + 1..].contains("UChoiceNew"));
    }

    #[test]
    fn namespace_insertion_requires_one_exact_base_class() {
        let duplicate = "namespace A { class UBase {} }\nnamespace B { class UBase {} }\n";
        let error =
            append_to_class_namespace(duplicate, "UBase", "class UNew : UBase {}").unwrap_err();
        assert!(error.to_string().contains("exactly one"), "{error}");
    }

    fn authored_topic(source: &str) -> dialog::ClassOutline {
        dialog::read_outline(source).unwrap().classes.remove(0)
    }

    #[test]
    fn a_new_topic_requires_real_unreal_overrides_and_a_nonzero_debug_id() {
        let class = authored_topic(
            r#"class UNew : UBase
{
    default DebugId = 42;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return true; }
    UFUNCTION(BlueprintOverride)
    void Act() { }
}"#,
        );
        validate_new_topic_source_contract(&class).unwrap();

        let old_spelling = authored_topic(
            r#"class UNew : UBase
{
    default DebugId = 42;
    UFUNCTION()
    bool IsVisible_Implementation() { return true; }
    UFUNCTION()
    void Act_Implementation() { }
}"#,
        );
        let error = validate_new_topic_source_contract(&old_spelling).unwrap_err();
        assert!(error.to_string().contains("BlueprintOverride"), "{error}");

        let mixed_spelling = authored_topic(
            r#"class UNew : UBase
{
    default DebugId = 42;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return true; }
    UFUNCTION(BlueprintOverride)
    void Act() { }
    UFUNCTION()
    void Act_Implementation() { }
}"#,
        );
        let error = validate_new_topic_source_contract(&mixed_spelling).unwrap_err();
        assert!(error.to_string().contains("Act_Implementation"), "{error}");

        let overloaded = authored_topic(
            r#"class UNew : UBase
{
    default DebugId = 42;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return true; }
    UFUNCTION(BlueprintOverride)
    void Act() { }
    void Act(int32 Value) { }
}"#,
        );
        let error = validate_new_topic_source_contract(&overloaded).unwrap_err();
        assert!(error.to_string().contains("overload"), "{error}");

        let zero = authored_topic(
            r#"class UNew : UBase
{
    default DebugId = 0;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return true; }
    UFUNCTION(BlueprintOverride)
    void Act() { }
}"#,
        );
        let error = validate_new_topic_source_contract(&zero).unwrap_err();
        assert!(error.to_string().contains("unset DebugId 0"), "{error}");

        let outside_int64 = authored_topic(
            r#"class UNew : UBase
{
    default DebugId = 9223372036854775808;
    UFUNCTION(BlueprintOverride)
    bool IsVisible() const { return true; }
    UFUNCTION(BlueprintOverride)
    void Act() { }
}"#,
        );
        let error = validate_new_topic_source_contract(&outside_int64).unwrap_err();
        assert!(error.to_string().contains("signed 64-bit"), "{error}");
    }

    #[test]
    fn a_subdialog_topic_requires_the_subtopic_default() {
        let missing = authored_topic("class UNew : UBase { default DebugId = 42; }");
        let error = validate_new_topic_placement(&missing, true).unwrap_err();
        assert!(error.to_string().contains("Subdialog"), "{error}");
        assert!(error.to_string().contains("bIsSubTopic = true"), "{error}");

        let authored = authored_topic(
            "class UNew : UBase { default DebugId = 42; default bIsSubTopic = true; }",
        );
        validate_new_topic_placement(&authored, true).unwrap();
    }

    #[test]
    fn a_root_topic_rejects_the_subtopic_default() {
        let authored = authored_topic(
            "class UNew : UBase { default DebugId = 42; default bIsSubTopic = true; }",
        );
        let error = validate_new_topic_placement(&authored, false).unwrap_err();
        assert!(error.to_string().contains("root"), "{error}");
        assert!(error.to_string().contains("bIsSubTopic = true"), "{error}");

        let absent = authored_topic("class UNew : UBase { default DebugId = 42; }");
        validate_new_topic_placement(&absent, false).unwrap();
    }

    #[test]
    fn indirect_new_topic_inheritance_fails_closed() {
        let outline = dialog::read_outline(
            r#"namespace G1R::Conversation
{
    class URoot : UConversationTopic { }
    class UDirect : URoot { }
    class UIndirect : UDirect { }
}"#,
        )
        .unwrap();
        let root = outline
            .classes
            .iter()
            .find(|class| class.name == "URoot")
            .unwrap();
        let added = vec!["UDirect".to_owned(), "UIndirect".to_owned()];

        let error = direct_added_topic_classes(&outline, &added, root).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("UIndirect"), "{message}");
        assert!(message.contains("derives indirectly"), "{message}");
        assert!(message.contains("UDirect"), "{message}");
    }

    #[test]
    fn inheritance_below_a_shipped_concrete_topic_fails_closed() {
        let outline = dialog::read_outline(
            r#"namespace G1R::Conversation
{
    class URoot : UConversationTopic { }
    class UExistingConcreteTopic : URoot { }
    class UNew : UExistingConcreteTopic { }
}"#,
        )
        .unwrap();
        let root = outline
            .classes
            .iter()
            .find(|class| class.name == "URoot")
            .unwrap();
        let added = vec!["UNew".to_owned()];

        let error = direct_added_topic_classes(&outline, &added, root).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("UNew"), "{message}");
        assert!(message.contains("derives indirectly"), "{message}");
        assert!(message.contains("UExistingConcreteTopic"), "{message}");
    }

    #[test]
    fn unrelated_new_helper_inheritance_stays_allowed() {
        let outline = dialog::read_outline(
            r#"namespace G1R::Conversation
{
    class URoot : UConversationTopic { }
    class UExistingHelper : UObject { }
    class UNewHelper : UExistingHelper { }
}"#,
        )
        .unwrap();
        let root = outline
            .classes
            .iter()
            .find(|class| class.name == "URoot")
            .unwrap();
        let added = vec!["UNewHelper".to_owned()];

        assert!(direct_added_topic_classes(&outline, &added, root)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn a_new_class_inheritance_cycle_fails_closed() {
        let outline = dialog::read_outline(
            r#"namespace G1R::Conversation
{
    class URoot : UConversationTopic { }
    class UFirst : USecond { }
    class USecond : UFirst { }
}"#,
        )
        .unwrap();
        let root = outline
            .classes
            .iter()
            .find(|class| class.name == "URoot")
            .unwrap();
        let added = vec!["UFirst".to_owned(), "USecond".to_owned()];

        let error = direct_added_topic_classes(&outline, &added, root).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("inheritance cycle"), "{message}");
        assert!(message.contains("UFirst"), "{message}");
        assert!(message.contains("USecond"), "{message}");
    }

    fn new_tree_topic(name: &str, debug_id: i64, subtopic: bool, children: &[&str]) -> String {
        let placement = if subtopic {
            "    default bIsSubTopic = true;\n"
        } else {
            ""
        };
        let act = if children.is_empty() {
            "        this.EndConversation();".to_owned()
        } else {
            format!(
                "        ::Subdialog(this, {});",
                fixed_subdialog_arguments(children)
            )
        };
        format!(
            r#"class {name} : URoot
{{
    default DebugId = {debug_id};
{placement}    UFUNCTION(BlueprintOverride)
    bool IsVisible() const {{ return true; }}
    UFUNCTION(BlueprintOverride)
    void Act() {{
{act}
    }}
}}
"#
        )
    }

    fn new_tree_report(classes: &[&str]) -> dialog::EditReport {
        dialog::EditReport {
            violations: Vec::new(),
            changed: Vec::new(),
            changed_defaults: Vec::new(),
            added_classes: classes.iter().map(|class| (*class).to_owned()).collect(),
            added_functions: Vec::new(),
            new_strings: Vec::new(),
            new_static_names: Vec::new(),
            unchanged: false,
        }
    }

    fn check_new_tree(source: &str, classes: &[&str]) -> Result<()> {
        let outline = dialog::read_outline(source).unwrap();
        let root = outline
            .classes
            .iter()
            .find(|class| class.name == "URoot")
            .unwrap();
        let topics = classes
            .iter()
            .map(|class| (*class).to_owned())
            .collect::<BTreeSet<_>>();
        validate_new_topic_tree(
            source,
            &outline,
            &new_tree_report(classes),
            root,
            &topics,
            &BTreeMap::new(),
        )
    }

    fn add_say_before_subdialog(topic: String) -> String {
        topic.replacen(
            "        ::Subdialog",
            "        ::Say(this.GetHero().GetAI(), ::LocText(\"GORE_TEST_LINE\"), GameplayTag::Expression_Neutral, nullptr, false, NAME_None, NAME_None, FGameplayTag::Empty);\n        ::Subdialog",
            1,
        )
    }

    fn add_say_after_subdialog(topic: String) -> String {
        topic.replacen(
            "\n    }\n}\n",
            "\n        ::Say(this.GetHero().GetAI(), ::LocText(\"GORE_TEST_LINE\"), GameplayTag::Expression_Neutral, nullptr, false, NAME_None, NAME_None, FGameplayTag::Empty);\n    }\n}\n",
            1,
        )
    }

    fn move_subdialog_to_helper(topic: String) -> String {
        let call_start = topic.find("        ::Subdialog").unwrap();
        let call_end = call_start + topic[call_start..].find(";\n").unwrap();
        let call = topic[call_start..=call_end].to_owned();
        let mut edited = format!(
            "{}        OpenMenu();{}",
            &topic[..call_start],
            &topic[call_end + 1..]
        );
        let class_close = edited.rfind("\n}\n").unwrap();
        edited.insert_str(
            class_close,
            &format!("\n    void OpenMenu() {{\n{call}\n    }}"),
        );
        edited
    }

    #[test]
    fn an_all_new_multilevel_topic_tree_accepts_an_unconditional_say_before_either_transition() {
        for say_in_root in [false, true] {
            let mut root = new_tree_topic("URootChoice", 1, false, &["ULevelOneA", "ULevelOneB"]);
            let mut level_one = new_tree_topic("ULevelOneA", 2, true, &["ULevelTwo"]);
            if say_in_root {
                root = add_say_before_subdialog(root);
            } else {
                level_one = add_say_before_subdialog(level_one);
            }
            let source = format!(
                "class URoot : UG1RDialogTopic {{}}\n{root}{level_one}{}{}",
                new_tree_topic("ULevelOneB", 3, true, &[]),
                new_tree_topic("ULevelTwo", 4, true, &[]),
            );
            check_new_tree(
                &source,
                &["URootChoice", "ULevelOneA", "ULevelOneB", "ULevelTwo"],
            )
            .unwrap();
        }
    }

    #[test]
    fn only_an_unconditional_top_level_say_separates_nested_menu_transitions() {
        let is_immediate = |body: &str| {
            let source = format!("class UTopic {{ void Act() {{ {body} }} }}");
            immediate_subdialog_acts(&source)
                .unwrap()
                .contains("UTopic")
        };
        let subdialog = "::Subdialog(this, UChild);";

        assert!(is_immediate(&format!("if (true) {{ {subdialog} }}")));
        assert!(is_immediate(&format!("{{}} {subdialog}")));
        assert!(is_immediate(&format!("int Local = 0; {subdialog}")));
        assert!(is_immediate(&format!(
            "if (true) ::Say(this, ::LocText(\"LINE\")); {subdialog}"
        )));
        assert!(!is_immediate(&format!(
            "::Say(this, ::LocText(\"LINE\")); {subdialog}"
        )));
    }

    #[test]
    fn an_all_new_topic_tree_rejects_a_shadowing_free_say_function() {
        let root =
            add_say_before_subdialog(new_tree_topic("URootChoice", 1, false, &["ULevelOne"]));
        let level_one = new_tree_topic("ULevelOne", 2, true, &["ULevelTwo"]);
        let source = format!(
            "void Say() {{}}\nclass URoot : UG1RDialogTopic {{}}\n{root}{level_one}{}",
            new_tree_topic("ULevelTwo", 3, true, &[]),
        );
        let error =
            check_new_tree(&source, &["URootChoice", "ULevelOne", "ULevelTwo"]).unwrap_err();
        assert!(
            error.to_string().contains("free function named `Say`"),
            "{error}"
        );
    }

    #[test]
    fn an_all_new_topic_tree_rejects_helper_indirect_subdialog_edges() {
        let source = format!(
            "class URoot : UG1RDialogTopic {{}}\n{}{}",
            move_subdialog_to_helper(new_tree_topic("URootChoice", 1, false, &["UChild"],)),
            new_tree_topic("UChild", 2, true, &[]),
        );
        let error = check_new_tree(&source, &["URootChoice", "UChild"]).unwrap_err();
        assert!(error.to_string().contains("helper-indirect"), "{error}");
    }

    #[test]
    fn an_all_new_multilevel_topic_tree_rejects_consecutive_actionless_subdialogs() {
        let source = format!(
            "class URoot : UG1RDialogTopic {{}}\n{}{}{}{}",
            new_tree_topic("URootChoice", 1, false, &["ULevelOneA", "ULevelOneB"]),
            new_tree_topic("ULevelOneA", 2, true, &["ULevelTwo"]),
            new_tree_topic("ULevelOneB", 3, true, &[]),
            new_tree_topic("ULevelTwo", 4, true, &[]),
        );
        let error = check_new_tree(
            &source,
            &["URootChoice", "ULevelOneA", "ULevelOneB", "ULevelTwo"],
        )
        .unwrap_err();
        let message = error.to_string();
        assert!(message.contains("URootChoice -> ULevelOneA"), "{message}");
        assert!(message.contains("actionless `Subdialog`"), "{message}");
        assert!(message.contains("`::Say(...)`"), "{message}");

        let action_after_transition = format!(
            "class URoot : UG1RDialogTopic {{}}\n{}{}{}",
            new_tree_topic("URootChoice", 1, false, &["ULevelOne"]),
            add_say_after_subdialog(new_tree_topic("ULevelOne", 2, true, &["ULevelTwo"],)),
            new_tree_topic("ULevelTwo", 3, true, &[]),
        );
        let error = check_new_tree(
            &action_after_transition,
            &["URootChoice", "ULevelOne", "ULevelTwo"],
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("URootChoice -> ULevelOne"),
            "{error}"
        );
    }

    #[test]
    fn an_all_new_parent_requires_the_compiler_qualified_global_subdialog_form() {
        let source = format!(
            "class URoot : UG1RDialogTopic {{}}\n{}{}",
            new_tree_topic("UParent", 1, false, &["UChild"]),
            new_tree_topic("UChild", 2, true, &[]),
        )
        .replace("::Subdialog(this, ", "this.Subdialog(");
        let error = check_new_tree(&source, &["UParent", "UChild"]).unwrap_err();
        assert!(error.to_string().contains("global `::Subdialog"), "{error}");
    }

    #[test]
    fn every_new_topic_is_a_class_beside_its_private_root() {
        let wrong_kind = "class URoot : UG1RDialogTopic {}\nstruct UChoice : URoot {\n    default DebugId = 1;\n    UFUNCTION(BlueprintOverride) bool IsVisible() const { return true; }\n    UFUNCTION(BlueprintOverride) void Act() { this.EndConversation(); }\n}\n";
        let error = check_new_tree(wrong_kind, &["UChoice"]).unwrap_err();
        assert!(error.to_string().contains("declared as a class"), "{error}");

        let wrong_namespace = "namespace RootSpace { class URoot : UG1RDialogTopic {} }\nnamespace OtherSpace { class UChoice : RootSpace::URoot {\n    default DebugId = 1;\n    UFUNCTION(BlueprintOverride) bool IsVisible() const { return true; }\n    UFUNCTION(BlueprintOverride) void Act() { this.EndConversation(); }\n} }\n";
        let outline = dialog::read_outline(wrong_namespace).unwrap();
        let root = outline
            .classes
            .iter()
            .find(|class| class.name == "URoot")
            .unwrap();
        let error = validate_new_topic_tree(
            wrong_namespace,
            &outline,
            &new_tree_report(&["UChoice"]),
            root,
            &BTreeSet::from(["UChoice".to_owned()]),
            &BTreeMap::new(),
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("beside its private root"),
            "{error}"
        );
    }

    #[test]
    fn all_new_topic_trees_reject_duplicates_holes_and_foreign_children() {
        let duplicate = format!(
            "class URoot : UG1RDialogTopic {{}}\n{}{}",
            new_tree_topic("UA", 1, false, &["UB", "UB"]),
            new_tree_topic("UB", 2, true, &[]),
        );
        let error = check_new_tree(&duplicate, &["UA", "UB"]).unwrap_err();
        assert!(error.to_string().contains("duplicates topic UB"), "{error}");

        let mut hole_slots = vec![
            "UB".to_owned(),
            EMPTY_SUBDIALOG_SLOT.to_owned(),
            "UC".to_owned(),
        ];
        hole_slots.resize(SUBDIALOG_TOPIC_SLOTS, EMPTY_SUBDIALOG_SLOT.to_owned());
        let hole_parent = new_tree_topic("UA", 1, false, &[]).replace(
            "this.EndConversation();",
            &format!("::Subdialog(this, {});", hole_slots.join(", ")),
        );
        let holes = format!(
            "class URoot : UG1RDialogTopic {{}}\n{hole_parent}{}{}",
            new_tree_topic("UB", 2, true, &[]),
            new_tree_topic("UC", 3, true, &[]),
        );
        let error = check_new_tree(&holes, &["UA", "UB", "UC"]).unwrap_err();
        assert!(error.to_string().contains("after an empty slot"), "{error}");

        let foreign = format!(
            "class URoot : UG1RDialogTopic {{}}\n{}",
            new_tree_topic("UA", 1, false, &["UShippedElsewhere"]),
        );
        let error = check_new_tree(&foreign, &["UA"]).unwrap_err();
        assert!(error.to_string().contains("foreign"), "{error}");
    }

    #[test]
    fn all_new_topic_trees_reject_multi_parent_cycles_orphans_and_debug_id_reuse() {
        let multi_parent = format!(
            "class URoot : UG1RDialogTopic {{}}\n{}{}{}",
            new_tree_topic("UA", 1, false, &["UC"]),
            new_tree_topic("UB", 2, false, &["UC"]),
            new_tree_topic("UC", 3, true, &[]),
        );
        let error = check_new_tree(&multi_parent, &["UA", "UB", "UC"]).unwrap_err();
        assert!(
            error.to_string().contains("multiple Subdialog parents"),
            "{error}"
        );

        let cycle = format!(
            "class URoot : UG1RDialogTopic {{}}\n{}{}",
            add_say_before_subdialog(new_tree_topic("UA", 1, true, &["UB"])),
            new_tree_topic("UB", 2, true, &["UA"]),
        );
        let error = check_new_tree(&cycle, &["UA", "UB"]).unwrap_err();
        assert!(error.to_string().contains("cycle"), "{error}");

        let orphan = format!(
            "class URoot : UG1RDialogTopic {{}}\n{}{}",
            new_tree_topic("UA", 1, false, &[]),
            new_tree_topic("UOrphan", 2, true, &[]),
        );
        let error = check_new_tree(&orphan, &["UA", "UOrphan"]).unwrap_err();
        assert!(error.to_string().contains("wired as a root"), "{error}");

        let duplicate_id = format!(
            "class URoot : UG1RDialogTopic {{}}\n{}{}",
            new_tree_topic("UA", 1, false, &[]),
            new_tree_topic("UB", 1, false, &[]),
        );
        let error = check_new_tree(&duplicate_id, &["UA", "UB"]).unwrap_err();
        assert!(error.to_string().contains("duplicate DebugId"), "{error}");
    }

    #[test]
    fn a_new_conversation_base_binds_settings_root_and_participant() {
        let source = r#"
namespace G1R::Conversation
{
class UConversationCharacterSettings_G1R_NEW_NPC : UConversationCharacterSettings
{
    default ForCharacter = n"NEW_NPC";
}
class UTopic_Hero__NEW_NPC : UG1RDialogTopic
{
    default ForCharacter = n"NEW_NPC";
    default WithCharacter = n"Hero";
}
class UFirst : UTopic_Hero__NEW_NPC { }
}
"#;
        let outline = dialog::read_outline(source).unwrap();
        let report = new_tree_report(&[
            "UConversationCharacterSettings_G1R_NEW_NPC",
            "UTopic_Hero__NEW_NPC",
            "UFirst",
        ]);
        assert_eq!(
            validate_new_conversation_base(&outline, &report, "NEW_NPC", true)
                .unwrap()
                .name,
            "UTopic_Hero__NEW_NPC"
        );

        let spaced = source.replace("n\"NEW_NPC\"", "n\"N E W _ N P C\"");
        let spaced_outline = dialog::read_outline(&spaced).unwrap();
        let settings = spaced_outline
            .classes
            .iter()
            .find(|class| class.name == "UConversationCharacterSettings_G1R_NEW_NPC")
            .unwrap();
        assert_eq!(
            literal_name_default(settings, "ForCharacter").unwrap(),
            Some("N E W _ N P C".to_owned()),
            "whitespace inside the literal is content, not formatting"
        );
        assert!(validate_new_conversation_base(&spaced_outline, &report, "NEW_NPC", true).is_err());

        let wrong_namespace = source.replace("G1R::Conversation", "Other::Conversation");
        let wrong_outline = dialog::read_outline(&wrong_namespace).unwrap();
        let error =
            validate_new_conversation_base(&wrong_outline, &report, "NEW_NPC", true).unwrap_err();
        assert!(
            error.to_string().contains("namespace `G1R::Conversation`"),
            "{error}"
        );
    }

    #[test]
    fn a_topicless_edit_retains_one_bound_shipped_settings_class() {
        let source = r#"
namespace G1R::Conversation
{
class UConversationCharacterSettings_G1R_NEW_NPC : UConversationCharacterSettings
{
    default ForCharacter = n"NEW_NPC";
}
}
"#;
        let outline = dialog::read_outline(source).unwrap();
        let report = new_tree_report(&[]);
        assert_eq!(
            validate_topicless_settings(&outline, &report, "NEW_NPC")
                .unwrap()
                .name,
            "UConversationCharacterSettings_G1R_NEW_NPC"
        );

        let rebound = source.replace("n\"NEW_NPC\"", "n\"OTHER_NPC\"");
        let rebound_outline = dialog::read_outline(&rebound).unwrap();
        assert!(validate_topicless_settings(&rebound_outline, &report, "NEW_NPC").is_err());

        let duplicate = source.replace(
            "}\n}",
            "}\nclass UExtraSettings : UConversationCharacterSettings { default ForCharacter = n\"NEW_NPC\"; }\n}",
        );
        let duplicate_outline = dialog::read_outline(&duplicate).unwrap();
        assert!(validate_topicless_settings(&duplicate_outline, &report, "NEW_NPC").is_err());
    }

    #[test]
    fn a_topicless_story_module_cannot_be_turned_into_a_first_conversation() {
        let outline = dialog::read_outline(
            "class USettings : UConversationCharacterSettings {}\n\
             class UTopic_Hero__NEW_NPC : UG1RDialogTopic {}\n",
        )
        .unwrap();
        let conversation = Conversation {
            module: "Story.G1R.Conversation.Conversation_NEW_NPC".to_owned(),
            root_class: None,
            participants: vec!["Hero".to_owned(), "NEW_NPC".to_owned()],
            topics: Vec::new(),
            roots: Vec::new(),
            coverage: Default::default(),
        };
        let report = new_tree_report(&["UTopic_Hero__NEW_NPC"]);
        let error =
            validate_topicless_module_is_not_scaffolded(Some(&conversation), &outline, &report)
                .unwrap_err();
        assert!(error.to_string().contains("not discovered"), "{error}");
        assert!(error.to_string().contains("new-conversation"), "{error}");
    }

    #[test]
    fn a_settings_anchor_keeps_global_settings_and_places_topics_in_the_conversation_namespace() {
        let pristine = r#"class UConversationCharacterSettings_Ambient_NEW_NPC : UConversationCharacterSettings
{
    default ForCharacter = n"NEW_NPC";
}
"#;
        let generated = r#"class UTopic_Hero__NEW_NPC : UG1RDialogTopic
{
    default ForCharacter = n"NEW_NPC";
    default WithCharacter = n"Hero";
}
class UFirst : UTopic_Hero__NEW_NPC { }
"#;
        let source = append_conversation_namespace(pristine, generated);
        assert!(source.starts_with(pristine));
        let outline = dialog::read_outline(&source).unwrap();
        let report = new_tree_report(&["UTopic_Hero__NEW_NPC", "UFirst"]);
        assert_eq!(
            validate_new_conversation_base(&outline, &report, "NEW_NPC", false)
                .unwrap()
                .namespace,
            CONVERSATION_NAMESPACE
        );
        let settings = validate_settings_anchor_source(&outline, "NEW_NPC").unwrap();
        assert_eq!(settings.namespace, "");

        let mut changed = report;
        changed.changed.push(dialog::ChangedBody {
            class: "UConversationCharacterSettings_Ambient_NEW_NPC".to_owned(),
            member: "UConversationCharacterSettings_Ambient_NEW_NPC()".to_owned(),
        });
        let error = validate_settings_anchor_declarations_unchanged(&changed).unwrap_err();
        assert!(error.to_string().contains("shipped declaration"), "{error}");
    }

    #[test]
    fn settings_anchor_identity_is_exact_and_case_insensitive_only_for_the_npc() {
        let module = "AI.AIAgent.Human.Config.OC_GRD_Guard30_281N.ConversationCharacterSettings_OC_GRD_Guard30_281N";
        let anchor = conversation_settings_anchor_identity(module).unwrap();
        assert_eq!(anchor.module, module);
        assert_eq!(anchor.participant, "OC_GRD_Guard30_281N");
        assert!(anchor
            .participant
            .eq_ignore_ascii_case("oc_grd_guard30_281n"));
        assert!(conversation_settings_anchor_identity(
            "Story.G1R.Conversation.Conversation_OC_GRD_Guard30_281N"
        )
        .is_none());
        assert!(conversation_settings_anchor_identity(
            "AI.AIAgent.Human.Config.OC_GRD_Guard30_281N.ConversationCharacterSettings_OTHER"
        )
        .is_none());

        let duplicate = ConversationSettingsAnchor {
            module: module.to_ascii_lowercase(),
            participant: "oc_grd_guard30_281n".to_owned(),
        };
        let error =
            select_exact_settings_anchor("OC_GRD_Guard30_281N", &[anchor.clone(), duplicate])
                .unwrap_err();
        assert!(
            error.to_string().contains("ambiguous NPC binding"),
            "{error}"
        );
        let error = select_exact_settings_anchor("OTHER", &[anchor]).unwrap_err();
        assert!(
            error.to_string().contains("not discovered by the game"),
            "{error}"
        );
    }

    #[test]
    fn generated_topic_debug_ids_are_stable_and_avoid_the_module() {
        let first = generated_topic_debug_id("Story.Dialog", "UChoiceNew", "");
        assert_ne!(first, 0);
        assert_eq!(
            first,
            generated_topic_debug_id("Story.Dialog", "UChoiceNew", "")
        );
        let occupied = format!("    default DebugId = {first};\n");
        assert_ne!(
            first,
            generated_topic_debug_id("Story.Dialog", "UChoiceNew", &occupied)
        );
    }

    fn command_manifest() -> EditManifest {
        EditManifest {
            operation: DialogModuleOperation::Edit,
            module: "Story.G1R.Conversation.Test".to_owned(),
            relative_path: "Story/G1R/Conversation/Test.as".to_owned(),
            source_file: "Test.as".to_owned(),
            pristine_file: "pristine/Test.as".to_owned(),
            participant: "TestNpc".to_owned(),
            cache_sha256: "00".repeat(32),
            requires_topic_scaffold: false,
            dialog_topics: Vec::new(),
        }
    }

    #[test]
    fn old_dialog_manifests_default_to_edit_operation() {
        let document = serde_json::json!({
            "module": "Story.G1R.Conversation.Test",
            "relative_path": "Story/G1R/Conversation/Test.as",
            "source_file": "Test.as",
            "pristine_file": "pristine/Test.as",
            "participant": "TestNpc",
            "cache_sha256": "00"
        });
        let manifest: EditManifest = serde_json::from_value(document).unwrap();
        assert_eq!(manifest.operation, DialogModuleOperation::Edit);
        assert!(!manifest.requires_topic_scaffold);
    }

    #[test]
    fn legacy_add_manifests_cannot_bypass_the_loaded_anchor_contract() {
        let mut manifest = command_manifest();
        manifest.operation = DialogModuleOperation::Add;
        manifest.requires_topic_scaffold = false;
        let error = validate_manifest_settings_anchor(&[], &manifest).unwrap_err();
        assert!(
            error.to_string().contains("not discovered by the game"),
            "{error}"
        );
    }

    #[test]
    fn a_settings_anchor_manifest_cannot_disable_its_scaffold_checks() {
        let mut manifest = command_manifest();
        manifest.module =
            "AI.AIAgent.Human.Config.NEW_NPC.ConversationCharacterSettings_NEW_NPC".to_owned();
        manifest.participant = "NEW_NPC".to_owned();
        manifest.requires_topic_scaffold = false;
        let error = validate_manifest_settings_anchor(&[], &manifest).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must retain `requires_topic_scaffold = true`"),
            "{error}"
        );
    }

    #[test]
    fn stage_uses_the_strict_standalone_edit_contract() {
        let command = compile_module_command(
            &command_manifest(),
            Path::new("work/source.as"),
            Path::new("work/compiler"),
            Path::new("work/output.mini.Cache"),
            Path::new("game root"),
            true,
        );
        assert_eq!(
            command,
            "gore as compile-module --backend standalone --op edit \
             --module 'Story.G1R.Conversation.Test' \
             --rel-path 'Story/G1R/Conversation/Test.as' \
             --source 'work/source.as' --work-dir 'work/compiler' \
             --allow-new-symbols -o 'work/output.mini.Cache' --game 'game root'"
        );
    }

    #[test]
    fn stage_omits_new_symbol_mode_for_defaults_and_bodies_only() {
        let command = compile_module_command(
            &command_manifest(),
            Path::new("source.as"),
            Path::new("compiler"),
            Path::new("output.Cache"),
            Path::new("game"),
            false,
        );
        assert!(command.contains("--backend standalone --op edit"));
        assert!(!command.contains("--allow-new-symbols"));
    }

    #[test]
    fn a_new_conversation_stages_as_one_settings_anchor_edit_with_new_symbols() {
        let mut manifest = command_manifest();
        manifest.requires_topic_scaffold = true;
        manifest.module =
            "AI.AIAgent.Human.Config.NEW_NPC.ConversationCharacterSettings_NEW_NPC".to_owned();
        manifest.relative_path =
            "AI/AIAgent/Human/Config/NEW_NPC/ConversationCharacterSettings_NEW_NPC.as".to_owned();
        let command = compile_module_command(
            &manifest,
            Path::new("source.as"),
            Path::new("compiler"),
            Path::new("output.Cache"),
            Path::new("game"),
            true,
        );
        assert!(command.contains("--backend standalone --op edit"));
        assert!(command.contains("--allow-new-symbols"));
    }

    #[test]
    fn checkout_refuses_to_overwrite_an_existing_workspace() {
        let temp = tempfile::tempdir().unwrap();
        let out = temp.path().join("dialog-edit");
        fs::create_dir_all(&out).unwrap();
        let sentinel = out.join("authored-work.as");
        fs::write(&sentinel, b"keep this edit").unwrap();

        let error =
            checkout("NPC", &out, Some(temp.path().join("missing.Cache")), None).unwrap_err();

        assert!(error.to_string().contains("is not empty"), "{error}");
        assert_eq!(fs::read(&sentinel).unwrap(), b"keep this edit");
    }

    #[test]
    fn stage_rejects_nonportable_mod_names_before_opening_the_workspace() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("missing-workspace");

        for name in ["../escape", r"C:\escape", "CON", "name.", "name:stream"] {
            let error = stage(&workspace, name, None, None).unwrap_err();
            assert!(
                error.to_string().contains("invalid --mod-name"),
                "unexpected error for {name:?}: {error}"
            );
        }
        assert!(!workspace.exists());
        assert!(!temp.path().join("escape.mini.Cache").exists());
    }

    #[test]
    fn stage_binds_the_compile_target_to_the_checkout_cache_hash() {
        let game = tempfile::tempdir().unwrap();
        let cache = gore_mod::resolve_game_paths(game.path()).script_cache;
        fs::create_dir_all(cache.parent().unwrap()).unwrap();
        fs::write(&cache, b"checkout cache").unwrap();

        let mut manifest = command_manifest();
        manifest.cache_sha256 = digest_of(b"checkout cache");
        assert_eq!(
            compiler_game_for(&manifest, Some(game.path().to_owned())).unwrap(),
            game.path()
        );

        fs::write(&cache, b"updated cache").unwrap();
        let error = compiler_game_for(&manifest, Some(game.path().to_owned())).unwrap_err();
        assert!(error.to_string().contains("not the cache"), "{error}");
    }

    /// End-to-end command oracle for the native same-module root shape. A synthetic cache cannot
    /// exercise dialog checkout faithfully, so opt in with the same real-cache fixture used by
    /// `gore-as`'s dialog agreement tests. The temporary game tree contains only a hard link (or
    /// copy) of that cache and is used solely to qualify the command printed by `stage`.
    #[test]
    fn real_cache_native_root_scaffolds_checks_and_stages_script_only() {
        let Some(cache) = std::env::var_os("GORE_AS_REAL_CACHE").map(PathBuf::from) else {
            eprintln!("skip: set GORE_AS_REAL_CACHE");
            return;
        };
        assert!(cache.is_file(), "{} is not a script cache", cache.display());

        let temp = tempfile::tempdir().unwrap();
        let fake_game = temp.path().join("game");
        let fake_cache = gore_mod::resolve_game_paths(&fake_game).script_cache;
        fs::create_dir_all(fake_cache.parent().unwrap()).unwrap();
        if fs::hard_link(&cache, &fake_cache).is_err() {
            fs::copy(&cache, &fake_cache).unwrap();
        }

        let out = temp.path().join("native-root");
        let class = "UChoiceGoreNativeRootOracle";
        let npc = "Story.G1R.Conversation.Conversation_OC_STT_DIEGO";
        let bytes = fs::read(&cache).unwrap();
        let graph = dialog::build(&bytes).unwrap();
        let conversation = resolve_one(&graph, npc).unwrap();
        let expected_root_priority = automatic_root_priority_rank(conversation).unwrap();
        assert_ne!(expected_root_priority, -1);
        new_topic(NewTopicRequest {
            npc: npc.to_owned(),
            caption: None,
            caption_key: Some("GORE_DIALOG_NATIVE_ROOT_ORACLE".to_owned()),
            class: Some(class.to_owned()),
            subdialog_of: None,
            subdialog_position: None,
            priority_rank: None,
            mod_name: "GoreNativeRootOracle".to_owned(),
            out: out.clone(),
            cache: Some(cache.clone()),
            game: None,
        })
        .unwrap();

        let manifest_path = out.join(MANIFEST_NAME);
        let manifest_text = fs::read_to_string(&manifest_path).unwrap();
        let mut manifest: EditManifest = serde_json::from_str(&manifest_text).unwrap();
        assert!(manifest.dialog_topics.is_empty());
        assert!(
            !manifest_text.contains("dialog_topics"),
            "a native root scaffold must not serialize an adapter row"
        );
        let authored = fs::read_to_string(out.join(&manifest.source_file)).unwrap();
        let outline = dialog::read_outline(&authored).unwrap();
        let added = outline
            .classes
            .iter()
            .find(|candidate| candidate.name == class)
            .unwrap();
        assert_eq!(literal_bool_default(added, "bIsSubTopic").unwrap(), None);
        let root_priority = added
            .defaults
            .iter()
            .filter(|default| default.target == "PriorityRank")
            .collect::<Vec<_>>();
        let [root_priority] = root_priority.as_slice() else {
            panic!(
                "expected one authored PriorityRank default, got {}",
                root_priority.len()
            );
        };
        assert_eq!(
            code_tokens(&root_priority.statement)
                .unwrap()
                .into_iter()
                .map(|token| token.text)
                .collect::<Vec<_>>(),
            vec![
                "default".to_owned(),
                "PriorityRank".to_owned(),
                "=".to_owned(),
                expected_root_priority.to_string(),
                ";".to_owned(),
            ]
        );

        check(&out, true, Some(cache.clone()), Some(fake_game.clone())).unwrap();
        stage(
            &out,
            "GoreNativeRootOracle",
            Some(cache.clone()),
            Some(fake_game.clone()),
        )
        .unwrap();
        let spec: serde_json::Value =
            serde_json::from_slice(&fs::read(out.join("spec.json")).unwrap()).unwrap();
        assert!(spec.get("dialog_topics").is_none());
        assert_eq!(spec["scripts"][0]["op"], "edit");
        assert_eq!(
            spec.as_object().unwrap().len(),
            2,
            "stage must stay script-only"
        );

        // Exercise the shipped fixed-width form as well: Diego's teaching menu has a stable
        // trailing TEXT_BACK child and therefore qualifies both default placement and the
        // generated sub-topic defaults against the same real cache.
        let sub_out = temp.path().join("teach-subtopic");
        let sub_class = "UChoiceGoreTeachPlacementOracle";
        new_topic(NewTopicRequest {
            npc: npc.to_owned(),
            caption: None,
            caption_key: Some("GORE_DIALOG_TEACH_PLACEMENT_ORACLE".to_owned()),
            class: Some(sub_class.to_owned()),
            subdialog_of: Some("UChoiceDiegoTeach".to_owned()),
            subdialog_position: None,
            priority_rank: None,
            mod_name: "GoreTeachPlacementOracle".to_owned(),
            out: sub_out.clone(),
            cache: Some(cache.clone()),
            game: None,
        })
        .unwrap();

        let sub_manifest: EditManifest =
            serde_json::from_slice(&fs::read(sub_out.join(MANIFEST_NAME)).unwrap()).unwrap();
        assert!(sub_manifest.dialog_topics.is_empty());
        let sub_authored = fs::read_to_string(sub_out.join(&sub_manifest.source_file)).unwrap();
        let sub_outline = dialog::read_outline(&sub_authored).unwrap();
        let sub_added = sub_outline
            .classes
            .iter()
            .find(|candidate| candidate.name == sub_class)
            .unwrap();
        assert_eq!(
            literal_bool_default(sub_added, "bIsSubTopic").unwrap(),
            Some(true)
        );
        let priority = sub_added
            .defaults
            .iter()
            .filter(|default| default.target == "PriorityRank")
            .collect::<Vec<_>>();
        let [priority] = priority.as_slice() else {
            panic!(
                "expected one authored PriorityRank default, got {}",
                priority.len()
            );
        };
        assert_eq!(
            code_tokens(&priority.statement)
                .unwrap()
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>(),
            ["default", "PriorityRank", "=", "0", ";"]
        );
        assert!(
            sub_authored.contains(&format!(
                "{sub_class}, G1R::Conversation::UChoiceDiegoTeachBack,"
            )),
            "the new child must be immediately before Diego's trailing TEXT_BACK topic"
        );

        check(&sub_out, true, Some(cache.clone()), Some(fake_game.clone())).unwrap();
        stage(
            &sub_out,
            "GoreTeachPlacementOracle",
            Some(cache.clone()),
            Some(fake_game.clone()),
        )
        .unwrap();
        let sub_spec: serde_json::Value =
            serde_json::from_slice(&fs::read(sub_out.join("spec.json")).unwrap()).unwrap();
        assert!(sub_spec.get("dialog_topics").is_none());
        assert_eq!(sub_spec["scripts"][0]["op"], "edit");

        // Old checked-out workspaces may still carry an explicit adapter row. Keep accepting that
        // shape when (and only when) its participant, new class, and sentinel bind to this cache.
        let participant = conversation.npc_participants().next().unwrap();
        let sentinel = sentinel_of(conversation).unwrap();
        manifest.dialog_topics.push(DialogTopicRegistration {
            id: "legacy-native-root-oracle".to_owned(),
            participant_name: participant.to_ascii_lowercase(),
            topic_class: reflected_topic_path(class),
            sentinel_class: reflected_topic_path(&sentinel.class),
            allow_hidden: true,
        });
        fs::write(
            &manifest_path,
            format!("{}\n", serde_json::to_string_pretty(&manifest).unwrap()),
        )
        .unwrap();
        check(&out, true, Some(cache), Some(fake_game)).unwrap();
    }

    #[test]
    fn real_cache_new_conversation_edits_one_loaded_settings_anchor_and_fails_closed() {
        let Some(cache) = std::env::var_os("GORE_AS_REAL_CACHE").map(PathBuf::from) else {
            eprintln!("skip: set GORE_AS_REAL_CACHE");
            return;
        };
        assert!(cache.is_file(), "{} is not a script cache", cache.display());

        let temp = tempfile::tempdir().unwrap();
        let fake_game = temp.path().join("game");
        let fake_cache = gore_mod::resolve_game_paths(&fake_game).script_cache;
        fs::create_dir_all(fake_cache.parent().unwrap()).unwrap();
        if fs::hard_link(&cache, &fake_cache).is_err() {
            fs::copy(&cache, &fake_cache).unwrap();
        }

        let bytes = fs::read(&cache).unwrap();
        let missing_out = temp.path().join("unbound-add");
        let missing_error = new_conversation(NewConversationRequest {
            npc: "GORE_TEST_DIALOG_NPC".to_owned(),
            caption: None,
            caption_key: Some("GORE_DIALOG_UNBOUND_ORACLE".to_owned()),
            class: Some("UChoiceGoreUnboundOracle".to_owned()),
            priority_rank: DEFAULT_ROOT_PRIORITY_RANK,
            mod_name: "GoreUnboundOracle".to_owned(),
            out: missing_out.clone(),
            cache: Some(cache.clone()),
            game: None,
        })
        .unwrap_err();
        assert!(
            format!("{missing_error:#}").contains("not discovered by the game"),
            "{missing_error:#}"
        );
        assert!(!missing_out.exists());

        // This exact settings anchor is the live runtime oracle: editing it made the forced
        // opening, spoken line, new Subdialog menu and clean end execute for the Guard.
        let participant = "OC_GRD_Guard30_281N";
        let anchor = resolve_conversation_settings_anchor(&bytes, participant).unwrap();
        assert_eq!(
            anchor.module,
            "AI.AIAgent.Human.Config.OC_GRD_Guard30_281N.ConversationCharacterSettings_OC_GRD_Guard30_281N"
        );
        let out = temp.path().join("settings-anchor-edit");
        let first_class = "UChoiceGoreSettingsAnchorOracle";
        let first_priority = 7;
        new_conversation(NewConversationRequest {
            npc: participant.to_owned(),
            caption: None,
            caption_key: Some("GORE_DIALOG_SETTINGS_ANCHOR_ORACLE".to_owned()),
            class: Some(first_class.to_owned()),
            priority_rank: first_priority,
            mod_name: "GoreSettingsAnchorOracle".to_owned(),
            out: out.clone(),
            cache: Some(cache.clone()),
            game: None,
        })
        .unwrap();
        let manifest_path = out.join(MANIFEST_NAME);
        let manifest: EditManifest =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        assert_eq!(manifest.operation, DialogModuleOperation::Edit);
        assert!(manifest.requires_topic_scaffold);
        assert_eq!(manifest.module, anchor.module);
        let pristine = fs::read_to_string(out.join(&manifest.pristine_file)).unwrap();
        let source_path = out.join(&manifest.source_file);
        let authored = fs::read_to_string(&source_path).unwrap();
        assert!(
            authored.starts_with(&pristine),
            "the shipped settings source must remain an exact prefix"
        );
        let outline = dialog::read_outline(&authored).unwrap();
        let settings = validate_settings_anchor_source(&outline, participant).unwrap();
        assert_eq!(settings.namespace, "");
        let root = outline
            .classes
            .iter()
            .find(|class| class.name == expected_topic_root(participant))
            .unwrap();
        assert_eq!(root.namespace, CONVERSATION_NAMESPACE);
        let first = outline
            .classes
            .iter()
            .find(|class| class.name == first_class)
            .unwrap();
        assert!(first.defaults.iter().any(|default| {
            default.target == "PriorityRank"
                && code_tokens(&default.statement).is_ok_and(|tokens| {
                    tokens
                        .iter()
                        .map(|token| token.text.as_str())
                        .collect::<Vec<_>>()
                        == ["default", "PriorityRank", "=", "7", ";"]
                })
        }));
        check(&out, true, Some(cache.clone()), Some(fake_game.clone())).unwrap();

        // Extend the generated root option into a tree made solely from new topic classes. This
        // runs through the product check/stage path, not just the graph helper tests above.
        let mut tree = fs::read_to_string(&source_path).unwrap();
        let root_class = expected_topic_root(participant);
        let children = fixed_subdialog_arguments(&["UChoiceGoreBrandNewChild"]);
        tree = tree.replacen(
            "        this.EndConversation();",
            &format!("        ::Subdialog(this, {children});"),
            1,
        );
        let extra = format!(
            "\n{}{}",
            add_say_before_subdialog(new_tree_topic(
                "UChoiceGoreBrandNewChild",
                i64::MAX - 1,
                true,
                &["UChoiceGoreBrandNewGrandchild"],
            ))
            .replace(": URoot", &format!(": {root_class}")),
            new_tree_topic("UChoiceGoreBrandNewGrandchild", i64::MAX, true, &[],)
                .replace(": URoot", &format!(": {root_class}")),
        );
        let namespace_end = tree.rfind('}').unwrap();
        tree.insert_str(namespace_end, &extra);
        fs::write(&source_path, &tree).unwrap();
        check(&out, true, Some(cache.clone()), Some(fake_game.clone())).unwrap();
        stage(
            &out,
            "GoreSettingsAnchorOracle",
            Some(cache.clone()),
            Some(fake_game.clone()),
        )
        .unwrap();
        let spec: serde_json::Value =
            serde_json::from_slice(&fs::read(out.join("spec.json")).unwrap()).unwrap();
        assert_eq!(spec["scripts"][0]["op"], "edit");
        assert_eq!(spec["scripts"][0]["module_name"], anchor.module);
        assert!(spec.get("dialog_topics").is_none());

        // A manifest cannot redirect the scaffold to a separate Story module or resurrect the
        // old unbound --op add path.
        let safe_module = anchor.module.clone();
        let safe_relative_path = manifest.relative_path.clone();
        let mut tampered = manifest;
        tampered.operation = DialogModuleOperation::Add;
        tampered.module = "Story.G1R.Conversation.Conversation_OC_GRD_Guard30_281N".to_owned();
        tampered.relative_path =
            "Story/G1R/Conversation/Conversation_OC_GRD_Guard30_281N.as".to_owned();
        fs::write(
            &manifest_path,
            format!("{}\n", serde_json::to_string_pretty(&tampered).unwrap()),
        )
        .unwrap();
        let error = match open_edit(&out, Some(cache.clone()), Some(fake_game.clone())) {
            Ok(_) => panic!("an unbound add manifest was accepted"),
            Err(error) => error,
        };
        assert!(
            format!("{error:#}").contains("not discovered by the game"),
            "{error:#}"
        );

        tampered.operation = DialogModuleOperation::Edit;
        tampered.module = safe_module;
        tampered.relative_path = safe_relative_path;
        fs::write(
            &manifest_path,
            format!("{}\n", serde_json::to_string_pretty(&tampered).unwrap()),
        )
        .unwrap();

        // Restoring only the pristine settings is not a valid new-conversation edit: the
        // manifest still promises a private root and at least one option.
        fs::write(&source_path, pristine).unwrap();
        let error = match open_edit(&out, Some(cache), Some(fake_game)) {
            Ok(_) => panic!("a gutted settings-anchor workspace was accepted"),
            Err(error) => error,
        };
        assert!(
            format!("{error:#}").contains("new conversation"),
            "{error:#}"
        );
    }

    #[test]
    fn powershell_arguments_escape_single_quotes() {
        assert_eq!(
            powershell_quote(Path::new("work/author's source.as")),
            "'work/author''s source.as'"
        );
    }
}
