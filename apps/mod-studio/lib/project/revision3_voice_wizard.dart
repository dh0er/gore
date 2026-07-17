import 'dart:convert';
import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

import '../core/mod_ffi.dart';
import 'revision3_voice_authoring.dart';

typedef Revision3VoiceOggPicker = Future<String?> Function();
typedef Revision3VoiceOggPreviewLauncher = Future<void> Function(String path);

@immutable
final class Revision3VoiceTakeDialogCopy {
  const Revision3VoiceTakeDialogCopy.english() : _german = false;

  const Revision3VoiceTakeDialogCopy.german() : _german = true;

  final bool _german;

  String _text(String english, String german) => _german ? german : english;

  String get title => _text('Add a Voice take', 'Voice-Take hinzufügen');
  String get fixedContextUnavailable => _text(
    'This Voice action no longer matches one intact dialog line and language in the exact current project. Close it and reopen the action from the current workspace. No files were changed.',
    'Diese Voice-Aktion passt im exakt aktuellen Projekt nicht mehr zu genau einer intakten Dialogzeile und Sprache. Schließe sie und öffne die Aktion erneut aus dem aktuellen Arbeitsbereich. Es wurden keine Dateien geändert.',
  );
  String get catalogLoadFailed => _text(
    'Dialog lines could not be read from the exact current project. No project, game, or save files were changed.',
    'Die Dialogzeilen konnten nicht aus dem exakt aktuellen Projekt gelesen werden. Projekt-, Spiel- und Speicherdateien wurden nicht geändert.',
  );
  String get pickerFailed => _text(
    'The Ogg picker could not be opened. Choose the recording again.',
    'Die Ogg-Dateiauswahl konnte nicht geöffnet werden. Wähle die Aufnahme erneut aus.',
  );
  String get previewOpened => _text(
    'Opened the selected local recording for author preview. This does not approve or qualify the audio for the game.',
    'Die ausgewählte lokale Aufnahme wurde zur Autoren-Vorschau geöffnet. Dadurch wird das Audio weder freigegeben noch für das Spiel qualifiziert.',
  );
  String get previewFailed => _text(
    'The local recording preview could not be opened. Choose the recording again or check the configured audio player.',
    'Die lokale Aufnahme konnte nicht zur Vorschau geöffnet werden. Wähle sie erneut aus oder prüfe den eingerichteten Audioplayer.',
  );
  String get invalidForm => _text(
    'Review the dialog line, language, recording, take name, and status, then try again.',
    'Prüfe Dialogzeile, Sprache, Aufnahme, Take-Namen und Status und versuche es erneut.',
  );
  String get requiresReopen => _text(
    'This project can no longer be verified as current. Close this window and reopen the managed project before continuing.',
    'Dieses Projekt kann nicht mehr als aktuell bestätigt werden. Schließe dieses Fenster und öffne das verwaltete Projekt erneut, bevor du fortfährst.',
  );
  String get staleCheckpoint => _text(
    'The managed project changed while this window was open. Close it and add the take again from the current project.',
    'Das verwaltete Projekt wurde geändert, während dieses Fenster geöffnet war. Schließe es und füge den Take erneut aus dem aktuellen Projekt hinzu.',
  );
  String get saveFailed => _text(
    'The Voice take could not be saved. Nothing was built, deployed, or written into the game or a save. Review the form and try again.',
    'Der Voice-Take konnte nicht gespeichert werden. Es wurde nichts gebaut, bereitgestellt oder in das Spiel beziehungsweise einen Spielstand geschrieben. Prüfe das Formular und versuche es erneut.',
  );
  String get savingStatus => _text(
    'Saving Voice take to the managed project…',
    'Voice-Take wird im verwalteten Projekt gespeichert…',
  );
  String get loading => _text(
    'Loading dialog lines from the current project',
    'Dialogzeilen werden aus dem aktuellen Projekt geladen',
  );
  String get refreshDialogLines =>
      _text('Refresh dialog lines', 'Dialogzeilen aktualisieren');
  String get refreshVoiceContext =>
      _text('Refresh Voice context', 'Voice-Kontext aktualisieren');
  String get close => _text('Close', 'Schließen');
  String get cancel => _text('Cancel', 'Abbrechen');
  String get savingAction => _text('Saving take…', 'Take wird gespeichert…');
  String get submit =>
      _text('Add take to project', 'Take zum Projekt hinzufügen');
  String get boundaryProjectOnly =>
      _text('Saved to project only', 'Nur im Projekt gespeichert');
  String get boundaryNotInGame =>
      _text('Not yet usable in game', 'Noch nicht im Spiel nutzbar');
  String get boundaryDescription => _text(
    'This imports one real Ogg recording into the managed project. It does not compile, deploy, modify game files, or touch a save.',
    'Diese Aktion importiert eine echte Ogg-Aufnahme in das verwaltete Projekt. Sie kompiliert oder installiert nichts und ändert weder Spieldateien noch Spielstände.',
  );
  String get lineLabel => _text('Dialog line', 'Dialogzeile');
  String get lineHint => _text(
    'Search by speaker or line name',
    'Nach Sprecher oder Zeilenname suchen',
  );
  String get lineHelper => _text(
    'Type to search, then choose one exact existing line.',
    'Suche und wähle anschließend genau eine vorhandene Zeile aus.',
  );
  String get lineRequired => _text(
    'Search for and choose a dialog line',
    'Suche und wähle eine Dialogzeile aus',
  );
  String get localeLabel => _text('Language code', 'Sprachcode');
  String get localeHint => 'de';
  String get localeHelper =>
      _text('Examples: de, en, en-US', 'Beispiele: de, en, en-US');
  String get localeRequired =>
      _text('Enter a language code', 'Gib einen Sprachcode ein');
  String get localeInvalid => _text(
    'Use a language code such as de or en-US',
    'Verwende einen Sprachcode wie de oder en-US',
  );
  String get oggLabel => _text('Ogg recording', 'Ogg-Aufnahme');
  String get oggHelper => _text(
    'Vorbis and Opus Ogg files are validated before the project changes.',
    'Vorbis- und Opus-Ogg-Dateien werden geprüft, bevor das Projekt geändert wird.',
  );
  String get previewTooltip => _text(
    'Preview selected local Ogg',
    'Ausgewählte lokale Ogg-Datei vorhören',
  );
  String get browseTooltip => _text('Choose Ogg file', 'Ogg-Datei auswählen');
  String get takeNameLabel => _text('Take name', 'Take-Name');
  String get takeNameHint =>
      _text('Asghan German take 1', 'Asghan Deutsch Take 1');
  String get takeNameHelper => _text(
    'Initially suggested from the Ogg file name; you can rename it.',
    'Wird zunächst aus dem Ogg-Dateinamen vorgeschlagen und kann umbenannt werden.',
  );
  String get takeNameRequired =>
      _text('Enter a take name', 'Gib einen Take-Namen ein');
  String get takeNameTooLong =>
      _text('Take name is too long', 'Der Take-Name ist zu lang');
  String get statusLabel => _text('Review status', 'Prüfstatus');
  String get statusHelper => _text(
    'Manually set metadata; audio is not reviewed or approved automatically.',
    'Dieser Metadatenstatus wird manuell gesetzt; Audio wird nicht automatisch geprüft oder freigegeben.',
  );
  String status(AuthoringRevision3VoiceTakeStatus value) => switch (value) {
    AuthoringRevision3VoiceTakeStatus.draft => _text('Draft', 'Entwurf'),
    AuthoringRevision3VoiceTakeStatus.recorded => _text(
      'Recorded',
      'Aufgenommen',
    ),
    AuthoringRevision3VoiceTakeStatus.reviewed => _text('Reviewed', 'Geprüft'),
    AuthoringRevision3VoiceTakeStatus.approved => _text(
      'Approved',
      'Freigegeben',
    ),
  };
  String get selectTakeTitle => _text(
    'Use this as the selected take',
    'Diesen Take als ausgewählten Take verwenden',
  );
  String get selectTakeSubtitle => _text(
    'Available only after marking the take Approved.',
    'Erst verfügbar, nachdem der Take als Freigegeben markiert wurde.',
  );
  String voiceLanguage(String locale) =>
      _text('Voice language: $locale', 'Voice-Sprache: $locale');
  String get slotBlocked => _text(
    'This line already has a Voice slot for this language, but its project graph is not safe to extend. Choose another language or repair the project first.',
    'Diese Zeile besitzt bereits einen Voice-Slot für diese Sprache, aber ihr Projektgraph kann nicht sicher erweitert werden. Wähle eine andere Sprache oder repariere zuerst das Projekt.',
  );
  String get slotMissing => _text(
    'No Voice slot exists for this line and language yet. One will be added with the take.',
    'Für diese Zeile und Sprache gibt es noch keinen Voice-Slot. Er wird zusammen mit dem Take hinzugefügt.',
  );
  String slotExisting(int count, {required bool selected}) => _german
      ? '$count vorhandene${count == 1 ? 'r Take' : ' Takes'} · ${selected ? 'Ein Take ist derzeit ausgewählt.' : 'Derzeit ist kein Take ausgewählt.'}'
      : '$count existing take${count == 1 ? '' : 's'} · ${selected ? 'A take is currently selected.' : 'No take is currently selected.'}';
  String get replacementTitle =>
      _text('A take is already selected', 'Ein Take ist bereits ausgewählt');
  String get replacementDescription => _text(
    'Selecting this approved take will replace the current selection for this dialog line and language.',
    'Die Auswahl dieses freigegebenen Takes ersetzt die aktuelle Auswahl für diese Dialogzeile und Sprache.',
  );
  String get replacementConfirm => _text(
    'I understand and want to replace it',
    'Ich verstehe das und möchte die Auswahl ersetzen',
  );
  String get localizationPreservedTitle => _text(
    'Existing dialog text is preserved',
    'Vorhandener Dialogtext bleibt erhalten',
  );
  String get localizationPreservedDescription => _text(
    'Text editing is unavailable here until the current language text can be displayed and verified safely.',
    'Textbearbeitung ist hier nicht verfügbar, bis der Text der aktuellen Sprache sicher angezeigt und geprüft werden kann.',
  );
  String get sourceRequired =>
      _text('Choose an Ogg recording', 'Wähle eine Ogg-Aufnahme aus');
  String get sourceExtension => _text(
    'Choose a file ending in .ogg',
    'Wähle eine Datei mit der Endung .ogg aus',
  );
  String get pickerTypeLabel => _text('Ogg audio', 'Ogg-Audio');
  String get previewSourceInvalid => _text(
    'Choose a valid local Ogg recording first.',
    'Wähle zuerst eine gültige lokale Ogg-Aufnahme aus.',
  );
  String get previewSourceNotFile => _text(
    'The selected Ogg must be a regular local file.',
    'Die ausgewählte Ogg-Aufnahme muss eine reguläre lokale Datei sein.',
  );
  String get previewLauncherRejected => _text(
    'No external application accepted the selected Ogg recording.',
    'Keine externe Anwendung konnte die ausgewählte Ogg-Aufnahme öffnen.',
  );
  String importError(String code) => switch (code) {
    'AUTHORING_REVISION3_VOICE_GAME_ROOT_UNAVAILABLE' => _text(
      'The configured Gothic 1 Remake installation is unavailable. Check it in Settings, then try again.',
      'Die eingerichtete Gothic-1-Remake-Installation ist nicht verfügbar. Prüfe sie in den Einstellungen und versuche es erneut.',
    ),
    'AUTHORING_REVISION3_VOICE_STORE_GAME_ALIAS' => _text(
      'This project folder overlaps the configured game installation. Move the project outside the game folder before adding a Voice take.',
      'Der Projektordner überschneidet sich mit der eingerichteten Spielinstallation. Verschiebe das Projekt aus dem Spielordner, bevor du einen Voice-Take hinzufügst.',
    ),
    'AUTHORING_REVISION3_VOICE_INPUT_MISSING' => _text(
      'The selected Ogg file no longer exists. Choose the recording again.',
      'Die ausgewählte Ogg-Datei ist nicht mehr vorhanden. Wähle die Aufnahme erneut aus.',
    ),
    'AUTHORING_REVISION3_VOICE_INPUT_UNAVAILABLE' => _text(
      'The selected Ogg file could not be read. Close any app that is holding it, then try again.',
      'Die ausgewählte Ogg-Datei konnte nicht gelesen werden. Schließe Anwendungen, die darauf zugreifen, und versuche es erneut.',
    ),
    'AUTHORING_REVISION3_VOICE_INPUT_UNSAFE' => _text(
      'The selected source could not be opened safely. Choose a regular local .ogg file.',
      'Die ausgewählte Aufnahme konnte nicht sicher geöffnet werden. Wähle eine reguläre lokale .ogg-Datei aus.',
    ),
    'AUTHORING_REVISION3_VOICE_INPUT_LIMIT' => _text(
      'The selected Ogg file is larger than the supported import limit.',
      'Die ausgewählte Ogg-Datei überschreitet die unterstützte Importgröße.',
    ),
    'AUTHORING_REVISION3_VOICE_OGG_INVALID' => _text(
      'The selected file is not a supported, valid Vorbis or Opus Ogg recording.',
      'Die ausgewählte Datei ist keine unterstützte gültige Vorbis- oder Opus-Ogg-Aufnahme.',
    ),
    'AUTHORING_REVISION3_VOICE_INPUT_CHANGED' => _text(
      'The Ogg file changed while it was being verified. Wait for the recording to finish, then choose it again.',
      'Die Ogg-Datei wurde während der Prüfung geändert. Warte, bis die Aufnahme abgeschlossen ist, und wähle sie erneut aus.',
    ),
    'AUTHORING_REVISION3_VOICE_LIMIT' => _text(
      'This project cannot accept another Voice take at its current capacity.',
      'Dieses Projekt kann bei seiner aktuellen Kapazität keinen weiteren Voice-Take aufnehmen.',
    ),
    'AUTHORING_REVISION3_VOICE_INTENT_INVALID' ||
    'AUTHORING_REVISION3_VOICE_STATUS_INVALID' => _text(
      'The Voice take details are no longer valid for this line. Review the form and try again.',
      'Die Angaben zum Voice-Take sind für diese Zeile nicht mehr gültig. Prüfe das Formular und versuche es erneut.',
    ),
    _ => saveFailed,
  };
}

/// Visible normal-mode workflow for attaching a real Ogg take to one existing
/// managed-R3 dialog line. It exposes no entity IDs, CAS hashes, build,
/// deployment, game-write, save-write, or runtime controls.
class Revision3VoiceTakeDialog extends StatefulWidget {
  const Revision3VoiceTakeDialog({
    required this.service,
    required this.copy,
    this.pickOgg,
    this.previewOgg,
    this.initialLineId,
    this.initialLocale,
    this.fixedContext = false,
    super.key,
  });

  final Revision3VoiceAuthoringService service;
  final Revision3VoiceOggPicker? pickOgg;
  final Revision3VoiceOggPreviewLauncher? previewOgg;
  final Revision3VoiceTakeDialogCopy copy;

  /// Optional exact-current selection supplied by a preceding project action,
  /// such as creating a new DialogLine. The freshly loaded catalog still has
  /// to contain the line; stale or malformed values are discarded.
  final String? initialLineId;
  final String? initialLocale;

  /// Keeps a line/locale handoff fixed for an in-workspace action. The exact
  /// freshly loaded catalog must still prove that context safe before any
  /// authoring controls or mutation become available.
  final bool fixedContext;

  @override
  State<Revision3VoiceTakeDialog> createState() =>
      _Revision3VoiceTakeDialogState();
}

class _Revision3VoiceTakeDialogState extends State<Revision3VoiceTakeDialog> {
  final _formKey = GlobalKey<FormState>();
  final _locale = TextEditingController();
  final _source = TextEditingController();
  final _takeName = TextEditingController();

  Revision3VoiceCatalog? _catalog;
  String? _lineId;
  AuthoringRevision3VoiceTakeStatus _status =
      AuthoringRevision3VoiceTakeStatus.recorded;
  bool _selectTake = false;
  bool _replacementConfirmed = false;
  bool _takeNameWasManuallyEdited = false;
  bool _loading = true;
  bool _publishing = false;
  bool _publicationStarted = false;
  bool _requiresReopen = false;
  bool _staleCheckpoint = false;
  bool _picking = false;
  bool _previewing = false;
  bool _fixedContextInvalid = false;
  String? _error;
  int _loadGeneration = 0;
  int _catalogEpoch = 0;

  @override
  void initState() {
    super.initState();
    _lineId = widget.initialLineId;
    _locale.text = widget.initialLocale ?? '';
    _loadCatalog();
  }

  @override
  void didUpdateWidget(covariant Revision3VoiceTakeDialog oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.service, widget.service) ||
        oldWidget.fixedContext != widget.fixedContext ||
        oldWidget.initialLineId != widget.initialLineId ||
        oldWidget.initialLocale != widget.initialLocale) {
      _lineId = widget.initialLineId;
      _locale.text = widget.initialLocale ?? '';
      _loadCatalog(clear: true);
    }
  }

  @override
  void dispose() {
    _loadGeneration += 1;
    _locale.dispose();
    _source.dispose();
    _takeName.dispose();
    super.dispose();
  }

  Future<void> _loadCatalog({bool clear = false}) async {
    final generation = ++_loadGeneration;
    setState(() {
      _loading = true;
      _error = null;
      if (clear) {
        _catalog = null;
        _lineId = widget.fixedContext ? widget.initialLineId : null;
        _locale.text = widget.fixedContext ? widget.initialLocale ?? '' : '';
        _fixedContextInvalid = false;
      }
    });
    try {
      final catalog = await widget.service.loadCatalog();
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _catalog = catalog;
        _catalogEpoch += 1;
        if (widget.fixedContext) {
          final requestedLine = catalog.line(widget.initialLineId ?? '');
          final requestedLocale = widget.initialLocale;
          final contextIsValid =
              requestedLine != null &&
              requestedLocale != null &&
              revision3VoiceLocaleIsCanonical(requestedLocale) &&
              requestedLine.isLocaleAuthorable(requestedLocale);
          if (contextIsValid) {
            _lineId = requestedLine.lineId;
            _locale.text = requestedLocale;
            _fixedContextInvalid = false;
          } else {
            _lineId = null;
            _locale.clear();
            _fixedContextInvalid = true;
            _error = widget.copy.fixedContextUnavailable;
          }
          _selectTake = false;
          _replacementConfirmed = false;
        } else if (catalog.line(_lineId ?? '') == null) {
          _lineId = null;
          _selectTake = false;
          _replacementConfirmed = false;
        }
        if (!widget.fixedContext && _locale.text.isEmpty) {
          _locale.text = catalog.suggestedLocales.first;
        }
        _loading = false;
      });
    } catch (_) {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _catalog = null;
        _error = widget.copy.catalogLoadFailed;
      });
    }
  }

  Future<void> _pickSource() async {
    final picker = widget.pickOgg ?? () => _pickRevision3VoiceOgg(widget.copy);
    if (_picking || _publishing) return;
    setState(() {
      _picking = true;
      _error = null;
    });
    try {
      final path = await picker();
      if (!mounted || path == null) return;
      setState(() {
        _source.text = path;
        if (!_takeNameWasManuallyEdited) {
          final leaf = path.replaceAll('\\', '/').split('/').last;
          _takeName.text = leaf.toLowerCase().endsWith('.ogg')
              ? leaf.substring(0, leaf.length - 4)
              : leaf;
        }
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _error = widget.copy.pickerFailed;
      });
    } finally {
      if (mounted) setState(() => _picking = false);
    }
  }

  Future<void> _previewSource() async {
    if (_picking || _previewing || _publishing) return;
    final path = _source.text;
    setState(() {
      _previewing = true;
      _error = null;
    });
    try {
      await (widget.previewOgg ??
          (path) => _previewRevision3VoiceOgg(path, widget.copy))(path);
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(widget.copy.previewOpened)));
    } catch (_) {
      if (!mounted) return;
      setState(() => _error = widget.copy.previewFailed);
    } finally {
      if (mounted) setState(() => _previewing = false);
    }
  }

  Revision3VoiceDialogLineChoice? get _selectedLine =>
      _catalog?.line(_lineId ?? '');

  Revision3VoiceExistingSlotSummary? get _selectedSlotSummary =>
      _selectedLine?.slotSummaryForLocale(_locale.text.trim());

  bool get _selectedLocaleBlocked {
    final line = _selectedLine;
    final locale = _locale.text.trim();
    return line != null &&
        revision3VoiceLocaleIsCanonical(locale) &&
        !line.isLocaleAuthorable(locale);
  }

  bool get _replacementConfirmationRequired =>
      _selectTake &&
      _status == AuthoringRevision3VoiceTakeStatus.approved &&
      (_selectedSlotSummary?.hasSelectedTake ?? false);

  bool get _fixedContextIsCurrent {
    if (!widget.fixedContext) return true;
    final line = _selectedLine;
    final locale = widget.initialLocale;
    return !_fixedContextInvalid &&
        line != null &&
        line.lineId == widget.initialLineId &&
        locale != null &&
        _locale.text == locale &&
        revision3VoiceLocaleIsCanonical(locale) &&
        line.isLocaleAuthorable(locale);
  }

  void _selectLine(Revision3VoiceDialogLineChoice line) {
    setState(() {
      _lineId = line.lineId;
      _selectTake = false;
      _replacementConfirmed = false;
    });
  }

  void _clearChangedLineSearch(String value) {
    final selected = _selectedLine;
    if (selected != null && value != selected.displayLabel) {
      setState(() {
        _lineId = null;
        _selectTake = false;
        _replacementConfirmed = false;
      });
    }
  }

  void _changeLocale(String value) {
    setState(() {
      _locale.text = value;
      _selectTake = false;
      _replacementConfirmed = false;
    });
  }

  Future<void> _submit() async {
    final catalog = _catalog;
    final lineId = _lineId;
    if (_publishing ||
        _requiresReopen ||
        _staleCheckpoint ||
        catalog == null ||
        lineId == null ||
        !_fixedContextIsCurrent ||
        _selectedLocaleBlocked ||
        (_replacementConfirmationRequired && !_replacementConfirmed) ||
        !(_formKey.currentState?.validate() ?? false)) {
      return;
    }

    final Revision3VoiceTakeAuthoringInput input;
    try {
      input = Revision3VoiceTakeAuthoringInput(
        lineId: lineId,
        locale: _locale.text,
        sourcePath: _source.text,
        takeDisplayName: _takeName.text,
        status: _status,
        selectTake: _selectTake,
        dialogText: null,
      );
    } on FormatException {
      setState(() => _error = widget.copy.invalidForm);
      return;
    }

    setState(() {
      _publishing = true;
      _publicationStarted = true;
      _error = null;
    });
    var completed = false;
    try {
      final publication = await widget.service.publish(
        checkpoint: catalog,
        input: input,
      );
      if (!mounted) return;
      completed = true;
      Navigator.of(context).pop(publication);
    } on Revision3VoiceTakeRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _requiresReopen = true;
        _error = widget.copy.requiresReopen;
      });
    } on Revision3VoiceTakeStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _staleCheckpoint = true;
        _error = widget.copy.staleCheckpoint;
      });
    } on ModFfiException catch (error) {
      if (!mounted) return;
      setState(() => _error = widget.copy.importError(error.code));
    } catch (_) {
      if (!mounted) return;
      setState(() => _error = widget.copy.saveFailed);
    } finally {
      if (mounted && !completed) {
        setState(() {
          _publishing = false;
          _publicationStarted = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final blocked = _requiresReopen || _staleCheckpoint;
    final busy = _loading || _publishing || _picking || _previewing;
    return PopScope(
      canPop: !_publicationStarted,
      child: AlertDialog(
        key: const Key('revision3-voice-wizard'),
        title: Row(
          children: [
            const Icon(Icons.record_voice_over_outlined),
            const SizedBox(width: 10),
            Expanded(child: Text(widget.copy.title)),
          ],
        ),
        content: SizedBox(
          width: 680,
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 680),
            child: SingleChildScrollView(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  _VoiceBoundaryBanner(copy: widget.copy),
                  const SizedBox(height: 16),
                  if (_publishing) ...[
                    _VoiceLiveStatus(
                      key: const Key('revision3-voice-saving-status'),
                      message: widget.copy.savingStatus,
                    ),
                    const SizedBox(height: 16),
                  ],
                  if (_error case final error?) ...[
                    _VoiceMessage(
                      key: const Key('revision3-voice-error'),
                      message: error,
                    ),
                    const SizedBox(height: 16),
                  ],
                  if (_loading)
                    Semantics(
                      liveRegion: true,
                      label: widget.copy.loading,
                      child: const Padding(
                        padding: EdgeInsets.symmetric(vertical: 40),
                        child: Center(
                          child: CircularProgressIndicator(
                            key: Key('revision3-voice-loading'),
                          ),
                        ),
                      ),
                    )
                  else if (_catalog == null)
                    Center(
                      child: OutlinedButton.icon(
                        key: const Key('revision3-voice-retry'),
                        onPressed: _loadCatalog,
                        icon: const Icon(Icons.refresh),
                        label: Text(widget.copy.refreshDialogLines),
                      ),
                    )
                  else if (_fixedContextInvalid)
                    Center(
                      child: OutlinedButton.icon(
                        key: const Key('revision3-voice-fixed-context-retry'),
                        onPressed: blocked ? null : _loadCatalog,
                        icon: const Icon(Icons.refresh),
                        label: Text(widget.copy.refreshVoiceContext),
                      ),
                    )
                  else
                    _buildForm(enabled: !busy && !blocked),
                ],
              ),
            ),
          ),
        ),
        actions: [
          TextButton(
            key: const Key('revision3-voice-cancel'),
            onPressed: _publicationStarted
                ? null
                : () => Navigator.of(context).pop(),
            child: Text(blocked ? widget.copy.close : widget.copy.cancel),
          ),
          FilledButton.icon(
            key: const Key('revision3-voice-submit'),
            onPressed:
                busy ||
                    _catalog == null ||
                    _lineId == null ||
                    !_fixedContextIsCurrent ||
                    blocked ||
                    _selectedLocaleBlocked ||
                    (_replacementConfirmationRequired && !_replacementConfirmed)
                ? null
                : _submit,
            icon: _publishing
                ? const SizedBox.square(
                    dimension: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.library_add_outlined),
            label: Text(
              _publishing ? widget.copy.savingAction : widget.copy.submit,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildForm({required bool enabled}) {
    final catalog = _catalog!;
    return Form(
      key: _formKey,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (widget.fixedContext)
            _VoiceFixedContextBreadcrumb(
              lineLabel: _selectedLine!.displayLabel,
              locale: _locale.text,
              copy: widget.copy,
            )
          else
            RawAutocomplete<Revision3VoiceDialogLineChoice>(
              key: ValueKey('revision3-voice-line-$_catalogEpoch'),
              initialValue: TextEditingValue(
                text: _selectedLine?.displayLabel ?? '',
              ),
              displayStringForOption: (line) => line.displayLabel,
              optionsBuilder: (value) {
                final query = value.text.trim();
                if (query.isEmpty) {
                  return const <Revision3VoiceDialogLineChoice>[];
                }
                return catalog.lines
                    .where((line) => line.matches(query))
                    .take(50);
              },
              onSelected: _selectLine,
              fieldViewBuilder:
                  (context, controller, focusNode, onFieldSubmitted) =>
                      TextFormField(
                        key: const Key('revision3-voice-line-search'),
                        controller: controller,
                        focusNode: focusNode,
                        enabled: enabled,
                        decoration: InputDecoration(
                          labelText: widget.copy.lineLabel,
                          hintText: widget.copy.lineHint,
                          helperText: widget.copy.lineHelper,
                          border: const OutlineInputBorder(),
                        ),
                        onChanged: _clearChangedLineSearch,
                        onFieldSubmitted: (_) => onFieldSubmitted(),
                        validator: (_) =>
                            _lineId == null ? widget.copy.lineRequired : null,
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
                        key: const Key('revision3-voice-line-results'),
                        padding: EdgeInsets.zero,
                        shrinkWrap: true,
                        itemCount: bounded.length,
                        itemBuilder: (context, index) {
                          final line = bounded[index];
                          return ListTile(
                            title: Text(line.displayLabel),
                            onTap: () => onSelected(line),
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
            TextFormField(
              key: const Key('revision3-voice-locale'),
              controller: _locale,
              enabled: enabled,
              maxLength: 35,
              decoration: InputDecoration(
                labelText: widget.copy.localeLabel,
                hintText: widget.copy.localeHint,
                helperText: widget.copy.localeHelper,
                border: const OutlineInputBorder(),
              ),
              validator: (value) => _validateLocale(value, widget.copy),
              onChanged: (value) {
                setState(() {
                  _selectTake = false;
                  _replacementConfirmed = false;
                });
              },
            ),
          if (!widget.fixedContext && catalog.suggestedLocales.isNotEmpty) ...[
            Wrap(
              spacing: 8,
              runSpacing: 4,
              children: [
                for (final locale in catalog.suggestedLocales)
                  ChoiceChip(
                    key: Key('revision3-voice-locale-$locale'),
                    label: Text(locale),
                    selected: _locale.text.trim() == locale,
                    onSelected: enabled
                        ? (selected) {
                            if (selected && _locale.text != locale) {
                              _changeLocale(locale);
                            }
                          }
                        : null,
                  ),
              ],
            ),
            const SizedBox(height: 14),
          ],
          if (_selectedLine != null &&
              revision3VoiceLocaleIsCanonical(_locale.text.trim())) ...[
            _VoiceSlotSummary(
              summary: _selectedSlotSummary,
              blocked: _selectedLocaleBlocked,
              copy: widget.copy,
            ),
            const SizedBox(height: 14),
          ],
          TextFormField(
            key: const Key('revision3-voice-source'),
            controller: _source,
            enabled: enabled,
            decoration: InputDecoration(
              labelText: widget.copy.oggLabel,
              helperText: widget.copy.oggHelper,
              border: const OutlineInputBorder(),
              suffixIcon: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  IconButton(
                    key: const Key('revision3-voice-preview'),
                    tooltip: widget.copy.previewTooltip,
                    onPressed:
                        enabled &&
                            _validateSource(_source.text, widget.copy) == null
                        ? _previewSource
                        : null,
                    icon: _previewing
                        ? const SizedBox.square(
                            dimension: 18,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.play_arrow),
                  ),
                  IconButton(
                    key: const Key('revision3-voice-browse'),
                    tooltip: widget.copy.browseTooltip,
                    onPressed: enabled ? _pickSource : null,
                    icon: const Icon(Icons.folder_open),
                  ),
                ],
              ),
            ),
            onChanged: (_) => setState(() => _error = null),
            validator: (value) => _validateSource(value, widget.copy),
          ),
          const SizedBox(height: 14),
          TextFormField(
            key: const Key('revision3-voice-take-name'),
            controller: _takeName,
            enabled: enabled,
            maxLength: 256,
            decoration: InputDecoration(
              labelText: widget.copy.takeNameLabel,
              hintText: widget.copy.takeNameHint,
              helperText: widget.copy.takeNameHelper,
              border: const OutlineInputBorder(),
            ),
            onChanged: (_) => _takeNameWasManuallyEdited = true,
            validator: (value) => (value?.trim().isEmpty ?? true)
                ? widget.copy.takeNameRequired
                : utf8.encode(value!.trim()).length > 256
                ? widget.copy.takeNameTooLong
                : null,
          ),
          const SizedBox(height: 14),
          DropdownButtonFormField<AuthoringRevision3VoiceTakeStatus>(
            key: const Key('revision3-voice-status'),
            initialValue: _status,
            decoration: InputDecoration(
              labelText: widget.copy.statusLabel,
              helperText: widget.copy.statusHelper,
              border: const OutlineInputBorder(),
            ),
            items: [
              for (final status in AuthoringRevision3VoiceTakeStatus.values)
                DropdownMenuItem(
                  value: status,
                  child: Text(widget.copy.status(status)),
                ),
            ],
            onChanged: enabled
                ? (value) {
                    if (value == null) return;
                    setState(() {
                      _status = value;
                      if (value != AuthoringRevision3VoiceTakeStatus.approved) {
                        _selectTake = false;
                      }
                      _replacementConfirmed = false;
                    });
                  }
                : null,
          ),
          CheckboxListTile(
            key: const Key('revision3-voice-select'),
            contentPadding: EdgeInsets.zero,
            value: _selectTake,
            title: Text(widget.copy.selectTakeTitle),
            subtitle: Text(widget.copy.selectTakeSubtitle),
            onChanged:
                enabled && _status == AuthoringRevision3VoiceTakeStatus.approved
                ? (value) => setState(() {
                    _selectTake = value ?? false;
                    _replacementConfirmed = false;
                  })
                : null,
          ),
          if (_replacementConfirmationRequired) ...[
            const SizedBox(height: 8),
            _VoiceReplacementWarning(
              confirmed: _replacementConfirmed,
              enabled: enabled,
              copy: widget.copy,
              onChanged: (value) =>
                  setState(() => _replacementConfirmed = value),
            ),
          ],
          const Divider(height: 24),
          _VoiceLocalizationPreservedNotice(copy: widget.copy),
        ],
      ),
    );
  }
}

class _VoiceFixedContextBreadcrumb extends StatelessWidget {
  const _VoiceFixedContextBreadcrumb({
    required this.lineLabel,
    required this.locale,
    required this.copy,
  });

  final String lineLabel;
  final String locale;
  final Revision3VoiceTakeDialogCopy copy;

  @override
  Widget build(BuildContext context) => Container(
    key: const Key('revision3-voice-fixed-context'),
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

class _VoiceSlotSummary extends StatelessWidget {
  const _VoiceSlotSummary({
    required this.summary,
    required this.blocked,
    required this.copy,
  });

  final Revision3VoiceExistingSlotSummary? summary;
  final bool blocked;
  final Revision3VoiceTakeDialogCopy copy;

  @override
  Widget build(BuildContext context) {
    final value = summary;
    final message = blocked
        ? copy.slotBlocked
        : value == null
        ? copy.slotMissing
        : copy.slotExisting(
            value.candidateCount,
            selected: value.hasSelectedTake,
          );
    return Container(
      key: const Key('revision3-voice-slot-summary'),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Icon(Icons.library_music_outlined, size: 20),
          const SizedBox(width: 10),
          Expanded(child: Text(message)),
        ],
      ),
    );
  }
}

class _VoiceReplacementWarning extends StatelessWidget {
  const _VoiceReplacementWarning({
    required this.confirmed,
    required this.enabled,
    required this.onChanged,
    required this.copy,
  });

  final bool confirmed;
  final bool enabled;
  final ValueChanged<bool> onChanged;
  final Revision3VoiceTakeDialogCopy copy;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      key: const Key('revision3-voice-replacement-warning'),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: scheme.errorContainer,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            copy.replacementTitle,
            style: Theme.of(
              context,
            ).textTheme.titleSmall?.copyWith(color: scheme.onErrorContainer),
          ),
          const SizedBox(height: 4),
          Text(
            copy.replacementDescription,
            style: TextStyle(color: scheme.onErrorContainer),
          ),
          Material(
            type: MaterialType.transparency,
            child: CheckboxListTile(
              key: const Key('revision3-voice-confirm-replacement'),
              contentPadding: EdgeInsets.zero,
              value: confirmed,
              title: Text(copy.replacementConfirm),
              controlAffinity: ListTileControlAffinity.leading,
              onChanged: enabled ? (value) => onChanged(value ?? false) : null,
            ),
          ),
        ],
      ),
    );
  }
}

class _VoiceLocalizationPreservedNotice extends StatelessWidget {
  const _VoiceLocalizationPreservedNotice({required this.copy});

  final Revision3VoiceTakeDialogCopy copy;

  @override
  Widget build(BuildContext context) => ListTile(
    key: const Key('revision3-voice-localization-preserved'),
    contentPadding: EdgeInsets.zero,
    leading: const Icon(Icons.lock_outline),
    title: Text(copy.localizationPreservedTitle),
    subtitle: Text(copy.localizationPreservedDescription),
  );
}

class _VoiceBoundaryBanner extends StatelessWidget {
  const _VoiceBoundaryBanner({required this.copy});

  final Revision3VoiceTakeDialogCopy copy;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      key: const Key('revision3-voice-boundary'),
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
              Chip(label: Text(copy.boundaryProjectOnly)),
              Chip(label: Text(copy.boundaryNotInGame)),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            copy.boundaryDescription,
            style: TextStyle(color: scheme.onSecondaryContainer),
          ),
        ],
      ),
    );
  }
}

class _VoiceLiveStatus extends StatelessWidget {
  const _VoiceLiveStatus({required this.message, super.key});

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

class _VoiceMessage extends StatelessWidget {
  const _VoiceMessage({required this.message, super.key});

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

String? _validateLocale(String? value, Revision3VoiceTakeDialogCopy copy) {
  final normalized = value?.trim() ?? '';
  if (normalized.isEmpty) return copy.localeRequired;
  if (!revision3VoiceLocaleIsCanonical(normalized)) {
    return copy.localeInvalid;
  }
  return null;
}

Future<String?> _pickRevision3VoiceOgg(
  Revision3VoiceTakeDialogCopy copy,
) async {
  final file = await openFile(
    acceptedTypeGroups: [
      XTypeGroup(label: copy.pickerTypeLabel, extensions: const ['ogg']),
    ],
  );
  return file?.path;
}

Future<void> _previewRevision3VoiceOgg(
  String path,
  Revision3VoiceTakeDialogCopy copy,
) async {
  _validateRevision3VoicePreviewPath(path, copy);
  final opened = await launchUrl(
    Uri.file(path, windows: Platform.isWindows),
    mode: LaunchMode.externalApplication,
  );
  if (!opened) {
    throw FileSystemException(copy.previewLauncherRejected);
  }
}

void _validateRevision3VoicePreviewPath(
  String path,
  Revision3VoiceTakeDialogCopy copy,
) {
  if (path.isEmpty ||
      path.trim() != path ||
      !path.toLowerCase().endsWith('.ogg') ||
      path.runes.any((rune) => rune < 0x20 || (rune >= 0x7f && rune <= 0x9f))) {
    throw FormatException(copy.previewSourceInvalid);
  }
  if (FileSystemEntity.typeSync(path, followLinks: false) !=
      FileSystemEntityType.file) {
    throw FormatException(copy.previewSourceNotFile);
  }
}

String? _validateSource(String? value, Revision3VoiceTakeDialogCopy copy) {
  final source = value ?? '';
  if (source.isEmpty) return copy.sourceRequired;
  if (source.trim() != source || !source.toLowerCase().endsWith('.ogg')) {
    return copy.sourceExtension;
  }
  return null;
}
