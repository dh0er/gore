//! Assembly of per-module dialog trees from the shipping script cache.

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use super::super::disasm::disassemble;
use super::super::model::{parse_modules, Class, Func, Module};
use super::super::refs::RefResolver;
use super::extract;
use super::model::{Conversation, Coverage, DialogGraph, StepKind, Topic, Visibility};

/// The base class every dialog topic ultimately derives from.
const TOPIC_BASE: &str = "UG1RDialogTopic";

/// The per-conversation settings class, which exists even where no topic does.
const SETTINGS_BASE: &str = "UConversationCharacterSettings";

#[derive(Debug, Error)]
pub enum DialogError {
    #[error("could not parse script cache: {0}")]
    Parse(String),
    #[error("could not disassemble {module}.{class}::{function}: {detail}")]
    Disassemble {
        module: String,
        class: String,
        function: String,
        detail: String,
    },
}

/// Read every conversation the cache declares.
pub fn build(cache: &[u8]) -> Result<DialogGraph, DialogError> {
    let modules = parse_modules(cache).map_err(|error| DialogError::Parse(error.to_string()))?;
    let refs = RefResolver::build(cache).map_err(|error| DialogError::Parse(error.to_string()))?;

    let mut conversations = Vec::new();
    for module in &modules {
        if let Some(conversation) = conversation(module, &refs)? {
            conversations.push(conversation);
        }
    }
    conversations.sort_by(|left, right| left.module.cmp(&right.module));
    Ok(DialogGraph { conversations })
}

/// The class that roots this module's topic hierarchy, if it has one.
fn root_class(module: &Module) -> Option<&Class> {
    module
        .classes
        .iter()
        .find(|class| class.super_class.as_deref() == Some(TOPIC_BASE))
}

/// Participants named by a root class such as `UTopic_Hero__OM_STT_VIPER_302`.
fn participants(root: &str) -> Vec<String> {
    let body = root
        .strip_prefix("UTopic_")
        .or_else(|| root.strip_prefix("Topic_"))
        .unwrap_or(root);
    body.split("__")
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Whether a module is one of the game's conversation modules by name. Only the topicless case
/// needs this: a module that declares a topic root class is a conversation whatever it is called,
/// while `UConversationCharacterSettings` subclasses also live in the per-NPC AI config modules,
/// which are not conversations.
fn is_conversation_module(module: &str) -> bool {
    module
        .rsplit('.')
        .next()
        .is_some_and(|leaf| leaf.starts_with("Conversation_"))
}

/// Participants from a module named `...Conversation.Conversation_<A>[__<B>]`.
fn participants_from_module(module: &str) -> Vec<String> {
    let leaf = module.rsplit('.').next().unwrap_or(module);
    let body = leaf.strip_prefix("Conversation_").unwrap_or(leaf);
    body.split("__")
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

fn method<'a>(class: &'a Class, name: &str) -> Option<&'a Func> {
    class.methods.iter().find(|method| method.name == name)
}

fn instructions(
    module: &Module,
    class: &Class,
    function: &Func,
) -> Result<Vec<super::super::disasm::Instr>, DialogError> {
    disassemble(&function.bytecode).map_err(|error| DialogError::Disassemble {
        module: module.name.clone(),
        class: class.name.clone(),
        function: function.name.clone(),
        detail: error.to_string(),
    })
}

/// Root menu order is rank first and module declaration order for equal ranks. The cache's class
/// array is the authored order the game preserves; class-name sorting is only for the public
/// `topics` inventory and must not become a menu-order tie-breaker.
fn ordered_root_classes(topics: &[Topic], children_seen: &BTreeSet<String>) -> Vec<String> {
    let mut ordered: Vec<&Topic> = topics
        .iter()
        .filter(|topic| !children_seen.contains(&topic.class))
        .collect();
    ordered.sort_by_key(|topic| topic.priority.unwrap_or(0));
    ordered
        .into_iter()
        .map(|topic| topic.class.clone())
        .collect()
}

/// The conversation settings class a module declares, if any.
fn settings_class(module: &Module) -> Option<&Class> {
    module
        .classes
        .iter()
        .find(|class| class.super_class.as_deref() == Some(SETTINGS_BASE))
}

/// Read one module into a conversation, or `None` when it is not a conversation at all.
///
/// A handful of shipped conversations declare character settings and no topics. They are kept,
/// with no topics, because "this NPC has an empty conversation" is an answer and "no such NPC"
/// is not.
fn conversation(module: &Module, refs: &RefResolver) -> Result<Option<Conversation>, DialogError> {
    let root = root_class(module);
    if root.is_none() && !(is_conversation_module(&module.name) && settings_class(module).is_some())
    {
        return Ok(None);
    }

    // Topics are the classes below the root, at any depth within this module.
    let mut is_topic: BTreeMap<&str, bool> = BTreeMap::new();
    if let Some(root) = root {
        is_topic.insert(root.name.as_str(), false);
    }
    let mut changed = true;
    while changed {
        changed = false;
        for class in &module.classes {
            if is_topic.contains_key(class.name.as_str()) {
                continue;
            }
            let Some(parent) = class.super_class.as_deref() else {
                continue;
            };
            if is_topic.contains_key(parent) {
                is_topic.insert(class.name.as_str(), true);
                changed = true;
            }
        }
    }

    let mut coverage = Coverage::default();
    let mut topics = Vec::new();
    let mut children_seen: BTreeSet<String> = BTreeSet::new();

    for class in &module.classes {
        if !is_topic.get(class.name.as_str()).copied().unwrap_or(false) {
            continue;
        }

        let mut defaults = extract::Defaults::default();
        if let Some(initializer) = method(class, "__InitDefaults") {
            let decoded = instructions(module, class, initializer)?;
            defaults = extract::defaults(&decoded, refs);
        }
        coverage.calls_suppressed += defaults.suppressed;
        coverage.calls_unresolved += defaults.unresolved;

        let visibility = match method(class, "IsVisible_Implementation") {
            Some(function) => {
                let decoded = instructions(module, class, function)?;
                Visibility::Scripted {
                    checks: extract::visibility(&decoded, refs),
                }
            }
            None => Visibility::Always,
        };

        let mut act = Vec::new();
        if let Some(function) = method(class, "Act_Implementation") {
            let decoded = instructions(module, class, function)?;
            let read = extract::act(&decoded, refs);
            coverage.calls_suppressed += read.suppressed;
            coverage.calls_unresolved += read.unresolved;
            coverage.says_incomplete += read.says_incomplete;
            act = read.steps;
        }

        for step in &act {
            coverage.steps += 1;
            if !matches!(step.kind, StepKind::Call { .. }) {
                coverage.steps_typed += 1;
            }
            if let StepKind::Subdialog { children } = &step.kind {
                children_seen.extend(children.iter().cloned());
            }
        }

        coverage.topics += 1;
        if defaults.flags.is_ambient {
            coverage.ambient_topics += 1;
        } else if matches!(defaults.caption, super::model::Caption::Unresolved) {
            coverage.topics_without_caption += 1;
        }

        topics.push(Topic {
            class: class.name.clone(),
            super_class: class.super_class.clone(),
            caption: defaults.caption,
            priority: defaults.priority,
            flags: defaults.flags,
            rules: defaults.rules,
            settings: defaults.settings,
            visibility,
            act,
        });
    }

    let declared: BTreeSet<&str> = topics.iter().map(|topic| topic.class.as_str()).collect();
    coverage.dangling_children = children_seen
        .iter()
        .filter(|child| !declared.contains(child.as_str()))
        .count();
    coverage.sub_topic_flag_disagreements = topics
        .iter()
        .filter(|topic| topic.flags.is_sub_topic != children_seen.contains(&topic.class))
        .count();

    // Menu order follows declared rank; equal ranks retain module declaration order.
    let roots = ordered_root_classes(&topics, &children_seen);

    // Keep the complete inventory deterministic without changing the independently captured menu
    // order above.
    topics.sort_by(|left, right| left.class.cmp(&right.class));

    let participants = match root {
        Some(root) => participants(&root.name),
        // Without a topic root the module name is the only identity left, and it carries the
        // same participant: `Conversation_SC_NOV_NOVICE12_1319`.
        None => participants_from_module(&module.name),
    };

    Ok(Some(Conversation {
        module: module.name.clone(),
        root_class: root.map(|class| class.name.clone()),
        participants,
        topics,
        roots,
        coverage,
    }))
}

#[cfg(test)]
mod tests {
    use super::super::model::Caption;
    use super::*;

    fn topic(class: &str, priority: Option<i64>) -> Topic {
        Topic {
            class: class.to_owned(),
            super_class: None,
            caption: Caption::Unresolved,
            priority,
            flags: Default::default(),
            rules: Vec::new(),
            settings: Vec::new(),
            visibility: Visibility::Always,
            act: Vec::new(),
        }
    }

    #[test]
    fn equal_rank_roots_keep_module_declaration_order() {
        let topics = vec![
            topic("UImplicit", None),
            topic("UForced", Some(-1)),
            topic("UZebra", Some(0)),
            topic("UChild", Some(0)),
            topic("UAlpha", None),
            topic("UEnd", Some(5)),
        ];
        let children = BTreeSet::from(["UChild".to_owned()]);

        assert_eq!(
            ordered_root_classes(&topics, &children),
            vec![
                "UForced".to_owned(),
                "UImplicit".to_owned(),
                "UZebra".to_owned(),
                "UAlpha".to_owned(),
                "UEnd".to_owned()
            ]
        );
    }

    #[test]
    fn participants_come_from_the_root_class_name() {
        assert_eq!(
            participants("UTopic_Hero__OM_STT_VIPER_302"),
            vec!["Hero".to_owned(), "OM_STT_VIPER_302".to_owned()]
        );
        assert_eq!(
            participants("UTopic_SC_GUR_CORKALOM_1201__SC_GUR_CORANGAR_1202"),
            vec![
                "SC_GUR_CORKALOM_1201".to_owned(),
                "SC_GUR_CORANGAR_1202".to_owned()
            ]
        );
    }

    #[test]
    fn an_unprefixed_root_name_still_yields_a_participant() {
        assert_eq!(participants("Whatever"), vec!["Whatever".to_owned()]);
    }

    #[test]
    fn only_conversation_modules_count_without_a_topic_root() {
        assert!(is_conversation_module(
            "Story.G1R.Conversation.Conversation_SC_NOV_NOVICE12_1319"
        ));
        assert!(!is_conversation_module(
            "AI.AIAgent.Human.Config.SC_NOV_Novice12_1319.ConversationCharacterSettings_SC_NOV_Novice12_1319"
        ));
    }

    #[test]
    fn a_topicless_conversation_takes_its_participant_from_the_module_name() {
        assert_eq!(
            participants_from_module("Story.G1R.Conversation.Conversation_SC_NOV_NOVICE12_1319"),
            vec!["SC_NOV_NOVICE12_1319".to_owned()]
        );
    }
}
