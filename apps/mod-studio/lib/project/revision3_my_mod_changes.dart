import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';

const _voiceSlotGeneratorId = 'gore-authoring.voice-slot';
const _voiceSlotGeneratorVersion = 1;

/// Semantic content kinds exposed by the exact-current "My mod / Changes"
/// projection.
///
/// DataAssets deliberately have no [Revision3ContentEntityKind] equivalent:
/// they are projected only from the independently verified stage registry.
enum Revision3MyModContentKind {
  quest,
  npc,
  itemPatch,
  dataAsset,
  dialogLine,
  localization,
  voiceSlot,
  voiceTake,
  generatedScript,
}

/// The exact typed edge which placed one entry below another.
enum Revision3MyModRelationship {
  topLevel,
  generatedScript,
  npcGreeting,
  questTranscript,
  dialogLocalization,
  dialogVoiceSlot,
  voiceCandidate,
}

/// Why a helper is intentionally shown as technical instead of being attached
/// to author content.
enum Revision3MyModTechnicalReason {
  unresolvedReference,
  unprovenGeneratedOwnership,
}

/// One immutable semantic entry in "My mod / Changes".
///
/// This is a project-content projection only. It intentionally carries no
/// build, deployment, publication, or runtime claim.
final class Revision3MyModEntry {
  Revision3MyModEntry._entity({
    required this.kind,
    required this.relationship,
    required Revision3ContentEntity entity,
    required List<Revision3MyModEntry> children,
    this.qualifier,
    this.selected = false,
    this.technicalReason,
  }) : stableId = entity.id,
       displayName = entity.displayName,
       entity = entity,
       dataAssetStage = null,
       children = List<Revision3MyModEntry>.unmodifiable(children);

  Revision3MyModEntry._dataAsset({
    required AuthoringRevision3DataAssetStage stage,
  }) : kind = Revision3MyModContentKind.dataAsset,
       relationship = Revision3MyModRelationship.topLevel,
       stableId = stage.targetPath,
       displayName = stage.targetPath,
       entity = null,
       dataAssetStage = stage,
       qualifier = null,
       selected = false,
       technicalReason = null,
       children = const <Revision3MyModEntry>[];

  final Revision3MyModContentKind kind;
  final Revision3MyModRelationship relationship;
  final String stableId;
  final String displayName;
  final Revision3ContentEntity? entity;
  final AuthoringRevision3DataAssetStage? dataAssetStage;
  final String? qualifier;
  final bool selected;
  final Revision3MyModTechnicalReason? technicalReason;
  final List<Revision3MyModEntry> children;

  int get problemCount => entity?.problemCount ?? 0;
}

/// An exact-current, UI-independent semantic projection for "My mod /
/// Changes".
final class Revision3MyModChanges {
  Revision3MyModChanges._({
    required this.projectId,
    required this.projectRevision,
    required List<Revision3MyModEntry> changes,
    required List<Revision3MyModEntry> technical,
  }) : changes = List<Revision3MyModEntry>.unmodifiable(changes),
       technical = List<Revision3MyModEntry>.unmodifiable(technical);

  final String projectId;
  final int projectRevision;

  /// Author-facing top-level changes. Exact helpers can be nested below them.
  final List<Revision3MyModEntry> changes;

  /// Problematic helpers and generated helpers whose ownership is not proven.
  final List<Revision3MyModEntry> technical;

  /// Project every entity exactly once and add DataAssets only from
  /// [dataAssetStages].
  ///
  /// [dataAssetStages] must come from the session's exact-head stage-registry
  /// loader. The checkpoint/head binding stays at that coordinator boundary;
  /// this projection independently checks every non-empty stage against the
  /// exact content-index project, executable, revision, uniqueness, and order.
  factory Revision3MyModChanges.fromExactCurrent({
    required Revision3ContentIndex contentIndex,
    required List<AuthoringRevision3DataAssetStage> dataAssetStages,
  }) {
    _requireSameSnapshot(contentIndex, dataAssetStages);
    return _Revision3MyModProjector(
      contentIndex: contentIndex,
      dataAssetStages: dataAssetStages,
    ).project();
  }
}

/// The independently verified inputs do not describe one project snapshot.
final class Revision3MyModSnapshotMismatch implements Exception {
  const Revision3MyModSnapshotMismatch(this.message);

  final String message;

  @override
  String toString() => 'Revision3MyModSnapshotMismatch: $message';
}

void _requireSameSnapshot(
  Revision3ContentIndex contentIndex,
  List<AuthoringRevision3DataAssetStage> dataAssetStages,
) {
  final foldedTargets = <String>{};
  String? previousTarget;
  for (final stage in dataAssetStages) {
    if (stage.projectId != contentIndex.projectId ||
        stage.projectTargetExecutable.sha256 !=
            contentIndex.targetExecutableSha256 ||
        stage.projectTargetExecutable.byteLength !=
            contentIndex.targetExecutableByteLength ||
        stage.stagedProjectRevision > contentIndex.projectRevision) {
      throw const Revision3MyModSnapshotMismatch(
        'a DataAsset stage belongs to another project snapshot',
      );
    }
    if (!foldedTargets.add(stage.targetPath.toLowerCase()) ||
        (previousTarget != null &&
            previousTarget.compareTo(stage.targetPath) >= 0)) {
      throw const Revision3MyModSnapshotMismatch(
        'DataAsset stages are not one unique canonical target order',
      );
    }
    previousTarget = stage.targetPath;
  }
}

final class _Revision3MyModProjector {
  _Revision3MyModProjector({
    required this.contentIndex,
    required this.dataAssetStages,
  });

  final Revision3ContentIndex contentIndex;
  final List<AuthoringRevision3DataAssetStage> dataAssetStages;

  late final Map<String, Revision3ContentEntity> _entities =
      <String, Revision3ContentEntity>{
        for (final entity in contentIndex.entities) entity.id: entity,
      };

  late final Map<String, _Placement> _placements = _buildPlacements();
  late final Map<String, List<Revision3ContentEntity>> _childrenByParent =
      _buildChildrenByParent();

  Revision3MyModChanges project() {
    final changes = <Revision3MyModEntry>[];
    final technical = <Revision3MyModEntry>[];

    for (final kind in const <Revision3ContentEntityKind>[
      Revision3ContentEntityKind.questDraft,
      Revision3ContentEntityKind.npcDraft,
      Revision3ContentEntityKind.itemPatch,
      Revision3ContentEntityKind.dialogLine,
      Revision3ContentEntityKind.localizationEntry,
      Revision3ContentEntityKind.voiceSlot,
      Revision3ContentEntityKind.voiceTake,
      Revision3ContentEntityKind.scriptModule,
    ]) {
      final entities =
          contentIndex.entities
              .where((candidate) => candidate.kind == kind)
              .toList(growable: false)
            ..sort(_compareEntities);
      for (final entity in entities) {
        if (_placements.containsKey(entity.id)) continue;
        final reason = _technicalReason(entity);
        final entry = _entryFor(
          entity,
          relationship: Revision3MyModRelationship.topLevel,
          technicalReason: reason,
        );
        if (reason == null) {
          changes.add(entry);
        } else {
          technical.add(entry);
        }
      }
      if (kind == Revision3ContentEntityKind.itemPatch) {
        final sortedStages = dataAssetStages.toList(growable: false)
          ..sort(_compareDataAssetStages);
        changes.addAll(
          sortedStages.map(
            (stage) => Revision3MyModEntry._dataAsset(stage: stage),
          ),
        );
      }
    }

    return Revision3MyModChanges._(
      projectId: contentIndex.projectId,
      projectRevision: contentIndex.projectRevision,
      changes: changes,
      technical: technical,
    );
  }

  Map<String, _Placement> _buildPlacements() {
    final placements = <String, _Placement>{};

    for (final entity in contentIndex.entities) {
      final placement = switch (entity.kind) {
        Revision3ContentEntityKind.scriptModule => _generatedScriptPlacement(
          entity,
        ),
        Revision3ContentEntityKind.dialogLine => _dialogPlacement(entity),
        Revision3ContentEntityKind.localizationEntry => _localizationPlacement(
          entity,
        ),
        Revision3ContentEntityKind.voiceSlot => _voiceSlotPlacement(entity),
        Revision3ContentEntityKind.voiceTake => _voiceTakePlacement(entity),
        Revision3ContentEntityKind.npcDraft ||
        Revision3ContentEntityKind.questDraft ||
        Revision3ContentEntityKind.itemPatch => null,
      };
      if (placement != null) placements[entity.id] = placement;
    }
    return placements;
  }

  Map<String, List<Revision3ContentEntity>> _buildChildrenByParent() {
    final children = <String, List<Revision3ContentEntity>>{};
    for (final entity in contentIndex.entities) {
      final placement = _placements[entity.id];
      if (placement == null) continue;
      children
          .putIfAbsent(placement.parentId, () => <Revision3ContentEntity>[])
          .add(entity);
    }
    return <String, List<Revision3ContentEntity>>{
      for (final entry in children.entries)
        entry.key: entry.value..sort(_compareChildren),
    };
  }

  _Placement? _generatedScriptPlacement(Revision3ContentEntity module) {
    if (module.problemCount != 0 || module.origin.type != 'generated') {
      return null;
    }
    final ownerTarget = module.origin.generatedOwner;
    if (ownerTarget == null ||
        ownerTarget.projectId != contentIndex.projectId ||
        (ownerTarget.expectedKind != Revision3ContentEntityKind.questDraft &&
            ownerTarget.expectedKind != Revision3ContentEntityKind.npcDraft)) {
      return null;
    }
    final owner = _entities[ownerTarget.entityId];
    if (owner == null ||
        owner.kind != ownerTarget.expectedKind ||
        owner.problemCount != 0) {
      return null;
    }

    final ownerClaims = module.references
        .where(
          (reference) =>
              reference.role == 'origin_owner' ||
              reference.role == 'script_owner',
        )
        .toList(growable: false);
    if (ownerClaims.length != 2 ||
        ownerClaims.where((item) => item.role == 'origin_owner').length != 1 ||
        ownerClaims.where((item) => item.role == 'script_owner').length != 1 ||
        !ownerClaims.every(
          (reference) =>
              _isExactReference(reference, target: owner, qualifier: null),
        )) {
      return null;
    }

    final backlinks = contentIndex
        .backlinksToEntity(module.id)
        .where((backlink) => backlink.reference.role == 'draft_script_module')
        .toList(growable: false);
    if (backlinks.length != 1 ||
        !identical(backlinks.single.source, owner) ||
        !_isExactReference(
          backlinks.single.reference,
          target: module,
          qualifier: null,
        )) {
      return null;
    }
    return _Placement(
      parentId: owner.id,
      relationship: Revision3MyModRelationship.generatedScript,
    );
  }

  _Placement? _dialogPlacement(Revision3ContentEntity dialog) {
    if (dialog.problemCount != 0) return null;
    final claims = contentIndex
        .backlinksToEntity(dialog.id)
        .where(
          (backlink) =>
              backlink.reference.role == 'npc_greeting_line' ||
              backlink.reference.role == 'quest_transcript_line',
        )
        .toList(growable: false);
    if (claims.length != 1) return null;

    final claim = claims.single;
    final owner = claim.source;
    final relationship = switch ((owner.kind, claim.reference.role)) {
      (Revision3ContentEntityKind.npcDraft, 'npc_greeting_line') =>
        Revision3MyModRelationship.npcGreeting,
      (Revision3ContentEntityKind.questDraft, 'quest_transcript_line') =>
        Revision3MyModRelationship.questTranscript,
      _ => null,
    };
    if (relationship == null ||
        owner.problemCount != 0 ||
        !_isExactReference(claim.reference, target: dialog) ||
        !_generatedOwnershipAllows(dialog, owner)) {
      return null;
    }
    return _Placement(parentId: owner.id, relationship: relationship);
  }

  _Placement? _localizationPlacement(Revision3ContentEntity localization) =>
      _uniqueTypedParent(
        child: localization,
        parentKind: Revision3ContentEntityKind.dialogLine,
        role: 'dialog_localization',
        relationship: Revision3MyModRelationship.dialogLocalization,
        requireNullQualifier: true,
      );

  _Placement? _voiceSlotPlacement(Revision3ContentEntity slot) =>
      _uniqueTypedParent(
        child: slot,
        parentKind: Revision3ContentEntityKind.dialogLine,
        role: 'dialog_voice_slot',
        relationship: Revision3MyModRelationship.dialogVoiceSlot,
      );

  _Placement? _voiceTakePlacement(Revision3ContentEntity take) =>
      _uniqueTypedParent(
        child: take,
        parentKind: Revision3ContentEntityKind.voiceSlot,
        role: 'voice_candidate',
        relationship: Revision3MyModRelationship.voiceCandidate,
        requireNullQualifier: true,
      );

  _Placement? _uniqueTypedParent({
    required Revision3ContentEntity child,
    required Revision3ContentEntityKind parentKind,
    required String role,
    required Revision3MyModRelationship relationship,
    bool requireNullQualifier = false,
  }) {
    if (child.problemCount != 0) return null;
    final claims = contentIndex
        .backlinksToEntity(child.id)
        .where((backlink) => backlink.reference.role == role)
        .toList(growable: false);
    if (claims.length != 1) return null;
    final claim = claims.single;
    final parent = claim.source;
    if (parent.kind != parentKind ||
        parent.problemCount != 0 ||
        !_isExactReference(
          claim.reference,
          target: child,
          qualifier: requireNullQualifier ? null : _anyQualifier,
        ) ||
        !_generatedOwnershipAllows(child, parent)) {
      return null;
    }
    return _Placement(
      parentId: parent.id,
      relationship: relationship,
      qualifier: claim.reference.qualifier,
    );
  }

  bool _generatedOwnershipAllows(
    Revision3ContentEntity child,
    Revision3ContentEntity parent,
  ) {
    if (child.origin.type != 'generated') return true;
    if (child.kind != Revision3ContentEntityKind.voiceSlot ||
        child.origin.label != _voiceSlotGeneratorId ||
        child.origin.generatorVersion != _voiceSlotGeneratorVersion) {
      return false;
    }
    final owner = child.origin.generatedOwner;
    if (owner == null ||
        owner.projectId != contentIndex.projectId ||
        owner.entityId != parent.id ||
        owner.expectedKind != parent.kind) {
      return false;
    }
    final claims = child.references
        .where((reference) => reference.role == 'origin_owner')
        .toList(growable: false);
    return claims.length == 1 &&
        _isExactReference(claims.single, target: parent, qualifier: null);
  }

  bool _isExactReference(
    Revision3ContentReference reference, {
    required Revision3ContentEntity target,
    Object? qualifier = _anyQualifier,
  }) =>
      reference.resolution == Revision3ContentReferenceResolution.resolved &&
      reference.target.projectId == contentIndex.projectId &&
      reference.target.entityId == target.id &&
      reference.target.expectedKind == target.kind &&
      (identical(qualifier, _anyQualifier) || reference.qualifier == qualifier);

  Revision3MyModTechnicalReason? _technicalReason(
    Revision3ContentEntity entity,
  ) {
    if (entity.origin.type == 'generated' ||
        entity.kind == Revision3ContentEntityKind.scriptModule) {
      if (entity.problemCount != 0) {
        return Revision3MyModTechnicalReason.unresolvedReference;
      }
      return Revision3MyModTechnicalReason.unprovenGeneratedOwnership;
    }
    return null;
  }

  Revision3MyModEntry _entryFor(
    Revision3ContentEntity entity, {
    required Revision3MyModRelationship relationship,
    String? qualifier,
    Revision3MyModTechnicalReason? technicalReason,
  }) {
    final children = <Revision3MyModEntry>[];
    for (final child
        in _childrenByParent[entity.id] ?? const <Revision3ContentEntity>[]) {
      final placement = _placements[child.id]!;
      children.add(
        _entryFor(
          child,
          relationship: placement.relationship,
          qualifier: placement.qualifier,
        ),
      );
    }
    final selected =
        relationship == Revision3MyModRelationship.voiceCandidate &&
        entity.kind == Revision3ContentEntityKind.voiceTake &&
        _isSelectedTake(entity);
    return Revision3MyModEntry._entity(
      kind: _contentKind(entity.kind),
      relationship: relationship,
      entity: entity,
      children: children,
      qualifier: qualifier,
      selected: selected,
      technicalReason: technicalReason,
    );
  }

  bool _isSelectedTake(Revision3ContentEntity take) {
    final placement = _placements[take.id];
    if (placement == null) return false;
    final slot = _entities[placement.parentId];
    if (slot == null) return false;
    return slot.references.any(
      (reference) =>
          reference.role == 'voice_selected' &&
          _isExactReference(reference, target: take, qualifier: null),
    );
  }
}

int _compareEntities(
  Revision3ContentEntity left,
  Revision3ContentEntity right,
) {
  final byDisplayName = left.displayName.toLowerCase().compareTo(
    right.displayName.toLowerCase(),
  );
  return byDisplayName != 0 ? byDisplayName : left.id.compareTo(right.id);
}

int _compareChildren(
  Revision3ContentEntity left,
  Revision3ContentEntity right,
) {
  final byKind = _childKindOrder(
    left.kind,
  ).compareTo(_childKindOrder(right.kind));
  return byKind != 0 ? byKind : _compareEntities(left, right);
}

int _childKindOrder(Revision3ContentEntityKind kind) => switch (kind) {
  Revision3ContentEntityKind.scriptModule => 0,
  Revision3ContentEntityKind.dialogLine => 1,
  Revision3ContentEntityKind.localizationEntry => 2,
  Revision3ContentEntityKind.voiceSlot => 3,
  Revision3ContentEntityKind.voiceTake => 4,
  Revision3ContentEntityKind.npcDraft ||
  Revision3ContentEntityKind.questDraft ||
  Revision3ContentEntityKind.itemPatch => 5,
};

int _compareDataAssetStages(
  AuthoringRevision3DataAssetStage left,
  AuthoringRevision3DataAssetStage right,
) {
  final byDisplayName = left.targetPath.toLowerCase().compareTo(
    right.targetPath.toLowerCase(),
  );
  return byDisplayName != 0
      ? byDisplayName
      : left.targetPath.compareTo(right.targetPath);
}

Revision3MyModContentKind _contentKind(
  Revision3ContentEntityKind kind,
) => switch (kind) {
  Revision3ContentEntityKind.questDraft => Revision3MyModContentKind.quest,
  Revision3ContentEntityKind.npcDraft => Revision3MyModContentKind.npc,
  Revision3ContentEntityKind.itemPatch => Revision3MyModContentKind.itemPatch,
  Revision3ContentEntityKind.dialogLine => Revision3MyModContentKind.dialogLine,
  Revision3ContentEntityKind.localizationEntry =>
    Revision3MyModContentKind.localization,
  Revision3ContentEntityKind.voiceSlot => Revision3MyModContentKind.voiceSlot,
  Revision3ContentEntityKind.voiceTake => Revision3MyModContentKind.voiceTake,
  Revision3ContentEntityKind.scriptModule =>
    Revision3MyModContentKind.generatedScript,
};

final class _Placement {
  const _Placement({
    required this.parentId,
    required this.relationship,
    this.qualifier,
  });

  final String parentId;
  final Revision3MyModRelationship relationship;
  final String? qualifier;
}

const _anyQualifier = Object();
