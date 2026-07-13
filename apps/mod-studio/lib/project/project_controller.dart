import 'dart:async';
import 'dart:convert';

import 'package:file_selector/file_selector.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:flutter_riverpod/misc.dart' show ProviderListenable;

import '../audio/domain/audio_replacements_notifier.dart';
import '../editor/domain/overrides_notifier.dart';
import '../loc/domain/loc_edits_notifier.dart';
import '../scripts/domain/script_mods_notifier.dart';
import '../textures/domain/texture_replacements_notifier.dart';
import '../voice/domain/voice_edits_notifier.dart';
import 'dialog_topics_notifier.dart';
import 'project_io.dart';
import 'project_model.dart';

/// The current mod name (also the bundle/project file name). Shared by the
/// build/deploy dialog and project save.
final modNameProvider = StateProvider<String>((ref) => 'MyMod');

/// Mod metadata carried through load -> gather (save/build), so opening a project and saving or
/// deploying it preserves the version, author, and UE4SS load delay instead of resetting them.
final modVersionProvider = StateProvider<String>((ref) => '');
final modAuthorProvider = StateProvider<String>((ref) => '');
final modDelayMsProvider = StateProvider<int>((ref) => 0);

/// The file adopted by the current project session, or null for an untitled
/// project. A successful Save As/Open changes it; a failed operation does not.
final currentProjectPathProvider = StateProvider<String?>((ref) => null);

/// Signature of the last state written to / loaded from a `.goremod`. Used to tell whether the
/// current staged state still matches what was saved, so a saved project isn't treated as having
/// unsaved changes. Null until the first save/open/new in this session.
final savedProjectSignatureProvider = StateProvider<String?>((ref) => null);

typedef ProjectSaver = Future<void> Function(ModProject project, String path);
typedef ProjectLoader = Future<LoadedProject> Function(String path);

/// Injectable disk boundaries for deterministic session tests.
final projectSaverProvider = Provider<ProjectSaver>((ref) => saveProject);
final projectLoaderProvider = Provider<ProjectLoader>((ref) => loadProject);

typedef _ProviderReader = T Function<T>(ProviderListenable<T> provider);

/// Whether any editor domain has pending content to build/save.
bool projectIsDirty(WidgetRef ref) =>
    ref.watch(overridesProvider).count > 0 ||
    ref.watch(locEditsProvider).isDirty ||
    ref.watch(audioReplacementsProvider).count > 0 ||
    ref.watch(textureReplacementsProvider).count > 0 ||
    ref.watch(scriptModsProvider).count > 0 ||
    ref.watch(dialogTopicsProvider).count > 0 ||
    ref.watch(voiceEditsProvider).count > 0;

String _signatureOf(ModProject project) => jsonEncode(project.toJson());
final String _freshProjectSignature = _signatureOf(ModProject(name: 'MyMod'));

/// A stable signature of the full current project (name + metadata + all editor domains).
String _projectSignature(_ProviderReader read) =>
    _signatureOf(_gatherProject(read));

/// Record the current state as the saved baseline.
///
/// The session controller deliberately does not call this after an asynchronous
/// save: it records the signature captured before I/O instead, so edits made
/// while the write is running remain dirty.
void markProjectSaved(WidgetRef ref) =>
    ref.read(savedProjectSignatureProvider.notifier).state = _projectSignature(
      ref.read,
    );

/// Whether there are staged changes NOT yet written to a project file. Once a baseline exists
/// (after a save/open/new), this is purely current-vs-saved - so even CLEARING a loaded project
/// (which leaves no content but differs from the loaded signature) counts as unsaved. Before any
/// baseline (a fresh session), an empty editor is clean and any staged content is unsaved.
bool hasUnsavedChanges(WidgetRef ref) => _hasUnsavedChanges(ref.read);

bool _hasUnsavedChanges(_ProviderReader read) {
  final saved = read(savedProjectSignatureProvider);
  if (saved == null) {
    return _projectSignature(read) != _freshProjectSignature;
  }
  return _projectSignature(read) != saved;
}

/// Snapshot all editor state into a [ModProject].
ModProject gatherProject(WidgetRef ref) => _gatherProject(ref.read);

ModProject _gatherProject(_ProviderReader read) {
  final name = read(modNameProvider).trim();
  return ModProject(
    name: name.isEmpty ? 'MyMod' : name,
    version: read(modVersionProvider),
    author: read(modAuthorProvider),
    delayMs: read(modDelayMsProvider),
    overrides: read(overridesProvider).entries,
    locEdits: read(locEditsProvider).edits,
    audio: read(audioReplacementsProvider).entries,
    textures: read(textureReplacementsProvider).entries,
    scripts: read(scriptModsProvider).entries,
    dialogTopics: read(dialogTopicsProvider).entries,
    voice: read(voiceEditsProvider).entries,
  );
}

ModProject _captureProject(_ProviderReader read) {
  final project = _gatherProject(read);
  project.validateUniqueTargets();
  return ModProject(
    name: project.name,
    version: project.version,
    author: project.author,
    delayMs: project.delayMs,
    overrides: List.unmodifiable(project.overrides),
    locEdits: Map<String, Map<String, String>>.unmodifiable({
      for (final entry in project.locEdits.entries)
        entry.key: Map<String, String>.unmodifiable(entry.value),
    }),
    audio: List.unmodifiable(project.audio),
    textures: List.unmodifiable(project.textures),
    scripts: List.unmodifiable(project.scripts),
    dialogTopics: List.unmodifiable(project.dialogTopics),
    voice: List.unmodifiable(project.voice),
  );
}

/// Replace all editor state from a loaded [ModProject].
void applyProject(WidgetRef ref, ModProject project) =>
    _applyProject(ref.read, project);

void _applyProject(_ProviderReader read, ModProject project) {
  // Validate every keyed domain before replacing any provider, so an external
  // duplicate cannot be silently collapsed or produce a half-applied project.
  project.validateUniqueTargets();
  read(dialogTopicsProvider.notifier).loadAll(project.dialogTopics);
  read(modNameProvider.notifier).state = project.name;
  read(modVersionProvider.notifier).state = project.version;
  read(modAuthorProvider.notifier).state = project.author;
  read(modDelayMsProvider.notifier).state = project.delayMs;
  final overrides = read(overridesProvider.notifier)..clearAll();
  for (final override in project.overrides) {
    overrides.setOverride(override);
  }
  read(locEditsProvider.notifier).loadAll(project.locEdits);
  read(audioReplacementsProvider.notifier).loadAll(project.audio);
  read(textureReplacementsProvider.notifier).loadAll(project.textures);
  read(scriptModsProvider.notifier).loadAll(project.scripts);
  read(voiceEditsProvider.notifier).loadAll(project.voice);
}

void _clearProject(_ProviderReader read) {
  read(modNameProvider.notifier).state = 'MyMod';
  read(modVersionProvider.notifier).state = '';
  read(modAuthorProvider.notifier).state = '';
  read(modDelayMsProvider.notifier).state = 0;
  read(overridesProvider.notifier).clearAll();
  read(locEditsProvider.notifier).clearAll();
  read(audioReplacementsProvider.notifier).clearAll();
  read(textureReplacementsProvider.notifier).clearAll();
  read(scriptModsProvider.notifier).clearAll();
  read(dialogTopicsProvider.notifier).clearAll();
  read(voiceEditsProvider.notifier).clearAll();
}

/// Owns one ProviderScope's project path, extracted workspace, and ordered I/O
/// lane. No two Save/Open/New operations in the same scope run concurrently.
class ProjectSessionController {
  ProjectSessionController({
    required this._ref,
    required this._saver,
    required this._loader,
  });

  final Ref _ref;
  final ProjectSaver _saver;
  final ProjectLoader _loader;

  Future<void> _tail = Future<void>.value();
  ProjectWorkspaceLease? _workspace;
  final List<ProjectWorkspaceLease> _retiredWorkspaces = [];
  bool _disposed = false;

  String? get currentPath => _ref.read(currentProjectPathProvider);
  bool get hasUnsavedChanges => _hasUnsavedChanges(_ref.read);

  /// Workspaces whose safety-checked release failed after their project had
  /// already been replaced. They remain visible for diagnostics until scope
  /// disposal; a cleanup failure never rolls back or fails the adopted state.
  List<String> get retainedWorkspacePaths =>
      List.unmodifiable(_retiredWorkspaces.map((workspace) => workspace.path));

  /// Save a stable snapshot and adopt [path] only after the write succeeds.
  Future<String> saveToPath(String path) =>
      _enqueue(() => _saveCapturedTo(path));

  Future<String> _saveCapturedTo(String path) async {
    final captured = _captureProject(_ref.read);
    final capturedSignature = _signatureOf(captured);
    await _saver(captured, path);
    _ref.read(currentProjectPathProvider.notifier).state = path;
    _ref.read(savedProjectSignatureProvider.notifier).state = capturedSignature;
    return path;
  }

  /// Save to the path adopted by the current session.
  Future<String> saveToCurrentPath() => _enqueue(() {
    final path = currentPath;
    if (path == null) {
      throw StateError('the current project has no path; use Save As');
    }
    return _saveCapturedTo(path);
  });

  /// Like [saveToCurrentPath], but returns null for an untitled project. The
  /// interactive UI uses this ordered check before showing the Save As picker.
  Future<String?> saveToCurrentPathIfAny() => _enqueue(() {
    final path = currentPath;
    if (path == null) return Future<String?>.value();
    return _saveCapturedTo(path);
  });

  /// Fully load and apply a candidate before adopting its path/workspace.
  Future<String> openFromPath(String path) => _enqueue(() async {
    LoadedProject? candidate;
    late String appliedSignature;
    try {
      candidate = await _loader(path);
      candidate.project.validateUniqueTargets();
      _applyProject(_ref.read, candidate.project);
      appliedSignature = _projectSignature(_ref.read);
    } catch (error, stackTrace) {
      try {
        await candidate?.workspace?.release();
      } catch (_) {
        // Preserve the Open error; the lease refuses unsafe deletion and leaves
        // evidence in place if cleanup itself cannot be proven safe.
      }
      Error.throwWithStackTrace(error, stackTrace);
    }

    // Candidate ownership transfers to the session before old-workspace
    // cleanup. If that cleanup refuses an unsafe delete, the newly adopted
    // project must keep its own workspace alive.
    final previousWorkspace = _workspace;
    _workspace = candidate.workspace;
    _ref.read(currentProjectPathProvider.notifier).state = path;
    _ref.read(savedProjectSignatureProvider.notifier).state = appliedSignature;
    await _releaseRetired(previousWorkspace);
    return path;
  });

  /// Start a clean untitled project and release the previously adopted source
  /// workspace after the provider swap succeeds.
  Future<void> newProject() => _enqueue(() async {
    _clearProject(_ref.read);
    final cleanSignature = _projectSignature(_ref.read);
    final previousWorkspace = _workspace;
    _workspace = null;
    _ref.read(currentProjectPathProvider.notifier).state = null;
    _ref.read(savedProjectSignatureProvider.notifier).state = cleanSignature;
    await _releaseRetired(previousWorkspace);
  });

  Future<void> _releaseRetired(ProjectWorkspaceLease? workspace) async {
    if (workspace == null) return;
    try {
      await workspace.release();
    } catch (_) {
      if (!_retiredWorkspaces.contains(workspace)) {
        _retiredWorkspaces.add(workspace);
      }
    }
  }

  Future<T> _enqueue<T>(Future<T> Function() operation) {
    if (_disposed) {
      return Future<T>.error(StateError('project session is disposed'));
    }
    final result = _tail.then((_) => operation());
    _tail = result.then<void>((_) {}, onError: (Object _, StackTrace _) {});
    return result;
  }

  Future<void> _dispose() {
    if (_disposed) return _tail;
    _disposed = true;
    final result = _tail.then((_) async {
      final workspaces = <ProjectWorkspaceLease>[
        ?_workspace,
        ..._retiredWorkspaces,
      ];
      _workspace = null;
      _retiredWorkspaces.clear();
      for (final workspace in workspaces) {
        try {
          await workspace.release();
        } catch (_) {
          // Scope disposal is best-effort. A lease that cannot prove a safe
          // deletion intentionally preserves its root for diagnostics.
        }
      }
    });
    _tail = result.then<void>((_) {}, onError: (Object _, StackTrace _) {});
    return result;
  }
}

/// One controller is retained per ProviderScope, which makes the async lane and
/// workspace ownership local to that editor session.
final projectSessionProvider = Provider<ProjectSessionController>((ref) {
  final controller = ProjectSessionController(
    ref: ref,
    saver: ref.read(projectSaverProvider),
    loader: ref.read(projectLoaderProvider),
  );
  ref.onDispose(() => unawaited(controller._dispose()));
  return controller;
});

/// Clear all editor state (New project), serialized with Save/Open.
Future<void> newProject(WidgetRef ref) =>
    ref.read(projectSessionProvider).newProject();

/// Save to [path] without showing a picker. Primarily useful for tests and
/// non-interactive integrations.
Future<String> saveProjectToPath(WidgetRef ref, String path) =>
    ref.read(projectSessionProvider).saveToPath(path);

/// Save to the path adopted by the current project session.
Future<String> saveProjectToCurrentPath(WidgetRef ref) =>
    ref.read(projectSessionProvider).saveToCurrentPath();

/// Open [path] without showing a picker.
Future<String> openProjectFromPath(WidgetRef ref, String path) =>
    ref.read(projectSessionProvider).openFromPath(path);

/// Save to the current path, or prompt for Save As when the project is
/// untitled. Returns the path, or null if the picker was cancelled.
Future<String?> saveProjectInteractive(WidgetRef ref) async {
  final saved = await ref.read(projectSessionProvider).saveToCurrentPathIfAny();
  if (saved != null) return saved;
  return saveProjectAsInteractive(ref);
}

/// Pick a compatibility-project destination without performing any I/O.
///
/// The app-wide current-project coordinator uses this boundary so the actual
/// save remains inside its serialized Legacy/R3 operation lane.
Future<String?> pickProjectSavePath(WidgetRef ref) async {
  final name = ref.read(modNameProvider).trim();
  final location = await getSaveLocation(
    suggestedName: '${name.isEmpty ? 'mod' : name}$kProjectExtension',
    acceptedTypeGroups: const [
      XTypeGroup(label: 'gore-mod project', extensions: ['goremod']),
    ],
  );
  return location?.path;
}

/// Always prompt for a new path and save there.
Future<String?> saveProjectAsInteractive(WidgetRef ref) async {
  final path = await pickProjectSavePath(ref);
  if (path == null) return null;
  return ref.read(projectSessionProvider).saveToPath(path);
}

/// Pick a compatibility project archive without loading it.
///
/// Keeping selection separate lets the global coordinator validate and adopt
/// the candidate within the same cross-format lane.
Future<String?> pickProjectOpenPath() async {
  final file = await openFile(
    acceptedTypeGroups: const [
      XTypeGroup(label: 'gore-mod project', extensions: ['goremod']),
    ],
  );
  return file?.path;
}

/// Prompt for a project file and load it into the editor. Returns the path, or null.
Future<String?> openProjectInteractive(WidgetRef ref) async {
  final path = await pickProjectOpenPath();
  if (path == null) return null;
  return ref.read(projectSessionProvider).openFromPath(path);
}
