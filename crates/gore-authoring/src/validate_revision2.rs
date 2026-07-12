use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::model_revision2::{
    Entity, EntityKind, EntityPayload, GeneratedStoryIdentity, NpcDraft, OriginRef,
    ProjectRevision2, QuestDraft, ScriptModule, StoryRegenerationError, TypedRef,
};
use crate::{
    Diagnostic, DiagnosticCode, EntityId, Sha256Digest, ValidationProfile,
    DRAFT_QUEST_GENERATOR_ID, DRAFT_QUEST_GENERATOR_VERSION, LOGICAL_NPC_CLONE_GENERATOR_ID,
    LOGICAL_NPC_CLONE_GENERATOR_VERSION,
};

impl ProjectRevision2 {
    /// Validate revision-2 story entities under production policy in deterministic order.
    ///
    /// This intentionally does not claim complete revision-2 build readiness: migrated voice,
    /// dialog, localization, and asset semantics still require the future combined validator.
    pub fn validate_story_entities(&self) -> Vec<Diagnostic> {
        self.validate_story_entities_with_profile(ValidationProfile::Production)
    }

    /// Validate durable story intent, generator reproducibility, and bidirectional ownership.
    ///
    /// Experimental policy permits retaining runtime-unqualified drafts as warnings. It never
    /// relaxes malformed input, reference, provenance, ownership, or generated-byte drift.
    pub fn validate_story_entities_with_profile(
        &self,
        profile: ValidationProfile,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if self.target.executable.byte_len == 0 {
            diagnostics.push(Diagnostic::project_error(
                DiagnosticCode::InvalidGenerationAnchor,
                "target.executable.byte_len",
                "game generation executable seal must have a non-zero byte length",
            ));
        }

        for (map_id, entity) in &self.entities {
            if map_id != &entity.id {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::EntityKeyIdMismatch,
                        *map_id,
                        "id",
                        format!(
                            "entity map key {map_id} does not match embedded id {}",
                            entity.id
                        ),
                    )
                    .related(entity.id),
                );
            }

            match &entity.payload {
                EntityPayload::NpcDraft(draft) => {
                    validate_npc_draft(
                        self,
                        *map_id,
                        &entity.origin,
                        draft,
                        profile,
                        &mut diagnostics,
                    );
                }
                EntityPayload::QuestDraft(draft) => {
                    validate_quest_draft(
                        self,
                        *map_id,
                        &entity.origin,
                        draft,
                        profile,
                        &mut diagnostics,
                    );
                }
                EntityPayload::ScriptModule(module) => {
                    validate_script_module(self, *map_id, entity, module, &mut diagnostics);
                }
                EntityPayload::LocalizationEntry(_)
                | EntityPayload::DialogLine(_)
                | EntityPayload::VoiceSlot(_)
                | EntityPayload::VoiceTake(_) => {}
            }
        }

        validate_global_story_identities(self, &mut diagnostics);

        for diagnostic in &mut diagnostics {
            diagnostic.related_entities.sort_unstable();
            diagnostic.related_entities.dedup();
        }
        diagnostics.sort_by(|left, right| {
            (
                left.severity,
                left.entity,
                left.property_path.as_deref(),
                left.code,
                left.message.as_str(),
                left.related_entities.as_slice(),
            )
                .cmp(&(
                    right.severity,
                    right.entity,
                    right.property_path.as_deref(),
                    right.code,
                    right.message.as_str(),
                    right.related_entities.as_slice(),
                ))
        });
        diagnostics
    }
}

fn validate_global_story_identities(project: &ProjectRevision2, diagnostics: &mut Vec<Diagnostic>) {
    let mut runtime_ids = BTreeMap::<String, (EntityId, String)>::new();
    let mut module_namespaces = BTreeMap::<String, (EntityId, String)>::new();
    let mut module_paths = BTreeMap::<String, (EntityId, String)>::new();
    let mut symbols = BTreeMap::<String, (EntityId, String)>::new();

    for (id, entity) in &project.entities {
        if matches!(
            &entity.payload,
            EntityPayload::NpcDraft(_) | EntityPayload::QuestDraft(_)
        ) {
            if let OriginRef::New {
                authored_runtime_id,
            } = &entity.origin
            {
                push_identity_collision(
                    &mut runtime_ids,
                    authored_runtime_id,
                    *id,
                    DiagnosticCode::DuplicateAuthoredRuntimeId,
                    "origin.authored_runtime_id",
                    "authored runtime identity",
                    diagnostics,
                );
            }
        }

        if let EntityPayload::ScriptModule(module) = &entity.payload {
            push_identity_collision(
                &mut module_namespaces,
                &module.module_namespace,
                *id,
                DiagnosticCode::DuplicateScriptModuleNamespace,
                "payload.data.module_namespace",
                "script module namespace",
                diagnostics,
            );
            push_identity_collision(
                &mut module_paths,
                &module.module_relative_path,
                *id,
                DiagnosticCode::DuplicateScriptModulePath,
                "payload.data.module_relative_path",
                "script module path",
                diagnostics,
            );
        }
    }

    for (id, entity) in &project.entities {
        let generated = match &entity.payload {
            EntityPayload::NpcDraft(draft) => draft.regenerate_script_module_with_identity(
                TypedRef::new(project.project_id, *id, EntityKind::NpcDraft),
            ),
            EntityPayload::QuestDraft(draft) => draft.regenerate_script_module_with_identity(
                TypedRef::new(project.project_id, *id, EntityKind::QuestDraft),
            ),
            _ => continue,
        };
        let Ok((_, GeneratedStoryIdentity { symbols: names, .. })) = generated else {
            // The draft-specific validator already emits the authoritative regeneration error.
            continue;
        };
        let module_id = match &entity.payload {
            EntityPayload::NpcDraft(draft) => draft.script_module.id,
            EntityPayload::QuestDraft(draft) => draft.script_module.id,
            _ => unreachable!(),
        };
        for symbol in names {
            push_identity_collision(
                &mut symbols,
                &symbol,
                module_id,
                DiagnosticCode::DuplicateGeneratedSymbol,
                "payload.data.source",
                "generated AngelScript symbol",
                diagnostics,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn push_identity_collision(
    seen: &mut BTreeMap<String, (EntityId, String)>,
    value: &str,
    owner: EntityId,
    code: DiagnosticCode,
    property_path: &str,
    label: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let folded = value.to_ascii_lowercase();
    match seen.get(&folded) {
        Some((first_owner, first_value)) if first_owner != &owner => diagnostics.push(
            Diagnostic::error(
                code,
                owner,
                property_path,
                format!("{label} {value:?} collides case-insensitively with {first_value:?}"),
            )
            .related(*first_owner),
        ),
        Some(_) => {}
        None => {
            seen.insert(folded, (owner, value.to_owned()));
        }
    }
}

fn validate_npc_draft(
    project: &ProjectRevision2,
    owner: EntityId,
    origin: &OriginRef,
    draft: &NpcDraft,
    profile: ValidationProfile,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_new_draft_origin(owner, origin, &draft.input.unique_name, diagnostics);
    if draft.input.target != project.target {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::InvalidGeneratorInput,
            owner,
            "payload.data.input.target",
            "NPC generator target does not match the project target",
        ));
    }
    let target = validate_required_ref(
        project,
        owner,
        "payload.data.script_module",
        &draft.script_module,
        EntityKind::ScriptModule,
        diagnostics,
    );
    let owner_ref = TypedRef::new(project.project_id, owner, EntityKind::NpcDraft);
    validate_regeneration(
        owner,
        draft.script_module.id,
        target,
        draft.regenerate_script_module(owner_ref),
        diagnostics,
    );
    push_runtime_unqualified(owner, EntityKind::NpcDraft, profile, diagnostics);
}

fn validate_quest_draft(
    project: &ProjectRevision2,
    owner: EntityId,
    origin: &OriginRef,
    draft: &QuestDraft,
    profile: ValidationProfile,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_new_draft_origin(owner, origin, &draft.input.technical_id, diagnostics);
    if draft.input.target != project.target {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::InvalidGeneratorInput,
            owner,
            "payload.data.input.target",
            "quest generator target does not match the project target",
        ));
    }
    if draft.input.quest_id != owner {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::InvalidGeneratorInput,
            owner,
            "payload.data.input.quest_id",
            format!(
                "quest generator entity id {} does not match owner {owner}",
                draft.input.quest_id
            ),
        ));
    }
    let target = validate_required_ref(
        project,
        owner,
        "payload.data.script_module",
        &draft.script_module,
        EntityKind::ScriptModule,
        diagnostics,
    );
    let owner_ref = TypedRef::new(project.project_id, owner, EntityKind::QuestDraft);
    validate_regeneration(
        owner,
        draft.script_module.id,
        target,
        draft.regenerate_script_module(owner_ref),
        diagnostics,
    );
    push_runtime_unqualified(owner, EntityKind::QuestDraft, profile, diagnostics);
}

fn validate_new_draft_origin(
    owner: EntityId,
    origin: &OriginRef,
    expected_runtime_id: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if matches!(
        origin,
        OriginRef::New {
            authored_runtime_id
        } if !authored_runtime_id.is_empty() && authored_runtime_id == expected_runtime_id
    ) {
        return;
    }
    diagnostics.push(Diagnostic::error(
        DiagnosticCode::InvalidOrigin,
        owner,
        "origin",
        format!(
            "story draft origin must be new with authored_runtime_id exactly {expected_runtime_id:?}"
        ),
    ));
}

fn validate_regeneration(
    draft_id: EntityId,
    module_map_key: EntityId,
    referenced: Option<&Entity>,
    regenerated: Result<ScriptModule, StoryRegenerationError>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let expected = match regenerated {
        Ok(expected) => expected,
        Err(error) => {
            let code = match error {
                StoryRegenerationError::GeneratorContract { .. } => {
                    DiagnosticCode::GeneratorContractDrift
                }
                _ => DiagnosticCode::InvalidGeneratorInput,
            };
            diagnostics.push(Diagnostic::error(
                code,
                draft_id,
                "payload.data",
                error.to_string(),
            ));
            return;
        }
    };
    let Some(entity) = referenced else {
        return;
    };
    let EntityPayload::ScriptModule(actual) = &entity.payload else {
        return;
    };
    compare_generated_module(module_map_key, actual, &expected, diagnostics);
}

fn compare_generated_module(
    module_id: EntityId,
    actual: &ScriptModule,
    expected: &ScriptModule,
    diagnostics: &mut Vec<Diagnostic>,
) {
    macro_rules! compare {
        ($field:ident) => {
            if actual.$field != expected.$field {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::GeneratedScriptDrift,
                    module_id,
                    concat!("payload.data.", stringify!($field)),
                    concat!(
                        "persisted script ",
                        stringify!($field),
                        " does not match deterministic regeneration"
                    ),
                ));
            }
        };
    }
    compare!(generator_id);
    compare!(generator_version);
    compare!(owner);
    compare!(module_namespace);
    compare!(module_relative_path);
    compare!(source);
    compare!(source_sha256);
    compare!(input_fingerprint);
    compare!(status);
}

fn validate_script_module(
    project: &ProjectRevision2,
    module_id: EntityId,
    entity: &Entity,
    module: &ScriptModule,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if module.owner.project_id != project.project_id {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::ReferenceProjectMismatch,
            module_id,
            "payload.data.owner",
            format!(
                "script owner project {} does not match authored project {}",
                module.owner.project_id, project.project_id
            ),
        ));
    }
    if !matches!(
        module.owner.expected_kind,
        EntityKind::NpcDraft | EntityKind::QuestDraft
    ) {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::ReferenceDeclaredKindMismatch,
            module_id,
            "payload.data.owner.expected_kind",
            "script owner must declare npc_draft or quest_draft",
        ));
    }

    if module.owner.project_id == project.project_id {
        match project.entities.get(&module.owner.id) {
            None => diagnostics.push(Diagnostic::error(
                DiagnosticCode::MissingReference,
                module_id,
                "payload.data.owner.id",
                format!("script owner {} does not exist", module.owner.id),
            )),
            Some(owner) if owner.kind() != module.owner.expected_kind => diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::ReferenceTargetKindMismatch,
                    module_id,
                    "payload.data.owner",
                    format!(
                        "script owner {} has kind {:?}, expected {:?}",
                        owner.id,
                        owner.kind(),
                        module.owner.expected_kind
                    ),
                )
                .related(owner.id),
            ),
            Some(owner) => {
                validate_reverse_ownership(project, module_id, module, owner, diagnostics)
            }
        }
    }

    let expected_contract = match module.owner.expected_kind {
        EntityKind::NpcDraft => Some((
            LOGICAL_NPC_CLONE_GENERATOR_ID,
            LOGICAL_NPC_CLONE_GENERATOR_VERSION,
        )),
        EntityKind::QuestDraft => Some((DRAFT_QUEST_GENERATOR_ID, DRAFT_QUEST_GENERATOR_VERSION)),
        _ => None,
    };
    if expected_contract.is_some_and(|(id, version)| {
        module.generator_id != id || module.generator_version != version
    }) {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::GeneratorContractDrift,
            module_id,
            "payload.data.generator_id",
            "script module generator contract does not match its owner kind",
        ));
    }

    match &entity.origin {
        OriginRef::Generated {
            generator_id,
            generator_version,
            owner,
        } if generator_id == &module.generator_id
            && generator_version == &module.generator_version
            && owner == &module.owner => {}
        _ => diagnostics.push(Diagnostic::error(
            DiagnosticCode::GeneratorContractDrift,
            module_id,
            "origin",
            "script module origin must exactly mirror its generator and owner",
        )),
    }

    let actual_sha = Sha256Digest::from_bytes(Sha256::digest(module.source.as_bytes()).into());
    if actual_sha != module.source_sha256 {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::GeneratedScriptDrift,
            module_id,
            "payload.data.source_sha256",
            "script source SHA-256 does not match the persisted source bytes",
        ));
    }
}

fn validate_reverse_ownership(
    project: &ProjectRevision2,
    module_id: EntityId,
    module: &ScriptModule,
    owner: &Entity,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let owner_module = match &owner.payload {
        EntityPayload::NpcDraft(draft) => Some(&draft.script_module),
        EntityPayload::QuestDraft(draft) => Some(&draft.script_module),
        _ => None,
    };
    let expected = TypedRef::new(project.project_id, module_id, EntityKind::ScriptModule);
    if owner_module != Some(&expected) {
        diagnostics.push(
            Diagnostic::error(
                DiagnosticCode::ScriptModuleOwnershipMismatch,
                module_id,
                "payload.data.owner",
                format!(
                    "owner {} does not point back to this script module",
                    module.owner.id
                ),
            )
            .related(owner.id),
        );
    }
}

fn validate_required_ref<'a>(
    project: &'a ProjectRevision2,
    owner: EntityId,
    path: &str,
    reference: &TypedRef,
    required_kind: EntityKind,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a Entity> {
    if reference.project_id != project.project_id {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::ReferenceProjectMismatch,
            owner,
            path,
            format!(
                "reference project {} does not match authored project {}",
                reference.project_id, project.project_id
            ),
        ));
    }
    if reference.expected_kind != required_kind {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::ReferenceDeclaredKindMismatch,
            owner,
            path,
            format!(
                "reference declares {:?}, but this property requires {required_kind:?}",
                reference.expected_kind
            ),
        ));
    }
    if reference.project_id != project.project_id {
        return None;
    }
    let Some(target) = project.entities.get(&reference.id) else {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::MissingReference,
            owner,
            path,
            format!("referenced entity {} does not exist", reference.id),
        ));
        return None;
    };
    if target.kind() != required_kind {
        diagnostics.push(
            Diagnostic::error(
                DiagnosticCode::ReferenceTargetKindMismatch,
                owner,
                path,
                format!(
                    "referenced entity {} has kind {:?}, expected {required_kind:?}",
                    reference.id,
                    target.kind()
                ),
            )
            .related(reference.id),
        );
        return None;
    }
    Some(target)
}

fn push_runtime_unqualified(
    owner: EntityId,
    kind: EntityKind,
    profile: ValidationProfile,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let message = match kind {
        EntityKind::NpcDraft => "NpcDraft has deterministic offline source, but external vanilla/dependency symbol collision freedom and runtime behavior remain unqualified".to_owned(),
        EntityKind::QuestDraft => "QuestDraft has deterministic offline source, but runtime discovery, transitions, effects, and persistence remain unqualified".to_owned(),
        _ => format!(
            "{kind:?} has deterministic offline source but no verified runtime qualification evidence"
        ),
    };
    diagnostics.push(match profile {
        ValidationProfile::Production => Diagnostic::error(
            DiagnosticCode::RuntimeUnqualified,
            owner,
            "payload.data.script_module",
            message,
        ),
        ValidationProfile::Experimental => Diagnostic::warning(
            DiagnosticCode::RuntimeUnqualified,
            owner,
            "payload.data.script_module",
            message,
        ),
    });
}
