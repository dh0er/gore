import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';

import '../story/ui/story_npc_archetype_picker.dart';
import 'revision3_npc_authoring.dart';

typedef Revision3NpcArchetypeChooser =
    Future<String?> Function(BuildContext context, Revision3NpcCatalog catalog);

/// Author-facing copy for the bounded managed-R3 NPC Draft wizard.
///
/// The surrounding workspace may inject localized copy without giving this
/// isolated editor access to generated localization classes. English remains
/// the default so existing callers keep their current presentation.
@immutable
final class Revision3NpcWizardCopy {
  const Revision3NpcWizardCopy({
    required this.title,
    required this.catalogLoadFailed,
    required this.archetypeNotQualified,
    required this.archetypePickerFailed,
    required this.archetypeRequired,
    required this.catalogChanged,
    required this.requiresReopen,
    required this.staleCheckpoint,
    required this.saveFailed,
    required this.catalogLoadingSemanticsLabel,
    required this.retryCatalogLabel,
    required this.closeLabel,
    required this.cancelLabel,
    required this.saveLabel,
    required this.discardTitle,
    required this.discardDescription,
    required this.keepEditingLabel,
    required this.discardLabel,
    required this.basicsTitle,
    required this.displayNameLabel,
    required this.displayNameHint,
    required this.displayNameHelp,
    required this.displayNameRequired,
    required this.displayNameTooLong,
    required this.displayNameInvalid,
    required this.startingPointTitle,
    required this.noArchetypeSelected,
    required this.chooseArchetypeDescription,
    required this.selectedArchetypeDescription,
    required this.chooseArchetypeLabel,
    required this.changeArchetypeLabel,
    required this.inheritanceBoundaryDescription,
    required this.capabilitySemanticsLabel,
    required this.offlineDraftLabel,
    required this.buildBlockedLabel,
    required this.runtimeUnqualifiedLabel,
    required this.notSpawnedLabel,
    required this.capabilityDescription,
    required this.archetypePicker,
  });

  static const english = Revision3NpcWizardCopy(
    title: 'Make a character draft',
    catalogLoadFailed:
        'NPC archetypes could not be refreshed from the configured game. No project or game files were changed.',
    archetypeNotQualified:
        'That archetype is not qualified for offline NPC Draft creation.',
    archetypePickerFailed:
        'The NPC archetype picker could not be opened safely.',
    archetypeRequired: 'Choose a character archetype.',
    catalogChanged:
        'The game archetype catalog changed while this wizard was open. Choose the archetype again.',
    requiresReopen:
        'This project can no longer be verified as current. Close this wizard and reopen the managed project before continuing.',
    staleCheckpoint:
        'The managed project changed while this wizard was open. Close this wizard and create the NPC draft again from the current project.',
    saveFailed:
        'The NPC draft could not be saved. Nothing was compiled, deployed, spawned, or written into the game. You can review the form and try again.',
    catalogLoadingSemanticsLabel: 'Refreshing NPC archetypes from the game',
    retryCatalogLabel: 'Refresh game choices',
    closeLabel: 'Close',
    cancelLabel: 'Cancel',
    saveLabel: 'Save draft to project',
    discardTitle: 'Discard character draft changes?',
    discardDescription:
        'Your unsaved character name and archetype choice will be lost.',
    keepEditingLabel: 'Keep editing',
    discardLabel: 'Discard',
    basicsTitle: 'Character basics',
    displayNameLabel: 'Character name',
    displayNameHint: 'North Gate Guard',
    displayNameHelp: 'Technical names and IDs are generated automatically.',
    displayNameRequired: 'Enter a character name',
    displayNameTooLong: 'Character name is too long',
    displayNameInvalid:
        'Character name cannot contain line breaks or control characters',
    startingPointTitle: 'Starting point',
    noArchetypeSelected: 'No archetype selected',
    chooseArchetypeDescription:
        'Choose an offline-qualified existing character as the structural starting point.',
    selectedArchetypeDescription:
        'Offline-qualified logical clone starting point.',
    chooseArchetypeLabel: 'Choose\u2026',
    changeArchetypeLabel: 'Change\u2026',
    inheritanceBoundaryDescription:
        'This draft inherits only the proven three-class structural chain. Visuals, faction, stats, inventory, routine, dialog, quests, and a world spawn are not authored by this step.',
    capabilitySemanticsLabel: 'NPC Draft capability limits',
    offlineDraftLabel: 'Offline draft',
    buildBlockedLabel: 'Build blocked',
    runtimeUnqualifiedLabel: 'Runtime unqualified',
    notSpawnedLabel: 'Not spawned',
    capabilityDescription:
        'This saves a logical character shell in the managed project. It does not compile, deploy, spawn a character, write game files, change a save, or prove gameplay behavior.',
    archetypePicker: StoryNpcArchetypePickerLabels(
      title: 'Choose a character archetype',
      search: 'Search characters and classes',
      showExperimental: 'Show static-linkage evidence (not selectable)',
      offlineQualified: 'Offline Draft supported',
      experimentalStaticLinkage: 'Inspect only',
      empty: 'No matching NPC archetypes.',
      spawnClass: 'Spawn definition',
      aiConfigClass: 'AI configuration',
      characterDefinitionClass: 'Character definition',
      actorBlueprint: 'Actor Blueprint',
      bodyBlueprintFamily: 'Body family',
      humanBaseFamily: 'Human base',
      humanWomanFamily: 'Human woman',
      otherFamily: 'Other',
    ),
  );

  static const german = Revision3NpcWizardCopy(
    title: 'Charakterentwurf erstellen',
    catalogLoadFailed:
        'Die Charaktervorlagen konnten nicht aktualisiert werden. Es wurden keine Dateien geändert.',
    archetypeNotQualified:
        'Diese Vorlage ist nicht für einen Offline-Entwurf freigegeben.',
    archetypePickerFailed:
        'Die Auswahl der Charaktervorlage konnte nicht sicher geöffnet werden.',
    archetypeRequired: 'Wähle eine Charaktervorlage.',
    catalogChanged:
        'Die Charaktervorlagen haben sich geändert. Wähle die Vorlage erneut.',
    requiresReopen:
        'Das Projekt ist nicht mehr sicher aktuell. Schließe diesen Dialog und öffne das Projekt erneut.',
    staleCheckpoint:
        'Das Projekt hat sich geändert. Schließe diesen Dialog und beginne erneut.',
    saveFailed:
        'Der Charakterentwurf konnte nicht gespeichert werden. Spiel und Spielstände blieben unverändert.',
    catalogLoadingSemanticsLabel: 'Charaktervorlagen werden aktualisiert',
    retryCatalogLabel: 'Spielvorlagen neu laden',
    closeLabel: 'Schließen',
    cancelLabel: 'Abbrechen',
    saveLabel: 'Entwurf im Projekt speichern',
    discardTitle: 'Änderungen am Charakterentwurf verwerfen?',
    discardDescription:
        'Der ungespeicherte Charaktername und die Vorlagenauswahl gehen verloren.',
    keepEditingLabel: 'Weiter bearbeiten',
    discardLabel: 'Verwerfen',
    basicsTitle: 'Grunddaten',
    displayNameLabel: 'Charaktername',
    displayNameHint: 'Wache am Nordtor',
    displayNameHelp: 'Technische Namen und Kennungen entstehen automatisch.',
    displayNameRequired: 'Gib einen Charakternamen ein',
    displayNameTooLong: 'Der Charaktername ist zu lang',
    displayNameInvalid:
        'Der Charaktername darf keine Zeilenumbrüche oder Steuerzeichen enthalten',
    startingPointTitle: 'Ausgangsfigur',
    noArchetypeSelected: 'Keine Vorlage ausgewählt',
    chooseArchetypeDescription:
        'Wähle eine offline geprüfte Figur als strukturellen Ausgangspunkt.',
    selectedArchetypeDescription:
        'Offline geprüfter Ausgangspunkt für den logischen Klon.',
    chooseArchetypeLabel: 'Auswählen\u2026',
    changeArchetypeLabel: 'Ändern\u2026',
    inheritanceBoundaryDescription:
        'Dieser Schritt übernimmt nur die geprüfte Klassenstruktur. Aussehen, Fraktion, Werte, Inventar, Tagesablauf, Dialoge, Quests und Spawn werden noch nicht erstellt.',
    capabilitySemanticsLabel: 'Grenzen des Charakterentwurfs',
    offlineDraftLabel: 'Offline-Entwurf',
    buildBlockedLabel: 'Build blockiert',
    runtimeUnqualifiedLabel: 'Runtime ungeprüft',
    notSpawnedLabel: 'Nicht gespawnt',
    capabilityDescription:
        'Dies speichert eine logische Charakterhülle im Projekt. Es kompiliert, installiert oder spawnt nichts und ändert weder Spiel noch Spielstand.',
    archetypePicker: StoryNpcArchetypePickerLabels(
      title: 'Charaktervorlage auswählen',
      search: 'Figuren und Klassen durchsuchen',
      showExperimental: 'Statische Hinweise anzeigen',
      offlineQualified: 'Offline-Entwurf unterstützt',
      experimentalStaticLinkage: 'Nur ansehen',
      empty: 'Keine passenden Charaktervorlagen.',
      spawnClass: 'Spawn-Definition',
      aiConfigClass: 'KI-Konfiguration',
      characterDefinitionClass: 'Charakterdefinition',
      actorBlueprint: 'Actor Blueprint',
      bodyBlueprintFamily: 'Körperfamilie',
      humanBaseFamily: 'Menschliche Basis',
      humanWomanFamily: 'Menschliche Frau',
      otherFamily: 'Andere',
    ),
  );

  final String title;
  final String catalogLoadFailed;
  final String archetypeNotQualified;
  final String archetypePickerFailed;
  final String archetypeRequired;
  final String catalogChanged;
  final String requiresReopen;
  final String staleCheckpoint;
  final String saveFailed;
  final String catalogLoadingSemanticsLabel;
  final String retryCatalogLabel;
  final String closeLabel;
  final String cancelLabel;
  final String saveLabel;
  final String discardTitle;
  final String discardDescription;
  final String keepEditingLabel;
  final String discardLabel;
  final String basicsTitle;
  final String displayNameLabel;
  final String displayNameHint;
  final String displayNameHelp;
  final String displayNameRequired;
  final String displayNameTooLong;
  final String displayNameInvalid;
  final String startingPointTitle;
  final String noArchetypeSelected;
  final String chooseArchetypeDescription;
  final String selectedArchetypeDescription;
  final String chooseArchetypeLabel;
  final String changeArchetypeLabel;
  final String inheritanceBoundaryDescription;
  final String capabilitySemanticsLabel;
  final String offlineDraftLabel;
  final String buildBlockedLabel;
  final String runtimeUnqualifiedLabel;
  final String notSpawnedLabel;
  final String capabilityDescription;
  final StoryNpcArchetypePickerLabels archetypePicker;
}

/// Guided normal-mode surface over the exact managed-R3 NPC Draft transaction.
///
/// It creates only a logical clone shell. Generated identities, source, class
/// names and paths stay hidden; spawn/build/runtime actions are intentionally
/// absent.
class Revision3NpcWizardDialog extends StatefulWidget {
  const Revision3NpcWizardDialog({
    required this.gameRoot,
    required this.loadCatalog,
    required this.publish,
    this.chooseArchetype,
    this.initialCatalogId,
    this.copy = Revision3NpcWizardCopy.english,
    super.key,
  });

  final String gameRoot;
  final Revision3NpcCatalogLoader loadCatalog;
  final Revision3NpcDraftPublisher publish;
  final Revision3NpcArchetypeChooser? chooseArchetype;
  final String? initialCatalogId;
  final Revision3NpcWizardCopy copy;

  @override
  State<Revision3NpcWizardDialog> createState() =>
      _Revision3NpcWizardDialogState();
}

class _Revision3NpcWizardDialogState extends State<Revision3NpcWizardDialog> {
  final _formKey = GlobalKey<FormState>();
  final _displayName = TextEditingController();

  Revision3NpcCatalog? _catalog;
  String? _catalogId;
  String? _error;
  bool _catalogLoading = true;
  bool _choosing = false;
  bool _publishing = false;
  bool _publicationStarted = false;
  bool _requiresReopen = false;
  bool _staleCheckpoint = false;
  bool _mayApplyInitialCatalogId = true;
  bool _baselineCatalogIdCaptured = false;
  String? _baselineCatalogId;
  bool _allowPop = false;
  bool _confirmingDiscard = false;
  int _loadGeneration = 0;

  @override
  void initState() {
    super.initState();
    _displayName.addListener(_fieldChanged);
    _loadCatalog();
  }

  @override
  void didUpdateWidget(covariant Revision3NpcWizardDialog oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.gameRoot != widget.gameRoot) _loadCatalog(clear: true);
  }

  @override
  void dispose() {
    _loadGeneration += 1;
    _displayName.removeListener(_fieldChanged);
    _displayName.dispose();
    super.dispose();
  }

  void _fieldChanged() {
    if (mounted) setState(() {});
  }

  Future<void> _loadCatalog({bool clear = false}) async {
    final generation = ++_loadGeneration;
    setState(() {
      _catalogLoading = true;
      _error = null;
      if (clear) {
        _catalog = null;
        _catalogId = null;
        _mayApplyInitialCatalogId = true;
      }
    });
    try {
      final catalog = await widget.loadCatalog(widget.gameRoot);
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _adoptCatalog(
          catalog,
          allowInitialCatalogId: _mayApplyInitialCatalogId,
        );
        if (!_baselineCatalogIdCaptured) {
          _baselineCatalogId = _catalogId;
          _baselineCatalogIdCaptured = true;
        }
        _mayApplyInitialCatalogId = false;
        _catalogLoading = false;
      });
    } catch (_) {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _catalogLoading = false;
        _error = widget.copy.catalogLoadFailed;
      });
    }
  }

  void _adoptCatalog(
    Revision3NpcCatalog catalog, {
    bool allowInitialCatalogId = false,
  }) {
    final oldSelection = _catalogId;
    final initialCatalogId = widget.initialCatalogId;
    _catalog = catalog;
    _catalogId = oldSelection != null && catalog.contains(oldSelection)
        ? oldSelection
        : allowInitialCatalogId &&
              initialCatalogId != null &&
              catalog.contains(initialCatalogId)
        ? initialCatalogId
        : null;
  }

  Future<void> _chooseArchetype() async {
    final catalog = _catalog;
    if (catalog == null || _busy || _locked) return;
    setState(() {
      _choosing = true;
      _error = null;
    });
    try {
      final selected = await (widget.chooseArchetype ?? _defaultChooser)(
        context,
        catalog,
      );
      if (!mounted || selected == null) return;
      if (!catalog.contains(selected)) {
        setState(() {
          _error = widget.copy.archetypeNotQualified;
        });
        return;
      }
      setState(() => _catalogId = selected);
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _error = widget.copy.archetypePickerFailed;
      });
    } finally {
      if (mounted) setState(() => _choosing = false);
    }
  }

  Future<String?> _defaultChooser(
    BuildContext context,
    Revision3NpcCatalog catalog,
  ) {
    final index = catalog.archetypeIndex;
    if (index == null) {
      throw const FormatException('NPC archetype evidence is unavailable.');
    }
    return showStoryNpcArchetypePicker(
      context: context,
      index: index,
      labels: widget.copy.archetypePicker,
    );
  }

  Future<void> _submit() async {
    if (_busy || _locked || _catalog == null) return;
    if (!(_formKey.currentState?.validate() ?? false)) return;
    final catalogId = _catalogId;
    if (catalogId == null) {
      setState(() => _error = widget.copy.archetypeRequired);
      return;
    }
    final input = Revision3NpcDraftAuthoringInput(
      parentCatalogId: catalogId,
      displayName: _displayName.text,
    );
    setState(() {
      _publishing = true;
      _publicationStarted = false;
      _error = null;
    });

    var completed = false;
    try {
      // Refresh immediately before the native transaction. Native code also
      // rebuilds and resolves this opaque selector from its own trusted inputs.
      final fresh = await widget.loadCatalog(widget.gameRoot);
      if (!mounted) return;
      if (!fresh.contains(catalogId)) {
        setState(() {
          _adoptCatalog(fresh);
          _publishing = false;
          _publicationStarted = false;
          _error = widget.copy.catalogChanged;
        });
        return;
      }
      setState(() => _publicationStarted = true);
      final publication = await widget.publish(
        gameRoot: widget.gameRoot,
        input: input,
      );
      if (!mounted) return;
      completed = true;
      await _popAfterUnlock(publication);
    } on Revision3NpcDraftRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _requiresReopen = true;
        _publishing = false;
        _publicationStarted = false;
        _error = widget.copy.requiresReopen;
      });
    } on Revision3NpcDraftStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _staleCheckpoint = true;
        _publishing = false;
        _publicationStarted = false;
        _error = widget.copy.staleCheckpoint;
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _publishing = false;
        _publicationStarted = false;
        _error = widget.copy.saveFailed;
      });
    } finally {
      if (mounted && !completed && _publishing) {
        setState(() {
          _publishing = false;
          _publicationStarted = false;
        });
      }
    }
  }

  bool get _busy =>
      _catalogLoading || _choosing || _publishing || _publicationStarted;
  bool get _locked => _requiresReopen || _staleCheckpoint;
  bool get _hasChanges =>
      _displayName.text.isNotEmpty ||
      (_baselineCatalogIdCaptured
          ? _catalogId != _baselineCatalogId
          : _catalogId != null);

  @override
  Widget build(BuildContext context) => PopScope(
    canPop: _allowPop || (!_busy && !_hasChanges),
    onPopInvokedWithResult: (didPop, _) {
      if (!didPop) unawaited(_requestDismiss());
    },
    child: AlertDialog(
      key: const Key('revision3-npc-wizard'),
      scrollable: true,
      title: Row(
        children: [
          const Icon(Icons.person_add_alt_1_outlined),
          const SizedBox(width: 10),
          Expanded(child: Text(widget.copy.title)),
        ],
      ),
      content: SizedBox(
        width: 680,
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxHeight: 650),
          child: SingleChildScrollView(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _NpcDraftBoundaryBanner(copy: widget.copy),
                const SizedBox(height: 16),
                if (_error != null) ...[
                  _NpcWizardMessage(
                    key: const Key('revision3-npc-wizard-error'),
                    message: _error!,
                  ),
                  const SizedBox(height: 16),
                ],
                if (_catalogLoading)
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 40),
                    child: Center(
                      child: Semantics(
                        liveRegion: true,
                        label: widget.copy.catalogLoadingSemanticsLabel,
                        child: const CircularProgressIndicator(
                          key: Key('revision3-npc-catalog-loading'),
                        ),
                      ),
                    ),
                  )
                else if (_catalog == null)
                  Center(
                    child: OutlinedButton.icon(
                      key: const Key('revision3-npc-catalog-retry'),
                      onPressed: _loadCatalog,
                      icon: const Icon(Icons.refresh),
                      label: Text(widget.copy.retryCatalogLabel),
                    ),
                  )
                else
                  _buildForm(context),
              ],
            ),
          ),
        ),
      ),
      actions: [
        TextButton(
          key: const Key('revision3-npc-cancel'),
          onPressed: _busy || _confirmingDiscard ? null : _requestDismiss,
          child: Text(
            _locked ? widget.copy.closeLabel : widget.copy.cancelLabel,
          ),
        ),
        FilledButton.icon(
          key: const Key('revision3-npc-submit'),
          onPressed: _busy || _catalog == null || _locked ? null : _submit,
          icon: _publishing
              ? const SizedBox.square(
                  dimension: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Icon(Icons.save_outlined),
          label: Text(widget.copy.saveLabel),
        ),
      ],
    ),
  );

  Future<void> _requestDismiss() async {
    if (_busy || _confirmingDiscard) return;
    if (_locked) {
      await _popAfterUnlock();
      return;
    }
    if (!_hasChanges) {
      await _popAfterUnlock();
      return;
    }
    setState(() => _confirmingDiscard = true);
    final discard = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        key: const Key('revision3-npc-discard-dialog'),
        title: Text(widget.copy.discardTitle),
        content: Text(widget.copy.discardDescription),
        actions: [
          TextButton(
            key: const Key('revision3-npc-keep-editing'),
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(widget.copy.keepEditingLabel),
          ),
          FilledButton(
            key: const Key('revision3-npc-discard'),
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(widget.copy.discardLabel),
          ),
        ],
      ),
    );
    if (!mounted) return;
    setState(() => _confirmingDiscard = false);
    if (discard == true) await _popAfterUnlock();
  }

  Future<void> _popAfterUnlock([Revision3NpcDraftPublication? result]) async {
    if (!mounted) return;
    setState(() => _allowPop = true);
    await WidgetsBinding.instance.endOfFrame;
    if (mounted) Navigator.of(context).pop(result);
  }

  Widget _buildForm(BuildContext context) {
    final selected = _catalog?.choice(_catalogId ?? '');
    final enabled = !_busy && !_locked;
    final compactArchetype =
        MediaQuery.sizeOf(context).width < 760 ||
        MediaQuery.textScalerOf(context).scale(1) >= 1.5;
    Widget archetypeDetails() => Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Icon(Icons.person_search_outlined),
        const SizedBox(width: 10),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                selected?.displayName ?? widget.copy.noArchetypeSelected,
                key: const Key('revision3-npc-selected-archetype-label'),
                style: Theme.of(context).textTheme.titleSmall,
              ),
              const SizedBox(height: 2),
              Text(
                selected == null
                    ? widget.copy.chooseArchetypeDescription
                    : widget.copy.selectedArchetypeDescription,
              ),
            ],
          ),
        ),
      ],
    );
    Widget archetypeButton() => OutlinedButton(
      key: const Key('revision3-npc-choose-archetype'),
      onPressed: enabled ? _chooseArchetype : null,
      child: Text(
        selected == null
            ? widget.copy.chooseArchetypeLabel
            : widget.copy.changeArchetypeLabel,
      ),
    );
    return Form(
      key: _formKey,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            widget.copy.basicsTitle,
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 10),
          TextFormField(
            key: const Key('revision3-npc-display-name'),
            controller: _displayName,
            enabled: enabled,
            maxLength: 256,
            textInputAction: TextInputAction.done,
            decoration: InputDecoration(
              labelText: widget.copy.displayNameLabel,
              hintText: widget.copy.displayNameHint,
              helperText: widget.copy.displayNameHelp,
              border: const OutlineInputBorder(),
            ),
            validator: _validateNpcName,
          ),
          const SizedBox(height: 16),
          Text(
            widget.copy.startingPointTitle,
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 8),
          Container(
            key: const Key('revision3-npc-selected-archetype'),
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              border: Border.all(color: Theme.of(context).colorScheme.outline),
              borderRadius: BorderRadius.circular(8),
            ),
            child: compactArchetype
                ? Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      archetypeDetails(),
                      const SizedBox(height: 10),
                      Align(
                        alignment: Alignment.centerLeft,
                        child: archetypeButton(),
                      ),
                    ],
                  )
                : Row(
                    children: [
                      Expanded(child: archetypeDetails()),
                      const SizedBox(width: 8),
                      archetypeButton(),
                    ],
                  ),
          ),
          const SizedBox(height: 14),
          Text(widget.copy.inheritanceBoundaryDescription),
        ],
      ),
    );
  }

  String? _validateNpcName(String? value) {
    final normalized = value?.trim() ?? '';
    if (normalized.isEmpty) return widget.copy.displayNameRequired;
    if (utf8.encode(normalized).length > 256) {
      return widget.copy.displayNameTooLong;
    }
    if (normalized.runes.any(
      (rune) => rune < 0x20 || (rune >= 0x7f && rune <= 0x9f),
    )) {
      return widget.copy.displayNameInvalid;
    }
    return null;
  }
}

class _NpcDraftBoundaryBanner extends StatelessWidget {
  const _NpcDraftBoundaryBanner({required this.copy});

  final Revision3NpcWizardCopy copy;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Semantics(
      container: true,
      label: copy.capabilitySemanticsLabel,
      child: Container(
        key: const Key('revision3-npc-boundary'),
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
                Chip(label: Text(copy.offlineDraftLabel)),
                Chip(label: Text(copy.buildBlockedLabel)),
                Chip(label: Text(copy.runtimeUnqualifiedLabel)),
                Chip(label: Text(copy.notSpawnedLabel)),
              ],
            ),
            const SizedBox(height: 8),
            Text(
              copy.capabilityDescription,
              style: TextStyle(color: scheme.onSecondaryContainer),
            ),
          ],
        ),
      ),
    );
  }
}

class _NpcWizardMessage extends StatelessWidget {
  const _NpcWizardMessage({required this.message, super.key});

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
