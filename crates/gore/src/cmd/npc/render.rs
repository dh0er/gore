//! Ausgabe für die NPC-Lesekommandos: eine Zeile je Treffer, oder ein JSON-Dokument.

use crate::cmd::find::CatalogEntry;

/// Ein Katalogeintrag als Listenzeile: Kategorie linksbündig auf 8, dann Id, dann Klasse.
pub fn list_line(entry: &CatalogEntry) -> String {
    let class = entry.class.as_deref().unwrap_or("—");
    format!("{:<8} {}  {}", entry.category, entry.id, class)
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
}
