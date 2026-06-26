import 'package:file_selector/file_selector.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';

import '../audio/domain/audio_replacements_notifier.dart';
import '../editor/domain/overrides_notifier.dart';
import '../loc/domain/loc_edits_notifier.dart';
import 'project_io.dart';
import 'project_model.dart';

/// The current mod name (also the bundle/project file name). Shared by the
/// build/deploy dialog and project save.
final modNameProvider = StateProvider<String>((ref) => 'MyMod');

/// Mod metadata carried through load → gather (save/build), so opening a project and saving or
/// deploying it preserves the version, author, and UE4SS load delay instead of resetting them.
final modVersionProvider = StateProvider<String>((ref) => '');
final modAuthorProvider = StateProvider<String>((ref) => '');
final modDelayMsProvider = StateProvider<int>((ref) => 0);

/// Whether any editor domain has pending content to build/save.
bool projectIsDirty(WidgetRef ref) =>
    ref.watch(overridesProvider).count > 0 ||
    ref.watch(locEditsProvider).isDirty ||
    ref.watch(audioReplacementsProvider).count > 0;

/// Snapshot all editor state into a [ModProject].
ModProject gatherProject(WidgetRef ref) {
  final name = ref.read(modNameProvider).trim();
  return ModProject(
    name: name.isEmpty ? 'MyMod' : name,
    version: ref.read(modVersionProvider),
    author: ref.read(modAuthorProvider),
    delayMs: ref.read(modDelayMsProvider),
    overrides: ref.read(overridesProvider).entries,
    locEdits: ref.read(locEditsProvider).edits,
    audio: ref.read(audioReplacementsProvider).entries,
  );
}

/// Replace all editor state from a loaded [ModProject].
void applyProject(WidgetRef ref, ModProject project) {
  ref.read(modNameProvider.notifier).state = project.name;
  ref.read(modVersionProvider.notifier).state = project.version;
  ref.read(modAuthorProvider.notifier).state = project.author;
  ref.read(modDelayMsProvider.notifier).state = project.delayMs;
  final overrides = ref.read(overridesProvider.notifier)..clearAll();
  for (final o in project.overrides) {
    overrides.setOverride(o);
  }
  ref.read(locEditsProvider.notifier).loadAll(project.locEdits);
  ref.read(audioReplacementsProvider.notifier).loadAll(project.audio);
}

/// Clear all editor state (New project).
void newProject(WidgetRef ref) {
  ref.read(modNameProvider.notifier).state = 'MyMod';
  ref.read(modVersionProvider.notifier).state = '';
  ref.read(modAuthorProvider.notifier).state = '';
  ref.read(modDelayMsProvider.notifier).state = 0;
  ref.read(overridesProvider.notifier).clearAll();
  ref.read(locEditsProvider.notifier).clearAll();
  ref.read(audioReplacementsProvider.notifier).clearAll();
}

/// Prompt for a path and save the current project. Returns the path, or null if cancelled.
Future<String?> saveProjectInteractive(WidgetRef ref) async {
  final name = ref.read(modNameProvider).trim();
  final location = await getSaveLocation(
    suggestedName: '${name.isEmpty ? 'mod' : name}$kProjectExtension',
    acceptedTypeGroups: const [
      XTypeGroup(label: 'gore-mod project', extensions: ['goremod']),
    ],
  );
  if (location == null) return null;
  await saveProject(gatherProject(ref), location.path);
  return location.path;
}

/// Prompt for a project file and load it into the editor. Returns the path, or null.
Future<String?> openProjectInteractive(WidgetRef ref) async {
  final file = await openFile(
    acceptedTypeGroups: const [
      XTypeGroup(label: 'gore-mod project', extensions: ['goremod']),
    ],
  );
  if (file == null) return null;
  final project = await loadProject(file.path);
  applyProject(ref, project);
  return file.path;
}
