import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:path/path.dart' as p;

import '../core/mod_ffi.dart';
import 'revision3_dataasset_authoring.dart';

typedef Revision3ReviewedDataAssetExactBuild =
    Future<AuthoringRevision3ReviewedDataAssetBuildResult> Function({
      required String packName,
      required String output,
    });

typedef Revision3DataAssetBuildParentDirectoryPicker =
    Future<String?> Function();

/// Small, write-new-only build surface for one already-reviewed DataAsset edit.
class Revision3DataAssetBuildDialog extends StatefulWidget {
  const Revision3DataAssetBuildDialog({
    required this.targetPath,
    required this.build,
    required this.pickExistingParentDirectory,
    super.key,
  });

  final String targetPath;
  final Revision3ReviewedDataAssetExactBuild build;
  final Revision3DataAssetBuildParentDirectoryPicker
  pickExistingParentDirectory;

  @override
  State<Revision3DataAssetBuildDialog> createState() =>
      _Revision3DataAssetBuildDialogState();
}

class _Revision3DataAssetBuildDialogState
    extends State<Revision3DataAssetBuildDialog> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _packName;

  String? _parentDirectory;
  String? _error;
  bool _choosingParent = false;
  bool _building = false;
  bool _terminalError = false;
  AuthoringRevision3ReviewedDataAssetBuildResult? _result;

  bool get _busy => _choosingParent || _building;

  String? get _outputPreview {
    final parent = _parentDirectory;
    final name = _packName.text;
    if (parent == null || validateRevision3DataAssetPackName(name) != null) {
      return null;
    }
    return p.join(parent, name);
  }

  @override
  void initState() {
    super.initState();
    _packName = TextEditingController(
      text: suggestedRevision3DataAssetPackName(widget.targetPath),
    );
  }

  @override
  void dispose() {
    _packName.dispose();
    super.dispose();
  }

  Future<void> _chooseParent() async {
    if (_busy || _terminalError || _result != null) return;
    setState(() {
      _choosingParent = true;
      _error = null;
    });
    try {
      final selected = await widget.pickExistingParentDirectory();
      if (!mounted || selected == null) return;
      final validation = validateRevision3DataAssetBuildParent(selected);
      if (validation != null) {
        setState(() => _error = validation);
        return;
      }
      setState(() => _parentDirectory = p.normalize(selected));
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _error =
            'The destination folder could not be inspected safely. Nothing was created.';
      });
    } finally {
      if (mounted) setState(() => _choosingParent = false);
    }
  }

  Future<void> _buildFiles() async {
    if (_busy || _terminalError || _result != null) return;
    if (!(_formKey.currentState?.validate() ?? false)) return;
    final parent = _parentDirectory;
    if (parent == null) {
      setState(() => _error = 'Choose an existing destination folder.');
      return;
    }

    setState(() {
      _building = true;
      _error = null;
    });
    var buildBoundaryEntered = false;
    String? attemptedOutput;
    try {
      final parentError = validateRevision3DataAssetBuildParent(parent);
      if (parentError != null) {
        if (mounted) setState(() => _error = parentError);
        return;
      }
      final packName = _packName.text;
      final output = p.join(parent, packName);
      attemptedOutput = output;
      final outputType = FileSystemEntity.typeSync(output, followLinks: false);
      if (outputType != FileSystemEntityType.notFound) {
        if (!mounted) return;
        setState(() {
          _error = outputType == FileSystemEntityType.link
              ? 'The new folder path is a link. Choose a different pack name.'
              : 'That new folder already exists. Choose a different pack name.';
        });
        return;
      }

      buildBoundaryEntered = true;
      final result = await widget.build(packName: packName, output: output);
      if (!mounted) return;
      if (result.targetPath != widget.targetPath ||
          result.packName != packName ||
          result.output != output) {
        setState(() {
          _terminalError = true;
          _error =
              'The completed build does not match this saved edit and destination. Close this window and inspect the destination before trying again.';
        });
        return;
      }
      setState(() => _result = result);
    } on Revision3DataAssetRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _terminalError = true;
        _error = buildBoundaryEntered
            ? _possibleBuildMessage(attemptedOutput)
            : 'This project can no longer be verified as current. Close this window and reopen the project.';
      });
    } on Revision3DataAssetStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _terminalError = true;
        _error =
            'The project changed while this window was open. Close this window and open the build again.';
      });
    } on ModFfiException catch (error) {
      if (!mounted) return;
      final publicationMayExist =
          buildBoundaryEntered &&
          !_definitelyPrepublicationBuildError(error.code);
      setState(() {
        _terminalError = publicationMayExist || _terminalBuildError(error.code);
        _error = publicationMayExist
            ? _possibleBuildMessage(attemptedOutput)
            : revision3DataAssetBuildErrorMessage(error);
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _terminalError = buildBoundaryEntered;
        _error = buildBoundaryEntered
            ? _possibleBuildMessage(attemptedOutput)
            : 'The files could not be built exactly. Nothing was added to the game or project.';
      });
    } finally {
      if (mounted) setState(() => _building = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final result = _result;
    return PopScope(
      canPop: !_busy,
      child: AlertDialog(
        key: const Key('revision3-dataasset-build-dialog'),
        title: const Text('Build DataAsset files'),
        content: SizedBox(
          width: 620,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _BuildNotice(
                  assetLabel: revision3DataAssetLabel(widget.targetPath),
                  targetPath: widget.targetPath,
                ),
                const SizedBox(height: 16),
                Form(
                  key: _formKey,
                  child: TextFormField(
                    key: const Key('revision3-dataasset-build-pack-name'),
                    controller: _packName,
                    enabled: !_busy && !_terminalError && result == null,
                    autovalidateMode: AutovalidateMode.onUserInteraction,
                    decoration: const InputDecoration(
                      labelText: 'Pack name',
                      helperText:
                          'Also used for the brand-new output folder and mod files.',
                      border: OutlineInputBorder(),
                    ),
                    validator: validateRevision3DataAssetPackName,
                    onChanged: (_) => setState(() => _error = null),
                  ),
                ),
                const SizedBox(height: 12),
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    OutlinedButton.icon(
                      key: const Key('revision3-dataasset-build-choose-parent'),
                      onPressed: _busy || _terminalError || result != null
                          ? null
                          : _chooseParent,
                      icon: _choosingParent
                          ? const SizedBox.square(
                              dimension: 16,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Icon(Icons.folder_open_outlined),
                      label: const Text('Choose destination folder'),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        _parentDirectory ?? 'No destination folder selected',
                        key: const Key('revision3-dataasset-build-parent'),
                      ),
                    ),
                  ],
                ),
                if (_outputPreview case final output?) ...[
                  const SizedBox(height: 12),
                  _BuildFact(
                    label: 'New folder',
                    value: output,
                    valueKey: const Key(
                      'revision3-dataasset-build-output-preview',
                    ),
                  ),
                ],
                if (_error case final error?) ...[
                  const SizedBox(height: 14),
                  _BuildError(message: error),
                ],
                if (result != null) ...[
                  const Divider(height: 32),
                  _BuildResult(result: result),
                ],
              ],
            ),
          ),
        ),
        actions: [
          TextButton(
            key: const Key('revision3-dataasset-build-close'),
            onPressed: _busy ? null : () => Navigator.of(context).pop(result),
            child: Text(result == null && !_terminalError ? 'Cancel' : 'Close'),
          ),
          if (result == null)
            FilledButton.icon(
              key: const Key('revision3-dataasset-build-submit'),
              onPressed:
                  !_busy &&
                      !_terminalError &&
                      _parentDirectory != null &&
                      validateRevision3DataAssetPackName(_packName.text) == null
                  ? _buildFiles
                  : null,
              icon: _building
                  ? const SizedBox.square(
                      dimension: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.inventory_2_outlined),
              label: Text(_building ? 'Building...' : 'Build files'),
            ),
        ],
      ),
    );
  }
}

String? validateRevision3DataAssetBuildParent(String value) {
  if (value.isEmpty || !p.isAbsolute(value)) {
    return 'Choose an absolute existing destination folder.';
  }
  final type = FileSystemEntity.typeSync(value, followLinks: false);
  return switch (type) {
    FileSystemEntityType.directory => null,
    FileSystemEntityType.link =>
      'The selected destination is a link. Choose a real existing folder.',
    _ => 'Choose an existing destination folder.',
  };
}

String? validateRevision3DataAssetPackName(String? raw) {
  final value = raw ?? '';
  if (value.isEmpty) return 'Enter a pack name.';
  if (utf8.encode(value).length > 96) {
    return 'The pack name must be at most 96 ASCII characters.';
  }
  if (!value.runes.every((rune) => rune <= 0x7f)) {
    return 'Use only ASCII letters, digits, underscores, and hyphens.';
  }
  if (!RegExp(r'^[A-Za-z0-9][A-Za-z0-9_-]*$').hasMatch(value)) {
    return 'Start with a letter or digit, then use only letters, digits, underscores, or hyphens.';
  }
  final folded = value.toUpperCase();
  if (const {'CON', 'PRN', 'AUX', 'NUL'}.contains(folded) ||
      RegExp(r'^(COM|LPT)[1-9]$').hasMatch(folded)) {
    return 'That pack name is reserved by Windows.';
  }
  return null;
}

String suggestedRevision3DataAssetPackName(String targetPath) {
  final label = revision3DataAssetLabel(targetPath);
  var stem = label
      .split('')
      .map(
        (character) =>
            RegExp(r'[A-Za-z0-9_-]').hasMatch(character) ? character : '_',
      )
      .join();
  if (stem.isEmpty || !RegExp(r'^[A-Za-z0-9]').hasMatch(stem)) {
    stem = 'DataAsset_$stem';
  }
  const suffix = '_Mod';
  final maximumStem = 96 - suffix.length;
  if (stem.length > maximumStem) stem = stem.substring(0, maximumStem);
  final candidate = '$stem$suffix';
  return validateRevision3DataAssetPackName(candidate) == null
      ? candidate
      : 'DataAsset_Mod';
}

String revision3DataAssetLabel(String targetPath) {
  final segments = targetPath.split('/');
  return segments.isEmpty || segments.last.isEmpty ? targetPath : segments.last;
}

bool _terminalBuildError(String code) => const {
  'AUTHORING_REVISION3_DATAASSET_BUILD_HEAD_CONFLICT',
  'AUTHORING_REVISION3_DATAASSET_BUILD_INPUT_INVALID',
  'AUTHORING_REVISION3_DATAASSET_BUILD_INPUT_LIMIT',
  'AUTHORING_REVISION3_DATAASSET_BUILD_PROJECT_CONFLICT',
  'AUTHORING_REVISION3_DATAASSET_BUILD_PROJECT_INVALID',
  'AUTHORING_REVISION3_DATAASSET_BUILD_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_DATAASSET_BUILD_SOURCE_CONFLICT',
  'AUTHORING_REVISION3_DATAASSET_BUILD_SOURCE_INVALID',
  'AUTHORING_REVISION3_DATAASSET_BUILD_TARGET_INVALID',
  'AUTHORING_REVISION3_DATAASSET_BUILD_TARGET_MISSING',
  'AUTHORING_REVISION3_DATAASSET_BUILD_TARGET_NOT_REVIEWED',
}.contains(code);

bool _definitelyPrepublicationBuildError(String code) => const {
  'AUTHORING_REVISION3_DATAASSET_BUILD_HEAD_CONFLICT',
  'AUTHORING_REVISION3_DATAASSET_BUILD_INPUT_INVALID',
  'AUTHORING_REVISION3_DATAASSET_BUILD_INPUT_LIMIT',
  'AUTHORING_REVISION3_DATAASSET_BUILD_OUTPUT_EXISTS',
  'AUTHORING_REVISION3_DATAASSET_BUILD_OUTPUT_INVALID',
  'AUTHORING_REVISION3_DATAASSET_BUILD_PACK_FAILED',
  'AUTHORING_REVISION3_DATAASSET_BUILD_PACK_NAME_INVALID',
  'AUTHORING_REVISION3_DATAASSET_BUILD_PROJECT_CONFLICT',
  'AUTHORING_REVISION3_DATAASSET_BUILD_PROJECT_INVALID',
  'AUTHORING_REVISION3_DATAASSET_BUILD_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_DATAASSET_BUILD_SOURCE_CONFLICT',
  'AUTHORING_REVISION3_DATAASSET_BUILD_SOURCE_INVALID',
  'AUTHORING_REVISION3_DATAASSET_BUILD_TARGET_INVALID',
  'AUTHORING_REVISION3_DATAASSET_BUILD_TARGET_MISSING',
  'AUTHORING_REVISION3_DATAASSET_BUILD_TARGET_NOT_REVIEWED',
}.contains(code);

String _possibleBuildMessage(String? output) => output == null
    ? 'The build may already exist in the chosen destination. Check that folder before trying again.'
    : 'The build may already exist at $output. Check that folder before trying again.';

String revision3DataAssetBuildErrorMessage(
  ModFfiException error,
) => switch (error.code) {
  'AUTHORING_REVISION3_DATAASSET_BUILD_OUTPUT_EXISTS' =>
    'That new folder already exists. Choose a different pack name.',
  'AUTHORING_REVISION3_DATAASSET_BUILD_HEAD_CONFLICT' ||
  'AUTHORING_REVISION3_DATAASSET_BUILD_PROJECT_CONFLICT' ||
  'AUTHORING_REVISION3_DATAASSET_BUILD_SOURCE_CONFLICT' =>
    'The project changed while the files were being built. Close this window and reopen the project.',
  'AUTHORING_REVISION3_DATAASSET_BUILD_TARGET_MISSING' =>
    'This saved edit is no longer available. Refresh the DataAsset list.',
  'AUTHORING_REVISION3_DATAASSET_BUILD_TARGET_NOT_REVIEWED' ||
  'AUTHORING_REVISION3_DATAASSET_BUILD_PROJECT_INVALID' =>
    'This saved edit is not ready to build. Review or recreate the edit first.',
  'AUTHORING_REVISION3_DATAASSET_BUILD_SOURCE_INVALID' =>
    'The installed game no longer matches this saved edit. Finish any game update, then reopen the project.',
  'AUTHORING_REVISION3_DATAASSET_BUILD_OUTPUT_INVALID' =>
    'That destination is not safe for new mod files. Choose another existing folder outside the project and game.',
  'AUTHORING_REVISION3_DATAASSET_BUILD_PACK_FAILED' ||
  'AUTHORING_REVISION3_DATAASSET_BUILD_PUBLICATION_FAILED' =>
    'The files could not be created and verified in that new folder. Choose another pack name before trying again.',
  _ =>
    'The files could not be built exactly. Nothing was added to the game or project.',
};

class _BuildNotice extends StatelessWidget {
  const _BuildNotice({required this.assetLabel, required this.targetPath});

  final String assetLabel;
  final String targetPath;

  @override
  Widget build(BuildContext context) => Container(
    key: const Key('revision3-dataasset-build-notice'),
    padding: const EdgeInsets.all(12),
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.secondaryContainer,
      borderRadius: BorderRadius.circular(8),
    ),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            const Icon(Icons.data_object_outlined),
            const SizedBox(width: 10),
            Expanded(
              child: Text(
                assetLabel,
                key: const Key('revision3-dataasset-build-asset-label'),
                style: Theme.of(context).textTheme.titleMedium,
              ),
            ),
          ],
        ),
        const SizedBox(height: 6),
        SelectableText(
          targetPath,
          key: const Key('revision3-dataasset-build-target-path'),
        ),
        const SizedBox(height: 10),
        const Text(
          'Creates a new set of mod files for this saved edit. Your project and game installation are not changed.',
        ),
      ],
    ),
  );
}

class _BuildResult extends StatelessWidget {
  const _BuildResult({required this.result});

  final AuthoringRevision3ReviewedDataAssetBuildResult result;

  @override
  Widget build(BuildContext context) {
    if (result.publicationIsUncertain) {
      return Column(
        key: const Key('revision3-dataasset-build-uncertain'),
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Check the destination',
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 6),
          Text(_possibleBuildMessage(result.output)),
        ],
      );
    }
    return Column(
      key: const Key('revision3-dataasset-build-complete'),
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text('Build complete', style: Theme.of(context).textTheme.titleMedium),
        const SizedBox(height: 6),
        Text('Files created in ${result.output}'),
        if (result.hasCleanupWarning) ...[
          const SizedBox(height: 8),
          const Text(
            'The mod files are complete, but some temporary files could not be cleaned up automatically.',
          ),
        ],
      ],
    );
  }
}

class _BuildFact extends StatelessWidget {
  const _BuildFact({required this.label, required this.value, this.valueKey});

  final String label;
  final String value;
  final Key? valueKey;

  @override
  Widget build(BuildContext context) => Row(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      SizedBox(
        width: 110,
        child: Text(label, style: Theme.of(context).textTheme.labelLarge),
      ),
      const SizedBox(width: 8),
      Expanded(child: SelectableText(value, key: valueKey)),
    ],
  );
}

class _BuildError extends StatelessWidget {
  const _BuildError({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) => Semantics(
    liveRegion: true,
    child: Container(
      key: const Key('revision3-dataasset-build-error'),
      padding: const EdgeInsets.all(10),
      color: Theme.of(context).colorScheme.errorContainer,
      child: Text(message),
    ),
  );
}
