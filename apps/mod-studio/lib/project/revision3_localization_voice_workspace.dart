import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';

import 'revision3_dialog_localization_authoring.dart';
import 'revision3_voice_authoring.dart';
import 'revision3_voice_production_card.dart';

typedef Revision3LocalizationVoiceAction = FutureOr<void> Function();
typedef Revision3LocalizationVoiceContextAction =
    FutureOr<void> Function({
      required String initialLineId,
      required String initialLocale,
    });
typedef Revision3LocalizationPublished =
    void Function(Revision3DialogLocalizationEditPublication publication);
typedef Revision3LocalizationVoiceCatalogLoader =
    Future<Revision3VoiceCatalog> Function();

enum _UnsavedExternalActionDecision {
  keepEditing,
  discardAndContinue,
  saveAndContinue,
}

enum _VoiceContextActionKind { addTake, manageTakes, resolveTarget }

enum _PublicationAuthorityStatus { waiting, ready, invalid }

final class _LocalizationSaveCompletion {
  const _LocalizationSaveCompletion({
    required this.publication,
    required this.originProjectId,
    required this.originProjectRevision,
    required this.originCheckpointIdentity,
    required this.choiceStableKey,
    required this.publicationCheckpointIdentity,
  });

  final Revision3DialogLocalizationEditPublication publication;
  final String originProjectId;
  final int originProjectRevision;
  final Object originCheckpointIdentity;
  final String choiceStableKey;
  final Object? publicationCheckpointIdentity;
}

/// Author-facing copy for the coherent Localization & Voice workspace.
///
/// The widget deliberately receives all copy from its host. It owns no game,
/// build, deployment, or runtime authority and never exposes technical project
/// identities in its normal presentation.
@immutable
final class Revision3LocalizationVoiceWorkspaceCopy {
  const Revision3LocalizationVoiceWorkspaceCopy({
    required this.title,
    required this.description,
    required this.projectTextsLabel,
    required this.searchLabel,
    required this.refreshLabel,
    required this.newLineLabel,
    required this.addVoiceLabel,
    required this.manageVoiceLabel,
    required this.resolveVoiceLabel,
    required this.loadingLabel,
    required this.emptyTitle,
    required this.emptyDescription,
    required this.loadFailedTitle,
    required this.retryLabel,
    required this.selectTextLabel,
    required this.languagesLabel,
    required this.usedByLinesLabel,
    required this.voiceContextTitle,
    required this.voiceSelectLineLabel,
    required this.voiceSetupExistsLabel,
    required this.voiceSetupMissingLabel,
    required this.noLineLabel,
    required this.speakerLabel,
    required this.addLanguageLabel,
    required this.removeLanguageLabel,
    required this.languageCodeLabel,
    required this.languageCodeHint,
    required this.languageExistsMessage,
    required this.dialogTextLabel,
    required this.addLabel,
    required this.cancelLabel,
    required this.saveLabel,
    required this.savingLabel,
    required this.savedLabel,
    required this.voiceLockedLabel,
    required this.voiceSlotRemovalLockedLabel,
    required this.minimumLanguageLockedLabel,
    required this.sharedTextNotice,
    required this.offlineNotice,
    required this.unsavedTitle,
    required this.unsavedDescription,
    required this.discardLabel,
    required this.keepEditingLabel,
    required this.voiceUnsavedTitle,
    required this.voiceUnsavedDescription,
    required this.discardAndContinueLabel,
    required this.saveAndContinueLabel,
    required this.staleMessage,
    required this.reopenMessage,
    required this.invalidInputMessage,
    required this.genericFailureMessage,
    required this.voiceActionFailedMessage,
    this.importVoiceFolderLabel = 'Import recordings folder',
  });

  const Revision3LocalizationVoiceWorkspaceCopy.english()
    : this(
        title: 'Localization & Voice',
        description:
            'Write and translate project dialog, then review each language\'s takes, selection, and target in the same workspace.',
        projectTextsLabel: 'Project texts',
        searchLabel: 'Search project texts',
        refreshLabel: 'Refresh',
        newLineLabel: 'New dialog line',
        addVoiceLabel: 'Add take for any line',
        manageVoiceLabel: 'Manage takes for any line',
        resolveVoiceLabel: 'Resolve target for any line',
        loadingLabel: 'Opening project texts',
        emptyTitle: 'No project text yet',
        emptyDescription:
            'Create a dialog line to start writing and translating text.',
        loadFailedTitle: 'Project texts could not be opened',
        retryLabel: 'Try again',
        selectTextLabel: 'Select a project text to edit',
        languagesLabel: 'Languages',
        usedByLinesLabel: 'Used by dialog lines',
        voiceContextTitle: 'Voice for this dialog line',
        voiceSelectLineLabel: 'Select a dialog line above',
        voiceSetupExistsLabel: 'setup exists',
        voiceSetupMissingLabel: 'no setup yet',
        noLineLabel: 'Not used by a dialog line yet',
        speakerLabel: 'Speaker label',
        addLanguageLabel: 'Add language',
        removeLanguageLabel: 'Remove language',
        languageCodeLabel: 'Language',
        languageCodeHint: 'For example de, en, or pt-BR',
        languageExistsMessage: 'This language is already present.',
        dialogTextLabel: 'Dialog text',
        addLabel: 'Add',
        cancelLabel: 'Cancel',
        saveLabel: 'Save changes',
        savingLabel: 'Saving changes',
        savedLabel: 'Project text saved',
        voiceLockedLabel:
            'This text has recorded voice takes, so its transcript is locked in this editor.',
        voiceSlotRemovalLockedLabel:
            'This language is connected to a Voice slot and cannot be removed here.',
        minimumLanguageLockedLabel:
            'Keep at least one language for this project text.',
        sharedTextNotice:
            'This project text is shared. Saving changes updates every listed dialog line.',
        offlineNotice:
            'Changes are saved only to this managed project. Build and in-game behavior remain separate.',
        unsavedTitle: 'Discard unsaved changes?',
        unsavedDescription:
            'You changed this project text. Switching now would discard those edits.',
        discardLabel: 'Discard changes',
        keepEditingLabel: 'Keep editing',
        voiceUnsavedTitle: 'Save text before continuing?',
        voiceUnsavedDescription:
            'Save these text changes and continue directly to the selected action, keep editing, or deliberately discard the text changes.',
        discardAndContinueLabel: 'Discard and continue',
        saveAndContinueLabel: 'Save and continue',
        staleMessage:
            'The project changed while this text was open. Refresh and try again.',
        reopenMessage:
            'The project must be reopened before text editing can continue.',
        invalidInputMessage:
            'Check that every language and dialog text is valid and not empty.',
        genericFailureMessage: 'The project text could not be saved.',
        voiceActionFailedMessage:
            'The selected action did not finish cleanly. Refresh the project before trying again; the exact current project will show whether a change was published. This workspace did not change game or save files.',
      );

  final String title;
  final String description;
  final String projectTextsLabel;
  final String searchLabel;
  final String refreshLabel;
  final String newLineLabel;
  final String addVoiceLabel;
  final String manageVoiceLabel;
  final String resolveVoiceLabel;
  final String loadingLabel;
  final String emptyTitle;
  final String emptyDescription;
  final String loadFailedTitle;
  final String retryLabel;
  final String selectTextLabel;
  final String languagesLabel;
  final String usedByLinesLabel;
  final String voiceContextTitle;
  final String voiceSelectLineLabel;
  final String voiceSetupExistsLabel;
  final String voiceSetupMissingLabel;
  final String noLineLabel;
  final String speakerLabel;
  final String addLanguageLabel;
  final String removeLanguageLabel;
  final String languageCodeLabel;
  final String languageCodeHint;
  final String languageExistsMessage;
  final String dialogTextLabel;
  final String addLabel;
  final String cancelLabel;
  final String saveLabel;
  final String savingLabel;
  final String savedLabel;
  final String voiceLockedLabel;
  final String voiceSlotRemovalLockedLabel;
  final String minimumLanguageLockedLabel;
  final String sharedTextNotice;
  final String offlineNotice;
  final String unsavedTitle;
  final String unsavedDescription;
  final String discardLabel;
  final String keepEditingLabel;
  final String voiceUnsavedTitle;
  final String voiceUnsavedDescription;
  final String discardAndContinueLabel;
  final String saveAndContinueLabel;
  final String staleMessage;
  final String reopenMessage;
  final String invalidInputMessage;
  final String genericFailureMessage;
  final String voiceActionFailedMessage;
  final String importVoiceFolderLabel;
}

/// Direct, responsive project-text workspace.
///
/// Unlike the former section landing page, this surface keeps discovery,
/// selection, full-text editing, exact Voice production facts, and publication
/// together. Mutating Voice actions are supplied by the host and remain
/// separate bounded workflows.
class Revision3LocalizationVoiceWorkspace extends StatefulWidget {
  const Revision3LocalizationVoiceWorkspace({
    required this.projectId,
    required this.projectRevision,
    required this.projectCheckpointIdentity,
    required this.service,
    required this.copy,
    this.loadVoiceCatalog,
    this.voiceProductionCopy = Revision3VoiceProductionCardCopy.english,
    this.onCreateDialogLine,
    this.onAddVoiceTake,
    this.onImportVoiceFolder,
    this.onManageVoiceTakes,
    this.onResolveVoiceTarget,
    this.onAddVoiceTakeFor,
    this.onManageVoiceTakesFor,
    this.onResolveVoiceTargetFor,
    this.onPublished,
    this.onDirtyChanged,
    this.notice,
    super.key,
  });

  final String projectId;
  final int projectRevision;

  /// Opaque exact-checkpoint token used only for lifecycle invalidation.
  ///
  /// Its equality must change whenever the project head changes, including a
  /// head rebind that keeps the same public project revision. It is never
  /// rendered or forwarded as mutation authority.
  final Object projectCheckpointIdentity;
  final Revision3DialogLocalizationEditAuthoringService service;
  final Revision3LocalizationVoiceWorkspaceCopy copy;
  final Revision3LocalizationVoiceCatalogLoader? loadVoiceCatalog;
  final Revision3VoiceProductionCardCopy voiceProductionCopy;
  final Revision3LocalizationVoiceAction? onCreateDialogLine;
  final Revision3LocalizationVoiceAction? onAddVoiceTake;
  final Revision3LocalizationVoiceAction? onImportVoiceFolder;
  final Revision3LocalizationVoiceAction? onManageVoiceTakes;
  final Revision3LocalizationVoiceAction? onResolveVoiceTarget;
  final Revision3LocalizationVoiceContextAction? onAddVoiceTakeFor;
  final Revision3LocalizationVoiceContextAction? onManageVoiceTakesFor;
  final Revision3LocalizationVoiceContextAction? onResolveVoiceTargetFor;
  final Revision3LocalizationPublished? onPublished;
  final ValueChanged<bool>? onDirtyChanged;
  final String? notice;

  @override
  State<Revision3LocalizationVoiceWorkspace> createState() =>
      _Revision3LocalizationVoiceWorkspaceState();
}

class _Revision3LocalizationVoiceWorkspaceState
    extends State<Revision3LocalizationVoiceWorkspace> {
  final _search = TextEditingController();
  Revision3DialogLocalizationEditCatalog? _catalog;
  Revision3VoiceCatalog? _voiceCatalog;
  Revision3DialogLocalizationEditSeed? _seed;
  Object? _catalogError;
  Object? _voiceCatalogError;
  Object? _seedError;
  String? _selectedKey;
  String? _voiceLineId;
  String? _voiceLocale;
  Map<String, TextEditingController> _texts = {};
  Map<String, String> _baseline = const {};
  bool _loadingCatalog = false;
  bool _loadingVoiceCatalog = false;
  bool _loadingSeed = false;
  bool _saving = false;
  int? _runningExternalActionOwner;
  bool _showEditorOnCompact = false;
  bool _checkpointChangedWhileDirty = false;
  bool _lastReportedDirty = false;
  int _catalogEpoch = 0;
  int _voiceCatalogEpoch = 0;
  int _seedEpoch = 0;
  int _saveEpoch = 0;
  int _externalActionEpoch = 0;
  Future<void>? _catalogReloadFuture;
  Future<void>? _voiceCatalogReloadFuture;

  @override
  void initState() {
    super.initState();
    _search.addListener(_searchChanged);
    unawaited(_reloadCatalog());
  }

  @override
  void didUpdateWidget(
    covariant Revision3LocalizationVoiceWorkspace oldWidget,
  ) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.projectId != widget.projectId) {
      _saveEpoch++;
      _saving = false;
      _invalidateExternalAction();
      _selectedKey = null;
      _voiceLineId = null;
      _voiceLocale = null;
      _showEditorOnCompact = false;
      _checkpointChangedWhileDirty = false;
      _voiceCatalog = null;
      _voiceCatalogError = null;
      _disposeTextControllers();
      _reportDirty();
      unawaited(_reloadCatalog(clearCurrent: true));
    } else if (oldWidget.projectRevision != widget.projectRevision ||
        oldWidget.projectCheckpointIdentity !=
            widget.projectCheckpointIdentity) {
      _invalidateVoiceCatalog();
      if (_saving) {
        // Keep the submitted draft frozen until the pending publication tells
        // us whether this is its exact rebind or unrelated checkpoint drift.
      } else if (_dirty) {
        _checkpointChangedWhileDirty = true;
      } else {
        unawaited(_reloadCatalog());
      }
    } else if (oldWidget.loadVoiceCatalog == null &&
        widget.loadVoiceCatalog != null) {
      unawaited(_reloadVoiceCatalog());
    }
    if (!identical(oldWidget.onDirtyChanged, widget.onDirtyChanged)) {
      oldWidget.onDirtyChanged?.call(false);
      _reportDirty(force: true);
    }
  }

  @override
  void dispose() {
    _catalogEpoch++;
    _voiceCatalogEpoch++;
    _seedEpoch++;
    _saveEpoch++;
    _invalidateExternalAction();
    _search
      ..removeListener(_searchChanged)
      ..dispose();
    _disposeTextControllers();
    _lastReportedDirty = false;
    widget.onDirtyChanged?.call(false);
    super.dispose();
  }

  void _searchChanged() => setState(() {});

  bool get _runningExternalAction => _runningExternalActionOwner != null;

  bool get _contextMutationBlocked =>
      _saving || _loadingCatalog || _runningExternalAction;

  int? _beginExternalAction() {
    if (_runningExternalAction) return null;
    final owner = ++_externalActionEpoch;
    setState(() => _runningExternalActionOwner = owner);
    return owner;
  }

  bool _externalActionOwnerIsCurrent(int owner) =>
      mounted && _runningExternalActionOwner == owner;

  void _endExternalAction(int owner) {
    if (!_externalActionOwnerIsCurrent(owner)) return;
    setState(() => _runningExternalActionOwner = null);
  }

  void _invalidateExternalAction() {
    _externalActionEpoch++;
    _runningExternalActionOwner = null;
  }

  void _disposeTextControllers() {
    for (final controller in _texts.values) {
      controller.dispose();
    }
    _texts = {};
    _baseline = const {};
  }

  void _invalidateVoiceCatalog() {
    _voiceCatalogEpoch++;
    _voiceCatalog = null;
    _voiceCatalogError = null;
    _loadingVoiceCatalog = false;
  }

  Future<void> _reloadCatalog({bool clearCurrent = false}) {
    final reload = _reloadCatalogImpl(clearCurrent: clearCurrent);
    _catalogReloadFuture = reload;
    return reload;
  }

  Future<void> _reloadCatalogImpl({bool clearCurrent = false}) async {
    unawaited(_reloadVoiceCatalog(clearCurrent: clearCurrent));
    final epoch = ++_catalogEpoch;
    ++_seedEpoch;
    setState(() {
      _loadingCatalog = true;
      _catalogError = null;
      _seedError = null;
      _loadingSeed = false;
      if (clearCurrent) {
        _catalog = null;
        _seed = null;
      }
    });
    try {
      final catalog = await widget.service.loadCatalog();
      if (!mounted || epoch != _catalogEpoch) return;
      if (catalog.projectId != widget.projectId ||
          catalog.projectRevision != widget.projectRevision) {
        throw const Revision3DialogLocalizationEditStaleCheckpointException();
      }
      var selectedKey = _selectedKey;
      if (selectedKey == null ||
          catalog.choiceByStableKey(selectedKey) == null) {
        selectedKey = catalog.choices.firstOrNull?.stableKey;
      }
      setState(() {
        _catalog = catalog;
        _selectedKey = selectedKey;
        _loadingCatalog = false;
      });
      if (selectedKey == null) {
        _replaceSeed(null);
        setState(() => _checkpointChangedWhileDirty = false);
      } else {
        await _loadSeed(catalog, selectedKey);
      }
    } catch (error) {
      if (!mounted || epoch != _catalogEpoch) return;
      setState(() {
        _catalogError = error;
        _loadingCatalog = false;
      });
    }
  }

  Future<void> _reloadVoiceCatalog({bool clearCurrent = false}) {
    final reload = _reloadVoiceCatalogImpl(clearCurrent: clearCurrent);
    _voiceCatalogReloadFuture = reload;
    return reload;
  }

  Future<void> _reloadVoiceCatalogImpl({bool clearCurrent = false}) async {
    final loader = widget.loadVoiceCatalog;
    final epoch = ++_voiceCatalogEpoch;
    if (loader == null) {
      if (!mounted) return;
      setState(() {
        _voiceCatalog = null;
        _voiceCatalogError = null;
        _loadingVoiceCatalog = false;
      });
      return;
    }
    setState(() {
      _loadingVoiceCatalog = true;
      _voiceCatalogError = null;
      if (clearCurrent) _voiceCatalog = null;
    });
    try {
      final catalog = await loader();
      if (!mounted || epoch != _voiceCatalogEpoch) return;
      if (catalog.projectId != widget.projectId ||
          catalog.projectRevision != widget.projectRevision) {
        throw const Revision3DialogLocalizationEditStaleCheckpointException();
      }
      setState(() {
        _voiceCatalog = catalog;
        _loadingVoiceCatalog = false;
      });
    } catch (error) {
      if (!mounted || epoch != _voiceCatalogEpoch) return;
      setState(() {
        _voiceCatalog = null;
        _voiceCatalogError = error;
        _loadingVoiceCatalog = false;
      });
    }
  }

  Future<void> _loadSeed(
    Revision3DialogLocalizationEditCatalog catalog,
    String stableKey,
  ) async {
    final choice = catalog.choiceByStableKey(stableKey);
    if (choice == null) return;
    final epoch = ++_seedEpoch;
    setState(() {
      _loadingSeed = true;
      _seedError = null;
      _seed = null;
      _disposeTextControllers();
    });
    _reportDirty();
    try {
      final seed = await widget.service.loadSeed(
        catalog: catalog,
        choice: choice,
      );
      if (!mounted || epoch != _seedEpoch || _selectedKey != stableKey) return;
      _replaceSeed(seed);
      setState(() {
        _loadingSeed = false;
        _checkpointChangedWhileDirty = false;
      });
    } catch (error) {
      if (!mounted || epoch != _seedEpoch || _selectedKey != stableKey) return;
      setState(() {
        _seedError = error;
        _loadingSeed = false;
      });
    }
  }

  void _replaceSeed(Revision3DialogLocalizationEditSeed? seed) {
    _disposeTextControllers();
    _seed = seed;
    if (seed == null) {
      _voiceLineId = null;
      _voiceLocale = null;
    } else {
      final retainedLine = seed.lineBacklinks
          .where((line) => line.lineId == _voiceLineId)
          .firstOrNull;
      final selectedLine =
          retainedLine ??
          (seed.lineBacklinks.length == 1 ? seed.lineBacklinks.single : null);
      _voiceLineId = selectedLine?.lineId;
      final locales = seed.locales.map((locale) => locale.locale).toList();
      if (!locales.contains(_voiceLocale)) {
        final slotted = selectedLine?.voiceSlotLocales
            .where(locales.contains)
            .firstOrNull;
        _voiceLocale = slotted ?? locales.firstOrNull;
      }
    }
    if (seed != null) {
      _baseline = Map<String, String>.unmodifiable({
        for (final locale in seed.locales) locale.locale: locale.text,
      });
      _texts = {
        for (final locale in seed.locales)
          locale.locale: TextEditingController(text: locale.text)
            ..addListener(_textChanged),
      };
    }
    _reportDirty();
  }

  void _textChanged() {
    if (!mounted) return;
    setState(() {});
    _reportDirty();
  }

  Map<String, String> get _currentTexts => Map<String, String>.fromEntries(
    (_texts.entries.toList(growable: false)
          ..sort((left, right) => left.key.compareTo(right.key)))
        .map((entry) => MapEntry(entry.key, entry.value.text)),
  );

  bool get _dirty => !_sameTexts(_baseline, _currentTexts);

  void _reportDirty({bool force = false}) {
    final dirty = _dirty;
    if (!force && dirty == _lastReportedDirty) return;
    _lastReportedDirty = dirty;
    widget.onDirtyChanged?.call(dirty);
  }

  Future<bool> _confirmDiscard() async {
    if (!_dirty || _saving) return !_saving;
    return await showDialog<bool>(
          context: context,
          builder: (context) => AlertDialog(
            title: Text(widget.copy.unsavedTitle),
            content: Text(widget.copy.unsavedDescription),
            actions: [
              TextButton(
                onPressed: () => Navigator.of(context).pop(false),
                child: Text(widget.copy.keepEditingLabel),
              ),
              FilledButton(
                onPressed: () => Navigator.of(context).pop(true),
                child: Text(widget.copy.discardLabel),
              ),
            ],
          ),
        ) ??
        false;
  }

  Future<_UnsavedExternalActionDecision> _confirmExternalAction() async {
    if (!_dirty || _saving) {
      return _saving
          ? _UnsavedExternalActionDecision.keepEditing
          : _UnsavedExternalActionDecision.discardAndContinue;
    }
    final canSave = _seed != null && !_checkpointChangedWhileDirty;
    return await showDialog<_UnsavedExternalActionDecision>(
          context: context,
          builder: (context) => AlertDialog(
            title: Text(widget.copy.voiceUnsavedTitle),
            content: Text(widget.copy.voiceUnsavedDescription),
            actions: [
              TextButton(
                onPressed: () => Navigator.of(
                  context,
                ).pop(_UnsavedExternalActionDecision.keepEditing),
                child: Text(widget.copy.keepEditingLabel),
              ),
              TextButton(
                onPressed: () => Navigator.of(
                  context,
                ).pop(_UnsavedExternalActionDecision.discardAndContinue),
                child: Text(widget.copy.discardAndContinueLabel),
              ),
              FilledButton(
                onPressed: canSave
                    ? () => Navigator.of(
                        context,
                      ).pop(_UnsavedExternalActionDecision.saveAndContinue)
                    : null,
                child: Text(widget.copy.saveAndContinueLabel),
              ),
            ],
          ),
        ) ??
        _UnsavedExternalActionDecision.keepEditing;
  }

  Future<void> _selectChoice(String stableKey) async {
    if (_contextMutationBlocked) return;
    if (_selectedKey == stableKey) {
      if (!_showEditorOnCompact) {
        setState(() => _showEditorOnCompact = true);
      }
      return;
    }
    if (!await _confirmDiscard() || !mounted || _contextMutationBlocked) return;
    if (_checkpointChangedWhileDirty) {
      setState(() {
        _showEditorOnCompact = false;
        _replaceSeed(null);
      });
      await _reloadCatalog(clearCurrent: true);
      return;
    }
    final catalog = _catalog;
    if (catalog == null || catalog.choiceByStableKey(stableKey) == null) return;
    setState(() {
      _selectedKey = stableKey;
      _showEditorOnCompact = true;
    });
    await _loadSeed(catalog, stableKey);
  }

  Future<void> _addLocale() async {
    if (_contextMutationBlocked) return;
    final catalogEpoch = _catalogEpoch;
    final seedEpoch = _seedEpoch;
    final added = await showDialog<_AddedLocale>(
      context: context,
      builder: (context) => _AddLocaleDialog(
        copy: widget.copy,
        existingLocales: Set<String>.unmodifiable(_texts.keys),
      ),
    );
    if (added == null || !mounted) return;
    if (_saving || _runningExternalAction) return;
    if (_loadingCatalog ||
        _checkpointChangedWhileDirty ||
        catalogEpoch != _catalogEpoch ||
        seedEpoch != _seedEpoch) {
      _showMessage(widget.copy.staleMessage);
      return;
    }
    if (_texts.containsKey(added.locale)) return;
    final controller = TextEditingController(text: added.text)
      ..addListener(_textChanged);
    setState(() => _texts[added.locale] = controller);
    _reportDirty();
  }

  void _removeLocale(Revision3DialogLocalizationLocaleSeed locale) {
    if (_contextMutationBlocked || !locale.canRemove || _texts.length <= 1) {
      return;
    }
    final controller = _texts.remove(locale.locale);
    controller
      ?..removeListener(_textChanged)
      ..dispose();
    if (_voiceLocale == locale.locale) {
      _voiceLocale = _texts.keys.firstOrNull;
    }
    setState(() {});
    _reportDirty();
  }

  Future<_LocalizationSaveCompletion?> _save() async {
    final seed = _seed;
    if (seed == null ||
        !_dirty ||
        _saving ||
        _loadingCatalog ||
        _checkpointChangedWhileDirty) {
      return null;
    }
    late final Revision3DialogLocalizationEditInput input;
    try {
      input = Revision3DialogLocalizationEditInput(texts: _currentTexts);
    } on FormatException {
      _showMessage(widget.copy.invalidInputMessage);
      return null;
    }
    final originProjectId = widget.projectId;
    final originProjectRevision = widget.projectRevision;
    final originCheckpointIdentity = widget.projectCheckpointIdentity;
    final originService = widget.service;
    final originOnPublished = widget.onPublished;
    final originCopy = widget.copy;
    final submittedTexts = Map<String, String>.unmodifiable(input.texts);
    final saveEpoch = ++_saveEpoch;
    bool saveIsActive() => mounted && _saveEpoch == saveEpoch;
    bool originIsCurrent() =>
        saveIsActive() &&
        widget.projectId == originProjectId &&
        widget.projectRevision == originProjectRevision &&
        widget.projectCheckpointIdentity == originCheckpointIdentity;
    void reportFailure(String originMessage) {
      if (!saveIsActive() || widget.projectId != originProjectId) return;
      if (originIsCurrent()) {
        _showMessage(originMessage);
        return;
      }
      _checkpointChangedWhileDirty = _dirty;
      _showMessage(widget.copy.staleMessage);
    }

    setState(() => _saving = true);
    try {
      final publication = await originService.publish(seed: seed, input: input);
      if (!saveIsActive() || widget.projectId != originProjectId) return null;
      final completedAtOrigin = originIsCurrent();
      final completedAtExactPublication =
          widget.projectRevision == publication.projectRevision &&
          widget.projectCheckpointIdentity != originCheckpointIdentity;
      if (!completedAtOrigin && !completedAtExactPublication) {
        _checkpointChangedWhileDirty = _dirty;
        _showMessage(widget.copy.staleMessage);
        return null;
      }
      _baseline = submittedTexts;
      _checkpointChangedWhileDirty = false;
      _reportDirty();
      _showMessage(
        completedAtOrigin ? originCopy.savedLabel : widget.copy.savedLabel,
      );
      final completion = _LocalizationSaveCompletion(
        publication: publication,
        originProjectId: originProjectId,
        originProjectRevision: originProjectRevision,
        originCheckpointIdentity: originCheckpointIdentity,
        choiceStableKey: seed.choice.stableKey,
        publicationCheckpointIdentity: completedAtExactPublication
            ? widget.projectCheckpointIdentity
            : null,
      );
      if (completedAtOrigin) {
        originOnPublished?.call(publication);
      } else {
        setState(() => _saving = false);
        await _reloadCatalog();
        final voiceReload = _voiceCatalogReloadFuture;
        if (voiceReload != null) await voiceReload;
      }
      return completion;
    } on Revision3DialogLocalizationEditRequiresReopenException {
      reportFailure(originCopy.reopenMessage);
    } on Revision3DialogLocalizationEditStaleCheckpointException {
      reportFailure(originCopy.staleMessage);
    } on Revision3DialogLocalizationEditLockedVoiceTextException {
      reportFailure(originCopy.voiceLockedLabel);
    } on FormatException {
      reportFailure(originCopy.invalidInputMessage);
    } catch (_) {
      reportFailure(originCopy.genericFailureMessage);
    } finally {
      if (saveIsActive() && widget.projectId == originProjectId && _saving) {
        setState(() => _saving = false);
      }
    }
    return null;
  }

  Object? _currentPublicationCheckpoint(_LocalizationSaveCompletion saved) {
    if (!mounted ||
        widget.projectId != saved.originProjectId ||
        widget.projectRevision != saved.publication.projectRevision ||
        widget.projectCheckpointIdentity == saved.originCheckpointIdentity) {
      return null;
    }
    return widget.projectCheckpointIdentity;
  }

  bool _hasPinnedPublicationCheckpoint(
    _LocalizationSaveCompletion saved,
    Object publicationCheckpointIdentity,
  ) =>
      mounted &&
      widget.projectId == saved.originProjectId &&
      widget.projectRevision == saved.publication.projectRevision &&
      widget.projectCheckpointIdentity == publicationCheckpointIdentity;

  _PublicationAuthorityStatus _publicationAuthorityStatus(
    _LocalizationSaveCompletion saved, {
    required Object publicationCheckpointIdentity,
    required bool requiresVoiceAuthority,
    _VoiceContextActionKind? contextKind,
    String? lineId,
    String? locale,
  }) {
    if (!_hasPinnedPublicationCheckpoint(
      saved,
      publicationCheckpointIdentity,
    )) {
      return _PublicationAuthorityStatus.invalid;
    }
    if (_loadingCatalog || _loadingSeed) {
      return _PublicationAuthorityStatus.waiting;
    }
    final catalog = _catalog;
    final seed = _seed;
    if (_catalogError != null ||
        _seedError != null ||
        catalog == null ||
        seed == null ||
        catalog.projectId != widget.projectId ||
        catalog.projectRevision != widget.projectRevision ||
        seed.choice.stableKey != saved.choiceStableKey) {
      return _PublicationAuthorityStatus.invalid;
    }
    if (!requiresVoiceAuthority) return _PublicationAuthorityStatus.ready;
    if (widget.loadVoiceCatalog == null || _loadingVoiceCatalog) {
      return _PublicationAuthorityStatus.waiting;
    }
    final voiceCatalog = _voiceCatalog;
    if (_voiceCatalogError != null ||
        voiceCatalog == null ||
        voiceCatalog.projectId != widget.projectId ||
        voiceCatalog.projectRevision != widget.projectRevision) {
      return _PublicationAuthorityStatus.invalid;
    }
    if (contextKind == null) return _PublicationAuthorityStatus.ready;
    if (lineId == null || locale == null) {
      return _PublicationAuthorityStatus.invalid;
    }
    final seedHasLine = seed.lineBacklinks.any((line) => line.lineId == lineId);
    final seedHasLocale = seed.locales.any((entry) => entry.locale == locale);
    final line = voiceCatalog.line(lineId);
    if (!seedHasLine ||
        !seedHasLocale ||
        line == null ||
        !voiceCatalog.suggestedLocales.contains(locale)) {
      return _PublicationAuthorityStatus.invalid;
    }
    final contextIsCurrent = switch (contextKind) {
      _VoiceContextActionKind.addTake => line.isLocaleAuthorable(locale),
      _VoiceContextActionKind.manageTakes =>
        line.slotSummaryForLocale(locale) != null,
      _VoiceContextActionKind.resolveTarget => line.isLocaleTargetable(locale),
    };
    return contextIsCurrent
        ? _PublicationAuthorityStatus.ready
        : _PublicationAuthorityStatus.invalid;
  }

  Future<Object?> _awaitPublicationAuthority(
    _LocalizationSaveCompletion saved, {
    required bool requiresVoiceAuthority,
    _VoiceContextActionKind? contextKind,
    String? lineId,
    String? locale,
  }) async {
    await WidgetsBinding.instance.endOfFrame;
    final publicationCheckpointIdentity =
        saved.publicationCheckpointIdentity ??
        _currentPublicationCheckpoint(saved);
    if (publicationCheckpointIdentity == null ||
        !_hasPinnedPublicationCheckpoint(
          saved,
          publicationCheckpointIdentity,
        )) {
      return null;
    }
    final catalogReload = _catalogReloadFuture;
    if (catalogReload == null) return null;
    await catalogReload;
    if (!_hasPinnedPublicationCheckpoint(
      saved,
      publicationCheckpointIdentity,
    )) {
      return null;
    }
    if (requiresVoiceAuthority) {
      final voiceReload = _voiceCatalogReloadFuture;
      if (voiceReload == null) return null;
      await voiceReload;
    }
    if (!mounted) return null;
    await WidgetsBinding.instance.endOfFrame;
    if (!_hasPinnedPublicationCheckpoint(
      saved,
      publicationCheckpointIdentity,
    )) {
      return null;
    }
    final ready =
        _publicationAuthorityStatus(
          saved,
          publicationCheckpointIdentity: publicationCheckpointIdentity,
          requiresVoiceAuthority: requiresVoiceAuthority,
          contextKind: contextKind,
          lineId: lineId,
          locale: locale,
        ) ==
        _PublicationAuthorityStatus.ready;
    return ready ? publicationCheckpointIdentity : null;
  }

  Future<void> _runExternalAction(
    Revision3LocalizationVoiceAction? Function() resolveAction, {
    bool Function()? authorityIsCurrent,
    VoidCallback? onAuthorityDrift,
    bool requiresVoiceAuthority = false,
    _VoiceContextActionKind? contextKind,
    String? lineId,
    String? locale,
  }) async {
    if (_saving ||
        _loadingCatalog ||
        _runningExternalAction ||
        resolveAction() == null ||
        (authorityIsCurrent != null && !authorityIsCurrent())) {
      return;
    }
    final originProjectId = widget.projectId;
    final decision = await _confirmExternalAction();
    if (!mounted) return;
    if (widget.projectId != originProjectId) {
      _showMessage(widget.copy.staleMessage);
      return;
    }
    if (authorityIsCurrent != null && !authorityIsCurrent()) {
      onAuthorityDrift?.call();
      return;
    }
    if (decision == _UnsavedExternalActionDecision.keepEditing) return;
    if (decision == _UnsavedExternalActionDecision.saveAndContinue) {
      final externalActionOwner = _beginExternalAction();
      if (externalActionOwner == null) return;
      Object? actionCheckpointIdentity;
      int? actionProjectRevision;
      bool continuationIsCurrent() {
        if (!_externalActionOwnerIsCurrent(externalActionOwner) ||
            widget.projectId != originProjectId) {
          return false;
        }
        final checkpointIdentity = actionCheckpointIdentity;
        return checkpointIdentity == null ||
            (widget.projectRevision == actionProjectRevision &&
                widget.projectCheckpointIdentity == checkpointIdentity);
      }

      try {
        final saved = await _save();
        if (!continuationIsCurrent() || saved == null) return;
        final publicationCheckpointIdentity = await _awaitPublicationAuthority(
          saved,
          requiresVoiceAuthority: requiresVoiceAuthority,
          contextKind: contextKind,
          lineId: lineId,
          locale: locale,
        );
        if (!continuationIsCurrent()) return;
        if (publicationCheckpointIdentity == null ||
            !_hasPinnedPublicationCheckpoint(
              saved,
              publicationCheckpointIdentity,
            )) {
          _replaceMessage(
            '${widget.copy.savedLabel}. ${widget.copy.staleMessage}',
          );
          return;
        }
        actionCheckpointIdentity = publicationCheckpointIdentity;
        actionProjectRevision = saved.publication.projectRevision;
        final action = resolveAction();
        if (!continuationIsCurrent()) return;
        if (action == null) {
          _replaceMessage(
            '${widget.copy.savedLabel}. ${widget.copy.staleMessage}',
          );
          return;
        }
        await action();
        if (!continuationIsCurrent()) return;
      } catch (_) {
        if (continuationIsCurrent()) {
          _showMessage(widget.copy.voiceActionFailedMessage);
        }
      } finally {
        _endExternalAction(externalActionOwner);
      }
      return;
    }
    final reloadAfterAction = _checkpointChangedWhileDirty;
    final actionProjectId = widget.projectId;
    final actionProjectRevision = widget.projectRevision;
    final actionCheckpointIdentity = widget.projectCheckpointIdentity;
    if (_dirty) {
      setState(() {
        _replaceSeed(_seed);
        if (!reloadAfterAction) {
          _checkpointChangedWhileDirty = false;
        }
      });
    }
    final action = resolveAction();
    if (action == null) return;
    final externalActionOwner = _beginExternalAction();
    if (externalActionOwner == null) return;
    bool continuationIsCurrent() =>
        _externalActionOwnerIsCurrent(externalActionOwner) &&
        widget.projectId == actionProjectId &&
        widget.projectRevision == actionProjectRevision &&
        widget.projectCheckpointIdentity == actionCheckpointIdentity;
    try {
      await action();
      if (!continuationIsCurrent()) return;
      if (reloadAfterAction) {
        await _reloadCatalog(clearCurrent: true);
      }
    } catch (_) {
      if (continuationIsCurrent()) {
        _showMessage(widget.copy.voiceActionFailedMessage);
      }
    } finally {
      _endExternalAction(externalActionOwner);
    }
  }

  Revision3LocalizationVoiceContextAction? _contextAction(
    _VoiceContextActionKind kind,
  ) => switch (kind) {
    _VoiceContextActionKind.addTake => widget.onAddVoiceTakeFor,
    _VoiceContextActionKind.manageTakes => widget.onManageVoiceTakesFor,
    _VoiceContextActionKind.resolveTarget => widget.onResolveVoiceTargetFor,
  };

  Future<void> _runContextAction(
    _VoiceContextActionKind kind,
    Revision3VoiceCatalog checkpoint,
  ) async {
    final projectId = widget.projectId;
    final projectRevision = widget.projectRevision;
    final projectCheckpointIdentity = widget.projectCheckpointIdentity;
    final lineId = _voiceLineId;
    final locale = _voiceLocale;
    if (lineId == null || locale == null) return;
    bool authorityIsCurrent() =>
        mounted &&
        widget.projectId == projectId &&
        widget.projectRevision == projectRevision &&
        widget.projectCheckpointIdentity == projectCheckpointIdentity &&
        !_saving &&
        !_loadingCatalog &&
        !_runningExternalAction &&
        !_checkpointChangedWhileDirty &&
        !_loadingVoiceCatalog &&
        _voiceCatalogError == null &&
        identical(_voiceCatalog, checkpoint) &&
        _voiceLineId == lineId &&
        _voiceLocale == locale &&
        checkpoint.line(lineId) != null;
    if (!authorityIsCurrent()) {
      return;
    }
    Revision3LocalizationVoiceAction? resolveAction() {
      final currentAction = _contextAction(kind);
      if (currentAction == null) return null;
      return () => currentAction(initialLineId: lineId, initialLocale: locale);
    }

    await _runExternalAction(
      resolveAction,
      authorityIsCurrent: authorityIsCurrent,
      onAuthorityDrift: () => _showMessage(widget.copy.staleMessage),
      requiresVoiceAuthority: true,
      contextKind: kind,
      lineId: lineId,
      locale: locale,
    );
  }

  void _selectVoiceLine(Revision3DialogLocalizationLineBacklink line) {
    final seed = _seed;
    if (seed == null || _contextMutationBlocked) return;
    final locales = seed.locales.map((locale) => locale.locale).toList();
    final retainedLocale = locales.contains(_voiceLocale) ? _voiceLocale : null;
    final slottedLocale = line.voiceSlotLocales
        .where(locales.contains)
        .firstOrNull;
    setState(() {
      _voiceLineId = line.lineId;
      _voiceLocale = retainedLocale ?? slottedLocale ?? locales.firstOrNull;
    });
  }

  void _selectVoiceLocale(String locale) {
    if (_contextMutationBlocked) return;
    setState(() => _voiceLocale = locale);
  }

  Future<void> _refreshCatalogFromBrowser() async {
    if (_contextMutationBlocked) return;
    if (!await _confirmDiscard() || !mounted || _contextMutationBlocked) return;
    await _reloadCatalog();
  }

  Future<void> _retryCatalog({required bool clearCurrent}) async {
    if (_contextMutationBlocked) return;
    await _reloadCatalog(clearCurrent: clearCurrent);
  }

  Future<void> _retrySeed() async {
    if (_contextMutationBlocked) return;
    final catalog = _catalog;
    final key = _selectedKey;
    if (catalog == null || key == null) return;
    await _loadSeed(catalog, key);
  }

  void _leaveFailedCompactEditor() {
    if (_contextMutationBlocked) return;
    setState(() => _showEditorOnCompact = false);
  }

  Future<void> _leaveCompactEditor() async {
    if (_contextMutationBlocked) return;
    if (!await _confirmDiscard() || !mounted || _contextMutationBlocked) return;
    final reload = _checkpointChangedWhileDirty;
    setState(() {
      _replaceSeed(reload ? null : _seed);
      _showEditorOnCompact = false;
    });
    if (reload && !_contextMutationBlocked) {
      await _reloadCatalog(clearCurrent: true);
    }
  }

  Future<void> _refreshChangedCheckpoint() async {
    if (_contextMutationBlocked ||
        !await _confirmDiscard() ||
        !mounted ||
        _contextMutationBlocked) {
      return;
    }
    setState(() {
      _showEditorOnCompact = false;
      _replaceSeed(null);
    });
    await _reloadCatalog(clearCurrent: true);
  }

  void _showMessage(String message) {
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message)));
  }

  void _replaceMessage(String message) {
    final messenger = ScaffoldMessenger.of(context);
    messenger
      ..removeCurrentSnackBar()
      ..showSnackBar(SnackBar(content: Text(message)));
  }

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final wide = constraints.maxWidth >= 900;
      final denseHeader = constraints.maxHeight < 520;
      return Column(
        key: const Key('revision3-localization-voice-workspace'),
        children: [
          _WorkspaceHeader(
            copy: widget.copy,
            notice: widget.notice,
            onCreateDialogLine:
                widget.onCreateDialogLine == null ||
                    _loadingCatalog ||
                    _saving ||
                    _runningExternalAction
                ? null
                : () => _runExternalAction(() => widget.onCreateDialogLine),
            onAddVoiceTake:
                widget.onAddVoiceTake == null ||
                    _loadingCatalog ||
                    _saving ||
                    _runningExternalAction
                ? null
                : () => _runExternalAction(
                    () => widget.onAddVoiceTake,
                    requiresVoiceAuthority: true,
                  ),
            onImportVoiceFolder:
                widget.onImportVoiceFolder == null ||
                    _loadingCatalog ||
                    _saving ||
                    _runningExternalAction
                ? null
                : () => _runExternalAction(
                    () => widget.onImportVoiceFolder,
                    requiresVoiceAuthority: true,
                  ),
            onManageVoiceTakes:
                widget.onManageVoiceTakes == null ||
                    _loadingCatalog ||
                    _saving ||
                    _runningExternalAction
                ? null
                : () => _runExternalAction(
                    () => widget.onManageVoiceTakes,
                    requiresVoiceAuthority: true,
                  ),
            onResolveVoiceTarget:
                widget.onResolveVoiceTarget == null ||
                    _loadingCatalog ||
                    _saving ||
                    _runningExternalAction
                ? null
                : () => _runExternalAction(
                    () => widget.onResolveVoiceTarget,
                    requiresVoiceAuthority: true,
                  ),
            dense: denseHeader,
            compactActions: !wide || denseHeader,
          ),
          const Divider(height: 1),
          Expanded(
            child: wide
                ? Row(
                    children: [
                      SizedBox(
                        width: 340,
                        child: _buildBrowser(dense: denseHeader),
                      ),
                      const VerticalDivider(width: 1),
                      Expanded(child: _buildEditor(compact: false)),
                    ],
                  )
                : _showEditorOnCompact && _selectedKey != null
                ? _buildEditor(compact: true)
                : _buildBrowser(dense: denseHeader),
          ),
        ],
      );
    },
  );

  Widget _buildBrowser({required bool dense}) {
    if (_catalogError != null) {
      final stale = _checkpointChangedWhileDirty;
      final retryable = _loadErrorRetryable(_catalogError!);
      return _WorkspaceFailure(
        title: stale
            ? widget.copy.staleMessage
            : _loadErrorTitle(_catalogError!),
        retryLabel: stale ? widget.copy.refreshLabel : widget.copy.retryLabel,
        retryKey: const Key('revision3-localization-catalog-retry'),
        showRetry: retryable,
        retry: _contextMutationBlocked || !retryable
            ? null
            : stale
            ? () => _retryCatalog(clearCurrent: true)
            : () => _retryCatalog(clearCurrent: false),
      );
    }
    final catalog = _catalog;
    if (catalog == null) {
      return Center(
        child: Semantics(
          liveRegion: true,
          label: widget.copy.loadingLabel,
          child: const CircularProgressIndicator(),
        ),
      );
    }
    final query = _search.text;
    final choices = catalog.choices
        .where((choice) => choice.matches(query))
        .toList(growable: false);
    return Column(
      key: const Key('revision3-localization-text-browser'),
      children: [
        if (!dense)
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 16, 8, 8),
            child: Row(
              children: [
                Expanded(
                  child: Text(
                    widget.copy.projectTextsLabel,
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                IconButton(
                  key: const Key('revision3-localization-browser-refresh'),
                  tooltip: widget.copy.refreshLabel,
                  onPressed: _contextMutationBlocked
                      ? null
                      : _refreshCatalogFromBrowser,
                  icon: const Icon(Icons.refresh),
                ),
              ],
            ),
          ),
        Padding(
          padding: dense
              ? const EdgeInsets.all(8)
              : const EdgeInsets.fromLTRB(12, 0, 12, 12),
          child: TextField(
            key: const Key('revision3-localization-search'),
            controller: _search,
            decoration: InputDecoration(
              labelText: widget.copy.searchLabel,
              prefixIcon: const Icon(Icons.search),
              suffixIcon: dense
                  ? IconButton(
                      key: const Key('revision3-localization-browser-refresh'),
                      tooltip: widget.copy.refreshLabel,
                      onPressed: _contextMutationBlocked
                          ? null
                          : _refreshCatalogFromBrowser,
                      icon: const Icon(Icons.refresh),
                    )
                  : null,
              border: const OutlineInputBorder(),
              isDense: true,
            ),
          ),
        ),
        if (catalog.choices.isEmpty)
          Expanded(
            child: _WorkspaceEmpty(
              title: widget.copy.emptyTitle,
              description: widget.copy.emptyDescription,
            ),
          )
        else if (choices.isEmpty)
          Expanded(
            child: _WorkspaceEmpty(
              title: widget.copy.selectTextLabel,
              description: widget.copy.searchLabel,
            ),
          )
        else
          Expanded(
            child: ListView.builder(
              key: const Key('revision3-localization-text-list'),
              itemCount: choices.length,
              itemBuilder: (context, index) {
                final choice = choices[index];
                final selected = choice.stableKey == _selectedKey;
                final visibleContext = choice.visibleContextLabelFor(query);
                return ListTile(
                  key: ValueKey(
                    'revision3-localization-choice-${choice.stableKey}',
                  ),
                  selected: selected,
                  leading: const Icon(Icons.chat_bubble_outline),
                  title: Text(
                    choice.displayLabel,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                  isThreeLine: visibleContext != null,
                  subtitle: visibleContext == null
                      ? Text(
                          choice.locales.join('  ·  '),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        )
                      : Column(
                          mainAxisSize: MainAxisSize.min,
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              visibleContext,
                              key: ValueKey(
                                'revision3-localization-choice-context-${choice.stableKey}',
                              ),
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                            Text(
                              choice.locales.join('  ·  '),
                              key: ValueKey(
                                'revision3-localization-choice-locales-${choice.stableKey}',
                              ),
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                          ],
                        ),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: _contextMutationBlocked
                      ? null
                      : () => unawaited(_selectChoice(choice.stableKey)),
                );
              },
            ),
          ),
      ],
    );
  }

  Widget _buildEditor({required bool compact}) {
    if (_loadingSeed) {
      return Center(
        child: Semantics(
          liveRegion: true,
          label: widget.copy.loadingLabel,
          child: const CircularProgressIndicator(),
        ),
      );
    }
    if (_seedError != null) {
      final stale =
          _checkpointChangedWhileDirty ||
          _seedError is Revision3DialogLocalizationEditStaleCheckpointException;
      final retryable = _loadErrorRetryable(_seedError!);
      final failure = _WorkspaceFailure(
        title: _checkpointChangedWhileDirty
            ? widget.copy.staleMessage
            : _loadErrorTitle(_seedError!),
        retryLabel: stale ? widget.copy.refreshLabel : widget.copy.retryLabel,
        retryKey: const Key('revision3-localization-seed-retry'),
        showRetry: retryable,
        retry: _contextMutationBlocked || !retryable
            ? null
            : stale
            ? () => _retryCatalog(clearCurrent: true)
            : _retrySeed,
      );
      if (!compact) return failure;
      return Column(
        children: [
          Material(
            color: Theme.of(context).colorScheme.surfaceContainerLow,
            child: Align(
              alignment: Alignment.centerLeft,
              child: IconButton(
                key: const Key('revision3-localization-editor-back'),
                tooltip: widget.copy.projectTextsLabel,
                onPressed: _contextMutationBlocked
                    ? null
                    : _leaveFailedCompactEditor,
                icon: const Icon(Icons.arrow_back),
              ),
            ),
          ),
          Expanded(child: failure),
        ],
      );
    }
    final seed = _seed;
    if (seed == null) {
      return _WorkspaceEmpty(
        title: widget.copy.selectTextLabel,
        description: widget.copy.offlineNotice,
      );
    }
    final locales = <Revision3DialogLocalizationLocaleSeed>[];
    final byLocale = {for (final locale in seed.locales) locale.locale: locale};
    for (final locale in _texts.keys.toList()..sort()) {
      locales.add(
        byLocale[locale] ??
            Revision3DialogLocalizationLocaleSeed.added(
              locale: locale,
              text: _texts[locale]!.text,
            ),
      );
    }
    return Column(
      key: const Key('revision3-localization-text-editor'),
      children: [
        _buildEditorHeader(seed, compact: compact),
        Expanded(
          child: ListView(
            key: const Key('revision3-localization-editor-scroll'),
            padding: const EdgeInsets.all(20),
            children: [
              if (_checkpointChangedWhileDirty) ...[
                _WorkspaceNotice(
                  icon: Icons.update_outlined,
                  message: widget.copy.staleMessage,
                ),
                const SizedBox(height: 8),
                Align(
                  alignment: Alignment.centerLeft,
                  child: OutlinedButton.icon(
                    key: const Key(
                      'revision3-localization-refresh-changed-project',
                    ),
                    onPressed: _contextMutationBlocked
                        ? null
                        : _refreshChangedCheckpoint,
                    icon: const Icon(Icons.refresh),
                    label: Text(widget.copy.refreshLabel),
                  ),
                ),
                const Divider(height: 28),
              ],
              LayoutBuilder(
                builder: (context, constraints) {
                  final languageEditors = _buildLocalizationEditors(locales);
                  final voiceInspector = _buildLineVoiceInspector(seed);
                  if (constraints.maxWidth < 760) {
                    return Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        languageEditors,
                        const Divider(height: 32),
                        voiceInspector,
                      ],
                    );
                  }
                  return Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Expanded(child: languageEditors),
                      const SizedBox(width: 20),
                      SizedBox(width: 340, child: voiceInspector),
                    ],
                  );
                },
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildLocalizationEditors(
    List<Revision3DialogLocalizationLocaleSeed> locales,
  ) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      Row(
        children: [
          Expanded(
            child: Text(
              widget.copy.languagesLabel,
              style: Theme.of(context).textTheme.titleMedium,
            ),
          ),
          OutlinedButton.icon(
            key: const Key('revision3-localization-add-language'),
            onPressed: _saving || _loadingCatalog || _runningExternalAction
                ? null
                : _addLocale,
            icon: const Icon(Icons.add),
            label: Text(widget.copy.addLanguageLabel),
          ),
        ],
      ),
      const SizedBox(height: 12),
      for (final locale in locales) ...[
        _LocaleEditor(
          copy: widget.copy,
          locale: locale,
          controller: _texts[locale.locale]!,
          saving: _saving || _loadingCatalog || _runningExternalAction,
          canRemove: locale.canRemove && _texts.length > 1,
          onRemove: () => _removeLocale(locale),
        ),
        const SizedBox(height: 14),
      ],
    ],
  );

  Widget _buildLineVoiceInspector(Revision3DialogLocalizationEditSeed seed) =>
      Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (seed.lineBacklinks.length > 1) ...[
            _WorkspaceNotice(
              icon: Icons.hub_outlined,
              message: widget.copy.sharedTextNotice,
            ),
            const SizedBox(height: 16),
          ],
          Text(
            widget.copy.usedByLinesLabel,
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 8),
          if (seed.lineBacklinks.isEmpty)
            Text(widget.copy.noLineLabel)
          else
            for (final line in seed.lineBacklinks)
              ListTile(
                key: ValueKey(
                  'revision3-localization-voice-line-${line.lineId}',
                ),
                contentPadding: EdgeInsets.zero,
                dense: true,
                selected: line.lineId == _voiceLineId,
                leading: const Icon(Icons.short_text),
                title: Text(line.displayLabel),
                subtitle: line.speakerLabel == null
                    ? null
                    : Text('${widget.copy.speakerLabel}: ${line.speakerLabel}'),
                trailing: const Icon(Icons.chevron_right),
                onTap: _saving || _loadingCatalog || _runningExternalAction
                    ? null
                    : () => _selectVoiceLine(line),
              ),
          if (seed.lineBacklinks.isNotEmpty) ...[
            const SizedBox(height: 12),
            _buildVoiceContext(seed),
          ],
        ],
      );

  Widget _buildVoiceContext(Revision3DialogLocalizationEditSeed seed) {
    final line = seed.lineBacklinks
        .where((candidate) => candidate.lineId == _voiceLineId)
        .firstOrNull;
    final locale = _voiceLocale;
    final slotExpected =
        line != null &&
        locale != null &&
        line.voiceSlotLocales.contains(locale);
    final voiceCatalog = _voiceCatalog;
    final exactLine = line == null ? null : voiceCatalog?.line(line.lineId);
    final exactSummary = locale == null
        ? null
        : exactLine?.slotSummaryForLocale(locale);
    final busy =
        _saving ||
        _loadingCatalog ||
        _loadingVoiceCatalog ||
        _runningExternalAction ||
        _checkpointChangedWhileDirty;
    final canAdd =
        !busy &&
        locale != null &&
        exactLine?.isLocaleAuthorable(locale) == true &&
        widget.onAddVoiceTakeFor != null;
    final canManage =
        !busy && exactSummary != null && widget.onManageVoiceTakesFor != null;
    final canResolve =
        !busy &&
        locale != null &&
        exactLine?.isLocaleTargetable(locale) == true &&
        widget.onResolveVoiceTargetFor != null;
    final theme = Theme.of(context);
    final subtitle = line == null
        ? widget.copy.voiceSelectLineLabel
        : locale == null
        ? line.displayLabel
        : '${line.displayLabel} • $locale';
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Material(
          color: theme.colorScheme.surfaceContainerLow,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
            side: BorderSide(color: theme.colorScheme.outlineVariant),
          ),
          child: Padding(
            padding: const EdgeInsets.fromLTRB(14, 8, 14, 12),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                ListTile(
                  contentPadding: EdgeInsets.zero,
                  dense: true,
                  leading: Icon(
                    slotExpected ? Icons.mic : Icons.mic_none_outlined,
                    color: theme.colorScheme.primary,
                  ),
                  title: Text(widget.copy.voiceContextTitle),
                  subtitle: Text(
                    subtitle,
                    key: line == null
                        ? const Key(
                            'revision3-localization-voice-select-line-hint',
                          )
                        : null,
                  ),
                ),
                if (line != null) ...[
                  const SizedBox(height: 4),
                  Wrap(
                    key: const Key('revision3-localization-voice-locales'),
                    spacing: 8,
                    runSpacing: 8,
                    children: [
                      for (final item in seed.locales)
                        ChoiceChip(
                          key: ValueKey(
                            'revision3-localization-voice-locale-${item.locale}',
                          ),
                          selected: locale == item.locale,
                          avatar: Icon(
                            line.voiceSlotLocales.contains(item.locale)
                                ? Icons.mic
                                : Icons.mic_none,
                            size: 16,
                          ),
                          label: Text(item.locale),
                          onSelected: busy
                              ? null
                              : (_) => _selectVoiceLocale(item.locale),
                        ),
                    ],
                  ),
                ],
              ],
            ),
          ),
        ),
        const SizedBox(height: 10),
        Revision3VoiceProductionCard(
          line: exactLine,
          locale: locale,
          slotExpected: slotExpected,
          projectionRejected:
              _voiceCatalog != null &&
              line != null &&
              locale != null &&
              exactLine == null,
          loading: _loadingVoiceCatalog,
          error: _checkpointChangedWhileDirty
              ? const Revision3DialogLocalizationEditStaleCheckpointException()
              : _voiceCatalogError,
          copy: widget.voiceProductionCopy,
          onAddTake: canAdd
              ? () => unawaited(
                  _runContextAction(
                    _VoiceContextActionKind.addTake,
                    voiceCatalog!,
                  ),
                )
              : null,
          onManageTakes: canManage
              ? () => unawaited(
                  _runContextAction(
                    _VoiceContextActionKind.manageTakes,
                    voiceCatalog!,
                  ),
                )
              : null,
          onResolveTarget: canResolve
              ? () => unawaited(
                  _runContextAction(
                    _VoiceContextActionKind.resolveTarget,
                    voiceCatalog!,
                  ),
                )
              : null,
        ),
      ],
    );
  }

  Widget _buildEditorHeader(
    Revision3DialogLocalizationEditSeed seed, {
    required bool compact,
  }) {
    final save = FilledButton.icon(
      key: const Key('revision3-localization-save'),
      onPressed:
          _dirty &&
              !_saving &&
              !_loadingCatalog &&
              !_runningExternalAction &&
              !_checkpointChangedWhileDirty
          ? _save
          : null,
      icon: _saving
          ? const SizedBox.square(
              dimension: 16,
              child: CircularProgressIndicator(strokeWidth: 2),
            )
          : const Icon(Icons.save_outlined),
      label: Text(_saving ? widget.copy.savingLabel : widget.copy.saveLabel),
    );
    if (!compact) {
      return Material(
        color: Theme.of(context).colorScheme.surfaceContainerLow,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 10, 16, 10),
          child: Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      seed.choice.displayLabel,
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                    Text(
                      widget.copy.offlineNotice,
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              save,
            ],
          ),
        ),
      );
    }
    return Material(
      color: Theme.of(context).colorScheme.surfaceContainerLow,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(8, 8, 12, 10),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                IconButton(
                  key: const Key('revision3-localization-editor-back'),
                  tooltip: widget.copy.projectTextsLabel,
                  onPressed: _contextMutationBlocked
                      ? null
                      : _leaveCompactEditor,
                  icon: const Icon(Icons.arrow_back),
                ),
                const SizedBox(width: 4),
                Expanded(
                  child: Text(
                    seed.choice.displayLabel,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
              ],
            ),
            Padding(
              padding: const EdgeInsets.only(left: 52),
              child: Text(
                widget.copy.offlineNotice,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ),
            const SizedBox(height: 8),
            Align(alignment: Alignment.centerRight, child: save),
          ],
        ),
      ),
    );
  }

  String _loadErrorTitle(Object error) => switch (error) {
    Revision3DialogLocalizationEditRequiresReopenException() =>
      widget.copy.reopenMessage,
    Revision3DialogLocalizationEditStaleCheckpointException() =>
      widget.copy.staleMessage,
    _ => widget.copy.loadFailedTitle,
  };

  bool _loadErrorRetryable(Object error) =>
      error is! Revision3DialogLocalizationEditRequiresReopenException;
}

enum _WorkspaceHeaderAction {
  addVoice,
  importVoiceFolder,
  manageVoice,
  resolveVoice,
}

class _WorkspaceHeader extends StatelessWidget {
  const _WorkspaceHeader({
    required this.copy,
    required this.notice,
    required this.onCreateDialogLine,
    required this.onAddVoiceTake,
    required this.onImportVoiceFolder,
    required this.onManageVoiceTakes,
    required this.onResolveVoiceTarget,
    required this.dense,
    required this.compactActions,
  });

  final Revision3LocalizationVoiceWorkspaceCopy copy;
  final String? notice;
  final Revision3LocalizationVoiceAction? onCreateDialogLine;
  final Revision3LocalizationVoiceAction? onAddVoiceTake;
  final Revision3LocalizationVoiceAction? onImportVoiceFolder;
  final Revision3LocalizationVoiceAction? onManageVoiceTakes;
  final Revision3LocalizationVoiceAction? onResolveVoiceTarget;
  final bool dense;
  final bool compactActions;

  @override
  Widget build(BuildContext context) {
    final primaryAction = FilledButton.icon(
      key: const Key('revision3-localization-new-line'),
      onPressed: onCreateDialogLine == null
          ? null
          : () => onCreateDialogLine!(),
      icon: const Icon(Icons.add_comment_outlined),
      label: Text(copy.newLineLabel, overflow: TextOverflow.ellipsis),
    );
    final secondaryActions = <Widget>[
      _HeaderAction(
        key: const Key('revision3-localization-add-voice'),
        icon: Icons.mic_none_outlined,
        label: copy.addVoiceLabel,
        action: onAddVoiceTake,
      ),
      _HeaderAction(
        key: const Key('revision3-localization-import-voice-folder'),
        icon: Icons.drive_folder_upload_outlined,
        label: copy.importVoiceFolderLabel,
        action: onImportVoiceFolder,
      ),
      _HeaderAction(
        key: const Key('revision3-localization-manage-voice'),
        icon: Icons.library_music_outlined,
        label: copy.manageVoiceLabel,
        action: onManageVoiceTakes,
      ),
      _HeaderAction(
        key: const Key('revision3-localization-resolve-voice'),
        icon: Icons.link_outlined,
        label: copy.resolveVoiceLabel,
        action: onResolveVoiceTarget,
      ),
    ];
    final actionStrip = compactActions
        ? LayoutBuilder(
            builder: (context, constraints) {
              final showAddVoiceShortcut = constraints.maxWidth >= 560;
              return Row(
                children: [
                  Expanded(child: primaryAction),
                  if (showAddVoiceShortcut) ...[
                    const SizedBox(width: 8),
                    Expanded(child: secondaryActions.first),
                  ],
                  const SizedBox(width: 8),
                  PopupMenuButton<_WorkspaceHeaderAction>(
                    key: const Key('revision3-localization-more-actions'),
                    tooltip: MaterialLocalizations.of(
                      context,
                    ).moreButtonTooltip,
                    icon: const Icon(Icons.more_horiz),
                    onSelected: (selection) {
                      final action = switch (selection) {
                        _WorkspaceHeaderAction.addVoice => onAddVoiceTake,
                        _WorkspaceHeaderAction.importVoiceFolder =>
                          onImportVoiceFolder,
                        _WorkspaceHeaderAction.manageVoice =>
                          onManageVoiceTakes,
                        _WorkspaceHeaderAction.resolveVoice =>
                          onResolveVoiceTarget,
                      };
                      action?.call();
                    },
                    itemBuilder: (context) => [
                      if (!showAddVoiceShortcut)
                        _overflowAction(
                          key: const Key('revision3-localization-add-voice'),
                          value: _WorkspaceHeaderAction.addVoice,
                          icon: Icons.mic_none_outlined,
                          label: copy.addVoiceLabel,
                          action: onAddVoiceTake,
                        ),
                      _overflowAction(
                        key: const Key(
                          'revision3-localization-import-voice-folder',
                        ),
                        value: _WorkspaceHeaderAction.importVoiceFolder,
                        icon: Icons.drive_folder_upload_outlined,
                        label: copy.importVoiceFolderLabel,
                        action: onImportVoiceFolder,
                      ),
                      _overflowAction(
                        key: const Key('revision3-localization-manage-voice'),
                        value: _WorkspaceHeaderAction.manageVoice,
                        icon: Icons.library_music_outlined,
                        label: copy.manageVoiceLabel,
                        action: onManageVoiceTakes,
                      ),
                      _overflowAction(
                        key: const Key('revision3-localization-resolve-voice'),
                        value: _WorkspaceHeaderAction.resolveVoice,
                        icon: Icons.link_outlined,
                        label: copy.resolveVoiceLabel,
                        action: onResolveVoiceTarget,
                      ),
                    ],
                  ),
                ],
              );
            },
          )
        : Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [primaryAction, ...secondaryActions],
          );
    final title = Text(
      copy.title,
      maxLines: dense ? 1 : null,
      overflow: dense ? TextOverflow.ellipsis : null,
      style: Theme.of(context).textTheme.titleLarge,
    );
    return Material(
      color: Theme.of(context).colorScheme.surface,
      child: Padding(
        padding: dense
            ? const EdgeInsets.fromLTRB(12, 8, 12, 8)
            : const EdgeInsets.fromLTRB(20, 16, 20, 14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Padding(
                  padding: const EdgeInsets.only(top: 2),
                  child: Icon(
                    Icons.record_voice_over_outlined,
                    size: dense ? 24 : 28,
                  ),
                ),
                SizedBox(width: dense ? 8 : 12),
                Expanded(
                  child: dense
                      ? Tooltip(message: copy.description, child: title)
                      : Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            title,
                            const SizedBox(height: 3),
                            Text(copy.description),
                          ],
                        ),
                ),
              ],
            ),
            if (notice != null) ...[
              SizedBox(height: dense ? 3 : 6),
              Text(
                notice!,
                maxLines: dense ? 1 : null,
                overflow: dense ? TextOverflow.ellipsis : null,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ],
            SizedBox(height: dense ? 6 : 12),
            actionStrip,
          ],
        ),
      ),
    );
  }

  PopupMenuItem<_WorkspaceHeaderAction> _overflowAction({
    required Key key,
    required _WorkspaceHeaderAction value,
    required IconData icon,
    required String label,
    required Revision3LocalizationVoiceAction? action,
  }) => PopupMenuItem<_WorkspaceHeaderAction>(
    key: key,
    value: value,
    enabled: action != null,
    child: Row(
      children: [
        Icon(icon),
        const SizedBox(width: 12),
        Flexible(child: Text(label)),
      ],
    ),
  );
}

class _HeaderAction extends StatelessWidget {
  const _HeaderAction({
    required this.icon,
    required this.label,
    required this.action,
    super.key,
  });

  final IconData icon;
  final String label;
  final Revision3LocalizationVoiceAction? action;

  @override
  Widget build(BuildContext context) => OutlinedButton.icon(
    onPressed: action == null ? null : () => action!(),
    icon: Icon(icon),
    label: Text(label, overflow: TextOverflow.ellipsis),
  );
}

class _LocaleEditor extends StatelessWidget {
  const _LocaleEditor({
    required this.copy,
    required this.locale,
    required this.controller,
    required this.saving,
    required this.canRemove,
    required this.onRemove,
  });

  final Revision3LocalizationVoiceWorkspaceCopy copy;
  final Revision3DialogLocalizationLocaleSeed locale;
  final TextEditingController controller;
  final bool saving;
  final bool canRemove;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Material(
      key: ValueKey('revision3-localization-locale-${locale.locale}'),
      color: scheme.surfaceContainerLow,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(14),
        side: BorderSide(color: scheme.outlineVariant),
      ),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 10,
                    vertical: 5,
                  ),
                  decoration: BoxDecoration(
                    color: scheme.secondaryContainer,
                    borderRadius: BorderRadius.circular(20),
                  ),
                  child: Text(
                    locale.locale,
                    style: TextStyle(color: scheme.onSecondaryContainer),
                  ),
                ),
                const Spacer(),
                IconButton(
                  tooltip: canRemove
                      ? copy.removeLanguageLabel
                      : locale.hasVoiceSlot
                      ? copy.voiceSlotRemovalLockedLabel
                      : copy.minimumLanguageLockedLabel,
                  onPressed: saving || !canRemove ? null : onRemove,
                  icon: const Icon(Icons.delete_outline),
                ),
              ],
            ),
            const SizedBox(height: 8),
            TextField(
              key: ValueKey('revision3-localization-text-${locale.locale}'),
              controller: controller,
              readOnly: locale.textLocked || saving,
              minLines: 3,
              maxLines: 8,
              decoration: InputDecoration(
                labelText: copy.dialogTextLabel,
                border: const OutlineInputBorder(),
              ),
            ),
            if (locale.textLocked) ...[
              const SizedBox(height: 10),
              _WorkspaceNotice(
                icon: Icons.lock_outline,
                message: copy.voiceLockedLabel,
              ),
            ] else if (locale.hasVoiceSlot && !locale.canRemove) ...[
              const SizedBox(height: 10),
              _WorkspaceNotice(
                icon: Icons.link_outlined,
                message: copy.voiceSlotRemovalLockedLabel,
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _AddLocaleDialog extends StatefulWidget {
  const _AddLocaleDialog({required this.copy, required this.existingLocales});

  final Revision3LocalizationVoiceWorkspaceCopy copy;
  final Set<String> existingLocales;

  @override
  State<_AddLocaleDialog> createState() => _AddLocaleDialogState();
}

class _AddLocaleDialogState extends State<_AddLocaleDialog> {
  final _locale = TextEditingController();
  final _text = TextEditingController();

  @override
  void dispose() {
    _locale.dispose();
    _text.dispose();
    super.dispose();
  }

  void _submit() {
    final locale = _locale.text.trim();
    final text = _text.text;
    if (!_localeIsCanonical(locale) ||
        widget.existingLocales.contains(locale) ||
        text.trim().isEmpty) {
      return;
    }
    Navigator.of(context).pop(_AddedLocale(locale, text));
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: Text(widget.copy.addLanguageLabel),
    content: ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 520),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          TextField(
            key: const Key('revision3-localization-new-locale-code'),
            controller: _locale,
            autofocus: true,
            decoration: InputDecoration(
              labelText: widget.copy.languageCodeLabel,
              hintText: widget.copy.languageCodeHint,
              errorText: widget.existingLocales.contains(_locale.text.trim())
                  ? widget.copy.languageExistsMessage
                  : null,
              border: const OutlineInputBorder(),
            ),
            onChanged: (_) => setState(() {}),
          ),
          const SizedBox(height: 14),
          TextField(
            key: const Key('revision3-localization-new-locale-text'),
            controller: _text,
            minLines: 3,
            maxLines: 8,
            decoration: InputDecoration(
              labelText: widget.copy.dialogTextLabel,
              border: const OutlineInputBorder(),
            ),
            onChanged: (_) => setState(() {}),
          ),
        ],
      ),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.of(context).pop(),
        child: Text(widget.copy.cancelLabel),
      ),
      FilledButton(
        onPressed:
            _localeIsCanonical(_locale.text.trim()) &&
                !widget.existingLocales.contains(_locale.text.trim()) &&
                _text.text.trim().isNotEmpty
            ? _submit
            : null,
        child: Text(widget.copy.addLabel),
      ),
    ],
  );
}

class _AddedLocale {
  const _AddedLocale(this.locale, this.text);

  final String locale;
  final String text;
}

class _WorkspaceNotice extends StatelessWidget {
  const _WorkspaceNotice({required this.icon, required this.message});

  final IconData icon;
  final String message;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(icon, size: 18, color: scheme.onSurfaceVariant),
        const SizedBox(width: 8),
        Expanded(
          child: Text(
            message,
            style: TextStyle(color: scheme.onSurfaceVariant),
          ),
        ),
      ],
    );
  }
}

class _WorkspaceFailure extends StatelessWidget {
  const _WorkspaceFailure({
    required this.title,
    required this.retryLabel,
    required this.retry,
    required this.showRetry,
    this.retryKey,
  });

  final String title;
  final String retryLabel;
  final FutureOr<void> Function()? retry;
  final bool showRetry;
  final Key? retryKey;

  @override
  Widget build(BuildContext context) => Center(
    child: Padding(
      padding: const EdgeInsets.all(24),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Icon(Icons.error_outline, size: 38),
          const SizedBox(height: 12),
          Text(title, textAlign: TextAlign.center),
          if (showRetry) ...[
            const SizedBox(height: 12),
            FilledButton(
              key: retryKey,
              onPressed: retry,
              child: Text(retryLabel),
            ),
          ],
        ],
      ),
    ),
  );
}

class _WorkspaceEmpty extends StatelessWidget {
  const _WorkspaceEmpty({required this.title, required this.description});

  final String title;
  final String description;

  @override
  Widget build(BuildContext context) => Center(
    child: Padding(
      padding: const EdgeInsets.all(24),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Icon(Icons.translate_outlined, size: 42),
          const SizedBox(height: 12),
          Text(title, style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 6),
          Text(description, textAlign: TextAlign.center),
        ],
      ),
    ),
  );
}

bool _sameTexts(Map<String, String> left, Map<String, String> right) {
  if (left.length != right.length) return false;
  for (final entry in left.entries) {
    if (right[entry.key] != entry.value) return false;
  }
  return true;
}

bool _localeIsCanonical(String value) {
  if (value.isEmpty ||
      utf8.encode(value).length > 35 ||
      value.codeUnits.any((unit) => unit > 0x7f)) {
    return false;
  }
  final segments = value.split('-');
  final language = segments.first;
  if (language.length < 2 ||
      language.length > 8 ||
      !RegExp(r'^[a-z]+$').hasMatch(language)) {
    return false;
  }
  final canonical = <String>[language];
  for (var index = 1; index < segments.length; index++) {
    final segment = segments[index];
    if (segment.isEmpty ||
        segment.length > 8 ||
        !RegExp(r'^[A-Za-z0-9]+$').hasMatch(segment)) {
      return false;
    }
    canonical.add(
      segment.length == 4 && RegExp(r'^[A-Za-z]+$').hasMatch(segment)
          ? '${segment[0].toUpperCase()}${segment.substring(1).toLowerCase()}'
          : segment.length == 2 && RegExp(r'^[A-Za-z]+$').hasMatch(segment)
          ? segment.toUpperCase()
          : segment.toLowerCase(),
    );
  }
  return value == canonical.join('-');
}
