//! `gore dialog` — read the game's dialog trees.
//!
//! Everything here is offline and read-only. The tree comes out of the installed shipping script
//! cache; the text comes out of the shared localization catalog that `gore loc extract` writes.
//! Neither the game nor a save is touched, and nothing is deployed.
//!
//! What the cache declares is not the same as what a player sees: a topic's rules and its
//! `IsVisible` override decide that per save state, and this command deliberately reports both
//! rather than pretending to evaluate them.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::PathBuf;

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

fn read_graph(cache: Option<PathBuf>, game: Option<PathBuf>) -> Result<DialogGraph> {
    let path = cache_path(cache, game)?;
    let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
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
    let owner = topic.class.trim_start_matches('U');
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
        Arg::Class { name } => Some(name.trim_start_matches('U').to_owned()),
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
        Arg::Class { name } => name.trim_start_matches('U').to_owned(),
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
    let wanted = topic.trim_start_matches('U').to_lowercase();
    let found = graph.conversations.iter().find_map(|conversation| {
        conversation
            .topics
            .iter()
            .find(|candidate| candidate.class.trim_start_matches('U').to_lowercase() == wanted)
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
}
