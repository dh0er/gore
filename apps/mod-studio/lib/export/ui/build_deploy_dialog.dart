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
import '../../l10n/app_localizations.dart';
import '../../loc/domain/loc_edits_notifier.dart';
import '../../project/dialog_topics_notifier.dart';
import '../../project/project_controller.dart';
import '../../scripts/domain/script_mods_notifier.dart';
import '../../scripts/domain/script_compile_install_state_provider.dart';
import '../../scripts/ui/script_compile_install_state_banner.dart';
import '../../textures/domain/texture_replacements_notifier.dart';
import '../../voice/domain/voice_edits_notifier.dart';

/// Build the unified mod bundle (overrides + loc + audio + textures + AngelScript) and
/// optionally deploy it to the game install. Mirrors the loc/audio delivery model: loc, audio,
/// and the AngelScript cache are applied to the user's own pristine game files at deploy with
/// `*.gore-bak` backups; textures deploy as an additive `~mods` Zen triplet.
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
      final msg = '$e';
      // gore-mod's deploy owner-guard carries a stable marker; surface it in the user's
      // language instead of the raw FFI error. Every other error stays raw — it holds the
      // actionable detail.
      _set(
        mounted && msg.contains('manager loadout active')
            ? AppLocalizations.of(context).managerDeployActive
            : msg,
      );
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
    final installSafety = ref.read(scriptCompileInstallSafetyProvider.notifier);
    Directory? tmp;
    try {
      final checked = await installSafety.refresh();
      if (!mounted) return;
      if (!checked.liveMutationAllowed) {
        throw StateError(
          'Deploy blocked: close the game or resolve the recovery/inspection warning, then choose Recheck.',
        );
      }
      tmp = await Directory.systemTemp.createTemp('goremod_build_');
      // createTemp is async; if the dialog closed while it ran, don't gather state from a
      // disposed ref or deploy to the game after the UI is gone.
      if (!mounted) return;
      final spec = gatherProject(ref).toBuildSpec();
      final bundle = await _ffi.modBuild(spec, tmp.path);
      await _ffi.modDeploy(bundle, gameRoot);
      _set('Deployed to game. Launch the game to see your changes.');
    } finally {
      // A failed mutation can itself leave recovery evidence. Always re-probe;
      // the controller records probe failures as blocking state without
      // replacing the primary build/deploy error.
      await installSafety.refresh();
      if (tmp != null) {
        try {
          await tmp.delete(recursive: true);
        } catch (_) {}
      }
    }
  });

  Future<void> _undeploy(String gameRoot) => _run(() async {
    final installSafety = ref.read(scriptCompileInstallSafetyProvider.notifier);
    try {
      final undone = await _ffi.modUndeploy(gameRoot);
      _set(
        undone
            ? 'Undeployed — original game files restored.'
            : 'Nothing was deployed — no changes to undo.',
      );
    } finally {
      await installSafety.refresh();
    }
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final overrides = ref.watch(overridesProvider).count;
    final locEdits = ref.watch(locEditsProvider).entryCount;
    final audio = ref.watch(audioReplacementsProvider).count;
    final textures = ref.watch(textureReplacementsProvider).count;
    final scripts = ref.watch(scriptModsProvider).count;
    final dialogTopics = ref.watch(dialogTopicsProvider).count;
    final voiceEdits = ref.watch(voiceEditsProvider);
    final installSafety = ref.watch(scriptCompileInstallSafetyProvider);
    final voice = voiceEdits.count;
    // Adding a new archive member is preserved as authoring data, but that path has not yet
    // passed runtime qualification. Keep it visible as Draft content without presenting Build
    // or Deploy as safe. Replacing an observed member remains supported.
    final hasDraftVoiceAdds = voiceEdits.entries.any(
      (edit) => edit.operation == VoicePatchOperation.add,
    );
    // Block Build/Deploy while any staged script is uncompiled OR was edited after compiling —
    // building would otherwise read an empty/stale mini-cache. The warning text below uses the
    // same flag.
    final scriptsNotReady = ref
        .watch(scriptModsProvider)
        .entries
        .any((s) => !scriptCompileFresh(s));
    final gameRoot = gameRootFromExe(ref.watch(gameExePathProvider));
    // Building/deploying an empty bundle would only retire the active mod, so require content.
    final hasContent =
        overrides +
            locEdits +
            audio +
            textures +
            scripts +
            dialogTopics +
            voice >
        0;

    return AlertDialog(
      title: const Text('Build & Deploy Mod'),
      content: SizedBox(
        width: 460,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (installSafety.showBlockingBanner)
                ScriptCompileInstallStateBanner(
                  state: installSafety,
                  onRecheck: () => ref
                      .read(scriptCompileInstallSafetyProvider.notifier)
                      .refresh(),
                ),
              TextFormField(
                initialValue: ref.read(modNameProvider),
                decoration: const InputDecoration(
                  labelText: 'Mod name',
                  isDense: true,
                ),
                onChanged: (v) => ref.read(modNameProvider.notifier).state = v,
              ),
              const SizedBox(height: 8),
              TextFormField(
                initialValue: ref.read(modDelayMsProvider).toString(),
                decoration: const InputDecoration(
                  labelText: 'UE4SS load delay (ms)',
                  helperText:
                      'Wait before applying overrides at game start (0 = none)',
                  isDense: true,
                ),
                keyboardType: TextInputType.number,
                // Keep the provider in sync so build/deploy uses the chosen delay instead of always
                // defaulting to 0. Blank or non-numeric input falls back to 0.
                onChanged: (v) => ref.read(modDelayMsProvider.notifier).state =
                    int.tryParse(v.trim()) ?? 0,
              ),
              const SizedBox(height: 12),
              Text('Contents', style: theme.textTheme.labelMedium),
              Text('• $overrides item override(s)'),
              Text('• $locEdits localized text edit(s)'),
              Text('• $audio audio replacement(s)'),
              Text('• $textures texture replacement(s)'),
              Text('• $scripts script mod(s)'),
              Text('• $dialogTopics runtime dialog topic(s)'),
              Text('• $voice dialog voice edit(s)'),
              if (scriptsNotReady)
                Text(
                  'Some script mods are not compiled or were edited after compiling — (re)compile '
                  'them in the AngelScript tab first.',
                  style: TextStyle(color: theme.colorScheme.error),
                ),
              if (hasDraftVoiceAdds)
                Text(
                  'New voice archive members are Draft-only and not runtime-qualified yet. '
                  'Use a replacement to Build or Deploy.',
                  style: TextStyle(color: theme.colorScheme.error),
                ),
              const SizedBox(height: 12),
              if (gameRoot == null)
                Text(
                  'Set the game path in Settings to deploy directly.',
                  style: TextStyle(color: theme.colorScheme.error),
                ),
              if (_busy)
                const Padding(
                  padding: EdgeInsets.symmetric(vertical: 8),
                  child: LinearProgressIndicator(),
                ),
              if (_status != null)
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: Text(
                    _status!,
                    style: TextStyle(
                      color: _error
                          ? theme.colorScheme.error
                          : theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
            ],
          ),
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
          onPressed:
              (_busy || !hasContent || scriptsNotReady || hasDraftVoiceAdds)
              ? null
              : _buildToFolder,
          icon: const Icon(Icons.folder_zip_outlined, size: 18),
          label: const Text('Build to folder…'),
        ),
        FilledButton.icon(
          onPressed:
              (_busy ||
                  gameRoot == null ||
                  !hasContent ||
                  scriptsNotReady ||
                  hasDraftVoiceAdds ||
                  !installSafety.liveMutationAllowed)
              ? null
              : () => _deploy(gameRoot),
          icon: const Icon(Icons.rocket_launch_outlined, size: 18),
          label: const Text('Deploy to game'),
        ),
      ],
    );
  }
}
