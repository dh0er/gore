//! Den AngelScript-Quelltext einer verfassten Figur erzeugen.
//!
//! Alle Klassen einer Figur kommen in **ein** Modul. Vanilla verteilt sie auf vier plus zwei
//! geteilte (`Spawning/SpawningDefinition_Human.as` mit 6648 Zeilen und
//! `InteractiveObjects/NpcVisualLibrary.as`); die mitzuändern hieße, drei weitere geteilte Module
//! zur Kollisionsfläche zu machen, ohne etwas zu gewinnen. AngelScript registriert Klassen global,
//! die Modulzuordnung ist frei.
//!
//! Die Klassenformen stammen aus dem ausgelieferten Baum, nicht aus Annahmen: `OC_STT_Diego` und
//! seine Nachbarn sind die Vorlage für jede Zeile, die hier erzeugt wird.

/// Woraus eine neue Figur gebaut wird.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewNpc {
    /// Die Id der neuen Figur, z.B. `MY_NPC`.
    pub id: String,
    /// Die ausgelieferte Figur, von der abgeleitet wird, z.B. `OC_STT_Diego`.
    pub derived_from: String,
    /// Gildenbasis ohne Präfix, z.B. `OldCamp_Guard`. `None` erbt die der Vorlage.
    pub guild: Option<String>,
    /// Wegpunkt für den Tagesablauf. `None` lässt den Tagesablauf weg.
    pub waypoint: Option<String>,
    /// Stimme als Tag-Zusatz, z.B. `VoiceType_G1R_Voice05_Diego`. `None` lässt die Stimme weg.
    pub voice_tag: Option<String>,
    /// `true` baut das Aussehen zur Laufzeit aus Teilen statt die vorgebackene Gestalt zu erben.
    pub modular_visuals: bool,
    /// `true` fügt eine leere Händlerkonfiguration hinzu.
    pub trader: bool,
    /// Die Elternklasse der Figurendefinition, und die Werte, die dabei ausgeschrieben werden.
    ///
    /// Nicht die Vorlage selbst: der Compiler erklärt das erzeugte `__InitDefaults` einer Klasse
    /// **ohne Unterklassen** für `final`, und eine ausgelieferte Figur ist fast immer ein solches
    /// Blatt. Von ihr abzuleiten scheitert mit
    /// `declared as final and cannot be overridden`. Also wird von ihrem nächsten Vorfahren mit
    /// Geschwistern abgeleitet und alles Übersprungene hier ausgeschrieben.
    pub definition_parent: String,
    pub definition_defaults: Vec<String>,
    /// Dasselbe für das Aussehen.
    pub visuals_parent: String,
    pub visuals_defaults: Vec<String>,
    /// Dasselbe für Bindeglied und Spawn-Handle.
    ///
    /// Sie tragen sehr wohl eigene Werte, und einer davon ist tragend: die Spawn-Definition nennt
    /// mit `AIAgentCharacterClass` den Actor-Blueprint, der einer Figur ihren Körper gibt. Ohne
    /// ihn entsteht ein Agent ohne Darsteller — im Spielstand vorhanden, in der Welt ohne
    /// Skelett, ohne Animation, nicht fokussierbar. Im Spiel gesehen, nicht vermutet.
    pub config_parent: String,
    pub config_defaults: Vec<String>,
    pub spawn_parent: String,
    pub spawn_defaults: Vec<String>,
}

/// Der Modulname einer verfassten Figur.
pub fn module_name(id: &str) -> String {
    format!("AI.AIAgent.Human.Config.{id}.{id}")
}

/// Der Pfad relativ zum `Script/`-Baum.
pub fn relative_path(id: &str) -> String {
    format!("AI/AIAgent/Human/Config/{id}/{id}.as")
}

/// Die Klasse, die den Tagesablauf der Figur trägt, wenn sie einen hat.
pub fn routine_class(npc: &NewNpc) -> Option<String> {
    npc.waypoint
        .as_ref()
        .map(|_| format!("UDailyRoutine_{}_Start", npc.id))
}

/// Die Spawn-Definition der Figur — das, was eine Spawn-Zeile im Levelskript nennt.
pub fn spawn_class(id: &str) -> String {
    format!("USpawnAIAgentDefinition_{id}")
}

/// Ein Klassenblock aus Kopfzeile und `default`-Zeilen, im Stil des emittierten Baums.
///
/// Konstruktoren werden bewusst nicht erzeugt: der Compiler legt sie selbst an, und eine von Hand
/// geschriebene Fassung wäre eine weitere Stelle, an der die Neuübersetzung abweichen könnte.
fn class_block(name: &str, parent: &str, defaults: &[String]) -> String {
    let mut out = format!("class {name} : {parent}\n{{\n");
    for line in defaults {
        out.push_str("    default ");
        out.push_str(line);
        out.push_str(";\n");
    }
    out.push_str("}\n");
    out
}

/// Der vollständige Quelltext des Moduls.
pub fn source(npc: &NewNpc) -> String {
    let id = npc.id.as_str();
    let from = npc.derived_from.as_str();

    let mut out = format!(
        "// gore npc: authored character {id}, derived from {from}.\n\
         // Everything not spelled out below is inherited from that character.\n\n"
    );

    // Die Fraktion ist keine Eigenschaft, sondern die Elternklasse. `--guild` tauscht genau die
    // aus; sonst gilt die vom Aufrufer ermittelte ableitbare Elternklasse.
    let definition_parent = match &npc.guild {
        Some(guild) => format!("UCharacterDefinition_Human_{guild}"),
        None => npc.definition_parent.clone(),
    };
    let mut definition_defaults = vec![
        format!("m_UniqueName = n\"{id}\""),
        format!(
            "m_CharacterVisualsDefinition = UCharacterVisualsDefinition_Human_{id}::StaticClass()"
        ),
    ];
    definition_defaults.extend(npc.definition_defaults.iter().cloned());
    out.push_str(&class_block(
        &format!("UCharacterDefinition_Human_{id}"),
        &definition_parent,
        &definition_defaults,
    ));
    out.push('\n');

    if npc.modular_visuals {
        out.push_str(
            "// Built from parts at runtime instead of borrowing a baked model. Measured in game on\n\
             // 2026-09-05: this renders a working, animated, focusable body — but it came out\n\
             // looking like the player character, not like the template. The parts do not carry\n\
             // across. Prefer the borrowed model unless you know why you want this.\n",
        );
        let mut visuals = npc.visuals_defaults.clone();
        visuals.retain(|line| {
            !line.starts_with("m_PreBakedName") && !line.starts_with("m_HasPreBakedSK")
        });
        visuals.push("m_HasPreBakedSK = false".to_string());
        out.push_str(&class_block(
            &format!("UCharacterVisualsDefinition_Human_{id}"),
            &npc.visuals_parent,
            &visuals,
        ));
    } else {
        let mut visuals = npc.visuals_defaults.clone();
        visuals.retain(|line| {
            !line.starts_with("Person =")
                && !line.starts_with("m_PreBakedName")
                && !line.starts_with("m_HasPreBakedSK")
        });
        visuals.push(format!("Person = \"{from}\""));
        visuals.push(format!("m_PreBakedName = \"{from}\""));
        visuals.push("m_HasPreBakedSK = true".to_string());
        out.push_str(&class_block(
            &format!("UCharacterVisualsDefinition_Human_{id}"),
            &npc.visuals_parent,
            &visuals,
        ));
    }
    out.push('\n');

    let mut config_defaults = vec![format!(
        "m_CharacterDefinition = UCharacterDefinition_Human_{id}::StaticClass()"
    )];
    config_defaults.extend(
        npc.config_defaults
            .iter()
            .filter(|line| !line.starts_with("m_CharacterDefinition"))
            .cloned(),
    );
    out.push_str(&class_block(
        &format!("UAIAgentConfig_Human_{id}"),
        &npc.config_parent,
        &config_defaults,
    ));
    out.push('\n');

    let mut spawn_defaults = vec![format!(
        "AIAgentConfigClass = UAIAgentConfig_Human_{id}::StaticClass()"
    )];
    spawn_defaults.extend(
        npc.spawn_defaults
            .iter()
            .filter(|line| !line.starts_with("AIAgentConfigClass"))
            .cloned(),
    );
    out.push_str(&class_block(
        &spawn_class(id),
        &npc.spawn_parent,
        &spawn_defaults,
    ));
    out.push('\n');

    // Der Gesprächsanker. Das Spiel findet ihn über `ForCharacter`, nicht über den Klassennamen;
    // `_Ambient_` folgt der Form, die 808 ausgelieferte Figuren in ihrem eigenen Konfigurationsmodul
    // tragen. Er ist zugleich das Stück, das `gore dialog new-conversation` für eine neue Figur
    // bisher vermisst hat.
    let mut settings = vec![format!("ForCharacter = n\"{id}\"")];
    if let Some(voice) = &npc.voice_tag {
        settings.push(format!(
            "VoiceTypeSubsets.Add(FVoiceTypeSubset(GameplayTag::{voice}))"
        ));
    }
    out.push_str(&class_block(
        &format!("UConversationCharacterSettings_Ambient_{id}"),
        "UConversationCharacterSettings",
        &settings,
    ));

    if let Some(waypoint) = &npc.waypoint {
        out.push('\n');
        out.push_str(&class_block(
            &format!("UDailyRoutine_{id}_Start"),
            "UAIState_DailyRoutine_Human",
            &[
                format!(
                    "Schedule(0, 0, UAIState_Stand(), n\"{waypoint}\", 1000.0f, \
                     TSubclassOf<UNavArea>(nullptr), nullptr)"
                ),
                // Ohne das steht die Figur an ihrem Weltpunkt, und wo der liegt weiss niemand:
                // Weltpunkte stehen nicht im Ortskatalog. Mit Teleport landet sie am Wegpunkt,
                // und der hat Koordinaten, die man nachschlagen und ansteuern kann.
                // 280 ausgelieferte Tagesablaeufe machen es genauso.
                "TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode(1)".to_string(),
            ],
        ));
    }

    if npc.trader {
        out.push('\n');
        out.push_str(
            "// The wares go here, as `AddTraderItemAllDifficulties(...)` lines. See\n\
             // `LevelScripts/TradersData.as` in an emitted tree for the shipped shape. `m_Region`\n\
             // and `m_Type` group the shop in the UI and are left to the base class on purpose:\n\
             // inventing values for them would be a guess, not a default.\n",
        );
        out.push_str(&class_block(
            &format!("UTraderConfig_{id}"),
            "UTraderConfigBase",
            &[format!("m_UniqueName = n\"{id}\"")],
        ));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diego_clone() -> NewNpc {
        NewNpc {
            id: "MY_NPC".to_string(),
            derived_from: "OC_STT_Diego".to_string(),
            guild: None,
            waypoint: Some("FP_OC_SMALLTALK_33".to_string()),
            voice_tag: Some("VoiceType_G1R_Voice05_Diego".to_string()),
            modular_visuals: false,
            trader: false,
            definition_parent: "UCharacterDefinition_Human_OldCamp_Shadow".to_string(),
            definition_defaults: Vec::new(),
            visuals_parent: "UArmorVisualsDefinition_MaleNPC".to_string(),
            visuals_defaults: Vec::new(),
            config_parent: "UAIAgentConfig_Human".to_string(),
            config_defaults: Vec::new(),
            spawn_parent: "USpawnAIAgentDefinition".to_string(),
            spawn_defaults: vec![
                "AIAgentCharacterClass = n\"Blueprint'/Game/AI/AIAgent/Human/AIAgentCharacter_Human_Base.AIAgentCharacter_Human_Base_C'\""
                    .to_string(),
            ],
        }
    }

    #[test]
    fn the_module_name_and_path_follow_the_shipped_layout() {
        assert_eq!(
            module_name("MY_NPC"),
            "AI.AIAgent.Human.Config.MY_NPC.MY_NPC"
        );
        assert_eq!(
            relative_path("MY_NPC"),
            "AI/AIAgent/Human/Config/MY_NPC/MY_NPC.as"
        );
    }

    #[test]
    fn the_chain_links_the_three_classes_to_each_other() {
        let source = source(&diego_clone());
        assert!(source.contains(
            "class UCharacterDefinition_Human_MY_NPC : UCharacterDefinition_Human_OldCamp_Shadow"
        ));
        assert!(source.contains(r#"default m_UniqueName = n"MY_NPC";"#));
        assert!(source.contains("class UAIAgentConfig_Human_MY_NPC : UAIAgentConfig_Human"));
        assert!(source.contains(
            "default m_CharacterDefinition = UCharacterDefinition_Human_MY_NPC::StaticClass();"
        ));
        assert!(source.contains("class USpawnAIAgentDefinition_MY_NPC : USpawnAIAgentDefinition"));
        assert!(source
            .contains("default AIAgentConfigClass = UAIAgentConfig_Human_MY_NPC::StaticClass();"));
    }

    #[test]
    fn a_guild_override_replaces_the_character_definitions_parent() {
        let mut npc = diego_clone();
        npc.guild = Some("OldCamp_Guard".to_string());
        let source = source(&npc);
        assert!(source.contains(
            "class UCharacterDefinition_Human_MY_NPC : UCharacterDefinition_Human_OldCamp_Guard"
        ));
        // Die übrigen Glieder bleiben, wie der Aufrufer sie ermittelt hat.
        assert!(source.contains("class UAIAgentConfig_Human_MY_NPC : UAIAgentConfig_Human"));
    }

    #[test]
    fn the_borrowed_prebaked_model_is_the_default() {
        let source = source(&diego_clone());
        assert!(source.contains(
            "class UCharacterVisualsDefinition_Human_MY_NPC : UArmorVisualsDefinition_MaleNPC"
        ));
        assert!(source.contains(r#"default m_PreBakedName = "OC_STT_Diego";"#));
        assert!(source.contains("default m_HasPreBakedSK = true;"));
        assert!(source.contains(
            "default m_CharacterVisualsDefinition = UCharacterVisualsDefinition_Human_MY_NPC::StaticClass();"
        ));
    }

    #[test]
    fn modular_visuals_turn_the_prebaked_model_off_and_say_it_is_unproven() {
        let mut npc = diego_clone();
        npc.modular_visuals = true;
        let source = source(&npc);
        assert!(source.contains("default m_HasPreBakedSK = false;"));
        assert!(!source.contains("m_PreBakedName"));
        // 817 ausgelieferte Figuren tragen ein vorgebackenes Modell, null bauen sich zur Laufzeit
        // zusammen. Wer diesen Weg nimmt, muss das in der Quelle lesen können.
        assert!(source.contains("looking like the player character"));
    }

    #[test]
    fn the_conversation_settings_anchor_the_new_id_and_carry_the_voice() {
        let source = source(&diego_clone());
        assert!(source.contains(
            "class UConversationCharacterSettings_Ambient_MY_NPC : UConversationCharacterSettings"
        ));
        assert!(source.contains(r#"default ForCharacter = n"MY_NPC";"#));
        assert!(source.contains(
            "default VoiceTypeSubsets.Add(FVoiceTypeSubset(GameplayTag::VoiceType_G1R_Voice05_Diego));"
        ));
    }

    #[test]
    fn without_a_voice_the_settings_still_anchor_the_id() {
        let mut npc = diego_clone();
        npc.voice_tag = None;
        let source = source(&npc);
        assert!(source.contains(r#"default ForCharacter = n"MY_NPC";"#));
        assert!(!source.contains("VoiceTypeSubsets"));
    }

    #[test]
    fn a_waypoint_produces_a_daily_routine_and_none_leaves_it_out() {
        let with_waypoint = source(&diego_clone());
        assert!(with_waypoint
            .contains("class UDailyRoutine_MY_NPC_Start : UAIState_DailyRoutine_Human"));
        assert!(with_waypoint.contains(r#"n"FP_OC_SMALLTALK_33""#));
        assert_eq!(
            routine_class(&diego_clone()).as_deref(),
            Some("UDailyRoutine_MY_NPC_Start")
        );

        let mut npc = diego_clone();
        npc.waypoint = None;
        assert!(!source(&npc).contains("UDailyRoutine_MY_NPC_Start"));
        assert_eq!(routine_class(&npc), None);
    }

    #[test]
    fn a_routine_teleports_the_character_to_its_waypoint() {
        // Sonst steht sie an ihrem Weltpunkt, und dessen Lage kennt niemand — Weltpunkte stehen
        // nicht im Ortskatalog. Am Wegpunkt ist sie auffindbar, weil der Koordinaten hat.
        // Im Spiel gelernt: die erste Testfigur war schlicht nicht zu finden.
        assert!(source(&diego_clone())
            .contains("default TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode(1);"));
    }

    #[test]
    fn without_a_waypoint_there_is_nothing_to_teleport_to() {
        let mut npc = diego_clone();
        npc.waypoint = None;
        assert!(!source(&npc).contains("TeleportToCurrentTaskWhen"));
    }

    #[test]
    fn the_schedule_keeps_the_shipped_argument_shape() {
        // Wörtlich die Form aus `DailyRoutine_OC_STT_Diego.as`: Stunde, Minute, Zustand,
        // Wegpunkt, Reichweite, NavArea, Zusatz.
        assert!(source(&diego_clone()).contains(
            "Schedule(0, 0, UAIState_Stand(), n\"FP_OC_SMALLTALK_33\", 1000.0f, TSubclassOf<UNavArea>(nullptr), nullptr);"
        ));
    }

    #[test]
    fn a_trader_gets_an_empty_configuration_keyed_by_its_unique_name() {
        let mut npc = diego_clone();
        npc.trader = true;
        let with_trader = source(&npc);
        assert!(with_trader.contains("class UTraderConfig_MY_NPC : UTraderConfigBase"));
        assert!(!source(&diego_clone()).contains("UTraderConfigBase"));
    }

    #[test]
    fn the_source_names_where_it_came_from() {
        // Wer die Datei in einem halben Jahr öffnet, muss ohne Manifest sehen, was hier erbt.
        let source = source(&diego_clone());
        assert!(source.starts_with("// gore npc:"));
        assert!(source.contains("OC_STT_Diego"));
    }

    #[test]
    fn no_constructor_is_emitted() {
        // Der Compiler legt sie selbst an. Eine handgeschriebene wäre eine weitere Stelle, an der
        // die Neuübersetzung vom Ausgelieferten abweichen könnte.
        assert!(!source(&diego_clone()).contains("MY_NPC()"));
    }

    #[test]
    fn without_inherited_defaults_the_definition_only_states_its_identity() {
        let source = source(&diego_clone());
        assert!(!source.contains("SetAttributeValue"));
    }

    #[test]
    fn inherited_defaults_are_written_out_below_the_identity() {
        // Der Unterschied zwischen Ableiten und Klonen: hier stehen die Werte in der Datei und
        // sind aenderbar, statt unsichtbar von der Vorlage zu kommen.
        let mut npc = diego_clone();
        npc.definition_defaults = vec![
            "SetAttributeValue(\"AttributeSet_Health.Health\", 540.0f, TSubclassOf<UDifficultySettings>(nullptr))".to_string(),
            "m_Personality = UGothicCharacterPersonality_Brave_Archer_Patient::StaticClass()".to_string(),
        ];
        let source = source(&npc);
        assert!(source.contains("default SetAttributeValue(\"AttributeSet_Health.Health\", 540.0f"));
        assert!(source.contains("default m_Personality = UGothicCharacterPersonality_Brave_Archer_Patient::StaticClass();"));
        // Die Identitaet steht weiterhin zuerst und wird nicht ueberschrieben.
        let unique = source.find("m_UniqueName").expect("unique name");
        let health = source.find("AttributeSet_Health").expect("health");
        assert!(unique < health);
    }

    #[test]
    fn the_generator_never_derives_from_the_template_itself() {
        // Der Compiler erklaert das erzeugte `__InitDefaults` einer Klasse ohne Unterklassen fuer
        // `final`. Eine ausgelieferte Figur ist so ein Blatt, und von ihr abzuleiten scheitert mit
        // `declared as final and cannot be overridden` — nach 32 Minuten Uebersetzung, nicht
        // sofort. Der Aufrufer sucht deshalb den naechsten Vorfahren mit Geschwistern; dieser Test
        // haelt fest, dass der Generator wirklich nur den benutzt, was ihm gegeben wurde.
        let source = source(&diego_clone());
        for leaf in [
            "UCharacterDefinition_Human_OC_STT_Diego",
            "UAIAgentConfig_Human_OC_STT_Diego",
            "USpawnAIAgentDefinition_OC_STT_Diego",
        ] {
            assert!(
                !source.contains(&format!(": {leaf}")),
                "derives from the template leaf {leaf}"
            );
        }
    }

    #[test]
    fn the_spawn_definition_names_the_actor_blueprint() {
        // Das Stueck, dessen Fehlen im Spiel als "stocksteif, keine Animation, verschwindet beim
        // Naeherkommen" ankam: ohne `AIAgentCharacterClass` bekommt der Agent keinen Darsteller.
        // Er steht dann im Spielstand und hat trotzdem keinen Koerper.
        let source = source(&diego_clone());
        assert!(source.contains("default AIAgentCharacterClass = n\"Blueprint'/Game/AI/AIAgent/Human/AIAgentCharacter_Human_Base.AIAgentCharacter_Human_Base_C'\";"));
    }

    #[test]
    fn our_own_link_wins_over_the_carried_one() {
        // Die mitgeschleppten Werte der Vorlage duerfen die Kette nicht zurueckbiegen: der Verweis
        // auf das naechste Glied gehoert der neuen Figur.
        let mut npc = diego_clone();
        npc.spawn_defaults = vec![
            "AIAgentConfigClass = UAIAgentConfig_Human_OC_STT_Diego::StaticClass()".to_string(),
        ];
        npc.config_defaults = vec![
            "m_CharacterDefinition = UCharacterDefinition_Human_OC_STT_Diego::StaticClass()"
                .to_string(),
        ];
        let source = source(&npc);
        assert!(!source.contains("AIAgentConfigClass = UAIAgentConfig_Human_OC_STT_Diego"));
        assert!(!source.contains("m_CharacterDefinition = UCharacterDefinition_Human_OC_STT_Diego"));
        assert!(source
            .contains("default AIAgentConfigClass = UAIAgentConfig_Human_MY_NPC::StaticClass();"));
    }

    #[test]
    fn every_class_block_is_closed() {
        // Ein unbalanciertes Erzeugnis würde erst der Compiler nach vielen Minuten melden.
        for npc in [
            diego_clone(),
            NewNpc {
                trader: true,
                modular_visuals: true,
                waypoint: None,
                voice_tag: None,
                guild: Some("OldCamp_Guard".to_string()),
                ..diego_clone()
            },
        ] {
            let source = source(&npc);
            assert_eq!(
                source.matches('{').count(),
                source.matches('}').count(),
                "unbalanced braces for {npc:?}"
            );
        }
    }
}
