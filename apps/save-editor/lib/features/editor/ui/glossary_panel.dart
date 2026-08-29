import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/ui/design/app_theme.dart';

import '../domain/character_index.dart';
import '../domain/editor_models.dart';
import '../domain/editor_notifier.dart';
import '../domain/glossary_models.dart';
import '../domain/glossary_npc_catalog.dart';
import '../domain/glossary_segment_text_catalog.dart';
import '../domain/npc_actors_page.dart';
import '../domain/pending_edits.dart';
import '../domain/progression_models.dart';

enum _GlossarySection {
  oldCamp,
  newCamp,
  swampCamp,
  outsiders,
  creatures,
  locations,
  tutorials,
}

enum _NpcFilter { all, traders, teachers, armorers, hostile, dead }

/// Dedicated save-backed glossary editor mounted by World > Glossary.
///
/// The game persists individual document segments in the Hero's long-term
/// memory. Creature/location segments additionally mirror a pseudo-quest
/// state; [GlossarySegmentEdit] lets the core update both signals atomically.
class GlossaryDetail extends ConsumerStatefulWidget {
  const GlossaryDetail({
    super.key,
    required this.notifier,
    required this.editable,
    required this.reloadKey,
    required this.theme,
    this.npcCatalogLoader,
    this.segmentTextCatalogLoader,
  });

  final EditorNotifier notifier;
  final bool editable;
  final SaveInspection reloadKey;
  final ThemeData theme;
  final Future<List<NpcGlossaryCatalogEntry>> Function()? npcCatalogLoader;
  final Future<GlossarySegmentTextCatalog> Function()? segmentTextCatalogLoader;

  @override
  ConsumerState<GlossaryDetail> createState() => _GlossaryDetailState();
}

class _GlossaryDetailState extends ConsumerState<GlossaryDetail> {
  static const _tutorialPendingKey = 'progression.tutorials';
  static const _tutorialQuestStates = <String>[
    'EQuestState::None',
    'EQuestState::Available',
    'EQuestState::Running',
    'EQuestState::Succeeded',
    'EQuestState::Failed',
  ];
  static const _tutorialGateOrder = <String>[
    'Tut_CombatBasics',
    'Tut_Crafting',
    'Tut_Crime',
    'Tut_Drugs',
    'Tut_Lockpicking',
    'Tut_Magic',
    'Tut_Map',
    'Tut_MeleeCombat',
    'Tut_Navigation',
    'Tut_Perception',
    'Tut_PlayerProgression',
    'Tut_Ranged',
    'Tut_Riding',
    'Tut_Sleep',
    'Tut_Trading',
  ];

  final TextEditingController _search = TextEditingController();
  final Map<String, bool> _pending = {};
  final Map<String, QuestStateChange> _tutorialPending = {};
  void Function()? _removeNotifierListener;
  List<_GlossaryDocument> _documents = const [];
  List<ProgressionQuest> _tutorials = const [];
  _GlossarySection _section = _GlossarySection.oldCamp;
  _NpcFilter _npcFilter = _NpcFilter.all;
  String _query = '';
  String? _selectedDocumentClass;
  String? _error;
  String? _tutorialError;
  bool _loading = false;
  bool _characterStatusAvailable = false;
  bool _npcStatusAvailable = false;
  int _loadEpoch = 0;

  @override
  void initState() {
    super.initState();
    _listenToNotifier();
    _load();
  }

  @override
  void didUpdateWidget(covariant GlossaryDetail oldWidget) {
    super.didUpdateWidget(oldWidget);
    final notifierChanged = !identical(widget.notifier, oldWidget.notifier);
    if (notifierChanged) {
      _removeNotifierListener?.call();
      _listenToNotifier();
    }
    if (widget.reloadKey != oldWidget.reloadKey || notifierChanged) {
      final sameSave =
          !notifierChanged &&
          widget.reloadKey.path != null &&
          widget.reloadKey.path == oldWidget.reloadKey.path;
      if (!sameSave) {
        // A different slot must never inherit the previous save's documents or
        // optimistic edits. For a same-save refresh (notably after Save), keep
        // both until _load atomically replaces them with the fresh snapshot.
        // Clearing the overrides immediately exposed the old document snapshot
        // for a few seconds, making switches and entry counts visibly revert.
        _pending.clear();
        _tutorialPending.clear();
        _documents = const [];
        _tutorials = const [];
        _selectedDocumentClass = null;
        _search.clear();
        _query = '';
        _npcFilter = _NpcFilter.all;
      }
      _load();
    }
  }

  @override
  void dispose() {
    _removeNotifierListener?.call();
    _search.dispose();
    super.dispose();
  }

  void _listenToNotifier() {
    _removeNotifierListener = widget.notifier.addListener((_) {
      if (!mounted) return;
      final tutorialPending = _registeredTutorialPending(_tutorials);
      setState(() {
        _tutorialPending
          ..clear()
          ..addAll(tutorialPending);
      });
    }, fireImmediately: false);
  }

  Future<void> _load() async {
    final epoch = ++_loadEpoch;
    setState(() {
      _loading = true;
      _error = null;
      _tutorialError = null;
    });
    try {
      final results = await Future.wait<Object>([
        widget.notifier.loadGlossary(),
        widget.notifier.loadProgressionTutorials(),
        (widget.npcCatalogLoader ?? loadGlossaryNpcCatalog)(),
        (widget.segmentTextCatalogLoader ?? loadGlossarySegmentTextCatalog)(),
        widget.notifier.loadAllCharacters(),
        widget.notifier.loadAllNpcActors(),
      ]);
      if (!mounted || epoch != _loadEpoch) return;

      final glossary = results[0] as GlossaryPage;
      final tutorials = results[1] as ProgressionQuestPage;
      final npcCatalog = results[2] as List<NpcGlossaryCatalogEntry>;
      final segmentTextCatalog = results[3] as GlossarySegmentTextCatalog;
      final characters = results[4] as CharacterIndexPage;
      final npcActors = results[5] as NpcActorsPage;
      if (glossary.error != null) {
        setState(() {
          _loading = false;
          _error = glossary.error;
          _documents = const [];
          _tutorials = const [];
          _characterStatusAvailable = false;
          _npcStatusAvailable = false;
          _tutorialError = tutorials.error;
        });
        return;
      }

      final characterByUniqueName = <String, CharacterRow>{
        for (final row in characters.characters)
          row.uniqueName.toLowerCase(): row,
      };
      final npcById = <String, NpcActor>{
        for (final npc in npcActors.npcs) npc.id.toLowerCase(): npc,
      };
      final rawUnlocks = <String, GlossarySegmentUnlock>{
        for (final unlock in glossary.segmentUnlocks)
          _segmentKey(unlock.documentClass, unlock.segmentClass): unlock,
      };

      final documents = <_GlossaryDocument>[];
      for (final entry in npcCatalog) {
        final character = characterByUniqueName[entry.uniqueName.toLowerCase()];
        final actor = character?.globalId == null
            ? null
            : npcById[character!.globalId!.toLowerCase()];
        documents.add(
          _GlossaryDocument.fromNpc(
            entry,
            rawUnlocks,
            segmentTextCatalog: segmentTextCatalog,
            writable: glossary.canSetSegment,
            npcGlobalId: character?.globalId,
            // Only an explicit permanent override is persisted. The game's
            // computed guild/story/area relationship is unavailable offline,
            // so never manufacture Neutral or infer Enemy from stale crime
            // snapshots here.
            relationship: actor?.personalRelationship,
          ),
        );
      }
      for (final category in glossary.categories) {
        final section = switch (category.id.toLowerCase()) {
          'creatures' || 'creature' => _GlossarySection.creatures,
          'locations' || 'location' => _GlossarySection.locations,
          _ => null,
        };
        if (section == null) continue;
        documents.addAll(
          category.entries.map(
            (entry) => _GlossaryDocument.fromProgression(
              entry,
              section,
              segmentTextCatalog,
            ),
          ),
        );
      }

      if (!mounted || epoch != _loadEpoch) return;
      final pending = <String, bool>{};
      for (final document in documents) {
        for (final segment in document.segments) {
          final target = widget.notifier.pendingGlossarySegment(
            segment.documentClass,
            segment.segmentClass,
          );
          if (target != null) {
            pending[_segmentKey(segment.documentClass, segment.segmentClass)] =
                target;
          }
        }
      }
      final tutorialRows = tutorials.error == null
          ? tutorials.quests.where(_isTutorialGate).toList(growable: false)
          : const <ProgressionQuest>[];
      final tutorialPending = _registeredTutorialPending(tutorialRows);
      final warnings = <String>{
        if (characters.error?.trim().isNotEmpty == true) characters.error!,
        if (npcActors.error?.trim().isNotEmpty == true) npcActors.error!,
      };
      setState(() {
        _loading = false;
        _documents = documents;
        _tutorials = tutorialRows;
        _pending
          ..clear()
          ..addAll(pending);
        _tutorialPending
          ..clear()
          ..addAll(tutorialPending);
        _characterStatusAvailable = characters.error == null;
        _npcStatusAvailable = npcActors.error == null;
        _tutorialError = tutorials.error;
        // Character/NPC status enriches two filters. Keep the glossary usable
        // on a partial failure, but surface it and disable filters whose answer
        // would otherwise be silently incomplete.
        _error = warnings.isEmpty ? null : warnings.join('\n');
        if (!_npcFilterAvailable(_npcFilter)) {
          _npcFilter = _NpcFilter.all;
        }
      });
    } catch (error) {
      if (!mounted || epoch != _loadEpoch) return;
      final message = AppLocalizations.of(
        context,
      ).glossaryLoadFailed(error.toString());
      setState(() {
        _loading = false;
        _documents = const [];
        _tutorials = const [];
        _characterStatusAvailable = false;
        _npcStatusAvailable = false;
        _error = message;
        _tutorialError = message;
      });
    }
  }

  bool _isTutorialGate(ProgressionQuest quest) =>
      _tutorialGateId(quest).startsWith('Tut_');

  String _tutorialGateId(ProgressionQuest quest) {
    if (quest.name.startsWith('Tut_')) return quest.name;
    const prefix = 'Quest_Tutorials_';
    if (quest.id.startsWith(prefix)) return quest.id.substring(prefix.length);
    return quest.name.isNotEmpty ? quest.name : quest.id;
  }

  Map<String, QuestStateChange> _registeredTutorialPending(
    List<ProgressionQuest> tutorials,
  ) {
    final registered = widget.notifier.pendingEditFor(_tutorialPendingKey);
    if (registered == null) return const {};
    final result = <String, QuestStateChange>{};
    for (final edit in registered.edits) {
      if (edit['path'] != 'private.typed.setValue') continue;
      final value = edit['value'];
      if (value is! Map || value['value'] is! String) continue;
      final rawPath = value['path'];
      if (rawPath is! List) continue;
      final path = rawPath.whereType<String>().toList(growable: false);
      for (final tutorial in tutorials) {
        if (!listEquals(path, tutorial.statePath)) continue;
        result[tutorial.questClass] = QuestStateChange(
          statePath: tutorial.statePath,
          state: value['value'] as String,
        );
        break;
      }
    }
    return result;
  }

  String? _effectiveTutorialState(ProgressionQuest tutorial) =>
      _tutorialPending[tutorial.questClass]?.state ?? tutorial.currentState;

  bool _tutorialUnlocked(ProgressionQuest tutorial) {
    final state = _effectiveTutorialState(tutorial);
    return state == 'EQuestState::Running' || state == 'EQuestState::Succeeded';
  }

  int get _tutorialUnlockedCount => _tutorials.where(_tutorialUnlocked).length;

  void _pushTutorialPending() {
    if (_tutorialPending.isEmpty) {
      widget.notifier.clearPendingEdit(_tutorialPendingKey);
      return;
    }
    widget.notifier.setPendingEdit(
      _tutorialPendingKey,
      PendingSaveEdit(
        edits: _tutorialPending.values
            .map((change) => change.toEditJson())
            .toList(growable: false),
      ),
    );
  }

  void _setTutorialState(ProgressionQuest tutorial, String? state) {
    setState(() {
      if (state == null || state == tutorial.currentState) {
        _tutorialPending.remove(tutorial.questClass);
      } else {
        _tutorialPending[tutorial.questClass] = QuestStateChange(
          statePath: tutorial.statePath,
          state: state,
        );
      }
    });
    _pushTutorialPending();
  }

  void _resetTutorialPending() {
    if (_tutorialPending.isEmpty) return;
    setState(_tutorialPending.clear);
    widget.notifier.clearPendingEdit(_tutorialPendingKey);
  }

  bool _effectiveSegment(_GlossarySegment segment) =>
      _pending[_segmentKey(segment.documentClass, segment.segmentClass)] ??
      segment.unlocked;

  bool _segmentHasPendingEdit(_GlossarySegment segment) =>
      widget.notifier.pendingGlossarySegment(
        segment.documentClass,
        segment.segmentClass,
      ) !=
      null;

  bool _documentHasPendingEdit(_GlossaryDocument document) =>
      document.segments.any(_segmentHasPendingEdit);

  bool _documentVisible(_GlossaryDocument document) =>
      document.segments.any(_effectiveSegment);

  bool _hasEffectiveRole(_GlossaryDocument document, NpcGlossaryRole role) =>
      document.segments.any(
        (segment) => segment.roles.contains(role) && _effectiveSegment(segment),
      );

  NpcRelationship? _effectiveRelationship(_GlossaryDocument document) {
    final npcGlobalId = document.npcGlobalId;
    return (npcGlobalId == null
            ? null
            : widget.notifier.pendingNpcRelationship(npcGlobalId)) ??
        document.relationship;
  }

  bool _isHostile(_GlossaryDocument document) =>
      _effectiveRelationship(document) == NpcRelationship.enemy;

  bool _matchesNpcFilter(_GlossaryDocument document) {
    return switch (_npcFilter) {
      _NpcFilter.all => true,
      _NpcFilter.traders => _hasEffectiveRole(document, NpcGlossaryRole.trader),
      _NpcFilter.teachers => _hasEffectiveRole(
        document,
        NpcGlossaryRole.teacher,
      ),
      _NpcFilter.armorers => _hasEffectiveRole(
        document,
        NpcGlossaryRole.armorer,
      ),
      _NpcFilter.hostile => _isHostile(document),
      // The in-game Dead glossary subcategory is a BySegmentName filter for
      // `_Dead`, independent from the actor's authoritative State.Dead tag.
      // This also makes removing the glossary entry update the filter live.
      _NpcFilter.dead => _hasEffectiveRole(document, NpcGlossaryRole.dead),
    };
  }

  bool _npcFilterAvailable(_NpcFilter filter) => switch (filter) {
    _NpcFilter.hostile => _characterStatusAvailable && _npcStatusAvailable,
    _ => true,
  };

  List<_GlossaryDocument> _visibleInSection(_GlossarySection section) =>
      _documents
          .where(
            (document) =>
                document.section == section && _documentVisible(document),
          )
          .toList(growable: false);

  int _sectionCount(_GlossarySection section) =>
      section == _GlossarySection.tutorials
      ? _tutorialUnlockedCount
      : _visibleInSection(section).length;

  List<_GlossaryDocument> _filteredDocuments(
    Map<String, Map<String, String>> catalog,
    GameLang lang,
  ) {
    final query = _query;
    final result = _visibleInSection(_section).where((document) {
      if (document.isNpc && !_matchesNpcFilter(document)) return false;
      if (query.isEmpty) return true;
      final name = document.displayName(catalog, lang).toLowerCase();
      return name.contains(query) ||
          document.rawName.toLowerCase().contains(query) ||
          document.documentClass.toLowerCase().contains(query);
    }).toList();
    result.sort(
      (a, b) => a
          .displayName(catalog, lang)
          .toLowerCase()
          .compareTo(b.displayName(catalog, lang).toLowerCase()),
    );
    return result;
  }

  void _selectSection(_GlossarySection section) {
    setState(() {
      _section = section;
      _npcFilter = _NpcFilter.all;
      _selectedDocumentClass = null;
      _query = '';
      _search.clear();
    });
  }

  void _setSegment(
    _GlossaryDocument document,
    _GlossarySegment segment,
    bool unlocked,
  ) {
    final key = _segmentKey(document.documentClass, segment.segmentClass);
    setState(() {
      if (unlocked == segment.unlocked) {
        _pending.remove(key);
      } else {
        _pending[key] = unlocked;
      }
    });
    if (unlocked == segment.unlocked) {
      widget.notifier.clearPendingGlossarySegment(
        document.documentClass,
        segment.segmentClass,
      );
    } else {
      widget.notifier.setPendingGlossarySegment(
        GlossarySegmentEdit(
          documentClass: document.documentClass,
          segmentClass: segment.segmentClass,
          unlocked: unlocked,
          questStatePath: segment.questStatePath,
        ),
      );
    }
    if (!_documentVisible(document)) {
      setState(() => _selectedDocumentClass = null);
    }
  }

  void _resetPending() {
    final pendingSegments = <_GlossarySegment>[
      for (final document in _documents)
        for (final segment in document.segments)
          if (_segmentHasPendingEdit(segment)) segment,
    ];
    for (final segment in pendingSegments) {
      widget.notifier.clearPendingGlossarySegment(
        segment.documentClass,
        segment.segmentClass,
      );
    }
    setState(() {
      for (final segment in pendingSegments) {
        _pending.remove(
          _segmentKey(segment.documentClass, segment.segmentClass),
        );
      }
    });
  }

  List<_GlossaryDocument> _addableHiddenDocuments() => _documents
      .where(
        (document) =>
            document.section == _section &&
            !_documentVisible(document) &&
            document.primarySegment?.writable == true,
      )
      .toList();

  Future<void> _showAddDialog(
    Map<String, Map<String, String>> catalog,
    GameLang lang,
  ) async {
    // Only offer entries whose primary segment can actually be written. On a
    // save without a clean SegmentUnlocked template the glossary is readable,
    // but selecting an unwritable hidden entry would otherwise be a silent
    // no-op after the dialog closes.
    final hidden = _addableHiddenDocuments();
    hidden.sort(
      (a, b) => a
          .displayName(catalog, lang)
          .toLowerCase()
          .compareTo(b.displayName(catalog, lang).toLowerCase()),
    );
    final selected = await showDialog<_GlossaryDocument>(
      context: context,
      builder: (context) => _AddGlossaryEntryDialog(
        documents: hidden,
        catalog: catalog,
        lang: lang,
        showObjectIds: ref.read(showObjectIdsProvider),
      ),
    );
    if (selected == null || !mounted) return;
    final primary = selected.primarySegment;
    if (primary == null || !primary.writable) return;
    _setSegment(selected, primary, true);
    setState(() => _selectedDocumentClass = selected.documentClass);
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final lang = ref.watch(currentGameLangProvider);
    final locCatalog = ref.watch(locCatalogProvider).value ?? const {};
    final showObjectIds = ref.watch(showObjectIdsProvider);
    final scheme = widget.theme.colorScheme;
    final filtered = _filteredDocuments(locCatalog, lang);
    final selected = _documents.cast<_GlossaryDocument?>().firstWhere(
      (document) =>
          document?.documentClass == _selectedDocumentClass &&
          _documentVisible(document!),
      orElse: () => null,
    );

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Stack(
          fit: StackFit.expand,
          children: [
            ExcludeFocus(
              excluding: _loading,
              child: AbsorbPointer(
                absorbing: _loading,
                child: _loading && _documents.isEmpty
                    ? const Center(child: CircularProgressIndicator())
                    : _error != null && _documents.isEmpty
                    ? Center(
                        child: ConstrainedBox(
                          constraints: const BoxConstraints(maxWidth: 520),
                          child: Text(
                            _error!,
                            textAlign: TextAlign.center,
                            style: TextStyle(color: scheme.error),
                          ),
                        ),
                      )
                    : LayoutBuilder(
                        builder: (context, constraints) {
                          if (_section == _GlossarySection.tutorials) {
                            return _buildTutorialLayout(
                              l10n,
                              constraints,
                              showObjectIds,
                            );
                          }
                          const categoryWidth = 220.0;
                          const documentWidth = 300.0;
                          const dividerAndSpacingWidth = 50.0;
                          const minimumDetailWidth = 250.0;
                          final compact =
                              constraints.maxWidth <
                              categoryWidth +
                                  documentWidth +
                                  dividerAndSpacingWidth +
                                  minimumDetailWidth;
                          final narrow = constraints.maxWidth < 520;
                          final list = _buildDocumentList(
                            l10n,
                            locCatalog,
                            lang,
                            filtered,
                            showObjectIds,
                          );
                          final detail = selected == null
                              ? _GlossaryEmptyDetail(
                                  message: l10n.glossarySelectEntry,
                                )
                              : _buildEntryDetail(
                                  l10n,
                                  locCatalog,
                                  lang,
                                  selected,
                                  showObjectIds,
                                );
                          if (narrow) {
                            if (selected != null) {
                              return Column(
                                crossAxisAlignment: CrossAxisAlignment.stretch,
                                children: [
                                  Align(
                                    alignment: Alignment.centerLeft,
                                    child: BackButton(
                                      key: const Key('glossary-detail-back'),
                                      onPressed: _loading
                                          ? null
                                          : () => setState(
                                              () =>
                                                  _selectedDocumentClass = null,
                                            ),
                                    ),
                                  ),
                                  Expanded(child: detail),
                                ],
                              );
                            }
                            return Column(
                              crossAxisAlignment: CrossAxisAlignment.stretch,
                              children: [
                                _buildCategoryDropdown(l10n),
                                const SizedBox(height: 8),
                                Expanded(child: list),
                              ],
                            );
                          }
                          if (compact) {
                            final listWidth = (constraints.maxWidth * .46)
                                .clamp(210.0, 300.0)
                                .toDouble();
                            return Row(
                              crossAxisAlignment: CrossAxisAlignment.stretch,
                              children: [
                                SizedBox(
                                  width: listWidth,
                                  child: Column(
                                    crossAxisAlignment:
                                        CrossAxisAlignment.stretch,
                                    children: [
                                      _buildCategoryDropdown(l10n),
                                      const SizedBox(height: 8),
                                      Expanded(child: list),
                                    ],
                                  ),
                                ),
                                const SizedBox(width: 10),
                                const VerticalDivider(width: 1),
                                const SizedBox(width: 10),
                                Expanded(child: detail),
                              ],
                            );
                          }
                          return Row(
                            crossAxisAlignment: CrossAxisAlignment.stretch,
                            children: [
                              SizedBox(
                                width: categoryWidth,
                                child: _buildCategoryPicker(l10n),
                              ),
                              const SizedBox(width: 12),
                              const VerticalDivider(width: 1),
                              const SizedBox(width: 12),
                              SizedBox(width: documentWidth, child: list),
                              const SizedBox(width: 12),
                              const VerticalDivider(width: 1),
                              const SizedBox(width: 12),
                              Expanded(child: detail),
                            ],
                          );
                        },
                      ),
              ),
            ),
            if (_loading && _documents.isNotEmpty)
              const Positioned(
                top: 0,
                left: 0,
                right: 0,
                child: LinearProgressIndicator(minHeight: 2),
              ),
          ],
        ),
      ),
    );
  }

  Widget _buildCategoryPicker(AppLocalizations l10n) {
    return ListView(
      children: [
        for (final section in _GlossarySection.values)
          ListTile(
            dense: true,
            selected: _section == section,
            selectedTileColor: widget.theme.colorScheme.primaryContainer,
            selectedColor: widget.theme.colorScheme.primary,
            leading: Icon(_sectionIcon(section), size: 19),
            title: Text(
              _sectionLabel(l10n, section),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
            trailing: Text('${_sectionCount(section)}'),
            onTap: _loading ? null : () => _selectSection(section),
          ),
      ],
    );
  }

  Widget _buildCategoryDropdown(AppLocalizations l10n) {
    return KeyedSubtree(
      key: const Key('glossary-category-dropdown'),
      child: DropdownButtonFormField<_GlossarySection>(
        key: ValueKey(_section),
        initialValue: _section,
        isExpanded: true,
        decoration: const InputDecoration(isDense: true),
        items: [
          for (final section in _GlossarySection.values)
            DropdownMenuItem(
              value: section,
              child: Row(
                children: [
                  Icon(_sectionIcon(section), size: 18),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      _sectionLabel(l10n, section),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  const SizedBox(width: 6),
                  Text('${_sectionCount(section)}'),
                ],
              ),
            ),
        ],
        onChanged: _loading
            ? null
            : (section) {
                if (section != null && section != _section) {
                  _selectSection(section);
                }
              },
      ),
    );
  }

  Widget _buildTutorialLayout(
    AppLocalizations l10n,
    BoxConstraints constraints,
    bool showObjectIds,
  ) {
    final list = _buildTutorialGateList(l10n, showObjectIds);
    if (constraints.maxWidth < 640) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _buildCategoryDropdown(l10n),
          const SizedBox(height: 8),
          Expanded(child: list),
        ],
      );
    }
    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SizedBox(width: 220, child: _buildCategoryPicker(l10n)),
        const SizedBox(width: 12),
        const VerticalDivider(width: 1),
        const SizedBox(width: 12),
        Expanded(child: list),
      ],
    );
  }

  List<ProgressionQuest> _filteredTutorials(AppLocalizations l10n) {
    final query = _query;
    final tutorials = _tutorials.where((tutorial) {
      if (query.isEmpty) return true;
      return _tutorialGateTitle(l10n, tutorial).toLowerCase().contains(query) ||
          _tutorialGateId(tutorial).toLowerCase().contains(query) ||
          tutorial.id.toLowerCase().contains(query);
    }).toList();
    tutorials.sort((a, b) {
      final aIndex = _tutorialGateOrder.indexOf(_tutorialGateId(a));
      final bIndex = _tutorialGateOrder.indexOf(_tutorialGateId(b));
      final normalizedA = aIndex < 0 ? _tutorialGateOrder.length : aIndex;
      final normalizedB = bIndex < 0 ? _tutorialGateOrder.length : bIndex;
      return normalizedA.compareTo(normalizedB);
    });
    return tutorials;
  }

  Widget _buildTutorialGateList(AppLocalizations l10n, bool showObjectIds) {
    final tutorials = _filteredTutorials(l10n);
    return Column(
      key: const Key('glossary-tutorial-gates'),
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                l10n.glossaryTutorials,
                style: widget.theme.textTheme.titleMedium,
              ),
            ),
            if (widget.editable && _tutorialPending.isNotEmpty)
              IconButton(
                tooltip: l10n.tutorialResetChanges,
                onPressed: _loading ? null : _resetTutorialPending,
                icon: const Icon(Icons.undo_outlined),
              ),
          ],
        ),
        const SizedBox(height: 4),
        Text(
          l10n.tutorialGateNote,
          style: widget.theme.textTheme.bodySmall?.copyWith(
            color: widget.theme.colorScheme.onSurfaceVariant,
          ),
        ),
        if (_tutorialError != null) ...[
          const SizedBox(height: 8),
          Text(
            _tutorialError!,
            style: TextStyle(color: widget.theme.colorScheme.error),
          ),
        ],
        const SizedBox(height: 10),
        TextField(
          key: const Key('tutorial-gate-search'),
          controller: _search,
          decoration: InputDecoration(
            labelText: l10n.glossarySearch,
            prefixIcon: const Icon(Icons.search),
          ),
          onChanged: _loading
              ? null
              : (value) => setState(() => _query = value.trim().toLowerCase()),
        ),
        const SizedBox(height: 8),
        Text(
          l10n.tutorialGateUnlockCount(
            _tutorialUnlockedCount,
            _tutorials.length,
          ),
          style: widget.theme.textTheme.bodySmall?.copyWith(
            color: widget.theme.colorScheme.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: 4),
        Expanded(
          child: tutorials.isEmpty
              ? _GlossaryEmptyDetail(
                  message: _tutorials.isEmpty
                      ? l10n.tutorialNoGates
                      : l10n.glossaryNoMatch,
                )
              : ListView.separated(
                  key: const Key('tutorial-gate-list'),
                  itemCount: tutorials.length,
                  separatorBuilder: (_, _) => const Divider(height: 1),
                  itemBuilder: (context, index) {
                    final tutorial = tutorials[index];
                    final effectiveState = _effectiveTutorialState(tutorial);
                    final pending = _tutorialPending.containsKey(
                      tutorial.questClass,
                    );
                    final knownState =
                        effectiveState != null &&
                        _tutorialQuestStates.contains(effectiveState);
                    return ListTile(
                      key: ValueKey('tutorial-gate-${tutorial.id}'),
                      dense: true,
                      leading: Icon(
                        _tutorialUnlocked(tutorial)
                            ? Icons.visibility_outlined
                            : Icons.visibility_off_outlined,
                      ),
                      title: Text(_tutorialGateTitle(l10n, tutorial)),
                      subtitle: !showObjectIds && !pending
                          ? null
                          : Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                if (showObjectIds)
                                  SelectableText(
                                    tutorial.id,
                                    maxLines: 1,
                                    style: widget.theme.textTheme.bodySmall
                                        ?.copyWith(
                                          color: widget
                                              .theme
                                              .colorScheme
                                              .onSurfaceVariant,
                                          fontFamily:
                                              uiAwareMonospaceFontFamily(
                                                context,
                                              ),
                                        ),
                                  ),
                                if (pending) Text(l10n.glossaryPending),
                              ],
                            ),
                      trailing:
                          widget.editable && tutorial.writable && knownState
                          ? DropdownButton<String>(
                              key: ValueKey('tutorial-state-${tutorial.id}'),
                              value: effectiveState,
                              underline: const SizedBox.shrink(),
                              items: [
                                for (final state in _tutorialQuestStates)
                                  DropdownMenuItem(
                                    value: state,
                                    child: Text(
                                      _localizedTutorialState(l10n, state),
                                    ),
                                  ),
                              ],
                              onChanged: (state) =>
                                  _setTutorialState(tutorial, state),
                            )
                          : Text(_localizedTutorialState(l10n, effectiveState)),
                    );
                  },
                ),
        ),
      ],
    );
  }

  String _tutorialGateTitle(AppLocalizations l10n, ProgressionQuest tutorial) =>
      switch (_tutorialGateId(tutorial)) {
        'Tut_CombatBasics' => l10n.tutorialGateCombatBasics,
        'Tut_Crafting' => l10n.tutorialGateCrafting,
        'Tut_Crime' => l10n.tutorialGateCrime,
        'Tut_Drugs' => l10n.tutorialGateDrugs,
        'Tut_Lockpicking' => l10n.tutorialGateLockpicking,
        'Tut_Magic' => l10n.tutorialGateMagic,
        'Tut_Map' => l10n.tutorialGateMap,
        'Tut_MeleeCombat' => l10n.tutorialGateMeleeCombat,
        'Tut_Navigation' => l10n.tutorialGateNavigation,
        'Tut_Perception' => l10n.tutorialGatePerception,
        'Tut_PlayerProgression' => l10n.tutorialGatePlayerProgression,
        'Tut_Ranged' => l10n.tutorialGateRanged,
        'Tut_Riding' => l10n.tutorialGateRiding,
        'Tut_Sleep' => l10n.tutorialGateSleep,
        'Tut_Trading' => l10n.tutorialGateTrading,
        final id => _humanize(id.replaceFirst('Tut_', '')),
      };

  String _localizedTutorialState(AppLocalizations l10n, String? rawState) =>
      switch (rawState?.split('::').last) {
        'None' => l10n.questStateNone,
        'Available' => l10n.questStateAvailable,
        'Running' => l10n.questStateRunning,
        'Succeeded' => l10n.questStateSucceeded,
        'Failed' => l10n.questStateFailed,
        _ => l10n.questStateUnknown,
      };

  Widget _buildDocumentList(
    AppLocalizations l10n,
    Map<String, Map<String, String>> catalog,
    GameLang lang,
    List<_GlossaryDocument> filtered,
    bool showObjectIds,
  ) {
    final isNpcSection = _section.index <= _GlossarySection.outsiders.index;
    final addableHiddenCount = _addableHiddenDocuments().length;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: TextField(
                controller: _search,
                decoration: InputDecoration(
                  labelText: l10n.glossarySearch,
                  prefixIcon: const Icon(Icons.search),
                ),
                onChanged: _loading
                    ? null
                    : (value) =>
                          setState(() => _query = value.trim().toLowerCase()),
              ),
            ),
            if (widget.editable) ...[
              const SizedBox(width: 4),
              IconButton(
                tooltip: l10n.glossaryAddEntry,
                onPressed: _loading || addableHiddenCount == 0
                    ? null
                    : () => _showAddDialog(catalog, lang),
                icon: const Icon(Icons.add_circle_outline),
              ),
              if (_documents.any(_documentHasPendingEdit))
                IconButton(
                  tooltip: l10n.glossaryResetChanges,
                  onPressed: _loading ? null : _resetPending,
                  icon: const Icon(Icons.undo_outlined),
                ),
            ],
          ],
        ),
        if (_error != null) ...[
          const SizedBox(height: 6),
          Text(
            _error!,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(color: widget.theme.colorScheme.error),
          ),
        ],
        if (isNpcSection) ...[
          const SizedBox(height: 8),
          KeyedSubtree(
            key: const Key('glossary-npc-filter-dropdown'),
            child: DropdownButtonFormField<_NpcFilter>(
              key: ValueKey((_section, _npcFilter)),
              initialValue: _npcFilter,
              isExpanded: true,
              decoration: InputDecoration(
                isDense: true,
                labelText: l10n.glossaryFilterLabel,
                prefixIcon: const Icon(Icons.filter_list_outlined),
              ),
              items: [
                for (final filter in _NpcFilter.values)
                  DropdownMenuItem(
                    value: filter,
                    enabled: _npcFilterAvailable(filter),
                    child: Text(
                      _npcFilterLabel(l10n, filter),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
              ],
              onChanged: _loading
                  ? null
                  : (filter) {
                      if (filter == null || filter == _npcFilter) return;
                      setState(() {
                        _npcFilter = filter;
                        _selectedDocumentClass = null;
                      });
                    },
            ),
          ),
          if (_npcFilter == _NpcFilter.hostile) ...[
            const SizedBox(height: 6),
            Text(
              l10n.glossaryRelationshipFilterNote,
              style: widget.theme.textTheme.bodySmall?.copyWith(
                color: widget.theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ],
        const SizedBox(height: 6),
        Text(
          l10n.glossaryEntryCount(filtered.length),
          style: widget.theme.textTheme.bodySmall?.copyWith(
            color: widget.theme.colorScheme.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: 4),
        Expanded(
          child: filtered.isEmpty
              ? _GlossaryEmptyDetail(message: l10n.glossaryNoVisibleEntries)
              : ListView.separated(
                  itemCount: filtered.length,
                  separatorBuilder: (_, _) => const Divider(height: 1),
                  itemBuilder: (context, index) {
                    final document = filtered[index];
                    final unlockedCount = document.segments
                        .where(_effectiveSegment)
                        .length;
                    final portraitUnlocked =
                        document.isNpc &&
                        _hasEffectiveRole(document, NpcGlossaryRole.portrait);
                    return ListTile(
                      dense: true,
                      selected:
                          _selectedDocumentClass == document.documentClass,
                      leading: document.isNpc
                          ? CircleAvatar(
                              radius: 17,
                              child: Icon(
                                portraitUnlocked
                                    ? Icons.person
                                    : Icons.person_outline,
                                size: 20,
                              ),
                            )
                          : Icon(_sectionIcon(document.section)),
                      title: Text(
                        document.displayName(catalog, lang),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      subtitle: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Text(
                            l10n.glossarySegmentsCount(
                              unlockedCount,
                              document.segments.length,
                            ),
                          ),
                          if (showObjectIds && document.technicalNpcId != null)
                            Text(
                              document.technicalNpcId!,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: TextStyle(
                                fontFamily: uiAwareMonospaceFontFamily(context),
                                fontSize: 11,
                              ),
                            ),
                        ],
                      ),
                      trailing: _documentHasPendingEdit(document)
                          ? Icon(
                              Icons.edit_outlined,
                              size: 17,
                              color: widget.theme.colorScheme.primary,
                            )
                          : null,
                      onTap: _loading
                          ? null
                          : () => setState(
                              () => _selectedDocumentClass =
                                  document.documentClass,
                            ),
                    );
                  },
                ),
        ),
      ],
    );
  }

  List<String> _localizedSegmentParagraphs(
    Map<String, Map<String, String>> catalog,
    GameLang lang,
    _GlossarySegment segment,
  ) {
    final paragraphs = <String>[];
    for (final textId in segment.textIds) {
      final text = resolveGameText(catalog, textId, lang)?.trim();
      if (text != null && text.isNotEmpty) paragraphs.add(text);
    }
    return paragraphs;
  }

  Widget _buildSegmentText(
    AppLocalizations l10n,
    _GlossarySegment segment,
    List<String> paragraphs,
  ) {
    if (paragraphs.isEmpty) return Text(_segmentLabel(l10n, segment));

    final fullText = paragraphs.join('\n\n');
    return Tooltip(
      message: fullText,
      waitDuration: const Duration(milliseconds: 450),
      constraints: const BoxConstraints(maxWidth: 520),
      child: Text(fullText, maxLines: 2, overflow: TextOverflow.ellipsis),
    );
  }

  Future<void> _showSegmentTextDialog(
    AppLocalizations l10n,
    String documentName,
    List<String> paragraphs,
  ) {
    return showDialog<void>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(documentName),
        content: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 620, maxHeight: 520),
          child: SingleChildScrollView(
            child: SelectionArea(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  for (var index = 0; index < paragraphs.length; index++) ...[
                    Text(
                      paragraphs[index],
                      style: widget.theme.textTheme.bodyLarge,
                    ),
                    if (index + 1 < paragraphs.length)
                      const SizedBox(height: 14),
                  ],
                ],
              ),
            ),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: Text(l10n.close),
          ),
        ],
      ),
    );
  }

  Widget _buildEntryDetail(
    AppLocalizations l10n,
    Map<String, Map<String, String>> catalog,
    GameLang lang,
    _GlossaryDocument document,
    bool showObjectIds,
  ) {
    final name = document.displayName(catalog, lang);
    final portraitUnlocked =
        document.isNpc && _hasEffectiveRole(document, NpcGlossaryRole.portrait);
    final statusRoles = <NpcGlossaryRole>[
      NpcGlossaryRole.trader,
      NpcGlossaryRole.teacher,
      NpcGlossaryRole.armorer,
    ].where((role) => _hasEffectiveRole(document, role)).toList();
    final deadEntryUnlocked =
        document.isNpc && _hasEffectiveRole(document, NpcGlossaryRole.dead);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (document.isNpc) ...[
              CircleAvatar(
                radius: 28,
                child: Icon(
                  portraitUnlocked ? Icons.person : Icons.person_outline,
                  size: 34,
                ),
              ),
              const SizedBox(width: 12),
            ] else ...[
              Icon(_sectionIcon(document.section), size: 42),
              const SizedBox(width: 12),
            ],
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(name, style: widget.theme.textTheme.titleLarge),
                  if (showObjectIds && document.technicalNpcId != null)
                    SelectableText(
                      document.technicalNpcId!,
                      maxLines: 1,
                      style: widget.theme.textTheme.bodySmall?.copyWith(
                        color: widget.theme.colorScheme.onSurfaceVariant,
                        fontFamily: uiAwareMonospaceFontFamily(context),
                      ),
                    ),
                  const SizedBox(height: 2),
                  Text(
                    document.isNpc
                        ? (portraitUnlocked
                              ? l10n.glossaryPortraitUnlocked
                              : l10n.glossaryPortraitSilhouette)
                        : _sectionLabel(l10n, document.section),
                    style: widget.theme.textTheme.bodySmall?.copyWith(
                      color: widget.theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
        if (document.isNpc &&
            (statusRoles.isNotEmpty ||
                _isHostile(document) ||
                deadEntryUnlocked)) ...[
          const SizedBox(height: 12),
          Wrap(
            spacing: 6,
            runSpacing: 6,
            children: [
              for (final role in statusRoles)
                Chip(
                  visualDensity: VisualDensity.compact,
                  avatar: Icon(_roleIcon(role), size: 16),
                  label: Text(_roleLabel(l10n, role)),
                ),
              if (_isHostile(document))
                Chip(
                  visualDensity: VisualDensity.compact,
                  avatar: const Icon(Icons.gpp_bad_outlined, size: 16),
                  label: Text(l10n.glossaryFilterHostile),
                ),
              if (deadEntryUnlocked)
                Chip(
                  visualDensity: VisualDensity.compact,
                  avatar: const Icon(Icons.dangerous_outlined, size: 16),
                  label: Text(l10n.glossaryFilterDead),
                ),
            ],
          ),
        ],
        const SizedBox(height: 12),
        Text(l10n.glossarySegments, style: widget.theme.textTheme.titleMedium),
        const Divider(),
        Expanded(
          child: ListView.separated(
            itemCount: document.segments.length,
            separatorBuilder: (_, _) => const Divider(height: 1),
            itemBuilder: (context, index) {
              final segment = document.segments[index];
              final effective = _effectiveSegment(segment);
              final pending = _segmentHasPendingEdit(segment);
              final paragraphs = _localizedSegmentParagraphs(
                catalog,
                lang,
                segment,
              );
              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  SwitchListTile.adaptive(
                    value: effective,
                    title: _buildSegmentText(l10n, segment, paragraphs),
                    subtitle: pending ? Text(l10n.glossaryPending) : null,
                    secondary: Icon(
                      effective
                          ? Icons.visibility_outlined
                          : Icons.visibility_off_outlined,
                    ),
                    onChanged: !_loading && widget.editable && segment.writable
                        ? (value) => _setSegment(document, segment, value)
                        : null,
                  ),
                  if (paragraphs.isNotEmpty)
                    Align(
                      alignment: Alignment.centerRight,
                      child: IconButton(
                        tooltip: l10n.glossaryShowFullText,
                        visualDensity: VisualDensity.compact,
                        padding: EdgeInsets.zero,
                        constraints: const BoxConstraints.tightFor(
                          width: 32,
                          height: 32,
                        ),
                        onPressed: _loading
                            ? null
                            : () => _showSegmentTextDialog(
                                l10n,
                                name,
                                paragraphs,
                              ),
                        icon: const Icon(Icons.open_in_full, size: 17),
                      ),
                    ),
                ],
              );
            },
          ),
        ),
      ],
    );
  }
}

class _GlossaryDocument {
  const _GlossaryDocument({
    required this.rawName,
    required this.documentClass,
    required this.section,
    required this.segments,
    this.isNpc = false,
    this.uniqueName,
    this.npcCatalogId,
    this.npcGlobalId,
    this.relationship,
  });

  factory _GlossaryDocument.fromProgression(
    GlossaryEntry entry,
    _GlossarySection section,
    GlossarySegmentTextCatalog segmentTextCatalog,
  ) => _GlossaryDocument(
    rawName: entry.name,
    documentClass: entry.documentClass,
    section: section,
    segments: [
      for (final segment in entry.segments)
        _GlossarySegment(
          id: segment.id,
          label: segment.name,
          documentClass: entry.documentClass,
          segmentClass: segment.segmentClass,
          unlocked: segment.unlocked,
          writable: segment.writable,
          questStatePath: segment.statePath,
          textIds:
              segmentTextCatalog[segment.segmentClass.toLowerCase()] ??
              const [],
        ),
    ],
  );

  factory _GlossaryDocument.fromNpc(
    NpcGlossaryCatalogEntry entry,
    Map<String, GlossarySegmentUnlock> rawUnlocks, {
    required GlossarySegmentTextCatalog segmentTextCatalog,
    required bool writable,
    required String? npcGlobalId,
    required NpcRelationship? relationship,
  }) => _GlossaryDocument(
    rawName: entry.id,
    documentClass: entry.documentClass,
    section: switch (entry.camp) {
      NpcGlossaryCamp.oldCamp => _GlossarySection.oldCamp,
      NpcGlossaryCamp.newCamp => _GlossarySection.newCamp,
      NpcGlossaryCamp.swampCamp => _GlossarySection.swampCamp,
      NpcGlossaryCamp.outsiders => _GlossarySection.outsiders,
    },
    isNpc: true,
    uniqueName: entry.uniqueName,
    npcCatalogId: entry.id,
    npcGlobalId: npcGlobalId,
    relationship: relationship,
    segments: [
      for (final segment in entry.segments)
        _GlossarySegment(
          id: segment.id,
          label: segment.label,
          usesNpcCatalogLabel: true,
          documentClass: entry.documentClass,
          segmentClass: segment.segmentClass,
          unlocked:
              rawUnlocks[_segmentKey(entry.documentClass, segment.segmentClass)]
                  ?.unlocked ??
              false,
          writable: writable,
          roles: segment.roles,
          textIds:
              segmentTextCatalog[segment.segmentClass.toLowerCase()] ??
              const [],
        ),
    ],
  );

  final String rawName;
  final String documentClass;
  final _GlossarySection section;
  final List<_GlossarySegment> segments;
  final bool isNpc;
  final String? uniqueName;
  final String? npcCatalogId;
  final String? npcGlobalId;
  final NpcRelationship? relationship;

  /// Best available technical NPC identifier for optional advanced display.
  /// Prefer the actual spawned GlobalId, then the dialog key/catalog id.
  String? get technicalNpcId {
    if (!isNpc) return null;
    for (final candidate in [npcGlobalId, uniqueName, npcCatalogId]) {
      if (candidate?.trim().isNotEmpty == true) return candidate;
    }
    return null;
  }

  _GlossarySegment? get primarySegment {
    if (isNpc) {
      // Prefer the canonical first-meeting segment when a document also has
      // an alternate Introduction_N variant (currently Herek).
      for (final segment in segments) {
        if (segment.id == 'Introduction' &&
            segment.roles.contains(NpcGlossaryRole.portrait)) {
          return segment;
        }
      }
      for (final segment in segments) {
        if (segment.roles.contains(NpcGlossaryRole.portrait)) return segment;
      }
    } else {
      for (final segment in segments) {
        if (segment.id.toLowerCase().contains('unlock') ||
            segment.label.toLowerCase() == 'unlock') {
          return segment;
        }
      }
    }
    return segments.isEmpty ? null : segments.first;
  }

  String displayName(Map<String, Map<String, String>> catalog, GameLang lang) {
    if (isNpc) {
      final localized =
          localizedGameName(catalog, lang, uniqueName ?? '') ??
          localizedGameName(catalog, lang, npcCatalogId ?? '');
      if (localized != null && localized.trim().isNotEmpty) return localized;
      final parts = (npcCatalogId ?? rawName)
          .split('_')
          .where((part) => part.isNotEmpty)
          .toList();
      return _humanize(parts.isEmpty ? rawName : parts.last);
    }
    final localized = localizedGameName(catalog, lang, rawName);
    return localized?.trim().isNotEmpty == true
        ? localized!
        : _humanize(rawName);
  }
}

class _GlossarySegment {
  const _GlossarySegment({
    required this.id,
    required this.label,
    required this.documentClass,
    required this.segmentClass,
    required this.unlocked,
    required this.writable,
    this.usesNpcCatalogLabel = false,
    this.questStatePath = const [],
    this.roles = const {},
    this.textIds = const [],
  });

  final String id;
  final String label;
  final String documentClass;
  final String segmentClass;
  final bool unlocked;
  final bool writable;
  final bool usesNpcCatalogLabel;
  final List<String> questStatePath;
  final Set<NpcGlossaryRole> roles;
  final List<String> textIds;
}

class _AddGlossaryEntryDialog extends StatefulWidget {
  const _AddGlossaryEntryDialog({
    required this.documents,
    required this.catalog,
    required this.lang,
    required this.showObjectIds,
  });

  final List<_GlossaryDocument> documents;
  final Map<String, Map<String, String>> catalog;
  final GameLang lang;
  final bool showObjectIds;

  @override
  State<_AddGlossaryEntryDialog> createState() =>
      _AddGlossaryEntryDialogState();
}

class _AddGlossaryEntryDialogState extends State<_AddGlossaryEntryDialog> {
  final TextEditingController _search = TextEditingController();
  String _query = '';

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final filtered = widget.documents.where((document) {
      if (_query.isEmpty) return true;
      return document
              .displayName(widget.catalog, widget.lang)
              .toLowerCase()
              .contains(_query) ||
          document.rawName.toLowerCase().contains(_query);
    }).toList();
    return AlertDialog(
      title: Text(l10n.glossaryAddTitle),
      content: SizedBox(
        width: 520,
        height: 430,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            TextField(
              controller: _search,
              autofocus: true,
              decoration: InputDecoration(
                labelText: l10n.glossarySearch,
                prefixIcon: const Icon(Icons.search),
              ),
              onChanged: (value) =>
                  setState(() => _query = value.trim().toLowerCase()),
            ),
            const SizedBox(height: 8),
            Expanded(
              child: widget.documents.isEmpty
                  ? _GlossaryEmptyDetail(message: l10n.glossaryNoHiddenEntries)
                  : filtered.isEmpty
                  ? _GlossaryEmptyDetail(message: l10n.glossaryNoMatch)
                  : ListView.separated(
                      itemCount: filtered.length,
                      separatorBuilder: (_, _) => const Divider(height: 1),
                      itemBuilder: (context, index) {
                        final document = filtered[index];
                        final enabled =
                            document.primarySegment?.writable == true;
                        return ListTile(
                          leading: Icon(
                            document.isNpc
                                ? Icons.person_outline
                                : _sectionIcon(document.section),
                          ),
                          title: Text(
                            document.displayName(widget.catalog, widget.lang),
                          ),
                          subtitle: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Text(_sectionLabel(l10n, document.section)),
                              if (widget.showObjectIds &&
                                  document.technicalNpcId != null)
                                Text(
                                  document.technicalNpcId!,
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: TextStyle(
                                    fontFamily: uiAwareMonospaceFontFamily(
                                      context,
                                    ),
                                    fontSize: 11,
                                  ),
                                ),
                            ],
                          ),
                          enabled: enabled,
                          onTap: enabled
                              ? () => Navigator.of(context).pop(document)
                              : null,
                        );
                      },
                    ),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
      ],
    );
  }
}

class _GlossaryEmptyDetail extends StatelessWidget {
  const _GlossaryEmptyDetail({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) => Center(
    child: Padding(
      padding: const EdgeInsets.all(20),
      child: Text(
        message,
        textAlign: TextAlign.center,
        style: Theme.of(context).textTheme.bodyMedium?.copyWith(
          color: Theme.of(context).colorScheme.onSurfaceVariant,
        ),
      ),
    ),
  );
}

String _segmentKey(String documentClass, String segmentClass) =>
    '$documentClass\u0000$segmentClass';

IconData _sectionIcon(_GlossarySection section) => switch (section) {
  _GlossarySection.oldCamp => Icons.fort_outlined,
  _GlossarySection.newCamp => Icons.landscape_outlined,
  _GlossarySection.swampCamp => Icons.grass_outlined,
  _GlossarySection.outsiders => Icons.person_pin_circle_outlined,
  _GlossarySection.creatures => Icons.pets_outlined,
  _GlossarySection.locations => Icons.place_outlined,
  _GlossarySection.tutorials => Icons.school_outlined,
};

String _sectionLabel(AppLocalizations l10n, _GlossarySection section) =>
    switch (section) {
      _GlossarySection.oldCamp => l10n.glossaryOldCamp,
      _GlossarySection.newCamp => l10n.glossaryNewCamp,
      _GlossarySection.swampCamp => l10n.glossarySwampCamp,
      _GlossarySection.outsiders => l10n.glossaryOutsiders,
      _GlossarySection.creatures => l10n.glossaryCreatures,
      _GlossarySection.locations => l10n.glossaryLocations,
      _GlossarySection.tutorials => l10n.glossaryTutorials,
    };

String _npcFilterLabel(AppLocalizations l10n, _NpcFilter filter) =>
    switch (filter) {
      _NpcFilter.all => l10n.categoryAll,
      _NpcFilter.traders => l10n.glossaryFilterTraders,
      _NpcFilter.teachers => l10n.glossaryFilterTeachers,
      _NpcFilter.armorers => l10n.glossaryFilterArmorers,
      _NpcFilter.hostile => l10n.glossaryFilterHostile,
      _NpcFilter.dead => l10n.glossaryFilterDead,
    };

IconData _roleIcon(NpcGlossaryRole role) => switch (role) {
  NpcGlossaryRole.portrait => Icons.portrait_outlined,
  NpcGlossaryRole.trader => Icons.storefront_outlined,
  NpcGlossaryRole.teacher => Icons.school_outlined,
  NpcGlossaryRole.armorer => Icons.shield_outlined,
  NpcGlossaryRole.dead => Icons.dangerous_outlined,
  NpcGlossaryRole.hostile => Icons.gpp_bad_outlined,
};

String _roleLabel(AppLocalizations l10n, NpcGlossaryRole role) =>
    switch (role) {
      NpcGlossaryRole.portrait => l10n.glossarySegmentIntroduction,
      NpcGlossaryRole.trader => l10n.glossaryFilterTraders,
      NpcGlossaryRole.teacher => l10n.glossaryFilterTeachers,
      NpcGlossaryRole.armorer => l10n.glossaryFilterArmorers,
      NpcGlossaryRole.dead => l10n.glossaryFilterDead,
      NpcGlossaryRole.hostile => l10n.glossaryFilterHostile,
    };

String _segmentLabel(AppLocalizations l10n, _GlossarySegment segment) {
  final raw = segment.label.isEmpty ? segment.id : segment.label;
  final normalized = raw.replaceAll(' ', '_');
  final numberedRole = RegExp(
    r'^(Introduction|Teacher|Trader|Dealer|Armor|Armorer)_?(\d+)$',
    caseSensitive: false,
  ).firstMatch(normalized);
  if (numberedRole != null) {
    final base = switch (numberedRole.group(1)!.toLowerCase()) {
      'introduction' => l10n.glossarySegmentIntroduction,
      'teacher' => l10n.glossaryFilterTeachers,
      'trader' || 'dealer' => l10n.glossaryFilterTraders,
      _ => l10n.glossaryFilterArmorers,
    };
    return '$base ${numberedRole.group(2)}';
  }
  final exact = normalized.toLowerCase();
  if (exact == 'introduction') return l10n.glossarySegmentIntroduction;
  if (exact == 'trader' || exact == 'dealer') {
    return l10n.glossaryFilterTraders;
  }
  if (exact == 'teacher') return l10n.glossaryFilterTeachers;
  if (exact == 'armor' || exact == 'armorer') {
    return l10n.glossaryFilterArmorers;
  }
  if (segment.roles.length > 1 &&
      segment.roles.contains(NpcGlossaryRole.portrait)) {
    return segment.roles.map((role) => _roleLabel(l10n, role)).join(' · ');
  }
  if (raw.toLowerCase() == 'unlock') return l10n.glossarySegmentUnlock;
  final entryMatch = RegExp(
    r'^Entry(\d+)$',
    caseSensitive: false,
  ).firstMatch(raw);
  if (entryMatch != null) {
    return l10n.glossarySegmentEntry(int.parse(entryMatch.group(1)!));
  }
  return segment.usesNpcCatalogLabel
      ? l10n.glossaryCatalogSegmentLabel(segment.id, _humanize(raw))
      : _humanize(raw);
}

String _humanize(String value) {
  final spaced = value
      .replaceAll('_', ' ')
      .replaceAllMapped(
        RegExp(r'([a-z0-9])([A-Z])'),
        (match) => '${match.group(1)} ${match.group(2)}',
      )
      .trim();
  if (spaced.isEmpty) return value;
  return spaced[0].toUpperCase() + spaced.substring(1);
}
