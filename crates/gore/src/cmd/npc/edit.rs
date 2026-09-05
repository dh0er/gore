//! Ein ausgeliefertes Levelskript ändern: eine Spawn-Zeile hinzufügen oder eine entfernen.
//!
//! Der Rest der Datei bleibt Zeichen für Zeichen, wie er war. `check` prüft das später gegen die
//! Pristine-Kopie; hier wird es dadurch erreicht, dass nur ganze Zeilen eingefügt oder entfernt
//! und nie bestehende umgeschrieben werden. Ein Levelskript trägt bis zu 401 fremde Einträge —
//! was der Autor nicht angefasst hat, darf sich nicht bewegen.

use super::sites::WORLD_POINT_BASE;

/// Was beim Ändern schiefgehen kann.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    #[error("no world point {0} in this level script")]
    NoSuchWorldPoint(String),
    #[error("world point {0} has no OnWorldStart body to add to")]
    NoBody(String),
    #[error("{0} already spawns {1}")]
    AlreadySpawns(String, String),
    #[error("no world point in this level script spawns {0}")]
    NotSpawnedHere(String),
}

/// Die Spawn-Zeile, die eine Figur an einem Weltpunkt setzt.
///
/// Zweites Argument ist eine Tagesablauf-Instanz oder `nullptr` — beides kommt im ausgelieferten
/// Baum vor (`Map_OldCamp_IE_script.as` übergibt Routinen, die meisten übergeben `nullptr`).
pub fn spawn_line(indent: &str, spawn_class: &str, routine_class: Option<&str>) -> String {
    let routine = match routine_class {
        Some(class) => format!("{class}()"),
        None => "nullptr".to_string(),
    };
    format!(
        "{indent}this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>({spawn_class}::StaticClass()), {routine});"
    )
}

/// Die Einrückung einer Zeile — alles vor dem ersten Nicht-Leerzeichen.
fn indent_of(line: &str) -> &str {
    &line[..line.len() - line.trim_start().len()]
}

/// Beginnt hier die Klasse `world_point`?
fn opens_world_point(line: &str, world_point: &str) -> bool {
    let trimmed = line.trim();
    trimmed
        .strip_prefix("class ")
        .and_then(|rest| rest.split_once(':'))
        .is_some_and(|(name, base)| name.trim() == world_point && base.trim() == WORLD_POINT_BASE)
}

/// Beginnt hier irgendeine Klasse?
fn opens_any_class(line: &str) -> bool {
    line.trim().starts_with("class ")
}

/// `source` mit einer zusätzlichen Spawn-Zeile in `world_point`s `OnWorldStart`.
///
/// Die neue Zeile landet unmittelbar hinter der letzten vorhandenen Spawn-Zeile derselben Klasse,
/// mit deren Einrückung; gibt es keine, hinter der öffnenden Klammer von `OnWorldStart`.
pub fn add_spawn(
    source: &str,
    world_point: &str,
    spawn_class: &str,
    routine_class: Option<&str>,
) -> Result<String, EditError> {
    let lines: Vec<&str> = source.lines().collect();
    let Some(start) = lines
        .iter()
        .position(|line| opens_world_point(line, world_point))
    else {
        return Err(EditError::NoSuchWorldPoint(world_point.to_string()));
    };
    let end = lines[start + 1..]
        .iter()
        .position(|line| opens_any_class(line))
        .map_or(lines.len(), |offset| start + 1 + offset);

    let mut last_spawn = None;
    let mut body_open = None;
    for (index, line) in lines[start..end].iter().enumerate() {
        let index = start + index;
        if line.contains("SpawnAIAgent(") {
            if line.contains(spawn_class) {
                return Err(EditError::AlreadySpawns(
                    world_point.to_string(),
                    spawn_class.to_string(),
                ));
            }
            last_spawn = Some(index);
        }
        if body_open.is_none() && line.contains("OnWorldStart()") {
            // Die öffnende Klammer steht in der Zeile darauf, im Stil des Emitters.
            body_open = lines
                .get(index + 1)
                .filter(|next| next.trim() == "{")
                .map(|_| index + 1);
        }
    }

    let (at, indent) = match last_spawn {
        Some(index) => (index + 1, indent_of(lines[index]).to_string()),
        None => match body_open {
            // Ohne vorhandene Spawn-Zeile richtet sich die Einrückung nach der Klammer plus einer
            // Ebene, so wie der Emitter Rümpfe schreibt.
            Some(index) => (index + 1, format!("{}    ", indent_of(lines[index]))),
            None => return Err(EditError::NoBody(world_point.to_string())),
        },
    };

    let mut out: Vec<String> = lines[..at].iter().map(|line| line.to_string()).collect();
    out.push(spawn_line(&indent, spawn_class, routine_class));
    out.extend(lines[at..].iter().map(|line| line.to_string()));
    let mut text = out.join("\n");
    if source.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}

/// `source` ohne die Spawn-Zeilen, die `spawn_class` setzen.
///
/// Die Weltpunkt-Klassen selbst bleiben stehen; nur ihre Zeile verschwindet. Ein leerer
/// `OnWorldStart`-Rumpf ist gültig und setzt schlicht niemanden mehr.
pub fn remove_spawn(source: &str, spawn_class: &str) -> Result<String, EditError> {
    let matches = |line: &str| line.contains("SpawnAIAgent(") && line.contains(spawn_class);
    if !source.lines().any(matches) {
        return Err(EditError::NotSpawnedHere(spawn_class.to_string()));
    }
    let kept: Vec<&str> = source.lines().filter(|line| !matches(line)).collect();
    let mut text = kept.join("\n");
    if source.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Zwei Weltpunkte, wie der Emitter sie schreibt. Rohtext-Literal mit Absicht: eine
    /// Zeilenfortsetzung mit Rueckstrich schluckt die fuehrende Einrueckung der Folgezeile, und
    /// genau die ist hier Pruefgegenstand.
    const SOURCE: &str = r#"class UWP_A : UWorldPointScript
{
    UFUNCTION()
    void OnWorldStart()
    {
        this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_Diego::StaticClass()), nullptr);
        return;
    }
}

class UWP_B : UWorldPointScript
{
    UFUNCTION()
    void OnWorldStart()
    {
        this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_Other::StaticClass()), nullptr);
        return;
    }
}
"#;

    #[test]
    fn add_spawn_inserts_one_line_and_changes_nothing_else() {
        let out = add_spawn(SOURCE, "UWP_A", "USpawnAIAgentDefinition_MY_NPC", None).expect("edit");
        let before: Vec<&str> = SOURCE.lines().collect();
        let after: Vec<&str> = out.lines().collect();
        assert_eq!(after.len(), before.len() + 1);
        // Jede ursprüngliche Zeile kommt unverändert wieder vor, in derselben Reihenfolge.
        let mut original = before.iter();
        for line in &after {
            if line.contains("MY_NPC") {
                continue;
            }
            assert_eq!(Some(line), original.next());
        }
        assert!(original.next().is_none());
    }

    #[test]
    fn the_new_line_lands_in_the_named_world_point_after_the_existing_spawn() {
        let out = add_spawn(SOURCE, "UWP_A", "USpawnAIAgentDefinition_MY_NPC", None).expect("edit");
        let lines: Vec<&str> = out.lines().collect();
        let diego = lines
            .iter()
            .position(|l| l.contains("_Diego"))
            .expect("diego");
        let mine = lines
            .iter()
            .position(|l| l.contains("MY_NPC"))
            .expect("mine");
        assert_eq!(mine, diego + 1);
        // und nicht im anderen Weltpunkt
        let other = lines
            .iter()
            .position(|l| l.contains("_Other"))
            .expect("other");
        assert!(mine < other);
    }

    #[test]
    fn the_new_line_borrows_the_indentation_of_the_one_above_it() {
        let out = add_spawn(SOURCE, "UWP_A", "USpawnAIAgentDefinition_MY_NPC", None).expect("edit");
        let line = out
            .lines()
            .find(|l| l.contains("MY_NPC"))
            .expect("the new line");
        assert!(line.starts_with("        this.SpawnAIAgent("));
    }

    #[test]
    fn a_world_point_without_a_spawn_yet_gets_the_line_inside_its_body() {
        let source = "class UWP_C : UWorldPointScript\n{\n    UFUNCTION()\n    void OnWorldStart()\n    {\n        return;\n    }\n}\n";
        let out = add_spawn(source, "UWP_C", "USpawnAIAgentDefinition_MY_NPC", None).expect("edit");
        let lines: Vec<&str> = out.lines().collect();
        let brace = lines
            .iter()
            .position(|l| l.trim() == "{" && l.starts_with("    "))
            .expect("body brace");
        assert!(lines[brace + 1].contains("MY_NPC"));
        assert!(lines[brace + 1].starts_with("        "));
    }

    #[test]
    fn a_routine_class_is_passed_as_the_second_argument() {
        let out = add_spawn(
            SOURCE,
            "UWP_A",
            "USpawnAIAgentDefinition_MY_NPC",
            Some("UDailyRoutine_MY_NPC_Start"),
        )
        .expect("edit");
        assert!(out.contains(
            "USpawnAIAgentDefinition_MY_NPC::StaticClass()), UDailyRoutine_MY_NPC_Start());"
        ));
    }

    #[test]
    fn an_unknown_world_point_is_refused_by_name() {
        assert_eq!(
            add_spawn(SOURCE, "UWP_NOPE", "USpawnAIAgentDefinition_MY_NPC", None),
            Err(EditError::NoSuchWorldPoint("UWP_NOPE".to_string()))
        );
    }

    #[test]
    fn a_class_that_is_not_a_world_point_is_not_a_target() {
        let source = "class UWP_A : USomethingElse\n{\n    void OnWorldStart()\n    {\n    }\n}\n";
        assert_eq!(
            add_spawn(source, "UWP_A", "USpawnAIAgentDefinition_MY_NPC", None),
            Err(EditError::NoSuchWorldPoint("UWP_A".to_string()))
        );
    }

    #[test]
    fn adding_the_same_character_twice_is_refused() {
        let once =
            add_spawn(SOURCE, "UWP_A", "USpawnAIAgentDefinition_MY_NPC", None).expect("edit");
        assert!(matches!(
            add_spawn(&once, "UWP_A", "USpawnAIAgentDefinition_MY_NPC", None),
            Err(EditError::AlreadySpawns(_, _))
        ));
    }

    #[test]
    fn remove_spawn_drops_only_the_named_characters_lines() {
        let out = remove_spawn(SOURCE, "USpawnAIAgentDefinition_Diego").expect("edit");
        assert!(!out.contains("_Diego"));
        assert!(out.contains("_Other"));
        // Die Klasse selbst bleibt stehen, nur ihr Rumpf verliert die Zeile.
        assert!(out.contains("class UWP_A : UWorldPointScript"));
        assert!(out.contains("void OnWorldStart()"));
        assert_eq!(out.lines().count(), SOURCE.lines().count() - 1);
    }

    #[test]
    fn removing_a_character_that_is_not_spawned_here_is_refused() {
        assert_eq!(
            remove_spawn(SOURCE, "USpawnAIAgentDefinition_Nobody"),
            Err(EditError::NotSpawnedHere(
                "USpawnAIAgentDefinition_Nobody".to_string()
            ))
        );
    }

    #[test]
    fn removing_a_character_that_stands_at_two_points_drops_both() {
        let twice =
            add_spawn(SOURCE, "UWP_B", "USpawnAIAgentDefinition_Diego", None).expect("edit");
        let out = remove_spawn(&twice, "USpawnAIAgentDefinition_Diego").expect("edit");
        assert!(!out.contains("_Diego"));
        assert!(out.contains("_Other"));
    }

    #[test]
    fn the_trailing_newline_survives_both_edits() {
        assert!(
            add_spawn(SOURCE, "UWP_A", "USpawnAIAgentDefinition_X", None)
                .expect("edit")
                .ends_with('\n')
        );
        assert!(remove_spawn(SOURCE, "USpawnAIAgentDefinition_Diego")
            .expect("edit")
            .ends_with('\n'));
    }

    #[test]
    fn the_spawn_line_keeps_the_indentation_it_is_given() {
        assert!(spawn_line("        ", "UX", None).starts_with("        this.SpawnAIAgent("));
    }
}
