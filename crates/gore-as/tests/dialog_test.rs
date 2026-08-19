//! Oracles for the dialog reader, run against a real shipping cache.
//!
//! These are agreement tests, not fixtures: the shipping cache is the only place the game's
//! dialog exists, and a fixture small enough to commit would only prove that the extractor
//! reproduces a fixture. Set `GORE_AS_REAL_CACHE` to a `PrecompiledScript_Shipping.Cache` to run
//! them; without it each one skips.

use std::collections::{BTreeMap, BTreeSet};

use gore_as::cache::dialog::{self, Caption, DialogGraph, StepKind};
use gore_as::cache::knowledge_metadata::extract_knowledge_metadata;

fn real_cache() -> Option<Vec<u8>> {
    let path = std::env::var("GORE_AS_REAL_CACHE").ok()?;
    match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(error) => panic!("GORE_AS_REAL_CACHE={path} could not be read: {error}"),
    }
}

fn graph() -> Option<DialogGraph> {
    let bytes = real_cache()?;
    Some(dialog::build(&bytes).expect("the real cache should read as dialog"))
}

/// The caption a topic declares must be the one the independent knowledge extractor finds. That
/// extractor feeds the shipped knowledge catalog and was written against the same bytecode from
/// the other direction, so agreement across thousands of classes is real evidence.
#[test]
fn captions_agree_with_the_knowledge_extractor() {
    let Some(bytes) = real_cache() else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let graph = dialog::build(&bytes).expect("dialog");
    let known: BTreeMap<String, (Option<String>, Option<String>)> =
        extract_knowledge_metadata(&bytes)
            .expect("knowledge metadata")
            .into_iter()
            .map(|row| (row.id, (row.loc_key, row.caption)))
            .collect();

    let (mut compared, mut disagreed) = (0usize, 0usize);
    let mut examples = Vec::new();
    for conversation in &graph.conversations {
        for topic in &conversation.topics {
            let id = topic.class.trim_start_matches('U');
            let Some((loc_key, literal)) = known.get(id) else {
                continue;
            };
            compared += 1;
            let agrees = match &topic.caption {
                Caption::LocKey { key } => loc_key.as_deref() == Some(key.as_str()),
                Caption::Literal { text } => literal.as_deref() == Some(text.as_str()),
                Caption::Unresolved => loc_key.is_none() && literal.is_none(),
            };
            if !agrees {
                disagreed += 1;
                if examples.len() < 10 {
                    examples.push(format!(
                        "{id}: dialog {:?} vs knowledge {:?}/{:?}",
                        topic.caption, loc_key, literal
                    ));
                }
            }
        }
    }

    assert!(compared > 2_000, "only {compared} captions were comparable");
    assert_eq!(
        disagreed,
        0,
        "{disagreed} of {compared} captions disagree:\n{}",
        examples.join("\n")
    );
}

/// Every sub-menu names topics its own module declares. A child resolved to a class that is not
/// there would mean the argument scan picked up an unrelated class reference.
#[test]
fn every_sub_menu_names_a_topic_of_its_module() {
    let Some(graph) = graph() else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let mut dangling = Vec::new();
    for conversation in &graph.conversations {
        let declared: BTreeSet<&str> = conversation
            .topics
            .iter()
            .map(|topic| topic.class.as_str())
            .collect();
        for topic in &conversation.topics {
            for step in &topic.act {
                if let StepKind::Subdialog { children } = &step.kind {
                    for child in children {
                        if !declared.contains(child.as_str()) {
                            dangling.push(format!("{} -> {child}", topic.class));
                        }
                    }
                }
            }
        }
    }
    assert!(
        dangling.is_empty(),
        "{} sub-menu entries name an undeclared class:\n{}",
        dangling.len(),
        dangling
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// `bIsSubTopic` is the class's own claim about where it belongs, and the `Subdialog` calls are
/// the wiring that puts it there. They are derived from different bytecode by different code, so
/// their agreement is the check that the root/child split is real rather than naming convention.
#[test]
fn sub_topic_flags_agree_with_the_sub_menu_wiring() {
    let Some(graph) = graph() else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let disagreements: usize = graph
        .conversations
        .iter()
        .map(|conversation| conversation.coverage.sub_topic_flag_disagreements)
        .sum();
    let topics: usize = graph
        .conversations
        .iter()
        .map(|conversation| conversation.coverage.topics)
        .sum();
    assert!(topics > 2_000, "only {topics} topics were read");
    assert_eq!(
        disagreements, 0,
        "{disagreements} of {topics} topics disagree with their sub-menu wiring"
    );
}

/// Nothing may be lost silently: every call site resolves to a symbol, every chooseable topic
/// has a caption, and every spoken line has both a speaker and a key.
#[test]
fn nothing_is_read_away_silently() {
    let Some(graph) = graph() else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let mut total = gore_as::cache::dialog::Coverage::default();
    for conversation in &graph.conversations {
        let coverage = &conversation.coverage;
        total.topics += coverage.topics;
        total.topics_without_caption += coverage.topics_without_caption;
        total.ambient_topics += coverage.ambient_topics;
        total.steps += coverage.steps;
        total.steps_typed += coverage.steps_typed;
        total.says_incomplete += coverage.says_incomplete;
        total.calls_unresolved += coverage.calls_unresolved;
        total.calls_suppressed += coverage.calls_suppressed;
        total.dangling_children += coverage.dangling_children;
    }
    eprintln!(
        "{} conversations, {} topics ({} ambient), {} steps ({} typed), {} calls suppressed",
        graph.conversations.len(),
        total.topics,
        total.ambient_topics,
        total.steps,
        total.steps_typed,
        total.calls_suppressed,
    );

    assert_eq!(total.calls_unresolved, 0, "unresolved call sites");
    assert_eq!(
        total.dangling_children, 0,
        "sub-menu entries without a class"
    );
    assert_eq!(
        total.topics_without_caption, 0,
        "chooseable topics without a caption"
    );
    assert_eq!(total.says_incomplete, 0, "lines missing a speaker or a key");
    assert!(
        total.steps_typed * 100 / total.steps.max(1) >= 70,
        "only {} of {} steps carry a typed node",
        total.steps_typed,
        total.steps
    );
}
