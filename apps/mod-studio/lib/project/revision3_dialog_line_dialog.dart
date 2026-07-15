import 'dart:async';

import 'package:flutter/material.dart';

import '../core/mod_ffi.dart';
import 'revision3_dialog_line_authoring.dart';
import 'revision3_voice_authoring.dart';

final class Revision3DialogLineEntryDialogCopy {
  const Revision3DialogLineEntryDialogCopy({
    required this.title,
    required this.introduction,
    required this.projectOnlyBoundary,
    required this.createMode,
    required this.reuseMode,
    required this.lineNameLabel,
    required this.lineNameHint,
    required this.speakerLabel,
    required this.speakerHint,
    required this.localeLabel,
    required this.textLabel,
    required this.reuseSearchLabel,
    required this.noReusableText,
    required this.createVoiceSlotLabel,
    required this.createVoiceSlotHelp,
    required this.cancel,
    required this.save,
    required this.saving,
    required this.loading,
    required this.loadFailed,
    required this.retry,
    required this.stale,
    required this.requiresReopen,
    required this.invalidInput,
    required this.saveFailed,
    required this.saved,
    required this.done,
    required this.addRecording,
  });

  final String title;
  final String introduction;
  final String projectOnlyBoundary;
  final String createMode;
  final String reuseMode;
  final String lineNameLabel;
  final String lineNameHint;
  final String speakerLabel;
  final String speakerHint;
  final String localeLabel;
  final String textLabel;
  final String reuseSearchLabel;
  final String noReusableText;
  final String createVoiceSlotLabel;
  final String createVoiceSlotHelp;
  final String cancel;
  final String save;
  final String saving;
  final String loading;
  final String loadFailed;
  final String retry;
  final String stale;
  final String requiresReopen;
  final String invalidInput;
  final String saveFailed;
  final String Function(int projectRevision) saved;
  final String done;
  final String addRecording;
}

final class Revision3DialogLineEntryDialogResult {
  const Revision3DialogLineEntryDialogResult({
    required this.publication,
    required this.openVoiceNext,
  });

  final Revision3DialogLineEntryPublication publication;
  final bool openVoiceNext;
}

/// Responsive normal-mode prerequisite editor. It exposes no entity IDs,
/// archive members, game writes, topic registration, build, or runtime claim.
class Revision3DialogLineEntryDialog extends StatefulWidget {
  const Revision3DialogLineEntryDialog({
    required this.service,
    required this.copy,
    this.initialMode = Revision3DialogLineEntryMode.create,
    this.allowOpenVoiceNext = true,
    super.key,
  });

  final Revision3DialogLineEntryAuthoringService service;
  final Revision3DialogLineEntryDialogCopy copy;
  final Revision3DialogLineEntryMode initialMode;
  final bool allowOpenVoiceNext;

  @override
  State<Revision3DialogLineEntryDialog> createState() =>
      _Revision3DialogLineEntryDialogState();
}

class _Revision3DialogLineEntryDialogState
    extends State<Revision3DialogLineEntryDialog> {
  final _formKey = GlobalKey<FormState>();
  final _lineName = TextEditingController();
  final _speaker = TextEditingController();
  final _locale = TextEditingController();
  final _text = TextEditingController();
  final _reuseSearch = TextEditingController();

  Revision3DialogLineEntryCatalog? _catalog;
  late Revision3DialogLineEntryMode _mode;
  String? _localizationId;
  Revision3DialogReusableLocalizationPreview? _localizationPreview;
  bool _createVoiceSlot = true;
  bool _loading = true;
  bool _previewLoading = false;
  bool _publishing = false;
  bool _requiresReopen = false;
  String? _error;
  Revision3DialogLineEntryPublication? _publication;
  int _loadGeneration = 0;
  int _previewGeneration = 0;

  @override
  void initState() {
    super.initState();
    _mode = widget.initialMode;
    _reuseSearch.addListener(_refreshSearch);
    _locale.addListener(_refreshLocale);
    _load();
  }

  @override
  void didUpdateWidget(covariant Revision3DialogLineEntryDialog oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.service, widget.service)) _load(clear: true);
  }

  @override
  void dispose() {
    _loadGeneration++;
    _previewGeneration++;
    _reuseSearch.removeListener(_refreshSearch);
    _locale.removeListener(_refreshLocale);
    _lineName.dispose();
    _speaker.dispose();
    _locale.dispose();
    _text.dispose();
    _reuseSearch.dispose();
    super.dispose();
  }

  void _refreshSearch() {
    if (!mounted) return;
    final selectedId = _localizationId;
    final selectionRemainsVisible =
        selectedId == null ||
        _filteredChoices.any((choice) => choice.id == selectedId);
    final selected = selectionRemainsVisible ? _selectedLocalization : null;
    _previewGeneration++;
    setState(() {
      _localizationPreview = null;
      _previewLoading = false;
      _error = null;
      if (!selectionRemainsVisible) {
        _localizationId = null;
      }
    });
    if (selected != null) unawaited(_loadLocalizationPreview(selected));
  }

  void _refreshLocale() {
    if (mounted) setState(() {});
  }

  Future<void> _load({bool clear = false}) async {
    final generation = ++_loadGeneration;
    _previewGeneration++;
    setState(() {
      _loading = true;
      _previewLoading = false;
      _localizationPreview = null;
      _error = null;
      if (clear) {
        _catalog = null;
        _localizationId = null;
      }
    });
    try {
      final catalog = await widget.service.loadCatalog();
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _catalog = catalog;
        if (_locale.text.isEmpty) {
          _locale.text = catalog.suggestedLocales.first;
        }
        if (catalog.localization(_localizationId ?? '') == null) {
          _localizationId = null;
        }
        _loading = false;
      });
    } on Revision3DialogLineEntryRequiresReopenException {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _requiresReopen = true;
        _error = widget.copy.requiresReopen;
      });
    } catch (_) {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loading = false;
        _catalog = null;
        _error = widget.copy.loadFailed;
      });
    }
  }

  List<Revision3DialogReusableLocalizationChoice> get _filteredChoices {
    final catalog = _catalog;
    if (catalog == null) return const [];
    final query = _reuseSearch.text;
    return catalog.reusableLocalizations
        .where((choice) => choice.matches(query))
        .take(100)
        .toList(growable: false);
  }

  Revision3DialogReusableLocalizationChoice? get _selectedLocalization =>
      _catalog?.localization(_localizationId ?? '');

  void _changeMode(Revision3DialogLineEntryMode mode) {
    if (_publishing || mode == _mode) return;
    _previewGeneration++;
    final catalog = _catalog;
    if (mode == Revision3DialogLineEntryMode.create && catalog != null) {
      final locale = _locale.text.trim();
      if (!catalog.suggestedLocales.contains(locale)) {
        _locale.text = catalog.suggestedLocales.first;
      }
    }
    setState(() {
      _mode = mode;
      _error = null;
      _localizationId = null;
      _localizationPreview = null;
      _previewLoading = false;
    });
  }

  void _selectLocalization(Revision3DialogReusableLocalizationChoice choice) {
    _previewGeneration++;
    setState(() {
      _localizationId = choice.id;
      _localizationPreview = null;
      _previewLoading = false;
      _error = null;
    });
    unawaited(_loadLocalizationPreview(choice));
  }

  Future<void> _loadLocalizationPreview(
    Revision3DialogReusableLocalizationChoice choice,
  ) async {
    final catalog = _catalog;
    if (catalog == null || _localizationId != choice.id) return;
    final generation = ++_previewGeneration;
    setState(() {
      _previewLoading = true;
      _localizationPreview = null;
      _error = null;
    });
    try {
      final preview = await widget.service.loadReusableLocalizationPreview(
        checkpoint: catalog,
        localizationId: choice.id,
      );
      if (!mounted ||
          generation != _previewGeneration ||
          _mode != Revision3DialogLineEntryMode.reuseExact ||
          _localizationId != choice.id) {
        return;
      }
      final authorableLocales = preview.authorableLocales;
      setState(() {
        _previewLoading = false;
        _localizationPreview = preview;
        _error = authorableLocales.isEmpty ? widget.copy.noReusableText : null;
      });
      if (authorableLocales.isEmpty) {
        _locale.clear();
      } else if (!authorableLocales.contains(_locale.text.trim())) {
        _locale.text = authorableLocales.first;
      }
    } on Revision3DialogLineEntryStaleCheckpointException {
      if (!mounted || generation != _previewGeneration) return;
      setState(() {
        _previewLoading = false;
        _localizationPreview = null;
        _error = widget.copy.stale;
      });
    } on Revision3DialogLineEntryRequiresReopenException {
      if (!mounted || generation != _previewGeneration) return;
      setState(() {
        _previewLoading = false;
        _localizationPreview = null;
        _requiresReopen = true;
        _error = widget.copy.requiresReopen;
      });
    } catch (_) {
      if (!mounted || generation != _previewGeneration) return;
      setState(() {
        _previewLoading = false;
        _localizationPreview = null;
        _error = widget.copy.loadFailed;
      });
    }
  }

  Future<void> _submit() async {
    final catalog = _catalog;
    if (_publishing ||
        _requiresReopen ||
        catalog == null ||
        !(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    final Revision3DialogLineEntryInput input;
    try {
      input = switch (_mode) {
        Revision3DialogLineEntryMode.create =>
          Revision3DialogLineEntryInput.create(
            lineDisplayName: _lineName.text,
            speakerHint: _speaker.text,
            locale: _locale.text,
            text: _text.text,
            createVoiceSlot: _createVoiceSlot,
          ),
        Revision3DialogLineEntryMode.reuseExact =>
          Revision3DialogLineEntryInput.reuseExact(
            lineDisplayName: _lineName.text,
            speakerHint: _speaker.text,
            locale: _locale.text,
            localizationId: _localizationId ?? '',
            createVoiceSlot: _createVoiceSlot,
          ),
      };
    } on FormatException {
      setState(() => _error = widget.copy.invalidInput);
      return;
    }
    setState(() {
      _publishing = true;
      _error = null;
    });
    try {
      final publication = await widget.service.publish(
        checkpoint: catalog,
        input: input,
      );
      if (!mounted) return;
      setState(() {
        _publication = publication;
        _publishing = false;
      });
    } on Revision3DialogLineEntryStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _publishing = false;
        _error = widget.copy.stale;
      });
    } on Revision3DialogLineEntryNoReusableTextException {
      if (!mounted) return;
      setState(() {
        _publishing = false;
        _error = widget.copy.noReusableText;
      });
    } on Revision3DialogLineEntryRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _publishing = false;
        _requiresReopen = true;
        _error = widget.copy.requiresReopen;
      });
    } on FormatException {
      if (!mounted) return;
      setState(() {
        _publishing = false;
        _error = widget.copy.invalidInput;
      });
    } on ModFfiException catch (error) {
      if (!mounted) return;
      setState(() {
        _publishing = false;
        _error = _dialogEntryFfiMessage(error.code, widget.copy);
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _publishing = false;
        _error = widget.copy.saveFailed;
      });
    }
  }

  void _finish(bool openVoiceNext) {
    final publication = _publication;
    if (publication == null) return;
    Navigator.of(context).pop(
      Revision3DialogLineEntryDialogResult(
        publication: publication,
        openVoiceNext: openVoiceNext,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final narrow = MediaQuery.sizeOf(context).width < 700;
    final frame = PopScope(
      canPop: !_publishing,
      child: _DialogLineFrame(
        title: widget.copy.title,
        content: _publication == null
            ? _buildEditor(context)
            : _buildSuccess(context),
        actions: _buildActions(),
      ),
    );
    if (narrow) {
      return Dialog.fullscreen(
        key: const Key('revision3-dialog-line-fullscreen'),
        child: frame,
      );
    }
    return Dialog(
      key: const Key('revision3-dialog-line-modal'),
      clipBehavior: Clip.antiAlias,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 720, maxHeight: 820),
        child: frame,
      ),
    );
  }

  Widget _buildEditor(BuildContext context) {
    final copy = widget.copy;
    final catalog = _catalog;
    return Form(
      key: _formKey,
      child: ListView(
        key: const Key('revision3-dialog-line-editor'),
        padding: const EdgeInsets.fromLTRB(24, 16, 24, 24),
        children: [
          Text(copy.introduction),
          const SizedBox(height: 12),
          _BoundaryNotice(text: copy.projectOnlyBoundary),
          const SizedBox(height: 18),
          SegmentedButton<Revision3DialogLineEntryMode>(
            key: const Key('revision3-dialog-line-mode'),
            segments: [
              ButtonSegment(
                value: Revision3DialogLineEntryMode.create,
                icon: const Icon(Icons.edit_note_outlined),
                label: Text(copy.createMode),
              ),
              ButtonSegment(
                value: Revision3DialogLineEntryMode.reuseExact,
                icon: const Icon(Icons.link_outlined),
                label: Text(copy.reuseMode),
              ),
            ],
            selected: {_mode},
            onSelectionChanged: _publishing
                ? null
                : (selection) => _changeMode(selection.single),
          ),
          const SizedBox(height: 18),
          if (_error != null) ...[
            MaterialBanner(
              key: const Key('revision3-dialog-line-error'),
              content: Text(_error!),
              actions: const [SizedBox.shrink()],
            ),
            const SizedBox(height: 12),
          ],
          if (_loading)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 40),
              child: Center(
                child: Column(
                  children: [
                    const CircularProgressIndicator(),
                    const SizedBox(height: 12),
                    Text(copy.loading),
                  ],
                ),
              ),
            )
          else if (catalog == null)
            Align(
              alignment: Alignment.centerLeft,
              child: OutlinedButton.icon(
                key: const Key('revision3-dialog-line-retry'),
                onPressed: _requiresReopen ? null : _load,
                icon: const Icon(Icons.refresh),
                label: Text(copy.retry),
              ),
            )
          else ...[
            TextFormField(
              key: const Key('revision3-dialog-line-name'),
              controller: _lineName,
              enabled: !_publishing,
              maxLength: 192,
              decoration: InputDecoration(
                labelText: copy.lineNameLabel,
                hintText: copy.lineNameHint,
                border: const OutlineInputBorder(),
              ),
              validator: (value) => value == null || value.trim().isEmpty
                  ? copy.invalidInput
                  : null,
            ),
            const SizedBox(height: 12),
            TextFormField(
              key: const Key('revision3-dialog-line-speaker'),
              controller: _speaker,
              enabled: !_publishing,
              maxLength: 256,
              decoration: InputDecoration(
                labelText: copy.speakerLabel,
                hintText: copy.speakerHint,
                border: const OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 12),
            if (_mode == Revision3DialogLineEntryMode.reuseExact)
              _buildReusePicker(copy, catalog),
            TextFormField(
              key: const Key('revision3-dialog-line-locale'),
              controller: _locale,
              enabled: !_publishing,
              maxLength: 35,
              decoration: InputDecoration(
                labelText: copy.localeLabel,
                border: const OutlineInputBorder(),
              ),
              validator: (value) {
                final locale = value?.trim() ?? '';
                if (!revision3VoiceLocaleIsCanonical(locale)) {
                  return copy.invalidInput;
                }
                final selected = _selectedLocalization;
                if (_mode == Revision3DialogLineEntryMode.reuseExact &&
                    (selected == null ||
                        _localizationPreview?.locale(locale)?.hasNonemptyText !=
                            true)) {
                  return copy.invalidInput;
                }
                return null;
              },
            ),
            const SizedBox(height: 8),
            Wrap(
              spacing: 8,
              runSpacing: 4,
              children: [
                for (final locale in _availableLocales(catalog))
                  ChoiceChip(
                    key: Key('revision3-dialog-line-locale-$locale'),
                    label: Text(locale),
                    selected: _locale.text.trim() == locale,
                    onSelected: _publishing
                        ? null
                        : (selected) {
                            if (selected) {
                              setState(() => _locale.text = locale);
                            }
                          },
                  ),
              ],
            ),
            const SizedBox(height: 12),
            if (_mode == Revision3DialogLineEntryMode.create) ...[
              TextFormField(
                key: const Key('revision3-dialog-line-text'),
                controller: _text,
                enabled: !_publishing,
                minLines: 4,
                maxLines: 10,
                maxLength: 64 * 1024,
                decoration: InputDecoration(
                  labelText: copy.textLabel,
                  alignLabelWithHint: true,
                  border: const OutlineInputBorder(),
                ),
                validator: (value) => value == null || value.trim().isEmpty
                    ? copy.invalidInput
                    : null,
              ),
              const SizedBox(height: 8),
            ],
            SwitchListTile.adaptive(
              key: const Key('revision3-dialog-line-create-slot'),
              contentPadding: EdgeInsets.zero,
              value: _createVoiceSlot,
              onChanged: _publishing
                  ? null
                  : (value) => setState(() => _createVoiceSlot = value),
              title: Text(copy.createVoiceSlotLabel),
              subtitle: Text(copy.createVoiceSlotHelp),
            ),
          ],
        ],
      ),
    );
  }

  Widget _buildReusePicker(
    Revision3DialogLineEntryDialogCopy copy,
    Revision3DialogLineEntryCatalog catalog,
  ) {
    final choices = _filteredChoices;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        TextField(
          key: const Key('revision3-dialog-line-reuse-search'),
          controller: _reuseSearch,
          enabled: !_publishing,
          decoration: InputDecoration(
            labelText: copy.reuseSearchLabel,
            prefixIcon: const Icon(Icons.search),
            border: const OutlineInputBorder(),
          ),
        ),
        const SizedBox(height: 8),
        if (catalog.reusableLocalizations.isEmpty)
          Padding(
            key: const Key('revision3-dialog-line-no-reuse'),
            padding: const EdgeInsets.symmetric(vertical: 12),
            child: Text(copy.noReusableText),
          )
        else
          ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 220),
            child: RadioGroup<String>(
              groupValue: _localizationId,
              onChanged: (value) {
                if (_publishing || value == null) return;
                final choice = catalog.localization(value);
                if (choice != null) _selectLocalization(choice);
              },
              child: ListView.builder(
                key: const Key('revision3-dialog-line-reuse-results'),
                shrinkWrap: true,
                itemCount: choices.length,
                itemBuilder: (context, index) {
                  final choice = choices[index];
                  return RadioListTile<String>(
                    value: choice.id,
                    enabled: !_publishing,
                    title: Text(_friendlyChoiceLabel(choice, catalog)),
                    subtitle:
                        _localizationId == choice.id &&
                            _localizationPreview != null
                        ? Text(
                            _localizationPreview!.authorableLocales.join(', '),
                          )
                        : null,
                  );
                },
              ),
            ),
          ),
        if (_previewLoading) ...[
          const SizedBox(height: 12),
          Row(
            key: const Key('revision3-dialog-line-preview-loading'),
            children: [
              const SizedBox.square(
                dimension: 18,
                child: CircularProgressIndicator(strokeWidth: 2),
              ),
              const SizedBox(width: 10),
              Expanded(child: Text(copy.loading)),
            ],
          ),
        ] else if (_selectedLocalePreview case final preview?) ...[
          const SizedBox(height: 12),
          DecoratedBox(
            key: const Key('revision3-dialog-line-reuse-preview'),
            decoration: BoxDecoration(
              color: Theme.of(context).colorScheme.surfaceContainerHighest,
              borderRadius: BorderRadius.circular(10),
            ),
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    copy.textLabel,
                    style: Theme.of(context).textTheme.labelLarge,
                  ),
                  const SizedBox(height: 6),
                  SelectableText(
                    preview.truncated ? '${preview.text}\u2026' : preview.text,
                    key: const Key('revision3-dialog-line-preview-text'),
                  ),
                ],
              ),
            ),
          ),
        ],
        const SizedBox(height: 12),
      ],
    );
  }

  Revision3DialogReusableLocalePreview? get _selectedLocalePreview {
    final preview = _localizationPreview?.locale(_locale.text.trim());
    return preview?.hasNonemptyText == true ? preview : null;
  }

  String _friendlyChoiceLabel(
    Revision3DialogReusableLocalizationChoice choice,
    Revision3DialogLineEntryCatalog catalog,
  ) {
    final sameLabels = catalog.reusableLocalizations
        .where(
          (candidate) =>
              candidate.displayLabel.toLowerCase() ==
              choice.displayLabel.toLowerCase(),
        )
        .toList(growable: false);
    if (sameLabels.length < 2) return choice.displayLabel;
    return '${choice.displayLabel} (${sameLabels.indexOf(choice) + 1})';
  }

  List<String> _availableLocales(Revision3DialogLineEntryCatalog catalog) =>
      _mode == Revision3DialogLineEntryMode.reuseExact
      ? (_localizationPreview?.authorableLocales ?? const [])
      : catalog.suggestedLocales;

  Widget _buildSuccess(BuildContext context) {
    final publication = _publication!;
    return ListView(
      key: const Key('revision3-dialog-line-success'),
      padding: const EdgeInsets.all(24),
      children: [
        const Icon(Icons.check_circle_outline, size: 52, color: Colors.green),
        const SizedBox(height: 16),
        Text(
          widget.copy.saved(publication.projectRevision),
          textAlign: TextAlign.center,
          style: Theme.of(context).textTheme.titleMedium,
        ),
        const SizedBox(height: 16),
        _BoundaryNotice(text: widget.copy.projectOnlyBoundary),
      ],
    );
  }

  List<Widget> _buildActions() {
    final publication = _publication;
    if (publication != null) {
      return [
        TextButton(
          key: const Key('revision3-dialog-line-done'),
          onPressed: () => _finish(false),
          child: Text(widget.copy.done),
        ),
        if (widget.allowOpenVoiceNext)
          FilledButton.icon(
            key: const Key('revision3-dialog-line-open-voice'),
            onPressed: () => _finish(true),
            icon: const Icon(Icons.mic_none_outlined),
            label: Text(widget.copy.addRecording),
          ),
      ];
    }
    return [
      TextButton(
        key: const Key('revision3-dialog-line-cancel'),
        onPressed: _publishing ? null : () => Navigator.of(context).pop(),
        child: Text(widget.copy.cancel),
      ),
      FilledButton.icon(
        key: const Key('revision3-dialog-line-submit'),
        onPressed:
            _publishing ||
                _loading ||
                _catalog == null ||
                _requiresReopen ||
                (_mode == Revision3DialogLineEntryMode.reuseExact &&
                    (_localizationId == null ||
                        _previewLoading ||
                        _localizationPreview
                                ?.locale(_locale.text.trim())
                                ?.hasNonemptyText !=
                            true))
            ? null
            : _submit,
        icon: _publishing
            ? const SizedBox.square(
                dimension: 18,
                child: CircularProgressIndicator(strokeWidth: 2),
              )
            : const Icon(Icons.save_outlined),
        label: Text(_publishing ? widget.copy.saving : widget.copy.save),
      ),
    ];
  }
}

class _DialogLineFrame extends StatelessWidget {
  const _DialogLineFrame({
    required this.title,
    required this.content,
    required this.actions,
  });

  final String title;
  final Widget content;
  final List<Widget> actions;

  @override
  Widget build(BuildContext context) => Column(
    children: [
      AppBar(
        automaticallyImplyLeading: false,
        title: Text(title),
        actions: [
          IconButton(
            key: const Key('revision3-dialog-line-close'),
            tooltip: MaterialLocalizations.of(context).closeButtonTooltip,
            onPressed:
                actions.any((action) {
                  if (action case TextButton(:final onPressed)) {
                    return onPressed == null;
                  }
                  return false;
                })
                ? null
                : () => Navigator.of(context).pop(),
            icon: const Icon(Icons.close),
          ),
          const SizedBox(width: 8),
        ],
      ),
      Expanded(child: content),
      const Divider(height: 1),
      SafeArea(
        top: false,
        minimum: const EdgeInsets.fromLTRB(16, 10, 16, 10),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.end,
          children: [
            for (var index = 0; index < actions.length; index++) ...[
              if (index > 0) const SizedBox(width: 8),
              actions[index],
            ],
          ],
        ),
      ),
    ],
  );
}

class _BoundaryNotice extends StatelessWidget {
  const _BoundaryNotice({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) => DecoratedBox(
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.surfaceContainerHighest,
      borderRadius: BorderRadius.circular(10),
    ),
    child: Padding(
      padding: const EdgeInsets.all(12),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Icon(Icons.shield_outlined, size: 20),
          const SizedBox(width: 10),
          Expanded(child: Text(text)),
        ],
      ),
    ),
  );
}

String _dialogEntryFfiMessage(
  String code,
  Revision3DialogLineEntryDialogCopy copy,
) => switch (code) {
  'AUTHORING_REVISION3_DIALOG_ENTITY_CONFLICT' ||
  'AUTHORING_REVISION3_DIALOG_IDENTITY_CONFLICT' ||
  'AUTHORING_REVISION3_DIALOG_LOCALIZATION_CONFLICT' ||
  'AUTHORING_REVISION3_DIALOG_LOCALE_CONFLICT' ||
  'AUTHORING_REVISION3_DIALOG_REQUEST_REJECTED' => copy.invalidInput,
  'AUTHORING_REVISION3_DIALOG_HEAD_CONFLICT' => copy.stale,
  _ => copy.saveFailed,
};
