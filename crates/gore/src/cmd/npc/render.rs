//! Ausgabe für die NPC-Lesekommandos: eine Zeile je Treffer, oder ein JSON-Dokument.

use gore_as::cache::faithfulness;

use super::{chain::NpcChain, Emitted};
use crate::cmd::find::CatalogEntry;

/// Ein Katalogeintrag als Listenzeile: Kategorie linksbündig auf 8, dann Id, dann Klasse.
pub fn list_line(entry: &CatalogEntry) -> String {
    let class = entry.class.as_deref().unwrap_or("—");
    format!("{:<8} {}  {}", entry.category, entry.id, class)
}

/// The resolved chain as a block of lines, one link per line.
pub fn chain_block(npc: &str, chain: &NpcChain) -> String {
    let link = |label: &str, value: &Option<String>| {
        format!("  {label:<22} {}\n", value.as_deref().unwrap_or("—"))
    };
    let mut out = format!("{npc}\n");
    out.push_str(&link("spawn definition", &chain.spawn_definition));
    out.push_str(&link("ai agent config", &chain.ai_agent_config));
    out.push_str(&link("character definition", &chain.character_definition));
    out.push_str(&link("guild base", &chain.guild_base));
    out.push_str(&link("unique name", &chain.unique_name));
    out
}

/// What is known about recompiling `module`, said so a reader cannot mistake silence for safety.
pub fn translation_line(emitted: &Emitted, module: &str) -> String {
    match faithfulness::for_module(&emitted.cache_seal, emitted.binds_seal.as_ref(), module) {
        None => "translation: NOT MEASURED for this game version".to_string(),
        Some(known) if known.divergent_functions == 0 => {
            "translation: measured, no known difference".to_string()
        }
        Some(known) => format!(
            "translation: {} divergent function(s), {} behaviour risk(s)",
            known.divergent_functions, known.behaviour_risks
        ),
    }
}

/// The same judgement as a JSON object, so a caller does not have to parse the sentence.
pub fn translation_json(emitted: &Emitted, module: &str) -> serde_json::Value {
    match faithfulness::for_module(&emitted.cache_seal, emitted.binds_seal.as_ref(), module) {
        None => serde_json::json!({ "measured": false }),
        Some(known) => serde_json::json!({
            "measured": true,
            "divergent_functions": known.divergent_functions,
            "behaviour_risks": known.behaviour_risks,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn npc(id: &str, category: &str, class: &str) -> CatalogEntry {
        CatalogEntry {
            domain: "npc",
            id: id.to_string(),
            category: category.to_string(),
            class: Some(class.to_string()),
            module: None,
            loc_key: None,
            caption: None,
        }
    }

    #[test]
    fn list_line_pads_the_category_and_keeps_id_and_class() {
        let line = list_line(&npc(
            "OC_STT_Diego",
            "human",
            "CharacterDefinition_Human_OC_STT_Diego",
        ));
        assert_eq!(
            line,
            "human    OC_STT_Diego  CharacterDefinition_Human_OC_STT_Diego"
        );
    }

    #[test]
    fn list_line_survives_a_row_without_a_class() {
        let mut entry = npc("OC_STT_Diego", "human", "x");
        entry.class = None;
        assert_eq!(list_line(&entry), "human    OC_STT_Diego  —");
    }

    #[test]
    fn chain_block_prints_every_link_it_resolved() {
        let chain = NpcChain {
            spawn_definition: Some("USpawnAIAgentDefinition_OC_STT_Diego".to_string()),
            ai_agent_config: Some("UAIAgentConfig_Human_OC_STT_Diego".to_string()),
            character_definition: Some("UCharacterDefinition_Human_OC_STT_Diego".to_string()),
            guild_base: Some("UCharacterDefinition_Human_OldCamp_Shadow".to_string()),
            unique_name: Some("OC_STT_Diego".to_string()),
        };
        let block = chain_block("OC_STT_Diego", &chain);
        let lines: Vec<&str> = block.lines().collect();
        assert_eq!(lines[0], "OC_STT_Diego");
        assert_eq!(
            lines[1],
            "  spawn definition       USpawnAIAgentDefinition_OC_STT_Diego"
        );
        assert_eq!(lines.len(), 6);
        assert!(block.ends_with('\n'));
    }

    #[test]
    fn chain_block_marks_an_unresolved_link_instead_of_dropping_it() {
        // Eine halb aufgelöste Kette muss ihre Lücke zeigen: ein weggelassenes Glied sähe aus
        // wie eine Figur ohne Gilde, statt wie eine, die hier nicht aufgelöst werden konnte.
        let chain = NpcChain {
            spawn_definition: Some("USpawnAIAgentDefinition_Nobody".to_string()),
            ..NpcChain::default()
        };
        let lines: Vec<String> = chain_block("Nobody", &chain)
            .lines()
            .map(str::to_string)
            .collect();
        assert_eq!(lines.len(), 6);
        assert_eq!(lines[2], "  ai agent config        —");
        assert_eq!(lines[3], "  character definition   —");
        assert_eq!(lines[4], "  guild base             —");
        assert_eq!(lines[5], "  unique name            —");
    }
}
