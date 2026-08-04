use gore_authoring::{
    LogicalNpcCloneAuthoringStatus, LogicalNpcCloneDraft, LogicalNpcCloneDraftError,
    LogicalNpcCloneField, LogicalNpcCloneRuntimeStatus, LOGICAL_NPC_CLONE_GENERATOR_ID,
    LOGICAL_NPC_CLONE_GENERATOR_VERSION, MAX_ANGELSCRIPT_IDENTIFIER_BYTES,
    MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES, MAX_ANGELSCRIPT_MODULE_SEGMENTS,
    MAX_LOGICAL_NPC_UNIQUE_NAME_BYTES,
};

const MODULE: &str = "GoreMods.Probe.NpcLogicalCloneV1";
const UNIQUE_NAME: &str = "GORE_LOGICAL_ASGHAN_CLONE_V1";
const PARENT_CHARACTER: &str = "UCharacterDefinition_Human_OM_GRD_Asghan_263";
const PARENT_AGENT: &str = "UAIAgentConfig_Human_OM_GRD_Asghan_263";
const PARENT_SPAWN: &str = "USpawnAIAgentDefinition_OM_GRD_Asghan_263";

fn valid_draft() -> LogicalNpcCloneDraft {
    LogicalNpcCloneDraft::new(
        MODULE,
        UNIQUE_NAME,
        PARENT_CHARACTER,
        PARENT_AGENT,
        PARENT_SPAWN,
    )
    .unwrap()
}

#[test]
fn asghan_fixture_generates_the_offline_proven_three_class_shape() {
    let draft = valid_draft();
    let generated = draft.generate();

    assert_eq!(generated.generator_id, LOGICAL_NPC_CLONE_GENERATOR_ID);
    assert_eq!(
        generated.generator_version,
        LOGICAL_NPC_CLONE_GENERATOR_VERSION
    );
    assert_eq!(generated.module_namespace, MODULE);
    assert_eq!(
        generated.module_relative_path,
        "GoreMods/Probe/NpcLogicalCloneV1.as"
    );
    assert_eq!(generated.unique_name, UNIQUE_NAME);
    assert_eq!(
        generated.classes.character_definition,
        "UCharacterDefinition_Human_GORE_LOGICAL_ASGHAN_CLONE_V1"
    );
    assert_eq!(
        generated.classes.ai_agent_config,
        "UAIAgentConfig_Human_GORE_LOGICAL_ASGHAN_CLONE_V1"
    );
    assert_eq!(
        generated.classes.spawn_definition,
        "USpawnAIAgentDefinition_GORE_LOGICAL_ASGHAN_CLONE_V1"
    );
    assert_eq!(
        generated.source,
        concat!(
            "class UCharacterDefinition_Human_GORE_LOGICAL_ASGHAN_CLONE_V1\n",
            "    : UCharacterDefinition_Human_OM_GRD_Asghan_263\n",
            "{\n",
            "    default m_UniqueName = n\"GORE_LOGICAL_ASGHAN_CLONE_V1\";\n",
            "}\n",
            "\n",
            "class UAIAgentConfig_Human_GORE_LOGICAL_ASGHAN_CLONE_V1\n",
            "    : UAIAgentConfig_Human_OM_GRD_Asghan_263\n",
            "{\n",
            "    default m_CharacterDefinition =\n",
            "        UCharacterDefinition_Human_GORE_LOGICAL_ASGHAN_CLONE_V1::StaticClass();\n",
            "}\n",
            "\n",
            "class USpawnAIAgentDefinition_GORE_LOGICAL_ASGHAN_CLONE_V1\n",
            "    : USpawnAIAgentDefinition_OM_GRD_Asghan_263\n",
            "{\n",
            "    default AIAgentConfigClass =\n",
            "        UAIAgentConfig_Human_GORE_LOGICAL_ASGHAN_CLONE_V1::StaticClass();\n",
            "}\n",
        )
    );
    assert_eq!(generated.source.len(), 618);
    assert_eq!(
        generated.source_sha256.to_string(),
        "c78f366a3701393b2657693b29a7673c38dbe59d1ac2ff6c4fb3c2a51163e5d0"
    );
    assert_eq!(
        generated.input_fingerprint.to_string(),
        "5f27b386533f60b0c7878f53c32ce5cfdccc93cf77f7a4b85d966d3b0125b7d2"
    );
    assert_eq!(generated.input_fingerprint, draft.input_fingerprint());
    assert_eq!(
        generated.status.authoring,
        LogicalNpcCloneAuthoringStatus::OfflineDraft
    );
    assert_eq!(
        generated.status.runtime,
        LogicalNpcCloneRuntimeStatus::RuntimeUnqualified
    );
}

#[test]
fn empty_reserved_injected_and_oversized_inputs_are_rejected() {
    assert!(matches!(
        LogicalNpcCloneDraft::new(
            "",
            UNIQUE_NAME,
            PARENT_CHARACTER,
            PARENT_AGENT,
            PARENT_SPAWN
        ),
        Err(LogicalNpcCloneDraftError::EmptyValue {
            field: LogicalNpcCloneField::ModuleNamespace
        })
    ));
    assert!(matches!(
        LogicalNpcCloneDraft::new(
            "GoreMods..Npc",
            UNIQUE_NAME,
            PARENT_CHARACTER,
            PARENT_AGENT,
            PARENT_SPAWN
        ),
        Err(LogicalNpcCloneDraftError::EmptyValue {
            field: LogicalNpcCloneField::ModuleSegment { index: 1 }
        })
    ));
    assert!(matches!(
        LogicalNpcCloneDraft::new(
            "GoreMods.class.Npc",
            UNIQUE_NAME,
            PARENT_CHARACTER,
            PARENT_AGENT,
            PARENT_SPAWN
        ),
        Err(LogicalNpcCloneDraftError::ReservedIdentifier {
            field: LogicalNpcCloneField::ModuleSegment { index: 1 }
        })
    ));
    assert!(matches!(
        LogicalNpcCloneDraft::new(
            "GoreMods.CON.Npc",
            UNIQUE_NAME,
            PARENT_CHARACTER,
            PARENT_AGENT,
            PARENT_SPAWN
        ),
        Err(LogicalNpcCloneDraftError::ReservedModuleSegment { index: 1 })
    ));
    assert!(matches!(
        LogicalNpcCloneDraft::new(
            MODULE,
            "NPC\";\nclass Injected",
            PARENT_CHARACTER,
            PARENT_AGENT,
            PARENT_SPAWN
        ),
        Err(LogicalNpcCloneDraftError::InvalidIdentifierCharacter {
            field: LogicalNpcCloneField::UniqueName,
            ..
        })
    ));
    assert!(matches!(
        LogicalNpcCloneDraft::new(
            MODULE,
            "CLASS",
            PARENT_CHARACTER,
            PARENT_AGENT,
            PARENT_SPAWN
        ),
        Err(LogicalNpcCloneDraftError::ReservedIdentifier {
            field: LogicalNpcCloneField::UniqueName
        })
    ));
    assert!(matches!(
        LogicalNpcCloneDraft::new(
            MODULE,
            "A".repeat(MAX_LOGICAL_NPC_UNIQUE_NAME_BYTES + 1),
            PARENT_CHARACTER,
            PARENT_AGENT,
            PARENT_SPAWN
        ),
        Err(LogicalNpcCloneDraftError::ValueTooLong {
            field: LogicalNpcCloneField::UniqueName,
            actual,
            max
        }) if actual == MAX_LOGICAL_NPC_UNIQUE_NAME_BYTES + 1
            && max == MAX_LOGICAL_NPC_UNIQUE_NAME_BYTES
    ));
    assert!(matches!(
        LogicalNpcCloneDraft::new(
            "A".repeat(MAX_ANGELSCRIPT_MODULE_NAMESPACE_BYTES + 1),
            UNIQUE_NAME,
            PARENT_CHARACTER,
            PARENT_AGENT,
            PARENT_SPAWN
        ),
        Err(LogicalNpcCloneDraftError::ValueTooLong {
            field: LogicalNpcCloneField::ModuleNamespace,
            ..
        })
    ));
    assert!(matches!(
        LogicalNpcCloneDraft::new(
            format!(
                "GoreMods.{}",
                "A".repeat(MAX_ANGELSCRIPT_IDENTIFIER_BYTES + 1)
            ),
            UNIQUE_NAME,
            PARENT_CHARACTER,
            PARENT_AGENT,
            PARENT_SPAWN
        ),
        Err(LogicalNpcCloneDraftError::ValueTooLong {
            field: LogicalNpcCloneField::ModuleSegment { index: 1 },
            ..
        })
    ));
    let too_many_segments = vec!["A"; MAX_ANGELSCRIPT_MODULE_SEGMENTS + 1].join(".");
    assert!(matches!(
        LogicalNpcCloneDraft::new(
            too_many_segments,
            UNIQUE_NAME,
            PARENT_CHARACTER,
            PARENT_AGENT,
            PARENT_SPAWN
        ),
        Err(LogicalNpcCloneDraftError::TooManyModuleSegments { actual, max })
            if actual == MAX_ANGELSCRIPT_MODULE_SEGMENTS + 1
                && max == MAX_ANGELSCRIPT_MODULE_SEGMENTS
    ));
    assert!(matches!(
        LogicalNpcCloneDraft::new(
            MODULE,
            UNIQUE_NAME,
            "UCharacterDefinition_Human_Parent;Injected",
            PARENT_AGENT,
            PARENT_SPAWN
        ),
        Err(LogicalNpcCloneDraftError::InvalidIdentifierCharacter {
            field: LogicalNpcCloneField::ParentCharacterDefinition,
            ..
        })
    ));
    assert!(matches!(
        LogicalNpcCloneDraft::new(
            MODULE,
            UNIQUE_NAME,
            format!(
                "UCharacterDefinition_{}",
                "A".repeat(MAX_ANGELSCRIPT_IDENTIFIER_BYTES)
            ),
            PARENT_AGENT,
            PARENT_SPAWN
        ),
        Err(LogicalNpcCloneDraftError::ValueTooLong {
            field: LogicalNpcCloneField::ParentCharacterDefinition,
            ..
        })
    ));
    assert!(matches!(
        LogicalNpcCloneDraft::new(
            MODULE,
            UNIQUE_NAME,
            "USomeOtherClass_Parent",
            PARENT_AGENT,
            PARENT_SPAWN
        ),
        Err(LogicalNpcCloneDraftError::UnexpectedParentClassPrefix {
            field: LogicalNpcCloneField::ParentCharacterDefinition,
            ..
        })
    ));
}

#[test]
fn generated_classes_must_not_collide_with_a_parent() {
    let cases = [
        (
            LogicalNpcCloneField::ParentCharacterDefinition,
            "UCharacterDefinition_Human_GORE_NPC",
            PARENT_AGENT,
            PARENT_SPAWN,
        ),
        (
            LogicalNpcCloneField::ParentAiAgentConfig,
            PARENT_CHARACTER,
            "UAIAgentConfig_Human_GORE_NPC",
            PARENT_SPAWN,
        ),
        (
            LogicalNpcCloneField::ParentSpawnDefinition,
            PARENT_CHARACTER,
            PARENT_AGENT,
            "USpawnAIAgentDefinition_GORE_NPC",
        ),
    ];

    for (expected_field, parent_character, parent_agent, parent_spawn) in cases {
        assert!(matches!(
            LogicalNpcCloneDraft::new(
                MODULE,
                "GORE_NPC",
                parent_character,
                parent_agent,
                parent_spawn
            ),
            Err(LogicalNpcCloneDraftError::ClassNameCollision { field, .. })
                if field == expected_field
        ));
    }
}

#[test]
fn generation_is_deterministic_and_side_effect_free() {
    let draft = valid_draft();
    let first = draft.generate();
    let second = draft.generate();

    assert_eq!(first, second);
    assert_eq!(draft.module_namespace(), MODULE);
    assert_eq!(draft.unique_name(), UNIQUE_NAME);
    assert_eq!(draft.parent_character_definition(), PARENT_CHARACTER);
    assert_eq!(draft.parent_ai_agent_config(), PARENT_AGENT);
    assert_eq!(draft.parent_spawn_definition(), PARENT_SPAWN);
    assert_eq!(draft.class_names(), &first.classes);
    assert_eq!(draft.input_fingerprint(), first.input_fingerprint);
    assert_eq!(first.generator_id, LOGICAL_NPC_CLONE_GENERATOR_ID);
    assert_eq!(first.generator_version, LOGICAL_NPC_CLONE_GENERATOR_VERSION);
}

#[test]
fn input_fingerprint_covers_every_input_without_upgrading_runtime() {
    let baseline = valid_draft().generate();
    let variants = [
        (
            "module namespace",
            "GoreMods.Probe.NpcLogicalCloneAlternate",
            UNIQUE_NAME,
            PARENT_CHARACTER,
            PARENT_AGENT,
            PARENT_SPAWN,
            true,
        ),
        (
            "unique name",
            MODULE,
            "GORE_LOGICAL_ASGHAN_CLONE_ALTERNATE",
            PARENT_CHARACTER,
            PARENT_AGENT,
            PARENT_SPAWN,
            false,
        ),
        (
            "character parent",
            MODULE,
            UNIQUE_NAME,
            "UCharacterDefinition_Human_OM_GRD_Asghan_264",
            PARENT_AGENT,
            PARENT_SPAWN,
            false,
        ),
        (
            "AI agent parent",
            MODULE,
            UNIQUE_NAME,
            PARENT_CHARACTER,
            "UAIAgentConfig_Human_OM_GRD_Asghan_264",
            PARENT_SPAWN,
            false,
        ),
        (
            "spawn parent",
            MODULE,
            UNIQUE_NAME,
            PARENT_CHARACTER,
            PARENT_AGENT,
            "USpawnAIAgentDefinition_OM_GRD_Asghan_264",
            false,
        ),
    ];

    for (
        label,
        module_namespace,
        unique_name,
        parent_character,
        parent_agent,
        parent_spawn,
        source_alias,
    ) in variants
    {
        let generated = LogicalNpcCloneDraft::new(
            module_namespace,
            unique_name,
            parent_character,
            parent_agent,
            parent_spawn,
        )
        .unwrap()
        .generate();

        assert_ne!(
            generated.input_fingerprint, baseline.input_fingerprint,
            "fingerprint aliased after changing {label}"
        );
        if source_alias {
            assert_eq!(
                generated.source_sha256, baseline.source_sha256,
                "non-source provenance change {label} unexpectedly changed source bytes"
            );
        } else {
            assert_ne!(
                generated.source_sha256, baseline.source_sha256,
                "source input change {label} aliased source bytes"
            );
        }
        assert_eq!(generated.generator_id, baseline.generator_id);
        assert_eq!(generated.generator_version, baseline.generator_version);
        assert_eq!(generated.status, baseline.status);
        assert_eq!(
            generated.status.authoring,
            LogicalNpcCloneAuthoringStatus::OfflineDraft
        );
        assert_eq!(
            generated.status.runtime,
            LogicalNpcCloneRuntimeStatus::RuntimeUnqualified
        );
    }
}

#[test]
fn input_fingerprint_length_prefixes_adjacent_field_boundaries() {
    let first = LogicalNpcCloneDraft::new(
        "GoreMods.Probe.AB",
        "C",
        PARENT_CHARACTER,
        PARENT_AGENT,
        PARENT_SPAWN,
    )
    .unwrap();
    let second = LogicalNpcCloneDraft::new(
        "GoreMods.Probe.A",
        "BC",
        PARENT_CHARACTER,
        PARENT_AGENT,
        PARENT_SPAWN,
    )
    .unwrap();

    assert_ne!(first.input_fingerprint(), second.input_fingerprint());
}
