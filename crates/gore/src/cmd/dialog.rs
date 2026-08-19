//! `gore dialog` — read the game's dialog trees.
//!
//! Everything here works offline and only ever reads the installation. The tree comes out of the
//! shipping script cache; the text comes out of the shared localization catalog that `gore loc
//! extract` writes. Nothing here launches the game, writes into the install, touches a save, or
//! deploys; the commands that produce something write only where they are pointed.
//!
//! `checkout`/`check`/`stage` prepare an edit to a shipped conversation module and say offline
//! whether the recompile path could carry it back. They stop at the compiler's door: producing
//! the mini-cache is `gore as compile-module`, and shipping it is `gore mod`.
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
    /// Scaffold a new dialog option for one NPC: the AngelScript source and a build spec
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
        /// Mod name, used for the module, the file path, and the bundle
        #[arg(long, default_value = "MyDialogMod")]
        mod_name: String,
        /// Output directory for the source and the build spec
        #[arg(short = 'o', long)]
        out: PathBuf,
        #[arg(long)]
        cache: Option<PathBuf>,
        #[arg(long)]
        game: Option<PathBuf>,
    },
    /// Take one conversation's AngelScript out of the cache so its bodies can be rewritten
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
    /// Check an edited conversation against what the recompile path can carry back
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
            mod_name,
            out,
            cache,
            game,
        } => new_topic(NewTopicRequest {
            npc,
            caption,
            caption_key,
            class,
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
    println!("you may change: what a method does — spoken lines, effects, their order and");
    println!("                branches, the `IsVisible` test, and which existing topics a");
    println!("                `Subdialog` offers");
    println!("you may not:    add, remove, rename or reorder classes and methods; change a");
    println!("                signature or a member variable; write a `default` statement;");
    println!("                name a type or a text id this game build does not already have");
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
    let report = dialog::verify(&taken.source, &authored, &known);
    Ok((manifest, report, source_path))
}

fn check(dir: &PathBuf, json: bool, cache: Option<PathBuf>, game: Option<PathBuf>) -> Result<()> {
    let (manifest, report, source_path) = open_edit(dir, cache, game)?;

    if json {
        let document = serde_json::json!({
            "module": manifest.module,
            "participant": manifest.participant,
            "unchanged": report.unchanged,
            "carryable": report.is_carryable(),
            "changed": report.changed.iter().map(|body| {
                serde_json::json!({ "class": body.class, "member": body.member })
            }).collect::<Vec<_>>(),
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
        println!("this edit cannot be carried back:");
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

    println!("this edit can be carried back. Rewritten:");
    for body in &report.changed {
        println!("  - {}::{}", body.class, body.member);
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

    let mini = format!("{mod_name}.mini.Cache");
    let spec = serde_json::json!({
        "meta": { "name": mod_name, "version": "0.1.0", "author": "" },
        "scripts": [{
            "op": "edit",
            "module_name": manifest.module,
            "mini_cache": mini,
        }],
    });
    let spec_path = dir.join("spec.json");
    fs::write(
        &spec_path,
        format!("{}\n", serde_json::to_string_pretty(&spec)?),
    )
    .with_context(|| format!("writing {}", spec_path.display()))?;

    println!("wrote {}", spec_path.display());
    println!();
    println!("rewritten:");
    for body in &report.changed {
        println!("  - {}::{}", body.class, body.member);
    }
    println!();
    println!("next:");
    println!(
        "  gore as compile-module --op edit --module {} --rel-path {} \\\n\
         \x20   --source {} --work-dir .gore-as-work -o {}",
        manifest.module,
        manifest.relative_path,
        source_path.display(),
        dir.join(&mini).display()
    );
    println!("  gore mod build --spec {} -o build", spec_path.display());
    println!("  gore mod deploy --bundle build/{mod_name}");
    println!();
    println!("no `--allow-new-symbols`: an edited module is remapped strictly onto this exact");
    println!("cache, which is what lets its captions and rules come back unchanged");
    Ok(())
}

// ─── new-topic ───────────────────────────────────────────────────────────────

pub struct NewTopicRequest {
    pub npc: String,
    pub caption: Option<String>,
    pub caption_key: Option<String>,
    pub class: Option<String>,
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

fn new_topic(request: NewTopicRequest) -> Result<()> {
    let (_, bytes) = read_cache(request.cache, request.game)?;
    let graph = dialog::build(&bytes).context("reading dialog from the script cache")?;
    let conversation = resolve_one(&graph, &request.npc)?;

    let Some(root_class) = conversation.root_class.clone() else {
        bail!(
            "{} declares no dialog topics, so there is no base class to derive from",
            participant_label(conversation)
        );
    };
    let Some(sentinel) = sentinel_of(conversation) else {
        bail!(
            "{} has no root option to use as the registration sentinel",
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
    let declared = declared_classes(&bytes)?;
    if declared.contains(&class.to_lowercase()) {
        bail!("the cache already declares a class called {class:?}. Pass a different --class");
    }

    let module = format!("{}.Dialog", request.mod_name);
    let rel_path = format!("{}/Dialog.as", request.mod_name);
    let helper = format!("{}Caption", class.trim_start_matches('U'));
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

    let source = format!(
        "// Generated by `gore dialog new-topic` for {participant}.\n\
         //\n\
         // The class derives from the conversation's own topic base, so the engine treats it as\n\
         // one of that NPC's options. The body below is the shape with runtime evidence behind\n\
         // it: a caption, an always-visible option, and an act that ends the conversation.\n\
         // Spoken lines, conditions and effects are yours to add — see\n\
         // `gore dialog show <topic>` for how the game writes them.\n\
         \n\
         {helper_block}class {class} : {root_class}\n\
         {{\n\
         {caption_line}\n\
         \x20   default PriorityRank = 2;\n\
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

    let spec = serde_json::json!({
        "meta": {
            "name": request.mod_name,
            "version": "0.1.0",
            "author": "",
        },
        "scripts": [{
            "op": "add",
            "module_name": module,
            "mini_cache": format!("{module}.mini.Cache"),
        }],
        "dialog_topics": [{
            "id": slug.to_lowercase(),
            "participant_name": participant.to_lowercase(),
            "topic_class": format!("/Script/Angelscript.{}", class.trim_start_matches('U')),
            "sentinel_class": format!(
                "/Script/Angelscript.{}",
                sentinel.class.trim_start_matches('U')
            ),
        }],
    });

    let source_path = request.out.join(&request.mod_name).join("Dialog.as");
    if let Some(parent) = source_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(&source_path, source)
        .with_context(|| format!("writing {}", source_path.display()))?;
    let spec_path = request.out.join("spec.json");
    fs::write(
        &spec_path,
        format!("{}\n", serde_json::to_string_pretty(&spec)?),
    )
    .with_context(|| format!("writing {}", spec_path.display()))?;

    if let Some(text) = &request.caption {
        if !text.is_ascii() {
            println!(
                "note: {text:?} is an untranslated literal with non-ASCII characters, which no                  GORE run has compiled yet. --caption-key with a real localization id is the                  route with evidence behind it."
            );
        }
    }
    println!("wrote {}", source_path.display());
    println!("wrote {}", spec_path.display());
    println!();
    println!("class     {class} : {root_class}");
    println!("sentinel  {}", sentinel.class);
    println!("next:");
    println!(
        "  gore as compile-module --op add --module {module} --rel-path {rel_path} \\\n\
         \x20   --source {} --work-dir .gore-as-work --allow-new-symbols \\\n\
         \x20   -o {}",
        source_path.display(),
        request.out.join(format!("{module}.mini.Cache")).display()
    );
    println!("  gore mod build --spec {} -o build", spec_path.display());
    println!("  gore mod deploy --bundle build/{}", request.mod_name);
    println!();
    println!(
        "the compile step drives the game's own compiler, so it needs the game installed and \
         takes a couple of minutes"
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
}
