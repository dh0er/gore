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

fn rewrite_first_default(
    source: &str,
    select: impl Fn(&str) -> bool,
    rewrite: impl Fn(&str) -> String,
) -> Option<String> {
    for (start, _) in source.match_indices("default ") {
        let end = start + source[start..].find(';')? + 1;
        let statement = &source[start..end];
        if !select(statement) {
            continue;
        }
        let replacement = rewrite(statement);
        if replacement == statement {
            continue;
        }
        let mut edited = source.to_owned();
        edited.replace_range(start..end, &replacement);
        return Some(edited);
    }
    None
}

fn verify_real_default_edit(
    modules: &[dialog::Checkout],
    known: &dialog::KnownNames,
    label: &str,
    select: impl Fn(&str) -> bool,
    rewrite: impl Fn(&str) -> String,
) -> dialog::EditReport {
    let (module, edited) = modules
        .iter()
        .find_map(|module| {
            rewrite_first_default(&module.source, &select, &rewrite).map(|edited| (module, edited))
        })
        .unwrap_or_else(|| panic!("the shipping dialog corpus has no {label} default to edit"));
    let report = dialog::verify(module, &edited, known);
    assert!(
        report.is_carryable(),
        "{label} edit in {} was refused: {:?}",
        module.module,
        report.violations
    );
    assert!(
        !report.changed_defaults.is_empty(),
        "{label} edit in {} was not reported as a changed default",
        module.module
    );
    report
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

/// A checkout nobody has touched must contain complete authored defaults and check cleanly, for
/// every conversation the game ships.
///
/// `checkout_many` already refuses when a cache class with `__InitDefaults` is absent from the
/// emitted coverage. This test independently parses that emitted source through the public dialog
/// outline and requires its authored-default classes to agree with the checkout metadata. It also
/// exercises the checker against every source shape the shipping corpus puts in front of it.
#[test]
fn every_conversation_checkout_has_authored_defaults_and_checks_cleanly() {
    let Some(bytes) = real_cache() else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let graph = dialog::build(&bytes).expect("dialog");
    let known = dialog::known_names(&bytes).expect("known names");

    let names: Vec<&str> = graph
        .conversations
        .iter()
        .map(|conversation| conversation.module.as_str())
        .collect();
    let taken = dialog::checkout_many(&bytes, &names, None).expect("checkout");

    let mut checked = 0usize;
    let mut complaints = Vec::new();
    for module in &taken {
        let outline = dialog::read_outline(&module.source)
            .unwrap_or_else(|error| panic!("{} checkout is invalid: {error}", module.module));
        let outlined_default_classes = outline
            .classes
            .iter()
            .filter(|class| !class.defaults.is_empty())
            .map(|class| class.name.clone())
            .collect::<BTreeSet<_>>();
        if module.default_classes.is_empty() {
            complaints.push(format!(
                "{}: checkout carries no authored-default classes",
                module.module
            ));
        }
        if outlined_default_classes != module.default_classes {
            complaints.push(format!(
                "{}: outline sees {:?}, checkout records {:?}",
                module.module, outlined_default_classes, module.default_classes
            ));
        }

        let report = dialog::verify(module, &module.source, &known);
        checked += 1;
        if !report.unchanged {
            complaints.push(format!("{}: a checkout differs from itself", module.module));
        }
        for violation in &report.violations {
            complaints.push(format!("{}: {}", module.module, violation.explain()));
        }
        if !report.changed.is_empty() {
            complaints.push(format!(
                "{}: {} method(s) reported as rewritten in an untouched checkout",
                module.module,
                report.changed.len()
            ));
        }
    }

    eprintln!("{checked} conversation modules checked out and verified");
    assert!(checked > 200, "only {checked} modules were checked");
    assert!(
        complaints.is_empty(),
        "{} complaint(s) about untouched checkouts:\n{}",
        complaints.len(),
        complaints
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );

    let caption = verify_real_default_edit(
        &taken,
        &known,
        "Caption",
        |statement| statement.starts_with("default Caption "),
        |_| "default Caption = LocText(\"GORE_DIALOG_REAL_CACHE_ORACLE_CAPTION\");".to_owned(),
    );
    assert!(caption
        .changed_defaults
        .iter()
        .any(|change| change.target == "Caption"));
    assert!(caption
        .new_strings
        .iter()
        .any(|value| value == "GORE_DIALOG_REAL_CACHE_ORACLE_CAPTION"));
    assert!(caption.requires_new_symbols());

    // The shipping corpus currently carries no authored PriorityRank statement. Setting its
    // previously implicit value is therefore an added default on an existing topic, not a
    // replacement; all shipped defaults in that class must still remain present.
    let priority = verify_real_default_edit(
        &taken,
        &known,
        "PriorityRank",
        |statement| statement.starts_with("default Caption "),
        |statement| format!("{statement}\ndefault PriorityRank = 31415;"),
    );
    assert!(priority
        .changed_defaults
        .iter()
        .any(|change| change.target == "PriorityRank"));

    let rules = verify_real_default_edit(
        &taken,
        &known,
        "Rules",
        |statement| statement.starts_with("default Rules."),
        |statement| format!("{statement}\n{statement}"),
    );
    assert!(rules
        .changed_defaults
        .iter()
        .any(|change| change.target == "Rules"));

    let flag = verify_real_default_edit(
        &taken,
        &known,
        "dialog flag",
        |statement| {
            [
                "default bIsSubTopic ",
                "default bIsAmbientTopic ",
                "default bIsFollowupTopic ",
            ]
            .iter()
            .any(|prefix| statement.starts_with(prefix))
        },
        |statement| {
            if statement.contains("true") {
                statement.replacen("true", "false", 1)
            } else {
                statement.replacen("false", "true", 1)
            }
        },
    );
    assert!(flag.changed_defaults.iter().any(|change| matches!(
        change.target.as_str(),
        "bIsSubTopic" | "bIsAmbientTopic" | "bIsFollowupTopic"
    )));

    let (subdialog_module, edited_subdialog) = taken
        .iter()
        .find_map(|module| {
            let position = module.source.find("Subdialog(this,")?;
            let mut edited = module.source.clone();
            edited.replace_range(
                position..position + "Subdialog(this,".len(),
                "Subdialog((this),",
            );
            Some((module, edited))
        })
        .expect("the shipping dialog corpus has no Subdialog call");
    assert_eq!(
        edited_subdialog.replace("Subdialog((this),", "Subdialog(this,"),
        subdialog_module.source,
        "the body oracle must retain the exact shipped Subdialog call"
    );
    let report = dialog::verify(subdialog_module, &edited_subdialog, &known);
    assert!(
        report.is_carryable(),
        "body-only Subdialog edit in {} was refused: {:?}",
        subdialog_module.module,
        report.violations
    );
    assert!(
        !report.changed.is_empty(),
        "body-only Subdialog edit in {} was not reported",
        subdialog_module.module
    );
}

/// Every topic class the tree reports is a class an edited module may name, and every
/// localization key it prints is a literal such a module may use. Without that, the checker would
/// refuse edits that only reuse what the conversation already says.
#[test]
fn the_names_a_tree_prints_are_names_an_edit_may_use() {
    let Some(bytes) = real_cache() else {
        eprintln!("skip: set GORE_AS_REAL_CACHE");
        return;
    };
    let graph = dialog::build(&bytes).expect("dialog");
    let known = dialog::known_names(&bytes).expect("known names");

    let mut missing_types = 0usize;
    let mut missing_strings = 0usize;
    for conversation in &graph.conversations {
        for topic in &conversation.topics {
            if !known.types.contains(&topic.class) {
                missing_types += 1;
            }
            if let Some(key) = topic.caption.loc_key() {
                if !known.strings.contains(key) {
                    missing_strings += 1;
                }
            }
            for step in &topic.act {
                if let StepKind::Say {
                    loc_key: Some(key), ..
                } = &step.kind
                {
                    if !known.strings.contains(key) {
                        missing_strings += 1;
                    }
                }
            }
        }
    }
    assert_eq!(missing_types, 0, "topic classes absent from the name index");
    assert_eq!(
        missing_strings, 0,
        "localization keys absent from the string index"
    );
}
