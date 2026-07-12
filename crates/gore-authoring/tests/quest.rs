use gore_authoring::{
    CatalogQualifiedParentQuest, CatalogQualifiedQuestGiver, ContentSeal,
    DraftQuestAuthoringStatus, DraftQuestCollisionCatalog, DraftQuestCollisionKind,
    DraftQuestDiscoveryStatus, DraftQuestSkeletonError, DraftQuestSkeletonInput,
    DraftQuestSkeletonV1, DraftQuestTransitionStatus, EntityId, GameGenerationAnchor, Sha256Digest,
    DRAFT_QUEST_GENERATOR_VERSION, MAX_DRAFT_QUEST_CATALOG_LAYER_BYTES,
    MAX_DRAFT_QUEST_DESCRIPTION_BYTES,
};

const MODULE: &str = "GoreMods.Probe.AsghanMiniQuest";
const TECHNICAL_ID: &str = "GORE_PROBE_ASGHAN_MINI";
const HELPER: &str = "GoreProbeAsghanText";
const PARENT: &str = "UQuest_SwampCamp_SCCHAPTER2";
const PARENT_SELECTOR: &str = "CatalogQuest_00263";
const GIVER: &str = "OM_GRD_Asghan_263";
const GIVER_SELECTOR: &str = "CatalogCharacter_00263";
const GIVER_LAYER: &str = "base-game.g1r.characters";
const PARENT_LAYER: &str = "dependency.story-pack.quests";
const COLLISION_LAYER: &str = "resolved-loadout.scripts.v1";

fn digest(byte: &str) -> Sha256Digest {
    byte.repeat(32).parse().unwrap()
}

fn seal(byte: &str, byte_len: u64) -> ContentSeal {
    ContentSeal {
        byte_len,
        sha256: digest(byte),
    }
}

fn generation(byte: &str) -> GameGenerationAnchor {
    GameGenerationAnchor {
        executable: seal(byte, 1_000_000),
    }
}

fn entity_id() -> EntityId {
    "0123456789abcdef0123456789abcdef".parse().unwrap()
}

fn input_with_catalog(catalog: DraftQuestCollisionCatalog) -> DraftQuestSkeletonInput {
    let target = generation("11");
    DraftQuestSkeletonInput {
        target: target.clone(),
        quest_id: entity_id(),
        module_namespace: MODULE.into(),
        technical_id: TECHNICAL_ID.into(),
        text_helper: HELPER.into(),
        parent_quest: CatalogQualifiedParentQuest::new(
            target.clone(),
            seal("44", 4_096),
            PARENT_LAYER,
            PARENT_SELECTOR,
            PARENT,
        )
        .unwrap(),
        giver: CatalogQualifiedQuestGiver::new(
            target,
            seal("22", 8_192),
            GIVER_LAYER,
            GIVER_SELECTOR,
            GIVER,
        )
        .unwrap(),
        title: "Gore probe at Asghan".into(),
        description: "Talk to Asghan once more to complete the probe quest.".into(),
        objective_title: "Talk to Asghan once more".into(),
        collision_catalog: catalog,
    }
}

fn input() -> DraftQuestSkeletonInput {
    input_with_catalog(
        DraftQuestCollisionCatalog::new(
            generation("11"),
            seal("33", 32_768),
            COLLISION_LAYER,
            vec![],
            vec![],
            vec![],
        )
        .unwrap(),
    )
}

fn giver(
    target: &GameGenerationAnchor,
    source_seal_byte: &str,
    layer: &str,
    selector: &str,
    runtime_unique_name: &str,
) -> CatalogQualifiedQuestGiver {
    CatalogQualifiedQuestGiver::new(
        target.clone(),
        seal(source_seal_byte, 8_192),
        layer,
        selector,
        runtime_unique_name,
    )
    .unwrap()
}

fn parent(
    target: &GameGenerationAnchor,
    source_seal_byte: &str,
    layer: &str,
    selector: &str,
    runtime_class: &str,
) -> CatalogQualifiedParentQuest {
    CatalogQualifiedParentQuest::new(
        target.clone(),
        seal(source_seal_byte, 4_096),
        layer,
        selector,
        runtime_class,
    )
    .unwrap()
}

fn collision_catalog(
    target: &GameGenerationAnchor,
    source_seal_byte: &str,
    layer: &str,
    modules: Vec<String>,
    relative_paths: Vec<String>,
    symbols: Vec<String>,
) -> DraftQuestCollisionCatalog {
    DraftQuestCollisionCatalog::new(
        target.clone(),
        seal(source_seal_byte, 32_768),
        layer,
        modules,
        relative_paths,
        symbols,
    )
    .unwrap()
}

#[test]
fn retained_asghan_lines_1_to_59_are_reproduced_byte_exactly() {
    let generated = DraftQuestSkeletonV1::new(input()).unwrap().generate();
    let expected = r#"FText GoreProbeAsghanText(const FName Text)
{
    FString Value = Text.ToString();
    return FText::FromString(Value);
}

class UQuest_GORE_PROBE_ASGHAN_MINI : UG1RQuest
{
    default ParentQuestClass = UQuest_SwampCamp_SCCHAPTER2::StaticClass();
    default QuestKind = EQuestKind::Side;
    default InvolvedCharacters.Add(n"Hero");
    default InvolvedCharacters.Add(n"OM_GRD_Asghan_263");
    default QuestGiverCharacterUniqueName = n"OM_GRD_Asghan_263";
    default NameText = GoreProbeAsghanText(n"Gore probe at Asghan");
    default DescriptionText = GoreProbeAsghanText(
        n"Talk to Asghan once more to complete the probe quest."
    );
    default bExternalStartTrigger = true;
}

UQuest_GORE_PROBE_ASGHAN_MINI GetGoreProbeAsghanMini()
{
    UQuestSubsystem Subsystem = UQuestSubsystem::Get();
    if (Subsystem == nullptr)
        return nullptr;

    TSubclassOf<UQuest> QuestClass =
        TSubclassOf<UQuest>(UQuest_GORE_PROBE_ASGHAN_MINI::StaticClass());
    UQuest Quest = Subsystem.GetQuestByClass(QuestClass);
    if (Quest == nullptr)
        return nullptr;

    return Cast<UQuest_GORE_PROBE_ASGHAN_MINI>(Quest);
}

class UQuest_GORE_PROBE_ASGHAN_MINI_OBJ_DONE : UG1RQuest
{
    default ParentQuestClass = UQuest_GORE_PROBE_ASGHAN_MINI::StaticClass();
    default QuestKind = EQuestKind::Subobjective;
    default NameText = GoreProbeAsghanText(n"Talk to Asghan once more");
    default bExternalStartTrigger = true;
    default bExternalSuccessTrigger = true;
    default bSucceedParent = true;
}

UQuest_GORE_PROBE_ASGHAN_MINI_OBJ_DONE GetGoreProbeAsghanMiniObjective()
{
    UQuestSubsystem Subsystem = UQuestSubsystem::Get();
    if (Subsystem == nullptr)
        return nullptr;

    TSubclassOf<UQuest> QuestClass =
        TSubclassOf<UQuest>(UQuest_GORE_PROBE_ASGHAN_MINI_OBJ_DONE::StaticClass());
    UQuest Quest = Subsystem.GetQuestByClass(QuestClass);
    if (Quest == nullptr)
        return nullptr;

    return Cast<UQuest_GORE_PROBE_ASGHAN_MINI_OBJ_DONE>(Quest);
}
"#;
    assert_eq!(generated.source, expected);
    assert_eq!(generated.source.len(), 2_008);
    assert_eq!(
        generated.source_sha256.to_string(),
        "eb38bf814685485977113cf67a679d4b4cb309a2dbcd229fae3a6d57f2a4ae82"
    );
    assert_eq!(
        generated.input_fingerprint.to_string(),
        "5987a4b5147fb76f34af3cf0f926f0c7de2450d4e370c1aee3d88bcf8121de93"
    );
    assert!(!generated.source.contains('\r'));
    assert!(generated.source.ends_with('\n'));
}

#[test]
fn capability_is_always_offline_and_runtime_unqualified() {
    let generated = DraftQuestSkeletonV1::new(input()).unwrap().generate();
    assert_eq!(generated.generator_version, DRAFT_QUEST_GENERATOR_VERSION);
    assert_eq!(
        generated.status.authoring,
        DraftQuestAuthoringStatus::OfflineDraft
    );
    assert_eq!(
        generated.status.discovery,
        DraftQuestDiscoveryStatus::RuntimeUnqualified
    );
    assert_eq!(
        generated.status.transitions,
        DraftQuestTransitionStatus::TransitionsRuntimeUnqualified
    );
    assert_eq!(generated.fixed_shape.quest_base_class, "UG1RQuest");
    assert!(generated.fixed_shape.root_external_start);
    assert!(generated.fixed_shape.objective_external_start);
    assert!(generated.fixed_shape.objective_external_success);
    assert!(generated.fixed_shape.objective_succeeds_parent);
}

#[test]
fn source_contains_only_defaults_and_read_only_getters() {
    let source = DraftQuestSkeletonV1::new(input())
        .unwrap()
        .generate()
        .source;
    for forbidden in [
        "UChoice",
        "UFUNCTION",
        "ShouldStart",
        "ShouldSucceed",
        "ShouldFail",
        "HandleQuest",
        "StartQuest(",
        "SucceedQuest(",
        "FailQuest(",
        "EndConversation",
        "UnlockDocument",
        "GiveExperience",
        "Knowledge",
        "ActedTopics",
        "Reward",
        "Journal",
    ] {
        assert!(!source.contains(forbidden), "forbidden token {forbidden}");
    }
    assert_eq!(source.matches("class UQuest_").count(), 2);
    assert_eq!(source.matches("GetQuestByClass").count(), 2);
}

#[test]
fn generation_and_catalog_seals_fail_closed_without_qualifying_runtime() {
    let mut bad_target = input();
    bad_target.target.executable.byte_len = 0;
    assert!(matches!(
        DraftQuestSkeletonV1::new(bad_target),
        Err(DraftQuestSkeletonError::InvalidSeal { .. })
    ));

    let mut mismatched_giver = input();
    mismatched_giver.giver = CatalogQualifiedQuestGiver::new(
        generation("44"),
        seal("22", 8_192),
        GIVER_LAYER,
        GIVER_SELECTOR,
        GIVER,
    )
    .unwrap();
    assert!(matches!(
        DraftQuestSkeletonV1::new(mismatched_giver),
        Err(DraftQuestSkeletonError::GenerationMismatch {
            field: gore_authoring::DraftQuestField::GiverGeneration
        })
    ));

    let mut mismatched_parent = input();
    mismatched_parent.parent_quest = CatalogQualifiedParentQuest::new(
        generation("55"),
        seal("44", 4_096),
        PARENT_LAYER,
        PARENT_SELECTOR,
        PARENT,
    )
    .unwrap();
    assert!(matches!(
        DraftQuestSkeletonV1::new(mismatched_parent),
        Err(DraftQuestSkeletonError::GenerationMismatch {
            field: gore_authoring::DraftQuestField::ParentGeneration
        })
    ));

    let mismatched_collision = DraftQuestCollisionCatalog::new(
        generation("66"),
        seal("33", 32_768),
        COLLISION_LAYER,
        vec![],
        vec![],
        vec![],
    )
    .unwrap();
    assert!(matches!(
        DraftQuestSkeletonV1::new(input_with_catalog(mismatched_collision)),
        Err(DraftQuestSkeletonError::GenerationMismatch {
            field: gore_authoring::DraftQuestField::CollisionGeneration
        })
    ));

    assert!(CatalogQualifiedQuestGiver::new(
        generation("11"),
        seal("22", 0),
        GIVER_LAYER,
        GIVER_SELECTOR,
        GIVER,
    )
    .is_err());
    assert!(CatalogQualifiedParentQuest::new(
        generation("11"),
        seal("44", 0),
        PARENT_LAYER,
        PARENT_SELECTOR,
        PARENT,
    )
    .is_err());
    assert!(DraftQuestCollisionCatalog::new(
        generation("11"),
        seal("33", 0),
        COLLISION_LAYER,
        vec![],
        vec![],
        vec![],
    )
    .is_err());
}

#[test]
fn identifiers_and_literals_reject_reserved_words_injection_and_length_overflow() {
    let mutations: [fn(&mut DraftQuestSkeletonInput); 3] = [
        |input: &mut DraftQuestSkeletonInput| input.module_namespace = "GoreMods.CON.Bad".into(),
        |input: &mut DraftQuestSkeletonInput| input.technical_id = "GORE__BAD".into(),
        |input: &mut DraftQuestSkeletonInput| input.text_helper = "class".into(),
    ];
    for mutate in mutations {
        let mut candidate = input();
        mutate(&mut candidate);
        assert!(DraftQuestSkeletonV1::new(candidate).is_err());
    }

    let mut injected = input();
    injected.title = "Bad\"; StartQuest(nullptr); //".into();
    assert!(DraftQuestSkeletonV1::new(injected).is_err());

    let mut newline = input();
    newline.objective_title = "line one\nline two".into();
    assert!(DraftQuestSkeletonV1::new(newline).is_err());

    let mut too_long = input();
    too_long.description = "x".repeat(MAX_DRAFT_QUEST_DESCRIPTION_BYTES + 1);
    assert!(matches!(
        DraftQuestSkeletonV1::new(too_long),
        Err(DraftQuestSkeletonError::ValueTooLong { .. })
    ));

    assert!(CatalogQualifiedQuestGiver::new(
        generation("11"),
        seal("22", 1),
        GIVER_LAYER,
        GIVER_SELECTOR,
        "Asghan\";StartQuest",
    )
    .is_err());
    assert!(CatalogQualifiedQuestGiver::new(
        generation("11"),
        seal("22", 1),
        GIVER_LAYER,
        "Selector\";StartQuest",
        GIVER,
    )
    .is_err());
    assert!(CatalogQualifiedParentQuest::new(
        generation("11"),
        seal("44", 1),
        PARENT_LAYER,
        PARENT_SELECTOR,
        "UG1RQuest",
    )
    .is_err());
}

#[test]
fn generated_symbols_are_pairwise_unique_and_gore_as_generated_names_are_reserved() {
    for helper in [
        "UQuest_GORE_PROBE_ASGHAN_MINI",
        "UQuest_GORE_PROBE_ASGHAN_MINI_OBJ_DONE",
        "GetGoreProbeAsghanMini",
        "getgoreprobeasghanminiobjective",
    ] {
        let mut candidate = input();
        candidate.text_helper = helper.into();
        assert!(matches!(
            DraftQuestSkeletonV1::new(candidate),
            Err(DraftQuestSkeletonError::GeneratedSymbolCollision { .. })
        ));
    }

    for generated_name in [
        "StaticClass",
        "staticclass",
        "Spawn",
        "Get",
        "GetOrCreate",
        "Create",
        "GetG1R",
    ] {
        let mut candidate = input();
        candidate.text_helper = generated_name.into();
        assert!(matches!(
            DraftQuestSkeletonV1::new(candidate),
            Err(DraftQuestSkeletonError::ReservedIdentifier { .. })
        ));
    }
}

#[test]
fn catalog_layers_are_bounded_canonical_and_retained_without_cross_layer_retargeting() {
    for invalid in [
        "",
        "Base-game.g1r",
        "base..game",
        "base/game",
        "1base-game",
        "base-game.",
    ] {
        assert!(CatalogQualifiedQuestGiver::new(
            generation("11"),
            seal("22", 1),
            invalid,
            GIVER_SELECTOR,
            GIVER,
        )
        .is_err());
    }
    assert!(CatalogQualifiedQuestGiver::new(
        generation("11"),
        seal("22", 1),
        format!("a{}", "b".repeat(MAX_DRAFT_QUEST_CATALOG_LAYER_BYTES)),
        GIVER_SELECTOR,
        GIVER,
    )
    .is_err());
    assert!(CatalogQualifiedParentQuest::new(
        generation("11"),
        seal("44", 1),
        "dependency story",
        PARENT_SELECTOR,
        PARENT,
    )
    .is_err());
    assert!(DraftQuestCollisionCatalog::new(
        generation("11"),
        seal("33", 1),
        "resolved-loadout/scripts",
        vec![],
        vec![],
        vec![],
    )
    .is_err());

    let generated = DraftQuestSkeletonV1::new(input()).unwrap().generate();
    assert_eq!(generated.giver.catalog_layer(), GIVER_LAYER);
    assert_eq!(generated.giver.canonical_selector(), GIVER_SELECTOR);
    assert_eq!(generated.giver.runtime_unique_name(), GIVER);
    assert_eq!(generated.parent_quest.catalog_layer(), PARENT_LAYER);
    assert_eq!(generated.parent_quest.canonical_selector(), PARENT_SELECTOR);
    assert_eq!(generated.parent_quest.runtime_class(), PARENT);
    assert!(!generated.source.contains(GIVER_SELECTOR));
    assert!(!generated.source.contains(PARENT_SELECTOR));
    assert_eq!(generated.collision_catalog.catalog_layer(), COLLISION_LAYER);
    assert_ne!(
        generated.giver.catalog_layer(),
        generated.parent_quest.catalog_layer()
    );
    assert_ne!(
        generated.parent_quest.catalog_layer(),
        generated.collision_catalog.catalog_layer()
    );
    assert_eq!(
        generated.status.discovery,
        DraftQuestDiscoveryStatus::RuntimeUnqualified
    );
}

#[test]
fn collision_catalog_rejects_unsafe_ambiguous_and_generated_names_case_insensitively() {
    for (modules, paths, symbols) in [
        (vec!["GoreMods.CON.Bad".into()], vec![], vec![]),
        (vec![], vec!["../escape.as".into()], vec![]),
        (vec![], vec!["GoreMods\\Bad.as".into()], vec![]),
        (vec![], vec![], vec!["Bad();".into()]),
    ] {
        assert!(DraftQuestCollisionCatalog::new(
            generation("11"),
            seal("33", 1),
            COLLISION_LAYER,
            modules,
            paths,
            symbols,
        )
        .is_err());
    }
    assert!(matches!(
        DraftQuestCollisionCatalog::new(
            generation("11"),
            seal("33", 1),
            COLLISION_LAYER,
            vec!["Existing.Module".into(), "existing.module".into()],
            vec![],
            vec![],
        ),
        Err(DraftQuestSkeletonError::DuplicateCollisionEntry { .. })
    ));

    for catalog in [
        DraftQuestCollisionCatalog::new(
            generation("11"),
            seal("33", 1),
            COLLISION_LAYER,
            vec![MODULE.to_ascii_lowercase()],
            vec![],
            vec![],
        )
        .unwrap(),
        DraftQuestCollisionCatalog::new(
            generation("11"),
            seal("33", 1),
            COLLISION_LAYER,
            vec![],
            vec!["goremods/probe/asghanminiquest.as".into()],
            vec![],
        )
        .unwrap(),
        DraftQuestCollisionCatalog::new(
            generation("11"),
            seal("33", 1),
            COLLISION_LAYER,
            vec![],
            vec![],
            vec!["uquest_gore_probe_asghan_mini".into()],
        )
        .unwrap(),
    ] {
        assert!(matches!(
            DraftQuestSkeletonV1::new(input_with_catalog(catalog)),
            Err(DraftQuestSkeletonError::GeneratedNameCollision { .. })
        ));
    }
}

#[test]
fn same_immutable_inputs_generate_identical_source_and_metadata() {
    let first = DraftQuestSkeletonV1::new(input()).unwrap().generate();
    let second = DraftQuestSkeletonV1::new(input()).unwrap().generate();
    assert_eq!(first, second);
    assert_eq!(first.quest_id, entity_id());
    assert_eq!(first.technical_names.module_namespace, MODULE);
    assert_eq!(
        first.technical_names.module_relative_path,
        "GoreMods/Probe/AsghanMiniQuest.as"
    );
    assert_eq!(
        first.technical_names.root_class,
        "UQuest_GORE_PROBE_ASGHAN_MINI"
    );
    assert_eq!(
        first.technical_names.objective_class,
        "UQuest_GORE_PROBE_ASGHAN_MINI_OBJ_DONE"
    );
    assert_eq!(first.input_fingerprint, second.input_fingerprint);
    assert_eq!(
        first.input_fingerprint,
        DraftQuestSkeletonV1::new(input())
            .unwrap()
            .input_fingerprint()
    );
}

#[test]
fn input_fingerprint_covers_source_semantics_and_non_emitted_provenance() {
    let baseline = DraftQuestSkeletonV1::new(input()).unwrap().generate();
    let mut variants = Vec::<(&str, DraftQuestSkeletonInput, bool)>::new();

    let mut changed = input();
    changed.quest_id = "fedcba9876543210fedcba9876543210".parse().unwrap();
    variants.push(("entity id", changed, true));

    let mut changed = input();
    changed.module_namespace = "GoreMods.Probe.AsghanMiniQuestAlternate".into();
    variants.push(("module namespace", changed, true));

    let mut changed = input();
    changed.technical_id = "GORE_PROBE_ASGHAN_MINI_ALT".into();
    variants.push(("technical id", changed, false));

    let mut changed = input();
    changed.text_helper = "GoreProbeAsghanAlternateText".into();
    variants.push(("text helper", changed, false));

    let mut changed = input();
    changed.giver = giver(
        &changed.target,
        "22",
        "dependency.characters.v2",
        GIVER_SELECTOR,
        GIVER,
    );
    variants.push(("giver layer", changed, true));

    let mut changed = input();
    changed.giver = giver(&changed.target, "25", GIVER_LAYER, GIVER_SELECTOR, GIVER);
    variants.push(("giver seal", changed, true));

    let mut changed = input();
    changed.giver = CatalogQualifiedQuestGiver::new(
        changed.target.clone(),
        seal("22", 8_193),
        GIVER_LAYER,
        GIVER_SELECTOR,
        GIVER,
    )
    .unwrap();
    variants.push(("giver seal byte length", changed, true));

    let mut changed = input();
    changed.giver = giver(
        &changed.target,
        "22",
        GIVER_LAYER,
        "CatalogCharacter_00264",
        GIVER,
    );
    variants.push(("giver selector", changed, true));

    let mut changed = input();
    changed.giver = giver(
        &changed.target,
        "22",
        GIVER_LAYER,
        GIVER_SELECTOR,
        "OM_GRD_Asghan_264",
    );
    variants.push(("giver runtime name", changed, false));

    let mut changed = input();
    changed.parent_quest = parent(
        &changed.target,
        "44",
        "base-game.g1r.quests",
        PARENT_SELECTOR,
        PARENT,
    );
    variants.push(("parent layer", changed, true));

    let mut changed = input();
    changed.parent_quest = parent(&changed.target, "45", PARENT_LAYER, PARENT_SELECTOR, PARENT);
    variants.push(("parent seal", changed, true));

    let mut changed = input();
    changed.parent_quest = CatalogQualifiedParentQuest::new(
        changed.target.clone(),
        seal("44", 4_097),
        PARENT_LAYER,
        PARENT_SELECTOR,
        PARENT,
    )
    .unwrap();
    variants.push(("parent seal byte length", changed, true));

    let mut changed = input();
    changed.parent_quest = parent(
        &changed.target,
        "44",
        PARENT_LAYER,
        "CatalogQuest_00264",
        PARENT,
    );
    variants.push(("parent selector", changed, true));

    let mut changed = input();
    changed.parent_quest = parent(
        &changed.target,
        "44",
        PARENT_LAYER,
        PARENT_SELECTOR,
        "UQuest_SwampCamp_SCCHAPTER3",
    );
    variants.push(("parent runtime class", changed, false));

    let mut changed = input();
    changed.title = "Alternate Gore probe at Asghan".into();
    variants.push(("title", changed, false));

    let mut changed = input();
    changed.description = "Alternate read-only discovery description.".into();
    variants.push(("description", changed, false));

    let mut changed = input();
    changed.objective_title = "Alternate read-only objective".into();
    variants.push(("objective title", changed, false));

    let mut changed = input();
    changed.collision_catalog = collision_catalog(
        &changed.target,
        "33",
        "resolved-loadout.scripts.v2",
        vec![],
        vec![],
        vec![],
    );
    variants.push(("collision layer", changed, true));

    let mut changed = input();
    changed.collision_catalog = collision_catalog(
        &changed.target,
        "35",
        COLLISION_LAYER,
        vec![],
        vec![],
        vec![],
    );
    variants.push(("collision seal", changed, true));

    let mut changed = input();
    changed.collision_catalog = DraftQuestCollisionCatalog::new(
        changed.target.clone(),
        seal("33", 32_769),
        COLLISION_LAYER,
        vec![],
        vec![],
        vec![],
    )
    .unwrap();
    variants.push(("collision seal byte length", changed, true));

    let mut changed = input();
    changed.collision_catalog = collision_catalog(
        &changed.target,
        "33",
        COLLISION_LAYER,
        vec!["Existing.Module".into()],
        vec![],
        vec![],
    );
    variants.push(("collision modules", changed, true));

    let mut changed = input();
    changed.collision_catalog = collision_catalog(
        &changed.target,
        "33",
        COLLISION_LAYER,
        vec![],
        vec!["Existing/Module.as".into()],
        vec![],
    );
    variants.push(("collision paths", changed, true));

    let mut changed = input();
    changed.collision_catalog = collision_catalog(
        &changed.target,
        "33",
        COLLISION_LAYER,
        vec![],
        vec![],
        vec!["ExistingSymbol".into()],
    );
    variants.push(("collision symbols", changed, true));

    let mut changed = input();
    let target = generation("77");
    changed.target = target.clone();
    changed.giver = giver(&target, "22", GIVER_LAYER, GIVER_SELECTOR, GIVER);
    changed.parent_quest = parent(&target, "44", PARENT_LAYER, PARENT_SELECTOR, PARENT);
    changed.collision_catalog =
        collision_catalog(&target, "33", COLLISION_LAYER, vec![], vec![], vec![]);
    variants.push(("game and anchor generations", changed, true));

    let mut changed = input();
    let target = GameGenerationAnchor {
        executable: seal("11", 1_000_001),
    };
    changed.target = target.clone();
    changed.giver = giver(&target, "22", GIVER_LAYER, GIVER_SELECTOR, GIVER);
    changed.parent_quest = parent(&target, "44", PARENT_LAYER, PARENT_SELECTOR, PARENT);
    changed.collision_catalog =
        collision_catalog(&target, "33", COLLISION_LAYER, vec![], vec![], vec![]);
    variants.push(("generation byte length", changed, true));

    for (label, candidate, source_alias) in variants {
        let generated = DraftQuestSkeletonV1::new(candidate).unwrap().generate();
        assert_ne!(
            generated.input_fingerprint, baseline.input_fingerprint,
            "fingerprint aliased after changing {label}"
        );
        if source_alias {
            assert_eq!(
                generated.source_sha256, baseline.source_sha256,
                "provenance-only change {label} unexpectedly changed source"
            );
            assert_eq!(generated.source, baseline.source);
        } else {
            assert_ne!(
                generated.source_sha256, baseline.source_sha256,
                "source input {label} unexpectedly aliased source bytes"
            );
        }
        assert_eq!(
            generated.status, baseline.status,
            "input change {label} changed runtime qualification"
        );
    }
}

#[test]
fn collision_inventory_fingerprint_is_ordered_and_casefold_canonical() {
    let target = generation("11");
    let first = input_with_catalog(collision_catalog(
        &target,
        "33",
        COLLISION_LAYER,
        vec!["Existing.ModuleB".into(), "Existing.ModuleA".into()],
        vec!["Existing/B.as".into(), "Existing/A.as".into()],
        vec!["ExistingSymbolB".into(), "ExistingSymbolA".into()],
    ));
    let second = input_with_catalog(collision_catalog(
        &target,
        "33",
        COLLISION_LAYER,
        vec!["existing.modulea".into(), "existing.moduleb".into()],
        vec!["existing/a.as".into(), "existing/b.as".into()],
        vec!["existingsymbola".into(), "existingsymbolb".into()],
    ));
    let first = DraftQuestSkeletonV1::new(first).unwrap().generate();
    let second = DraftQuestSkeletonV1::new(second).unwrap().generate();
    assert_eq!(first.source, second.source);
    assert_eq!(first.input_fingerprint, second.input_fingerprint);
}

#[test]
fn input_fingerprint_length_prefixes_field_boundaries() {
    let mut first = input();
    first.title = "AB".into();
    first.description = "C".into();
    let mut second = input();
    second.title = "A".into();
    second.description = "BC".into();

    let first = DraftQuestSkeletonV1::new(first).unwrap().generate();
    let second = DraftQuestSkeletonV1::new(second).unwrap().generate();
    assert_ne!(first.input_fingerprint, second.input_fingerprint);
}

#[test]
fn parent_may_not_alias_either_generated_class() {
    for parent in [
        "UQuest_GORE_PROBE_ASGHAN_MINI",
        "UQuest_gore_probe_asghan_mini_obj_done",
    ] {
        let mut candidate = input();
        candidate.parent_quest = CatalogQualifiedParentQuest::new(
            generation("11"),
            seal("44", 4_096),
            PARENT_LAYER,
            PARENT_SELECTOR,
            parent,
        )
        .unwrap();
        assert!(matches!(
            DraftQuestSkeletonV1::new(candidate),
            Err(DraftQuestSkeletonError::ParentClassCollision { .. })
        ));
    }
}

#[test]
fn collision_error_names_its_domain() {
    let catalog = DraftQuestCollisionCatalog::new(
        generation("11"),
        seal("33", 1),
        COLLISION_LAYER,
        vec![MODULE.into()],
        vec![],
        vec![],
    )
    .unwrap();
    assert!(matches!(
        DraftQuestSkeletonV1::new(input_with_catalog(catalog)),
        Err(DraftQuestSkeletonError::GeneratedNameCollision {
            kind: DraftQuestCollisionKind::Module,
            ..
        })
    ));
}
