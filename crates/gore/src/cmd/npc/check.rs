//! Ein verfasstes Arbeitsverzeichnis gegen den Übersetzungsvertrag prüfen.
//!
//! Jede Regel ist eine reine Funktion über Zeichenketten, damit sie ohne Spielinstallation
//! nachprüfbar ist. Das Kommando setzt sie nur zusammen und liest die Dateien dazu.
//!
//! Der wichtigste Befund ist der stille: ein Levelskript trägt bis zu 401 fremde Einträge, und
//! eine Zeile, die sich unbeabsichtigt bewegt hat, sähe im Spiel aus wie ein Fehler an ganz
//! anderer Stelle.

use super::{chain, defaults, edit};

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
    edit::is_spawn_line_for(text, spawn_class)
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

/// A new character's level script must be exactly the generated edit at its chosen world point.
/// This also rejects spawn-looking comments and a valid call moved into another class.
pub fn guard_generated_spawn(
    pristine: &str,
    edited: &str,
    world_point: &str,
    spawn_class: &str,
) -> Vec<Finding> {
    let no_routine = edit::add_spawn(pristine, world_point, spawn_class, None);
    let routine = spawn_class
        .strip_prefix("USpawnAIAgentDefinition_")
        .map(|id| format!("UDailyRoutine_{id}_Start"));
    let with_routine = routine
        .as_deref()
        .and_then(|name| edit::add_spawn(pristine, world_point, spawn_class, Some(name)).ok());
    if no_routine.as_ref().is_ok_and(|expected| expected == edited)
        || with_routine.as_deref() == Some(edited)
    {
        return Vec::new();
    }
    vec![Finding::blocking(format!(
        "the level script must contain only the generated spawn call for {spawn_class} in \
         {world_point}'s OnWorldStart body"
    ))]
}

/// A suppression must be exactly the generated removal, including its original world points.
pub fn guard_suppressed_spawn(pristine: &str, edited: &str, spawn_class: &str) -> Vec<Finding> {
    if edit::remove_spawn(pristine, spawn_class)
        .as_deref()
        .is_ok_and(|expected| expected == edited)
    {
        return Vec::new();
    }
    vec![Finding::blocking(format!(
        "the level script must contain exactly the generated removal of {spawn_class}'s spawn calls"
    ))]
}

/// The copied baseline must still be the source emitted from the selected cache.
pub fn guard_pristine_source(cached: Option<&str>, pristine: &str, module: &str) -> Vec<Finding> {
    if cached == Some(pristine) {
        return Vec::new();
    }
    vec![Finding::blocking(format!(
        "the pristine copy of {module} differs from the selected cache; author this workspace again"
    ))]
}

/// Die Klassen des verfassten Moduls gegen die Id prüfen.
pub fn guard_authored_module(
    source: &str,
    npc_id: &str,
    expected_actor_blueprint: &str,
) -> Vec<Finding> {
    let classes = defaults::parse_classes(source);
    let mut findings = Vec::new();
    if classes.is_empty() {
        findings.push(Finding::blocking(
            "the authored module declares no classes at all",
        ));
        return findings;
    }
    for class in classes.iter().filter(|class| class.namespace.is_some()) {
        findings.push(Finding::blocking(format!(
            "class {} is inside namespace {}; authored NPC classes must be global because the generated spawn and dialog references use global names",
            class.name,
            class.namespace.as_deref().unwrap_or_default()
        )));
    }

    let assigned = |class: &defaults::EmittedClass, field: &str| -> Vec<String> {
        class
            .assignments
            .iter()
            .filter(|(lhs, _)| lhs == field)
            .map(|(_, rhs)| rhs.to_string())
            .collect()
    };
    let expected_name = format!("n\"{npc_id}\"");

    let definition = classes
        .iter()
        .find(|class| class.name == format!("UCharacterDefinition_Human_{npc_id}"));
    match definition {
        None => findings.push(Finding::blocking(format!(
            "no class UCharacterDefinition_Human_{npc_id}: the character has no definition to \
             spawn from"
        ))),
        Some(class) => match assigned(class, chain::UNIQUE_NAME_FIELD).as_slice() {
            [name] if name == &expected_name => {}
            [name] => findings.push(Finding::blocking(format!(
                "m_UniqueName must be exactly {expected_name}, not {name:?}. The save keys a \
                 character by that name"
            ))),
            [] => findings.push(Finding::blocking(
                "the character definition sets no m_UniqueName. Without it the save cannot key \
                 this character",
            )),
            values => findings.push(Finding::blocking(format!(
                "the character definition sets m_UniqueName {} times. The authored identity must \
                 have exactly one value",
                values.len()
            ))),
        },
    }
    if let Some(class) = definition {
        let expected = format!("UCharacterVisualsDefinition_Human_{npc_id}::StaticClass()");
        match assigned(class, "m_CharacterVisualsDefinition").as_slice() {
            [value] if value == &expected => {}
            [value] => findings.push(Finding::blocking(format!(
                "m_CharacterVisualsDefinition must be exactly {expected}, not {value}"
            ))),
            [] => findings.push(Finding::blocking(format!(
                "the character definition sets no m_CharacterVisualsDefinition; it must reference {expected}"
            ))),
            values => findings.push(Finding::blocking(format!(
                "the character definition sets m_CharacterVisualsDefinition {} times; it must have exactly one target",
                values.len()
            ))),
        }
    }

    let config_name = format!("UAIAgentConfig_Human_{npc_id}");
    let config = classes.iter().find(|class| class.name == config_name);
    match config {
        None => findings.push(Finding::blocking(format!(
            "no class {config_name}: the spawn has no authored AI config to use"
        ))),
        Some(class) => {
            let expected = format!("UCharacterDefinition_Human_{npc_id}");
            let expected_call = format!("{expected}::StaticClass()");
            let values = assigned(class, chain::AI_CHARACTER_FIELD);
            match values.as_slice() {
                [value] if value == &expected_call => {}
                [value] => findings.push(Finding::blocking(format!(
                    "m_CharacterDefinition must be exactly {expected_call}, not {value}"
                ))),
                [] => findings.push(Finding::blocking(format!(
                    "{config_name} sets no m_CharacterDefinition"
                ))),
                values => findings.push(Finding::blocking(format!(
                    "{config_name} sets m_CharacterDefinition {} times; the authored chain must \
                     have exactly one target",
                    values.len()
                ))),
            }
        }
    }

    let spawn_name = format!("USpawnAIAgentDefinition_{npc_id}");
    let spawn = classes.iter().find(|class| class.name == spawn_name);
    match spawn {
        None => findings.push(Finding::blocking(format!(
            "no class {spawn_name}: the character has no authored spawn definition"
        ))),
        Some(class) => {
            let expected_call = format!("{config_name}::StaticClass()");
            let values = assigned(class, chain::SPAWN_AI_FIELD);
            match values.as_slice() {
                [value] if value == &expected_call => {}
                [value] => findings.push(Finding::blocking(format!(
                    "AIAgentConfigClass must be exactly {expected_call}, not {value}"
                ))),
                [] => findings.push(Finding::blocking(format!(
                    "{spawn_name} sets no AIAgentConfigClass"
                ))),
                values => findings.push(Finding::blocking(format!(
                    "{spawn_name} sets AIAgentConfigClass {} times; the authored chain must have \
                     exactly one target",
                    values.len()
                ))),
            }
            let actor_values = class
                .assignments
                .iter()
                .filter(|(field, _)| field == "AIAgentCharacterClass")
                .map(|(_, value)| value.as_str())
                .collect::<Vec<_>>();
            match actor_values.as_slice() {
                [value] if *value == expected_actor_blueprint => {}
                [value] => findings.push(Finding::blocking(format!(
                    "{spawn_name} AIAgentCharacterClass must retain the template's actor blueprint {expected_actor_blueprint}, not {value}"
                ))),
                [] => findings.push(Finding::blocking(format!(
                    "{spawn_name} sets no AIAgentCharacterClass actor blueprint"
                ))),
                values => findings.push(Finding::blocking(format!(
                    "{spawn_name} sets AIAgentCharacterClass {} times; the authored spawn needs exactly one actor blueprint",
                    values.len()
                ))),
            }
        }
    }

    let settings = classes
        .iter()
        .filter(|class| class.super_class.as_deref() == Some("UConversationCharacterSettings"))
        .collect::<Vec<_>>();
    let expected_settings = format!("UConversationCharacterSettings_Ambient_{npc_id}");
    if let [settings] = settings.as_slice() {
        if settings.name != expected_settings {
            findings.push(Finding::blocking(format!(
                "conversation settings class {:?} must be named {expected_settings} so dialog and \
                 voice can find it",
                settings.name
            )));
        }
    }
    match settings.as_slice() {
        [settings] => match assigned(settings, "ForCharacter").as_slice() {
            [name] if name == &expected_name => {}
            [name] => findings.push(Finding::blocking(format!(
                "ForCharacter must be exactly {expected_name}, not {name:?}. The game binds \
                 conversation settings by that name"
            ))),
            [] => findings.push(Finding::blocking(
                "the conversation settings set no ForCharacter, so nothing binds them to this \
                 character",
            )),
            values => findings.push(Finding::blocking(format!(
                "the conversation settings set ForCharacter {} times. The authored identity must \
                 have exactly one value",
                values.len()
            ))),
        },
        settings => {
            findings.push(Finding::blocking(format!(
                "the authored module declares {} direct UConversationCharacterSettings classes; \
                 exactly one voice/dialog anchor is required",
                settings.len()
            )))
        }
    }

    if let Some(visuals) = classes
        .iter()
        .find(|class| class.name.starts_with("UCharacterVisualsDefinition_Human_"))
    {
        let prebaked = assigned(visuals, "m_HasPreBakedSK").into_iter().next();
        let name = assigned(visuals, "m_PreBakedName").into_iter().next();
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
        match after
            .iter()
            .find(|other| other.name == class.name && other.namespace == class.namespace)
        {
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
        if !before
            .iter()
            .any(|other| other.name == class.name && other.namespace == class.namespace)
        {
            findings.push(Finding::blocking(format!(
                "class {} is new. Checking a shipped character out is for changing its values; a \
                 new class needs `gore npc new`, which carries the contract for one",
                class.name
            )));
        }
    }

    if pristine == edited {
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

    const ACTOR_BLUEPRINT: &str = "n\"Blueprint'/Game/AI/AIAgent/Human/AIAgentCharacter_Human_Base.AIAgentCharacter_Human_Base_C'\"";

    fn guard_authored_module(source: &str, npc_id: &str) -> Vec<Finding> {
        super::guard_authored_module(source, npc_id, ACTOR_BLUEPRINT)
    }

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
    fn a_new_spawn_must_match_the_selected_world_point_and_generated_call() {
        let source = format!(
            "{PRISTINE}\nclass UWP_B : UWorldPointScript\n{{\n    void OnWorldStart()\n    {{\n        return;\n    }}\n}}\n"
        );
        let class = "USpawnAIAgentDefinition_MINE";
        let valid = edit::add_spawn(&source, "UWP_A", class, None).unwrap();
        assert!(guard_generated_spawn(&source, &valid, "UWP_A", class).is_empty());
        let with_routine =
            edit::add_spawn(&source, "UWP_A", class, Some("UDailyRoutine_MINE_Start")).unwrap();
        assert!(guard_generated_spawn(&source, &with_routine, "UWP_A", class).is_empty());

        let wrong_point = edit::add_spawn(&source, "UWP_B", class, None).unwrap();
        assert!(!guard_generated_spawn(&source, &wrong_point, "UWP_A", class).is_empty());
        let comment = source.replace(
            "        return;",
            "        // this.SpawnAIAgent(USpawnAIAgentDefinition_MINE::StaticClass());\n        return;",
        );
        assert!(!guard_generated_spawn(&source, &comment, "UWP_A", class).is_empty());
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
    fn removing_a_prefixed_other_character_is_not_an_allowed_level_edit() {
        let pristine = PRISTINE.replace(
            "        return;",
            "        this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_Diego_Prime::StaticClass()), nullptr);\n        return;",
        );
        let edited = pristine
            .lines()
            .filter(|line| {
                !line.contains("USpawnAIAgentDefinition_Diego::StaticClass()")
                    && !line.contains("USpawnAIAgentDefinition_Diego_Prime::StaticClass()")
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!guard_level_diff(&pristine, &edited, "USpawnAIAgentDefinition_Diego").is_empty());
    }

    #[test]
    fn suppression_rejects_a_restored_or_relocated_spawn() {
        let source = format!(
            "{PRISTINE}\nclass UWP_B : UWorldPointScript\n{{\n    void OnWorldStart()\n    {{\n        return;\n    }}\n}}\n"
        );
        let suppressed = edit::remove_spawn(&source, "USpawnAIAgentDefinition_Diego").unwrap();
        assert!(
            guard_suppressed_spawn(&source, &suppressed, "USpawnAIAgentDefinition_Diego")
                .is_empty()
        );
        assert!(
            !guard_suppressed_spawn(&source, &source, "USpawnAIAgentDefinition_Diego").is_empty()
        );
        let relocated =
            edit::add_spawn(&suppressed, "UWP_B", "USpawnAIAgentDefinition_Diego", None).unwrap();
        assert!(
            !guard_suppressed_spawn(&source, &relocated, "USpawnAIAgentDefinition_Diego")
                .is_empty()
        );
    }

    #[test]
    fn a_modified_pristine_copy_is_blocking_even_if_authored_source_matches_it() {
        assert!(guard_pristine_source(Some(PRISTINE), PRISTINE, "LevelScripts.Test").is_empty());
        let altered = PRISTINE.replace("return;", "DoSomethingElse();");
        assert!(!guard_pristine_source(Some(PRISTINE), &altered, "LevelScripts.Test").is_empty());
        assert!(!guard_pristine_source(None, PRISTINE, "LevelScripts.Test").is_empty());
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

class UAIAgentConfig_Human_MINE : UAIAgentConfig_Human_OC_STT_Diego
{
    default m_CharacterDefinition = UCharacterDefinition_Human_MINE::StaticClass();
}

class USpawnAIAgentDefinition_MINE : USpawnAIAgentDefinition_OC_STT_Diego
{
    default AIAgentConfigClass = UAIAgentConfig_Human_MINE::StaticClass();
    default AIAgentCharacterClass = n"Blueprint'/Game/AI/AIAgent/Human/AIAgentCharacter_Human_Base.AIAgentCharacter_Human_Base_C'";
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
    fn authored_classes_inside_a_namespace_are_blocking() {
        let source = format!("namespace Example\n{{\n{AUTHORED}\n}}\n");
        let findings = guard_authored_module(&source, "MINE");
        assert!(findings.iter().any(|finding| {
            finding.severity == Severity::Blocking
                && finding.message.contains("UCharacterDefinition_Human_MINE")
                && finding.message.contains("namespace Example")
        }));
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
    fn identity_fields_require_exact_name_literals() {
        for (original, replacement, field) in [
            (
                "default m_UniqueName = n\"MINE\";",
                "default m_UniqueName = nMINE;",
                "m_UniqueName",
            ),
            (
                "default ForCharacter = n\"MINE\";",
                "default ForCharacter = nMINE;",
                "ForCharacter",
            ),
        ] {
            let source = AUTHORED.replacen(original, replacement, 1);
            let findings = guard_authored_module(&source, "MINE");
            assert!(findings.iter().any(|finding| {
                finding.severity == Severity::Blocking && finding.message.contains(field)
            }));
        }
    }

    #[test]
    fn an_identity_default_after_the_class_body_is_missing() {
        let source = AUTHORED
            .replacen("    default m_UniqueName = n\"MINE\";\n", "", 1)
            .replacen(
                "}\n\nclass UAIAgentConfig_Human_MINE",
                "}\ndefault m_UniqueName = n\"MINE\";\n\nclass UAIAgentConfig_Human_MINE",
                1,
            );
        let findings = guard_authored_module(&source, "MINE");
        assert!(findings.iter().any(|finding| {
            finding.severity == Severity::Blocking && finding.message.contains("m_UniqueName")
        }));
    }

    #[test]
    fn a_commented_out_unique_name_is_blocking() {
        let source = AUTHORED.replace(
            "    default m_UniqueName = n\"MINE\";",
            "    /*\n    default m_UniqueName = n\"MINE\";\n    */",
        );
        let findings = guard_authored_module(&source, "MINE");
        assert!(findings.iter().any(|finding| {
            finding.severity == Severity::Blocking && finding.message.contains("m_UniqueName")
        }));
    }

    #[test]
    fn duplicate_identity_assignments_are_blocking() {
        for (anchor, duplicate, field) in [
            (
                "    default m_UniqueName = n\"MINE\";",
                "    default m_UniqueName = n\"OTHER\";",
                "m_UniqueName",
            ),
            (
                "    default ForCharacter = n\"MINE\";",
                "    default ForCharacter = n\"OTHER\";",
                "ForCharacter",
            ),
        ] {
            let source = AUTHORED.replace(anchor, &format!("{anchor}\n{duplicate}"));
            let findings = guard_authored_module(&source, "MINE");
            assert!(findings.iter().any(|finding| {
                finding.severity == Severity::Blocking && finding.message.contains(field)
            }));
        }
    }

    #[test]
    fn authored_spawn_chain_must_target_its_own_classes() {
        for (from, to, field) in [
            (
                "UAIAgentConfig_Human_MINE::StaticClass()",
                "UAIAgentConfig_Human_OC_STT_Diego::StaticClass()",
                "AIAgentConfigClass",
            ),
            (
                "UCharacterDefinition_Human_MINE::StaticClass()",
                "UCharacterDefinition_Human_OC_STT_Diego::StaticClass()",
                "m_CharacterDefinition",
            ),
        ] {
            let findings = guard_authored_module(&AUTHORED.replace(from, to), "MINE");
            assert!(findings.iter().any(|finding| {
                finding.severity == Severity::Blocking && finding.message.contains(field)
            }));
        }
    }

    #[test]
    fn authored_spawn_chain_rejects_conditional_static_class_expressions() {
        for (call, field) in [
            ("UCharacterDefinition_Human_MINE::StaticClass()", "m_CharacterDefinition"),
            ("UAIAgentConfig_Human_MINE::StaticClass()", "AIAgentConfigClass"),
        ] {
            let source = AUTHORED.replace(call, &format!(
                "false ? {call} : UOther::StaticClass()"
            ));
            let findings = guard_authored_module(&source, "MINE");
            assert!(findings.iter().any(|finding| {
                finding.severity == Severity::Blocking && finding.message.contains(field)
            }));
        }
    }

    #[test]
    fn authored_definition_must_link_to_its_visuals_class() {
        let expected = "UCharacterVisualsDefinition_Human_MINE::StaticClass()";
        let missing = AUTHORED.replace(
            &format!("    default m_CharacterVisualsDefinition = {expected};\n"),
            "",
        );
        let findings = guard_authored_module(&missing, "MINE");
        assert!(findings.iter().any(|finding| {
            finding.severity == Severity::Blocking
                && finding.message.contains("m_CharacterVisualsDefinition")
        }));
        for replacement in [
            "UCharacterVisualsDefinition_Human_Other::StaticClass()",
            "false ? UCharacterVisualsDefinition_Human_MINE::StaticClass() : UOther::StaticClass()",
        ] {
            let source = AUTHORED.replace(expected, replacement);
            let findings = guard_authored_module(&source, "MINE");
            assert!(findings.iter().any(|finding| {
                finding.severity == Severity::Blocking
                    && finding.message.contains("m_CharacterVisualsDefinition")
            }));
        }
    }

    #[test]
    fn authored_spawn_requires_an_actor_blueprint() {
        for replacement in ["", "    default AIAgentCharacterClass = nullptr;\n"] {
            let source = AUTHORED.replace(
                "    default AIAgentCharacterClass = n\"Blueprint'/Game/AI/AIAgent/Human/AIAgentCharacter_Human_Base.AIAgentCharacter_Human_Base_C'\";\n",
                replacement,
            );
            let findings = guard_authored_module(&source, "MINE");
            assert!(findings.iter().any(|finding| {
                finding.severity == Severity::Blocking
                    && finding.message.contains("AIAgentCharacterClass")
            }));
        }
    }

    #[test]
    fn authored_spawn_rejects_a_nonexistent_actor_blueprint() {
        let source = AUTHORED.replace(ACTOR_BLUEPRINT, "n\"Bogus\"");
        let findings = guard_authored_module(&source, "MINE");
        assert!(findings.iter().any(|finding| {
            finding.severity == Severity::Blocking
                && finding.message.contains("AIAgentCharacterClass")
                && finding.message.contains("Bogus")
        }));
    }

    #[test]
    fn authored_spawn_chain_requires_all_classes() {
        for class in ["UAIAgentConfig_Human_MINE", "USpawnAIAgentDefinition_MINE"] {
            let source = AUTHORED.replace(
                &format!("class {class}"),
                &format!("class UMissing_{class}"),
            );
            let findings = guard_authored_module(&source, "MINE");
            assert!(findings.iter().any(|finding| {
                finding.severity == Severity::Blocking && finding.message.contains(class)
            }));
        }
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
    fn a_renamed_conversation_anchor_is_blocking_even_with_the_right_character() {
        let source = AUTHORED.replace(
            "UConversationCharacterSettings_Ambient_MINE : UConversationCharacterSettings",
            "UOtherSettings : UConversationCharacterSettings",
        );
        let findings = guard_authored_module(&source, "MINE");
        assert!(findings.iter().any(|finding| {
            finding.severity == Severity::Blocking
                && finding
                    .message
                    .contains("UConversationCharacterSettings_Ambient_MINE")
        }));
    }

    #[test]
    fn an_authored_module_requires_one_conversation_anchor() {
        let missing = AUTHORED.replace(
            "UConversationCharacterSettings_Ambient_MINE : UConversationCharacterSettings",
            "UConversationCharacterSettings_Ambient_MINE : UMissingSettingsBase",
        );
        let findings = guard_authored_module(&missing, "MINE");
        assert!(findings.iter().any(|finding| {
            finding.severity == Severity::Blocking
                && finding.message.contains("0 direct")
                && finding.message.contains("exactly one")
        }));

        let duplicate = format!(
            "{AUTHORED}\nclass UExtraSettings : UConversationCharacterSettings\n{{\n    default ForCharacter = n\"MINE\";\n}}\n"
        );
        let findings = guard_authored_module(&duplicate, "MINE");
        assert!(findings.iter().any(|finding| {
            finding.severity == Severity::Blocking
                && finding.message.contains("2 direct")
                && finding.message.contains("exactly one")
        }));
    }

    #[test]
    fn a_changed_value_in_a_checked_out_module_passes() {
        let edited = AUTHORED.replace("1000.0f", "500.0f");
        assert!(guard_checkout_diff(AUTHORED, &edited).is_empty());
    }

    #[test]
    fn wrapping_checked_out_classes_in_a_namespace_is_blocking() {
        let edited = format!("namespace Example {{\n{AUTHORED}\n}}\n");
        let findings = guard_checkout_diff(AUTHORED, &edited);
        assert!(findings.iter().any(|finding| {
            finding.severity == Severity::Blocking
                && finding.message.contains("UCharacterDefinition_Human_MINE")
                && finding.message.contains("gone")
        }));
    }

    #[test]
    fn a_changed_method_body_in_a_checked_out_module_passes() {
        let pristine = "class UX : UY\n{\n    void Run()\n    {\n        Before();\n    }\n}\n";
        let edited = pristine.replace("Before();", "After();");
        assert_eq!(defaults::parse_classes(pristine), defaults::parse_classes(&edited));
        assert!(guard_checkout_diff(pristine, &edited).is_empty());
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
