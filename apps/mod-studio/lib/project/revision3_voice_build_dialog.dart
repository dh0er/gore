import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:path/path.dart' as p;

import '../core/mod_ffi.dart';
import 'revision3_voice_authoring.dart';

typedef Revision3VoiceExactBuild =
    Future<AuthoringRevision3VoiceBuildResult> Function(String output);

typedef Revision3VoiceBuildParentDirectoryPicker = Future<String?> Function();

/// Focused offline-only surface for one exact managed-R3 Voice build.
///
/// The picker supplies an existing parent. The dialog accepts only one safe,
/// portable child name and rechecks that the resulting output does not exist
/// immediately before entering the native write-new build boundary.
class Revision3VoiceBuildDialog extends StatefulWidget {
  const Revision3VoiceBuildDialog({
    required this.build,
    required this.pickExistingParentDirectory,
    super.key,
  });

  final Revision3VoiceExactBuild build;
  final Revision3VoiceBuildParentDirectoryPicker pickExistingParentDirectory;

  @override
  State<Revision3VoiceBuildDialog> createState() =>
      _Revision3VoiceBuildDialogState();
}

class _Revision3VoiceBuildDialogState extends State<Revision3VoiceBuildDialog> {
  final _formKey = GlobalKey<FormState>();
  final _folderName = TextEditingController(text: 'voice-bundle');

  String? _parentDirectory;
  String? _error;
  bool _choosingParent = false;
  bool _building = false;
  bool _terminal = false;
  AuthoringRevision3VoiceBuildResult? _result;

  bool get _busy => _choosingParent || _building;

  String? get _outputPreview {
    final parent = _parentDirectory;
    final name = _folderName.text;
    if (parent == null || _validateFolderName(name) != null) return null;
    return p.join(parent, name);
  }

  @override
  void dispose() {
    _folderName.dispose();
    super.dispose();
  }

  Future<void> _chooseParent() async {
    if (_busy || _terminal || _result != null) return;
    setState(() {
      _choosingParent = true;
      _error = null;
    });
    try {
      final selected = await widget.pickExistingParentDirectory();
      if (!mounted || selected == null) return;
      final validation = _validateParentDirectory(selected);
      if (validation != null) {
        setState(() => _error = validation);
        return;
      }
      setState(() => _parentDirectory = p.normalize(selected));
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _error =
            'The parent folder could not be inspected safely. No build or deployment was attempted.';
      });
    } finally {
      if (mounted) setState(() => _choosingParent = false);
    }
  }

  Future<void> _build() async {
    if (_busy || _terminal || _result != null) return;
    if (!(_formKey.currentState?.validate() ?? false)) return;
    final parent = _parentDirectory;
    if (parent == null) {
      setState(() => _error = 'Choose an existing parent folder.');
      return;
    }

    setState(() {
      _building = true;
      _error = null;
    });
    try {
      final parentError = _validateParentDirectory(parent);
      if (parentError != null) {
        if (mounted) setState(() => _error = parentError);
        return;
      }
      final output = p.join(parent, _folderName.text);
      final outputType = FileSystemEntity.typeSync(output, followLinks: false);
      if (outputType != FileSystemEntityType.notFound) {
        if (!mounted) return;
        setState(() {
          _error = outputType == FileSystemEntityType.link
              ? 'The target path is a symlink. Choose a different new folder name.'
              : 'The target already exists. Choose a different new folder name.';
        });
        return;
      }

      final result = await widget.build(output);
      if (!mounted) return;
      setState(() => _result = result);
    } on Revision3VoiceBuildRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _terminal = true;
        _error =
            'This project can no longer be verified as current. Close this window and reopen the managed project before building another Voice bundle.';
      });
    } on Revision3VoiceBuildStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _terminal = true;
        _error =
            'The managed project changed while this window was open. Close this build window and open it again from the current project.';
      });
    } on ModFfiException catch (error) {
      if (!mounted) return;
      setState(() {
        _terminal =
            error.code ==
            'AUTHORING_REVISION3_VOICE_BUILD_PUBLICATION_UNCONFIRMED';
        _error = _voiceBuildErrorMessage(error);
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _error =
            'The Voice bundle could not be built exactly. No deployment was attempted. Before retrying, choose a new folder name if output was created.';
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
        key: const Key('revision3-voice-build-dialog'),
        title: const Text('Build Voice bundle'),
        content: SizedBox(
          width: 620,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const _OfflineBuildNotice(),
                const SizedBox(height: 16),
                Form(
                  key: _formKey,
                  child: TextFormField(
                    key: const Key('revision3-voice-build-folder-name'),
                    controller: _folderName,
                    enabled: !_busy && !_terminal && result == null,
                    autovalidateMode: AutovalidateMode.onUserInteraction,
                    decoration: const InputDecoration(
                      labelText: 'New folder name',
                      helperText:
                          'The bundle must be written to a brand-new child folder.',
                      border: OutlineInputBorder(),
                    ),
                    validator: _validateFolderName,
                    onChanged: (_) => setState(() => _error = null),
                  ),
                ),
                const SizedBox(height: 12),
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    OutlinedButton.icon(
                      key: const Key('revision3-voice-build-choose-parent'),
                      onPressed: _busy || _terminal || result != null
                          ? null
                          : _chooseParent,
                      icon: _choosingParent
                          ? const SizedBox.square(
                              dimension: 16,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Icon(Icons.folder_open_outlined),
                      label: const Text('Choose parent folder'),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        _parentDirectory ?? 'No parent folder selected',
                        key: const Key('revision3-voice-build-parent'),
                      ),
                    ),
                  ],
                ),
                if (_outputPreview case final output?) ...[
                  const SizedBox(height: 12),
                  _BuildFact(
                    label: 'New output',
                    value: output,
                    valueKey: const Key('revision3-voice-build-output-preview'),
                  ),
                ],
                if (_error case final error?) ...[
                  const SizedBox(height: 14),
                  _BuildError(message: error),
                ],
                if (result != null) ...[
                  const Divider(height: 32),
                  if (result.isBuilt)
                    _BuiltReceipt(result: result)
                  else
                    _BlockedReport(result: result),
                ],
              ],
            ),
          ),
        ),
        actions: [
          TextButton(
            key: const Key('revision3-voice-build-close'),
            onPressed: _busy ? null : () => Navigator.of(context).pop(result),
            child: Text(result == null ? 'Cancel' : 'Close'),
          ),
          if (result == null)
            FilledButton.icon(
              key: const Key('revision3-voice-build-submit'),
              onPressed:
                  !_busy &&
                      !_terminal &&
                      _parentDirectory != null &&
                      _validateFolderName(_folderName.text) == null
                  ? _build
                  : null,
              icon: _building
                  ? const SizedBox.square(
                      dimension: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.inventory_2_outlined),
              label: const Text('Build offline bundle'),
            ),
        ],
      ),
    );
  }
}

String? _validateParentDirectory(String value) {
  if (value.isEmpty || !p.isAbsolute(value)) {
    return 'Choose an absolute existing parent folder.';
  }
  final type = FileSystemEntity.typeSync(value, followLinks: false);
  return switch (type) {
    FileSystemEntityType.directory => null,
    FileSystemEntityType.link =>
      'The selected parent is a symlink. Choose a real existing folder.',
    _ => 'Choose an existing parent folder.',
  };
}

String? _validateFolderName(String? raw) {
  final value = raw ?? '';
  if (value.isEmpty) return 'Enter a new folder name.';
  if (value.trim() != value) {
    return 'The folder name cannot start or end with whitespace.';
  }
  if (utf8.encode(value).length > 255) {
    return 'The folder name is too long.';
  }
  if (value == '.' ||
      value == '..' ||
      value.contains('/') ||
      value.contains(r'\') ||
      value.contains(':') ||
      RegExp(r'[<>"|?*]').hasMatch(value) ||
      value.runes.any(
        (rune) => rune <= 0x1f || (rune >= 0x7f && rune <= 0x9f),
      ) ||
      value.endsWith('.') ||
      value.endsWith(' ')) {
    return 'Use one portable folder name without separators or reserved characters.';
  }
  final stem = value.split('.').first.replaceFirst(RegExp(r'[ .]+$'), '');
  final folded = stem.toUpperCase();
  if (const {
        'CON',
        'PRN',
        'AUX',
        'NUL',
        r'CLOCK$',
        r'CONIN$',
        r'CONOUT$',
      }.contains(folded) ||
      RegExp(r'^(COM|LPT)([1-9¹²³])$').hasMatch(folded)) {
    return 'That folder name is reserved by Windows.';
  }
  return null;
}

class _OfflineBuildNotice extends StatelessWidget {
  const _OfflineBuildNotice();

  @override
  Widget build(BuildContext context) => Container(
    key: const Key('revision3-voice-build-offline-notice'),
    padding: const EdgeInsets.all(12),
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.secondaryContainer,
      borderRadius: BorderRadius.circular(8),
    ),
    child: const Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(Icons.offline_bolt_outlined),
        SizedBox(width: 10),
        Expanded(
          child: Text(
            'Offline build only. This creates a sealed existing-member Voice bundle. It does not deploy or write to the game.',
          ),
        ),
      ],
    ),
  );
}

class _BuildError extends StatelessWidget {
  const _BuildError({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) => Container(
    key: const Key('revision3-voice-build-error'),
    padding: const EdgeInsets.all(12),
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.errorContainer,
      borderRadius: BorderRadius.circular(8),
    ),
    child: Text(
      message,
      style: TextStyle(color: Theme.of(context).colorScheme.onErrorContainer),
    ),
  );
}

class _BlockedReport extends StatelessWidget {
  const _BlockedReport({required this.result});

  final AuthoringRevision3VoiceBuildResult result;

  @override
  Widget build(BuildContext context) {
    final report = result.report!;
    return Column(
      key: const Key('revision3-voice-build-blocked'),
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text('Build blocked', style: Theme.of(context).textTheme.titleMedium),
        const SizedBox(height: 4),
        Text(
          '${report.readySlots} of ${report.totalSlots} Voice slots are ready. No bundle was created and deployment was not performed.',
        ),
        const SizedBox(height: 8),
        _BuildFact(
          label: 'Basis project revision',
          value: '${result.projectRevision}',
        ),
        const SizedBox(height: 10),
        for (final blocker in report.blockers)
          _BuildBlockerRow(blocker: blocker),
      ],
    );
  }
}

class _BuildBlockerRow extends StatelessWidget {
  const _BuildBlockerRow({required this.blocker});

  final AuthoringRevision3VoiceBuildBlocker blocker;

  @override
  Widget build(BuildContext context) {
    final title = switch (blocker.reason) {
      AuthoringRevision3VoiceBuildBlockReason.noVoiceSlots =>
        'No Voice slots exist in this project.',
      AuthoringRevision3VoiceBuildBlockReason.voicePayloadBudgetExceeded =>
        'The selected Voice payloads exceed the safe bundle memory budget.',
      AuthoringRevision3VoiceBuildBlockReason.unresolvedTarget =>
        'Resolve this Voice target.',
      AuthoringRevision3VoiceBuildBlockReason.ambiguousTarget =>
        'This Voice target is ambiguous.',
      AuthoringRevision3VoiceBuildBlockReason.unqualifiedAdd =>
        'This target is not a sealed existing-member replacement.',
      AuthoringRevision3VoiceBuildBlockReason.missingSelectedTake =>
        'Select an approved Voice take.',
      AuthoringRevision3VoiceBuildBlockReason.selectedTakeNotApproved =>
        'The selected Voice take is not approved.',
      AuthoringRevision3VoiceBuildBlockReason.selectedTakeCodecUnqualified =>
        'The selected Voice take uses an unsupported codec.',
      AuthoringRevision3VoiceBuildBlockReason.voiceSlotLimitExceeded =>
        'This project exceeds the 1024-slot Voice bundle limit.',
    };
    final locale = blocker.locale;
    final lineLabel = blocker.lineLabel;
    return ListTile(
      dense: true,
      contentPadding: EdgeInsets.zero,
      leading: const Icon(Icons.block_outlined),
      title: Text(title),
      subtitle: locale == null || lineLabel == null
          ? null
          : Text('$lineLabel · $locale'),
    );
  }
}

String _voiceBuildErrorMessage(ModFfiException error) => switch (error.code) {
  'AUTHORING_REVISION3_VOICE_BUILD_EXECUTABLE_UNAVAILABLE' =>
    'The installed game executable could not be read. Finish any game update and check the configured installation before trying again. No deployment was attempted.',
  'AUTHORING_REVISION3_VOICE_BUILD_EXECUTABLE_MISMATCH' =>
    'The installed game executable no longer matches this project generation. Re-import or retarget the managed project before building again. No deployment was attempted.',
  'AUTHORING_REVISION3_VOICE_BUILD_GAME_UNAVAILABLE' =>
    'The configured Gothic 1 Remake installation is unavailable. Check it in Settings before trying again. No deployment was attempted.',
  'AUTHORING_REVISION3_VOICE_BUILD_STORE_GAME_ALIAS' =>
    'This project folder overlaps the configured game installation. Move the project outside the game folder before building. No deployment was attempted.',
  'AUTHORING_REVISION3_VOICE_BUILD_GAME_OUTPUT_ALIAS' =>
    'The bundle output overlaps a Gothic 1 Remake installation. Choose a parent folder outside every game installation. No deployment was attempted.',
  'AUTHORING_REVISION3_VOICE_BUILD_STORE_OUTPUT_ALIAS' =>
    'The bundle output overlaps the managed project. Choose a parent folder outside the project. No deployment was attempted.',
  'AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_UNAVAILABLE' =>
    'The selected output parent is unavailable or cannot be traversed safely. Choose a real existing parent folder outside the project and game.',
  'AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_FAILED' =>
    'The new bundle folder could not be written completely. Do not use any output left there; choose a different new folder name before retrying. No deployment was attempted.',
  'AUTHORING_REVISION3_VOICE_BUILD_PROMOTION_FAILED' =>
    'The sealed bundle could not be promoted into the requested new output folder. A conflicting output was left untouched and owned staging was removed. Choose a different new folder name before retrying. No deployment was attempted.',
  'AUTHORING_REVISION3_VOICE_BUILD_CLEANUP_FAILED' =>
    'The Voice bundle was not published, but its temporary staging folder could not be removed completely. Remove the reported staging folder before retrying. No deployment was attempted.\n\nCleanup detail: ${error.message}',
  'AUTHORING_REVISION3_VOICE_BUILD_PUBLICATION_UNCONFIRMED' =>
    'The atomic publication may have succeeded, but its final identity or durability could not be confirmed. Do not retry, replace, or delete that exact output yet. Close this window and inspect the reported folder before deciding how to proceed. No deployment was attempted.\n\nPublication detail: ${error.message}',
  'AUTHORING_REVISION3_VOICE_BUILD_STORE_ROOT_CHANGED' =>
    'The managed project root changed while the bundle was being built. Close this window and reopen the project before building again. No deployment was attempted.',
  'AUTHORING_REVISION3_VOICE_BUILD_GAME_ROOT_CHANGED' =>
    'The game installation changed while the bundle was being built. Finish the update or file operation, then retry with a new folder name. No deployment was attempted.',
  'AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_ROOT_CHANGED' =>
    'The output parent changed while the bundle was being built. Finish the file operation, verify the parent, then retry with a new folder name. No deployment was attempted.',
  'AUTHORING_REVISION3_VOICE_BUILD_VERIFY_FAILED' =>
    'The written bundle could not be verified exactly. Do not use that output; choose a different new folder name before retrying. No deployment was attempted.',
  'AUTHORING_REVISION3_VOICE_BUILD_BUNDLE_INVALID' =>
    'The selected Voice content could not be lowered into one exact sealed bundle. Reopen the project, review its Voice slots, and try again. No deployment was attempted.',
  'AUTHORING_REVISION3_VOICE_BUILD_INPUT_INVALID' ||
  'AUTHORING_REVISION3_VOICE_BUILD_INPUT_LIMIT' =>
    'The Voice build request or output path exceeds the safe supported limits. Choose a shorter new output path and try again. No deployment was attempted.',
  'AUTHORING_REVISION3_VOICE_BUILD_RESPONSE_LIMIT' =>
    'The bundle was too large to return an exact build receipt. Do not use any unreceipted output; choose a new folder only after reducing the Voice build. No deployment was attempted.',
  _ =>
    'The Voice bundle could not be built exactly. No deployment was attempted. Before retrying, choose a new folder name if output was created.',
};

class _BuiltReceipt extends StatelessWidget {
  const _BuiltReceipt({required this.result});

  final AuthoringRevision3VoiceBuildResult result;

  @override
  Widget build(BuildContext context) => Column(
    key: const Key('revision3-voice-build-built'),
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      Text(
        'Sealed Voice bundle built',
        style: Theme.of(context).textTheme.titleMedium,
      ),
      const SizedBox(height: 4),
      const Text('Offline receipt only. Deployment was not performed.'),
      const SizedBox(height: 12),
      _BuildFact(
        label: 'Basis project revision',
        value: '${result.projectRevision}',
      ),
      _BuildFact(label: 'Output', value: result.output!),
      _BuildFact(label: 'Archive edits', value: '${result.editCount}'),
      _BuildFact(label: 'Bundle files', value: '${result.fileCount}'),
      _BuildFact(label: 'Sealed bytes', value: '${result.bundleBytes}'),
      _BuildFact(
        label: 'Bundle SHA-256',
        value: result.bundleSha256!,
        valueKey: const Key('revision3-voice-build-bundle-sha256'),
      ),
    ],
  );
}

class _BuildFact extends StatelessWidget {
  const _BuildFact({required this.label, required this.value, this.valueKey});

  final String label;
  final String value;
  final Key? valueKey;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(bottom: 6),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 120,
          child: Text(label, style: Theme.of(context).textTheme.labelLarge),
        ),
        const SizedBox(width: 8),
        Expanded(child: SelectableText(value, key: valueKey)),
      ],
    ),
  );
}
