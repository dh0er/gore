import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'app/domain/ui_settings.dart';
import 'app/game_paths.dart';
import 'app/ui/about_dialog.dart';
import 'app/ui/window_chrome.dart';
import 'conflicts/ui/conflict_panel.dart';
import 'l10n/app_localizations.dart';
import 'library/domain/conflicts_provider.dart';
import 'library/domain/library_notifier.dart';
import 'library/domain/models.dart';
import 'library/ui/detail_panel.dart';
import 'library/ui/mod_list.dart';
import 'settings/ui/settings_tab.dart';
import 'status/domain/status_notifier.dart';

/// Home: a Mods tab (library list + detail + conflicts, with an import/apply
/// action bar) and the unchanged Settings tab.
class HomePage extends ConsumerStatefulWidget {
  const HomePage({super.key});

  @override
  ConsumerState<HomePage> createState() => _HomePageState();
}

class _HomePageState extends ConsumerState<HomePage> {
  @override
  void initState() {
    super.initState();
    // Refresh deployment status once the first frame (and thus providers) are
    // ready. gameRoot may be null; the notifier records that as an error.
    WidgetsBinding.instance.addPostFrameCallback((_) => _refreshStatus());
  }

  String? get _gameRoot => gameRootFromExe(ref.read(gameExePathProvider));

  void _refreshStatus() {
    if (!mounted) return;
    ref.read(statusProvider.notifier).refresh(_gameRoot);
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;

    return Scaffold(
      appBar: AppBar(
        // The app bar doubles as the (frameless) title bar: the icon + name
        // sit in a drag area, and the window buttons live at the far end.
        title: WindowDragArea(
          child: Row(
            children: [
              const SizedBox(width: 8),
              Image.asset('assets/gore_manager_icon.png', height: 22),
              const SizedBox(width: 10),
              Text(l10n.appTitle),
              const Expanded(child: SizedBox(height: 32)),
            ],
          ),
        ),
        titleSpacing: 0,
        centerTitle: false,
        scrolledUnderElevation: 0,
        actions: [
          IconButton(
            icon: Icon(isDark ? Icons.light_mode : Icons.dark_mode),
            tooltip: isDark ? l10n.lightMode : l10n.darkMode,
            onPressed: () => ref.read(themeModeProvider.notifier).setThemeMode(
                  isDark ? ThemeMode.light : ThemeMode.dark,
                ),
          ),
          IconButton(
            icon: const Icon(Icons.info_outline),
            tooltip: l10n.about,
            onPressed: () => showDialog<void>(
              context: context,
              builder: (_) => const GoreManagerAboutDialog(),
            ),
          ),
          const WindowControls(),
          const SizedBox(width: 8),
        ],
      ),
      body: DefaultTabController(
        length: 2,
        child: Column(
          children: [
            Container(
              color: scheme.surfaceContainerLowest,
              child: Row(
                children: [
                  Expanded(
                    child: TabBar(
                      isScrollable: true,
                      tabAlignment: TabAlignment.start,
                      padding: const EdgeInsetsDirectional.only(start: 4),
                      tabs: [
                        Tab(
                          icon: const Icon(Icons.extension_outlined),
                          text: l10n.tabMods,
                        ),
                        Tab(
                          icon: const Icon(Icons.settings_outlined),
                          text: l10n.tabSettings,
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
            Expanded(
              child: TabBarView(
                children: [
                  _ModsTab(
                    onAfterMutation: _refreshStatus,
                  ),
                  const SettingsTab(),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// The Mods tab: action bar + (mod list | detail) + collapsible conflict panel.
class _ModsTab extends ConsumerWidget {
  const _ModsTab({required this.onAfterMutation});

  /// Called after any action that changes deployed/target state so the status
  /// chip can refresh.
  final VoidCallback onAfterMutation;

  String? _gameRoot(WidgetRef ref) =>
      gameRootFromExe(ref.read(gameExePathProvider));

  Future<void> _importFolder(WidgetRef ref) async {
    final path = await getDirectoryPath();
    if (path == null) return;
    await ref.read(libraryProvider.notifier).import(path);
    onAfterMutation();
  }

  Future<void> _importFile(WidgetRef ref, AppLocalizations l10n) async {
    final group = XTypeGroup(
      label: l10n.actionImport,
      extensions: const ['zip', 'pak', 'utoc', 'lcache', 'bank', 'Cache'],
    );
    final file = await openFile(acceptedTypeGroups: [group]);
    if (file == null) return;
    await ref.read(libraryProvider.notifier).import(file.path);
    onAfterMutation();
  }

  Future<void> _apply(BuildContext context, WidgetRef ref) async {
    final root = _gameRoot(ref);
    if (root == null) return;
    await ref.read(statusProvider.notifier).apply(root);
    // Applying resets deployed==target; the library isn't touched, but a
    // re-analyze is cheap and the status was already refreshed by apply().
    ref.invalidate(conflictsProvider);
  }

  Future<void> _undeployAll(BuildContext context, WidgetRef ref) async {
    final l10n = AppLocalizations.of(context);
    final root = _gameRoot(ref);
    if (root == null) return;
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        content: Text(l10n.undeployAllConfirm),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: Text(l10n.commonCancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: Text(l10n.undeployAllAction),
          ),
        ],
      ),
    );
    if (ok != true) return;
    await ref.read(statusProvider.notifier).undeployAll(root);
  }

  Future<void> _promptTakeOver(BuildContext context, WidgetRef ref) async {
    final l10n = AppLocalizations.of(context);
    final root = _gameRoot(ref);
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l10n.takeOverTitle),
        content: Text(l10n.takeOverBody),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: Text(l10n.commonCancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: Text(l10n.takeOverAction),
          ),
        ],
      ),
    );
    if (ok != true || root == null) return;
    await ref.read(statusProvider.notifier).undeployAll(root);
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final gamePath = ref.watch(gameExePathProvider);
    final gameRoot = gameRootFromExe(gamePath);
    final status = ref.watch(statusProvider);
    final library = ref.watch(libraryProvider);
    final conflicts = ref.watch(conflictsProvider);
    final conflictCount = conflicts.value?.length ?? 0;

    // Apply is enabled when the enabled loadout differs from what's deployed —
    // including the first-ever deploy (nothing_deployed + >=1 enabled mod).
    final applyEnabled = canApply(
      status.status,
      library,
      gameRoot != null,
      status.busy,
    );

    return Column(
      children: [
        // --- Action bar -------------------------------------------------
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
          child: Row(
            children: [
              // Import: folder or file, chosen from a menu (one picker can't
              // offer both directories and files).
              MenuAnchor(
                builder: (ctx, controller, _) => OutlinedButton.icon(
                  onPressed: () => controller.isOpen
                      ? controller.close()
                      : controller.open(),
                  icon: const Icon(Icons.add),
                  label: Text(l10n.actionImport),
                ),
                menuChildren: [
                  MenuItemButton(
                    leadingIcon: const Icon(Icons.folder_open),
                    onPressed: () => _importFolder(ref),
                    child: Text(l10n.importFolder),
                  ),
                  MenuItemButton(
                    leadingIcon: const Icon(Icons.insert_drive_file_outlined),
                    onPressed: () => _importFile(ref, l10n),
                    child: Text(l10n.importFile),
                  ),
                ],
              ),
              const SizedBox(width: 8),
              Tooltip(
                message: l10n.applyTooltip,
                child: FilledButton.icon(
                  onPressed:
                      applyEnabled ? () => _apply(context, ref) : null,
                  icon: const Icon(Icons.play_arrow),
                  label: Text(l10n.actionApply),
                ),
              ),
              const SizedBox(width: 12),
              _StatusChip(
                state: status,
                onStudioTap: () => _promptTakeOver(context, ref),
              ),
              if (status.busy || conflicts.isLoading) ...[
                const SizedBox(width: 12),
                const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              ],
              const Spacer(),
              PopupMenuButton<_OverflowAction>(
                onSelected: (action) => switch (action) {
                  _OverflowAction.refresh =>
                    ref.read(statusProvider.notifier).refresh(gameRoot),
                  _OverflowAction.undeployAll =>
                    _undeployAll(context, ref),
                },
                itemBuilder: (ctx) => [
                  PopupMenuItem(
                    value: _OverflowAction.refresh,
                    child: Text(l10n.refreshAction),
                  ),
                  PopupMenuItem(
                    value: _OverflowAction.undeployAll,
                    child: Text(l10n.undeployAllAction),
                  ),
                ],
              ),
            ],
          ),
        ),

        // Game-path hint / apply-report / errors banner.
        _InfoBanner(gameRoot: gameRoot, status: status),

        const Divider(height: 1),

        // --- List | detail ---------------------------------------------
        Expanded(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Expanded(child: ModList()),
              const VerticalDivider(width: 1),
              const SizedBox(width: 380, child: DetailPanel()),
            ],
          ),
        ),

        // --- Conflicts (collapsible) -----------------------------------
        Material(
          color: theme.colorScheme.surfaceContainerLowest,
          child: ExpansionTile(
            title: Text(l10n.conflictsTitle(conflictCount)),
            leading: const Icon(Icons.merge_type),
            children: const [
              SizedBox(height: 240, child: ConflictPanel()),
            ],
          ),
        ),
      ],
    );
  }
}

/// Whether the Apply button should be enabled.
///
/// Apply is offered whenever the target (the enabled subset of the current
/// loadout) differs from what is deployed — which includes the very first
/// deploy. Concretely:
///  * always disabled without a game path or while an FFI call is in flight;
///  * enabled for [ManagerStatusChangesPending] / [ManagerStatusGameUpdated]
///    (deployed drifted from target);
///  * enabled for [ManagerStatusNothingDeployed] only when the loadout has at
///    least one enabled mod (there is something to deploy) — this is what makes
///    the first-ever deploy, and the post-studio-take-over deploy, reachable;
///  * disabled otherwise: [ManagerStatusInSync] (nothing changed),
///    [ManagerStatusStudioDeployActive] (that path shows take-over, not Apply),
///    an unknown/null status, or NothingDeployed with zero enabled mods.
bool canApply(
  ManagerStatusView? status,
  LibraryState library,
  bool gameRootSet,
  bool busy,
) {
  if (!gameRootSet || busy) return false;
  return switch (status) {
    ManagerStatusChangesPending() => true,
    ManagerStatusGameUpdated() => true,
    ManagerStatusNothingDeployed() =>
      library.loadout.entries.any((e) => e.enabled),
    _ => false,
  };
}

enum _OverflowAction { refresh, undeployAll }

/// The deployment-status chip. Maps each [ManagerStatusView] variant to a
/// localized label + tone; tapping the studio-deploy variant runs [onStudioTap].
class _StatusChip extends StatelessWidget {
  const _StatusChip({required this.state, required this.onStudioTap});

  final StatusState state;
  final VoidCallback onStudioTap;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final scheme = Theme.of(context).colorScheme;
    final status = state.status;

    // studioActive (from a blocked apply) also surfaces the studio chip even
    // if mgr_status hasn't been re-read as studio_deploy_active yet.
    final isStudio =
        status is ManagerStatusStudioDeployActive || state.studioActive;

    final (String label, Color bg, Color fg, IconData icon) = switch (status) {
      ManagerStatusInSync() => (
          l10n.statusInSync,
          scheme.secondaryContainer,
          scheme.onSecondaryContainer,
          Icons.check_circle_outline,
        ),
      ManagerStatusChangesPending() => (
          l10n.statusChangesPending,
          scheme.tertiaryContainer,
          scheme.onTertiaryContainer,
          Icons.pending_outlined,
        ),
      ManagerStatusGameUpdated() => (
          l10n.statusGameUpdated,
          scheme.errorContainer,
          scheme.onErrorContainer,
          Icons.system_update_alt,
        ),
      ManagerStatusStudioDeployActive() => (
          l10n.statusStudioDeploy,
          scheme.errorContainer,
          scheme.onErrorContainer,
          Icons.lock_outline,
        ),
      ManagerStatusNothingDeployed() => (
          l10n.statusNothingDeployed,
          scheme.surfaceContainerHighest,
          scheme.onSurfaceVariant,
          Icons.circle_outlined,
        ),
      _ when isStudio => (
          l10n.statusStudioDeploy,
          scheme.errorContainer,
          scheme.onErrorContainer,
          Icons.lock_outline,
        ),
      _ => (
          l10n.statusNothingDeployed,
          scheme.surfaceContainerHighest,
          scheme.onSurfaceVariant,
          Icons.circle_outlined,
        ),
    };

    final chip = Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(16),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 16, color: fg),
          const SizedBox(width: 6),
          Text(label, style: TextStyle(color: fg, fontSize: 13)),
        ],
      ),
    );

    if (isStudio) {
      return InkWell(
        borderRadius: BorderRadius.circular(16),
        onTap: onStudioTap,
        child: chip,
      );
    }
    return chip;
  }
}

/// Contextual banner beneath the action bar: prompts to set the game path,
/// echoes the last apply report + warnings, or surfaces an error.
class _InfoBanner extends ConsumerWidget {
  const _InfoBanner({required this.gameRoot, required this.status});

  final String? gameRoot;
  final StatusState status;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final library = ref.watch(libraryProvider);

    final children = <Widget>[];

    if (gameRoot == null) {
      children.add(_line(
        theme,
        Icons.info_outline,
        l10n.errorSetGamePath,
        theme.colorScheme.onSurfaceVariant,
      ));
    }

    // Errors: a "no game path" sentinel maps to the friendly hint (already
    // shown above), other errors surface verbatim.
    if (status.error != null && status.error != StatusNotifier.noGamePath) {
      children.add(_line(
        theme,
        Icons.error_outline,
        status.error!,
        theme.colorScheme.error,
      ));
    }
    if (library.error != null) {
      children.add(_line(
        theme,
        Icons.error_outline,
        library.error!,
        theme.colorScheme.error,
      ));
    }

    final report = status.lastReport;
    if (report != null) {
      children.add(_line(
        theme,
        Icons.check_circle_outline,
        l10n.applyReportApplied(report.applied.length),
        theme.colorScheme.onSurfaceVariant,
      ));
      for (final w in report.warnings) {
        children.add(_line(
          theme,
          Icons.warning_amber_rounded,
          w,
          Colors.amber.shade800,
        ));
      }
    }

    if (children.isEmpty) return const SizedBox.shrink();
    return Container(
      width: double.infinity,
      color: theme.colorScheme.surfaceContainerLowest,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: children,
      ),
    );
  }

  Widget _line(ThemeData theme, IconData icon, String text, Color color) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(icon, size: 16, color: color),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              text,
              style: theme.textTheme.bodySmall?.copyWith(color: color),
            ),
          ),
        ],
      ),
    );
  }
}
