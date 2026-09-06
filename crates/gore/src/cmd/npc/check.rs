//! Ein verfasstes Arbeitsverzeichnis gegen den Übersetzungsvertrag prüfen.
//!
//! Jede Regel ist eine reine Funktion über Zeichenketten, damit sie ohne Spielinstallation
//! nachprüfbar ist. Das Kommando setzt sie nur zusammen und liest die Dateien dazu.
//!
//! Der wichtigste Befund ist der stille: ein Levelskript trägt bis zu 401 fremde Einträge, und
//! eine Zeile, die sich unbeabsichtigt bewegt hat, sähe im Spiel aus wie ein Fehler an ganz
//! anderer Stelle.

use super::defaults;

/// Wie ernst ein Befund ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Verhindert das Bauen.
    Blocking,
    /// Lässt bauen zu, aber jemand sollte hinsehen.
    Warning,
}

/// Ein einzelner Befund.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    pub message: String,
}

impl Finding {
    fn blocking(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Blocking,
            message: message.into(),
        }
    }

    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
        }
    }
}

/// Eine Zeile, die im editierten Levelskript hinzugekommen oder verschwunden ist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineChange {
    /// Zeilennummer in der Pristine-Fassung, 1-basiert.
    pub at: usize,
    pub added: bool,
    pub text: String,
}

/// Was sich zwischen Pristine und verfasster Fassung geändert hat.
///
/// Bewusst kein allgemeiner Diff: erlaubt sind nur ganze eingefügte oder entfernte Zeilen. Eine
/// umgeschriebene Zeile erscheint als ein Paar aus Entfernung und Einfügung und fällt damit auf.
pub fn line_changes(pristine: &str, edited: &str) -> Vec<LineChange> {
    let before: Vec<&str> = pristine.lines().collect();
    let after: Vec<&str> = edited.lines().collect();
    let mut changes = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < before.len() || j < after.len() {
        match (before.get(i), after.get(j)) {
            (Some(a), Some(b)) if a == b => {
                i += 1;
                j += 1;
            }
            (Some(a), Some(_)) if after[j..].contains(a) => {
                // Die Pristine-Zeile taucht später wieder auf: dazwischen wurde eingefügt.
                changes.push(LineChange {
                    at: i + 1,
                    added: true,
                    text: after[j].to_string(),
                });
                j += 1;
            }
            (Some(a), _) => {
                changes.push(LineChange {
                    at: i + 1,
                    added: false,
                    text: a.to_string(),
                });
                i += 1;
            }
            (None, Some(b)) => {
                changes.push(LineChange {
                    at: i + 1,
                    added: true,
                    text: b.to_string(),
                });
                j += 1;
            }
            (None, None) => break,
        }
    }
    changes
}

/// Ist diese Zeile eine Spawn-Zeile für `spawn_class`?
fn is_spawn_line_for(text: &str, spawn_class: &str) -> bool {
    text.contains("SpawnAIAgent(") && text.contains(spawn_class)
}

/// Der Diff-Wächter: nur die beabsichtigten Spawn-Zeilen dürfen sich bewegt haben.
pub fn guard_level_diff(pristine: &str, edited: &str, spawn_class: &str) -> Vec<Finding> {
    let changes = line_changes(pristine, edited);
    if changes.is_empty() {
        return vec![Finding::blocking(
            "the level script is unchanged, so there is nothing to build",
        )];
    }
    changes
        .iter()
        .filter(|change| !is_spawn_line_for(&change.text, spawn_class))
        .map(|change| {
            let verb = if change.added { "added" } else { "removed" };
            Finding::blocking(format!(
                "line {} was {verb} but does not spawn {spawn_class}: {}. Only the spawn lines of \
                 the character being authored may change; everything else in this level script \
                 belongs to other characters",
                change.at,
                change.text.trim()
            ))
        })
        .collect()
}

/// Die Klassen des verfassten Moduls gegen die Id prüfen.
pub fn guard_authored_module(source: &str, npc_id: &str) -> Vec<Finding> {
    let classes = defaults::parse_classes(source);
    let mut findings = Vec::new();
    if classes.is_empty() {
        findings.push(Finding::blocking(
            "the authored module declares no classes at all",
        ));
        return findings;
    }

    let assigned = |class: &defaults::EmittedClass, field: &str| -> Option<String> {
        class
            .assignments
            .iter()
            .find(|(lhs, _)| lhs == field)
            .map(|(_, rhs)| rhs.trim_start_matches('n').trim_matches('"').to_string())
    };

    let definition = classes
        .iter()
        .find(|class| class.name == format!("UCharacterDefinition_Human_{npc_id}"));
    match definition {
        None => findings.push(Finding::blocking(format!(
            "no class UCharacterDefinition_Human_{npc_id}: the character has no definition to \
             spawn from"
        ))),
        Some(class) => match assigned(class, "m_UniqueName").as_deref() {
            Some(name) if name == npc_id => {}
            Some(name) => findings.push(Finding::blocking(format!(
                "m_UniqueName is {name:?} but the character is {npc_id:?}. The save keys a \
                 character by that name, so the two have to agree"
            ))),
            None => findings.push(Finding::blocking(
                "the character definition sets no m_UniqueName. Without it the save cannot key \
                 this character",
            )),
        },
    }

    if let Some(settings) = classes
        .iter()
        .find(|class| class.super_class.as_deref() == Some("UConversationCharacterSettings"))
    {
        match assigned(settings, "ForCharacter").as_deref() {
            Some(name) if name == npc_id => {}
            Some(name) => findings.push(Finding::blocking(format!(
                "ForCharacter is {name:?} but the character is {npc_id:?}. The game binds \
                 conversation settings by that name"
            ))),
            None => findings.push(Finding::blocking(
                "the conversation settings set no ForCharacter, so nothing binds them to this \
                 character",
            )),
        }
    }

    if let Some(visuals) = classes
        .iter()
        .find(|class| class.name.starts_with("UCharacterVisualsDefinition_Human_"))
    {
        let prebaked = assigned(visuals, "m_HasPreBakedSK");
        let name = assigned(visuals, "m_PreBakedName");
        match (prebaked.as_deref(), name) {
            (Some("true"), None) => findings.push(Finding::blocking(
                "m_HasPreBakedSK is true but no m_PreBakedName says which baked model to use. A \
                 new id has none of its own, so it has to borrow one",
            )),
            (Some("false"), _) => findings.push(Finding::warning(
                "m_HasPreBakedSK is false: the looks are built from parts at runtime. Measured in \
                 game — this renders a working body, but it comes out looking like the player \
                 character rather than the template, whose part fields it inherits and does not \
                 use. Borrow a baked model instead unless you know why you want this",
            )),
            _ => {}
        }
    }

    findings
}

/// Der Wächter für eine ausgecheckte Figur: was das Modul deklariert, bleibt, wie es war.
///
/// Werte und Rümpfe dürfen sich ändern — genau dafür checkt man aus. Die Klassenstruktur nicht:
/// eine ausgelieferte Klasse zu entfernen, umzubenennen oder ihre Elternklasse zu tauschen
/// erzeugt ein anderes Symbol, und dann trifft der Remap gegen die Basis-Cache ins Leere. Eine
/// neue Klasse ist ebenfalls nichts für diesen Weg; sie verlangt den Vertrag, den `new` benutzt.
pub fn guard_checkout_diff(pristine: &str, edited: &str) -> Vec<Finding> {
    let before = defaults::parse_classes(pristine);
    let after = defaults::parse_classes(edited);
    let mut findings = Vec::new();

    for class in &before {
        match after.iter().find(|other| other.name == class.name) {
            None => findings.push(Finding::blocking(format!(
                "class {} is gone. A shipped class may change its values, but removing or \
                 renaming it produces a different symbol that no longer matches the base cache",
                class.name
            ))),
            Some(other) if other.super_class != class.super_class => {
                findings.push(Finding::blocking(format!(
                    "class {} now derives from {} instead of {}. The parent is part of the \
                     class's identity, so changing it makes it a different class",
                    class.name,
                    other.super_class.as_deref().unwrap_or("nothing"),
                    class.super_class.as_deref().unwrap_or("nothing")
                )));
            }
            Some(_) => {}
        }
    }

    for class in &after {
        if !before.iter().any(|other| other.name == class.name) {
            findings.push(Finding::blocking(format!(
                "class {} is new. Checking a shipped character out is for changing its values; a \
                 new class needs `gore npc new`, which carries the contract for one",
                class.name
            )));
        }
    }

    if before == after {
        findings.push(Finding::blocking(
            "the module is unchanged, so there is nothing to build",
        ));
    }
    findings
}

/// Die Wegpunkte, die der Tagesablauf anspricht.
pub fn scheduled_waypoints(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    for class in defaults::parse_classes(source) {
        for call in &class.calls {
            if !call.starts_with("Schedule(") && !call.starts_with("ScheduleIfRaining(") {
                continue;
            }
            // Das erste `n"..."`-Argument ist der Wegpunkt.
            if let Some(start) = call.find("n\"") {
                let rest = &call[start + 2..];
                if let Some(end) = rest.find('"') {
                    out.push(rest[..end].to_string());
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRISTINE: &str = r#"class UWP_A : UWorldPointScript
{
    void OnWorldStart()
    {
        this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_Diego::StaticClass()), nullptr);
        return;
    }
}
"#;

    fn with_added_line() -> String {
        PRISTINE.replace(
            "        return;\n",
            "        this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_MINE::StaticClass()), nullptr);\n        return;\n",
        )
    }

    #[test]
    fn an_added_spawn_line_is_the_only_change_and_passes() {
        let findings =
            guard_level_diff(PRISTINE, &with_added_line(), "USpawnAIAgentDefinition_MINE");
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_removed_spawn_line_of_the_named_character_passes() {
        let edited = PRISTINE
            .lines()
            .filter(|l| !l.contains("_Diego"))
            .collect::<Vec<_>>()
            .join("\n");
        let findings = guard_level_diff(PRISTINE, &edited, "USpawnAIAgentDefinition_Diego");
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_stray_change_elsewhere_is_blocking_and_names_the_line() {
        let edited = with_added_line().replace("        return;", "        DoSomethingElse();");
        let findings = guard_level_diff(PRISTINE, &edited, "USpawnAIAgentDefinition_MINE");
        assert!(findings
            .iter()
            .any(|f| f.severity == Severity::Blocking && f.message.contains("DoSomethingElse")));
    }

    #[test]
    fn an_unchanged_level_script_is_blocking() {
        let findings = guard_level_diff(PRISTINE, PRISTINE, "USpawnAIAgentDefinition_MINE");
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("unchanged"));
    }

    #[test]
    fn a_spawn_line_for_a_different_character_is_still_a_stray_change() {
        // Wer eine fremde Figur mit hineinschreibt, ändert das Spiel an einer Stelle, die er
        // nicht verantwortet.
        let edited = PRISTINE.replace(
            "        return;\n",
            "        this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_Other::StaticClass()), nullptr);\n        return;\n",
        );
        let findings = guard_level_diff(PRISTINE, &edited, "USpawnAIAgentDefinition_MINE");
        assert!(findings.iter().any(|f| f.severity == Severity::Blocking));
    }

    const AUTHORED: &str = r#"class UCharacterDefinition_Human_MINE : UCharacterDefinition_Human_OC_STT_Diego
{
    default m_UniqueName = n"MINE";
    default m_CharacterVisualsDefinition = UCharacterVisualsDefinition_Human_MINE::StaticClass();
}

class UCharacterVisualsDefinition_Human_MINE : UCharacterVisualsDefinition_Human_OC_STT_Diego
{
    default m_PreBakedName = "OC_STT_Diego";
    default m_HasPreBakedSK = true;
}

class UConversationCharacterSettings_Ambient_MINE : UConversationCharacterSettings
{
    default ForCharacter = n"MINE";
}

class UDailyRoutine_MINE_Start : UAIState_DailyRoutine_Human
{
    default Schedule(0, 0, UAIState_Stand(), n"FP_OC_SMALLTALK_33", 1000.0f, TSubclassOf<UNavArea>(nullptr), nullptr);
}
"#;

    #[test]
    fn a_well_formed_authored_module_has_no_findings() {
        assert!(guard_authored_module(AUTHORED, "MINE").is_empty());
    }

    #[test]
    fn a_unique_name_that_disagrees_with_the_id_is_blocking() {
        let source = AUTHORED.replace(r#"n"MINE""#, r#"n"OTHER""#);
        let findings = guard_authored_module(&source, "MINE");
        assert!(findings
            .iter()
            .any(|f| f.severity == Severity::Blocking && f.message.contains("m_UniqueName")));
    }

    #[test]
    fn a_missing_character_definition_is_blocking() {
        let findings = guard_authored_module("class UX : UY\n{\n}\n", "MINE");
        assert!(findings
            .iter()
            .any(|f| f.message.contains("UCharacterDefinition_Human_MINE")));
    }

    #[test]
    fn a_prebaked_flag_without_a_model_name_is_blocking() {
        let source = AUTHORED.replace("    default m_PreBakedName = \"OC_STT_Diego\";\n", "");
        let findings = guard_authored_module(&source, "MINE");
        assert!(findings
            .iter()
            .any(|f| f.severity == Severity::Blocking && f.message.contains("m_PreBakedName")));
    }

    #[test]
    fn modular_visuals_are_a_warning_not_a_refusal() {
        let source = AUTHORED.replace("m_HasPreBakedSK = true", "m_HasPreBakedSK = false");
        let findings = guard_authored_module(&source, "MINE");
        let modular = findings
            .iter()
            .find(|f| f.message.contains("built from parts"))
            .expect("a finding about modular visuals");
        assert_eq!(modular.severity, Severity::Warning);
    }

    #[test]
    fn a_conversation_anchor_for_the_wrong_character_is_blocking() {
        let source = AUTHORED.replace(
            "    default ForCharacter = n\"MINE\";",
            "    default ForCharacter = n\"SOMEONE\";",
        );
        let findings = guard_authored_module(&source, "MINE");
        assert!(findings
            .iter()
            .any(|f| f.severity == Severity::Blocking && f.message.contains("ForCharacter")));
    }

    #[test]
    fn a_changed_value_in_a_checked_out_module_passes() {
        let edited = AUTHORED.replace("1000.0f", "500.0f");
        assert!(guard_checkout_diff(AUTHORED, &edited).is_empty());
    }

    #[test]
    fn an_unchanged_checked_out_module_is_blocking() {
        let findings = guard_checkout_diff(AUTHORED, AUTHORED);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("unchanged"));
    }

    #[test]
    fn removing_a_shipped_class_is_blocking() {
        let edited = AUTHORED.replace(
            "class UDailyRoutine_MINE_Start : UAIState_DailyRoutine_Human",
            "class UNothing : UAIState_DailyRoutine_Human",
        );
        let findings = guard_checkout_diff(AUTHORED, &edited);
        assert!(findings
            .iter()
            .any(|f| f.message.contains("UDailyRoutine_MINE_Start") && f.message.contains("gone")));
        assert!(findings
            .iter()
            .any(|f| f.message.contains("UNothing") && f.message.contains("is new")));
    }

    #[test]
    fn swapping_a_parent_class_is_blocking() {
        // Die Elternklasse gehoert zur Identitaet: ein Tausch macht daraus ein anderes Symbol,
        // und der Remap gegen die Basis-Cache trifft ins Leere.
        let edited = AUTHORED.replace(
            "UCharacterDefinition_Human_MINE : UCharacterDefinition_Human_OC_STT_Diego",
            "UCharacterDefinition_Human_MINE : UCharacterDefinition_Human_OldCamp_Guard",
        );
        let findings = guard_checkout_diff(AUTHORED, &edited);
        assert!(findings
            .iter()
            .any(|f| f.severity == Severity::Blocking && f.message.contains("derives from")));
    }

    #[test]
    fn the_scheduled_waypoints_come_out_of_the_routine() {
        assert_eq!(scheduled_waypoints(AUTHORED), vec!["FP_OC_SMALLTALK_33"]);
    }

    #[test]
    fn a_module_without_a_routine_schedules_nothing() {
        assert!(scheduled_waypoints("class UX : UY\n{\n}\n").is_empty());
    }
}
