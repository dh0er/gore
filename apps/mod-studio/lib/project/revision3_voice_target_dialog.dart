import 'package:flutter/material.dart';

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_voice_authoring.dart';

/// Author-facing copy for [Revision3VoiceTargetDialog].
@immutable
final class Revision3VoiceTargetDialogCopy {
  const Revision3VoiceTargetDialogCopy({
    required this.title,
    required this.fixedContextUnavailable,
    required this.requiresReopen,
    required this.catalogLoadFailed,
    required this.staleCheckpoint,
    required this.resolveFailed,
    required this.resolvingStatus,
    required this.refreshSlotsLabel,
    required this.refreshContextLabel,
    required this.closeLabel,
    required this.cancelLabel,
    required this.resolvingLabel,
    required this.resolveLabel,
    required this.lineSearchLabel,
    required this.lineSearchHint,
    required this.lineSearchHelp,
    required this.localeLabel,
    required this.localeHelp,
    required this.voiceLanguageTemplate,
    required this.unresolvedTitle,
    required this.unresolvedMessage,
    required this.ambiguousTitle,
    required this.ambiguousMessage,
    required this.resolvedTitle,
    required this.resolvedMessage,
    required this.savesEvidenceLabel,
    required this.doesNotDeployLabel,
    required this.boundaryMessage,
    required this.outcomeTitle,
    required this.outcomeMessage,
    required this.emptyTitle,
    required this.emptyMessage,
    required this.gameRootUnavailable,
    required this.storeGameAlias,
    required this.executableUnavailable,
    required this.executableMismatch,
    required this.archiveUnavailable,
    required this.archiveUnsafe,
    required this.archiveInvalid,
    required this.archiveLimit,
    required this.archiveChanged,
    required this.localeUnsupported,
    required this.memberIneligible,
    required this.collision,
    required this.slotIneligible,
  });

  static const english = Revision3VoiceTargetDialogCopy(
    title: 'Resolve installed Voice target',
    fixedContextUnavailable:
        'This Voice action no longer matches one intact existing Voice target in the exact current project. Close it and reopen Resolve target from the current workspace. No project, game, or save files were changed.',
    requiresReopen:
        'This project can no longer be verified as current. Close this window and reopen the managed project before resolving another Voice target.',
    catalogLoadFailed:
        'Existing Voice slots could not be read from the exact current project. No project, game, or save files were changed.',
    staleCheckpoint:
        'The managed project changed while this window was open. Close this resolver and open it again from the current project.',
    resolveFailed:
        'The installed Voice target could not be resolved. No bundle was built, nothing was deployed, and no game or save file was changed. Check the installation and try again.',
    resolvingStatus:
        'Checking the installed Voice archive and saving exact evidence...',
    refreshSlotsLabel: 'Refresh existing Voice slots',
    refreshContextLabel: 'Refresh Voice context',
    closeLabel: 'Close',
    cancelLabel: 'Cancel',
    resolvingLabel: 'Resolving target...',
    resolveLabel: 'Resolve installed target',
    lineSearchLabel: 'Dialog line with an existing Voice slot',
    lineSearchHint: 'Search by speaker or line name',
    lineSearchHelp:
        'Only intact lines that already own a safe Voice slot are shown.',
    localeLabel: 'Existing Voice-slot language',
    localeHelp:
        'Only languages with an intact existing slot can be resolved here.',
    voiceLanguageTemplate: 'Voice language: {locale}',
    unresolvedTitle: 'Current target: unresolved',
    unresolvedMessage:
        'No exact installed archive member is currently linked to this slot.',
    ambiguousTitle: 'Current target: ambiguous',
    ambiguousMessage:
        'Multiple installed archive members matched previously; no member was chosen implicitly.',
    resolvedTitle: 'Current target: resolved',
    resolvedMessage:
        'One exact installed archive member is currently sealed for this slot. Resolving again refreshes that evidence.',
    savesEvidenceLabel: 'Saves evidence to project',
    doesNotDeployLabel: 'Does not deploy',
    boundaryMessage:
        'This checks the installed Voice archive for one existing slot and saves only exact match evidence. It does not change the archive, build a mod, deploy, or touch a save.',
    outcomeTitle: 'No match is invented',
    outcomeMessage:
        'Zero, one, or multiple exact matches are saved honestly as unresolved, resolved, or ambiguous.',
    emptyTitle: 'No existing safe Voice slot is available',
    emptyMessage:
        'Add or repair a Voice slot in the managed project first, then reopen this resolver.',
    gameRootUnavailable:
        'The configured Gothic 1 Remake installation is unavailable. Check it in Settings, then try again.',
    storeGameAlias:
        'This project folder overlaps the configured game installation. Move the project outside the game folder before resolving Voice targets.',
    executableUnavailable:
        'The installed game executable could not be read. Finish any game update, check the configured installation, then try again.',
    executableMismatch:
        'The installed game executable no longer matches this project generation. Re-import or retarget the managed project before resolving Voice targets.',
    archiveUnavailable:
        'The installed Voice archive for this language is unavailable. Finish any game update, check the installation, then try again.',
    archiveUnsafe:
        'The installed Voice archive could not be opened safely. Repair or verify the game installation before trying again.',
    archiveInvalid:
        'The installed Voice archive is invalid or unsupported. Verify the game installation before trying again.',
    archiveLimit:
        'The installed Voice archive exceeds the supported safe inspection limits.',
    archiveChanged:
        'The Voice archive changed while it was being inspected. Finish the game update, then try again.',
    localeUnsupported:
        'No supported installed Voice archive is known for this language.',
    memberIneligible:
        'A matching archive entry exists but is not safe for an exact managed replacement.',
    collision:
        'This exact installed Voice target is already owned by another slot in the project.',
    slotIneligible:
        'This Voice slot is no longer eligible for target resolution. Refresh the project and choose it again.',
  );

  static const german = Revision3VoiceTargetDialogCopy(
    title: 'Installiertes Voice-Ziel auflösen',
    fixedContextUnavailable:
        'Diese Voice-Aktion entspricht im exakt aktuellen Projekt nicht mehr genau einem intakten vorhandenen Voice-Ziel. Schließe sie und öffne „Ziel auflösen“ im aktuellen Arbeitsbereich erneut. Es wurden keine Projekt-, Spiel- oder Speicherdateien geändert.',
    requiresReopen:
        'Dieses Projekt kann nicht mehr als aktuell bestätigt werden. Schließe dieses Fenster und öffne das verwaltete Projekt erneut, bevor du ein weiteres Voice-Ziel auflöst.',
    catalogLoadFailed:
        'Vorhandene Voice-Slots konnten nicht aus dem exakt aktuellen Projekt gelesen werden. Es wurden keine Projekt-, Spiel- oder Speicherdateien geändert.',
    staleCheckpoint:
        'Das verwaltete Projekt wurde geändert, während dieses Fenster geöffnet war. Schließe diesen Dialog und öffne ihn im aktuellen Projekt erneut.',
    resolveFailed:
        'Das installierte Voice-Ziel konnte nicht aufgelöst werden. Es wurde kein Bundle gebaut, nichts bereitgestellt und keine Spiel- oder Speicherdatei geändert. Prüfe die Installation und versuche es erneut.',
    resolvingStatus:
        'Das installierte Voice-Archiv wird geprüft und der exakte Nachweis gespeichert …',
    refreshSlotsLabel: 'Vorhandene Voice-Slots aktualisieren',
    refreshContextLabel: 'Voice-Kontext aktualisieren',
    closeLabel: 'Schließen',
    cancelLabel: 'Abbrechen',
    resolvingLabel: 'Ziel wird aufgelöst …',
    resolveLabel: 'Installiertes Ziel auflösen',
    lineSearchLabel: 'Dialogzeile mit vorhandenem Voice-Slot',
    lineSearchHint: 'Nach Sprecher oder Zeilenname suchen',
    lineSearchHelp:
        'Es werden nur intakte Zeilen angezeigt, die bereits einen sicheren Voice-Slot besitzen.',
    localeLabel: 'Sprache des vorhandenen Voice-Slots',
    localeHelp:
        'Hier können nur Sprachen mit einem intakten vorhandenen Slot aufgelöst werden.',
    voiceLanguageTemplate: 'Voice-Sprache: {locale}',
    unresolvedTitle: 'Aktuelles Ziel: nicht aufgelöst',
    unresolvedMessage:
        'Mit diesem Slot ist derzeit kein exakter Eintrag des installierten Archivs verknüpft.',
    ambiguousTitle: 'Aktuelles Ziel: mehrdeutig',
    ambiguousMessage:
        'Zuvor passten mehrere Einträge des installierten Archivs; es wurde nicht stillschweigend ein Eintrag gewählt.',
    resolvedTitle: 'Aktuelles Ziel: aufgelöst',
    resolvedMessage:
        'Genau ein Eintrag des installierten Archivs ist derzeit für diesen Slot versiegelt. Erneutes Auflösen aktualisiert diesen Nachweis.',
    savesEvidenceLabel: 'Speichert Nachweis im Projekt',
    doesNotDeployLabel: 'Keine Bereitstellung',
    boundaryMessage:
        'Diese Aktion prüft das installierte Voice-Archiv für einen vorhandenen Slot und speichert nur den exakten Treffer-Nachweis. Sie ändert das Archiv nicht, baut keine Mod, stellt nichts bereit und verändert keinen Spielstand.',
    outcomeTitle: 'Kein Treffer wird erfunden',
    outcomeMessage:
        'Null, ein oder mehrere exakte Treffer werden ehrlich als nicht aufgelöst, aufgelöst oder mehrdeutig gespeichert.',
    emptyTitle: 'Kein sicherer vorhandener Voice-Slot verfügbar',
    emptyMessage:
        'Füge im verwalteten Projekt zuerst einen Voice-Slot hinzu oder repariere ihn und öffne diesen Dialog dann erneut.',
    gameRootUnavailable:
        'Die konfigurierte Installation von Gothic 1 Remake ist nicht verfügbar. Prüfe sie in den Einstellungen und versuche es erneut.',
    storeGameAlias:
        'Dieser Projektordner überschneidet sich mit der konfigurierten Spielinstallation. Verschiebe das Projekt aus dem Spielordner, bevor du Voice-Ziele auflöst.',
    executableUnavailable:
        'Die installierte ausführbare Spieldatei konnte nicht gelesen werden. Schließe alle Spielupdates ab, prüfe die konfigurierte Installation und versuche es erneut.',
    executableMismatch:
        'Die installierte ausführbare Spieldatei entspricht nicht mehr dieser Projektgeneration. Importiere das verwaltete Projekt erneut oder richte es neu aus, bevor du Voice-Ziele auflöst.',
    archiveUnavailable:
        'Das installierte Voice-Archiv für diese Sprache ist nicht verfügbar. Schließe alle Spielupdates ab, prüfe die Installation und versuche es erneut.',
    archiveUnsafe:
        'Das installierte Voice-Archiv konnte nicht sicher geöffnet werden. Repariere oder überprüfe die Spielinstallation, bevor du es erneut versuchst.',
    archiveInvalid:
        'Das installierte Voice-Archiv ist ungültig oder wird nicht unterstützt. Überprüfe die Spielinstallation und versuche es erneut.',
    archiveLimit:
        'Das installierte Voice-Archiv überschreitet die unterstützten Grenzen für eine sichere Prüfung.',
    archiveChanged:
        'Das Voice-Archiv wurde während der Prüfung geändert. Schließe das Spielupdate ab und versuche es erneut.',
    localeUnsupported:
        'Für diese Sprache ist kein unterstütztes installiertes Voice-Archiv bekannt.',
    memberIneligible:
        'Es gibt einen passenden Archiveintrag, aber er eignet sich nicht für einen sicheren exakten Ersatz.',
    collision:
        'Dieses exakte installierte Voice-Ziel gehört im Projekt bereits zu einem anderen Slot.',
    slotIneligible:
        'Dieser Voice-Slot eignet sich nicht mehr für die Zielauflösung. Aktualisiere das Projekt und wähle ihn erneut aus.',
  );

  final String title;
  final String fixedContextUnavailable;
  final String requiresReopen;
  final String catalogLoadFailed;
  final String staleCheckpoint;
  final String resolveFailed;
  final String resolvingStatus;
  final String refreshSlotsLabel;
  final String refreshContextLabel;
  final String closeLabel;
  final String cancelLabel;
  final String resolvingLabel;
  final String resolveLabel;
  final String lineSearchLabel;
  final String lineSearchHint;
  final String lineSearchHelp;
  final String localeLabel;
  final String localeHelp;
  final String voiceLanguageTemplate;
  final String unresolvedTitle;
  final String unresolvedMessage;
  final String ambiguousTitle;
  final String ambiguousMessage;
  final String resolvedTitle;
  final String resolvedMessage;
  final String savesEvidenceLabel;
  final String doesNotDeployLabel;
  final String boundaryMessage;
  final String outcomeTitle;
  final String outcomeMessage;
  final String emptyTitle;
  final String emptyMessage;
  final String gameRootUnavailable;
  final String storeGameAlias;
  final String executableUnavailable;
  final String executableMismatch;
  final String archiveUnavailable;
  final String archiveUnsafe;
  final String archiveInvalid;
  final String archiveLimit;
  final String archiveChanged;
  final String localeUnsupported;
  final String memberIneligible;
  final String collision;
  final String slotIneligible;

  String voiceLanguage(String locale) =>
      voiceLanguageTemplate.replaceAll('{locale}', locale);

  String nativeError(String code) => switch (code) {
    'AUTHORING_REVISION3_VOICE_TARGET_GAME_ROOT_UNAVAILABLE' =>
      gameRootUnavailable,
    'AUTHORING_REVISION3_VOICE_TARGET_STORE_GAME_ALIAS' => storeGameAlias,
    'AUTHORING_REVISION3_VOICE_TARGET_EXECUTABLE_UNAVAILABLE' =>
      executableUnavailable,
    'AUTHORING_REVISION3_VOICE_TARGET_EXECUTABLE_MISMATCH' =>
      executableMismatch,
    'AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_UNAVAILABLE' =>
      archiveUnavailable,
    'AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_UNSAFE' => archiveUnsafe,
    'AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_INVALID' => archiveInvalid,
    'AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_LIMIT' => archiveLimit,
    'AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_CHANGED' => archiveChanged,
    'AUTHORING_REVISION3_VOICE_TARGET_LOCALE_UNSUPPORTED' => localeUnsupported,
    'AUTHORING_REVISION3_VOICE_TARGET_MEMBER_INELIGIBLE' => memberIneligible,
    'AUTHORING_REVISION3_VOICE_TARGET_COLLISION' => collision,
    'AUTHORING_REVISION3_VOICE_TARGET_LOC_ID_INVALID' ||
    'AUTHORING_REVISION3_VOICE_TARGET_INTENT_INVALID' ||
    'AUTHORING_REVISION3_VOICE_TARGET_REQUEST_INVALID' => slotIneligible,
    _ => resolveFailed,
  };
}

/// Normal-mode workflow for refreshing the installed-archive evidence of one
/// existing, structurally safe managed-R3 Voice slot.
///
/// This dialog cannot create slots, edit technical identities, build, deploy,
/// modify game files, or touch a save. The selected line and locale always
/// originate from one exact content-index checkpoint.
class Revision3VoiceTargetDialog extends StatefulWidget {
  const Revision3VoiceTargetDialog({
    required this.service,
    required this.copy,
    this.initialLineId,
    this.initialLocale,
    this.fixedContext = false,
    super.key,
  });

  final Revision3VoiceTargetAuthoringService service;
  final String? initialLineId;
  final String? initialLocale;

  /// Keeps an in-workspace line/locale handoff fixed. The freshly loaded
  /// catalog must still prove the exact existing slot targetable.
  final bool fixedContext;
  final Revision3VoiceTargetDialogCopy copy;

  @override
  State<Revision3VoiceTargetDialog> createState() =>
      _Revision3VoiceTargetDialogState();
}

class _Revision3VoiceTargetDialogState
    extends State<Revision3VoiceTargetDialog> {
  Revision3VoiceCatalog? _catalog;
  List<Revision3VoiceDialogLineChoice> _lines = const [];
  String? _lineId;
  String? _locale;
  bool _loading = true;
  bool _resolving = false;
  bool _publicationStarted = false;
  bool _requiresReopen = false;
  bool _staleCheckpoint = false;
  bool _fixedContextInvalid = false;
  String? _error;
  int _loadGeneration = 0;
  int _catalogEpoch = 0;
  bool _initialSelectionConsumed = false;

  @override
  void initState() {
    super.initState();
    _loadCatalog();
  }

  @override
  void didUpdateWidget(covariant Revision3VoiceTargetDialog oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.service, widget.service) ||
        oldWidget.fixedContext != widget.fixedContext ||
        oldWidget.initialLineId != widget.initialLineId ||
        oldWidget.initialLocale != widget.initialLocale) {
      _initialSelectionConsumed = false;
      _loadCatalog(clear: true);
    }
  }

  @override
  void dispose() {
    _loadGeneration += 1;
    super.dispose();
  }

  Future<void> _loadCatalog({bool clear = false}) async {
    final generation = ++_loadGeneration;
    setState(() {
      _loading = true;
      _error = null;
      if (clear) {
        _catalog = null;
        _lines = const [];
        _lineId = null;
        _locale = null;
        _fixedContextInvalid = false;
      }
    });
    try {
      final catalog = await widget.service.loadCatalog();
      if (!mounted || generation != _loadGeneration) return;
      final lines = catalog.lines
          .where((line) => _safeLocales(line).isNotEmpty)
          .toList(growable: false);
      setState(() {
        _catalog = catalog;
        _lines = List.unmodifiable(lines);
        _catalogEpoch += 1;
        if (widget.fixedContext) {
          final requestedLine = _lineFrom(lines, widget.initialLineId);
          final requestedLocale = widget.initialLocale;
          if (requestedLine != null &&
              requestedLocale != null &&
              _safeLocales(requestedLine).contains(requestedLocale)) {
            _lineId = requestedLine.lineId;
            _locale = requestedLocale;
            _fixedContextInvalid = false;
          } else {
            _lineId = null;
            _locale = null;
            _fixedContextInvalid = true;
            _error = widget.copy.fixedContextUnavailable;
          }
        } else {
          final initial = _initialSelectionConsumed
              ? null
              : _lineFrom(lines, widget.initialLineId);
          _initialSelectionConsumed = true;
          final selected = initial ?? _lineFrom(lines, _lineId);
          if (selected == null) {
            _lineId = null;
            _locale = null;
          } else {
            _lineId = selected.lineId;
            final locales = _safeLocales(selected);
            final requestedLocale = initial == null
                ? _locale
                : widget.initialLocale;
            _locale = locales.contains(requestedLocale)
                ? requestedLocale
                : locales.first;
          }
        }
        _loading = false;
      });
    } on Revision3VoiceTargetRequiresReopenException {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _catalog = null;
        _lines = const [];
        _lineId = null;
        _locale = null;
        _requiresReopen = true;
        _error = widget.copy.requiresReopen;
      });
    } catch (_) {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _catalog = null;
        _lines = const [];
        _lineId = null;
        _locale = null;
        _error = widget.copy.catalogLoadFailed;
      });
    }
  }

  Revision3VoiceDialogLineChoice? get _selectedLine =>
      _lineFrom(_lines, _lineId);

  Revision3VoiceExistingSlotSummary? get _selectedSummary {
    final line = _selectedLine;
    final locale = _locale;
    return line == null || locale == null
        ? null
        : line.slotSummaryForLocale(locale);
  }

  bool get _fixedContextIsCurrent {
    if (!widget.fixedContext) return true;
    final line = _selectedLine;
    final locale = _locale;
    return !_fixedContextInvalid &&
        line != null &&
        line.lineId == widget.initialLineId &&
        locale != null &&
        locale == widget.initialLocale &&
        _safeLocales(line).contains(locale);
  }

  void _selectLine(Revision3VoiceDialogLineChoice line) {
    final locales = _safeLocales(line);
    if (locales.isEmpty) return;
    setState(() {
      _lineId = line.lineId;
      _locale = locales.first;
      _error = null;
    });
  }

  void _clearChangedLineSearch(String value) {
    final selected = _selectedLine;
    if (selected != null && value != selected.displayLabel) {
      setState(() {
        _lineId = null;
        _locale = null;
      });
    }
  }

  Future<void> _resolve() async {
    final catalog = _catalog;
    final line = _selectedLine;
    final locale = _locale;
    if (_resolving ||
        _requiresReopen ||
        _staleCheckpoint ||
        catalog == null ||
        line == null ||
        locale == null ||
        !_fixedContextIsCurrent ||
        !_safeLocales(line).contains(locale)) {
      return;
    }
    setState(() {
      _resolving = true;
      _publicationStarted = true;
      _error = null;
    });
    var completed = false;
    try {
      final publication = await widget.service.resolve(
        checkpoint: catalog,
        lineId: line.lineId,
        locale: locale,
      );
      if (!mounted) return;
      completed = true;
      Navigator.of(context).pop(publication);
    } on Revision3VoiceTargetRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _requiresReopen = true;
        _error = widget.copy.requiresReopen;
      });
    } on Revision3VoiceTargetStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _staleCheckpoint = true;
        _error = widget.copy.staleCheckpoint;
      });
    } on ModFfiException catch (error) {
      if (!mounted) return;
      setState(() => _error = widget.copy.nativeError(error.code));
    } catch (_) {
      if (!mounted) return;
      setState(() => _error = widget.copy.resolveFailed);
    } finally {
      if (mounted && !completed) {
        setState(() {
          _resolving = false;
          _publicationStarted = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final blocked = _requiresReopen || _staleCheckpoint;
    final canResolve =
        !_loading &&
        !_resolving &&
        !blocked &&
        _fixedContextIsCurrent &&
        _catalog != null &&
        _selectedLine != null &&
        _locale != null;
    return PopScope(
      canPop: !_publicationStarted,
      child: AlertDialog(
        key: const Key('revision3-voice-target-dialog'),
        title: Row(
          children: [
            const Icon(Icons.manage_search_outlined),
            const SizedBox(width: 10),
            Expanded(child: Text(widget.copy.title)),
          ],
        ),
        content: SizedBox(
          width: 680,
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 640),
            child: SingleChildScrollView(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  _TargetBoundaryBanner(copy: widget.copy),
                  const SizedBox(height: 16),
                  if (_resolving) ...[
                    _TargetLiveStatus(
                      key: const Key('revision3-voice-target-resolving'),
                      message: widget.copy.resolvingStatus,
                    ),
                    const SizedBox(height: 16),
                  ],
                  if (_error case final message?) ...[
                    _TargetMessage(
                      key: const Key('revision3-voice-target-error'),
                      message: message,
                    ),
                    const SizedBox(height: 16),
                  ],
                  if (_loading)
                    const Padding(
                      padding: EdgeInsets.symmetric(vertical: 40),
                      child: Center(
                        child: CircularProgressIndicator(
                          key: Key('revision3-voice-target-loading'),
                        ),
                      ),
                    )
                  else if (_catalog == null)
                    Center(
                      child: OutlinedButton.icon(
                        key: const Key('revision3-voice-target-retry'),
                        onPressed: blocked ? null : _loadCatalog,
                        icon: const Icon(Icons.refresh),
                        label: Text(widget.copy.refreshSlotsLabel),
                      ),
                    )
                  else if (_fixedContextInvalid)
                    Center(
                      child: OutlinedButton.icon(
                        key: const Key(
                          'revision3-voice-target-fixed-context-retry',
                        ),
                        onPressed: blocked ? null : _loadCatalog,
                        icon: const Icon(Icons.refresh),
                        label: Text(widget.copy.refreshContextLabel),
                      ),
                    )
                  else if (_lines.isEmpty)
                    _NoSafeVoiceSlots(copy: widget.copy)
                  else
                    _buildSelection(enabled: !_resolving && !blocked),
                ],
              ),
            ),
          ),
        ),
        actions: [
          TextButton(
            key: const Key('revision3-voice-target-cancel'),
            onPressed: _publicationStarted
                ? null
                : () => Navigator.of(context).pop(),
            child: Text(
              blocked ? widget.copy.closeLabel : widget.copy.cancelLabel,
            ),
          ),
          FilledButton.icon(
            key: const Key('revision3-voice-target-submit'),
            onPressed: canResolve ? _resolve : null,
            icon: _resolving
                ? const SizedBox.square(
                    dimension: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.travel_explore_outlined),
            label: Text(
              _resolving
                  ? widget.copy.resolvingLabel
                  : widget.copy.resolveLabel,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildSelection({required bool enabled}) {
    final line = _selectedLine;
    final locales = line == null ? const <String>[] : _safeLocales(line);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (widget.fixedContext)
          _TargetFixedContextBreadcrumb(
            lineLabel: line!.displayLabel,
            locale: _locale!,
            copy: widget.copy,
          )
        else
          RawAutocomplete<Revision3VoiceDialogLineChoice>(
            key: ValueKey('revision3-voice-target-line-$_catalogEpoch'),
            initialValue: TextEditingValue(text: line?.displayLabel ?? ''),
            displayStringForOption: (option) => option.displayLabel,
            optionsBuilder: (value) {
              final query = value.text.trim();
              if (query.isEmpty) {
                return const <Revision3VoiceDialogLineChoice>[];
              }
              return _lines.where((option) => option.matches(query)).take(50);
            },
            onSelected: _selectLine,
            fieldViewBuilder:
                (context, controller, focusNode, onFieldSubmitted) =>
                    TextFormField(
                      key: const Key('revision3-voice-target-line-search'),
                      controller: controller,
                      focusNode: focusNode,
                      enabled: enabled,
                      decoration: InputDecoration(
                        labelText: widget.copy.lineSearchLabel,
                        hintText: widget.copy.lineSearchHint,
                        helperText: widget.copy.lineSearchHelp,
                        border: const OutlineInputBorder(),
                      ),
                      onChanged: _clearChangedLineSearch,
                      onFieldSubmitted: (_) => onFieldSubmitted(),
                    ),
            optionsViewBuilder: (context, onSelected, options) {
              final bounded = options.toList(growable: false);
              return Align(
                alignment: Alignment.topLeft,
                child: Material(
                  elevation: 6,
                  clipBehavior: Clip.antiAlias,
                  borderRadius: BorderRadius.circular(8),
                  child: ConstrainedBox(
                    constraints: const BoxConstraints(
                      maxWidth: 640,
                      maxHeight: 300,
                    ),
                    child: ListView.builder(
                      key: const Key('revision3-voice-target-line-results'),
                      padding: EdgeInsets.zero,
                      shrinkWrap: true,
                      itemCount: bounded.length,
                      itemBuilder: (context, index) {
                        final option = bounded[index];
                        return ListTile(
                          title: Text(option.displayLabel),
                          onTap: () => onSelected(option),
                        );
                      },
                    ),
                  ),
                ),
              );
            },
          ),
        const SizedBox(height: 14),
        if (!widget.fixedContext)
          DropdownButtonFormField<String>(
            key: ValueKey(
              'revision3-voice-target-locale-${line?.lineId ?? 'none'}',
            ),
            initialValue: locales.contains(_locale) ? _locale : null,
            decoration: InputDecoration(
              labelText: widget.copy.localeLabel,
              helperText: widget.copy.localeHelp,
              border: const OutlineInputBorder(),
            ),
            items: [
              for (final locale in locales)
                DropdownMenuItem(value: locale, child: Text(locale)),
            ],
            onChanged: enabled && line != null
                ? (value) => setState(() {
                    _locale = value;
                    _error = null;
                  })
                : null,
          ),
        if (_selectedSummary case final summary?) ...[
          const SizedBox(height: 14),
          _CurrentTargetState(summary: summary, copy: widget.copy),
        ],
        const SizedBox(height: 14),
        _TargetOutcomeNotice(copy: widget.copy),
      ],
    );
  }
}

class _TargetFixedContextBreadcrumb extends StatelessWidget {
  const _TargetFixedContextBreadcrumb({
    required this.lineLabel,
    required this.locale,
    required this.copy,
  });

  final String lineLabel;
  final String locale;
  final Revision3VoiceTargetDialogCopy copy;

  @override
  Widget build(BuildContext context) => Container(
    key: const Key('revision3-voice-target-fixed-context'),
    padding: const EdgeInsets.all(12),
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.surfaceContainerHighest,
      borderRadius: BorderRadius.circular(8),
    ),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Icon(Icons.subdirectory_arrow_right, size: 20),
        const SizedBox(width: 10),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(lineLabel, style: Theme.of(context).textTheme.titleSmall),
              const SizedBox(height: 3),
              Text(copy.voiceLanguage(locale)),
            ],
          ),
        ),
      ],
    ),
  );
}

List<String> _safeLocales(Revision3VoiceDialogLineChoice line) {
  final locales =
      line.existingSlotLocales
          .where(
            (locale) =>
                line.isLocaleTargetable(locale) &&
                line.slotSummaryForLocale(locale) != null,
          )
          .toList(growable: false)
        ..sort();
  return locales;
}

Revision3VoiceDialogLineChoice? _lineFrom(
  List<Revision3VoiceDialogLineChoice> lines,
  String? id,
) {
  if (id == null) return null;
  for (final line in lines) {
    if (line.lineId == id) return line;
  }
  return null;
}

class _CurrentTargetState extends StatelessWidget {
  const _CurrentTargetState({required this.summary, required this.copy});

  final Revision3VoiceExistingSlotSummary summary;
  final Revision3VoiceTargetDialogCopy copy;

  @override
  Widget build(BuildContext context) {
    final (icon, title, message) = switch (summary.targetResolution) {
      Revision3ContentVoiceTargetResolution.unresolved => (
        Icons.link_off_outlined,
        copy.unresolvedTitle,
        copy.unresolvedMessage,
      ),
      Revision3ContentVoiceTargetResolution.ambiguous => (
        Icons.call_split_outlined,
        copy.ambiguousTitle,
        copy.ambiguousMessage,
      ),
      Revision3ContentVoiceTargetResolution.resolved => (
        Icons.verified_outlined,
        copy.resolvedTitle,
        copy.resolvedMessage,
      ),
    };
    return Container(
      key: const Key('revision3-voice-target-current-state'),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(icon, size: 20),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title, style: Theme.of(context).textTheme.titleSmall),
                const SizedBox(height: 3),
                Text(message),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _TargetBoundaryBanner extends StatelessWidget {
  const _TargetBoundaryBanner({required this.copy});

  final Revision3VoiceTargetDialogCopy copy;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      key: const Key('revision3-voice-target-boundary'),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: scheme.secondaryContainer,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              Chip(label: Text(copy.savesEvidenceLabel)),
              Chip(label: Text(copy.doesNotDeployLabel)),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            copy.boundaryMessage,
            style: TextStyle(color: scheme.onSecondaryContainer),
          ),
        ],
      ),
    );
  }
}

class _TargetOutcomeNotice extends StatelessWidget {
  const _TargetOutcomeNotice({required this.copy});

  final Revision3VoiceTargetDialogCopy copy;

  @override
  Widget build(BuildContext context) => ListTile(
    key: const Key('revision3-voice-target-outcome-notice'),
    contentPadding: EdgeInsets.zero,
    leading: const Icon(Icons.rule_folder_outlined),
    title: Text(copy.outcomeTitle),
    subtitle: Text(copy.outcomeMessage),
  );
}

class _NoSafeVoiceSlots extends StatelessWidget {
  const _NoSafeVoiceSlots({required this.copy});

  final Revision3VoiceTargetDialogCopy copy;

  @override
  Widget build(BuildContext context) => ListTile(
    key: const Key('revision3-voice-target-empty'),
    contentPadding: const EdgeInsets.symmetric(vertical: 24),
    leading: const Icon(Icons.info_outline),
    title: Text(copy.emptyTitle),
    subtitle: Text(copy.emptyMessage),
  );
}

class _TargetLiveStatus extends StatelessWidget {
  const _TargetLiveStatus({required this.message, super.key});

  final String message;

  @override
  Widget build(BuildContext context) => Semantics(
    liveRegion: true,
    child: Row(
      children: [
        const SizedBox.square(
          dimension: 18,
          child: CircularProgressIndicator(strokeWidth: 2),
        ),
        const SizedBox(width: 10),
        Expanded(child: Text(message)),
      ],
    ),
  );
}

class _TargetMessage extends StatelessWidget {
  const _TargetMessage({required this.message, super.key});

  final String message;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Semantics(
      liveRegion: true,
      child: Container(
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: scheme.errorContainer,
          borderRadius: BorderRadius.circular(8),
        ),
        child: Text(message, style: TextStyle(color: scheme.onErrorContainer)),
      ),
    );
  }
}
