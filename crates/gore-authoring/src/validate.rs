use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    AssetRef, ContentSeal, DialogLine, EntityId, EntityKind, EntityPayload, LocaleCode, OggCodec,
    OriginRef, ProjectId, ProjectV2, TypedRef, VoiceMemberProof, VoiceOperation, VoiceSlot,
    VoiceTake, VoiceTakeStatus, VoiceTarget, VoiceTargetResolution,
};

/// Validation strictness for mechanisms whose artifact path exists but runtime qualification does
/// not. Profiles are caller policy, never durable user-controlled qualification flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationProfile {
    Production,
    Experimental,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

/// Stable machine-readable validation codes for the phase-1 model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticCode {
    EntityKeyIdMismatch,
    ReferenceProjectMismatch,
    ReferenceDeclaredKindMismatch,
    MissingReference,
    ReferenceTargetKindMismatch,
    LocaleSlotMismatch,
    SlotTakeLocaleMismatch,
    DuplicateVoiceCandidate,
    MissingSelectedVoiceTake,
    SelectedVoiceTakeNotCandidate,
    SelectedVoiceTakeNotApproved,
    DuplicateVoiceTarget,
    UnresolvedVoiceTarget,
    AmbiguousVoiceTarget,
    InvalidAmbiguousTargetCardinality,
    DuplicateVoiceTargetCandidate,
    LocaleNotAuthored,
    MissingLocalizationValue,
    InvalidLocalizationId,
    InvalidAssetMetadata,
    MissingAsset,
    AssetSizeMismatch,
    AssetMediaTypeMismatch,
    InvalidArchiveSeal,
    InvalidVoiceTarget,
    MemberProofOperationMismatch,
    InvalidMemberProof,
    UnqualifiedVoiceAdd,
    InvalidOggMetadata,
    OpusDecodeUnproven,
    InvalidGenerationAnchor,
    InvalidOrigin,
    OriginGenerationMismatch,
    InvalidGeneratorInput,
    GeneratorContractDrift,
    GeneratedScriptDrift,
    ScriptModuleOwnershipMismatch,
    RuntimeUnqualified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: DiagnosticSeverity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<EntityId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property_path: Option<String>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_entities: Vec<EntityId>,
    pub blocks_build: bool,
}

impl Diagnostic {
    pub(crate) fn error(
        code: DiagnosticCode,
        entity: EntityId,
        property_path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: DiagnosticSeverity::Error,
            entity: Some(entity),
            property_path: Some(property_path.into()),
            message: message.into(),
            related_entities: Vec::new(),
            blocks_build: true,
        }
    }

    pub(crate) fn project_error(
        code: DiagnosticCode,
        property_path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: DiagnosticSeverity::Error,
            entity: None,
            property_path: Some(property_path.into()),
            message: message.into(),
            related_entities: Vec::new(),
            blocks_build: true,
        }
    }

    pub(crate) fn warning(
        code: DiagnosticCode,
        entity: EntityId,
        property_path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: DiagnosticSeverity::Warning,
            entity: Some(entity),
            property_path: Some(property_path.into()),
            message: message.into(),
            related_entities: Vec::new(),
            blocks_build: false,
        }
    }

    pub(crate) fn related(mut self, related: EntityId) -> Self {
        self.related_entities.push(related);
        self
    }
}

impl ProjectV2 {
    /// Validate under production policy and return diagnostics in canonical order.
    pub fn validate(&self) -> Vec<Diagnostic> {
        self.validate_with_profile(ValidationProfile::Production)
    }

    /// Validate the complete phase-1 graph under an explicit policy profile.
    pub fn validate_with_profile(&self, profile: ValidationProfile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if self.target.executable.byte_len == 0 {
            diagnostics.push(Diagnostic::project_error(
                DiagnosticCode::InvalidGenerationAnchor,
                "target.executable.byte_len",
                "game generation executable seal must have a non-zero byte length",
            ));
        }
        for (digest, asset) in &self.asset_store.assets {
            if asset.byte_len == 0 || asset.media_type.trim().is_empty() {
                diagnostics.push(Diagnostic::project_error(
                    DiagnosticCode::InvalidAssetMetadata,
                    format!("asset_store.assets.{digest}"),
                    "asset metadata must have a non-zero byte length and media type",
                ));
            }
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

            validate_origin(self, *map_id, &entity.origin, &mut diagnostics);

            match &entity.payload {
                EntityPayload::LocalizationEntry(localization) => {
                    if localization.loc_id.is_empty()
                        || localization.loc_id.chars().any(char::is_control)
                    {
                        diagnostics.push(Diagnostic::error(
                            DiagnosticCode::InvalidLocalizationId,
                            *map_id,
                            "payload.data.loc_id",
                            "localization id must be non-empty and contain no control characters",
                        ));
                    }
                    for locale in localization.texts.keys() {
                        validate_authored_locale(
                            self,
                            *map_id,
                            locale,
                            format!("payload.data.texts.{locale}"),
                            &mut diagnostics,
                        );
                    }
                    for locale in &self.authoring_locales {
                        if localization
                            .texts
                            .get(locale)
                            .is_none_or(|text| text.trim().is_empty())
                        {
                            diagnostics.push(Diagnostic::error(
                                DiagnosticCode::MissingLocalizationValue,
                                *map_id,
                                format!("payload.data.texts.{locale}"),
                                format!("localization has no authored value for locale {locale}"),
                            ));
                        }
                    }
                }
                EntityPayload::DialogLine(line) => {
                    validate_dialog_line(self, *map_id, line, &mut diagnostics);
                }
                EntityPayload::VoiceSlot(slot) => {
                    validate_voice_slot(self, *map_id, slot, profile, &mut diagnostics);
                }
                EntityPayload::VoiceTake(take) => {
                    validate_voice_take(self, *map_id, take, profile, &mut diagnostics);
                }
            }
        }

        validate_duplicate_voice_targets(self, &mut diagnostics);
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

fn validate_dialog_line(
    project: &ProjectV2,
    owner: EntityId,
    line: &DialogLine,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_ref(
        project,
        owner,
        "payload.data.localization",
        &line.localization,
        EntityKind::LocalizationEntry,
        diagnostics,
    );

    for (locale, slot_ref) in &line.voice_slots {
        let path = format!("payload.data.voice_slots.{locale}");
        validate_authored_locale(project, owner, locale, &path, diagnostics);
        validate_ref(
            project,
            owner,
            &path,
            slot_ref,
            EntityKind::VoiceSlot,
            diagnostics,
        );
        if slot_ref.project_id != project.project_id {
            continue;
        }
        let Some(entity) = project.entities.get(&slot_ref.id) else {
            continue;
        };
        let EntityPayload::VoiceSlot(slot) = &entity.payload else {
            continue;
        };
        if &slot.locale != locale {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::LocaleSlotMismatch,
                    owner,
                    path,
                    format!(
                        "line locale {locale} references a voice slot authored for {}",
                        slot.locale
                    ),
                )
                .related(slot_ref.id),
            );
        }
    }
}

fn validate_voice_slot(
    project: &ProjectV2,
    owner: EntityId,
    slot: &VoiceSlot,
    profile: ValidationProfile,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_authored_locale(
        project,
        owner,
        &slot.locale,
        "payload.data.locale",
        diagnostics,
    );
    validate_target_resolution(owner, &slot.target_resolution, profile, diagnostics);

    let mut candidate_ids = BTreeMap::<(ProjectId, EntityId), usize>::new();
    for (index, candidate) in slot.candidates.iter().enumerate() {
        let path = format!("payload.data.candidates.{index}");
        validate_ref(
            project,
            owner,
            &path,
            candidate,
            EntityKind::VoiceTake,
            diagnostics,
        );
        if let Some(first_index) = candidate_ids.insert((candidate.project_id, candidate.id), index)
        {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::DuplicateVoiceCandidate,
                    owner,
                    &path,
                    format!(
                        "voice take {} already appears at candidate index {first_index}",
                        candidate.id
                    ),
                )
                .related(candidate.id),
            );
        }
        validate_take_locale(project, owner, &path, &slot.locale, candidate, diagnostics);
    }

    let Some(selected) = &slot.selected else {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::MissingSelectedVoiceTake,
            owner,
            "payload.data.selected",
            "voice slot has no selected take",
        ));
        return;
    };
    validate_ref(
        project,
        owner,
        "payload.data.selected",
        selected,
        EntityKind::VoiceTake,
        diagnostics,
    );
    if !candidate_ids.contains_key(&(selected.project_id, selected.id)) {
        diagnostics.push(
            Diagnostic::error(
                DiagnosticCode::SelectedVoiceTakeNotCandidate,
                owner,
                "payload.data.selected",
                format!("selected take {} is not a slot candidate", selected.id),
            )
            .related(selected.id),
        );
    }
    validate_take_locale(
        project,
        owner,
        "payload.data.selected",
        &slot.locale,
        selected,
        diagnostics,
    );
    if selected.project_id == project.project_id {
        let Some(entity) = project.entities.get(&selected.id) else {
            return;
        };
        if let EntityPayload::VoiceTake(take) = &entity.payload {
            if take.status != VoiceTakeStatus::Approved {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::SelectedVoiceTakeNotApproved,
                        owner,
                        "payload.data.selected",
                        format!("selected take {} is not approved", selected.id),
                    )
                    .related(selected.id),
                );
            }
        }
    }
}

fn validate_take_locale(
    project: &ProjectV2,
    owner: EntityId,
    path: &str,
    slot_locale: &LocaleCode,
    take_ref: &TypedRef,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if take_ref.project_id != project.project_id {
        return;
    }
    let Some(entity) = project.entities.get(&take_ref.id) else {
        return;
    };
    let EntityPayload::VoiceTake(take) = &entity.payload else {
        return;
    };
    if &take.locale != slot_locale {
        diagnostics.push(
            Diagnostic::error(
                DiagnosticCode::SlotTakeLocaleMismatch,
                owner,
                path,
                format!(
                    "slot locale {slot_locale} references a voice take authored for {}",
                    take.locale
                ),
            )
            .related(take_ref.id),
        );
    }
}

fn validate_target_resolution(
    owner: EntityId,
    resolution: &VoiceTargetResolution,
    profile: ValidationProfile,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match resolution {
        VoiceTargetResolution::Unresolved => diagnostics.push(Diagnostic::error(
            DiagnosticCode::UnresolvedVoiceTarget,
            owner,
            "payload.data.target_resolution",
            "voice slot has no exact target match",
        )),
        VoiceTargetResolution::Ambiguous { candidates } => {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::AmbiguousVoiceTarget,
                owner,
                "payload.data.target_resolution",
                format!(
                    "voice slot has {} exact target candidates and requires author resolution",
                    candidates.len()
                ),
            ));
            if candidates.len() < 2 {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::InvalidAmbiguousTargetCardinality,
                    owner,
                    "payload.data.target_resolution.candidates",
                    format!(
                        "ambiguous voice target resolution requires at least 2 candidates, got {}",
                        candidates.len()
                    ),
                ));
            }
            let mut folded = BTreeMap::<(String, String), usize>::new();
            for (index, target) in candidates.iter().enumerate() {
                let path = format!("payload.data.target_resolution.candidates.{index}");
                validate_voice_target_structure(owner, &path, target, diagnostics);
                let key = folded_voice_target(target);
                if let Some(first_index) = folded.insert(key, index) {
                    diagnostics.push(Diagnostic::error(
                        DiagnosticCode::DuplicateVoiceTargetCandidate,
                        owner,
                        &path,
                        format!("voice target candidate duplicates candidate index {first_index}"),
                    ));
                }
            }
        }
        VoiceTargetResolution::Resolved { target } => {
            let path = "payload.data.target_resolution.target";
            validate_voice_target_structure(owner, path, target, diagnostics);
            if target.operation == VoiceOperation::Add {
                let message =
                    "new voice-member runtime binding is not qualified in schema revision 1";
                diagnostics.push(match profile {
                    ValidationProfile::Production => Diagnostic::error(
                        DiagnosticCode::UnqualifiedVoiceAdd,
                        owner,
                        format!("{path}.operation"),
                        message,
                    ),
                    ValidationProfile::Experimental => Diagnostic::warning(
                        DiagnosticCode::UnqualifiedVoiceAdd,
                        owner,
                        format!("{path}.operation"),
                        message,
                    ),
                });
            }
        }
    }
}

fn validate_voice_target_structure(
    owner: EntityId,
    path: &str,
    target: &VoiceTarget,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if target.archive_seal.byte_len == 0 {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::InvalidArchiveSeal,
            owner,
            format!("{path}.archive_seal.byte_len"),
            "voice archive seal must have a non-zero byte length",
        ));
    }
    if !valid_archive_name(&target.archive) {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::InvalidVoiceTarget,
            owner,
            format!("{path}.archive"),
            "voice archive must be one safe .zip filename",
        ));
    }
    if !valid_member_path(&target.member) {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::InvalidVoiceTarget,
            owner,
            format!("{path}.member"),
            "voice member must be a safe forward-slash relative .ogg path",
        ));
    }
    match (&target.operation, &target.member_proof) {
        (
            VoiceOperation::Replace,
            VoiceMemberProof::Present {
                uncompressed_size, ..
            },
        ) => {
            if *uncompressed_size == 0 {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::InvalidMemberProof,
                    owner,
                    format!("{path}.member_proof.uncompressed_size"),
                    "present voice member proof must have a non-zero uncompressed size",
                ));
            }
        }
        (VoiceOperation::Add, VoiceMemberProof::Absent) => {}
        (operation, proof) => {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::MemberProofOperationMismatch,
                owner,
                format!("{path}.member_proof"),
                format!(
                    "voice operation {operation:?} is inconsistent with member proof {proof:?}"
                ),
            ));
        }
    }
}

fn validate_voice_take(
    project: &ProjectV2,
    owner: EntityId,
    take: &VoiceTake,
    profile: ValidationProfile,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_authored_locale(
        project,
        owner,
        &take.locale,
        "payload.data.locale",
        diagnostics,
    );

    if take.asset.byte_len == 0 || take.asset.logical_name.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::InvalidAssetMetadata,
            owner,
            "payload.data.asset",
            "voice asset must have a non-zero byte length and logical name",
        ));
    }
    validate_asset_ref(project, owner, &take.asset, diagnostics);
    if take.ogg.channels == 0
        || take.ogg.sample_rate == 0
        || take.ogg.pages == 0
        || take.ogg.logical_streams == 0
    {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::InvalidOggMetadata,
            owner,
            "payload.data.ogg",
            "Ogg metadata counts, channel count, and sample rate must be non-zero",
        ));
    }
    if take.ogg.codec == OggCodec::Opus {
        let message = "Opus headers, packet framing, duration, and granules were validated, but the SILK/CELT payload has no decode proof";
        diagnostics.push(match profile {
            ValidationProfile::Production => Diagnostic::error(
                DiagnosticCode::OpusDecodeUnproven,
                owner,
                "payload.data.ogg.codec",
                message,
            ),
            ValidationProfile::Experimental => Diagnostic::warning(
                DiagnosticCode::OpusDecodeUnproven,
                owner,
                "payload.data.ogg.codec",
                message,
            ),
        });
    }
}

fn validate_asset_ref(
    project: &ProjectV2,
    owner: EntityId,
    asset_ref: &AssetRef,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(asset) = project.asset_store.assets.get(&asset_ref.sha256) else {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::MissingAsset,
            owner,
            "payload.data.asset.sha256",
            format!(
                "asset {} is missing from the AssetStore index",
                asset_ref.sha256
            ),
        ));
        return;
    };
    if asset.byte_len != asset_ref.byte_len {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::AssetSizeMismatch,
            owner,
            "payload.data.asset.byte_len",
            format!(
                "asset reference length {} does not match indexed length {}",
                asset_ref.byte_len, asset.byte_len
            ),
        ));
    }
    if asset.media_type != "audio/ogg" {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::AssetMediaTypeMismatch,
            owner,
            "payload.data.asset.sha256",
            format!(
                "voice asset {} has media type {:?}; expected canonical media type \"audio/ogg\"",
                asset_ref.sha256, asset.media_type
            ),
        ));
    }
}

fn validate_origin(
    project: &ProjectV2,
    owner: EntityId,
    origin: &OriginRef,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match origin {
        OriginRef::New {
            authored_runtime_id,
        } => {
            if authored_runtime_id.trim().is_empty() {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::InvalidOrigin,
                    owner,
                    "origin.authored_runtime_id",
                    "new origin must have a non-empty authored runtime identity",
                ));
            }
        }
        OriginRef::Vanilla {
            generation,
            catalog_layer,
            canonical_selector,
            source_seal,
        } => {
            if generation != &project.target {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::OriginGenerationMismatch,
                    owner,
                    "origin.generation",
                    "vanilla origin was captured from a different game generation",
                ));
            }
            if catalog_layer.trim().is_empty() || canonical_selector.trim().is_empty() {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::InvalidOrigin,
                    owner,
                    "origin",
                    "vanilla origin requires a catalog layer and canonical selector",
                ));
            }
            validate_origin_seal(owner, "origin.source_seal", source_seal, diagnostics);
        }
        OriginRef::Imported {
            importer,
            source_seal,
            ..
        } => {
            if importer.trim().is_empty() {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::InvalidOrigin,
                    owner,
                    "origin.importer",
                    "imported origin requires a non-empty importer identity",
                ));
            }
            validate_origin_seal(owner, "origin.source_seal", source_seal, diagnostics);
        }
        OriginRef::Generated {
            generator_id,
            generator_version,
            owner: generated_owner,
        } => {
            if generator_id.trim().is_empty() || *generator_version == 0 {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::InvalidOrigin,
                    owner,
                    "origin",
                    "generated origin requires a generator id and non-zero version",
                ));
            }
            validate_authored_ref(project, owner, "origin.owner", generated_owner, diagnostics);
        }
    }
}

fn validate_origin_seal(
    owner: EntityId,
    path: &str,
    seal: &ContentSeal,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if seal.byte_len == 0 {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::InvalidOrigin,
            owner,
            path,
            "origin source seal must have a non-zero byte length",
        ));
    }
}

fn validate_authored_ref(
    project: &ProjectV2,
    owner: EntityId,
    path: &str,
    reference: &TypedRef,
    diagnostics: &mut Vec<Diagnostic>,
) {
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
        return;
    }
    let Some(target) = project.entities.get(&reference.id) else {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::MissingReference,
            owner,
            path,
            format!("referenced authored entity {} does not exist", reference.id),
        ));
        return;
    };
    if target.kind() != reference.expected_kind {
        diagnostics.push(
            Diagnostic::error(
                DiagnosticCode::ReferenceTargetKindMismatch,
                owner,
                path,
                format!(
                    "referenced authored entity {} has kind {:?}, expected {:?}",
                    reference.id,
                    target.kind(),
                    reference.expected_kind
                ),
            )
            .related(reference.id),
        );
    }
}

fn validate_authored_locale(
    project: &ProjectV2,
    owner: EntityId,
    locale: &LocaleCode,
    path: impl Into<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !project.authoring_locales.contains(locale) {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::LocaleNotAuthored,
            owner,
            path,
            format!("locale {locale} is not declared in authoring_locales"),
        ));
    }
}

fn validate_ref(
    project: &ProjectV2,
    owner: EntityId,
    path: &str,
    reference: &TypedRef,
    required_kind: EntityKind,
    diagnostics: &mut Vec<Diagnostic>,
) {
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
        return;
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

    let Some(target) = project.entities.get(&reference.id) else {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::MissingReference,
            owner,
            path,
            format!("referenced entity {} does not exist", reference.id),
        ));
        return;
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
    }
}

fn validate_duplicate_voice_targets(project: &ProjectV2, diagnostics: &mut Vec<Diagnostic>) {
    let mut targets = BTreeMap::<(String, String), EntityId>::new();
    for (entity_id, entity) in &project.entities {
        let EntityPayload::VoiceSlot(slot) = &entity.payload else {
            continue;
        };
        let VoiceTargetResolution::Resolved { target } = &slot.target_resolution else {
            continue;
        };
        let key = folded_voice_target(target);
        if let Some(first) = targets.get(&key) {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::DuplicateVoiceTarget,
                    *entity_id,
                    "payload.data.target_resolution.target",
                    format!(
                        "voice deployment target {:?}|{:?} duplicates entity {first}",
                        target.archive, target.member
                    ),
                )
                .related(*first),
            );
        } else {
            targets.insert(key, *entity_id);
        }
    }
}

fn folded_voice_target(target: &VoiceTarget) -> (String, String) {
    (
        target.archive.replace('\\', "/").to_lowercase(),
        target.member.replace('\\', "/").to_lowercase(),
    )
}

fn valid_archive_name(value: &str) -> bool {
    !value.is_empty()
        && !value.contains(['/', '\\'])
        && value.to_ascii_lowercase().ends_with(".zip")
        && gore_vo::validate_archive_entry_path(value, &gore_vo::Limits::default()).is_ok()
}

fn valid_member_path(value: &str) -> bool {
    !value.contains('\\')
        && value.to_ascii_lowercase().ends_with(".ogg")
        && gore_vo::validate_archive_entry_path(value, &gore_vo::Limits::default()).is_ok()
}
