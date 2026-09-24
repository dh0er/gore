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

/// Ein Weltpunkt und ob dort schon jemand steht.
///
/// Der Unterschied ist im Spiel sichtbar geworden: zwei Figuren an demselben Punkt stehen
/// ineinander, der Fokus greift nur eine, und die andere flackert je nach Blickwinkel. Wer eine
/// Figur setzen will, will fast immer einen freien Punkt — und von denen gibt es mehr als doppelt
/// so viele wie belegte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldPoint {
    pub name: String,
    pub module: String,
    /// Die Spawn-Definitionen, die dieser Punkt heute setzt. Leer heißt frei.
    pub occupants: Vec<String>,
}

impl WorldPoint {
    /// Steht hier schon jemand?
    pub fn is_occupied(&self) -> bool {
        !self.occupants.is_empty()
    }
}

/// Der Klammerblock direkt nach einer Deklaration. Die emittierten Levelskripte setzen die
/// öffnende Klammer auf die nächste Zeile; die Tiefe hält auch verschachtelte Blöcke zusammen.
fn braced_body_after(source: &str, declaration_end: usize) -> Option<&str> {
    let tail = &source[declaration_end..];
    let open = tail.find(|c: char| !c.is_whitespace())?;
    if tail.as_bytes()[open] != b'{' {
        return None;
    }
    let mut depth = 0;
    for (offset, ch) in tail[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&tail[open + 1..open + offset]);
                }
            }
            _ => {}
        }
    }
    None
}

fn on_world_start_body(class_body: &str) -> Option<&str> {
    let mut offset = 0;
    for line in class_body.split_inclusive('\n') {
        if line.trim_start().starts_with("void OnWorldStart()") {
            let signature = line.find("void OnWorldStart()")?;
            let after_signature = offset + signature + "void OnWorldStart()".len();
            return braced_body_after(class_body, after_signature);
        }
        offset += line.len();
    }
    None
}

/// Jeder Weltpunkt eines emittierten Levelskripts, belegt oder nicht.
///
/// `parse_sites` sieht nur die belegten, weil es die Spawn-Zeilen liest. Diese Funktion geht von
/// den Klassen aus und findet deshalb auch die 2729 leeren.
pub fn parse_world_points(module: &str, source: &str) -> Vec<WorldPoint> {
    let mut out: Vec<WorldPoint> = Vec::new();
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("class ") {
            if let Some((name, base)) = rest.split_once(':') {
                if base.trim() == WORLD_POINT_BASE {
                    let occupants = braced_body_after(source, offset + line.len())
                        .and_then(on_world_start_body)
                        .into_iter()
                        .flat_map(str::lines)
                        .filter(|line| line.contains("SpawnAIAgent("))
                        .filter_map(spawn_definition_in)
                        .map(str::to_string)
                        .collect();
                    out.push(WorldPoint {
                        name: name.trim().to_string(),
                        module: module.to_string(),
                        occupants,
                    });
                }
            }
        }
        offset += line.len();
    }
    out
}

/// Jede Spawn-Stelle eines emittierten Levelskripts.
pub fn parse_sites(module: &str, source: &str) -> Vec<Site> {
    parse_world_points(module, source)
        .into_iter()
        .flat_map(|point| {
            point
                .occupants
                .into_iter()
                .map(move |spawn_definition| Site {
                    world_point: point.name.clone(),
                    module: point.module.clone(),
                    spawn_definition,
                })
        })
        .collect()
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
    fn parse_world_points_finds_the_empty_ones_too() {
        // Der Grund, warum es diese Funktion gibt: `parse_sites` liest Spawn-Zeilen und sieht
        // deshalb nur belegte Punkte. Wer eine Figur setzen will, braucht die freien.
        let source = format!(
            "{SOURCE}\nclass UWP_EMPTY : UWorldPointScript\n{{\n    void OnWorldStart()\n    {{\n    }}\n}}\n"
        );
        let points = parse_world_points("LevelScripts.Demo", &source);
        assert_eq!(points.len(), 3);
        let empty = points
            .iter()
            .find(|p| p.name == "UWP_EMPTY")
            .expect("the empty point");
        assert!(!empty.is_occupied());
        assert!(empty.occupants.is_empty());
    }

    #[test]
    fn an_occupied_point_lists_everyone_it_sets() {
        let points = parse_world_points("LevelScripts.Demo", SOURCE);
        let two = points
            .iter()
            .find(|p| p.name == "UWP_EZ_TWO")
            .expect("the busy point");
        assert!(two.is_occupied());
        assert_eq!(
            two.occupants,
            vec!["USpawnAIAgentDefinition_A", "USpawnAIAgentDefinition_B"]
        );
    }

    #[test]
    fn a_class_that_is_not_a_world_point_is_no_point_at_all() {
        let points = parse_world_points("LevelScripts.Demo", SOURCE);
        assert!(points.iter().all(|p| p.name != "UNotAWorldPoint"));
    }

    #[test]
    fn a_spawn_in_a_later_non_world_point_does_not_occupy_the_previous_point() {
        let source = format!(
            "{SOURCE}\nclass UHelper : UObject\n{{\n    void Go()\n    {{\n        this.SpawnAIAgent(USpawnAIAgentDefinition_WRONG, nullptr);\n    }}\n}}\n"
        );
        let points = parse_world_points("LevelScripts.Demo", &source);
        assert!(points
            .iter()
            .all(|point| !point.occupants.iter().any(|npc| npc.ends_with("_WRONG"))));
    }

    #[test]
    fn only_spawns_in_on_world_start_occupy_a_point() {
        let source = r#"class UWP_TEST : UWorldPointScript
{
    void OnWorldStart()
    {
        if (true)
        {
            this.SpawnAIAgent(USpawnAIAgentDefinition_REAL, nullptr);
        }
    }

    void Helper()
    {
        this.SpawnAIAgent(USpawnAIAgentDefinition_IN_CLASS_HELPER, nullptr);
    }
}

void ModuleHelper()
{
    this.SpawnAIAgent(USpawnAIAgentDefinition_AFTER_CLASS, nullptr);
}
"#;
        let points = parse_world_points("LevelScripts.Demo", source);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].occupants, ["USpawnAIAgentDefinition_REAL"]);
        let sites = parse_sites("LevelScripts.Demo", source);
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].spawn_definition, "USpawnAIAgentDefinition_REAL");
    }

    #[test]
    fn a_spawn_call_outside_any_world_point_class_is_ignored() {
        // Freie Funktionen und andere Klassen rufen `SpawnAIAgent` ebenfalls auf; nur ein
        // Weltpunkt ist eine Stelle, an die sich ein Mod hängen kann.
        let source = "class UHelper : UObject\n{\n    void Go()\n    {\n        this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_X::StaticClass()), nullptr);\n    }\n}\n";
        assert!(parse_sites("LevelScripts.Demo", source).is_empty());
    }
}
