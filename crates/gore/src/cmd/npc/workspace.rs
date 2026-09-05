//! Das Arbeitsverzeichnis einer verfassten Figur: was darin liegt und was das Manifest festhält.
//!
//! Nach dem Vorbild von `gore dialog checkout`: eine verfasste Datei je berührtem Modul, eine
//! unveränderte Pristine-Kopie daneben, ein Manifest dazwischen. Das Manifest bindet die Arbeit
//! an die exakte Cache, aus der sie stammt — `check` weigert sich später, wenn die sich geändert
//! hat, statt gegen eine andere Grundlage zu prüfen.

use serde::{Deserialize, Serialize};

/// Der Dateiname des Manifests im Arbeitsverzeichnis.
pub const MANIFEST_NAME: &str = "gore-npc-edit.json";

/// Was das Kommando mit der Figur vorhat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Operation {
    /// Eine neue Figur, abgeleitet von einer ausgelieferten.
    New,
    /// Eine neue Figur mit den aufgelösten Werten der Vorlage ausgeschrieben.
    Clone,
    /// Eine ausgelieferte Figur zum Bearbeiten herausgenommen.
    Checkout,
    /// Eine ausgelieferte Figur wird nicht mehr gesetzt.
    Suppress,
}

/// Ein berührtes Modul.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleEdit {
    /// Der Modulname, wie ihn die Cache führt.
    pub module: String,
    /// Pfad relativ zum `Script/`-Baum, z.B. `AI/AIAgent/Human/Config/MY_NPC/MY_NPC.as`.
    pub relative_path: String,
    /// Die verfasste Datei im Arbeitsverzeichnis.
    pub source_file: String,
    /// Die unveränderte Kopie, oder `None` für ein neues Modul.
    pub pristine_file: Option<String>,
    /// `add` für ein neues Modul, `edit` für ein ausgeliefertes.
    pub op: String,
}

/// Das Manifest eines Arbeitsverzeichnisses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub operation: Operation,
    pub npc_id: String,
    /// Die Figur, von der abgeleitet wurde. `None` bei `checkout` und `suppress`.
    pub derived_from: Option<String>,
    pub modules: Vec<ModuleEdit>,
    /// Die Weltpunkte, an denen die Figur gesetzt wird bzw. nicht mehr gesetzt wird.
    pub world_points: Vec<String>,
    /// Das Levelskript, das die Spawn-Zeile trägt.
    pub level_module: String,
    /// SHA-256 der Cache, gegen die verfasst wurde, als Kleinbuchstaben-Hex.
    pub cache_sha256: String,
    /// `true`, wenn `--modular-visuals` gesetzt war.
    pub modular_visuals: bool,
}

impl Manifest {
    /// Das neue Modul der Figur, wenn es eines gibt.
    pub fn authored_module(&self) -> Option<&ModuleEdit> {
        self.modules.iter().find(|edit| edit.op == "add")
    }

    /// Das editierte Levelskript.
    pub fn level_edit(&self) -> Option<&ModuleEdit> {
        self.modules
            .iter()
            .find(|edit| edit.module == self.level_module)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        Manifest {
            operation: Operation::New,
            npc_id: "MY_NPC".to_string(),
            derived_from: Some("OC_STT_Diego".to_string()),
            modules: vec![
                ModuleEdit {
                    module: "AI.AIAgent.Human.Config.MY_NPC.MY_NPC".to_string(),
                    relative_path: "AI/AIAgent/Human/Config/MY_NPC/MY_NPC.as".to_string(),
                    source_file: "MY_NPC.as".to_string(),
                    pristine_file: None,
                    op: "add".to_string(),
                },
                ModuleEdit {
                    module: "LevelScripts.Map_x2_y2_ExchangeZone_AI_script".to_string(),
                    relative_path: "LevelScripts/Map_x2_y2_ExchangeZone_AI_script.as".to_string(),
                    source_file: "Map_x2_y2_ExchangeZone_AI_script.as".to_string(),
                    pristine_file: Some("pristine/Map_x2_y2_ExchangeZone_AI_script.as".to_string()),
                    op: "edit".to_string(),
                },
            ],
            world_points: vec!["UWP_EZ_START_DIEGO_SPAWN".to_string()],
            level_module: "LevelScripts.Map_x2_y2_ExchangeZone_AI_script".to_string(),
            cache_sha256: "7a18f954e32af30fc24ae3a66ea35d3b5cb98560c8f5083c7846fc9ce1d77511"
                .to_string(),
            modular_visuals: false,
        }
    }

    #[test]
    fn a_manifest_round_trips_through_json() {
        let json = serde_json::to_string_pretty(&manifest()).expect("serialize");
        let back: Manifest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, manifest());
    }

    #[test]
    fn the_operation_spells_itself_in_kebab_case() {
        let json = serde_json::to_string(&Operation::Suppress).expect("serialize");
        assert_eq!(json, "\"suppress\"");
    }

    #[test]
    fn the_authored_module_is_the_added_one() {
        assert_eq!(
            manifest()
                .authored_module()
                .map(|edit| edit.module.as_str()),
            Some("AI.AIAgent.Human.Config.MY_NPC.MY_NPC")
        );
    }

    #[test]
    fn the_level_edit_is_found_by_the_level_module_name() {
        let manifest = manifest();
        let edit = manifest.level_edit().expect("level edit");
        assert_eq!(edit.op, "edit");
        assert!(edit.pristine_file.is_some());
    }

    #[test]
    fn a_suppression_has_no_authored_module() {
        let mut manifest = manifest();
        manifest.operation = Operation::Suppress;
        manifest.derived_from = None;
        manifest.modules.retain(|edit| edit.op == "edit");
        assert!(manifest.authored_module().is_none());
        assert!(manifest.level_edit().is_some());
    }
}
