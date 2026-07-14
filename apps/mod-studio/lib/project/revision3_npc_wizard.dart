import 'dart:convert';

import 'package:flutter/material.dart';

import '../story/ui/story_npc_archetype_picker.dart';
import 'revision3_npc_authoring.dart';

typedef Revision3NpcArchetypeChooser =
    Future<String?> Function(BuildContext context, Revision3NpcCatalog catalog);

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
    super.key,
  });

  final String gameRoot;
  final Revision3NpcCatalogLoader loadCatalog;
  final Revision3NpcDraftPublisher publish;
  final Revision3NpcArchetypeChooser? chooseArchetype;

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
  int _loadGeneration = 0;

  @override
  void initState() {
    super.initState();
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
    _displayName.dispose();
    super.dispose();
  }

  Future<void> _loadCatalog({bool clear = false}) async {
    final generation = ++_loadGeneration;
    setState(() {
      _catalogLoading = true;
      _error = null;
      if (clear) {
        _catalog = null;
        _catalogId = null;
      }
    });
    try {
      final catalog = await widget.loadCatalog(widget.gameRoot);
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _adoptCatalog(catalog);
        _catalogLoading = false;
      });
    } catch (_) {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _catalogLoading = false;
        _error =
            'NPC archetypes could not be refreshed from the configured game. No project or game files were changed.';
      });
    }
  }

  void _adoptCatalog(Revision3NpcCatalog catalog) {
    final oldSelection = _catalogId;
    _catalog = catalog;
    _catalogId = oldSelection != null && catalog.contains(oldSelection)
        ? oldSelection
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
          _error =
              'That archetype is not qualified for offline NPC Draft creation.';
        });
        return;
      }
      setState(() => _catalogId = selected);
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _error = 'The NPC archetype picker could not be opened safely.';
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
      labels: const StoryNpcArchetypePickerLabels(
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
  }

  Future<void> _submit() async {
    if (_busy || _locked || _catalog == null) return;
    if (!(_formKey.currentState?.validate() ?? false)) return;
    final catalogId = _catalogId;
    if (catalogId == null) {
      setState(() => _error = 'Choose a character archetype.');
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
          _error =
              'The game archetype catalog changed while this wizard was open. Choose the archetype again.';
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
      Navigator.of(context).pop(publication);
    } on Revision3NpcDraftRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _requiresReopen = true;
        _publishing = false;
        _publicationStarted = false;
        _error =
            'This project can no longer be verified as current. Close this wizard and reopen the managed project before continuing.';
      });
    } on Revision3NpcDraftStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _staleCheckpoint = true;
        _publishing = false;
        _publicationStarted = false;
        _error =
            'The managed project changed while this wizard was open. Close this wizard and create the NPC draft again from the current project.';
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _publishing = false;
        _publicationStarted = false;
        _error =
            'The NPC draft could not be saved. Nothing was compiled, deployed, spawned, or written into the game. You can review the form and try again.';
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

  bool get _busy => _catalogLoading || _choosing || _publishing;
  bool get _locked => _requiresReopen || _staleCheckpoint;

  @override
  Widget build(BuildContext context) => PopScope(
    canPop: !_publicationStarted,
    child: AlertDialog(
      key: const Key('revision3-npc-wizard'),
      title: const Row(
        children: [
          Icon(Icons.person_add_alt_1_outlined),
          SizedBox(width: 10),
          Expanded(child: Text('Make a character draft')),
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
                const _NpcDraftBoundaryBanner(),
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
                        label: 'Refreshing NPC archetypes from the game',
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
                      label: const Text('Refresh game choices'),
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
          onPressed: _publicationStarted
              ? null
              : () => Navigator.of(context).pop(),
          child: Text(_locked ? 'Close' : 'Cancel'),
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
          label: const Text('Save draft to project'),
        ),
      ],
    ),
  );

  Widget _buildForm(BuildContext context) {
    final selected = _catalog?.choice(_catalogId ?? '');
    final enabled = !_busy && !_locked;
    return Form(
      key: _formKey,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Character basics',
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 10),
          TextFormField(
            key: const Key('revision3-npc-display-name'),
            controller: _displayName,
            enabled: enabled,
            maxLength: 256,
            textInputAction: TextInputAction.done,
            decoration: const InputDecoration(
              labelText: 'Character name',
              hintText: 'North Gate Guard',
              helperText:
                  'Technical names and IDs are generated automatically.',
              border: OutlineInputBorder(),
            ),
            validator: _validateNpcName,
          ),
          const SizedBox(height: 16),
          Text(
            'Starting point',
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
            child: Row(
              children: [
                const Icon(Icons.person_search_outlined),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        selected?.displayName ?? 'No archetype selected',
                        style: Theme.of(context).textTheme.titleSmall,
                      ),
                      const SizedBox(height: 2),
                      Text(
                        selected == null
                            ? 'Choose an offline-qualified existing character as the structural starting point.'
                            : 'Offline-qualified logical clone starting point.',
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                OutlinedButton(
                  key: const Key('revision3-npc-choose-archetype'),
                  onPressed: enabled ? _chooseArchetype : null,
                  child: Text(selected == null ? 'Choose…' : 'Change…'),
                ),
              ],
            ),
          ),
          const SizedBox(height: 14),
          const Text(
            'This draft inherits only the proven three-class structural chain. Visuals, faction, stats, inventory, routine, dialog, quests, and a world spawn are not authored by this step.',
          ),
        ],
      ),
    );
  }
}

class _NpcDraftBoundaryBanner extends StatelessWidget {
  const _NpcDraftBoundaryBanner();

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Semantics(
      container: true,
      label: 'NPC Draft capability limits',
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
            const Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                Chip(label: Text('Offline draft')),
                Chip(label: Text('Build blocked')),
                Chip(label: Text('Runtime unqualified')),
                Chip(label: Text('Not spawned')),
              ],
            ),
            const SizedBox(height: 8),
            Text(
              'This saves a logical character shell in the managed project. It does not compile, deploy, spawn a character, write game files, change a save, or prove gameplay behavior.',
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

String? _validateNpcName(String? value) {
  final normalized = value?.trim() ?? '';
  if (normalized.isEmpty) return 'Enter a character name';
  if (utf8.encode(normalized).length > 256) {
    return 'Character name is too long';
  }
  if (normalized.runes.any(
    (rune) => rune < 0x20 || (rune >= 0x7f && rune <= 0x9f),
  )) {
    return 'Character name cannot contain line breaks or control characters';
  }
  return null;
}
