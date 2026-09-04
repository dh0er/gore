//! Die Klassenkette einer Figur auflösen.
//!
//! `USpawnAIAgentDefinition_<ID>` zeigt über `AIAgentConfigClass` auf `UAIAgentConfig_Human_<ID>`,
//! das über `m_CharacterDefinition` auf `UCharacterDefinition_Human_<ID>`. Die Verweise stehen als
//! `default`-Zuweisungen im emittierten Quelltext; die Feldnamen sind dieselben Konstanten, die
//! der versiegelte Extraktor in `gore-as` benutzt.

use std::collections::BTreeMap;

use super::defaults::{static_class_target, EmittedClass};

pub const SPAWN_AI_FIELD: &str = "AIAgentConfigClass";
pub const AI_CHARACTER_FIELD: &str = "m_CharacterDefinition";
pub const UNIQUE_NAME_FIELD: &str = "m_UniqueName";

/// Was von einer Figur aufgelöst werden konnte. Jedes Glied ist einzeln optional, damit eine
/// unvollständige Kette berichtet statt verschwiegen wird.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NpcChain {
    pub spawn_definition: Option<String>,
    pub ai_agent_config: Option<String>,
    pub character_definition: Option<String>,
    pub guild_base: Option<String>,
    pub unique_name: Option<String>,
}

/// Die Kette ab `spawn_class`, aufgelöst über einen Index aller emittierten Klassen.
pub fn resolve(index: &BTreeMap<String, EmittedClass>, spawn_class: &str) -> NpcChain {
    let mut chain = NpcChain {
        spawn_definition: Some(spawn_class.to_string()),
        ..NpcChain::default()
    };
    let Some(spawn) = index.get(spawn_class) else {
        return chain;
    };
    chain.ai_agent_config = assigned(spawn, SPAWN_AI_FIELD)
        .and_then(static_class_target)
        .map(str::to_string);

    let Some(config) = chain
        .ai_agent_config
        .as_deref()
        .and_then(|name| index.get(name))
    else {
        return chain;
    };
    chain.character_definition = assigned(config, AI_CHARACTER_FIELD)
        .and_then(static_class_target)
        .map(str::to_string);

    let Some(definition) = chain
        .character_definition
        .as_deref()
        .and_then(|name| index.get(name))
    else {
        return chain;
    };
    chain.guild_base = definition.super_class.clone();
    chain.unique_name = assigned(definition, UNIQUE_NAME_FIELD)
        .map(|raw| raw.trim_start_matches('n').trim_matches('"').to_string());
    chain
}

/// Der Wert einer `default`-Zuweisung, wenn die Klasse sie trägt.
pub fn assigned<'a>(class: &'a EmittedClass, field: &str) -> Option<&'a str> {
    class
        .assignments
        .iter()
        .find(|(lhs, _)| lhs == field)
        .map(|(_, rhs)| rhs.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class(name: &str, super_class: &str, assignments: &[(&str, &str)]) -> EmittedClass {
        EmittedClass {
            name: name.to_string(),
            super_class: Some(super_class.to_string()),
            assignments: assignments
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect(),
            calls: Vec::new(),
        }
    }

    fn diego() -> BTreeMap<String, EmittedClass> {
        let mut index = BTreeMap::new();
        for entry in [
            class(
                "USpawnAIAgentDefinition_OC_STT_Diego",
                "USpawnAIAgentDefinition",
                &[(
                    "AIAgentConfigClass",
                    "UAIAgentConfig_Human_OC_STT_Diego::StaticClass()",
                )],
            ),
            class(
                "UAIAgentConfig_Human_OC_STT_Diego",
                "UAIAgentConfig_Human",
                &[(
                    "m_CharacterDefinition",
                    "UCharacterDefinition_Human_OC_STT_Diego::StaticClass()",
                )],
            ),
            class(
                "UCharacterDefinition_Human_OC_STT_Diego",
                "UCharacterDefinition_Human_OldCamp_Shadow",
                &[("m_UniqueName", "n\"OC_STT_Diego\"")],
            ),
        ] {
            index.insert(entry.name.clone(), entry);
        }
        index
    }

    #[test]
    fn resolve_walks_spawn_to_config_to_definition() {
        let chain = resolve(&diego(), "USpawnAIAgentDefinition_OC_STT_Diego");
        assert_eq!(
            chain.ai_agent_config.as_deref(),
            Some("UAIAgentConfig_Human_OC_STT_Diego")
        );
        assert_eq!(
            chain.character_definition.as_deref(),
            Some("UCharacterDefinition_Human_OC_STT_Diego")
        );
    }

    #[test]
    fn resolve_reports_the_guild_base_and_the_unique_name() {
        let chain = resolve(&diego(), "USpawnAIAgentDefinition_OC_STT_Diego");
        assert_eq!(
            chain.guild_base.as_deref(),
            Some("UCharacterDefinition_Human_OldCamp_Shadow")
        );
        assert_eq!(chain.unique_name.as_deref(), Some("OC_STT_Diego"));
    }

    #[test]
    fn a_broken_link_leaves_the_rest_none_instead_of_guessing() {
        let mut index = diego();
        index.remove("UAIAgentConfig_Human_OC_STT_Diego");
        let chain = resolve(&index, "USpawnAIAgentDefinition_OC_STT_Diego");
        assert_eq!(
            chain.ai_agent_config.as_deref(),
            Some("UAIAgentConfig_Human_OC_STT_Diego")
        );
        assert_eq!(chain.character_definition, None);
        assert_eq!(chain.unique_name, None);
    }

    #[test]
    fn an_unknown_spawn_class_resolves_to_nothing_but_itself() {
        let chain = resolve(&diego(), "USpawnAIAgentDefinition_Nobody");
        assert_eq!(
            chain.spawn_definition.as_deref(),
            Some("USpawnAIAgentDefinition_Nobody")
        );
        assert_eq!(chain.ai_agent_config, None);
    }

    #[test]
    fn a_definition_without_a_unique_name_still_reports_its_guild() {
        let mut index = diego();
        index
            .get_mut("UCharacterDefinition_Human_OC_STT_Diego")
            .unwrap()
            .assignments
            .clear();
        let chain = resolve(&index, "USpawnAIAgentDefinition_OC_STT_Diego");
        assert_eq!(
            chain.guild_base.as_deref(),
            Some("UCharacterDefinition_Human_OldCamp_Shadow")
        );
        assert_eq!(chain.unique_name, None);
    }
}
