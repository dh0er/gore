import 'dart:async';
import 'dart:math' as math;

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'revision3_voice_authoring.dart';
import 'revision3_voice_folder_authoring.dart';

typedef Revision3VoiceFolderDirectoryPicker = Future<String?> Function();

@immutable
final class Revision3VoiceFolderImportDialogCopy {
  const Revision3VoiceFolderImportDialogCopy.english() : _german = false;

  const Revision3VoiceFolderImportDialogCopy.german() : _german = true;

  final bool _german;

  String _text(String english, String german) => _german ? german : english;

  String get title =>
      _text('Import Voice recordings', 'Voice-Aufnahmen importieren');
  String get chooseRecordings =>
      _text('Choose recordings', 'Aufnahmen auswählen');
  String get reviewChanges => _text('Review changes', 'Änderungen prüfen');
  String get result => _text('Result', 'Ergebnis');
  String get boundary => _text(
    'Review every recording before one all-or-nothing project save. No game or save files are changed.',
    'Prüfe jede Aufnahme vor einem gemeinsamen, vollständigen Speichern im Projekt. Spiel- und Spielstanddateien werden nicht geändert.',
  );
  String get projectOnly =>
      _text('Saved to this project only', 'Nur in diesem Projekt gespeichert');
  String get recordedOnly => _text(
    'Every imported take starts as Recorded and is not selected.',
    'Jeder importierte Take beginnt als „Aufgenommen“ und wird nicht ausgewählt.',
  );
  String get recordedStatus => _text('Recorded', 'Aufgenommen');
  String get unchangedBoundary => _text(
    'Dialog text, current take selection, Voice target, build output, game files, and saves stay unchanged.',
    'Dialogtext, aktuelle Take-Auswahl, Voice-Ziel, Build-Ausgabe, Spieldateien und Spielstände bleiben unverändert.',
  );
  String get folderLabel => _text('Recording folder', 'Aufnahmeordner');
  String get noFolder => _text('No folder selected', 'Kein Ordner ausgewählt');
  String get folderSelected =>
      _text('Recording folder selected', 'Aufnahmeordner ausgewählt');
  String get chooseFolder => _text('Choose folder…', 'Ordner auswählen…');
  String get changeFolder => _text('Change folder…', 'Ordner ändern…');
  String get pickerFailed => _text(
    'The folder picker could not be opened. Choose the folder again.',
    'Die Ordnerauswahl konnte nicht geöffnet werden. Wähle den Ordner erneut aus.',
  );
  String get localeLabel => _text('Voice language', 'Voice-Sprache');
  String get localeHint => 'de';
  String get localeHelper => _text(
    'Use one language code for this folder, for example de or en-US.',
    'Verwende für diesen Ordner einen Sprachcode, zum Beispiel de oder en-US.',
  );
  String get localeInvalid => _text(
    'Enter a canonical language code such as de or en-US.',
    'Gib einen kanonischen Sprachcode wie de oder en-US ein.',
  );
  String get reviewFolder => _text('Review folder', 'Ordnerinhalt prüfen');
  String get reviewing => _text(
    'Scanning and reviewing recordings',
    'Aufnahmen werden gesucht und geprüft',
  );
  String get reviewDescription => _text(
    'Checking friendly line matches, audio format, and the exact current project. Nothing is being saved yet.',
    'Freundliche Zeilenzuordnung, Audioformat und das exakt aktuelle Projekt werden geprüft. Es wird noch nichts gespeichert.',
  );
  String get cancelReview => _text('Cancel review', 'Prüfung abbrechen');
  String get reviewAgain => _text('Review again', 'Erneut prüfen');
  String get close => _text('Close', 'Schließen');
  String get cancel => _text('Cancel', 'Abbrechen');
  String get backToList =>
      _text('Back to recordings', 'Zurück zu den Aufnahmen');
  String get searchLabel => _text('Search recordings', 'Aufnahmen durchsuchen');
  String get filterAll => _text('All', 'Alle');
  String get filterReady => _text('Ready', 'Bereit');
  String get filterExisting => _text('Already present', 'Bereits vorhanden');
  String get filterBlocked => _text('Blocked', 'Blockiert');
  String get noRows => _text(
    'No recordings match this view.',
    'Keine Aufnahmen entsprechen dieser Ansicht.',
  );
  String get noReadyRows => _text(
    'No new recording is ready to import. Existing project recordings remain unchanged.',
    'Keine neue Aufnahme ist zum Import bereit. Vorhandene Projektaufnahmen bleiben unverändert.',
  );
  String summary(
    int ready,
    int alreadyPresent,
    int blocked,
    int ogg,
    int ignored,
  ) => _german
      ? '$ready neu bereit · $alreadyPresent bereits vorhanden · $blocked blockiert · $ogg Ogg insgesamt · $ignored weitere Einträge ignoriert'
      : '$ready new ready · $alreadyPresent already present · $blocked blocked · $ogg Ogg total · $ignored other entries ignored';
  String allOrNothing(int count, int alreadyPresent) => _german
      ? 'Alle $count neuen Aufnahmen werden gemeinsam gespeichert; $alreadyPresent bereits vorhandene bleiben unverändert. Schlägt eine fehl, wird keine hinzugefügt.'
      : 'All $count new recordings are saved together; $alreadyPresent already-present recordings stay unchanged. If one fails, none is added.';
  String blockedImport(int count) => _german
      ? 'Nichts kann importiert werden, solange $count Ogg-Aufnahme${count == 1 ? '' : 'n'} blockiert ${count == 1 ? 'ist' : 'sind'}. Es gibt keine Teilimport-Option.'
      : 'Nothing can be imported while $count Ogg recording${count == 1 ? '' : 's'} ${count == 1 ? 'is' : 'are'} blocked. There is no partial-import option.';
  String importRecordings(int count) => _german
      ? '$count Aufnahme${count == 1 ? '' : 'n'} importieren'
      : 'Import $count recording${count == 1 ? '' : 's'}';
  String get saving =>
      _text('Saving all recordings', 'Alle Aufnahmen werden gespeichert');
  String get savingDescription => _text(
    'The complete included set is being published as one project change and then reopened. This cannot be interrupted safely.',
    'Die vollständige enthaltene Auswahl wird als eine Projektänderung gespeichert und anschließend erneut geöffnet. Dies kann nicht sicher unterbrochen werden.',
  );
  String get publicationLocked => _text(
    'Saving cannot be interrupted safely. Please wait.',
    'Das Speichern kann nicht sicher unterbrochen werden. Bitte warte.',
  );
  String success(int count) => _german
      ? '$count Aufnahme${count == 1 ? '' : 'n'} wurde${count == 1 ? '' : 'n'} in einer Projektänderung importiert.'
      : '$count recording${count == 1 ? '' : 's'} imported in one project change.';
  String get successBoundary => _text(
    'Existing recordings were preserved. Nothing was built, installed, or written to the game or a save.',
    'Vorhandene Aufnahmen blieben erhalten. Es wurde nichts gebaut, installiert oder in das Spiel beziehungsweise einen Spielstand geschrieben.',
  );
  String get planFailed => _text(
    'The folder could not be reviewed safely. No project, game, or save files were changed.',
    'Der Ordner konnte nicht sicher geprüft werden. Projekt-, Spiel- und Spielstanddateien wurden nicht geändert.',
  );
  String get applyFailed => _text(
    'Nothing was imported. Review the marked recordings and try again from the exact current project.',
    'Es wurde nichts importiert. Prüfe die markierten Aufnahmen und versuche es erneut aus dem exakt aktuellen Projekt.',
  );
  String get stale => _text(
    'The project changed after this review. Review the folder again before saving.',
    'Das Projekt wurde nach dieser Prüfung geändert. Prüfe den Ordner vor dem Speichern erneut.',
  );
  String get requiresReopen => _text(
    'The current project can no longer be verified safely. Close this window and reopen the managed project before importing again.',
    'Das aktuelle Projekt kann nicht mehr sicher bestätigt werden. Schließe dieses Fenster und öffne das verwaltete Projekt erneut, bevor du erneut importierst.',
  );
  String get uncertain => _text(
    'The import may have been saved, but the current project could not be confirmed. Reopen the project and do not retry this import.',
    'Der Import wurde möglicherweise gespeichert, das aktuelle Projekt konnte aber nicht bestätigt werden. Öffne das Projekt erneut und wiederhole diesen Import nicht.',
  );
  String get selectRecording =>
      _text('Select a recording to review', 'Wähle eine Aufnahme zur Prüfung');
  String get dialogLine => _text('Dialog line', 'Dialogzeile');
  String get speaker => _text('Speaker', 'Sprecher');
  String get recording => _text('Recording', 'Aufnahme');
  String recordingNumber(int number) =>
      _german ? 'Aufnahme $number' : 'Recording $number';
  String dialogLineNumber(int number) =>
      _german ? 'Dialogzeile $number' : 'Dialog line $number';
  String get audio => _text('Audio', 'Audio');
  String get plannedTake => _text('New take', 'Neuer Take');
  String get semanticChange => _text('Project change', 'Projektänderung');
  String takeCountChange(int before, int after) =>
      _german ? '$before → $after Takes' : '$before → $after takes';
  String get selectionUnchanged => _text(
    'Current take selection stays unchanged',
    'Aktuelle Take-Auswahl bleibt unverändert',
  );
  String get targetUnchanged =>
      _text('Voice target stays unchanged', 'Voice-Ziel bleibt unverändert');
  String targetState(Revision3VoiceFolderTargetState state) => switch (state) {
    Revision3VoiceFolderTargetState.unresolved => _text(
      'Unresolved',
      'Nicht aufgelöst',
    ),
    Revision3VoiceFolderTargetState.ambiguous => _text(
      'Ambiguous',
      'Mehrdeutig',
    ),
    Revision3VoiceFolderTargetState.resolved => _text('Resolved', 'Aufgelöst'),
  };
  String codec(Revision3VoiceFolderCodec codec) => switch (codec) {
    Revision3VoiceFolderCodec.vorbis => 'Vorbis Ogg',
    Revision3VoiceFolderCodec.opus => 'Opus Ogg',
  };
  String get opusWarning => _text(
    'This recording can be kept in the project, but the current Voice bundle cannot build Opus.',
    'Diese Aufnahme kann im Projekt gespeichert werden, der aktuelle Voice-Build unterstützt Opus jedoch noch nicht.',
  );
  String status(Revision3VoiceFolderRowStatus status) => switch (status) {
    Revision3VoiceFolderRowStatus.ready => _text('Ready', 'Bereit'),
    Revision3VoiceFolderRowStatus.alreadyPresent => _text(
      'Already in project',
      'Bereits im Projekt',
    ),
    Revision3VoiceFolderRowStatus.unmatched => _text(
      'No matching dialog line',
      'Keine passende Dialogzeile',
    ),
    Revision3VoiceFolderRowStatus.ambiguous => _text(
      'Several dialog lines match',
      'Mehrere Dialogzeilen passen',
    ),
    Revision3VoiceFolderRowStatus.invalid => _text(
      'Invalid recording',
      'Ungültige Aufnahme',
    ),
  };
  String statusDescription(
    Revision3VoiceFolderRowStatus status,
  ) => switch (status) {
    Revision3VoiceFolderRowStatus.ready => _text(
      'This recording is included in the exact reviewed import.',
      'Diese Aufnahme ist im exakt geprüften Import enthalten.',
    ),
    Revision3VoiceFolderRowStatus.alreadyPresent => _text(
      'The same recording is already retained by this project and will not be imported again.',
      'Dieselbe Aufnahme ist bereits in diesem Projekt vorhanden und wird nicht erneut importiert.',
    ),
    Revision3VoiceFolderRowStatus.unmatched => _text(
      'No exact friendly line match was found. This workflow never guesses a destination.',
      'Es wurde keine exakte freundliche Zeilenzuordnung gefunden. Dieser Ablauf rät niemals ein Ziel.',
    ),
    Revision3VoiceFolderRowStatus.ambiguous => _text(
      'More than one friendly line matches. This workflow never chooses one implicitly.',
      'Mehr als eine freundliche Zeile passt. Dieser Ablauf wählt niemals stillschweigend eine davon aus.',
    ),
    Revision3VoiceFolderRowStatus.invalid => _text(
      'The file is not a supported safe Vorbis or Opus Ogg recording.',
      'Die Datei ist keine unterstützte sichere Vorbis- oder Opus-Ogg-Aufnahme.',
    ),
  };
}

/// Large review-first Voice folder workflow over one exact managed checkpoint.
///
/// It renders no source path, entity identity, LocID, hash, head, checkpoint,
/// or plan token. The injected service remains the sole plan/apply authority.
class Revision3VoiceFolderImportDialog extends StatefulWidget {
  const Revision3VoiceFolderImportDialog({
    required this.projectId,
    required this.projectRevision,
    required this.projectHead,
    required this.checkpointToken,
    required this.service,
    required this.copy,
    this.pickFolder,
    this.initialLocale = '',
    super.key,
  });

  final String projectId;
  final int projectRevision;
  final String projectHead;
  final String checkpointToken;
  final Revision3VoiceFolderAuthoringService service;
  final Revision3VoiceFolderImportDialogCopy copy;
  final Revision3VoiceFolderDirectoryPicker? pickFolder;
  final String initialLocale;

  @override
  State<Revision3VoiceFolderImportDialog> createState() =>
      _Revision3VoiceFolderImportDialogState();
}

/// Opens the locked workflow route. It can be closed with its own controls,
/// but never by accidentally clicking the modal barrier.
Future<Revision3VoiceFolderImportPublication?>
showRevision3VoiceFolderImportDialog({
  required BuildContext context,
  required String projectId,
  required int projectRevision,
  required String projectHead,
  required String checkpointToken,
  required Revision3VoiceFolderAuthoringService service,
  required Revision3VoiceFolderImportDialogCopy copy,
  Revision3VoiceFolderDirectoryPicker? pickFolder,
  String initialLocale = '',
}) => showDialog<Revision3VoiceFolderImportPublication>(
  context: context,
  barrierDismissible: false,
  builder: (context) => Revision3VoiceFolderImportDialog(
    projectId: projectId,
    projectRevision: projectRevision,
    projectHead: projectHead,
    checkpointToken: checkpointToken,
    service: service,
    copy: copy,
    pickFolder: pickFolder,
    initialLocale: initialLocale,
  ),
);

enum _VoiceFolderPhase { source, planning, review, applying, success, terminal }

enum _VoiceFolderFilter { all, ready, existing, blocked }

enum _VoiceFolderTerminal { requiresReopen, uncertain }

typedef _VoiceFolderBinding = ({
  String projectId,
  int revision,
  String head,
  String checkpoint,
});

class _Revision3VoiceFolderImportDialogState
    extends State<Revision3VoiceFolderImportDialog> {
  final _locale = TextEditingController();
  final _search = TextEditingController();
  _VoiceFolderPhase _phase = _VoiceFolderPhase.source;
  _VoiceFolderFilter _filter = _VoiceFolderFilter.all;
  _VoiceFolderTerminal? _terminal;
  Revision3VoiceFolderImportPlan? _plan;
  Revision3VoiceFolderImportPlan? _pendingApplyPlan;
  Revision3VoiceFolderImportPublication? _publication;
  String? _folderPath;
  String? _selectedRowToken;
  String? _error;
  bool _showCompactDetails = false;
  bool _picking = false;
  bool _publishing = false;
  int _ownerEpoch = 0;

  @override
  void initState() {
    super.initState();
    _locale.text = widget.initialLocale;
    _search.addListener(_searchChanged);
  }

  @override
  void didUpdateWidget(covariant Revision3VoiceFolderImportDialog oldWidget) {
    super.didUpdateWidget(oldWidget);
    final bindingChanged =
        oldWidget.projectId != widget.projectId ||
        oldWidget.projectRevision != widget.projectRevision ||
        oldWidget.projectHead != widget.projectHead ||
        oldWidget.checkpointToken != widget.checkpointToken;
    final serviceChanged = !identical(oldWidget.service, widget.service);
    if (!bindingChanged && !serviceChanged) return;

    final pending = _pendingApplyPlan;
    final possiblePublicationRebind =
        _publishing &&
        pending != null &&
        widget.projectId == pending.projectId &&
        widget.projectRevision == pending.projectRevision + 1 &&
        oldWidget.projectId == pending.projectId &&
        oldWidget.projectRevision == pending.projectRevision;
    if (possiblePublicationRebind) return;

    _ownerEpoch++;
    _pendingApplyPlan = null;
    _publishing = false;
    _plan = null;
    _publication = null;
    _selectedRowToken = null;
    _showCompactDetails = false;
    _terminal = null;
    _phase = _VoiceFolderPhase.source;
    _error = widget.copy.stale;
  }

  @override
  void dispose() {
    _ownerEpoch++;
    _locale.dispose();
    _search
      ..removeListener(_searchChanged)
      ..dispose();
    super.dispose();
  }

  _VoiceFolderBinding get _binding => (
    projectId: widget.projectId,
    revision: widget.projectRevision,
    head: widget.projectHead,
    checkpoint: widget.checkpointToken,
  );

  bool _ownerIsCurrent(int owner, _VoiceFolderBinding binding) =>
      mounted && owner == _ownerEpoch && _binding == binding;

  bool get _localeIsValid {
    final value = _locale.text;
    return value.trim() == value && revision3VoiceLocaleIsCanonical(value);
  }

  bool get _busy =>
      _picking ||
      _phase == _VoiceFolderPhase.planning ||
      _phase == _VoiceFolderPhase.applying;

  void _searchChanged() => setState(() {});

  Future<void> _pickFolder() async {
    if (_busy) return;
    final binding = _binding;
    final owner = ++_ownerEpoch;
    setState(() {
      _picking = true;
      _error = null;
    });
    try {
      final path =
          await (widget.pickFolder ??
              () => getDirectoryPath(
                confirmButtonText: widget.copy.chooseFolder,
              ))();
      if (!_ownerIsCurrent(owner, binding)) return;
      if (path != null) {
        setState(() {
          _folderPath = path;
          _invalidateReview();
        });
      }
    } catch (_) {
      if (!_ownerIsCurrent(owner, binding)) return;
      setState(() => _error = widget.copy.pickerFailed);
    } finally {
      if (_ownerIsCurrent(owner, binding)) {
        setState(() => _picking = false);
      }
    }
  }

  void _localeChanged(String _) {
    if (_busy) return;
    setState(() {
      _error = null;
      _invalidateReview();
    });
  }

  void _invalidateReview() {
    _plan = null;
    _publication = null;
    _selectedRowToken = null;
    _showCompactDetails = false;
    _terminal = null;
    _phase = _VoiceFolderPhase.source;
  }

  Future<void> _reviewFolder() async {
    final folderPath = _folderPath;
    if (_busy || folderPath == null || !_localeIsValid) {
      if (!_localeIsValid) setState(() => _error = widget.copy.localeInvalid);
      return;
    }
    final binding = _binding;
    final service = widget.service;
    final owner = ++_ownerEpoch;
    late final Revision3VoiceFolderPlanRequest request;
    try {
      request = Revision3VoiceFolderPlanRequest(
        folderPath: folderPath,
        locale: _locale.text,
        expectedProjectId: binding.projectId,
        expectedProjectRevision: binding.revision,
        expectedProjectHead: binding.head,
        expectedCheckpointToken: binding.checkpoint,
      );
    } on FormatException {
      setState(() => _error = widget.copy.planFailed);
      return;
    }
    setState(() {
      _phase = _VoiceFolderPhase.planning;
      _error = null;
      _plan = null;
      _publication = null;
      _terminal = null;
    });
    try {
      final plan = await service.plan(request);
      if (!_ownerIsCurrent(owner, binding)) return;
      final firstProblem = plan.rows.where((row) => row.isBlocked).firstOrNull;
      _search.clear();
      setState(() {
        _plan = plan;
        _selectedRowToken = (firstProblem ?? plan.rows.firstOrNull)?.rowToken;
        _phase = _VoiceFolderPhase.review;
        _filter = _VoiceFolderFilter.all;
      });
    } on Revision3VoiceFolderStaleCheckpointException {
      if (!_ownerIsCurrent(owner, binding)) return;
      setState(() {
        _phase = _VoiceFolderPhase.source;
        _error = widget.copy.stale;
      });
    } on Revision3VoiceFolderRequiresReopenException {
      if (!_ownerIsCurrent(owner, binding)) return;
      _setTerminal(_VoiceFolderTerminal.requiresReopen);
    } on Revision3VoiceFolderPublicationUncertainException {
      if (!_ownerIsCurrent(owner, binding)) return;
      _setTerminal(_VoiceFolderTerminal.uncertain);
    } catch (_) {
      if (!_ownerIsCurrent(owner, binding)) return;
      setState(() {
        _phase = _VoiceFolderPhase.source;
        _error = widget.copy.planFailed;
      });
    }
  }

  void _cancelPlanning() {
    if (_phase != _VoiceFolderPhase.planning) return;
    _ownerEpoch++;
    setState(() {
      _phase = _VoiceFolderPhase.source;
      _error = null;
    });
  }

  Future<void> _apply() async {
    final plan = _plan;
    if (_busy || plan == null || !plan.canApply) {
      return;
    }
    final origin = _binding;
    final service = widget.service;
    final owner = ++_ownerEpoch;
    _pendingApplyPlan = plan;
    setState(() {
      _phase = _VoiceFolderPhase.applying;
      _publishing = true;
      _error = null;
      _terminal = null;
    });
    try {
      final publication = await service.apply(plan: plan);
      if (!mounted || owner != _ownerEpoch) return;
      final current = _binding;
      final unchangedOrigin = current == origin;
      final exactPublicationRebind =
          current.projectId == publication.projectId &&
          current.revision == publication.projectRevision &&
          current.head == publication.projectHead &&
          current.checkpoint == publication.checkpointToken;
      if (!unchangedOrigin && !exactPublicationRebind) {
        _invalidateAfterAuthorityDrift();
        return;
      }
      setState(() {
        _publication = publication;
        _phase = _VoiceFolderPhase.success;
        _publishing = false;
        _pendingApplyPlan = null;
      });
    } on Revision3VoiceFolderStaleCheckpointException {
      if (!_applyOwnerStillCurrent(owner, origin)) return;
      setState(() {
        _phase = _VoiceFolderPhase.review;
        _publishing = false;
        _pendingApplyPlan = null;
        _error = widget.copy.stale;
      });
    } on Revision3VoiceFolderRequiresReopenException {
      if (!_operationOwnerStillCurrent(owner)) return;
      _publishing = false;
      _pendingApplyPlan = null;
      _setTerminal(_VoiceFolderTerminal.requiresReopen);
    } on Revision3VoiceFolderPublicationUncertainException {
      if (!_operationOwnerStillCurrent(owner)) return;
      _publishing = false;
      _pendingApplyPlan = null;
      _setTerminal(_VoiceFolderTerminal.uncertain);
    } catch (_) {
      if (!_applyOwnerStillCurrent(owner, origin)) return;
      setState(() {
        _phase = _VoiceFolderPhase.review;
        _publishing = false;
        _pendingApplyPlan = null;
        _error = widget.copy.applyFailed;
      });
    }
  }

  bool _applyOwnerStillCurrent(int owner, _VoiceFolderBinding origin) {
    if (!_operationOwnerStillCurrent(owner)) return false;
    if (_binding == origin) return true;
    _invalidateAfterAuthorityDrift();
    return false;
  }

  bool _operationOwnerStillCurrent(int owner) =>
      mounted && owner == _ownerEpoch;

  void _invalidateAfterAuthorityDrift() {
    _ownerEpoch++;
    setState(() {
      _publishing = false;
      _pendingApplyPlan = null;
      _plan = null;
      _publication = null;
      _phase = _VoiceFolderPhase.source;
      _error = widget.copy.stale;
    });
  }

  void _setTerminal(_VoiceFolderTerminal terminal) {
    setState(() {
      _terminal = terminal;
      _phase = _VoiceFolderPhase.terminal;
      _publishing = false;
      _pendingApplyPlan = null;
    });
  }

  List<Revision3VoiceFolderReviewRow> get _visibleRows {
    final plan = _plan;
    if (plan == null) return const [];
    final query = _search.text.trim().toLowerCase();
    return plan.rows
        .where((row) {
          final matchesFilter = switch (_filter) {
            _VoiceFolderFilter.all => true,
            _VoiceFolderFilter.ready => row.isReady,
            _VoiceFolderFilter.existing => row.isAlreadyPresent,
            _VoiceFolderFilter.blocked => row.isBlocked,
          };
          if (!matchesFilter) return false;
          if (query.isEmpty) return true;
          return <String>[
            widget.copy.recordingNumber(row.ordinal + 1),
            ?row.lineLabel,
            ?row.speakerLabel,
            ?row.takeDisplayName,
            widget.copy.status(row.status),
          ].any((value) => value.toLowerCase().contains(query));
        })
        .toList(growable: false);
  }

  Revision3VoiceFolderReviewRow? get _selectedRow {
    final plan = _plan;
    if (plan == null) return null;
    for (final row in plan.rows) {
      if (row.rowToken == _selectedRowToken) return row;
    }
    return null;
  }

  void _selectRow(Revision3VoiceFolderReviewRow row, {required bool compact}) {
    setState(() {
      _selectedRowToken = row.rowToken;
      _showCompactDetails = compact;
    });
  }

  void _setFilter(_VoiceFolderFilter filter) {
    setState(() {
      _filter = filter;
      final visible = _visibleRows;
      if (visible.isNotEmpty &&
          !visible.any((row) => row.rowToken == _selectedRowToken)) {
        _selectedRowToken = visible.first.rowToken;
      }
    });
  }

  void _close() => Navigator.of(context).pop(_publication);

  @override
  Widget build(BuildContext context) {
    final media = MediaQuery.of(context);
    final compactDialog = media.size.width < 720;
    final content = CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.enter, control: true): () {
          if (_phase == _VoiceFolderPhase.review) unawaited(_apply());
        },
      },
      child: FocusTraversalGroup(
        policy: OrderedTraversalPolicy(),
        child: Material(
          color: Theme.of(context).colorScheme.surface,
          child: Column(
            key: const Key('revision3-voice-folder-import'),
            children: [
              _buildHeader(),
              const Divider(height: 1),
              Expanded(child: _buildBody()),
              const Divider(height: 1),
              _buildFooter(),
            ],
          ),
        ),
      ),
    );
    final dialog = compactDialog
        ? Dialog.fullscreen(child: content)
        : Dialog(
            insetPadding: const EdgeInsets.all(24),
            clipBehavior: Clip.antiAlias,
            child: SizedBox(
              width: 1120,
              height: math.min(780, media.size.height - 48),
              child: content,
            ),
          );
    return PopScope(canPop: !_publishing, child: dialog);
  }

  Widget _buildHeader() => Padding(
    padding: const EdgeInsets.fromLTRB(20, 14, 10, 12),
    child: Row(
      children: [
        const Icon(Icons.library_music_outlined),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                widget.copy.title,
                style: Theme.of(context).textTheme.titleLarge,
              ),
              const SizedBox(height: 2),
              Text(
                _phaseLabel,
                key: const Key('revision3-voice-folder-phase'),
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ],
          ),
        ),
        IconButton(
          key: const Key('revision3-voice-folder-close'),
          tooltip: _publishing
              ? widget.copy.publicationLocked
              : widget.copy.close,
          onPressed: _publishing ? null : _close,
          icon: const Icon(Icons.close),
        ),
      ],
    ),
  );

  String get _phaseLabel => switch (_phase) {
    _VoiceFolderPhase.source ||
    _VoiceFolderPhase.planning => widget.copy.chooseRecordings,
    _VoiceFolderPhase.review ||
    _VoiceFolderPhase.applying => widget.copy.reviewChanges,
    _VoiceFolderPhase.success ||
    _VoiceFolderPhase.terminal => widget.copy.result,
  };

  Widget _buildBody() => switch (_phase) {
    _VoiceFolderPhase.source => _buildSource(),
    _VoiceFolderPhase.planning => _buildPlanning(),
    _VoiceFolderPhase.review => _buildReview(),
    _VoiceFolderPhase.applying => _buildApplying(),
    _VoiceFolderPhase.success => _buildSuccess(),
    _VoiceFolderPhase.terminal => _buildTerminal(),
  };

  Widget _buildSource() => SingleChildScrollView(
    padding: const EdgeInsets.all(24),
    child: Align(
      alignment: Alignment.topCenter,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 760),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _BoundaryCard(copy: widget.copy),
            if (_error case final error?) ...[
              const SizedBox(height: 16),
              _MessageCard(
                key: const Key('revision3-voice-folder-error'),
                message: error,
                error: true,
              ),
            ],
            const SizedBox(height: 22),
            Text(
              widget.copy.folderLabel,
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 8),
            Container(
              key: const Key('revision3-voice-folder-source'),
              padding: const EdgeInsets.all(14),
              decoration: BoxDecoration(
                color: Theme.of(context).colorScheme.surfaceContainerLow,
                borderRadius: BorderRadius.circular(12),
                border: Border.all(
                  color: Theme.of(context).colorScheme.outlineVariant,
                ),
              ),
              child: Row(
                children: [
                  const Icon(Icons.folder_outlined),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Text(
                      _folderPath == null
                          ? widget.copy.noFolder
                          : widget.copy.folderSelected,
                      key: const Key('revision3-voice-folder-friendly-name'),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  const SizedBox(width: 12),
                  OutlinedButton.icon(
                    key: const Key('revision3-voice-folder-pick'),
                    onPressed: _picking ? null : _pickFolder,
                    icon: _picking
                        ? const SizedBox.square(
                            dimension: 16,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.folder_open_outlined),
                    label: Text(
                      _folderPath == null
                          ? widget.copy.chooseFolder
                          : widget.copy.changeFolder,
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 18),
            TextField(
              key: const Key('revision3-voice-folder-locale'),
              controller: _locale,
              enabled: !_busy,
              maxLength: 35,
              decoration: InputDecoration(
                labelText: widget.copy.localeLabel,
                hintText: widget.copy.localeHint,
                helperText: widget.copy.localeHelper,
                errorText: _locale.text.isEmpty || _localeIsValid
                    ? null
                    : widget.copy.localeInvalid,
                border: const OutlineInputBorder(),
              ),
              onChanged: _localeChanged,
              onSubmitted: (_) {
                if (_folderPath != null && _localeIsValid) {
                  unawaited(_reviewFolder());
                }
              },
            ),
          ],
        ),
      ),
    ),
  );

  Widget _buildPlanning() => Center(
    child: Semantics(
      liveRegion: true,
      label: widget.copy.reviewing,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const CircularProgressIndicator(
                key: Key('revision3-voice-folder-planning'),
              ),
              const SizedBox(height: 18),
              Text(
                widget.copy.reviewing,
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 6),
              Text(widget.copy.reviewDescription, textAlign: TextAlign.center),
            ],
          ),
        ),
      ),
    ),
  );

  Widget _buildReview() {
    final plan = _plan!;
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(18, 12, 18, 10),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Semantics(
                liveRegion: true,
                child: Text(
                  widget.copy.summary(
                    plan.counts.ready,
                    plan.counts.alreadyPresent,
                    plan.counts.blocked,
                    plan.counts.ogg,
                    plan.counts.ignored,
                  ),
                  key: const Key('revision3-voice-folder-summary'),
                  style: Theme.of(context).textTheme.titleMedium,
                ),
              ),
              if (!plan.hasBlockingRows) ...[
                const SizedBox(height: 4),
                Text(
                  widget.copy.allOrNothing(
                    plan.counts.ready,
                    plan.counts.alreadyPresent,
                  ),
                ),
              ],
              if (_error case final error?) ...[
                const SizedBox(height: 10),
                _MessageCard(
                  key: const Key('revision3-voice-folder-error'),
                  message: error,
                  error: true,
                ),
              ],
            ],
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: LayoutBuilder(
            builder: (context, constraints) {
              final wide = constraints.maxWidth >= 820;
              if (wide) {
                return Row(
                  key: const Key('revision3-voice-folder-wide-review'),
                  children: [
                    SizedBox(
                      width: 380,
                      child: _buildRowBrowser(compact: false),
                    ),
                    const VerticalDivider(width: 1),
                    Expanded(
                      child: _buildRowDetails(_selectedRow, compact: false),
                    ),
                  ],
                );
              }
              if (_showCompactDetails && _selectedRow != null) {
                return KeyedSubtree(
                  key: const Key('revision3-voice-folder-compact-details'),
                  child: _buildRowDetails(_selectedRow, compact: true),
                );
              }
              return KeyedSubtree(
                key: const Key('revision3-voice-folder-compact-list'),
                child: _buildRowBrowser(compact: true),
              );
            },
          ),
        ),
        if (plan.hasBlockingRows)
          Padding(
            padding: const EdgeInsets.fromLTRB(18, 8, 18, 12),
            child: _MessageCard(
              key: const Key('revision3-voice-folder-blocked'),
              message: widget.copy.blockedImport(plan.counts.blocked),
              error: true,
            ),
          )
        else if (!plan.hasReadyRows)
          Padding(
            padding: const EdgeInsets.fromLTRB(18, 8, 18, 12),
            child: _MessageCard(
              key: const Key('revision3-voice-folder-no-ready'),
              message: widget.copy.noReadyRows,
              error: false,
            ),
          ),
      ],
    );
  }

  Widget _buildRowBrowser({required bool compact}) {
    final rows = _visibleRows;
    return CustomScrollView(
      key: const Key('revision3-voice-folder-row-browser'),
      slivers: [
        SliverPadding(
          padding: const EdgeInsets.all(12),
          sliver: SliverToBoxAdapter(
            child: Column(
              children: [
                TextField(
                  key: const Key('revision3-voice-folder-search'),
                  controller: _search,
                  decoration: InputDecoration(
                    labelText: widget.copy.searchLabel,
                    prefixIcon: const Icon(Icons.search),
                    isDense: true,
                    border: const OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 8),
                Wrap(
                  spacing: 6,
                  runSpacing: 6,
                  children: [
                    _filterChip(_VoiceFolderFilter.all, widget.copy.filterAll),
                    _filterChip(
                      _VoiceFolderFilter.ready,
                      widget.copy.filterReady,
                    ),
                    _filterChip(
                      _VoiceFolderFilter.existing,
                      widget.copy.filterExisting,
                    ),
                    _filterChip(
                      _VoiceFolderFilter.blocked,
                      widget.copy.filterBlocked,
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
        const SliverToBoxAdapter(child: Divider(height: 1)),
        if (rows.isEmpty)
          SliverFillRemaining(
            hasScrollBody: false,
            child: Center(child: Text(widget.copy.noRows)),
          )
        else
          SliverList.builder(
            key: const Key('revision3-voice-folder-rows'),
            itemCount: rows.length,
            itemBuilder: (context, index) {
              final row = rows[index];
              final selected = row.rowToken == _selectedRowToken;
              return Semantics(
                button: true,
                selected: selected,
                label:
                    '${widget.copy.recordingNumber(row.ordinal + 1)}, ${widget.copy.status(row.status)}'
                    '${row.lineLabel == null ? '' : ', ${row.lineLabel}'}',
                child: ListTile(
                  key: ValueKey('revision3-voice-folder-row-${row.ordinal}'),
                  selected: selected,
                  leading: Icon(_statusIcon(row.status)),
                  title: Text(
                    widget.copy.recordingNumber(row.ordinal + 1),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  subtitle: Text(
                    row.lineLabel ??
                        (row.isReady || row.isAlreadyPresent
                            ? widget.copy.dialogLineNumber(row.ordinal + 1)
                            : widget.copy.status(row.status)),
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                  trailing: compact ? const Icon(Icons.chevron_right) : null,
                  onTap: () => _selectRow(row, compact: compact),
                ),
              );
            },
          ),
      ],
    );
  }

  Widget _filterChip(_VoiceFolderFilter filter, String label) => ChoiceChip(
    key: ValueKey('revision3-voice-folder-filter-${filter.name}'),
    label: Text(label),
    selected: _filter == filter,
    onSelected: (_) => _setFilter(filter),
  );

  Widget _buildRowDetails(
    Revision3VoiceFolderReviewRow? row, {
    required bool compact,
  }) {
    if (row == null) return Center(child: Text(widget.copy.selectRecording));
    return Column(
      children: [
        if (compact)
          Material(
            color: Theme.of(context).colorScheme.surfaceContainerLow,
            child: Align(
              alignment: Alignment.centerLeft,
              child: IconButton(
                key: const Key('revision3-voice-folder-details-back'),
                tooltip: widget.copy.backToList,
                onPressed: () => setState(() => _showCompactDetails = false),
                icon: const Icon(Icons.arrow_back),
              ),
            ),
          ),
        Expanded(
          child: ListView(
            key: const Key('revision3-voice-folder-row-details'),
            padding: const EdgeInsets.all(22),
            children: [
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Icon(_statusIcon(row.status), size: 28),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          widget.copy.recordingNumber(row.ordinal + 1),
                          style: Theme.of(context).textTheme.titleLarge,
                        ),
                        const SizedBox(height: 3),
                        Text(
                          widget.copy.status(row.status),
                          style: Theme.of(context).textTheme.titleSmall,
                        ),
                      ],
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              Text(widget.copy.statusDescription(row.status)),
              const Divider(height: 28),
              _detail(
                widget.copy.recording,
                widget.copy.recordingNumber(row.ordinal + 1),
              ),
              if (row.codec case final codec?)
                _detail(
                  widget.copy.audio,
                  '${widget.copy.codec(codec)} · ${_formatBytes(row.byteLength!)}',
                ),
              if (row.codec == Revision3VoiceFolderCodec.opus) ...[
                const SizedBox(height: 8),
                _MessageCard(message: widget.copy.opusWarning, error: false),
              ],
              if (row.lineLabel case final line?) ...[
                _detail(widget.copy.dialogLine, line),
                if (row.speakerLabel case final speaker?)
                  _detail(widget.copy.speaker, speaker),
              ] else if (row.isReady || row.isAlreadyPresent)
                _detail(
                  widget.copy.dialogLine,
                  widget.copy.dialogLineNumber(row.ordinal + 1),
                ),
              if (row.takeDisplayName case final take?)
                _detail(
                  widget.copy.plannedTake,
                  '$take · ${widget.copy.recordedStatus}',
                ),
              if (row.beforeTakeCount case final before?) ...[
                const Divider(height: 28),
                _detail(
                  widget.copy.semanticChange,
                  widget.copy.takeCountChange(before, row.afterTakeCount!),
                ),
                _fact(
                  Icons.check_circle_outline,
                  widget.copy.selectionUnchanged,
                ),
                _fact(
                  Icons.link_outlined,
                  '${widget.copy.targetUnchanged} · '
                  '${widget.copy.targetState(row.targetState!)}',
                ),
              ],
              const Divider(height: 28),
              _fact(Icons.mic_none_outlined, widget.copy.recordedOnly),
              _fact(Icons.lock_outline, widget.copy.unchangedBoundary),
            ],
          ),
        ),
      ],
    );
  }

  Widget _detail(String label, String value) => Padding(
    padding: const EdgeInsets.symmetric(vertical: 5),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 130,
          child: Text(label, style: Theme.of(context).textTheme.labelLarge),
        ),
        const SizedBox(width: 10),
        Expanded(child: Text(value)),
      ],
    ),
  );

  Widget _fact(IconData icon, String text) => Padding(
    padding: const EdgeInsets.symmetric(vertical: 4),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(icon, size: 18),
        const SizedBox(width: 8),
        Expanded(child: Text(text)),
      ],
    ),
  );

  Widget _buildApplying() => Center(
    child: Semantics(
      liveRegion: true,
      label: widget.copy.saving,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 560),
        child: Padding(
          padding: const EdgeInsets.all(28),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const CircularProgressIndicator(
                key: Key('revision3-voice-folder-applying'),
              ),
              const SizedBox(height: 18),
              Text(
                widget.copy.saving,
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 6),
              Text(widget.copy.savingDescription, textAlign: TextAlign.center),
            ],
          ),
        ),
      ),
    ),
  );

  Widget _buildSuccess() {
    final publication = _publication!;
    return Center(
      child: Semantics(
        liveRegion: true,
        label: widget.copy.success(publication.importedCount),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 620),
          child: Padding(
            padding: const EdgeInsets.all(28),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(
                  Icons.check_circle_outline,
                  key: const Key('revision3-voice-folder-success'),
                  size: 52,
                  color: Theme.of(context).colorScheme.primary,
                ),
                const SizedBox(height: 16),
                Text(
                  widget.copy.success(publication.importedCount),
                  textAlign: TextAlign.center,
                  style: Theme.of(context).textTheme.titleLarge,
                ),
                const SizedBox(height: 8),
                Text(widget.copy.successBoundary, textAlign: TextAlign.center),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildTerminal() {
    final uncertain = _terminal == _VoiceFolderTerminal.uncertain;
    final message = uncertain
        ? widget.copy.uncertain
        : widget.copy.requiresReopen;
    return Center(
      child: Semantics(
        liveRegion: true,
        label: message,
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 620),
          child: Padding(
            padding: const EdgeInsets.all(28),
            child: _MessageCard(
              key: const Key('revision3-voice-folder-terminal'),
              message: message,
              error: true,
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildFooter() {
    final plan = _plan;
    if (_phase == _VoiceFolderPhase.applying) {
      return Padding(
        padding: const EdgeInsets.fromLTRB(16, 10, 16, 12),
        child: Align(
          alignment: Alignment.centerLeft,
          child: Text(widget.copy.publicationLocked),
        ),
      );
    }
    final actions = <Widget>[];
    if (_phase == _VoiceFolderPhase.source) {
      actions.addAll([
        TextButton(onPressed: _close, child: Text(widget.copy.cancel)),
        FilledButton.icon(
          key: const Key('revision3-voice-folder-review'),
          onPressed: _folderPath != null && _localeIsValid && !_picking
              ? _reviewFolder
              : null,
          icon: const Icon(Icons.fact_check_outlined),
          label: Text(widget.copy.reviewFolder),
        ),
      ]);
    } else if (_phase == _VoiceFolderPhase.planning) {
      actions.add(
        TextButton(
          key: const Key('revision3-voice-folder-cancel-review'),
          onPressed: _cancelPlanning,
          child: Text(widget.copy.cancelReview),
        ),
      );
    } else if (_phase == _VoiceFolderPhase.review) {
      actions.addAll([
        OutlinedButton.icon(
          key: const Key('revision3-voice-folder-review-again'),
          onPressed: _reviewFolder,
          icon: const Icon(Icons.refresh),
          label: Text(widget.copy.reviewAgain),
        ),
        FilledButton.icon(
          key: const Key('revision3-voice-folder-apply'),
          onPressed: plan != null && plan.canApply ? _apply : null,
          icon: const Icon(Icons.library_add_outlined),
          label: Text(widget.copy.importRecordings(plan?.counts.ready ?? 0)),
        ),
      ]);
    } else if (_phase == _VoiceFolderPhase.success ||
        _phase == _VoiceFolderPhase.terminal) {
      actions.add(
        FilledButton(
          key: const Key('revision3-voice-folder-result-close'),
          onPressed: _close,
          child: Text(widget.copy.close),
        ),
      );
    }
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 10, 16, 12),
      child: OverflowBar(
        alignment: MainAxisAlignment.end,
        overflowAlignment: OverflowBarAlignment.end,
        spacing: 8,
        overflowSpacing: 8,
        children: actions,
      ),
    );
  }
}

class _BoundaryCard extends StatelessWidget {
  const _BoundaryCard({required this.copy});

  final Revision3VoiceFolderImportDialogCopy copy;

  @override
  Widget build(BuildContext context) => Container(
    key: const Key('revision3-voice-folder-boundary'),
    padding: const EdgeInsets.all(16),
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.secondaryContainer,
      borderRadius: BorderRadius.circular(12),
    ),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            Chip(label: Text(copy.projectOnly)),
            Chip(label: Text(copy.recordedOnly)),
          ],
        ),
        const SizedBox(height: 8),
        Text(copy.boundary),
        const SizedBox(height: 5),
        Text(copy.unchangedBoundary),
      ],
    ),
  );
}

class _MessageCard extends StatelessWidget {
  const _MessageCard({required this.message, required this.error, super.key});

  final String message;
  final bool error;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: error ? scheme.errorContainer : scheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(error ? Icons.error_outline : Icons.info_outline, size: 20),
          const SizedBox(width: 10),
          Expanded(child: Text(message)),
        ],
      ),
    );
  }
}

IconData _statusIcon(Revision3VoiceFolderRowStatus status) => switch (status) {
  Revision3VoiceFolderRowStatus.ready => Icons.check_circle_outline,
  Revision3VoiceFolderRowStatus.alreadyPresent => Icons.inventory_2_outlined,
  Revision3VoiceFolderRowStatus.unmatched => Icons.link_off_outlined,
  Revision3VoiceFolderRowStatus.ambiguous => Icons.call_split_outlined,
  Revision3VoiceFolderRowStatus.invalid => Icons.error_outline,
};

String _formatBytes(int bytes) {
  if (bytes < 1024) return '$bytes B';
  if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KiB';
  return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MiB';
}

extension<T> on Iterable<T> {
  T? get firstOrNull {
    final iterator = this.iterator;
    return iterator.moveNext() ? iterator.current : null;
  }
}
