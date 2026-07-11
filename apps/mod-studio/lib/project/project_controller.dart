import 'dart:convert';

import 'package:file_selector/file_selector.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';

import '../audio/domain/audio_replacements_notifier.dart';
import '../editor/domain/overrides_notifier.dart';
import '../loc/domain/loc_edits_notifier.dart';
import '../scripts/domain/script_mods_notifier.dart';
import '../textures/domain/texture_replacements_notifier.dart';
import 'dialog_topics_notifier.dart';
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
    ref.watch(audioReplacementsProvider).count > 0 ||
    ref.watch(textureReplacementsProvider).count > 0 ||
    ref.watch(scriptModsProvider).count > 0 ||
    ref.watch(dialogTopicsProvider).count > 0;

/// Signature of the last state written to / loaded from a `.goremod`. Used to tell whether the
/// current staged state still matches what was saved, so a saved project isn't treated as having
/// unsaved changes. Null until the first save/open/new in this session.
final savedProjectSignatureProvider = StateProvider<String?>((ref) => null);

/// A stable signature of the full current project (name + metadata + all editor domains).
String _projectSignature(WidgetRef ref) =>
    jsonEncode(gatherProject(ref).toJson());

/// Record the current state as the saved baseline (after a save/open/new).
void markProjectSaved(WidgetRef ref) =>
    ref.read(savedProjectSignatureProvider.notifier).state = _projectSignature(
      ref,
    );

/// Whether there are staged changes NOT yet written to a project file. Once a baseline exists
/// (after a save/open/new), this is purely current-vs-saved — so even CLEARING a loaded project
/// (which leaves no content but differs from the loaded signature) counts as unsaved. Before any
/// baseline (a fresh session), an empty editor is clean and any staged content is unsaved.
bool hasUnsavedChanges(WidgetRef ref) {
  final saved = ref.read(savedProjectSignatureProvider);
  if (saved == null) {
    return ref.read(overridesProvider).count > 0 ||
        ref.read(locEditsProvider).isDirty ||
        ref.read(audioReplacementsProvider).count > 0 ||
        ref.read(textureReplacementsProvider).count > 0 ||
        ref.read(scriptModsProvider).count > 0 ||
        ref.read(dialogTopicsProvider).count > 0;
  }
  return _projectSignature(ref) != saved;
}

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
    textures: ref.read(textureReplacementsProvider).entries,
    scripts: ref.read(scriptModsProvider).entries,
    dialogTopics: ref.read(dialogTopicsProvider).entries,
  );
}

/// Replace all editor state from a loaded [ModProject].
void applyProject(WidgetRef ref, ModProject project) {
  // Validate/load the keyed dialog domain first. A malformed external project
  // with duplicate case-insensitive IDs must fail before any other editor
  // domain is replaced, rather than being silently collapsed or half-applied.
  ref.read(dialogTopicsProvider.notifier).loadAll(project.dialogTopics);
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
  ref.read(textureReplacementsProvider.notifier).loadAll(project.textures);
  ref.read(scriptModsProvider.notifier).loadAll(project.scripts);
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
  ref.read(textureReplacementsProvider.notifier).clearAll();
  ref.read(scriptModsProvider.notifier).clearAll();
  ref.read(dialogTopicsProvider.notifier).clearAll();
  markProjectSaved(
    ref,
  ); // a fresh project is in a clean (nothing-unsaved) state
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
  markProjectSaved(ref); // current state now matches the file on disk
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
  markProjectSaved(ref); // freshly loaded state matches the file on disk
  return file.path;
}
