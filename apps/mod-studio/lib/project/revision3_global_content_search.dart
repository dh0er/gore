import 'dart:async';

// ignore_for_file: prefer_initializing_formals

import 'package:flutter/foundation.dart';

import '../core/mod_ffi.dart';
import 'revision3_base_game_content_browser.dart';
import 'revision3_content_index.dart';

const revision3GlobalContentSearchResultLimit = 100;
const _scanYieldInterval = 128;
const _maxRetainedQueryRunes = 512;
const _maxRetainedErrorRunes = 1024;

typedef Revision3GlobalThisModContentLoader =
    Future<Revision3ContentIndex> Function();
typedef Revision3GlobalBaseGameContentLoader =
    Future<Revision3BaseGameContentCatalog> Function();
typedef Revision3GlobalInstalledContentLoader =
    Future<AuthoringRevision3DataAssetPackageIndexResult> Function();

enum Revision3GlobalContentSource { thisMod, baseGame, installed }

enum Revision3GlobalContentKind {
  thisModEntity,
  thisModAsset,
  baseNpc,
  baseQuest,
  experimentalBaseNpc,
  installedDataAsset,
}

enum Revision3GlobalContentReadiness {
  exactCurrent,
  exactCurrentWithProblems,
  offlineDraftRuntimeUnqualified,
  inspectOnlyRuntimeUnqualified,
  metadataOnlyRuntimeUnqualified,
}

enum Revision3GlobalContentActionKind {
  openThisModEntity,
  openThisModAsset,
  createBaseNpcDraft,
  createBaseQuestDraft,
  inspectInstalledDataAsset,
}

/// A same-source action identity. It never implies a cross-source reference or
/// dependency relationship.
final class Revision3GlobalContentAction {
  const Revision3GlobalContentAction._({
    required this.kind,
    required this.identity,
  });

  final Revision3GlobalContentActionKind kind;
  final String identity;

  static Revision3GlobalContentAction thisModEntity(String entityId) =>
      Revision3GlobalContentAction._(
        kind: Revision3GlobalContentActionKind.openThisModEntity,
        identity: entityId,
      );

  static Revision3GlobalContentAction thisModAsset(String sha256) =>
      Revision3GlobalContentAction._(
        kind: Revision3GlobalContentActionKind.openThisModAsset,
        identity: sha256,
      );

  static Revision3GlobalContentAction baseNpc(String catalogId) =>
      Revision3GlobalContentAction._(
        kind: Revision3GlobalContentActionKind.createBaseNpcDraft,
        identity: catalogId,
      );

  static Revision3GlobalContentAction baseQuest(String catalogId) =>
      Revision3GlobalContentAction._(
        kind: Revision3GlobalContentActionKind.createBaseQuestDraft,
        identity: catalogId,
      );

  static Revision3GlobalContentAction installed(String targetPath) =>
      Revision3GlobalContentAction._(
        kind: Revision3GlobalContentActionKind.inspectInstalledDataAsset,
        identity: targetPath,
      );
}

/// One bounded search row. [action] carries only authority already granted by
/// [source]; experimental Base-game NPC evidence deliberately has no action.
final class Revision3GlobalContentResult {
  const Revision3GlobalContentResult({
    required this.source,
    required this.kind,
    required this.readiness,
    required this.title,
    required this.subtitle,
    required this.action,
    this.entityKind,
  });

  final Revision3GlobalContentSource source;
  final Revision3GlobalContentKind kind;
  final Revision3GlobalContentReadiness readiness;
  final String title;
  final String subtitle;
  final Revision3GlobalContentAction? action;
  final Revision3ContentEntityKind? entityKind;
}

enum Revision3GlobalContentSourcePhase {
  idle,
  loading,
  complete,
  partial,
  error,
}

final class Revision3GlobalContentSourceState {
  const Revision3GlobalContentSourceState._({
    required this.phase,
    required this.results,
    required this.truncated,
    required this.error,
  });

  const Revision3GlobalContentSourceState.idle()
    : this._(
        phase: Revision3GlobalContentSourcePhase.idle,
        results: const <Revision3GlobalContentResult>[],
        truncated: false,
        error: null,
      );

  const Revision3GlobalContentSourceState.loading()
    : this._(
        phase: Revision3GlobalContentSourcePhase.loading,
        results: const <Revision3GlobalContentResult>[],
        truncated: false,
        error: null,
      );

  Revision3GlobalContentSourceState.results({
    required List<Revision3GlobalContentResult> results,
    required bool truncated,
    required bool sourcePartial,
  }) : this._(
         phase: truncated || sourcePartial
             ? Revision3GlobalContentSourcePhase.partial
             : Revision3GlobalContentSourcePhase.complete,
         results: List<Revision3GlobalContentResult>.unmodifiable(results),
         truncated: truncated,
         error: null,
       );

  Revision3GlobalContentSourceState.failure(Object failure)
    : this._(
        phase: Revision3GlobalContentSourcePhase.error,
        results: const <Revision3GlobalContentResult>[],
        truncated: false,
        error: _boundedRunes(failure.toString(), _maxRetainedErrorRunes),
      );

  final Revision3GlobalContentSourcePhase phase;
  final List<Revision3GlobalContentResult> results;
  final bool truncated;
  final String? error;
}

final class Revision3GlobalContentSearchSnapshot {
  const Revision3GlobalContentSearchSnapshot({
    required this.query,
    required this.thisMod,
    required this.baseGame,
    required this.installed,
  });

  const Revision3GlobalContentSearchSnapshot.idle()
    : this(
        query: '',
        thisMod: const Revision3GlobalContentSourceState.idle(),
        baseGame: const Revision3GlobalContentSourceState.idle(),
        installed: const Revision3GlobalContentSourceState.idle(),
      );

  final String query;
  final Revision3GlobalContentSourceState thisMod;
  final Revision3GlobalContentSourceState baseGame;
  final Revision3GlobalContentSourceState installed;

  Revision3GlobalContentSourceState stateFor(
    Revision3GlobalContentSource source,
  ) => switch (source) {
    Revision3GlobalContentSource.thisMod => thisMod,
    Revision3GlobalContentSource.baseGame => baseGame,
    Revision3GlobalContentSource.installed => installed,
  };

  Revision3GlobalContentSearchSnapshot replace(
    Revision3GlobalContentSource source,
    Revision3GlobalContentSourceState state,
  ) => Revision3GlobalContentSearchSnapshot(
    query: query,
    thisMod: source == Revision3GlobalContentSource.thisMod ? state : thisMod,
    baseGame: source == Revision3GlobalContentSource.baseGame
        ? state
        : baseGame,
    installed: source == Revision3GlobalContentSource.installed
        ? state
        : installed,
  );
}

/// Host-owned identities used to invalidate projections when the managed
/// project or an exact source snapshot changes.
final class Revision3GlobalContentSearchSourceIdentity {
  const Revision3GlobalContentSearchSourceIdentity({
    required this.project,
    required this.thisMod,
    required this.baseGame,
    required this.installed,
  });

  final String project;
  final String thisMod;
  final String baseGame;
  final String installed;
}

/// Coordinates three independent, query-triggered catalog reads.
///
/// Every search receives a new epoch. Late loads and chunked scans from older
/// epochs are ignored, so one source cannot overwrite a newer search.
final class Revision3GlobalContentSearchController extends ChangeNotifier {
  Revision3GlobalContentSearchController({
    required Revision3GlobalThisModContentLoader loadThisMod,
    required Revision3GlobalBaseGameContentLoader loadBaseGame,
    required Revision3GlobalInstalledContentLoader loadInstalled,
    Revision3GlobalContentSearchSourceIdentity? sourceIdentity,
  }) : _loadThisMod = loadThisMod,
       _loadBaseGame = loadBaseGame,
       _loadInstalled = loadInstalled,
       _sourceIdentity = sourceIdentity;

  Revision3GlobalThisModContentLoader _loadThisMod;
  Revision3GlobalBaseGameContentLoader _loadBaseGame;
  Revision3GlobalInstalledContentLoader _loadInstalled;

  var _epoch = 0;
  var _activeReadOperations = 0;
  var _disposed = false;
  Revision3GlobalContentSearchSourceIdentity? _sourceIdentity;
  Revision3GlobalContentSearchSnapshot _snapshot =
      const Revision3GlobalContentSearchSnapshot.idle();

  Revision3GlobalContentSearchSnapshot get snapshot => _snapshot;

  /// Whether a physical source-read operation is still outstanding.
  ///
  /// This deliberately outlives visible loading phases after [clear] or source
  /// invalidation. Native reads are not cancellable, so the UI must not launch
  /// another batch until every superseded Future has settled.
  bool get isLoading => _activeReadOperations != 0;

  /// Atomically replaces host closures before applying source invalidation.
  /// Closure identity itself is deliberately never treated as source identity.
  void updateSources({
    required Revision3GlobalThisModContentLoader loadThisMod,
    required Revision3GlobalBaseGameContentLoader loadBaseGame,
    required Revision3GlobalInstalledContentLoader loadInstalled,
    required Revision3GlobalContentSearchSourceIdentity sourceIdentity,
  }) {
    _loadThisMod = loadThisMod;
    _loadBaseGame = loadBaseGame;
    _loadInstalled = loadInstalled;
    updateSourceIdentity(sourceIdentity);
  }

  /// Invalidates catalog authority without comparing loader closures.
  ///
  /// A different project resets the query and every source. Within the same
  /// project only changed source projections are dropped. A revision-only
  /// change can retain an already completed Base-game projection. Any loading
  /// projection is dropped because advancing the epoch cancels its update.
  void updateSourceIdentity(
    Revision3GlobalContentSearchSourceIdentity identity,
  ) {
    final previous = _sourceIdentity;
    _sourceIdentity = identity;
    if (previous == null || previous.project != identity.project) {
      clear();
      return;
    }
    final thisModChanged = previous.thisMod != identity.thisMod;
    final baseGameChanged = previous.baseGame != identity.baseGame;
    final installedChanged = previous.installed != identity.installed;
    if (!thisModChanged && !baseGameChanged && !installedChanged) return;
    _epoch++;
    const idle = Revision3GlobalContentSourceState.idle();
    final base =
        baseGameChanged ||
            _snapshot.baseGame.phase ==
                Revision3GlobalContentSourcePhase.loading
        ? idle
        : _snapshot.baseGame;
    _snapshot = Revision3GlobalContentSearchSnapshot(
      query: _snapshot.query,
      thisMod:
          thisModChanged ||
              _snapshot.thisMod.phase ==
                  Revision3GlobalContentSourcePhase.loading
          ? idle
          : _snapshot.thisMod,
      baseGame: base,
      installed:
          installedChanged ||
              _snapshot.installed.phase ==
                  Revision3GlobalContentSourcePhase.loading
          ? idle
          : _snapshot.installed,
    );
    _notifyIfMounted();
  }

  Future<void> search(String rawQuery) async {
    final query = _boundedRunes(rawQuery.trim(), _maxRetainedQueryRunes);
    final epoch = ++_epoch;
    if (query.isEmpty) {
      _snapshot = const Revision3GlobalContentSearchSnapshot.idle();
      _notifyIfMounted();
      return;
    }

    _snapshot = Revision3GlobalContentSearchSnapshot(
      query: query,
      thisMod: const Revision3GlobalContentSourceState.loading(),
      baseGame: const Revision3GlobalContentSourceState.loading(),
      installed: const Revision3GlobalContentSourceState.loading(),
    );
    _activeReadOperations++;
    _notifyIfMounted();

    final foldedTerms = _foldedTerms(query);
    try {
      await Future.wait<void>(<Future<void>>[
        _searchThisMod(epoch, foldedTerms),
        _searchBaseGame(epoch, foldedTerms),
        _searchInstalled(epoch, foldedTerms),
      ]);
    } finally {
      _activeReadOperations--;
      _notifyIfMounted();
    }
  }

  /// Retries only one failed/idle source while retaining completed projections
  /// from the same query. A retry waits until no other source load is active so
  /// advancing the shared epoch cannot strand an in-flight projection.
  Future<void> retrySource(Revision3GlobalContentSource source) async {
    final query = _snapshot.query;
    if (query.isEmpty || isLoading) return;
    final epoch = ++_epoch;
    _activeReadOperations++;
    _snapshot = _snapshot.replace(
      source,
      const Revision3GlobalContentSourceState.loading(),
    );
    _notifyIfMounted();
    final foldedTerms = _foldedTerms(query);
    try {
      await switch (source) {
        Revision3GlobalContentSource.thisMod => _searchThisMod(
          epoch,
          foldedTerms,
        ),
        Revision3GlobalContentSource.baseGame => _searchBaseGame(
          epoch,
          foldedTerms,
        ),
        Revision3GlobalContentSource.installed => _searchInstalled(
          epoch,
          foldedTerms,
        ),
      };
    } finally {
      _activeReadOperations--;
      _notifyIfMounted();
    }
  }

  void clear() {
    _epoch++;
    _snapshot = const Revision3GlobalContentSearchSnapshot.idle();
    _notifyIfMounted();
  }

  Future<void> _searchThisMod(int epoch, List<String> terms) async {
    try {
      final index = await Future<Revision3ContentIndex>.sync(_loadThisMod);
      if (!_isCurrent(epoch)) return;
      final collector = _BoundedCollector();
      var scanned = 0;
      for (final entity in index.entities) {
        if (_matches(terms, <String>[
          entity.displayName,
          entity.id,
          entity.kind.displayName,
          entity.origin.label,
          ...entity.summary.searchTerms,
        ])) {
          if (!collector.add(
            Revision3GlobalContentResult(
              source: Revision3GlobalContentSource.thisMod,
              kind: Revision3GlobalContentKind.thisModEntity,
              readiness: entity.problemCount == 0
                  ? Revision3GlobalContentReadiness.exactCurrent
                  : Revision3GlobalContentReadiness.exactCurrentWithProblems,
              title: entity.displayName.isEmpty
                  ? entity.summary.primaryIdentity
                  : entity.displayName,
              subtitle: entity.id,
              action: Revision3GlobalContentAction.thisModEntity(entity.id),
              entityKind: entity.kind,
            ),
          )) {
            break;
          }
        }
        if (await _yieldAndCheck(++scanned, epoch)) return;
      }
      if (!collector.truncated) {
        for (final asset in index.assets) {
          if (_matches(terms, <String>[
            asset.sha256,
            asset.mediaType,
            asset.assetClass.displayName,
          ])) {
            if (!collector.add(
              Revision3GlobalContentResult(
                source: Revision3GlobalContentSource.thisMod,
                kind: Revision3GlobalContentKind.thisModAsset,
                readiness: Revision3GlobalContentReadiness.exactCurrent,
                title: asset.assetClass.displayName,
                subtitle: asset.sha256,
                action: Revision3GlobalContentAction.thisModAsset(asset.sha256),
              ),
            )) {
              break;
            }
          }
          if (await _yieldAndCheck(++scanned, epoch)) return;
        }
      }
      _publishResults(epoch, Revision3GlobalContentSource.thisMod, collector);
    } catch (error) {
      _publishFailure(epoch, Revision3GlobalContentSource.thisMod, error);
    }
  }

  Future<void> _searchBaseGame(int epoch, List<String> terms) async {
    try {
      final catalog = await Future<Revision3BaseGameContentCatalog>.sync(
        _loadBaseGame,
      );
      if (!_isCurrent(epoch)) return;
      final collector = _BoundedCollector();
      var scanned = 0;
      for (final choice in catalog.npcs.choices) {
        if (_matches(terms, <String>[choice.displayName, choice.catalogId])) {
          if (!collector.add(
            Revision3GlobalContentResult(
              source: Revision3GlobalContentSource.baseGame,
              kind: Revision3GlobalContentKind.baseNpc,
              readiness: Revision3GlobalContentReadiness
                  .offlineDraftRuntimeUnqualified,
              title: choice.displayName,
              subtitle: choice.catalogId,
              action: Revision3GlobalContentAction.baseNpc(choice.catalogId),
            ),
          )) {
            break;
          }
        }
        if (await _yieldAndCheck(++scanned, epoch)) return;
      }
      if (!collector.truncated) {
        for (final choice in catalog.quests.parents) {
          if (_matches(terms, <String>[
            choice.displayLabel,
            choice.catalogId,
          ])) {
            if (!collector.add(
              Revision3GlobalContentResult(
                source: Revision3GlobalContentSource.baseGame,
                kind: Revision3GlobalContentKind.baseQuest,
                readiness: Revision3GlobalContentReadiness
                    .offlineDraftRuntimeUnqualified,
                title: choice.displayLabel,
                subtitle: choice.catalogId,
                action: Revision3GlobalContentAction.baseQuest(
                  choice.catalogId,
                ),
              ),
            )) {
              break;
            }
          }
          if (await _yieldAndCheck(++scanned, epoch)) return;
        }
      }
      if (!collector.truncated) {
        final archetypes = catalog.npcs.archetypeIndex;
        if (archetypes != null) {
          for (final row in archetypes.rows) {
            if (!row.experimental ||
                !_matches(terms, <String>[
                  row.label,
                  row.spawnClass,
                  row.aiConfigClass,
                  row.characterDefinitionClass,
                  row.actorBlueprint,
                ])) {
              if (await _yieldAndCheck(++scanned, epoch)) return;
              continue;
            }
            if (!collector.add(
              Revision3GlobalContentResult(
                source: Revision3GlobalContentSource.baseGame,
                kind: Revision3GlobalContentKind.experimentalBaseNpc,
                readiness: Revision3GlobalContentReadiness
                    .inspectOnlyRuntimeUnqualified,
                title: row.label,
                subtitle: row.spawnClass,
                action: null,
              ),
            )) {
              break;
            }
            if (await _yieldAndCheck(++scanned, epoch)) return;
          }
        }
      }
      _publishResults(epoch, Revision3GlobalContentSource.baseGame, collector);
    } catch (error) {
      _publishFailure(epoch, Revision3GlobalContentSource.baseGame, error);
    }
  }

  Future<void> _searchInstalled(int epoch, List<String> terms) async {
    try {
      final snapshot =
          await Future<AuthoringRevision3DataAssetPackageIndexResult>.sync(
            _loadInstalled,
          );
      if (!_isCurrent(epoch)) return;
      final collector = _BoundedCollector();
      var scanned = 0;
      for (final candidate in snapshot.index.candidates) {
        if (_matches(terms, <String>[
          candidate.targetPath,
          candidate.packageIdHex,
        ])) {
          final segments = candidate.targetPath.split('/');
          if (!collector.add(
            Revision3GlobalContentResult(
              source: Revision3GlobalContentSource.installed,
              kind: Revision3GlobalContentKind.installedDataAsset,
              readiness: Revision3GlobalContentReadiness
                  .metadataOnlyRuntimeUnqualified,
              title: segments.isEmpty ? candidate.targetPath : segments.last,
              subtitle: candidate.targetPath,
              action: Revision3GlobalContentAction.installed(
                candidate.targetPath,
              ),
            ),
          )) {
            break;
          }
        }
        if (await _yieldAndCheck(++scanned, epoch)) return;
      }
      _publishResults(
        epoch,
        Revision3GlobalContentSource.installed,
        collector,
        sourcePartial:
            snapshot.index.status ==
            AuthoringRevision3DataAssetPackageIndexStatus.partialIndex,
      );
    } catch (error) {
      _publishFailure(epoch, Revision3GlobalContentSource.installed, error);
    }
  }

  Future<bool> _yieldAndCheck(int scanned, int epoch) async {
    if (scanned % _scanYieldInterval == 0) {
      await Future<void>.delayed(Duration.zero);
    }
    return !_isCurrent(epoch);
  }

  void _publishResults(
    int epoch,
    Revision3GlobalContentSource source,
    _BoundedCollector collector, {
    bool sourcePartial = false,
  }) {
    if (!_isCurrent(epoch)) return;
    _snapshot = _snapshot.replace(
      source,
      Revision3GlobalContentSourceState.results(
        results: collector.results,
        truncated: collector.truncated,
        sourcePartial: sourcePartial,
      ),
    );
    _notifyIfMounted();
  }

  void _publishFailure(
    int epoch,
    Revision3GlobalContentSource source,
    Object error,
  ) {
    if (!_isCurrent(epoch)) return;
    _snapshot = _snapshot.replace(
      source,
      Revision3GlobalContentSourceState.failure(error),
    );
    _notifyIfMounted();
  }

  bool _isCurrent(int epoch) => !_disposed && epoch == _epoch;

  void _notifyIfMounted() {
    if (!_disposed) notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    _epoch++;
    super.dispose();
  }
}

final class _BoundedCollector {
  final List<Revision3GlobalContentResult> results =
      <Revision3GlobalContentResult>[];
  var truncated = false;

  bool add(Revision3GlobalContentResult result) {
    if (results.length == revision3GlobalContentSearchResultLimit) {
      truncated = true;
      return false;
    }
    results.add(result);
    return true;
  }
}

bool _matches(List<String> terms, Iterable<String> values) {
  if (terms.isEmpty) return false;
  final folded = values.map(_fold).join('\u0000');
  return terms.every(folded.contains);
}

List<String> _foldedTerms(String query) => _fold(query)
    .split(RegExp(r'\s+'))
    .where((term) => term.isNotEmpty)
    .toList(growable: false);

/// Bounded, allocation-only case/accent fold for common Latin authoring text.
/// It intentionally does not claim full Unicode collation.
String _fold(String value) {
  final output = StringBuffer();
  for (final rune in value.toLowerCase().runes) {
    output.write(_latinFold[rune] ?? String.fromCharCode(rune));
  }
  return output.toString();
}

final Map<int, String> _latinFold = <int, String>{
  for (final rune in 'àáâãäåāăą'.runes) rune: 'a',
  for (final rune in 'çćĉċč'.runes) rune: 'c',
  for (final rune in 'ďđ'.runes) rune: 'd',
  for (final rune in 'èéêëēĕėęě'.runes) rune: 'e',
  for (final rune in 'ĝğġģ'.runes) rune: 'g',
  for (final rune in 'ĥħ'.runes) rune: 'h',
  for (final rune in 'ìíîïĩīĭįı'.runes) rune: 'i',
  for (final rune in 'ĵ'.runes) rune: 'j',
  for (final rune in 'ķ'.runes) rune: 'k',
  for (final rune in 'ĺļľŀł'.runes) rune: 'l',
  for (final rune in 'ñńņň'.runes) rune: 'n',
  for (final rune in 'òóôõöøōŏő'.runes) rune: 'o',
  for (final rune in 'ŕŗř'.runes) rune: 'r',
  for (final rune in 'śŝşš'.runes) rune: 's',
  for (final rune in 'ţťŧ'.runes) rune: 't',
  for (final rune in 'ùúûüũūŭůűų'.runes) rune: 'u',
  for (final rune in 'ŵ'.runes) rune: 'w',
  for (final rune in 'ýÿŷ'.runes) rune: 'y',
  for (final rune in 'źżž'.runes) rune: 'z',
  0x00e6: 'ae',
  0x0153: 'oe',
  0x00df: 'ss',
};

String _boundedRunes(String value, int limit) {
  if (value.length <= limit) return value;
  return String.fromCharCodes(value.runes.take(limit));
}
