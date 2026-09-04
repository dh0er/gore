//! Die Spawn-Stellen der Levelskripte.
//!
//! Jede Figur wird von `UWP_*::OnWorldStart` in die Welt gesetzt. Die Weltpunkt-Actors liegen
//! gekocht im Level und sind nicht erreichbar, die Rümpfe hier sind es. Ein Weltpunkt kann mehr
//! als eine Figur setzen, und dieselbe Figur kann an mehreren Weltpunkten stehen.

/// Eine Spawn-Stelle: welcher Weltpunkt, in welchem Levelskript, für welche Spawn-Definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    pub world_point: String,
    pub module: String,
    pub spawn_definition: String,
}

pub const WORLD_POINT_BASE: &str = "UWorldPointScript";

/// Präfix jeder Spawn-Definition. Das Unterstrich-Zeichen gehört dazu: die nackte Basisklasse
/// `USpawnAIAgentDefinition` steht als Typparameter in `TSubclassOf<…>` und ist keine Definition.
const SPAWN_DEFINITION_PREFIX: &str = "USpawnAIAgentDefinition_";

/// Die Spawn-Definition eines `SpawnAIAgent`-Aufrufs, in beiden ausgelieferten Schreibweisen.
///
/// Die gewöhnliche Form wickelt sie in `TSubclassOf<…>(… ::StaticClass())`. 14 der 1764 Aufrufe
/// im Spiel übergeben stattdessen eine nackte Klassenreferenz — alles Kreaturen in der Alten
/// Feste und der Freien Mine. Wer nur die erste Form liest, verschweigt diese Fundstellen, statt
/// sie zu melden.
fn spawn_definition_in(call: &str) -> Option<&str> {
    if let Some(target) = super::defaults::static_class_target(call) {
        return Some(target);
    }
    let at = call.find(SPAWN_DEFINITION_PREFIX)?;
    let rest = &call[at..];
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    Some(&rest[..end])
}

/// Jede Spawn-Stelle eines emittierten Levelskripts.
pub fn parse_sites(module: &str, source: &str) -> Vec<Site> {
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("class ") {
            current = match rest.split_once(':') {
                Some((name, base)) if base.trim() == WORLD_POINT_BASE => {
                    Some(name.trim().to_string())
                }
                _ => None,
            };
            continue;
        }
        let Some(world_point) = current.as_deref() else {
            continue;
        };
        if !trimmed.contains("SpawnAIAgent(") {
            continue;
        }
        let Some(spawn_definition) = spawn_definition_in(trimmed).map(str::to_string) else {
            continue;
        };
        out.push(Site {
            world_point: world_point.to_string(),
            module: module.to_string(),
            spawn_definition,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = r#"
class UWP_EZ_START_DIEGO_SPAWN : UWorldPointScript
{
    UFUNCTION()
    void OnWorldStart()
    {
        this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_OC_STT_Diego::StaticClass()), nullptr);
        return;
    }
}

class UWP_EZ_TWO : UWorldPointScript
{
    UFUNCTION()
    void OnWorldStart()
    {
        this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_A::StaticClass()), UDailyRoutine_A_Start());
        this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_B::StaticClass()), nullptr);
        return;
    }
}

class UNotAWorldPoint : USomethingElse
{
    default m_X = 1;
}
"#;

    #[test]
    fn parse_sites_finds_one_spawn_per_call() {
        let sites = parse_sites("LevelScripts.Demo", SOURCE);
        assert_eq!(sites.len(), 3);
        assert_eq!(sites[0].world_point, "UWP_EZ_START_DIEGO_SPAWN");
        assert_eq!(sites[0].module, "LevelScripts.Demo");
        assert_eq!(
            sites[0].spawn_definition,
            "USpawnAIAgentDefinition_OC_STT_Diego"
        );
    }

    #[test]
    fn a_world_point_may_set_more_than_one_character() {
        let sites = parse_sites("LevelScripts.Demo", SOURCE);
        let two: Vec<&Site> = sites
            .iter()
            .filter(|s| s.world_point == "UWP_EZ_TWO")
            .collect();
        assert_eq!(two.len(), 2);
        assert_eq!(two[0].spawn_definition, "USpawnAIAgentDefinition_A");
        assert_eq!(two[1].spawn_definition, "USpawnAIAgentDefinition_B");
    }

    #[test]
    fn a_class_that_is_not_a_world_point_contributes_nothing() {
        let sites = parse_sites("LevelScripts.Demo", SOURCE);
        assert!(sites.iter().all(|s| s.world_point != "UNotAWorldPoint"));
    }

    #[test]
    fn a_module_without_spawns_yields_nothing() {
        assert!(parse_sites("LevelScripts.Empty", "class UX : UY\n{\n}\n").is_empty());
    }

    #[test]
    fn a_naked_class_reference_is_a_site_too() {
        // 14 der 1764 Aufrufe im Spiel sehen so aus, alles Kreaturen in der Alten Feste und der
        // Freien Mine. Wörtlich aus `Map_x3_y1_FreeMine_AI_script.as`.
        let source = "class UWP_FM : UWorldPointScript\n{\n    void OnWorldStart()\n    {\n        this.SpawnAIAgent(USpawnAIAgentDefinition_LizardFire_Prime, nullptr);\n    }\n}\n";
        let sites = parse_sites("LevelScripts.Map_x3_y1_FreeMine_AI_script", source);
        assert_eq!(sites.len(), 1);
        assert_eq!(
            sites[0].spawn_definition,
            "USpawnAIAgentDefinition_LizardFire_Prime"
        );
    }

    #[test]
    fn the_generic_type_parameter_is_not_mistaken_for_a_definition() {
        // `TSubclassOf<USpawnAIAgentDefinition>` trägt die nackte Basisklasse. Sie hat keinen
        // Unterstrich-Zusatz und darf nie als Fundstelle durchgehen.
        assert_eq!(
            spawn_definition_in(
                "this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(x), nullptr);"
            ),
            None
        );
    }

    #[test]
    fn a_spawn_call_outside_any_world_point_class_is_ignored() {
        // Freie Funktionen und andere Klassen rufen `SpawnAIAgent` ebenfalls auf; nur ein
        // Weltpunkt ist eine Stelle, an die sich ein Mod hängen kann.
        let source = "class UHelper : UObject\n{\n    void Go()\n    {\n        this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_X::StaticClass()), nullptr);\n    }\n}\n";
        assert!(parse_sites("LevelScripts.Demo", source).is_empty());
    }
}
