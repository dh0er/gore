import 'dart:async';
import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../domain/story_catalog_adapter.dart';
import '../domain/story_draft_requests.dart';
import '../domain/story_workspace_bootstrap.dart';
import '../domain/story_workspace_controller.dart';
import '../domain/story_workspace_launcher.dart';
import 'story_workspace_view.dart';

enum StoryWorkspaceFlowMode { create, open }

typedef StoryWorkspaceDirectoryPicker = Future<String?> Function();
typedef StoryWorkspaceMetadataPrompt =
    Future<StoryProjectMetadata?> Function(BuildContext context);

/// UI-sized lease over the managed launch handle. Tests can supply a fake
/// without constructing trusted catalog/session DTOs.
abstract interface class StoryWorkspaceFlowSession {
  StoryWorkspaceState get state;
  StoryCatalogAdapter get catalog;
  Future<StoryDraftCreateResult> createNpc(StoryNpcDraftInput input);
  Future<void> close();
}

abstract interface class StoryWorkspaceFlowLauncher {
  Future<StoryWorkspaceFlowSession> create({
    required String configuredGamePath,
    required Directory workspaceRoot,
    required StoryProjectMetadata metadata,
  });

  Future<StoryWorkspaceFlowSession> open({
    required String configuredGamePath,
    required Directory workspaceRoot,
  });
}

/// Production adapter. Every launch still crosses [StoryWorkspaceLauncher];
/// the UI never constructs catalog selections or managed sessions itself.
final class ManagedStoryWorkspaceFlowLauncher
    implements StoryWorkspaceFlowLauncher {
  const ManagedStoryWorkspaceFlowLauncher(this.launcher);

  final StoryWorkspaceLauncher launcher;

  @override
  Future<StoryWorkspaceFlowSession> create({
    required String configuredGamePath,
    required Directory workspaceRoot,
    required StoryProjectMetadata metadata,
  }) async => _ManagedStoryWorkspaceFlowSession(
    await launcher.create(
      configuredGamePath: configuredGamePath,
      workspaceRoot: workspaceRoot,
      metadata: metadata,
    ),
  );

  @override
  Future<StoryWorkspaceFlowSession> open({
    required String configuredGamePath,
    required Directory workspaceRoot,
  }) async => _ManagedStoryWorkspaceFlowSession(
    await launcher.open(
      configuredGamePath: configuredGamePath,
      workspaceRoot: workspaceRoot,
    ),
  );
}

final class _ManagedStoryWorkspaceFlowSession
    implements StoryWorkspaceFlowSession {
  const _ManagedStoryWorkspaceFlowSession(this.launch);

  final StoryWorkspaceLaunch launch;

  @override
  StoryWorkspaceState get state => launch.workspace.controller.current;

  @override
  StoryCatalogAdapter get catalog => launch.workspace.adapter;

  @override
  Future<StoryDraftCreateResult> createNpc(StoryNpcDraftInput input) =>
      launch.workspace.controller.createNpc(input);

  @override
  Future<void> close() => launch.close();
}

/// Picks one existing managed directory, optionally gathers creation
/// metadata, then opens the full-screen draft-only workspace route.
Future<void> runStoryWorkspaceFlow({
  required BuildContext context,
  required StoryWorkspaceFlowMode mode,
  required String configuredGamePath,
  required StoryWorkspaceFlowLauncher launcher,
  StoryWorkspaceDirectoryPicker? pickDirectory,
  StoryWorkspaceMetadataPrompt? promptMetadata,
}) async {
  final String? selectedPath;
  try {
    selectedPath = await (pickDirectory ?? _pickWorkspaceDirectory)();
  } catch (_) {
    if (context.mounted) {
      await _showStableFlowError(
        context,
        'The workspace directory could not be selected. Please try again.',
      );
    }
    return;
  }
  if (!context.mounted || selectedPath == null) return;
  final workspacePath = selectedPath;

  StoryProjectMetadata? metadata;
  if (mode == StoryWorkspaceFlowMode.create) {
    metadata = await (promptMetadata ?? showStoryProjectMetadataDialog)(
      context,
    );
    if (!context.mounted || metadata == null) return;
  }

  await Navigator.of(context).push<void>(
    MaterialPageRoute<void>(
      fullscreenDialog: true,
      builder: (_) => StoryWorkspaceFlowPage(
        mode: mode,
        configuredGamePath: configuredGamePath,
        workspaceRoot: Directory(workspacePath),
        launcher: launcher,
        metadata: metadata,
      ),
    ),
  );
}

Future<String?> _pickWorkspaceDirectory() =>
    getDirectoryPath(confirmButtonText: 'Select workspace');

Future<void> _showStableFlowError(BuildContext context, String message) =>
    showDialog<void>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('Story workspace'),
        content: Text(message),
        actions: <Widget>[
          FilledButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('OK'),
          ),
        ],
      ),
    );

Future<StoryProjectMetadata?> showStoryProjectMetadataDialog(
  BuildContext context,
) => showDialog<StoryProjectMetadata>(
  context: context,
  builder: (_) => const _StoryProjectMetadataDialog(),
);

final class _StoryProjectMetadataDialog extends StatefulWidget {
  const _StoryProjectMetadataDialog();

  @override
  State<_StoryProjectMetadataDialog> createState() =>
      _StoryProjectMetadataDialogState();
}

final class _StoryProjectMetadataDialogState
    extends State<_StoryProjectMetadataDialog> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _name;
  late final TextEditingController _version;
  late final TextEditingController _author;

  @override
  void initState() {
    super.initState();
    _name = TextEditingController();
    _version = TextEditingController(text: '0.1.0');
    _author = TextEditingController();
  }

  @override
  void dispose() {
    _name.dispose();
    _version.dispose();
    _author.dispose();
    super.dispose();
  }

  void _submit() {
    if (!_formKey.currentState!.validate()) return;
    try {
      Navigator.of(context).pop(
        StoryProjectMetadata(
          name: _name.text.trim(),
          version: _version.text.trim(),
          author: _author.text.trim(),
        ),
      );
    } on FormatException {
      // Field validators mirror the bounded metadata contract. Keep this
      // defensive fallback friendly if that contract tightens later.
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Review the project details and try again.'),
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: const Text('Create Story workspace'),
    content: SizedBox(
      width: 440,
      child: Form(
        key: _formKey,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            TextFormField(
              key: const Key('story-project-name-field'),
              controller: _name,
              autofocus: true,
              textInputAction: TextInputAction.next,
              decoration: const InputDecoration(
                labelText: 'Mod name',
                border: OutlineInputBorder(),
              ),
              validator: (value) =>
                  _boundedMetadataError(value, required: true, maxBytes: 256),
            ),
            const SizedBox(height: 16),
            TextFormField(
              key: const Key('story-project-version-field'),
              controller: _version,
              textInputAction: TextInputAction.next,
              decoration: const InputDecoration(
                labelText: 'Version',
                hintText: '0.1.0',
                border: OutlineInputBorder(),
              ),
              validator: (value) =>
                  _boundedMetadataError(value, required: false, maxBytes: 128),
            ),
            const SizedBox(height: 16),
            TextFormField(
              key: const Key('story-project-author-field'),
              controller: _author,
              textInputAction: TextInputAction.done,
              onFieldSubmitted: (_) => _submit(),
              decoration: const InputDecoration(
                labelText: 'Author (optional)',
                border: OutlineInputBorder(),
              ),
              validator: (value) =>
                  _boundedMetadataError(value, required: false, maxBytes: 256),
            ),
          ],
        ),
      ),
    ),
    actions: <Widget>[
      TextButton(
        onPressed: () => Navigator.of(context).pop(),
        child: const Text('Cancel'),
      ),
      FilledButton(
        key: const Key('story-project-create-button'),
        onPressed: _submit,
        child: const Text('Create workspace'),
      ),
    ],
  );
}

String? _boundedMetadataError(
  String? raw, {
  required bool required,
  required int maxBytes,
}) {
  final value = raw?.trim() ?? '';
  if (required && value.isEmpty) return 'Enter a mod name.';
  var bytes = 0;
  for (final rune in value.runes) {
    if (rune < 0x20 || (rune >= 0x7f && rune <= 0x9f)) {
      return 'Remove control characters.';
    }
    bytes += rune <= 0x7f
        ? 1
        : rune <= 0x7ff
        ? 2
        : rune <= 0xffff
        ? 3
        : 4;
    if (bytes > maxBytes) return 'Shorten this value.';
  }
  return null;
}

final class StoryWorkspaceFlowPage extends StatefulWidget {
  const StoryWorkspaceFlowPage({
    required this.mode,
    required this.configuredGamePath,
    required this.workspaceRoot,
    required this.launcher,
    this.metadata,
    super.key,
  }) : assert(
         mode == StoryWorkspaceFlowMode.open || metadata != null,
         'create mode requires metadata',
       );

  final StoryWorkspaceFlowMode mode;
  final String configuredGamePath;
  final Directory workspaceRoot;
  final StoryWorkspaceFlowLauncher launcher;
  final StoryProjectMetadata? metadata;

  @override
  State<StoryWorkspaceFlowPage> createState() => _StoryWorkspaceFlowPageState();
}

final class _StoryWorkspaceFlowPageState extends State<StoryWorkspaceFlowPage> {
  StoryWorkspaceFlowSession? _session;
  String? _error;
  bool _loading = true;
  bool _closing = false;
  bool _starting = false;
  int _attempt = 0;
  Future<void>? _startFuture;

  @override
  void initState() {
    super.initState();
    unawaited(_start());
  }

  @override
  void dispose() {
    _attempt++;
    final session = _session;
    _session = null;
    if (session != null) unawaited(_closeQuietly(session));
    super.dispose();
  }

  Future<void> _start() {
    final inFlight = _startFuture;
    if (_starting || _closing) return inFlight ?? Future<void>.value();
    _starting = true;
    final operation = _runStart(++_attempt);
    _startFuture = operation;
    unawaited(
      operation.whenComplete(() {
        if (identical(_startFuture, operation)) {
          _startFuture = null;
          _starting = false;
        }
      }),
    );
    return operation;
  }

  Future<void> _runStart(int attempt) async {
    if (mounted) {
      setState(() {
        _loading = true;
        _error = null;
      });
    }
    StoryWorkspaceFlowSession? acquired;
    try {
      acquired = switch (widget.mode) {
        StoryWorkspaceFlowMode.create => await widget.launcher.create(
          configuredGamePath: widget.configuredGamePath,
          workspaceRoot: widget.workspaceRoot,
          metadata: widget.metadata!,
        ),
        StoryWorkspaceFlowMode.open => await widget.launcher.open(
          configuredGamePath: widget.configuredGamePath,
          workspaceRoot: widget.workspaceRoot,
        ),
      };
      if (!mounted || attempt != _attempt) {
        await _closeQuietly(acquired);
        return;
      }
      setState(() {
        _session = acquired;
        _loading = false;
      });
    } catch (error) {
      if (acquired != null) await _closeQuietly(acquired);
      if (!mounted || attempt != _attempt) return;
      setState(() {
        _loading = false;
        _error = _friendlyLaunchError(error);
      });
    }
  }

  Future<void> _closeAndPop() async {
    if (_closing) return;
    setState(() => _closing = true);
    final pendingStart = _startFuture;
    _attempt++;
    final session = _session;
    _session = null;
    if (session != null) await _closeQuietly(session);
    // A pending launch owns any handle it may still acquire. Invalidating its
    // attempt makes it close that handle; awaiting it here prevents the route
    // (and runStoryWorkspaceFlow) from returning while a hidden lock lane is
    // still being cleaned up.
    if (pendingStart != null) await pendingStart;
    if (mounted) Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) => PopScope<void>(
    canPop: false,
    onPopInvokedWithResult: (didPop, _) {
      if (!didPop) unawaited(_closeAndPop());
    },
    child: Scaffold(
      appBar: AppBar(
        leading: IconButton(
          key: const Key('story-workspace-back'),
          tooltip: 'Back',
          onPressed: _closing ? null : _closeAndPop,
          icon: const Icon(Icons.arrow_back),
        ),
        title: const Text('Story workspace (drafts)'),
      ),
      body: _buildBody(),
    ),
  );

  Widget _buildBody() {
    if (_loading || _closing) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            const CircularProgressIndicator(),
            const SizedBox(height: 16),
            Text(_closing ? 'Closing workspace...' : 'Opening workspace...'),
          ],
        ),
      );
    }
    final error = _error;
    if (error != null) {
      return Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 520),
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                Icon(
                  Icons.error_outline,
                  size: 44,
                  color: Theme.of(context).colorScheme.error,
                ),
                const SizedBox(height: 12),
                Text(
                  'Could not open Story workspace',
                  style: Theme.of(context).textTheme.titleLarge,
                ),
                const SizedBox(height: 8),
                Text(
                  error,
                  key: const Key('story-workspace-flow-error'),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 18),
                FilledButton.icon(
                  key: const Key('story-workspace-retry'),
                  onPressed: _starting || _closing ? null : _start,
                  icon: const Icon(Icons.refresh),
                  label: const Text('Try again'),
                ),
              ],
            ),
          ),
        ),
      );
    }
    final session = _session!;
    return StoryWorkspaceView(
      initialState: session.state,
      catalog: session.catalog,
      createNpc: session.createNpc,
    );
  }
}

Future<void> _closeQuietly(StoryWorkspaceFlowSession session) async {
  try {
    await session.close();
  } catch (_) {}
}

String _friendlyLaunchError(Object error) {
  if (error is StoryWorkspaceLaunchException) {
    return switch (error.code) {
      StoryWorkspaceLaunchError.invalidConfiguredGame ||
      StoryWorkspaceLaunchError.missingExecutable =>
        'The configured game installation is incomplete. Check the game path in Settings, then try again.',
      StoryWorkspaceLaunchError.ambiguousGameRoot =>
        'The configured folder contains an ambiguous game layout. Select the exact install root in Settings.',
      StoryWorkspaceLaunchError.unsafeFileType ||
      StoryWorkspaceLaunchError.pathInspectionFailed =>
        'The selected game or workspace path could not be verified safely.',
      StoryWorkspaceLaunchError.invalidWorkspace =>
        'The selected workspace must be an existing safe directory.',
      StoryWorkspaceLaunchError.catalogBuildFailed =>
        'The Story catalog could not be read from this game installation.',
      StoryWorkspaceLaunchError.workspaceBootstrapFailed =>
        'The managed workspace could not be created or opened. It may already be open or need recovery.',
    };
  }
  return 'Something went wrong while opening the Story workspace. Please try again.';
}
