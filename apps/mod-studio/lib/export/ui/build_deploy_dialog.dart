import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../app/domain/ui_settings.dart';
import '../../app/game_paths.dart';
import '../../audio/domain/audio_replacements_notifier.dart';
import '../../core/mod_ffi.dart';
import '../../core/providers.dart';
import '../../editor/domain/overrides_notifier.dart';
import '../../loc/domain/loc_edits_notifier.dart';
import '../../project/project_controller.dart';

/// Build the unified mod bundle (overrides + loc + audio) and optionally deploy it to the
/// game install. Mirrors the loc/audio delivery model: loc + audio are applied to the user's
/// own pristine game files at deploy with `*.gore-bak` backups.
class BuildDeployDialog extends ConsumerStatefulWidget {
  const BuildDeployDialog({super.key});

  @override
  ConsumerState<BuildDeployDialog> createState() => _BuildDeployDialogState();
}

class _BuildDeployDialogState extends ConsumerState<BuildDeployDialog> {
  bool _busy = false;
  String? _status;
  bool _error = false;

  ModFfi get _ffi => ModFfi(ref.read(coreServiceProvider));

  Future<void> _run(Future<void> Function() action) async {
    setState(() {
      _busy = true;
      _status = null;
      _error = false;
    });
    try {
      await action();
    } catch (e) {
      if (mounted) setState(() => _error = true);
      _set('$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  void _set(String msg) {
    if (mounted) setState(() => _status = msg);
  }

  Future<void> _buildToFolder() => _run(() async {
        final dir = await getDirectoryPath();
        if (dir == null) return;
        // The folder picker is async; bail if the dialog was dismissed meanwhile so we don't read
        // a disposed WidgetRef or build after the UI is gone.
        if (!mounted) return;
        final spec = gatherProject(ref).toBuildSpec();
        final bundle = await _ffi.modBuild(spec, dir);
        _set('Built bundle:\n$bundle');
      });

  Future<void> _deploy(String gameRoot) => _run(() async {
        final tmp = await Directory.systemTemp.createTemp('goremod_build_');
        try {
          // createTemp is async; if the dialog closed while it ran, don't gather state from a
          // disposed ref or deploy to the game after the UI is gone.
          if (!mounted) return;
          final spec = gatherProject(ref).toBuildSpec();
          final bundle = await _ffi.modBuild(spec, tmp.path);
          await _ffi.modDeploy(bundle, gameRoot);
          _set('Deployed to game. Launch the game to see your changes.');
        } finally {
          // The bundle was deployed (copied into the game); the temp build dir is no longer
          // needed, so don't leave it behind under the system temp directory.
          try {
            await tmp.delete(recursive: true);
          } catch (_) {}
        }
      });

  Future<void> _undeploy(String gameRoot) => _run(() async {
        await _ffi.modUndeploy(gameRoot);
        _set('Undeployed — original game files restored.');
      });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final overrides = ref.watch(overridesProvider).count;
    final locEdits = ref.watch(locEditsProvider).entryCount;
    final audio = ref.watch(audioReplacementsProvider).count;
    final gameRoot = gameRootFromExe(ref.watch(gameExePathProvider));
    // Building/deploying an empty bundle would only retire the active mod, so require content.
    final hasContent = overrides + locEdits + audio > 0;

    return AlertDialog(
      title: const Text('Build & Deploy Mod'),
      content: SizedBox(
        width: 460,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            TextFormField(
              initialValue: ref.read(modNameProvider),
              decoration: const InputDecoration(labelText: 'Mod name', isDense: true),
              onChanged: (v) => ref.read(modNameProvider.notifier).state = v,
            ),
            const SizedBox(height: 8),
            TextFormField(
              initialValue: ref.read(modDelayMsProvider).toString(),
              decoration: const InputDecoration(
                labelText: 'UE4SS load delay (ms)',
                helperText: 'Wait before applying overrides at game start (0 = none)',
                isDense: true,
              ),
              keyboardType: TextInputType.number,
              // Keep the provider in sync so build/deploy uses the chosen delay instead of always
              // defaulting to 0. Blank or non-numeric input falls back to 0.
              onChanged: (v) =>
                  ref.read(modDelayMsProvider.notifier).state = int.tryParse(v.trim()) ?? 0,
            ),
            const SizedBox(height: 12),
            Text('Contents', style: theme.textTheme.labelMedium),
            Text('• $overrides item override(s)'),
            Text('• $locEdits localized text edit(s)'),
            Text('• $audio audio replacement(s)'),
            const SizedBox(height: 12),
            if (gameRoot == null)
              Text(
                'Set the game path in Settings to deploy directly.',
                style: TextStyle(color: theme.colorScheme.error),
              ),
            if (_busy) const Padding(
              padding: EdgeInsets.symmetric(vertical: 8),
              child: LinearProgressIndicator(),
            ),
            if (_status != null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(
                  _status!,
                  style: TextStyle(
                    color: _error ? theme.colorScheme.error : theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: const Text('Close'),
        ),
        if (gameRoot != null)
          TextButton(
            onPressed: _busy ? null : () => _undeploy(gameRoot),
            child: const Text('Undeploy'),
          ),
        OutlinedButton.icon(
          onPressed: (_busy || !hasContent) ? null : _buildToFolder,
          icon: const Icon(Icons.folder_zip_outlined, size: 18),
          label: const Text('Build to folder…'),
        ),
        FilledButton.icon(
          onPressed: (_busy || gameRoot == null || !hasContent) ? null : () => _deploy(gameRoot),
          icon: const Icon(Icons.rocket_launch_outlined, size: 18),
          label: const Text('Deploy to game'),
        ),
      ],
    );
  }
}
