import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:path/path.dart' as p;

import '../core/mod_ffi.dart';
import 'revision3_voice_authoring.dart';
import 'revision3_voice_build_readiness_panel.dart';

typedef Revision3VoiceExactBuild =
    Future<AuthoringRevision3VoiceBuildResult> Function(String output);

typedef Revision3VoiceBuildParentDirectoryPicker = Future<String?> Function();
typedef Revision3VoiceBuildDeepLinkFailure = FutureOr<void> Function();

/// All author-facing copy rendered by the offline Voice build dialog.
@immutable
final class Revision3VoiceBuildDialogCopy {
  const Revision3VoiceBuildDialogCopy({
    required this.readiness,
    required this.title,
    required this.offlineNotice,
    required this.newFolderNameLabel,
    required this.newFolderNameHelp,
    required this.chooseParentFolderLabel,
    required this.noParentFolderSelected,
    required this.newOutputLabel,
    required this.cancelLabel,
    required this.closeLabel,
    required this.buildOfflineBundleLabel,
    required this.parentInspectFailed,
    required this.chooseExistingParent,
    required this.targetSymlink,
    required this.targetExists,
    required this.requiresReopen,
    required this.staleCheckpoint,
    required this.buildFailed,
    required this.planRequiresReopen,
    required this.planStaleCheckpoint,
    required this.planFailed,
    required this.parentMustBeAbsolute,
    required this.parentSymlink,
    required this.parentMustExist,
    required this.folderNameRequired,
    required this.folderNameWhitespace,
    required this.folderNameTooLong,
    required this.folderNamePortable,
    required this.folderNameWindowsReserved,
    required this.executableUnavailable,
    required this.executableMismatch,
    required this.gameUnavailable,
    required this.storeGameAlias,
    required this.gameOutputAlias,
    required this.storeOutputAlias,
    required this.outputUnavailable,
    required this.outputFailed,
    required this.promotionFailed,
    required this.cleanupFailed,
    required this.publicationUnconfirmed,
    required this.storeRootChanged,
    required this.gameRootChanged,
    required this.outputRootChanged,
    required this.verifyFailed,
    required this.bundleInvalid,
    required this.inputInvalid,
    required this.responseLimit,
    required this.builtTitle,
    required this.offlineReceipt,
    required this.basisRevisionLabel,
    required this.outputLabel,
    required this.archiveEditsLabel,
    required this.bundleFilesLabel,
    required this.sealedBytesLabel,
    required this.bundleSha256Label,
  });

  const Revision3VoiceBuildDialogCopy.english()
    : readiness = const Revision3VoiceBuildReadinessCopy.english(),
      title = 'Build Voice bundle',
      offlineNotice =
          'Offline build only. This creates a sealed existing-member Voice bundle. It does not deploy or write to the game.',
      newFolderNameLabel = 'New folder name',
      newFolderNameHelp =
          'The bundle must be written to a brand-new child folder.',
      chooseParentFolderLabel = 'Choose parent folder',
      noParentFolderSelected = 'No parent folder selected',
      newOutputLabel = 'New output',
      cancelLabel = 'Cancel',
      closeLabel = 'Close',
      buildOfflineBundleLabel = 'Build offline bundle',
      parentInspectFailed =
          'The parent folder could not be inspected safely. No build or deployment was attempted.',
      chooseExistingParent = 'Choose an existing parent folder.',
      targetSymlink =
          'The target path is a symlink. Choose a different new folder name.',
      targetExists =
          'The target already exists. Choose a different new folder name.',
      requiresReopen =
          'This project can no longer be verified as current. Close this window and reopen the managed project before building another Voice bundle.',
      staleCheckpoint =
          'The managed project changed while this window was open. Close this build window and open it again from the current project.',
      buildFailed =
          'The Voice bundle could not be built exactly. No deployment was attempted. Before retrying, choose a new folder name if output was created.',
      planRequiresReopen =
          'This project can no longer be verified as current. Close this window and reopen the managed project before building a Voice bundle.',
      planStaleCheckpoint =
          'The managed project changed while this window was open. Close this build window and open it again from the current project.',
      planFailed =
          'Voice readiness could not be verified for the exact current project. Output selection and build are unavailable until verification succeeds.',
      parentMustBeAbsolute = 'Choose an absolute existing parent folder.',
      parentSymlink =
          'The selected parent is a symlink. Choose a real existing folder.',
      parentMustExist = 'Choose an existing parent folder.',
      folderNameRequired = 'Enter a new folder name.',
      folderNameWhitespace =
          'The folder name cannot start or end with whitespace.',
      folderNameTooLong = 'The folder name is too long.',
      folderNamePortable =
          'Use one portable folder name without separators or reserved characters.',
      folderNameWindowsReserved = 'That folder name is reserved by Windows.',
      executableUnavailable =
          'The installed game executable could not be read. Finish any game update and check the configured installation before trying again. No deployment was attempted.',
      executableMismatch =
          'The installed game executable no longer matches this project generation. Re-import or retarget the managed project before building again. No deployment was attempted.',
      gameUnavailable =
          'The configured Gothic 1 Remake installation is unavailable. Check it in Settings before trying again. No deployment was attempted.',
      storeGameAlias =
          'This project folder overlaps the configured game installation. Move the project outside the game folder before building. No deployment was attempted.',
      gameOutputAlias =
          'The bundle output overlaps a Gothic 1 Remake installation. Choose a parent folder outside every game installation. No deployment was attempted.',
      storeOutputAlias =
          'The bundle output overlaps the managed project. Choose a parent folder outside the project. No deployment was attempted.',
      outputUnavailable =
          'The selected output parent is unavailable or cannot be traversed safely. Choose a real existing parent folder outside the project and game.',
      outputFailed =
          'The new bundle folder could not be written completely. Do not use any output left there; choose a different new folder name before retrying. No deployment was attempted.',
      promotionFailed =
          'The sealed bundle could not be promoted into the requested new output folder. A conflicting output was left untouched and owned staging was removed. Choose a different new folder name before retrying. No deployment was attempted.',
      cleanupFailed =
          'The Voice bundle was not published, but its temporary staging folder could not be removed completely. Remove the reported staging folder before retrying. No deployment was attempted.',
      publicationUnconfirmed =
          'The atomic publication may have succeeded, but its final identity or durability could not be confirmed. Do not retry, replace, or delete that exact output yet. Close this window and inspect the reported folder before deciding how to proceed. No deployment was attempted.',
      storeRootChanged =
          'The managed project root changed while the bundle was being built. Close this window and reopen the project before building again. No deployment was attempted.',
      gameRootChanged =
          'The game installation changed while the bundle was being built. Finish the update or file operation, then retry with a new folder name. No deployment was attempted.',
      outputRootChanged =
          'The output parent changed while the bundle was being built. Finish the file operation, verify the parent, then retry with a new folder name. No deployment was attempted.',
      verifyFailed =
          'The written bundle could not be verified exactly. Do not use that output; choose a different new folder name before retrying. No deployment was attempted.',
      bundleInvalid =
          'The selected Voice content could not be lowered into one exact sealed bundle. Reopen the project, review its Voice slots, and try again. No deployment was attempted.',
      inputInvalid =
          'The Voice build request or output path exceeds the safe supported limits. Choose a shorter new output path and try again. No deployment was attempted.',
      responseLimit =
          'The bundle was too large to return an exact build receipt. Do not use any unreceipted output; choose a new folder only after reducing the Voice build. No deployment was attempted.',
      builtTitle = 'Sealed Voice bundle built',
      offlineReceipt = 'Offline receipt only. Deployment was not performed.',
      basisRevisionLabel = 'Basis project revision',
      outputLabel = 'Output',
      archiveEditsLabel = 'Archive edits',
      bundleFilesLabel = 'Bundle files',
      sealedBytesLabel = 'Sealed bytes',
      bundleSha256Label = 'Bundle SHA-256';

  final Revision3VoiceBuildReadinessCopy readiness;
  final String title;
  final String offlineNotice;
  final String newFolderNameLabel;
  final String newFolderNameHelp;
  final String chooseParentFolderLabel;
  final String noParentFolderSelected;
  final String newOutputLabel;
  final String cancelLabel;
  final String closeLabel;
  final String buildOfflineBundleLabel;
  final String parentInspectFailed;
  final String chooseExistingParent;
  final String targetSymlink;
  final String targetExists;
  final String requiresReopen;
  final String staleCheckpoint;
  final String buildFailed;
  final String planRequiresReopen;
  final String planStaleCheckpoint;
  final String planFailed;
  final String parentMustBeAbsolute;
  final String parentSymlink;
  final String parentMustExist;
  final String folderNameRequired;
  final String folderNameWhitespace;
  final String folderNameTooLong;
  final String folderNamePortable;
  final String folderNameWindowsReserved;
  final String executableUnavailable;
  final String executableMismatch;
  final String gameUnavailable;
  final String storeGameAlias;
  final String gameOutputAlias;
  final String storeOutputAlias;
  final String outputUnavailable;
  final String outputFailed;
  final String promotionFailed;
  final String cleanupFailed;
  final String publicationUnconfirmed;
  final String storeRootChanged;
  final String gameRootChanged;
  final String outputRootChanged;
  final String verifyFailed;
  final String bundleInvalid;
  final String inputInvalid;
  final String responseLimit;
  final String builtTitle;
  final String offlineReceipt;
  final String basisRevisionLabel;
  final String outputLabel;
  final String archiveEditsLabel;
  final String bundleFilesLabel;
  final String sealedBytesLabel;
  final String bundleSha256Label;

  String planError(Object error) => switch (error) {
    Revision3VoiceBuildRequiresReopenException() => planRequiresReopen,
    Revision3VoiceBuildStaleCheckpointException() => planStaleCheckpoint,
    _ => planFailed,
  };

  String buildError(ModFfiException error) => switch (error.code) {
    'AUTHORING_REVISION3_VOICE_BUILD_EXECUTABLE_UNAVAILABLE' =>
      executableUnavailable,
    'AUTHORING_REVISION3_VOICE_BUILD_EXECUTABLE_MISMATCH' => executableMismatch,
    'AUTHORING_REVISION3_VOICE_BUILD_GAME_UNAVAILABLE' => gameUnavailable,
    'AUTHORING_REVISION3_VOICE_BUILD_STORE_GAME_ALIAS' => storeGameAlias,
    'AUTHORING_REVISION3_VOICE_BUILD_GAME_OUTPUT_ALIAS' => gameOutputAlias,
    'AUTHORING_REVISION3_VOICE_BUILD_STORE_OUTPUT_ALIAS' => storeOutputAlias,
    'AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_UNAVAILABLE' => outputUnavailable,
    'AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_FAILED' => outputFailed,
    'AUTHORING_REVISION3_VOICE_BUILD_PROMOTION_FAILED' => promotionFailed,
    'AUTHORING_REVISION3_VOICE_BUILD_CLEANUP_FAILED' =>
      '$cleanupFailed\n\n${error.message}',
    'AUTHORING_REVISION3_VOICE_BUILD_PUBLICATION_UNCONFIRMED' =>
      '$publicationUnconfirmed\n\n${error.message}',
    'AUTHORING_REVISION3_VOICE_BUILD_STORE_ROOT_CHANGED' => storeRootChanged,
    'AUTHORING_REVISION3_VOICE_BUILD_GAME_ROOT_CHANGED' => gameRootChanged,
    'AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_ROOT_CHANGED' => outputRootChanged,
    'AUTHORING_REVISION3_VOICE_BUILD_VERIFY_FAILED' => verifyFailed,
    'AUTHORING_REVISION3_VOICE_BUILD_BUNDLE_INVALID' => bundleInvalid,
    'AUTHORING_REVISION3_VOICE_BUILD_INPUT_INVALID' ||
    'AUTHORING_REVISION3_VOICE_BUILD_INPUT_LIMIT' => inputInvalid,
    'AUTHORING_REVISION3_VOICE_BUILD_RESPONSE_LIMIT' => responseLimit,
    _ => buildFailed,
  };
}

/// Focused offline-only surface for one exact managed-R3 Voice build.
///
/// The picker supplies an existing parent. The dialog accepts only one safe,
/// portable child name and rechecks that the resulting output does not exist
/// immediately before entering the native write-new build boundary.
class Revision3VoiceBuildDialog extends StatefulWidget {
  const Revision3VoiceBuildDialog({
    required this.plan,
    required this.build,
    required this.pickExistingParentDirectory,
    required this.onDeepLinkFailure,
    this.onResolveVoiceTarget,
    this.onManageVoiceTakes,
    this.copy = const Revision3VoiceBuildDialogCopy.english(),
    super.key,
  });

  final Revision3VoiceBuildPlanLoader plan;
  final Revision3VoiceExactBuild build;
  final Revision3VoiceBuildParentDirectoryPicker pickExistingParentDirectory;
  final Revision3VoiceBuildLineLocaleAction? onResolveVoiceTarget;
  final Revision3VoiceBuildLineLocaleAction? onManageVoiceTakes;

  /// Feedback owned by a stable surface outside this dialog.
  ///
  /// Blocker navigation closes the dialog before it starts. If that navigation
  /// then fails, the disposed dialog cannot render an inline error, so Home
  /// binds this callback to its still-mounted [ScaffoldMessengerState].
  final Revision3VoiceBuildDeepLinkFailure onDeepLinkFailure;
  final Revision3VoiceBuildDialogCopy copy;

  @override
  State<Revision3VoiceBuildDialog> createState() =>
      _Revision3VoiceBuildDialogState();
}

class _Revision3VoiceBuildDialogState extends State<Revision3VoiceBuildDialog> {
  final _formKey = GlobalKey<FormState>();
  final _folderName = TextEditingController(text: 'voice-bundle');

  String? _parentDirectory;
  String? _error;
  Object? _planError;
  bool _planning = false;
  bool _choosingParent = false;
  bool _building = false;
  bool _terminal = false;
  int _planEpoch = 0;
  AuthoringRevision3VoiceBuildPlanResult? _plan;
  AuthoringRevision3VoiceBuildResult? _result;

  bool get _busy => _choosingParent || _building;

  String? get _outputPreview {
    final parent = _parentDirectory;
    final name = _folderName.text;
    if (parent == null || _validateFolderName(name, widget.copy) != null) {
      return null;
    }
    return p.join(parent, name);
  }

  @override
  void initState() {
    super.initState();
    unawaited(_loadPlan());
  }

  @override
  void dispose() {
    _planEpoch++;
    _folderName.dispose();
    super.dispose();
  }

  Future<void> _loadPlan() async {
    if (_planning || _building || _terminal) return;
    final epoch = ++_planEpoch;
    setState(() {
      _planning = true;
      _planError = null;
    });
    try {
      final plan = await widget.plan();
      if (!mounted || epoch != _planEpoch) return;
      setState(() {
        _plan = plan;
        _result = null;
        _error = null;
        _planning = false;
      });
    } on Revision3VoiceBuildRequiresReopenException catch (error) {
      if (!mounted || epoch != _planEpoch) return;
      setState(() {
        _terminal = true;
        _plan = null;
        _planError = error;
        _planning = false;
      });
    } on Revision3VoiceBuildStaleCheckpointException catch (error) {
      if (!mounted || epoch != _planEpoch) return;
      setState(() {
        _terminal = true;
        _plan = null;
        _planError = error;
        _planning = false;
      });
    } catch (error) {
      if (!mounted || epoch != _planEpoch) return;
      setState(() {
        _plan = null;
        _result = null;
        _planError = error;
        _planning = false;
      });
    }
  }

  Revision3VoiceBuildLineLocaleAction? _closeBeforeAction(
    Revision3VoiceBuildLineLocaleAction? action,
  ) {
    if (action == null) return null;
    return ({required initialLineId, required initialLocale}) async {
      final onFailure = widget.onDeepLinkFailure;
      final route = ModalRoute.of(context);
      Navigator.of(context).pop(_result);
      if (route != null) await route.completed;
      try {
        await Future<void>.sync(
          () => action(
            initialLineId: initialLineId,
            initialLocale: initialLocale,
          ),
        );
      } catch (_) {
        await Future<void>.sync(onFailure);
      }
    };
  }

  Future<void> _chooseParent() async {
    if (_busy || _terminal || _result != null || _plan?.isReady != true) {
      return;
    }
    setState(() {
      _choosingParent = true;
      _error = null;
    });
    try {
      final selected = await widget.pickExistingParentDirectory();
      if (!mounted || selected == null) return;
      final validation = _validateParentDirectory(selected, widget.copy);
      if (validation != null) {
        setState(() => _error = validation);
        return;
      }
      setState(() => _parentDirectory = p.normalize(selected));
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _error = widget.copy.parentInspectFailed;
      });
    } finally {
      if (mounted) setState(() => _choosingParent = false);
    }
  }

  Future<void> _build() async {
    if (_busy || _terminal || _result != null || _plan?.isReady != true) {
      return;
    }
    if (!(_formKey.currentState?.validate() ?? false)) return;
    final parent = _parentDirectory;
    if (parent == null) {
      setState(() => _error = widget.copy.chooseExistingParent);
      return;
    }

    setState(() {
      _building = true;
      _error = null;
    });
    try {
      final parentError = _validateParentDirectory(parent, widget.copy);
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
              ? widget.copy.targetSymlink
              : widget.copy.targetExists;
        });
        return;
      }

      final result = await widget.build(output);
      if (!mounted) return;
      setState(() {
        _result = result;
        if (!result.isBuilt) _plan = null;
      });
    } on Revision3VoiceBuildRequiresReopenException {
      if (!mounted) return;
      setState(() {
        _terminal = true;
        _error = widget.copy.requiresReopen;
      });
    } on Revision3VoiceBuildStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _terminal = true;
        _error = widget.copy.staleCheckpoint;
      });
    } on ModFfiException catch (error) {
      if (!mounted) return;
      setState(() {
        _terminal =
            error.code ==
            'AUTHORING_REVISION3_VOICE_BUILD_PUBLICATION_UNCONFIRMED';
        _error = widget.copy.buildError(error);
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _error = widget.copy.buildFailed;
      });
    } finally {
      if (mounted) setState(() => _building = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final result = _result;
    final plan = _plan;
    final resolveVoiceTarget = _closeBeforeAction(widget.onResolveVoiceTarget);
    final manageVoiceTakes = _closeBeforeAction(widget.onManageVoiceTakes);
    final readyForOutput =
        !_planning &&
        _planError == null &&
        result == null &&
        plan?.isReady == true;
    return PopScope(
      canPop: !_busy,
      child: AlertDialog(
        key: const Key('revision3-voice-build-dialog'),
        title: Text(widget.copy.title),
        content: SizedBox(
          width: 620,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _OfflineBuildNotice(copy: widget.copy),
                const SizedBox(height: 16),
                if (_planning)
                  Semantics(
                    liveRegion: true,
                    label: widget.copy.readiness.checkingSemanticsLabel,
                    child: const LinearProgressIndicator(
                      key: Key('revision3-voice-build-plan-loading'),
                    ),
                  )
                else if (_planError case final error?)
                  _VoiceBuildPlanError(
                    message: widget.copy.planError(error),
                    retryLabel: widget.copy.readiness.retryLabel,
                    retry: _terminal ? null : _loadPlan,
                  )
                else if (result != null && result.isBuilt)
                  _BuiltReceipt(copy: widget.copy, result: result)
                else if (result != null)
                  Revision3VoiceBuildReadinessReport(
                    projectRevision: result.projectRevision,
                    totalSlots: result.report!.totalSlots,
                    readySlots: result.report!.readySlots,
                    blockers: result.report!.blockers,
                    isReady: false,
                    onResolveVoiceTarget: resolveVoiceTarget,
                    onManageVoiceTakes: manageVoiceTakes,
                    copy: widget.copy.readiness,
                  )
                else if (plan != null) ...[
                  Revision3VoiceBuildReadinessReport(
                    projectRevision: plan.projectRevision,
                    totalSlots: plan.totalSlots,
                    readySlots: plan.readySlots,
                    blockers: plan.blockers,
                    isReady: plan.isReady,
                    onResolveVoiceTarget: resolveVoiceTarget,
                    onManageVoiceTakes: manageVoiceTakes,
                    copy: widget.copy.readiness,
                  ),
                  if (readyForOutput) ...[
                    const Divider(height: 28),
                    Form(
                      key: _formKey,
                      child: TextFormField(
                        key: const Key('revision3-voice-build-folder-name'),
                        controller: _folderName,
                        enabled: !_busy && !_terminal,
                        autovalidateMode: AutovalidateMode.onUserInteraction,
                        decoration: InputDecoration(
                          labelText: widget.copy.newFolderNameLabel,
                          helperText: widget.copy.newFolderNameHelp,
                          border: const OutlineInputBorder(),
                        ),
                        validator: (value) =>
                            _validateFolderName(value, widget.copy),
                        onChanged: (_) => setState(() => _error = null),
                      ),
                    ),
                    const SizedBox(height: 12),
                    Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        OutlinedButton.icon(
                          key: const Key('revision3-voice-build-choose-parent'),
                          onPressed: _busy || _terminal ? null : _chooseParent,
                          icon: _choosingParent
                              ? const SizedBox.square(
                                  dimension: 16,
                                  child: CircularProgressIndicator(
                                    strokeWidth: 2,
                                  ),
                                )
                              : const Icon(Icons.folder_open_outlined),
                          label: Text(widget.copy.chooseParentFolderLabel),
                        ),
                        const SizedBox(width: 12),
                        Expanded(
                          child: Text(
                            _parentDirectory ??
                                widget.copy.noParentFolderSelected,
                            key: const Key('revision3-voice-build-parent'),
                          ),
                        ),
                      ],
                    ),
                    if (_outputPreview case final output?) ...[
                      const SizedBox(height: 12),
                      _BuildFact(
                        label: widget.copy.newOutputLabel,
                        value: output,
                        valueKey: const Key(
                          'revision3-voice-build-output-preview',
                        ),
                      ),
                    ],
                    if (_error case final error?) ...[
                      const SizedBox(height: 14),
                      _BuildError(message: error),
                    ],
                  ],
                ],
              ],
            ),
          ),
        ),
        actions: [
          TextButton(
            key: const Key('revision3-voice-build-close'),
            onPressed: _busy ? null : () => Navigator.of(context).pop(result),
            child: Text(
              _planning ? widget.copy.cancelLabel : widget.copy.closeLabel,
            ),
          ),
          if (readyForOutput)
            FilledButton.icon(
              key: const Key('revision3-voice-build-submit'),
              onPressed:
                  !_busy &&
                      !_terminal &&
                      _parentDirectory != null &&
                      _validateFolderName(_folderName.text, widget.copy) == null
                  ? _build
                  : null,
              icon: _building
                  ? const SizedBox.square(
                      dimension: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.inventory_2_outlined),
              label: Text(widget.copy.buildOfflineBundleLabel),
            ),
        ],
      ),
    );
  }
}

String? _validateParentDirectory(
  String value,
  Revision3VoiceBuildDialogCopy copy,
) {
  if (value.isEmpty || !p.isAbsolute(value)) {
    return copy.parentMustBeAbsolute;
  }
  final type = FileSystemEntity.typeSync(value, followLinks: false);
  return switch (type) {
    FileSystemEntityType.directory => null,
    FileSystemEntityType.link => copy.parentSymlink,
    _ => copy.parentMustExist,
  };
}

String? _validateFolderName(String? raw, Revision3VoiceBuildDialogCopy copy) {
  final value = raw ?? '';
  if (value.isEmpty) return copy.folderNameRequired;
  if (value.trim() != value) {
    return copy.folderNameWhitespace;
  }
  if (utf8.encode(value).length > 255) {
    return copy.folderNameTooLong;
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
    return copy.folderNamePortable;
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
    return copy.folderNameWindowsReserved;
  }
  return null;
}

class _OfflineBuildNotice extends StatelessWidget {
  const _OfflineBuildNotice({required this.copy});

  final Revision3VoiceBuildDialogCopy copy;

  @override
  Widget build(BuildContext context) => Container(
    key: const Key('revision3-voice-build-offline-notice'),
    padding: const EdgeInsets.all(12),
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.secondaryContainer,
      borderRadius: BorderRadius.circular(8),
    ),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Icon(Icons.offline_bolt_outlined),
        const SizedBox(width: 10),
        Expanded(child: Text(copy.offlineNotice)),
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

class _VoiceBuildPlanError extends StatelessWidget {
  const _VoiceBuildPlanError({
    required this.message,
    required this.retryLabel,
    required this.retry,
  });

  final String message;
  final String retryLabel;
  final VoidCallback? retry;

  @override
  Widget build(BuildContext context) => Container(
    key: const Key('revision3-voice-build-plan-error'),
    padding: const EdgeInsets.all(12),
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.errorContainer,
      borderRadius: BorderRadius.circular(8),
    ),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(
          Icons.error_outline,
          color: Theme.of(context).colorScheme.onErrorContainer,
        ),
        const SizedBox(width: 10),
        Expanded(
          child: Text(
            message,
            style: TextStyle(
              color: Theme.of(context).colorScheme.onErrorContainer,
            ),
          ),
        ),
        if (retry != null) ...[
          const SizedBox(width: 8),
          TextButton(
            key: const Key('revision3-voice-build-plan-retry'),
            onPressed: retry,
            child: Text(retryLabel),
          ),
        ],
      ],
    ),
  );
}

class _BuiltReceipt extends StatelessWidget {
  const _BuiltReceipt({required this.copy, required this.result});

  final Revision3VoiceBuildDialogCopy copy;
  final AuthoringRevision3VoiceBuildResult result;

  @override
  Widget build(BuildContext context) => Column(
    key: const Key('revision3-voice-build-built'),
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      Text(copy.builtTitle, style: Theme.of(context).textTheme.titleMedium),
      const SizedBox(height: 4),
      Text(copy.offlineReceipt),
      const SizedBox(height: 12),
      _BuildFact(
        label: copy.basisRevisionLabel,
        value: '${result.projectRevision}',
      ),
      _BuildFact(label: copy.outputLabel, value: result.output!),
      _BuildFact(label: copy.archiveEditsLabel, value: '${result.editCount}'),
      _BuildFact(label: copy.bundleFilesLabel, value: '${result.fileCount}'),
      _BuildFact(label: copy.sealedBytesLabel, value: '${result.bundleBytes}'),
      _BuildFact(
        label: copy.bundleSha256Label,
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
