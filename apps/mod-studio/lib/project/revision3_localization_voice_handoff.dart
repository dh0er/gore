import 'revision3_content_index.dart';

/// Exact, presentation-free context for opening one saved Text & Voice change.
///
/// Every identity comes from a validated current content index. Callers must
/// bind the result to that index's project checkpoint before navigating.
final class Revision3LocalizationVoiceEntityHandoff {
  const Revision3LocalizationVoiceEntityHandoff._({
    required this.localizationEntityId,
    this.dialogLineEntityId,
    this.locale,
    this.voiceSlotEntityId,
    this.voiceTakeEntityId,
  });

  final String localizationEntityId;
  final String? dialogLineEntityId;
  final String? locale;
  final String? voiceSlotEntityId;
  final String? voiceTakeEntityId;
}

/// Resolves a saved LocalizationEntry, DialogLine, VoiceSlot, or VoiceTake to
/// its unique exact Text & Voice owner.
///
/// Ambiguous, unresolved, cross-project, or structurally incomplete graphs
/// return null. In particular, a VoiceTake shared by multiple slots is never
/// assigned to an arbitrary line or locale.
Revision3LocalizationVoiceEntityHandoff?
resolveRevision3LocalizationVoiceEntityHandoff({
  required Revision3ContentIndex index,
  required Revision3ContentEntity entity,
}) {
  final current = index.entityById(entity.id);
  if (current == null ||
      !identical(current, entity) ||
      current.kind != entity.kind ||
      current.revision != entity.revision ||
      current.problemCount != 0) {
    return null;
  }
  return switch (current.kind) {
    Revision3ContentEntityKind.localizationEntry =>
      _isAuthorableLocalization(current)
          ? Revision3LocalizationVoiceEntityHandoff._(
              localizationEntityId: current.id,
            )
          : null,
    Revision3ContentEntityKind.dialogLine => _handoffForDialogLine(
      index,
      current,
    ),
    Revision3ContentEntityKind.voiceSlot => _handoffForVoiceSlot(
      index,
      current,
    ),
    Revision3ContentEntityKind.voiceTake => _handoffForVoiceTake(
      index,
      current,
    ),
    Revision3ContentEntityKind.npcDraft ||
    Revision3ContentEntityKind.questDraft ||
    Revision3ContentEntityKind.scriptModule ||
    Revision3ContentEntityKind.itemPatch => null,
  };
}

Revision3LocalizationVoiceEntityHandoff? _handoffForDialogLine(
  Revision3ContentIndex index,
  Revision3ContentEntity dialogLine,
) {
  if (dialogLine.kind != Revision3ContentEntityKind.dialogLine ||
      dialogLine.summary.dialogLine == null ||
      dialogLine.problemCount != 0) {
    return null;
  }
  final localizationReferences = dialogLine.references
      .where((reference) => reference.role == 'dialog_localization')
      .toList(growable: false);
  if (localizationReferences.length != 1) return null;
  final reference = localizationReferences.single;
  if (reference.qualifier != null ||
      reference.resolution != Revision3ContentReferenceResolution.resolved ||
      reference.target.projectId != index.projectId ||
      reference.target.expectedKind !=
          Revision3ContentEntityKind.localizationEntry) {
    return null;
  }
  final localization = index.entityById(reference.target.entityId);
  if (localization == null || !_isAuthorableLocalization(localization)) {
    return null;
  }
  return Revision3LocalizationVoiceEntityHandoff._(
    localizationEntityId: localization.id,
    dialogLineEntityId: dialogLine.id,
  );
}

Revision3LocalizationVoiceEntityHandoff? _handoffForVoiceSlot(
  Revision3ContentIndex index,
  Revision3ContentEntity voiceSlot,
) {
  if (voiceSlot.kind != Revision3ContentEntityKind.voiceSlot ||
      voiceSlot.summary.voiceSlot == null ||
      voiceSlot.problemCount != 0) {
    return null;
  }
  final owners = index
      .backlinksToEntity(voiceSlot.id)
      .where((backlink) => backlink.reference.role == 'dialog_voice_slot')
      .toList(growable: false);
  if (owners.length != 1) return null;
  final owner = owners.single;
  final locale = owner.reference.qualifier;
  if (owner.source.kind != Revision3ContentEntityKind.dialogLine ||
      locale == null ||
      locale.isEmpty ||
      locale != voiceSlot.summary.primaryIdentity ||
      !_isExactReference(
        index,
        owner.reference,
        entity: voiceSlot,
        qualifier: locale,
      )) {
    return null;
  }
  final lineHandoff = _handoffForDialogLine(index, owner.source);
  if (lineHandoff == null) return null;
  return Revision3LocalizationVoiceEntityHandoff._(
    localizationEntityId: lineHandoff.localizationEntityId,
    dialogLineEntityId: owner.source.id,
    locale: locale,
    voiceSlotEntityId: voiceSlot.id,
  );
}

Revision3LocalizationVoiceEntityHandoff? _handoffForVoiceTake(
  Revision3ContentIndex index,
  Revision3ContentEntity voiceTake,
) {
  final takeSummary = voiceTake.summary.voiceTake;
  if (voiceTake.kind != Revision3ContentEntityKind.voiceTake ||
      takeSummary == null ||
      voiceTake.problemCount != 0) {
    return null;
  }
  final owners = index
      .backlinksToEntity(voiceTake.id)
      .where((backlink) => backlink.reference.role == 'voice_candidate')
      .toList(growable: false);
  if (owners.length != 1) return null;
  final owner = owners.single;
  if (owner.source.kind != Revision3ContentEntityKind.voiceSlot ||
      !_isExactReference(index, owner.reference, entity: voiceTake)) {
    return null;
  }
  final slotHandoff = _handoffForVoiceSlot(index, owner.source);
  if (slotHandoff == null || slotHandoff.locale != takeSummary.locale) {
    return null;
  }
  return Revision3LocalizationVoiceEntityHandoff._(
    localizationEntityId: slotHandoff.localizationEntityId,
    dialogLineEntityId: slotHandoff.dialogLineEntityId,
    locale: slotHandoff.locale,
    voiceSlotEntityId: owner.source.id,
    voiceTakeEntityId: voiceTake.id,
  );
}

bool _isAuthorableLocalization(Revision3ContentEntity entity) {
  final summary = entity.summary.localizationEntry;
  return entity.kind == Revision3ContentEntityKind.localizationEntry &&
      entity.origin.type == 'new' &&
      entity.problemCount == 0 &&
      summary != null &&
      // ContentIndex construction has already required a sorted, unique list
      // of canonical locales. The authoring catalog additionally requires at
      // least one locale before it exposes the entry.
      summary.locales.isNotEmpty;
}

bool _isExactReference(
  Revision3ContentIndex index,
  Revision3ContentReference reference, {
  required Revision3ContentEntity entity,
  String? qualifier,
}) =>
    reference.resolution == Revision3ContentReferenceResolution.resolved &&
    reference.target.projectId == index.projectId &&
    reference.target.entityId == entity.id &&
    reference.target.expectedKind == entity.kind &&
    reference.qualifier == qualifier;
