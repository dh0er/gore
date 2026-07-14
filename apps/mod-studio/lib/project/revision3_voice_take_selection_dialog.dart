import 'package:flutter/material.dart';

import '../core/mod_ffi.dart';
import 'revision3_voice_authoring.dart';
import 'revision3_voice_take_selection_authoring.dart';

const _clearSelectionValue = '__no_voice_take_selected__';

/// Friendly project-only editor for selecting or clearing an existing Voice
/// take. Entity IDs, CAS heads, paths, and archive internals are never shown.
class Revision3VoiceTakeSelectionDialog extends StatefulWidget {
  const Revision3VoiceTakeSelectionDialog({super.key, required this.service});

  final Revision3VoiceTakeSelectionAuthoringService service;

  @override
  State<Revision3VoiceTakeSelectionDialog> createState() =>
      _Revision3VoiceTakeSelectionDialogState();
}

class _Revision3VoiceTakeSelectionDialogState
    extends State<Revision3VoiceTakeSelectionDialog> {
  final _searchController = TextEditingController();
  Revision3VoiceCatalog? _catalog;
  String? _lineId;
  String? _locale;
  String? _selectionValue;
  String? _error;
  bool _loading = true;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    _searchController.addListener(_searchChanged);
    _load();
  }

  @override
  void dispose() {
    _searchController
      ..removeListener(_searchChanged)
      ..dispose();
    super.dispose();
  }

  void _searchChanged() {
    if (mounted) setState(() {});
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final catalog = await widget.service.loadCatalog();
      if (!mounted) return;
      setState(() {
        _catalog = catalog;
        _lineId = null;
        _locale = null;
        _selectionValue = null;
        _loading = false;
      });
    } on Revision3VoiceTakeSelectionRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _loading = false;
        _error = 'Reopen the managed project before changing Voice takes.';
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _loading = false;
        _error = 'Voice takes could not be loaded: ${_friendlyError(error)}';
      });
    }
  }

  List<Revision3VoiceDialogLineChoice> get _visibleLines {
    final catalog = _catalog;
    if (catalog == null) return const [];
    final query = _searchController.text.trim().toLowerCase();
    return catalog.lines
        .where((line) {
          if (_intactLocales(line).isEmpty) return false;
          return query.isEmpty || line.matches(query);
        })
        .toList(growable: false);
  }

  List<String> _intactLocales(Revision3VoiceDialogLineChoice line) => line
      .existingSlotLocales
      .where((locale) => line.slotSummaryForLocale(locale) != null)
      .toList(growable: false);

  Revision3VoiceDialogLineChoice? get _selectedLine =>
      _lineId == null ? null : _catalog?.line(_lineId!);

  Revision3VoiceExistingSlotSummary? get _selectedSummary {
    final line = _selectedLine;
    final locale = _locale;
    return line == null || locale == null
        ? null
        : line.slotSummaryForLocale(locale);
  }

  void _chooseLine(Revision3VoiceDialogLineChoice line) {
    final locales = _intactLocales(line);
    final locale = locales.length == 1 ? locales.single : null;
    setState(() {
      _lineId = line.lineId;
      _locale = locale;
      _selectionValue = locale == null
          ? null
          : _selectionValueFor(line.slotSummaryForLocale(locale)!);
      _error = null;
    });
  }

  void _chooseLocale(String? locale) {
    final line = _selectedLine;
    setState(() {
      _locale = locale;
      _selectionValue = line == null || locale == null
          ? null
          : _selectionValueFor(line.slotSummaryForLocale(locale)!);
      _error = null;
    });
  }

  String _selectionValueFor(Revision3VoiceExistingSlotSummary summary) =>
      summary.selectedTakeId ?? _clearSelectionValue;

  bool get _hasChange {
    final summary = _selectedSummary;
    final value = _selectionValue;
    if (summary == null || value == null) return false;
    return value != _selectionValueFor(summary);
  }

  Future<void> _save() async {
    final catalog = _catalog;
    final lineId = _lineId;
    final locale = _locale;
    final value = _selectionValue;
    if (_busy ||
        !_hasChange ||
        catalog == null ||
        lineId == null ||
        locale == null ||
        value == null) {
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final publication = await widget.service.publish(
        checkpoint: catalog,
        lineId: lineId,
        locale: locale,
        selectedTakeId: value == _clearSelectionValue ? null : value,
      );
      if (!mounted) return;
      Navigator.of(context).pop(publication);
    } on Revision3VoiceTakeSelectionStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _error =
            'The project changed while this window was open. Close it and try again from the refreshed project.';
      });
    } on Revision3VoiceTakeSelectionRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _error = 'Reopen the managed project before changing Voice takes.';
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _error = 'Voice selection was not saved: ${_friendlyError(error)}';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final lines = _visibleLines;
    final line = _selectedLine;
    final summary = _selectedSummary;
    return AlertDialog(
      key: const Key('revision3-voice-take-selection-dialog'),
      title: const Text('Manage Voice takes'),
      content: SizedBox(
        width: 760,
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            mainAxisSize: MainAxisSize.min,
            children: [
              const Text(
                'Choose which existing Approved recording this dialog line should use. This changes the offline project only; it does not build, deploy, or make the line playable in game.',
              ),
              const SizedBox(height: 16),
              TextField(
                key: const Key('voice-selection-line-search'),
                controller: _searchController,
                enabled: !_loading && !_busy,
                decoration: const InputDecoration(
                  labelText: 'Find a dialog line',
                  hintText: 'Search by speaker or line name',
                  prefixIcon: Icon(Icons.search),
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 8),
              if (_loading)
                const Center(
                  child: Padding(
                    padding: EdgeInsets.all(24),
                    child: CircularProgressIndicator(),
                  ),
                )
              else if (_catalog != null && lines.isEmpty)
                const Padding(
                  padding: EdgeInsets.symmetric(vertical: 16),
                  child: Text(
                    'No matching dialog line has an intact existing Voice slot.',
                    key: Key('voice-selection-no-lines'),
                  ),
                )
              else if (_catalog != null)
                ConstrainedBox(
                  constraints: const BoxConstraints(maxHeight: 190),
                  child: ListView.builder(
                    key: const Key('voice-selection-lines'),
                    shrinkWrap: true,
                    itemCount: lines.length,
                    itemBuilder: (context, index) {
                      final choice = lines[index];
                      final selected = choice.lineId == _lineId;
                      return Card(
                        color: selected
                            ? theme.colorScheme.secondaryContainer
                            : null,
                        child: ListTile(
                          key: ValueKey('voice-selection-line-$index'),
                          selected: selected,
                          enabled: !_busy,
                          leading: const Icon(Icons.chat_bubble_outline),
                          title: Text(choice.displayLabel),
                          subtitle: Text(
                            '${_intactLocales(choice).length} Voice language${_intactLocales(choice).length == 1 ? '' : 's'}',
                          ),
                          onTap: _busy ? null : () => _chooseLine(choice),
                        ),
                      );
                    },
                  ),
                ),
              if (line != null) ...[
                const SizedBox(height: 16),
                DropdownButtonFormField<String>(
                  key: const Key('voice-selection-locale'),
                  initialValue: _locale,
                  decoration: const InputDecoration(
                    labelText: 'Voice language',
                    border: OutlineInputBorder(),
                  ),
                  items: [
                    for (final locale in _intactLocales(line))
                      DropdownMenuItem(value: locale, child: Text(locale)),
                  ],
                  onChanged: _busy ? null : _chooseLocale,
                ),
              ],
              if (summary != null) ...[
                const SizedBox(height: 18),
                Text('Selected take', style: theme.textTheme.titleMedium),
                const SizedBox(height: 6),
                RadioGroup<String>(
                  groupValue: _selectionValue,
                  onChanged: _busy
                      ? (_) {}
                      : (value) => setState(() {
                          _selectionValue = value;
                          _error = null;
                        }),
                  child: Column(
                    children: [
                      RadioListTile<String>(
                        key: const Key('voice-selection-clear'),
                        value: _clearSelectionValue,
                        enabled: !_busy,
                        contentPadding: EdgeInsets.zero,
                        title: const Text('No take selected'),
                        subtitle: Text(
                          summary.selectedTakeId == null
                              ? 'Current selection'
                              : 'Keep the recordings, but clear the active choice',
                        ),
                      ),
                      for (
                        var index = 0;
                        index < summary.candidates.length;
                        index++
                      )
                        _TakeChoiceTile(
                          index: index,
                          take: summary.candidates[index],
                          isCurrent:
                              summary.selectedTakeId ==
                              summary.candidates[index].id,
                          busy: _busy,
                        ),
                    ],
                  ),
                ),
                if (_selectionValue == _clearSelectionValue &&
                    summary.selectedTakeId != null)
                  Container(
                    key: const Key('voice-selection-clear-warning'),
                    margin: const EdgeInsets.only(top: 8),
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: theme.colorScheme.errorContainer,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Text(
                      'The takes stay in this project, but Voice build is blocked until an Approved take is selected again.',
                      style: TextStyle(
                        color: theme.colorScheme.onErrorContainer,
                      ),
                    ),
                  ),
                if (summary.candidates.isEmpty)
                  const Padding(
                    padding: EdgeInsets.only(top: 8),
                    child: Text(
                      'This Voice slot has no candidate takes yet.',
                      key: Key('voice-selection-no-candidates'),
                    ),
                  ),
              ],
              if (_error != null) ...[
                const SizedBox(height: 12),
                Text(
                  _error!,
                  key: const Key('voice-selection-error'),
                  style: TextStyle(color: theme.colorScheme.error),
                ),
              ],
            ],
          ),
        ),
      ),
      actions: [
        if (_error != null && _catalog == null && !_loading)
          TextButton(
            key: const Key('voice-selection-retry'),
            onPressed: _busy ? null : _load,
            child: const Text('Retry'),
          ),
        TextButton(
          key: const Key('voice-selection-cancel'),
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('voice-selection-save'),
          onPressed: _busy || !_hasChange ? null : _save,
          child: _busy
              ? const SizedBox.square(
                  dimension: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Text('Save selection'),
        ),
      ],
    );
  }
}

class _TakeChoiceTile extends StatelessWidget {
  const _TakeChoiceTile({
    required this.index,
    required this.take,
    required this.isCurrent,
    required this.busy,
  });

  final int index;
  final Revision3VoiceCandidateTake take;
  final bool isCurrent;
  final bool busy;

  @override
  Widget build(BuildContext context) => RadioListTile<String>(
    key: ValueKey('voice-selection-take-$index'),
    value: take.id,
    enabled: !busy && take.isApproved,
    contentPadding: EdgeInsets.zero,
    title: Text(take.displayLabel),
    subtitle: Text(
      '${take.statusLabel}${isCurrent ? ' · Current selection' : ''}${take.isApproved ? '' : ' · Approval required before selection'}',
    ),
  );
}

String _friendlyError(Object error) {
  if (error is ModFfiException) return error.message;
  if (error is FormatException) return error.message;
  return error.toString();
}
