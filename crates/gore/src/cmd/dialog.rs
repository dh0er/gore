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
    /// Insert a new dialog option into one NPC's checked-out conversation module
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
        /// Permit a state-dependent root topic to be cleanly hidden at runtime
        #[arg(long, conflicts_with = "subdialog_of")]
        allow_hidden: bool,
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
            allow_hidden,
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
            allow_hidden,
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
    format!(
        "/Script/Angelscript.{}",
        class_without_object_prefix(class)
    )
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

/// Bind a staged root registration to the exact class and base conversation checked above.
/// Without this gate an edited JSON manifest could point at a renamed/deleted class or another
/// participant and still build successfully, failing only when the runtime adapter looks it up.
fn validate_topic_registrations(
    manifest: &EditManifest,
    report: &dialog::EditReport,
    authored: &str,
    cache: &[u8],
) -> Result<()> {
    if report.added_classes.is_empty() && manifest.dialog_topics.is_empty() {
        return Ok(());
    }

    let graph = dialog::build(cache).context("re-reading the base dialog for registration checks")?;
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
    let qualified_root = if root_outline.namespace.is_empty() {
        root_class.to_owned()
    } else {
        format!("{}::{root_class}", root_outline.namespace)
    };
    let added_topics = outline
        .classes
        .iter()
        .filter(|class| {
            report.added_classes.contains(&class.name)
                && class.super_class.as_deref().is_some_and(|parent| {
                    (parent == root_class && class.namespace == root_outline.namespace)
                        || parent.strip_prefix("::").unwrap_or(parent) == qualified_root
                })
        })
        .map(|class| class.name.clone())
        .collect::<BTreeSet<_>>();

    let participants = conversation
        .npc_participants()
        .map(|participant| participant.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let sentinel = sentinel_of(conversation)
        .map(|topic| reflected_topic_path(&topic.class))
        .context("the base conversation has no root sentinel for registration")?;
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
        match (roots, owners.len(), subdialog_is_existing) {
            (1, 0, false) | (0, 1, true) => {}
            _ => bail!(
                "new topic {class} must be either registered once as a root or referenced once by Subdialog from a shipped class; found {roots} registration(s) and references from {:?}",
                owners
            ),
        }
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
        validate_topic_registrations(&manifest, &report, &authored, &bytes)
            .context("the dialog registration manifest is not bound to this checked source")?;
    }
    Ok((manifest, report, source_path))
}

fn check(dir: &PathBuf, json: bool, cache: Option<PathBuf>, game: Option<PathBuf>) -> Result<()> {
    let (manifest, report, source_path) = open_edit(dir, cache, game)?;

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
    fs::create_dir_all(&work_dir)
        .with_context(|| format!("creating {}", work_dir.display()))?;
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
    pub allow_hidden: bool,
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

/// Add one class argument to the one `Subdialog` call in one existing topic class.
///
/// This is intentionally narrower than a source rewriter: ambiguity is a refusal, and the final
/// `dialog check` independently verifies that no shipped declaration/default target was lost.
fn wire_subdialog(source: &str, parent: &str, child: &str) -> Result<String> {
    let tokens = code_tokens(source)?;
    let mut class_bodies = Vec::new();
    for index in 0..tokens.len().saturating_sub(1) {
        if tokens[index].text != "class" || tokens[index + 1].text != parent {
            continue;
        }
        let Some(open) = ((index + 2)..tokens.len()).find(|candidate| {
            matches!(tokens[*candidate].text.as_str(), "{" | ";")
        }) else {
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
                && tokens.get(*index + 1).is_some_and(|token| token.text == "(")
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
    let placeholder = ((open + 1)..close.saturating_sub(6)).find(|start| {
        [
            "TSubclassOf",
            "<",
            "UConversationTopic",
            ">",
            "(",
            "nullptr",
            ")",
        ]
        .iter()
        .enumerate()
        .all(|(offset, expected)| tokens[*start + offset].text == *expected)
    });
    let Some(placeholder) = placeholder else {
        bail!("the Subdialog call in {parent} has no empty topic slot");
    };
    let mut edited = source.to_owned();
    edited.replace_range(
        tokens[placeholder].start..tokens[placeholder + 6].end,
        child,
    );
    Ok(edited)
}

fn new_topic(request: NewTopicRequest) -> Result<()> {
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
    let topic_source = format!(
        "// Generated by `gore dialog new-topic` for {participant}.\n\
         //\n\
         // This class stays in the conversation's own module and namespace. Keep it there: a\n\
         // separate add-module cannot derive from the module-private topic base. Spoken lines, conditions\n\
         // and effects are yours to add; `gore dialog show <topic>` displays shipped examples.\n\
         \n\
         {helper_block}class {class} : {root_class}\n\
         {{\n\
         {caption_line}\n\
         \x20   default PriorityRank = 2;\n\
         {subtopic_default}\
         \n\
         \x20   UFUNCTION()\n\
         \x20   bool IsVisible_Implementation()\n\
         \x20   {{\n\
         \x20       return true;\n\
         \x20   }}\n\
         \n\
         \x20   UFUNCTION()\n\
         \x20   void Act_Implementation()\n\
         \x20   {{\n\
         \x20       this.EndConversation();\n\
         \x20   }}\n\
         }}\n"
    );

    let taken = dialog::checkout(&bytes, &conversation.module, native_api(&cache_path))
        .with_context(|| format!("taking {} out of the cache", conversation.module))?;
    let mut source = if let Some(parent_name) = request.subdialog_of.as_deref() {
        let parent = resolve_topic_in(conversation, parent_name)?;
        let subdialog_count = parent
            .act
            .iter()
            .filter(|step| matches!(step.kind, StepKind::Subdialog { .. }))
            .count();
        if subdialog_count != 1 {
            bail!(
                "{} has {subdialog_count} compiled Subdialog calls; --subdialog-of requires exactly one",
                parent.class
            );
        }
        wire_subdialog(&taken.source, &parent.class, &class)
            .with_context(|| format!("wiring the new topic into {}", parent.class))?
    } else {
        taken.source.clone()
    };
    source = append_to_class_namespace(&source, &root_class, &topic_source)
        .with_context(|| format!("placing {class} beside its conversation base {root_class}"))?;

    let dialog_topics = if request.subdialog_of.is_none() {
        let Some(sentinel) = sentinel_of(conversation) else {
            bail!(
                "{} has no root option to use as the registration sentinel",
                participant_label(conversation)
            );
        };
        vec![DialogTopicRegistration {
            id: slug.to_lowercase(),
            participant_name: participant.to_lowercase(),
            topic_class: reflected_topic_path(&class),
            sentinel_class: reflected_topic_path(&sentinel.class),
            allow_hidden: request.allow_hidden,
        }]
    } else {
        Vec::new()
    };

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
    if !report.added_classes.iter().any(|added| added == &class)
        || !report.requires_new_symbols()
    {
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
    } else {
        println!("root registration recorded for the staged bundle spec");
    }
    if request.caption_key.is_some() {
        println!("localization key recorded in source only; add its localized row separately");
    } else {
        println!("caption is an untranslated literal; localized rows remain a separate payload");
    }
    println!();
    println!("next:");
    println!(
        "  gore dialog check {}",
        powershell_quote(&request.out)
    );
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

    #[test]
    fn a_subdialog_child_is_wired_only_into_the_named_class_code() {
        let source = r#"
// class UParent { Subdialog(UBogus::StaticClass()); }
class UParent : UBase
{
    void Act_Implementation()
    {
        FString Example = "Subdialog(also not code)";
        this.Subdialog(UFirst, TSubclassOf<UConversationTopic>(nullptr));
    }
}

class UOther : UBase
{
    void Act_Implementation()
    {
        this.Subdialog(UUntouched, TSubclassOf<UConversationTopic>(nullptr));
    }
}
"#;
        let edited = wire_subdialog(source, "UParent", "UNewChild").unwrap();
        assert!(edited.contains("this.Subdialog(UFirst, UNewChild);"));
        assert!(edited.contains(
            "this.Subdialog(UUntouched, TSubclassOf<UConversationTopic>(nullptr));"
        ));
        assert_eq!(edited.matches("UNewChild").count(), 1);
        assert_eq!(
            subdialog_reference_owners(&edited, "UNewChild").unwrap(),
            ["UParent"]
        );
    }

    #[test]
    fn ambiguous_subdialog_wiring_fails_closed() {
        let source = r#"
class UParent : UBase
{
    void Act_Implementation()
    {
        this.Subdialog(UA, TSubclassOf<UConversationTopic>(nullptr));
        this.Subdialog(UB, TSubclassOf<UConversationTopic>(nullptr));
    }
}
"#;
        let error = wire_subdialog(source, "UParent", "UNewChild").unwrap_err();
        assert!(error.to_string().contains("exactly one"), "{error}");
    }

    #[test]
    fn a_full_subdialog_call_fails_closed() {
        let source = r#"
class UParent : UBase
{
    void Act_Implementation()
    {
        this.Subdialog(UA, UB);
    }
}
"#;
        let error = wire_subdialog(source, "UParent", "UNewChild").unwrap_err();
        assert!(error.to_string().contains("no empty topic slot"), "{error}");
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
        let duplicate =
            "namespace A { class UBase {} }\nnamespace B { class UBase {} }\n";
        let error = append_to_class_namespace(duplicate, "UBase", "class UNew : UBase {}")
            .unwrap_err();
        assert!(error.to_string().contains("exactly one"), "{error}");
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

    #[test]
    fn powershell_arguments_escape_single_quotes() {
        assert_eq!(
            powershell_quote(Path::new("work/author's source.as")),
            "'work/author''s source.as'"
        );
    }
}
