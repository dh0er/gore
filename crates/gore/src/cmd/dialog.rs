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

use super::find::{load_name_index, NameIndexState};

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
    /// Take one conversation's AngelScript, including class defaults, out for editing
    Checkout {
        /// Participant identifier (`om_stt_viper_302`), part of one, or a module name
        npc: String,
        /// Working directory for the source, its pristine copy, and the manifest
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
        /// Mod name for the bundle this edit ships in
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

/// Conversations whose participants or module match `needle`, matched case-insensitively.
fn matching<'a>(graph: &'a DialogGraph, needle: &str) -> Vec<&'a Conversation> {
    let needle = needle.to_lowercase();
    let exact: Vec<&Conversation> = graph
        .conversations
        .iter()
        .filter(|conversation| {
            conversation
                .npc_participants()
                .any(|participant| participant.to_lowercase() == needle)
                || conversation.module.to_lowercase() == needle
        })
        .collect();
    if !exact.is_empty() {
        return exact;
    }
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
                let chosen = columns
                    .iter()
                    .find_map(|column| {
                        names
                            .iter()
                            .find(|name| name.language.eq_ignore_ascii_case(column))
                    })
                    .or_else(|| names.first());
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
            for arg in &rule.args {
                if let Arg::Text { value } = arg {
                    keys.insert(value.to_lowercase());
                }
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
        Some(needle) => matching(&graph, needle),
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
            print_topic(conversation, topic, text, 0, depth, ids, &mut printed);
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
        print!("; {} unresolved call(s)", coverage.calls_unresolved);
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
        print_step(conversation, step, text, level + 1, depth, ids, printed);
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
            let next = level + 1;
            if depth.is_some_and(|limit| next > limit) {
                println!("{}… {} option(s) not shown", pad(next), children.len());
                return;
            }
            for child in children {
                match conversation.topic(child) {
                    Some(topic) => {
                        print_topic(conversation, topic, text, next, depth, ids, printed)
                    }
                    None => println!("{}- {child} (declared elsewhere)", pad(next)),
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
    print_topic(conversation, found, &text, 0, Some(1), true, &mut printed);
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
        let chosen = columns.iter().find_map(|column| {
            names
                .iter()
                .find(|name| name.language.eq_ignore_ascii_case(column))
        });
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
    /// Explicit root-topic registrations for the bundle spec. Subdialog topics are wired by the
    /// authored `Subdialog` call and need no transient root registration.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    dialog_topics: Vec<DialogTopicRegistration>,
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

fn subdialog_reference_owners(source: &str, class: &str) -> Result<Vec<String>> {
    let tokens = code_tokens(source)?;
    let mut owners = Vec::new();
    for declaration in 0..tokens.len().saturating_sub(1) {
        if !matches!(tokens[declaration].text.as_str(), "class" | "struct") {
            continue;
        }
        let Some(name) = tokens.get(declaration + 1).map(|token| token.text.clone()) else {
            continue;
        };
        let Some(open) = ((declaration + 2)..tokens.len())
            .find(|candidate| matches!(tokens[*candidate].text.as_str(), "{" | ";"))
        else {
            continue;
        };
        if tokens[open].text != "{" {
            continue;
        }
        let class_close = matching_close(&tokens, open, "{", "}")?;
        for call in (open + 1)..class_close.saturating_sub(1) {
            if tokens[call].text != "Subdialog" || tokens[call + 1].text != "(" {
                continue;
            }
            let close = matching_close(&tokens, call + 1, "(", ")")?;
            if close < class_close
                && tokens[call + 2..close]
                    .iter()
                    .any(|token| token.text == class)
            {
                owners.push(name.clone());
            }
        }
    }
    Ok(owners)
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

/// Bind each new topic to one native root, legacy adapter registration, or shipped Subdialog owner.
/// Legacy `dialog_topics` rows remain supported for old workspaces, but new same-module roots need
/// no adapter. Without this gate an edited JSON manifest could still point at a renamed/deleted
/// class or another participant and fail only when the runtime adapter looks it up.
fn validate_topic_registrations(
    manifest: &EditManifest,
    report: &dialog::EditReport,
    authored: &str,
    cache: &[u8],
) -> Result<()> {
    if report.added_classes.is_empty() && manifest.dialog_topics.is_empty() {
        return Ok(());
    }

    let graph =
        dialog::build(cache).context("re-reading the base dialog for registration checks")?;
    let matches = graph
        .conversations
        .iter()
        .filter(|conversation| conversation.module == manifest.module)
        .collect::<Vec<_>>();
    let [conversation] = matches.as_slice() else {
        bail!(
            "the edit module maps to {} base conversations; exactly one is required for a new topic",
            matches.len()
        );
    };
    let root_class = conversation
        .root_class
        .as_deref()
        .context("the base conversation has no private root topic class")?;
    let outline = dialog::read_outline(authored)
        .map_err(|reason| anyhow::anyhow!("inventorying new topic classes: {reason}"))?;
    let root_matches = outline
        .classes
        .iter()
        .filter(|class| class.name == root_class)
        .collect::<Vec<_>>();
    let [root_outline] = root_matches.as_slice() else {
        bail!(
            "the private root class {root_class} has {} source identities; exactly one is required",
            root_matches.len()
        );
    };
    let added_topics = direct_added_topic_classes(&outline, &report.added_classes, root_outline)?;

    for class_name in &added_topics {
        let class = outline
            .classes
            .iter()
            .find(|class| &class.name == class_name)
            .with_context(|| {
                format!("new topic {class_name} disappeared from the source outline")
            })?;
        validate_new_topic_source_contract(class)?;
    }

    let participants = conversation
        .npc_participants()
        .map(|participant| participant.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let sentinel = if manifest.dialog_topics.is_empty() {
        None
    } else {
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
        let sentinel = sentinel
            .as_deref()
            .expect("a registration list always resolves a sentinel first");
        if registration.sentinel_class != sentinel {
            bail!(
                "dialog_topics sentinel {:?} is not this conversation's checked sentinel {:?}",
                registration.sentinel_class,
                sentinel
            );
        }
        *registrations.entry(class).or_default() += 1;
    }

    for class in added_topics {
        let roots = registrations.get(&class).copied().unwrap_or(0);
        let owners = subdialog_reference_owners(authored, &class)?;
        let subdialog_is_existing = owners
            .as_slice()
            .first()
            .is_some_and(|owner| !report.added_classes.contains(owner));
        let is_subdialog = match (roots, owners.len(), subdialog_is_existing) {
            (0, 0, false) => false,
            (1, 0, false) => false,
            (0, 1, true) => true,
            _ => bail!(
                "new topic {class} must be a native root, registered once by a legacy adapter, or referenced once by Subdialog from a shipped class; found {roots} registration(s) and references from {:?}",
                owners
            ),
        };
        let outline = outline
            .classes
            .iter()
            .find(|candidate| candidate.name == class)
            .with_context(|| format!("new topic {class} disappeared from the source outline"))?;
        validate_new_topic_placement(outline, is_subdialog)?;
    }
    Ok(())
}

const MANIFEST_NAME: &str = "gore-dialog-edit.json";

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
    let (path, bytes) = read_cache(cache, game)?;
    let graph = dialog::build(&bytes).context("reading dialog from the script cache")?;
    let conversation = resolve_one(&graph, npc)?;
    if conversation.topics.is_empty() {
        bail!(
            "{} declares no topics, so there is nothing to edit",
            participant_label(conversation)
        );
    }

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
        module: taken.module.clone(),
        relative_path: taken.relative_path.clone(),
        source_file: leaf.clone(),
        pristine_file: format!("pristine/{leaf}"),
        participant: participant_label(conversation),
        cache_sha256: digest_of(&bytes),
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

    let taken = dialog::checkout(&bytes, &manifest.module, native_api(&path))
        .with_context(|| format!("re-reading {}", manifest.module))?;
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
            "op": "edit",
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
    println!("carry is only the fallback for sources that author no defaults at all.");
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
        "gore as compile-module --backend standalone --op edit --module {} --rel-path {} \
         --source {} --work-dir {}{} -o {} --game {}",
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
    pub mod_name: String,
    pub out: PathBuf,
    pub cache: Option<PathBuf>,
    pub game: Option<PathBuf>,
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

const SUBDIALOG_TOPIC_SLOTS: usize = 20;
const EMPTY_SUBDIALOG_SLOT: &str = "TSubclassOf<UConversationTopic>(nullptr)";
const BACK_CAPTION_KEY: &str = "TEXT_BACK";

type FixedSubdialogCalls = BTreeMap<String, Vec<Vec<Option<String>>>>;

#[derive(Clone, Copy)]
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
fn fixed_subdialog_calls(source: &str) -> Result<FixedSubdialogCalls> {
    let tokens = code_tokens(source)?;
    let mut calls = BTreeMap::<String, Vec<Vec<Option<String>>>>::new();

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
            calls.entry(owner.clone()).or_default().push(children);
        }
    }

    Ok(calls)
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
            "{} declares no dialog topics, so there is no base class to derive from",
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

    let helper = format!("{}Caption", class_without_object_prefix(&class));
    let caption_line = match (&request.caption, &request.caption_key) {
        (Some(text), _) => format!(
            "    default Caption = {helper}(n{});",
            serde_json::to_string(text)?
        ),
        (_, Some(key)) => format!(
            "    default Caption = LocText({});",
            serde_json::to_string(key)?
        ),
        _ => bail!("pass --caption or --caption-key"),
    };
    let helper_block = if request.caption.is_some() {
        format!(
            "FText {helper}(const FName Text)\n{{\n    return FText::FromString(Text.ToString());\n}}\n\n"
        )
    } else {
        String::new()
    };

    let subtopic_default = if request.subdialog_of.is_some() {
        "    default bIsSubTopic = true;\n"
    } else {
        ""
    };
    // Shipped sub-topics use rank 0 and are ordered by their fixed Subdialog argument slots.
    // Giving a newly wired child the root scaffold's rank 2 can move it past a trailing Back row
    // even when the source call places it correctly.
    let priority_rank = if request.subdialog_of.is_some() { 0 } else { 2 };
    let topic_source = format!(
        "// Generated by `gore dialog new-topic` for {participant}.\n\
         //\n\
         // This class stays in the conversation's own module and namespace. Keep it there: a\n\
         // separate add-module cannot derive from the module-private topic base. Spoken lines, conditions\n\
         // and effects are yours to add; `gore dialog show <topic>` displays shipped examples.\n\
         \n\
         {helper_block}class {class} : {root_class}\n\
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

    if request.out.exists()
        && fs::read_dir(&request.out)
            .with_context(|| format!("reading {}", request.out.display()))?
            .next()
            .is_some()
    {
        bail!(
            "{} is not empty; choose an empty --out directory so no stale spec or source survives",
            request.out.display()
        );
    }
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
        module: taken.module,
        relative_path: taken.relative_path,
        source_file: leaf.clone(),
        pristine_file: format!("pristine/{leaf}"),
        participant: participant_label(conversation),
        cache_sha256: digest_of(&bytes),
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

fn export(out: &PathBuf, cache: Option<PathBuf>, game: Option<PathBuf>) -> Result<()> {
    let graph = read_graph(cache, game)?;
    fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;
    for conversation in &graph.conversations {
        let name = format!("{}.json", conversation.module.replace('.', "_"));
        let path = out.join(name);
        let json = serde_json::to_string_pretty(conversation)?;
        fs::write(&path, json).with_context(|| format!("writing {}", path.display()))?;
    }
    println!(
        "wrote {} conversation(s) to {}",
        graph.conversations.len(),
        out.display()
    );
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
        let conversation = Conversation {
            module: "M".to_owned(),
            root_class: Some("URoot".to_owned()),
            participants: vec!["Hero".to_owned(), "NPC".to_owned()],
            topics: vec![
                topic(
                    "URootTopic",
                    "CAP_ROOT",
                    vec![
                        say("LINE_ONE"),
                        StepKind::Subdialog {
                            children: vec!["UChild".to_owned()],
                        },
                        say("LINE_ONE"),
                    ],
                ),
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
        assert_eq!(
            subdialog_reference_owners(&edited, "UNewChild").unwrap(),
            ["UParent"]
        );
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
            module: "Story.G1R.Conversation.Test".to_owned(),
            relative_path: "Story/G1R/Conversation/Test.as".to_owned(),
            source_file: "Test.as".to_owned(),
            pristine_file: "pristine/Test.as".to_owned(),
            participant: "TestNpc".to_owned(),
            cache_sha256: "00".repeat(32),
            dialog_topics: Vec::new(),
        }
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
        new_topic(NewTopicRequest {
            npc: npc.to_owned(),
            caption: None,
            caption_key: Some("GORE_DIALOG_NATIVE_ROOT_ORACLE".to_owned()),
            class: Some(class.to_owned()),
            subdialog_of: None,
            subdialog_position: None,
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
        let bytes = fs::read(&cache).unwrap();
        let graph = dialog::build(&bytes).unwrap();
        let conversation = resolve_one(&graph, npc).unwrap();
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
    fn powershell_arguments_escape_single_quotes() {
        assert_eq!(
            powershell_quote(Path::new("work/author's source.as")),
            "'work/author''s source.as'"
        );
    }
}
