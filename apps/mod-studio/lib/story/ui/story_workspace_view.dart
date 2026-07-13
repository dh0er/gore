import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter/material.dart';

import '../../core/mod_ffi.dart';
import '../domain/story_catalog_adapter.dart';
import '../domain/story_draft_requests.dart';
import '../domain/story_workspace_controller.dart';
import 'story_npc_archetype_picker.dart';

typedef StoryNpcDraftCreator =
    Future<StoryDraftCreateResult> Function(StoryNpcDraftInput input);
typedef StoryBuildReadinessChecker =
    Future<StoryBuildReadinessCheckResult> Function();

const _npcPickerLabels = StoryNpcArchetypePickerLabels(
  title: 'Choose an NPC archetype',
  search: 'Search by name, class, or Blueprint',
  showExperimental: 'Show experimental archetypes',
  offlineQualified: 'Offline clone qualified',
  experimentalStaticLinkage: 'Experimental · static linkage only',
  empty: 'No NPC archetypes match this search.',
  spawnClass: 'Spawn class',
  aiConfigClass: 'AI config class',
  characterDefinitionClass: 'Character definition class',
  actorBlueprint: 'Actor Blueprint',
  bodyBlueprintFamily: 'Body/Blueprint family',
  humanBaseFamily: 'Human base family',
  humanWomanFamily: 'Human-woman family',
  otherFamily: 'Other family',
);

/// Friendly, draft-only Story authoring surface.
///
/// The production caller supplies `StoryWorkspaceController.createNpc`; this
/// widget owns only transient form state and the latest verified projection
/// returned by that callback.
final class StoryWorkspaceView extends StatefulWidget {
  const StoryWorkspaceView({
    required this.initialState,
    required this.catalog,
    required this.createNpc,
    required this.checkBuildPlan,
    super.key,
  });

  final StoryWorkspaceState initialState;
  final StoryCatalogAdapter catalog;
  final StoryNpcDraftCreator createNpc;
  final StoryBuildReadinessChecker checkBuildPlan;

  @override
  State<StoryWorkspaceView> createState() => _StoryWorkspaceViewState();
}

final class _StoryWorkspaceViewState extends State<StoryWorkspaceView> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _displayNameController;
  late final TextEditingController _moduleNamespaceController;
  late final TextEditingController _uniqueNameController;
  late StoryWorkspaceState _workspace;
  String? _selectedCatalogId;
  String? _selectedDraftId;
  bool _busy = false;
  bool _checkingBuildPlan = false;
  String? _notice;
  String? _error;
  StoryBuildReadinessChecked? _buildReadiness;
  String? _buildReadinessNotice;
  String? _buildReadinessError;
  bool _technicalFieldsCustomized = false;
  List<AuthoringDiagnostic> _diagnostics = const <AuthoringDiagnostic>[];

  @override
  void initState() {
    super.initState();
    _workspace = widget.initialState;
    _selectedDraftId = _workspace.drafts.isEmpty
        ? null
        : _workspace.drafts.first.draftId;
    _selectedCatalogId = widget.catalog.npcChoices.isEmpty
        ? null
        : widget.catalog.npcChoices.first.catalogId;
    _displayNameController = TextEditingController();
    _moduleNamespaceController = TextEditingController();
    _uniqueNameController = TextEditingController();
    _applyChoiceDefaults();
  }

  @override
  void didUpdateWidget(covariant StoryWorkspaceView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!_managedActionBusy &&
        !identical(oldWidget.initialState, widget.initialState)) {
      _adoptWorkspace(widget.initialState);
      _clearBuildReadiness();
    }
    if (!identical(oldWidget.catalog, widget.catalog) &&
        !widget.catalog.npcChoices.any(
          (choice) => choice.catalogId == _selectedCatalogId,
        )) {
      _selectedCatalogId = widget.catalog.npcChoices.isEmpty
          ? null
          : widget.catalog.npcChoices.first.catalogId;
      _applyChoiceDefaults();
    }
  }

  @override
  void dispose() {
    _displayNameController.dispose();
    _moduleNamespaceController.dispose();
    _uniqueNameController.dispose();
    super.dispose();
  }

  StoryCatalogNpcChoice? get _selectedChoice {
    for (final choice in widget.catalog.npcChoices) {
      if (choice.catalogId == _selectedCatalogId) return choice;
    }
    return null;
  }

  StoryDraftState? get _selectedDraft =>
      _workspace.draftById(_selectedDraftId ?? '');

  bool get _managedActionBusy => _busy || _checkingBuildPlan;

  void _applyChoiceDefaults() {
    final choice = _selectedChoice;
    if (choice == null) return;
    _displayNameController.text = '${choice.displayName} Copy';
    _technicalFieldsCustomized = false;
    _applyTechnicalDefaults(_displayNameController.text);
  }

  void _applyTechnicalDefaults(String displayName) {
    final choice = _selectedChoice;
    if (choice == null) return;
    final token = _friendlyIdentifier(displayName);
    final usedNamespaces = <String>{
      for (final draft in _workspace.drafts)
        draft.moduleNamespace.toLowerCase(),
    };
    final usedRuntimeIds = <String>{
      for (final draft in _workspace.drafts) draft.runtimeId.toLowerCase(),
    };
    // One draft can independently occupy one namespace and one runtime ID. The workspace is
    // bounded, so trying one more salt than both sets combined closes the deterministic search.
    final maxAttempts = (_workspace.drafts.length * 2) + 1;
    for (var salt = 0; salt < maxAttempts; salt++) {
      final suffix = crypto.sha256
          .convert(
            utf8.encode(
              '${choice.catalogId}\u0000$displayName\u0000${_workspace.projectId}'
              '\u0000${_workspace.revision}\u0000${_workspace.drafts.length}'
              '\u0000$salt',
            ),
          )
          .toString()
          .substring(0, 32)
          .toUpperCase();
      final namespace = 'GoreMods.Npcs.${token}_$suffix';
      final runtimeId = 'Gore${token}_$suffix';
      if (usedNamespaces.contains(namespace.toLowerCase()) ||
          usedRuntimeIds.contains(runtimeId.toLowerCase())) {
        continue;
      }
      _moduleNamespaceController.text = namespace;
      _uniqueNameController.text = runtimeId;
      return;
    }
    throw StateError('could not derive a collision-free Story identity');
  }

  void _adoptWorkspace(
    StoryWorkspaceState workspace, {
    String? preferredDraftId,
  }) {
    _workspace = workspace;
    final candidate = preferredDraftId ?? _selectedDraftId;
    _selectedDraftId =
        candidate != null && _workspace.draftById(candidate) != null
        ? candidate
        : (_workspace.drafts.isEmpty ? null : _workspace.drafts.first.draftId);
    if (!_technicalFieldsCustomized) {
      _applyTechnicalDefaults(_displayNameController.text.trim());
    }
  }

  void _clearBuildReadiness() {
    _buildReadiness = null;
    _buildReadinessNotice = null;
    _buildReadinessError = null;
  }

  void _selectNpc(String? catalogId) {
    if (catalogId == null || _managedActionBusy) return;
    setState(() {
      _selectedCatalogId = catalogId;
      _notice = null;
      _error = null;
      _diagnostics = const <AuthoringDiagnostic>[];
      _applyChoiceDefaults();
    });
  }

  Future<void> _pickNpcArchetype() async {
    final index = widget.catalog.npcArchetypeIndex;
    if (index == null || _managedActionBusy) return;
    final selected = await showStoryNpcArchetypePicker(
      context: context,
      index: index,
      labels: _npcPickerLabels,
    );
    if (!mounted) return;
    _selectNpc(selected);
  }

  Future<void> _createDraft() async {
    if (_managedActionBusy || !_formKey.currentState!.validate()) return;
    final catalogId = _selectedCatalogId;
    if (catalogId == null) return;
    setState(() {
      _busy = true;
      _notice = null;
      _error = null;
      _diagnostics = const <AuthoringDiagnostic>[];
      _clearBuildReadiness();
    });

    try {
      final input = widget.catalog.createNpcDraftInput(
        catalogId: catalogId,
        displayName: _displayNameController.text.trim(),
        moduleNamespace: _moduleNamespaceController.text.trim(),
        uniqueName: _uniqueNameController.text.trim(),
      );
      final result = await widget.createNpc(input);
      if (!mounted) return;
      switch (result) {
        case StoryDraftCreateApplied applied:
          setState(() {
            _busy = false;
            _adoptWorkspace(
              applied.state,
              preferredDraftId: applied.draft.draftId,
            );
            _diagnostics = _userFacingDiagnostics(applied.diagnostics);
            _notice = 'Draft "${applied.draft.displayName}" created and saved.';
          });
        case StoryDraftCreateRejected rejected:
          setState(() {
            _busy = false;
            _adoptWorkspace(rejected.state);
            _diagnostics = _userFacingDiagnostics(rejected.diagnostics);
            _error =
                'The draft could not be created. Review the details below.';
          });
      }
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _error =
            'Something went wrong while saving the draft. Please try again.';
      });
    }
  }

  Future<void> _checkBuildReadiness() async {
    if (_managedActionBusy) return;
    setState(() {
      _checkingBuildPlan = true;
      _clearBuildReadiness();
    });
    try {
      final result = await widget.checkBuildPlan();
      if (!mounted) return;
      setState(() {
        _checkingBuildPlan = false;
        switch (result) {
          case StoryBuildReadinessChecked checked:
            _buildReadiness = checked;
          case StoryBuildReadinessStale():
            _buildReadiness = null;
            _buildReadinessNotice =
                'The workspace changed during the check. Run it again for the latest revision.';
        }
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _checkingBuildPlan = false;
        _buildReadiness = null;
        _buildReadinessError =
            'Build readiness could not be checked safely. No build or deployment was started.';
      });
    }
  }

  @override
  Widget build(BuildContext context) => SingleChildScrollView(
    padding: const EdgeInsets.all(24),
    child: Align(
      alignment: Alignment.topCenter,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 960),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            _buildDraftBanner(context),
            const SizedBox(height: 16),
            _buildReadinessCard(context),
            const SizedBox(height: 16),
            _buildNpcCard(context),
            const SizedBox(height: 16),
            _buildDraftsCard(context),
            const SizedBox(height: 16),
            _buildQuestCard(context),
          ],
        ),
      ),
    ),
  );

  Widget _buildDraftBanner(BuildContext context) => Semantics(
    container: true,
    label: 'Draft-only Story workspace warning',
    child: Card(
      color: Theme.of(context).colorScheme.tertiaryContainer,
      child: const Padding(
        padding: EdgeInsets.all(16),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Icon(Icons.science_outlined),
            SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    'Draft mode only',
                    style: TextStyle(fontWeight: FontWeight.bold),
                  ),
                  SizedBox(height: 4),
                  Text(
                    'NPCs created here are offline drafts and are not runtime-qualified yet.',
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    ),
  );

  Widget _buildReadinessCard(BuildContext context) {
    final report = _buildReadiness;
    final blockerCount = report?.blockingDiagnosticCount ?? 0;
    final additionalBlockers = blockerCount > 0 ? blockerCount - 1 : 0;
    return Card(
      key: const Key('story-build-readiness-card'),
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Text(
              'Build readiness',
              style: Theme.of(context).textTheme.titleLarge,
            ),
            const SizedBox(height: 6),
            const Text(
              'Inspect the current saved Story revision for known blockers.',
            ),
            const SizedBox(height: 6),
            const Text(
              'Inspection only: this does not compile, build, deploy, publish, or runtime-qualify the mod.',
            ),
            const SizedBox(height: 16),
            Align(
              alignment: Alignment.centerLeft,
              child: FilledButton.tonalIcon(
                key: const Key('story-check-build-plan-button'),
                onPressed: _managedActionBusy ? null : _checkBuildReadiness,
                icon: _checkingBuildPlan
                    ? const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.fact_check_outlined),
                label: Text(
                  _checkingBuildPlan
                      ? 'Checking readiness...'
                      : 'Check build readiness',
                ),
              ),
            ),
            if (report != null) ...<Widget>[
              const SizedBox(height: 16),
              Semantics(
                key: const Key('story-build-plan-result'),
                liveRegion: true,
                label: 'Build readiness result',
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      'Not build-ready yet',
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                    const SizedBox(height: 6),
                    Text(
                      'Checked saved revision ${report.projectRevision}: '
                      '${report.moduleCount} generated source module${report.moduleCount == 1 ? '' : 's'} inspected.',
                    ),
                    Text(
                      '$blockerCount blocking diagnostic${blockerCount == 1 ? '' : 's'} '
                      'across ${report.diagnosticCount} total diagnostic${report.diagnosticCount == 1 ? '' : 's'}.',
                    ),
                    const SizedBox(height: 8),
                    const Text(
                      '- Combined Story, voice, localization, and asset validation is not available yet.',
                    ),
                    if (additionalBlockers > 0)
                      Text(
                        '- $additionalBlockers additional project blocker${additionalBlockers == 1 ? '' : 's'} need attention.',
                      ),
                    const Text(
                      '- Runtime qualification and publishing remain unavailable.',
                    ),
                  ],
                ),
              ),
            ],
            if (_buildReadinessNotice != null) ...<Widget>[
              const SizedBox(height: 14),
              _StatusMessage(
                key: const Key('story-build-plan-stale'),
                icon: Icons.refresh_outlined,
                color: Theme.of(context).colorScheme.primary,
                message: _buildReadinessNotice!,
              ),
            ],
            if (_buildReadinessError != null) ...<Widget>[
              const SizedBox(height: 14),
              _StatusMessage(
                key: const Key('story-build-plan-error'),
                icon: Icons.error_outline,
                color: Theme.of(context).colorScheme.error,
                message: _buildReadinessError!,
              ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _buildNpcCard(BuildContext context) => Card(
    child: Padding(
      padding: const EdgeInsets.all(20),
      child: Form(
        key: _formKey,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Text(
              'Create an NPC draft',
              style: Theme.of(context).textTheme.titleLarge,
            ),
            const SizedBox(height: 6),
            const Text(
              'Start from a known character, then give your new NPC a friendly name and identity.',
            ),
            const SizedBox(height: 20),
            if (widget.catalog.npcArchetypeIndex != null)
              OutlinedButton(
                key: const Key('story-open-npc-archetype-picker'),
                onPressed: _managedActionBusy ? null : _pickNpcArchetype,
                style: OutlinedButton.styleFrom(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 16,
                    vertical: 14,
                  ),
                  alignment: Alignment.centerLeft,
                ),
                child: Row(
                  children: <Widget>[
                    const Icon(Icons.people_alt_outlined),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: <Widget>[
                          Text(
                            _selectedChoice?.displayName ??
                                'Choose an NPC archetype',
                            style: Theme.of(context).textTheme.titleSmall,
                          ),
                          const SizedBox(height: 2),
                          const Text(
                            'Search verified defaults; experimental archetypes stay disabled.',
                          ),
                        ],
                      ),
                    ),
                    const Icon(Icons.arrow_drop_down),
                  ],
                ),
              )
            else
              DropdownButtonFormField<String>(
                key: ValueKey<String?>(_selectedCatalogId),
                initialValue: _selectedCatalogId,
                isExpanded: true,
                decoration: const InputDecoration(
                  labelText: 'Start from NPC',
                  helperText:
                      'Copies safe defaults from the selected character.',
                  border: OutlineInputBorder(),
                ),
                items: <DropdownMenuItem<String>>[
                  for (final choice in widget.catalog.npcChoices)
                    DropdownMenuItem<String>(
                      value: choice.catalogId,
                      child: Text(choice.displayName),
                    ),
                ],
                onChanged: _managedActionBusy ? null : _selectNpc,
              ),
            const SizedBox(height: 16),
            TextFormField(
              key: const Key('story-display-name-field'),
              controller: _displayNameController,
              enabled: !_managedActionBusy,
              textInputAction: TextInputAction.next,
              decoration: const InputDecoration(
                labelText: 'Display name',
                hintText: 'For example: Mine Guard Arko',
                border: OutlineInputBorder(),
              ),
              onChanged: (value) {
                if (!_technicalFieldsCustomized) {
                  _applyTechnicalDefaults(value.trim());
                }
              },
              validator: (value) => _required(value, 'Enter a display name.'),
            ),
            const SizedBox(height: 16),
            ExpansionTile(
              key: const Key('story-technical-advanced'),
              tilePadding: EdgeInsets.zero,
              childrenPadding: const EdgeInsets.only(bottom: 8),
              title: const Text('Advanced: technical identity (optional)'),
              subtitle: const Text(
                'Safe values are generated automatically for normal use.',
              ),
              children: <Widget>[
                TextFormField(
                  key: const Key('story-module-namespace-field'),
                  controller: _moduleNamespaceController,
                  enabled: !_managedActionBusy,
                  textInputAction: TextInputAction.next,
                  autocorrect: false,
                  decoration: const InputDecoration(
                    labelText: 'Module namespace',
                    helperText: 'Optional location inside your mod.',
                    border: OutlineInputBorder(),
                  ),
                  onChanged: (_) => _technicalFieldsCustomized = true,
                  validator: _moduleNamespaceError,
                ),
                const SizedBox(height: 16),
                TextFormField(
                  key: const Key('story-unique-name-field'),
                  controller: _uniqueNameController,
                  enabled: !_managedActionBusy,
                  textInputAction: TextInputAction.done,
                  autocorrect: false,
                  onFieldSubmitted: (_) => _createDraft(),
                  decoration: const InputDecoration(
                    labelText: 'Unique name',
                    helperText: 'Optional internal identity for this NPC.',
                    border: OutlineInputBorder(),
                  ),
                  onChanged: (_) => _technicalFieldsCustomized = true,
                  validator: _uniqueNameError,
                ),
                Align(
                  alignment: Alignment.centerLeft,
                  child: TextButton.icon(
                    onPressed: _managedActionBusy
                        ? null
                        : () {
                            setState(() {
                              _technicalFieldsCustomized = false;
                              _applyTechnicalDefaults(
                                _displayNameController.text.trim(),
                              );
                            });
                          },
                    icon: const Icon(Icons.auto_fix_high_outlined),
                    label: const Text('Reset to automatic values'),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 18),
            Align(
              alignment: Alignment.centerLeft,
              child: FilledButton.icon(
                key: const Key('story-create-npc-button'),
                onPressed: _managedActionBusy ? null : _createDraft,
                icon: _busy
                    ? const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.add),
                label: Text(_busy ? 'Creating draft...' : 'Create draft'),
              ),
            ),
            if (_notice != null) ...<Widget>[
              const SizedBox(height: 14),
              _StatusMessage(
                key: const Key('story-success-message'),
                icon: Icons.check_circle_outline,
                color: Theme.of(context).colorScheme.primary,
                message: _notice!,
              ),
            ],
            if (_error != null) ...<Widget>[
              const SizedBox(height: 14),
              _StatusMessage(
                key: const Key('story-error-message'),
                icon: Icons.error_outline,
                color: Theme.of(context).colorScheme.error,
                message: _error!,
              ),
            ],
            if (_diagnostics.isNotEmpty) ...<Widget>[
              const SizedBox(height: 10),
              Semantics(
                liveRegion: true,
                label: 'Draft diagnostics',
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: <Widget>[
                    for (final diagnostic in _diagnostics)
                      Padding(
                        padding: const EdgeInsets.only(top: 4),
                        child: Text('- ${diagnostic.message}'),
                      ),
                  ],
                ),
              ),
            ],
          ],
        ),
      ),
    ),
  );

  Widget _buildDraftsCard(BuildContext context) {
    final selected = _selectedDraft;
    return Card(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 8, 20, 10),
              child: Text(
                'Saved drafts',
                style: Theme.of(context).textTheme.titleLarge,
              ),
            ),
            if (_workspace.drafts.isEmpty)
              const Padding(
                padding: EdgeInsets.fromLTRB(20, 4, 20, 16),
                child: Text('No Story drafts yet.'),
              )
            else ...<Widget>[
              for (final draft in _workspace.drafts)
                ListTile(
                  key: Key('story-draft-${draft.draftId}'),
                  selected: draft.draftId == _selectedDraftId,
                  leading: const Icon(Icons.person_outline),
                  title: Text(draft.displayName),
                  subtitle: Text(
                    '${_draftKindLabel(draft.kind)} - Offline, not runtime-qualified',
                  ),
                  onTap: () => setState(() => _selectedDraftId = draft.draftId),
                ),
              if (selected != null)
                ExpansionTile(
                  key: ValueKey<String>('story-source-${selected.draftId}'),
                  title: const Text('Advanced: generated source'),
                  subtitle: Text('Selected draft: ${selected.displayName}'),
                  children: <Widget>[
                    Container(
                      width: double.infinity,
                      margin: const EdgeInsets.fromLTRB(20, 0, 20, 16),
                      padding: const EdgeInsets.all(12),
                      color: Theme.of(
                        context,
                      ).colorScheme.surfaceContainerHighest,
                      child: SelectableText(
                        selected.source,
                        key: const Key('story-generated-source'),
                        style: const TextStyle(fontFamily: 'monospace'),
                      ),
                    ),
                  ],
                ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _buildQuestCard(BuildContext context) {
    final availability = widget.catalog.questAvailability;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Row(
              children: <Widget>[
                Expanded(
                  child: Text(
                    'Create a Quest draft',
                    style: Theme.of(context).textTheme.titleLarge,
                  ),
                ),
                const Chip(
                  avatar: Icon(Icons.lock_outline, size: 18),
                  label: Text('Not available yet'),
                ),
              ],
            ),
            const SizedBox(height: 8),
            const Text(
              'Quest creation is temporarily unavailable because the exact script collision inventory has not been verified yet.',
              key: Key('story-quest-disabled-reason'),
            ),
            const SizedBox(height: 16),
            _DisabledChoiceList(
              label: 'Quest family choices',
              choices: <String>[
                for (final choice in availability.parents) choice.displayName,
              ],
            ),
            const SizedBox(height: 16),
            _DisabledChoiceList(
              label: 'Quest giver choices',
              choices: <String>[
                for (final choice in availability.givers) choice.displayName,
              ],
            ),
          ],
        ),
      ),
    );
  }
}

final class _DisabledChoiceList extends StatelessWidget {
  const _DisabledChoiceList({required this.label, required this.choices});

  final String label;
  final List<String> choices;

  @override
  Widget build(BuildContext context) => Semantics(
    enabled: false,
    label: label,
    child: InputDecorator(
      decoration: InputDecoration(
        labelText: label,
        enabled: false,
        border: const OutlineInputBorder(),
      ),
      child: Wrap(
        spacing: 8,
        runSpacing: 8,
        children: <Widget>[
          for (final choice in choices) Chip(label: Text(choice)),
        ],
      ),
    ),
  );
}

final class _StatusMessage extends StatelessWidget {
  const _StatusMessage({
    required this.icon,
    required this.color,
    required this.message,
    super.key,
  });

  final IconData icon;
  final Color color;
  final String message;

  @override
  Widget build(BuildContext context) => Semantics(
    liveRegion: true,
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Icon(icon, color: color),
        const SizedBox(width: 8),
        Expanded(child: Text(message)),
      ],
    ),
  );
}

String? _required(String? value, String message) =>
    value == null || value.trim().isEmpty ? message : null;

String? _moduleNamespaceError(String? raw) {
  final required = _required(raw, 'Enter a module namespace.');
  if (required != null) return required;
  final value = raw!.trim();
  if (value.length > 255 ||
      !RegExp(
        r'^[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*$',
      ).hasMatch(value)) {
    return 'Use names separated by dots, for example GoreMods.Npcs.Arko.';
  }
  return null;
}

String? _uniqueNameError(String? raw) {
  final required = _required(raw, 'Enter a unique name.');
  if (required != null) return required;
  final value = raw!.trim();
  if (value.length > 64 ||
      !RegExp(r'^[A-Za-z_][A-Za-z0-9_]*$').hasMatch(value)) {
    return 'Use up to 64 letters, numbers, or underscores.';
  }
  return null;
}

String _friendlyIdentifier(String value) {
  final compact = value.replaceAll(RegExp(r'[^A-Za-z0-9_]'), '');
  if (compact.isEmpty || !RegExp(r'^[A-Za-z_]').hasMatch(compact)) {
    return 'Npc';
  }
  return compact.length <= 27 ? compact : compact.substring(0, 27);
}

String _draftKindLabel(AuthoringStoryDraftKind kind) => switch (kind) {
  AuthoringStoryDraftKind.npcDraft => 'NPC Draft',
  AuthoringStoryDraftKind.questDraft => 'Quest Draft',
};

List<AuthoringDiagnostic> _userFacingDiagnostics(
  List<AuthoringDiagnostic> diagnostics,
) => List<AuthoringDiagnostic>.unmodifiable(
  diagnostics.where(
    (diagnostic) =>
        diagnostic.code != 'REVISION2_COMBINED_VALIDATION_UNAVAILABLE',
  ),
);
