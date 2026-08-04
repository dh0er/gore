import 'package:goresave/features/editor/domain/game_time.dart';
import 'package:goresave/features/editor/domain/glossary_npc_catalog.dart';
import 'package:goresave/features/editor/domain/glossary_segment_text_catalog.dart';
import 'package:goresave/features/editor/domain/item_catalog.dart';
import 'package:goresave/features/editor/domain/progression_models.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/loc/progression_loc.dart';
import 'package:intl/intl.dart';

/// Stable semantic groups for memory events. UI code can use these for icons,
/// colors, grouping and filtering without having to understand gameplay tags.
enum MemoryEventCategory {
  quest,
  document,
  story,
  exploration,
  combat,
  social,
  item,
  learning,
  guild,
  crime,
  rest,
  other,
}

/// Known event actions. Unknown future tags deliberately resolve to [other]
/// and retain a readable tag-derived title instead of disappearing.
enum MemoryEventKind {
  questStarted,
  questSucceeded,
  questFailed,
  documentRead,
  documentSegmentUnlocked,
  documentSegmentViewed,
  chapterCompleted,
  areaEntered,
  areaLeft,
  characterKilled,
  characterDefeated,
  combatDodge,
  characterDebuffed,
  tradeAvailable,
  itemObtained,
  itemCrafted,
  skillStateRecorded,
  recipeLearned,
  guildJoined,
  crimeRecorded,
  slept,
  storyEvent,
  other,
}

enum MemoryEventFactKind {
  time,
  duration,
  chapter,
  instigator,
  affected,
  amount,
  primaryObject,
  secondaryObject,
  segmentText,
}

/// One localized, display-ready detail row. [technicalValue] preserves the
/// corresponding save identifier for an optional advanced-ID presentation.
class MemoryEventFact {
  const MemoryEventFact({
    required this.kind,
    required this.label,
    required this.value,
    this.technicalValue,
  });

  final MemoryEventFactKind kind;
  final String label;
  final String value;
  final String? technicalValue;
}

/// Localized semantic view of a raw [MemoryEvent].
class MemoryEventPresentation {
  const MemoryEventPresentation({
    required this.kind,
    required this.category,
    required this.categoryLabel,
    required this.title,
    required this.facts,
    required this.tags,
    this.subject,
    this.subjectId,
  });

  final MemoryEventKind kind;
  final MemoryEventCategory category;
  final String categoryLabel;
  final String title;
  final String? subject;
  final String? subjectId;
  final List<MemoryEventFact> facts;
  final List<String> tags;
}

/// Builds display-ready event data synchronously from catalogs that the editor
/// already keeps in memory. Construct one presenter and reuse it for a page.
class MemoryEventPresenter {
  MemoryEventPresenter({
    required this.l10n,
    GameLang? lang,
    this.locCatalog = const {},
    ItemCatalog? itemCatalog,
    Iterable<NpcGlossaryCatalogEntry> npcGlossaryCatalog = const [],
    this.segmentTextCatalog = const {},
  }) : lang = lang ?? _languageForLocale(l10n.localeName),
       _numberFormat = NumberFormat.decimalPattern(l10n.localeName)
         ..maximumFractionDigits = 2 {
    for (final entry in itemCatalog?.entries ?? const <ItemCatalogEntry>[]) {
      for (final key in _referenceKeys(
        entry.path,
      ).followedBy(_referenceKeys(entry.id))) {
        _itemsByReference[key] = entry;
      }
    }
    for (final entry in npcGlossaryCatalog) {
      for (final raw in [entry.id, entry.uniqueName, entry.documentClass]) {
        for (final key in _referenceKeys(raw)) {
          _npcsByReference[key] = entry;
        }
      }
      for (final segment in entry.segments) {
        for (final key in _referenceKeys(segment.segmentClass)) {
          _segmentsByReference[key] = _NpcSegmentMatch(entry, segment);
        }
      }
    }
  }

  final AppLocalizations l10n;
  final GameLang lang;
  final Map<String, Map<String, String>> locCatalog;
  final GlossarySegmentTextCatalog segmentTextCatalog;
  final NumberFormat _numberFormat;
  final Map<String, ItemCatalogEntry> _itemsByReference = {};
  final Map<String, NpcGlossaryCatalogEntry> _npcsByReference = {};
  final Map<String, _NpcSegmentMatch> _segmentsByReference = {};

  MemoryEventPresentation present(MemoryEvent event) {
    final kind = _kind(event);
    final category = _category(kind, event);
    final subject = _subject(event, kind);
    final primaryTag = _primaryTag(event);
    final fallbackAction = _humanizeIdentifier(
      primaryTag
              ?.replaceFirst(RegExp(r'^Memory\.', caseSensitive: false), '')
              .replaceAll('.', '_') ??
          'Event',
    );
    final action = l10n.memoryEventAction(kind.name, fallbackAction);
    final title = subject == null
        ? action
        : l10n.memoryEventTitleWithSubject(action, subject.value);

    return MemoryEventPresentation(
      kind: kind,
      category: category,
      categoryLabel: l10n.memoryEventCategory(category.name, category.name),
      title: title,
      subject: subject?.value,
      subjectId: subject?.technicalValue,
      facts: List.unmodifiable(_facts(event, kind)),
      tags: List.unmodifiable(event.tags),
    );
  }

  List<MemoryEventFact> _facts(MemoryEvent event, MemoryEventKind kind) {
    final facts = <MemoryEventFact>[];
    final time = event.timeSeconds;
    if (time != null && time >= 0) {
      final parts = GameTimeParts.fromTotalSeconds(time);
      final clock = _clock(parts.hour, parts.minute, parts.second);
      facts.add(
        _fact(
          MemoryEventFactKind.time,
          l10n.memoryEventGameTime(parts.day, clock),
          technicalValue: _formatNumber(time),
        ),
      );
    }

    final duration = event.durationSeconds;
    if (duration != null && duration >= 0) {
      facts.add(
        _fact(
          MemoryEventFactKind.duration,
          _formatDuration(duration),
          technicalValue: _formatNumber(duration),
        ),
      );
    }

    for (final pair in [
      (MemoryEventFactKind.instigator, event.instigator),
      (MemoryEventFactKind.affected, event.affected),
    ]) {
      final raw = pair.$2;
      if (raw == null) continue;
      final display = _actor(raw);
      facts.add(
        _fact(pair.$1, display, technicalValue: display == raw ? null : raw),
      );
    }

    final magnitude = event.magnitude;
    final chapter = kind == MemoryEventKind.chapterCompleted
        ? _chapterNumber(event)
        : null;
    if (chapter != null) {
      facts.add(
        _fact(
          MemoryEventFactKind.chapter,
          l10n.chapterLabel(chapter),
          technicalValue: magnitude?.toString(),
        ),
      );
    } else if (magnitude != null &&
        (magnitude != 0 || kind == MemoryEventKind.chapterCompleted)) {
      facts.add(
        _fact(
          MemoryEventFactKind.amount,
          _formatNumber(magnitude),
          technicalValue: magnitude.toString(),
        ),
      );
    }

    if (kind == MemoryEventKind.documentSegmentUnlocked ||
        kind == MemoryEventKind.documentSegmentViewed) {
      final segmentClass = event.optionalClass2;
      final textIds = segmentClass == null
          ? const <String>[]
          : segmentTextCatalog[segmentClass.toLowerCase()] ?? const <String>[];
      final paragraphs = <String>[];
      for (final textId in textIds) {
        final text = resolveGameText(locCatalog, textId, lang)?.trim();
        if (text != null && text.isNotEmpty) paragraphs.add(text);
      }
      if (paragraphs.isNotEmpty) {
        facts.add(
          _fact(
            MemoryEventFactKind.segmentText,
            paragraphs.join('\n\n'),
            technicalValue: textIds.join(', '),
          ),
        );
      }
    }

    for (final pair in [
      (MemoryEventFactKind.primaryObject, event.optionalClass1),
      (MemoryEventFactKind.secondaryObject, event.optionalClass2),
    ]) {
      final raw = pair.$2;
      if (raw == null) continue;
      final display = _object(raw);
      facts.add(
        _fact(pair.$1, display, technicalValue: display == raw ? null : raw),
      );
    }
    return facts;
  }

  MemoryEventFact _fact(
    MemoryEventFactKind kind,
    String value, {
    String? technicalValue,
  }) => MemoryEventFact(
    kind: kind,
    label: l10n.memoryEventFact(
      kind == MemoryEventFactKind.time ? 'gameTime' : kind.name,
      kind.name,
    ),
    value: value,
    technicalValue: technicalValue,
  );

  _ResolvedSubject? _subject(MemoryEvent event, MemoryEventKind kind) {
    switch (kind) {
      case MemoryEventKind.questStarted:
      case MemoryEventKind.questSucceeded:
      case MemoryEventKind.questFailed:
        return _questSubject(event);
      case MemoryEventKind.documentRead:
      case MemoryEventKind.documentSegmentUnlocked:
      case MemoryEventKind.documentSegmentViewed:
        return _documentSubject(
          event,
          includeSegment: kind != MemoryEventKind.documentRead,
        );
      case MemoryEventKind.chapterCompleted:
        return _chapterSubject(event);
      case MemoryEventKind.areaEntered:
      case MemoryEventKind.areaLeft:
        return _areaSubject(event);
      case MemoryEventKind.characterKilled:
      case MemoryEventKind.characterDefeated:
      case MemoryEventKind.combatDodge:
      case MemoryEventKind.characterDebuffed:
        return _combatSubject(event);
      case MemoryEventKind.tradeAvailable:
        return _fallbackSubject(event);
      case MemoryEventKind.itemObtained:
      case MemoryEventKind.itemCrafted:
        return _firstObjectSubject(event);
      case MemoryEventKind.skillStateRecorded:
        return _skillSubject(event);
      case MemoryEventKind.recipeLearned:
        return _firstObjectSubject(event);
      case MemoryEventKind.guildJoined:
        return _guildSubject(event);
      case MemoryEventKind.crimeRecorded:
        return _crimeSubject(event);
      case MemoryEventKind.storyEvent:
        return _storySubject(event);
      case MemoryEventKind.slept:
      case MemoryEventKind.other:
        return _fallbackSubject(event);
    }
  }

  _ResolvedSubject? _questSubject(MemoryEvent event) {
    _ResolvedSubject? fallback;
    for (final raw in _objectCandidates(event)) {
      final id = _classId(raw);
      if (!id.toLowerCase().contains('quest')) continue;
      final localized = localizedQuestName(locCatalog, lang, id);
      if (localized != null) return _ResolvedSubject(localized, raw);
      fallback ??= _ResolvedSubject(
        _humanizeIdentifier(id, prefixes: const ['Quest']),
        raw,
      );
    }
    if (fallback != null) return fallback;
    for (final tag in event.tags) {
      final questAt = tag.toLowerCase().indexOf('quest_');
      if (questAt < 0) continue;
      final id = tag.substring(questAt);
      return _ResolvedSubject(
        localizedQuestName(locCatalog, lang, id) ??
            _humanizeIdentifier(id, prefixes: const ['Quest']),
        id,
      );
    }
    return null;
  }

  _ResolvedSubject? _documentSubject(
    MemoryEvent event, {
    required bool includeSegment,
  }) {
    _NpcSegmentMatch? segmentMatch;
    NpcGlossaryCatalogEntry? npc;
    String? documentRaw;
    String? segmentRaw;
    for (final raw in _objectCandidates(event)) {
      final segment = _lookup(_segmentsByReference, raw);
      if (segment != null) {
        segmentMatch ??= segment;
        segmentRaw ??= raw;
        npc ??= segment.entry;
        continue;
      }
      final entry = _lookup(_npcsByReference, raw);
      if (entry != null) {
        npc ??= entry;
        documentRaw ??= raw;
      } else if (documentRaw == null) {
        documentRaw = raw;
      } else {
        segmentRaw ??= raw;
      }
    }

    if (npc != null) {
      final npcName = _npcName(npc);
      if (includeSegment && segmentMatch != null) {
        final segment = segmentMatch.segment;
        final fallback = _humanizeIdentifier(
          segment.label.isNotEmpty ? segment.label : segment.id,
        );
        final label = l10n.glossaryCatalogSegmentLabel(segment.id, fallback);
        return _ResolvedSubject(
          '$npcName — $label',
          segmentRaw ?? segment.segmentClass,
        );
      }
      return _ResolvedSubject(npcName, documentRaw ?? npc.documentClass);
    }

    if (documentRaw == null) return null;
    final document = _object(documentRaw);
    if (includeSegment && segmentRaw != null) {
      return _ResolvedSubject(
        '$document — ${_documentSegmentName(segmentRaw, documentRaw)}',
        '$documentRaw | $segmentRaw',
      );
    }
    return _ResolvedSubject(document, documentRaw);
  }

  _ResolvedSubject? _chapterSubject(MemoryEvent event) {
    for (final tag in event.tags) {
      final match = RegExp(
        r'(?:chapter|chapter_)(?:\.|_)?(\d+)',
        caseSensitive: false,
      ).firstMatch(tag);
      if (match != null) {
        final chapter = int.tryParse(match.group(1)!);
        if (chapter != null) {
          return _ResolvedSubject(l10n.chapterLabel(chapter), tag);
        }
      }
    }
    for (final raw in _objectCandidates(event)) {
      final id = _classId(raw);
      if (id.toLowerCase() == 'storyg1r') continue;
      return _ResolvedSubject(_object(raw), raw);
    }
    final chapter = _chapterNumber(event);
    if (chapter != null) {
      return _ResolvedSubject(
        l10n.chapterLabel(chapter),
        'Magnitude ${event.magnitude}',
      );
    }
    return null;
  }

  int? _chapterNumber(MemoryEvent event) {
    final magnitude = event.magnitude;
    if (magnitude == null ||
        magnitude < 0 ||
        magnitude > 99 ||
        magnitude != magnitude.roundToDouble()) {
      return null;
    }
    // The save's Story `Chapter` value is one greater than the most recent
    // Chapter.Completed magnitude (observed 0, 1 while Story.Chapter is 2).
    // Magnitude is therefore a zero-based completed-chapter index.
    return magnitude.toInt() + 1;
  }

  _ResolvedSubject? _areaSubject(MemoryEvent event) {
    for (final tag in event.tags) {
      final lower = tag.toLowerCase();
      if (!lower.contains('area') || _isAreaActionTag(lower)) continue;
      return _ResolvedSubject(_gameplayTagSubject(tag), tag);
    }
    return _firstObjectSubject(event);
  }

  _ResolvedSubject? _combatSubject(MemoryEvent event) {
    final species = _speciesSubject(event);
    final affected = event.affected;
    if (affected != null && _isSpawnActor(affected) && species != null) {
      return species;
    }
    for (final raw in [event.affected, event.instigator]) {
      if (raw != null && !_isHero(raw)) {
        return _ResolvedSubject(_actor(raw), raw);
      }
    }
    final object = _firstObjectSubject(event);
    if (object != null) return object;
    return species;
  }

  _ResolvedSubject? _speciesSubject(MemoryEvent event) {
    for (final tag in event.tags) {
      final lower = tag.toLowerCase();
      if ((lower.startsWith('species.') || lower.contains('.creature.')) &&
          !lower.startsWith('memory.character.defeated')) {
        return _ResolvedSubject(_gameplayTagSubject(tag), tag);
      }
    }
    return null;
  }

  _ResolvedSubject? _firstObjectSubject(MemoryEvent event) {
    final raw = _objectCandidates(event).firstOrNull;
    return raw == null ? null : _ResolvedSubject(_object(raw), raw);
  }

  _ResolvedSubject? _skillSubject(MemoryEvent event) {
    final rawSkills = <String>[];
    for (final tag in event.tags) {
      final lower = tag.toLowerCase();
      if (const {
            'memory.skill.learned',
            'memory.learned.skill',
          }.contains(lower) ||
          lower.startsWith('memory.')) {
        continue;
      }
      // Learned-skill memories are state snapshots, not deltas. An event can
      // carry every known skill including explicit `.Untrained` entries; never
      // claim those were learned. The localized action deliberately describes
      // the remaining list as a recorded state rather than a newly learned set.
      if (lower.endsWith('.untrained')) continue;
      rawSkills.add(tag);
    }
    for (final raw in _objectCandidates(event)) {
      rawSkills.add(raw);
    }
    if (rawSkills.isEmpty) return null;
    final labels = rawSkills.map(_skillName).toSet().toList(growable: false);
    return _ResolvedSubject(_summarize(labels), rawSkills.join(', '));
  }

  _ResolvedSubject? _guildSubject(MemoryEvent event) {
    for (final raw in [
      ...event.tags.where(
        (tag) =>
            tag.toLowerCase().contains('guild') &&
            tag.toLowerCase() != 'memory.guild.joined',
      ),
      ..._objectCandidates(event),
    ]) {
      final normalized = raw.toLowerCase().replaceAll(RegExp(r'[^a-z]'), '');
      final name = switch (normalized) {
        final value when value.contains('oldcamp') => l10n.factionGuildOldCamp,
        final value when value.contains('newcamp') => l10n.factionGuildNewCamp,
        final value when value.contains('swampcamp') =>
          l10n.factionGuildSwampCamp,
        _ => _gameplayTagSubject(raw),
      };
      return _ResolvedSubject(name, raw);
    }
    return null;
  }

  _ResolvedSubject? _crimeSubject(MemoryEvent event) {
    for (final tag in event.tags) {
      final lower = tag.toLowerCase();
      if (!lower.startsWith('memory.crime.') || lower == 'memory.crime') {
        continue;
      }
      final type = lower.split('.').last;
      final label = switch (type) {
        'lockpick' || 'lockpicking' => l10n.skillNameLockpicking,
        'murder' => _withoutCount(l10n.crimeMurder(1)),
        'assault' => _withoutCount(l10n.crimeAssault(1)),
        'theft' || 'steal' => _withoutCount(l10n.crimeTheft(1)),
        'trespass' || 'trespassing' => _withoutCount(l10n.crimeTrespassing(1)),
        'threat' => _withoutCount(l10n.crimeThreat(1)),
        _ => _humanizeIdentifier(type),
      };
      return _ResolvedSubject(label, tag);
    }
    return null;
  }

  _ResolvedSubject? _storySubject(MemoryEvent event) {
    final payloadEventName = event.payload?.valueFor('EventName');
    if (payloadEventName is String && payloadEventName.trim().isNotEmpty) {
      final raw = payloadEventName.trim();
      return _ResolvedSubject(
        _localizedName(raw) ?? _humanizeIdentifier(raw),
        raw,
      );
    }
    for (final raw in [event.affected, event.instigator]) {
      if (raw != null && !_isHero(raw)) {
        return _ResolvedSubject(_actor(raw), raw);
      }
    }
    for (final raw in _objectCandidates(event)) {
      if (_classId(raw).toLowerCase() != 'storyg1r') {
        return _ResolvedSubject(_object(raw), raw);
      }
    }
    for (final tag in event.tags) {
      final lower = tag.toLowerCase();
      if (lower.startsWith('memory.storyevent.') ||
          lower.startsWith('memory.story.event.')) {
        return _ResolvedSubject(_gameplayTagSubject(tag), tag);
      }
    }
    return null;
  }

  _ResolvedSubject? _fallbackSubject(MemoryEvent event) {
    for (final raw in [event.affected, event.instigator]) {
      if (raw != null && !_isHero(raw)) {
        return _ResolvedSubject(_actor(raw), raw);
      }
    }
    return _firstObjectSubject(event);
  }

  String _object(String raw) {
    final item = _lookup(_itemsByReference, raw);
    final npc = _lookup(_npcsByReference, raw);
    if (npc != null) return _npcName(npc);
    final id = item?.id ?? _classId(raw);
    final localized = _localizedName(id);
    if (localized != null) return localized;
    return _humanizeIdentifier(
      id,
      prefixes: const [
        'ItFo',
        'ItMi',
        'ItMw',
        'ItRw',
        'ItAr',
        'ItAm',
        'ItRi',
        'ReFo',
        'Recipe',
        'Document',
        'BP',
        'GE',
      ],
    );
  }

  String _actor(String raw) {
    if (_isHero(raw)) return l10n.memoryEventHero;
    final npc = _lookup(_npcsByReference, raw);
    if (npc != null) return _npcName(npc);
    final compact = _actorId(raw);
    return _localizedName(compact) ?? _humanizeActor(compact);
  }

  String _npcName(NpcGlossaryCatalogEntry entry) =>
      _localizedName(entry.uniqueName) ??
      _localizedName(entry.id) ??
      _humanizeActor(entry.id);

  String? _localizedName(String raw) {
    for (final candidate in _localizedIdCandidates(raw)) {
      final value = resolveGameText(
        locCatalog,
        locIdForCatalogId(candidate),
        lang,
      );
      if (value != null && value.trim().isNotEmpty) return value.trim();
    }
    return null;
  }

  Iterable<String> _localizedIdCandidates(String raw) sync* {
    final id = _classId(raw);
    yield id;
    if (id != raw) yield raw;

    // Recipe classes mirror the produced item class (`ReFo_*` -> `ItFo_*`,
    // `ReAr_*` -> `ItAr_*`, etc.). The extracted game catalog localizes the
    // item, not the recipe wrapper.
    if (id.length > 2 && id.toLowerCase().startsWith('re')) {
      yield 'It${id.substring(2)}';
    }

    const glossaryPrefix = 'document_glossary_';
    final lower = id.toLowerCase();
    if (lower.startsWith(glossaryPrefix)) {
      yield id.substring(glossaryPrefix.length);
    }
  }

  String _documentSegmentName(String raw, String documentRaw) {
    var segmentId = _classId(raw);
    const segmentPrefix = 'DocumentSegment_';
    if (segmentId.toLowerCase().startsWith(segmentPrefix.toLowerCase())) {
      segmentId = segmentId.substring(segmentPrefix.length);
    }

    var documentId = _classId(documentRaw);
    const documentPrefix = 'Document_';
    if (documentId.toLowerCase().startsWith(documentPrefix.toLowerCase())) {
      documentId = documentId.substring(documentPrefix.length);
    }
    final documentStem = '${documentId.toLowerCase()}_';
    if (segmentId.toLowerCase().startsWith(documentStem)) {
      segmentId = segmentId.substring(documentStem.length);
    }

    if (segmentId.toLowerCase() == 'unlock') {
      return l10n.glossarySegmentUnlock;
    }
    final entry = RegExp(
      r'^Entry_?(\d+)$',
      caseSensitive: false,
    ).firstMatch(segmentId);
    if (entry != null) {
      return l10n.glossarySegmentEntry(int.parse(entry.group(1)!));
    }
    return l10n.glossaryCatalogSegmentLabel(
      segmentId,
      _humanizeIdentifier(segmentId),
    );
  }

  String _skillName(String raw) {
    final normalized = raw.toLowerCase().replaceAll(RegExp(r'[^a-z0-9]'), '');
    if (normalized.contains('twohand')) return l10n.skillNameTwoHanded;
    if (normalized.contains('onehand')) return l10n.skillNameOneHanded;
    if (normalized.contains('crossbow')) return l10n.skillNameCrossbow;
    if (normalized.contains('fists')) return l10n.skillNameFists;
    if (normalized.contains('bow')) return l10n.skillNameBow;
    if (normalized.contains('pickpocket')) return l10n.skillNamePickpocketing;
    if (normalized.contains('picklock') || normalized.contains('lockpick')) {
      return l10n.skillNameLockpicking;
    }
    if (normalized.contains('alchemy')) return l10n.skillNameAlchemy;
    if (normalized.contains('inscription')) {
      return l10n.skillNameRuneInscription;
    }
    if (normalized.contains('blacksmith')) return l10n.skillNameBlacksmithing;
    if (normalized.contains('skinswampshark')) {
      return l10n.skillNameSkinSwampshark;
    }
    if (normalized.contains('teethswampshark')) {
      return l10n.skillNameBreakSwampsharkTeeth;
    }
    if (normalized.contains('minecrawlerplate') ||
        normalized.contains('mcplate')) {
      return l10n.skillNameTakeMinecrawlerPlates;
    }
    if (normalized.contains('mandible')) {
      return l10n.skillNameTakeMinecrawlerMandibles;
    }
    if (normalized.contains('shadowbeasthorn')) {
      return l10n.skillNameTakeShadowbeastHorn;
    }
    if (normalized.contains('tongueoffire')) {
      return l10n.skillNameTakeFireTongue;
    }
    if (normalized.contains('trollhorn')) return l10n.skillNameTakeTrollHorn;
    if (normalized.contains('skullarmor')) {
      return l10n.skillNameTakeSkullPlates;
    }
    if (normalized.contains('ulumulu')) return l10n.skillNameTakeUluMulu;
    if (normalized.contains('secretion')) {
      return l10n.skillNameTakeSecretion;
    }
    if (normalized.contains('stings')) return l10n.skillNameTakeStingers;
    if (normalized.contains('scutes')) return l10n.skillNameTakeScutes;
    if (normalized.contains('spines')) return l10n.skillNameTakeSpines;
    if (normalized.contains('organ')) return l10n.skillNameTakeOrgans;
    if (normalized.contains('teeth')) return l10n.skillNameBreakTeeth;
    if (normalized.contains('claw')) return l10n.skillNameTakeClaws;
    if (normalized.contains('fins')) return l10n.skillNameTakeFins;
    if (normalized.contains('fur')) return l10n.skillNameSkinFur;
    if (normalized.contains('skin')) return l10n.skillNameSkin;
    if (normalized.contains('acrobat')) return l10n.skillNameAcrobatics;
    if (normalized.contains('wallclimb')) return l10n.skillNameWallClimbing;
    if (normalized.contains('sneak')) return l10n.skillNameSneaking;
    if (normalized.contains('diving')) return l10n.skillNameDiving;
    if (normalized.contains('riding')) return l10n.skillNameRiding;
    if (normalized.contains('mining')) return l10n.skillNameMining;
    if (normalized.contains('orcish')) return l10n.skillNameOrcish;
    if (normalized.contains('magiccircle') ||
        normalized.contains('magecircle')) {
      return l10n.skillNameMagicCircle;
    }
    return _humanizeIdentifier(
      _classId(raw),
      prefixes: const ['Memory', 'Skill', 'GE'],
    );
  }

  String _gameplayTagSubject(String raw) {
    final pieces = raw.split('.');
    final meaningful = pieces
        .where(
          (piece) => !const {
            'memory',
            'area',
            'enter',
            'entered',
            'leave',
            'left',
            'guild',
            'joined',
            'story',
            'event',
          }.contains(piece.toLowerCase()),
        )
        .toList(growable: false);
    final candidate = meaningful.isEmpty ? pieces.last : meaningful.last;
    return _localizedName(candidate) ?? _humanizeIdentifier(candidate);
  }

  String _formatDuration(double seconds) {
    if (seconds < 60) {
      return l10n.memoryEventSecondsValue(_formatNumber(seconds));
    }
    final whole = seconds.floor();
    final hours = whole ~/ secondsPerHour;
    final minutes = (whole % secondsPerHour) ~/ secondsPerMinute;
    final remainder = whole % secondsPerMinute;
    return _clock(hours, minutes, remainder);
  }

  String _formatNumber(num value) {
    if (value is double && value == value.roundToDouble()) {
      return _numberFormat.format(value.toInt());
    }
    return _numberFormat.format(value);
  }

  String _summarize(List<String> values) {
    if (values.length <= 3) return values.join(', ');
    return l10n.memoryEventMoreValues(
      values.take(3).join(', '),
      values.length - 3,
    );
  }
}

class _ResolvedSubject {
  const _ResolvedSubject(this.value, this.technicalValue);

  final String value;
  final String technicalValue;
}

class _NpcSegmentMatch {
  const _NpcSegmentMatch(this.entry, this.segment);

  final NpcGlossaryCatalogEntry entry;
  final NpcGlossaryCatalogSegment segment;
}

MemoryEventKind _kind(MemoryEvent event) {
  bool has(String tag) =>
      event.tags.any((value) => value.toLowerCase() == tag.toLowerCase());
  bool hasPrefix(String prefix) => event.tags.any(
    (value) => value.toLowerCase().startsWith(prefix.toLowerCase()),
  );

  if (has('Memory.Quest.Started')) return MemoryEventKind.questStarted;
  if (has('Memory.Quest.Succeeded') || has('Memory.Quest.Completed')) {
    return MemoryEventKind.questSucceeded;
  }
  if (has('Memory.Quest.Failed')) return MemoryEventKind.questFailed;
  if (has('Memory.Document.SegmentUnlocked')) {
    return MemoryEventKind.documentSegmentUnlocked;
  }
  if (has('Memory.Document.SegmentViewed')) {
    return MemoryEventKind.documentSegmentViewed;
  }
  if (has('Memory.Document.Read')) return MemoryEventKind.documentRead;
  if (has('Memory.Chapter.Completed')) return MemoryEventKind.chapterCompleted;
  if (has('Memory.Area.Enter') || has('Memory.Area.Entered')) {
    return MemoryEventKind.areaEntered;
  }
  if (has('Memory.Area.Leave') || has('Memory.Area.Left')) {
    return MemoryEventKind.areaLeft;
  }
  if (has('Memory.Character.Defeated.Kill') || has('Memory.Execution')) {
    return MemoryEventKind.characterKilled;
  }
  if (has('Memory.Character.Defeated') ||
      has('Memory.WasDefeated') ||
      has('Memory.Combat.WasDefeated') ||
      has('Memory.SaveAndLoad.Defeated')) {
    return MemoryEventKind.characterDefeated;
  }
  if (has('Memory.Combat.Dodge')) return MemoryEventKind.combatDodge;
  if (has('Memory.Character.Debuffed')) {
    return MemoryEventKind.characterDebuffed;
  }
  if (has('Memory.Character.Can.Trade')) {
    return MemoryEventKind.tradeAvailable;
  }
  if (has('Memory.Item.Crafted')) return MemoryEventKind.itemCrafted;
  if (has('Memory.Item.Obtained') || has('Memory.Item.Acquired')) {
    return MemoryEventKind.itemObtained;
  }
  if (has('Memory.Recipe.Learned') || has('Memory.Learned.Recipe')) {
    return MemoryEventKind.recipeLearned;
  }
  if (has('Memory.Skill.Learned') || has('Memory.Learned.Skill')) {
    return MemoryEventKind.skillStateRecorded;
  }
  if (has('Memory.Guild.Joined')) return MemoryEventKind.guildJoined;
  if (hasPrefix('Memory.Crime')) return MemoryEventKind.crimeRecorded;
  if (hasPrefix('Memory.Sleep')) return MemoryEventKind.slept;
  if (hasPrefix('Memory.StoryEvent') || hasPrefix('Memory.Story.Event')) {
    return MemoryEventKind.storyEvent;
  }
  return MemoryEventKind.other;
}

MemoryEventCategory _category(MemoryEventKind kind, MemoryEvent event) {
  final known = switch (kind) {
    MemoryEventKind.questStarted ||
    MemoryEventKind.questSucceeded ||
    MemoryEventKind.questFailed => MemoryEventCategory.quest,
    MemoryEventKind.documentRead ||
    MemoryEventKind.documentSegmentUnlocked ||
    MemoryEventKind.documentSegmentViewed => MemoryEventCategory.document,
    MemoryEventKind.chapterCompleted ||
    MemoryEventKind.storyEvent => MemoryEventCategory.story,
    MemoryEventKind.areaEntered ||
    MemoryEventKind.areaLeft => MemoryEventCategory.exploration,
    MemoryEventKind.characterKilled ||
    MemoryEventKind.characterDefeated ||
    MemoryEventKind.combatDodge ||
    MemoryEventKind.characterDebuffed => MemoryEventCategory.combat,
    MemoryEventKind.tradeAvailable => MemoryEventCategory.social,
    MemoryEventKind.itemObtained ||
    MemoryEventKind.itemCrafted => MemoryEventCategory.item,
    MemoryEventKind.skillStateRecorded ||
    MemoryEventKind.recipeLearned => MemoryEventCategory.learning,
    MemoryEventKind.guildJoined => MemoryEventCategory.guild,
    MemoryEventKind.crimeRecorded => MemoryEventCategory.crime,
    MemoryEventKind.slept => MemoryEventCategory.rest,
    MemoryEventKind.other => null,
  };
  if (known != null) return known;

  final tag = (_primaryTag(event) ?? '').toLowerCase();
  if (tag.contains('.combat') ||
      tag.contains('.character.defeated') ||
      tag.contains('.character.debuff') ||
      tag.contains('defeated') ||
      tag.contains('execution') ||
      tag.contains('headshot')) {
    return MemoryEventCategory.combat;
  }
  if (tag.contains('.character.can.trade') ||
      tag.contains('.trade') ||
      tag.contains('.dialog')) {
    return MemoryEventCategory.social;
  }
  if (tag.contains('.startinteracting.sleep') || tag.contains('.sleep')) {
    return MemoryEventCategory.rest;
  }
  if (tag.contains('.startinteracting.craft') ||
      tag.contains('customizearmor') ||
      tag.contains('armorupgrade')) {
    return MemoryEventCategory.item;
  }
  if (tag.contains('.document')) return MemoryEventCategory.document;
  if (tag.contains('.quest')) return MemoryEventCategory.quest;
  if (tag.contains('.area')) return MemoryEventCategory.exploration;
  if (tag.contains('.item') || tag.contains('ltm.items')) {
    return MemoryEventCategory.item;
  }
  if (tag.contains('.learned') ||
      tag.contains('.skill') ||
      tag.contains('.recipe')) {
    return MemoryEventCategory.learning;
  }
  if (tag.contains('.crime')) return MemoryEventCategory.crime;
  if (tag.contains('.guild')) return MemoryEventCategory.guild;
  if (tag.contains('.sleep') || tag.contains('.rest')) {
    return MemoryEventCategory.rest;
  }
  if (tag.contains('.story') || tag.contains('.chapter')) {
    return MemoryEventCategory.story;
  }
  return MemoryEventCategory.other;
}

String? _primaryTag(MemoryEvent event) {
  for (final tag in event.tags) {
    if (tag.toLowerCase().startsWith('memory.')) return tag;
  }
  return event.tags.firstOrNull;
}

Iterable<String> _objectCandidates(MemoryEvent event) sync* {
  if (event.optionalClass1 != null) yield event.optionalClass1!;
  if (event.optionalClass2 != null) yield event.optionalClass2!;
}

T? _lookup<T>(Map<String, T> values, String raw) {
  for (final key in _referenceKeys(raw)) {
    final value = values[key];
    if (value != null) return value;
  }
  return null;
}

Iterable<String> _referenceKeys(String raw) sync* {
  final value = raw.trim();
  if (value.isEmpty) return;
  yield value.toLowerCase();
  final id = _classId(value);
  yield id.toLowerCase();
  final worldPointAt = id.toLowerCase().indexOf('-worldpointactor_');
  if (worldPointAt >= 0) {
    yield id.substring(0, worldPointAt).toLowerCase();
    yield id.substring(worldPointAt + '-worldpointactor_'.length).toLowerCase();
  }
  final waypointAt = id.toLowerCase().indexOf('-wp_');
  if (waypointAt >= 0) {
    yield id.substring(0, waypointAt).toLowerCase();
  }
  if (id.toLowerCase().endsWith('_c')) {
    yield id.substring(0, id.length - 2).toLowerCase();
  }
}

String _classId(String raw) {
  var value = raw.trim().replaceAll("'", '');
  if (value.contains('/')) value = value.split('/').last;
  if (value.contains('.')) value = value.split('.').last;
  if (value.endsWith('_C')) value = value.substring(0, value.length - 2);
  return value;
}

String _actorId(String raw) {
  final id = _classId(raw);
  final lower = id.toLowerCase();
  final worldPointAt = lower.indexOf('-worldpointactor_');
  if (worldPointAt >= 0) {
    final actorName = id.substring(worldPointAt + '-worldpointactor_'.length);
    if (actorName.isNotEmpty) return actorName;
    return id.substring(0, worldPointAt);
  }
  final waypointAt = lower.indexOf('-wp_');
  if (waypointAt >= 0) return id.substring(0, waypointAt);
  final spawn = RegExp(
    r'^(.*?)-.*(?:^|_)SPAWN(?:_|$)',
    caseSensitive: false,
  ).firstMatch(id);
  if (spawn != null && spawn.group(1)!.isNotEmpty) return spawn.group(1)!;
  final guidSuffix = RegExp(
    r'^(.*?)-[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$',
    caseSensitive: false,
  ).firstMatch(id);
  if (guidSuffix != null) return guidSuffix.group(1)!;
  return id;
}

bool _isHero(String raw) => _actorId(raw).toLowerCase() == 'hero';

bool _isSpawnActor(String raw) {
  final lower = _classId(raw).toLowerCase();
  return lower.contains('-wp_') || lower.contains('_spawn_');
}

bool _isAreaActionTag(String lower) =>
    lower == 'memory.area.enter' ||
    lower == 'memory.area.entered' ||
    lower == 'memory.area.leave' ||
    lower == 'memory.area.left';

String _humanizeActor(String raw) {
  var value = raw.replaceFirst(RegExp(r'_\d+$'), '');
  final parts = value.split('_').where((part) => part.isNotEmpty).toList();
  if (parts.length >= 3 &&
      parts[0] == parts[0].toUpperCase() &&
      parts[1] == parts[1].toUpperCase()) {
    value = parts.skip(2).join('_');
  }
  return _humanizeIdentifier(value);
}

String _humanizeIdentifier(String raw, {List<String> prefixes = const []}) {
  var value = _classId(raw).replaceAll(RegExp(r'[_\-]+'), ' ');
  value = value.replaceAllMapped(
    RegExp(r'([a-z0-9])([A-Z])'),
    (match) => '${match.group(1)} ${match.group(2)}',
  );
  final words = value
      .split(RegExp(r'\s+'))
      .where((word) => word.isNotEmpty)
      .toList();
  while (words.isNotEmpty &&
      prefixes.any(
        (prefix) => prefix.toLowerCase() == words.first.toLowerCase(),
      )) {
    words.removeAt(0);
  }
  if (words.isEmpty) return raw;
  return words
      .map((word) {
        if (word.length <= 3 && word == word.toUpperCase()) return word;
        return '${word[0].toUpperCase()}${word.substring(1)}';
      })
      .join(' ');
}

String _withoutCount(String value) =>
    value.replaceFirst(RegExp(r'^\s*1\s*'), '').trim();

String _clock(int hours, int minutes, int seconds) =>
    '${hours.toString().padLeft(2, '0')}:'
    '${minutes.toString().padLeft(2, '0')}:'
    '${seconds.toString().padLeft(2, '0')}';

GameLang _languageForLocale(String locale) {
  final lower = locale.toLowerCase();
  if (lower.startsWith('zh')) return gameLangByCode('zh-Hans');
  if (lower.startsWith('pt')) return gameLangByCode('pt-BR');
  return gameLangByCode(lower.split(RegExp(r'[_-]')).first);
}
