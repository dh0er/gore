import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'app/domain/game_launcher.dart';
import 'app/domain/ui_settings.dart';
import 'app/game_paths.dart';
import 'app/ui/about_dialog.dart';
import 'app/ui/window_chrome.dart';
import 'conflicts/ui/conflict_panel.dart';
import 'core/diagnostic_text.dart';
import 'core/mgr_ffi.dart';
import 'core/technical_details.dart';
import 'l10n/app_localizations.dart';
import 'library/domain/conflicts_provider.dart';
import 'library/domain/library_notifier.dart';
import 'library/domain/models.dart';
import 'library/ui/detail_panel.dart';
import 'library/ui/import_feedback.dart';
import 'library/ui/import_source_picker.dart';
import 'library/ui/mod_list.dart';
import 'preflight/domain/models.dart';
import 'preflight/domain/preflight_notifier.dart';
import 'settings/ui/settings_tab.dart';
import 'status/domain/status_notifier.dart';
import 'status/ui/status_details_dialog.dart';

bool _preflightFindingUsesInstallRecoveryGuide(PreflightCheckView finding) =>
    switch (finding.action) {
      PreflightActionKind.waitForInstallMutation ||
      PreflightActionKind.recoverInstall => true,
      _ => false,
    };

bool _preflightFindingUsesRetry(PreflightCheckView finding) =>
    switch (finding.action) {
      PreflightActionKind.selectGameRoot ||
      PreflightActionKind.recoverDeployment ||
      PreflightActionKind.waitForInstallMutation ||
      PreflightActionKind.recoverInstall ||
      PreflightActionKind.recoverManagerMutation ||
      PreflightActionKind.removeStudioDeployment ||
      PreflightActionKind.reviewApply ||
      PreflightActionKind.reviewReapply ||
      PreflightActionKind.inspectDeployment ||
      PreflightActionKind.runFullStatus => false,
      _ => true,
    };

enum _PreflightFocusTarget {
  retry,
  installMutationWait,
  installRecovery,
  managerMutationRecovery,
}

bool _preflightFocusTargetVisible(
  PreflightState state,
  _PreflightFocusTarget target,
  PreflightActionKind? expectedAction,
  String? expectedToken,
) {
  final finding = state.authoritative
      ? state.report?.primarySetupFinding
      : null;
  return switch (target) {
    _PreflightFocusTarget.retry =>
      state.error != null ||
          (finding != null && _preflightFindingUsesRetry(finding)),
    _PreflightFocusTarget.installMutationWait =>
      state.error == null &&
          expectedAction == PreflightActionKind.waitForInstallMutation &&
          finding?.action == expectedAction,
    _PreflightFocusTarget.installRecovery =>
      state.error == null &&
          expectedAction == PreflightActionKind.recoverInstall &&
          finding?.action == expectedAction &&
          finding != null,
    _PreflightFocusTarget.managerMutationRecovery =>
      state.error == null &&
          expectedAction == PreflightActionKind.recoverManagerMutation &&
          finding?.action == expectedAction &&
          expectedToken != null &&
          finding?.actionToken == expectedToken &&
          finding != null,
  };
}

/// Home: a Mods tab (library list + detail + conflicts, with an import/apply
/// action bar) and the unchanged Settings tab.
class HomePage extends ConsumerStatefulWidget {
  const HomePage({
    super.key,
    this.importSourcePicker = const FileSelectorImportSourcePicker(),
  });

  final ImportSourcePicker importSourcePicker;

  @override
  ConsumerState<HomePage> createState() => _HomePageState();
}

class _HomePageState extends ConsumerState<HomePage> {
  final FocusNode _importFocusNode = FocusNode(
    debugLabel: 'mod-manager-import-after-remove',
  );
  final FocusNode _libraryRefreshFocusNode = FocusNode(
    debugLabel: 'mod-manager-refresh-unknown-library',
  );
  final FocusNode _statusDetailsFocusNode = FocusNode(
    debugLabel: 'mod-manager-status-details',
  );
  final FocusNode _settingsGamePathFocusNode = FocusNode(
    debugLabel: 'mod-manager-settings-game-path',
  );
  final FocusNode _preflightRetryFocusNode = FocusNode(
    debugLabel: 'mod-manager-preflight-retry',
  );
  final FocusNode _preflightInstallMutationWaitFocusNode = FocusNode(
    debugLabel: 'mod-manager-preflight-install-mutation-wait',
  );
  final FocusNode _preflightInstallRecoveryFocusNode = FocusNode(
    debugLabel: 'mod-manager-preflight-install-recovery',
  );
  final FocusNode _preflightManagerMutationRecoveryFocusNode = FocusNode(
    debugLabel: 'mod-manager-preflight-manager-mutation-recovery',
  );
  ({FocusNode node, String? removedModId})? _pendingFocus;
  int _pendingFocusGeneration = 0;
  int _selectionGeneration = 0;
  bool _importRequestActive = false;
  final Set<Completer<void>> _preflightReloadWaiters = {};
  ({
    String root,
    int generation,
    _PreflightFocusTarget target,
    PreflightActionKind? action,
    String? actionToken,
    bool fallbackToStatus,
  })?
  _preflightRetryFocusRequest;

  Future<void> _startGame(BuildContext context) async {
    final exe = gameExecutableFor(ref.read(gameExePathProvider));
    if (exe == null) return;
    final launched = await ref.read(gameLauncherProvider)(exe);
    if (launched || !context.mounted) return;
    final l10n = AppLocalizations.of(context);
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(
        SnackBar(
          content: Text(
            l10n.startGameFailed,
            key: const ValueKey('start-game-failed'),
          ),
        ),
      );
  }

  String? get _gameRoot => gameRootFromExe(ref.read(gameExePathProvider));

  bool get _focusBlocked =>
      ref.read(libraryProvider).busy ||
      ref.read(statusProvider).busy ||
      ref.read(preflightProvider).busy ||
      ref.read(conflictsProvider).isLoading;

  void _refreshPreflightWhenIdle() {
    if (!mounted) return;
    final preflight = ref.read(preflightProvider);
    if (!preflight.pending ||
        preflight.busy ||
        ref.read(libraryProvider).busy ||
        ref.read(statusProvider).busy ||
        ref.read(conflictsProvider).isLoading) {
      return;
    }
    unawaited(ref.read(preflightProvider.notifier).refresh());
  }

  Future<void> _retryPreflightAndWait({
    required String expectedRoot,
    required bool restoreFocus,
    required PreflightActionKind expectedAction,
    required String expectedToken,
  }) async {
    final notifier = ref.read(preflightProvider.notifier);
    notifier.retry();
    final requestedGeneration = ref.read(preflightProvider).generation;
    _rememberPreflightInstallRecoveryFocus(
      expectedRoot,
      requestedGeneration,
      restoreFocus,
      expectedAction,
      true,
      expectedToken,
    );

    bool settled(PreflightState state) =>
        state.candidateRoot != expectedRoot ||
        (state.generation >= requestedGeneration &&
            !state.busy &&
            !state.pending &&
            (state.authoritative || state.error != null));

    if (settled(ref.read(preflightProvider))) return;

    final completed = Completer<void>();
    _preflightReloadWaiters.add(completed);
    final subscription = ref.listenManual<PreflightState>(preflightProvider, (
      previous,
      next,
    ) {
      if (!completed.isCompleted && settled(next)) completed.complete();
    });
    try {
      if (settled(ref.read(preflightProvider))) return;
      _refreshPreflightWhenIdle();
      await completed.future;
    } finally {
      subscription.close();
      _preflightReloadWaiters.remove(completed);
    }
  }

  void _queueFocusWhenIdle(FocusNode node, {String? removedModId}) {
    final selected = ref.read(selectedModProvider);
    if (removedModId != null && selected != null && selected != removedModId) {
      return;
    }
    _pendingFocus = (node: node, removedModId: removedModId);
    _pendingFocusGeneration++;
    _flushPendingFocusWhenIdle();
  }

  void _flushPendingFocusWhenIdle() {
    final pending = _pendingFocus;
    if (pending == null) return;
    final selected = ref.read(selectedModProvider);
    if (pending.removedModId != null &&
        selected != null &&
        selected != pending.removedModId) {
      _pendingFocus = null;
      _pendingFocusGeneration++;
      return;
    }
    if (_focusBlocked) return;

    final generation = _pendingFocusGeneration;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || generation != _pendingFocusGeneration) return;
      final latest = _pendingFocus;
      if (latest == null || _focusBlocked) return;
      final latestSelection = ref.read(selectedModProvider);
      if (latest.removedModId != null &&
          latestSelection != null &&
          latestSelection != latest.removedModId) {
        _pendingFocus = null;
        _pendingFocusGeneration++;
        return;
      }
      _pendingFocus = null;
      _pendingFocusGeneration++;
      latest.node.requestFocus();
    });
  }

  void _queueImportFocusAfterRemove(String removedModId) =>
      _queueFocusWhenIdle(_importFocusNode, removedModId: removedModId);

  void _queueRefreshFocusAfterRemove(String removedModId) =>
      _queueFocusWhenIdle(_libraryRefreshFocusNode, removedModId: removedModId);

  void _queueRefreshFocus() => _queueFocusWhenIdle(_libraryRefreshFocusNode);

  void _rememberPreflightRetryFocus(String? root, int generation) {
    _preflightRetryFocusRequest =
        root != null && _preflightRetryFocusNode.hasFocus
        ? (
            root: root,
            generation: generation,
            target: _PreflightFocusTarget.retry,
            action: null,
            actionToken: null,
            fallbackToStatus: false,
          )
        : null;
  }

  void _rememberPreflightInstallRecoveryFocus(
    String? root,
    int generation,
    bool restoreFocus,
    PreflightActionKind action,
    bool fallbackToStatus,
    String? actionToken,
  ) {
    _preflightRetryFocusRequest = root != null && restoreFocus
        ? (
            root: root,
            generation: generation,
            target: switch (action) {
              PreflightActionKind.waitForInstallMutation =>
                _PreflightFocusTarget.installMutationWait,
              PreflightActionKind.recoverInstall =>
                _PreflightFocusTarget.installRecovery,
              PreflightActionKind.recoverManagerMutation =>
                _PreflightFocusTarget.managerMutationRecovery,
              _ => throw ArgumentError.value(action, 'action'),
            },
            action: action,
            actionToken: actionToken,
            fallbackToStatus: fallbackToStatus,
          )
        : null;
  }

  FocusNode _preflightFocusNode(_PreflightFocusTarget target) =>
      switch (target) {
        _PreflightFocusTarget.retry => _preflightRetryFocusNode,
        _PreflightFocusTarget.installMutationWait =>
          _preflightInstallMutationWaitFocusNode,
        _PreflightFocusTarget.installRecovery =>
          _preflightInstallRecoveryFocusNode,
        _PreflightFocusTarget.managerMutationRecovery =>
          _preflightManagerMutationRecoveryFocusNode,
      };

  bool get _importAuthorityAvailable {
    final library = ref.read(libraryProvider);
    return !library.busy &&
        library.authoritative &&
        !ref.read(statusProvider).busy &&
        !ref.read(preflightProvider).busy &&
        !ref.read(conflictsProvider).isLoading;
  }

  bool get _canStartImport =>
      !_importRequestActive && _importAuthorityAvailable;

  Future<void> _importFolder() =>
      _pickAndImport(() => widget.importSourcePicker.pickFolder());

  Future<void> _importFile() => _pickAndImport(
    () => widget.importSourcePicker.pickFile(
      dialogLabel: AppLocalizations.of(context).actionImport,
    ),
  );

  Future<void> _pickAndImport(Future<String?> Function() pick) async {
    if (!_canStartImport) return;
    final selectionTicket = _selectionGeneration;
    setState(() => _importRequestActive = true);
    try {
      String? path;
      try {
        path = await pick();
      } catch (error) {
        if (!mounted) return;
        showImportFailureFeedback(
          context,
          MgrFfiException(
            'import picker: $error',
            code: 'IMPORT_PICKER_FAILED',
          ),
          ref.read(libraryProvider),
        );
        return;
      }
      if (!mounted ||
          path == null ||
          path.trim().isEmpty ||
          !_importAuthorityAvailable) {
        return;
      }

      MgrImportOutcome? outcome;
      try {
        outcome = await ref.read(libraryProvider.notifier).import(path);
      } on MgrFfiException catch (error) {
        if (!mounted) return;
        final library = ref.read(libraryProvider);
        showImportFailureFeedback(context, error, library);
        return;
      } catch (error) {
        if (!mounted) return;
        final library = ref.read(libraryProvider);
        showImportFailureFeedback(
          context,
          MgrFfiException('$error', code: 'IMPORT_UNKNOWN_FAILURE'),
          library,
        );
        return;
      }
      if (!mounted || outcome == null) return;

      final library = ref.read(libraryProvider);
      if (selectionTicket == _selectionGeneration &&
          library.authoritative &&
          library.modById(outcome.entry.id) != null) {
        ref.read(selectedModProvider.notifier).state = outcome.entry.id;
      }
      showImportSuccessFeedback(
        context,
        outcome,
        showMatchReason: ref.read(advancedDetailsProvider),
      );
    } finally {
      if (mounted) setState(() => _importRequestActive = false);
    }
  }

  void _settlePreflightRetryFocus(
    PreflightState? previous,
    PreflightState next,
  ) {
    final request = _preflightRetryFocusRequest;
    if (request == null || previous?.busy != true || next.busy) return;
    if (next.candidateRoot == request.root &&
        next.generation == request.generation &&
        next.pending) {
      return;
    }
    _preflightRetryFocusRequest = null;
    if (next.candidateRoot != request.root ||
        next.generation != request.generation ||
        (!_preflightFocusTargetVisible(
              next,
              request.target,
              request.action,
              request.actionToken,
            ) &&
            !request.fallbackToStatus)) {
      return;
    }
    final requestedNode = _preflightFocusNode(request.target);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      final latest = ref.read(preflightProvider);
      final requestedTargetVisible = _preflightFocusTargetVisible(
        latest,
        request.target,
        request.action,
        request.actionToken,
      );
      final targetNode = requestedTargetVisible
          ? requestedNode
          : request.fallbackToStatus
          ? _statusDetailsFocusNode
          : null;
      final currentFocus = FocusManager.instance.primaryFocus;
      final focusMovedElsewhere =
          currentFocus != null &&
          currentFocus != requestedNode &&
          currentFocus != targetNode &&
          currentFocus is! FocusScopeNode;
      if (latest.candidateRoot != request.root ||
          latest.generation != request.generation ||
          latest.busy ||
          latest.pending ||
          targetNode == null ||
          focusMovedElsewhere ||
          targetNode.context == null) {
        return;
      }
      targetNode.requestFocus();
    });
  }

  @override
  void dispose() {
    for (final waiter in _preflightReloadWaiters) {
      if (!waiter.isCompleted) waiter.complete();
    }
    _preflightReloadWaiters.clear();
    _importFocusNode.dispose();
    _libraryRefreshFocusNode.dispose();
    _statusDetailsFocusNode.dispose();
    _settingsGamePathFocusNode.dispose();
    _preflightRetryFocusNode.dispose();
    _preflightInstallMutationWaitFocusNode.dispose();
    _preflightInstallRecoveryFocusNode.dispose();
    _preflightManagerMutationRecoveryFocusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    // Choosing (or clearing) the game exe changes the root that deployment
    // status is judged against. The startup mgr_status ran before any path was
    // set, so without this the deployment chip + Apply gating stay stale until
    // the user hits the overflow Refresh. Re-derive the root from the *new*
    // path and refresh status on every change. ref.listen only fires on an
    // actual value change, so startup remains owned by the settled-library
    // listener below and path changes get exactly one refresh here.
    ref.listen<String?>(gameExePathProvider, (previous, next) {
      if (!mounted) return;
      ref
          .read(preflightProvider.notifier)
          .selectRoot(diagnosticGameRootCandidate(next));
      ref.read(statusProvider.notifier).refresh(gameRootFromExe(next));
      _refreshPreflightWhenIdle();
    });

    // Enabling, disabling, reordering, importing, or removing mods changes what a
    // deploy would install, but only the loadout is persisted — nothing else
    // re-queries mgr_status, so the deployment chip + Apply gating would stay
    // stale ("In sync", Apply disabled) until the manual Refresh. Re-check status
    // once the library settles (not busy) whenever the loadout entries changed.
    ref.listen<LibraryState>(libraryProvider, (previous, next) {
      if (!mounted) return;
      final invalidatesPreflight =
          previous == null ||
          (!previous.busy && next.busy) ||
          previous.authoritative != next.authoritative ||
          !_sameLoadoutEntries(previous.loadout, next.loadout);
      if (invalidatesPreflight) {
        ref.read(preflightProvider.notifier).invalidateLibrary();
      }
      if (next.authoritative) {
        final selected = ref.read(selectedModProvider);
        if (selected != null && next.modById(selected) == null) {
          ref.read(selectedModProvider.notifier).state = null;
        }
      }
      if (next.busy) return;
      final changed =
          previous == null ||
          previous.busy ||
          !_sameLoadoutEntries(previous.loadout, next.loadout);
      if (changed) {
        ref.read(statusProvider.notifier).refresh(_gameRoot);
      }
      _flushPendingFocusWhenIdle();
      _refreshPreflightWhenIdle();
    });
    ref.listen<StatusState>(statusProvider, (previous, next) {
      _flushPendingFocusWhenIdle();
      _refreshPreflightWhenIdle();
    });
    ref.listen(conflictsProvider, (previous, next) {
      _flushPendingFocusWhenIdle();
      _refreshPreflightWhenIdle();
    });
    ref.listen<PreflightState>(preflightProvider, (previous, next) {
      _settlePreflightRetryFocus(previous, next);
      _flushPendingFocusWhenIdle();
      _refreshPreflightWhenIdle();
    });
    ref.listen<String?>(selectedModProvider, (previous, next) {
      _selectionGeneration++;
      final pending = _pendingFocus;
      if (pending?.removedModId != null &&
          next != null &&
          next != pending!.removedModId) {
        _pendingFocus = null;
        _pendingFocusGeneration++;
      }
    });
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _refreshPreflightWhenIdle();
    });

    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final isDark = theme.brightness == Brightness.dark;
    final media = MediaQuery.of(context);
    // The title bar is a plain Row with no room to give: at narrow widths or
    // large text the zoom chip would push the window buttons off the edge.
    // Settings still shows the same percentage, so dropping it costs nothing.
    final textScale = media.textScaler.scale(1);
    final showZoomIndicator = media.size.width >= 720 && textScale <= 1.35;
    final startGameCompact = media.size.width < 760 || textScale > 1.35;
    final gameExe = gameExecutableFor(ref.watch(gameExePathProvider));
    final startGameBusy =
        ref.watch(libraryProvider).busy ||
        ref.watch(statusProvider).busy ||
        ref.watch(preflightProvider).busy ||
        ref.watch(conflictsProvider).isLoading;
    final canStartGame = gameExe != null && !startGameBusy;

    return Scaffold(
      appBar: AppBar(
        // The app bar doubles as the (frameless) title bar: the icon + name
        // sit in a drag area, and the window buttons live at the far end.
        title: WindowDragArea(
          child: Row(
            children: [
              const SizedBox(width: 8),
              Image.asset('assets/gore_manager_icon.png', height: 32),
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
          // The whole UI scales with Ctrl +/-, which is easy to trigger by
          // accident; showing the current factor makes that visible and
          // recoverable instead of leaving the app mysteriously large.
          if (showZoomIndicator) ...[
            Tooltip(
              message: l10n.zoomTip,
              child: Container(
                key: const ValueKey('ui-scale-indicator'),
                padding: const EdgeInsets.symmetric(
                  horizontal: 10,
                  vertical: 4,
                ),
                decoration: BoxDecoration(
                  color: scheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(16),
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.zoom_in,
                      size: 16,
                      color: scheme.onSurfaceVariant,
                    ),
                    const SizedBox(width: 4),
                    Text(
                      '${(ref.watch(uiScaleProvider) * 100).round()}%',
                      style: theme.textTheme.bodySmall,
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(width: 8),
          ],
          IconButton(
            icon: Icon(isDark ? Icons.light_mode : Icons.dark_mode),
            tooltip: isDark ? l10n.lightMode : l10n.darkMode,
            onPressed: () => ref
                .read(themeModeProvider.notifier)
                .setThemeMode(isDark ? ThemeMode.light : ThemeMode.dark),
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
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              const Icon(Icons.extension_outlined),
                              const SizedBox(width: 8),
                              Text(
                                l10n.tabMods,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                              ),
                            ],
                          ),
                        ),
                        Tab(
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              const Icon(Icons.settings_outlined),
                              const SizedBox(width: 8),
                              Text(
                                l10n.tabSettings,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                  // Far end of the tab row: the full window width to sit in,
                  // and outside the TabBarView so it stays put on either tab.
                  // Disabled while a Manager operation is writing to the
                  // installation: starting the game against a half-written
                  // game directory is the one genuinely unsafe moment.
                  Padding(
                    padding: const EdgeInsetsDirectional.only(start: 8, end: 8),
                    child: startGameCompact
                        // The TabBar shares this row and only scrolls, so a
                        // full-width button here would push the tabs out of
                        // reach. The glyph keeps the action without the cost.
                        ? Semantics(
                            label: l10n.actionStartGame,
                            button: true,
                            child: IconButton.filled(
                              key: const ValueKey('start-game-action'),
                              onPressed: canStartGame
                                  ? () => _startGame(context)
                                  : null,
                              tooltip:
                                  '${l10n.actionStartGame} — '
                                  '${l10n.startGameTooltip}',
                              icon: const Icon(Icons.play_arrow),
                            ),
                          )
                        : Tooltip(
                            message: l10n.startGameTooltip,
                            child: FilledButton.icon(
                              key: const ValueKey('start-game-action'),
                              onPressed: canStartGame
                                  ? () => _startGame(context)
                                  : null,
                              icon: const Icon(Icons.play_arrow, size: 22),
                              style: FilledButton.styleFrom(
                                minimumSize: const Size(0, 40),
                                padding: const EdgeInsets.symmetric(
                                  horizontal: 18,
                                ),
                                textStyle: theme.textTheme.titleSmall,
                              ),
                              label: Text(
                                l10n.actionStartGame,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                              ),
                            ),
                          ),
                  ),
                ],
              ),
            ),
            Expanded(
              child: TabBarView(
                children: [
                  _ModsTab(
                    importFocusNode: _importFocusNode,
                    importFolder: _importFolder,
                    importFile: _importFile,
                    importRequestActive: _importRequestActive,
                    libraryRefreshFocusNode: _libraryRefreshFocusNode,
                    statusDetailsFocusNode: _statusDetailsFocusNode,
                    settingsGamePathFocusNode: _settingsGamePathFocusNode,
                    preflightRetryFocusNode: _preflightRetryFocusNode,
                    preflightInstallMutationWaitFocusNode:
                        _preflightInstallMutationWaitFocusNode,
                    preflightInstallRecoveryFocusNode:
                        _preflightInstallRecoveryFocusNode,
                    preflightManagerMutationRecoveryFocusNode:
                        _preflightManagerMutationRecoveryFocusNode,
                    queueImportFocusAfterRemove: _queueImportFocusAfterRemove,
                    queueRefreshFocusAfterRemove: _queueRefreshFocusAfterRemove,
                    queueRefreshFocus: _queueRefreshFocus,
                    rememberPreflightRetryFocus: _rememberPreflightRetryFocus,
                    rememberPreflightInstallRecoveryFocus:
                        _rememberPreflightInstallRecoveryFocus,
                    retryPreflightAndWait: _retryPreflightAndWait,
                  ),
                  SettingsTab(gamePathFocusNode: _settingsGamePathFocusNode),
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
  const _ModsTab({
    required this.importFocusNode,
    required this.importFolder,
    required this.importFile,
    required this.importRequestActive,
    required this.libraryRefreshFocusNode,
    required this.statusDetailsFocusNode,
    required this.settingsGamePathFocusNode,
    required this.preflightRetryFocusNode,
    required this.preflightInstallMutationWaitFocusNode,
    required this.preflightInstallRecoveryFocusNode,
    required this.preflightManagerMutationRecoveryFocusNode,
    required this.queueImportFocusAfterRemove,
    required this.queueRefreshFocusAfterRemove,
    required this.queueRefreshFocus,
    required this.rememberPreflightRetryFocus,
    required this.rememberPreflightInstallRecoveryFocus,
    required this.retryPreflightAndWait,
  });

  final FocusNode importFocusNode;
  final Future<void> Function() importFolder;
  final Future<void> Function() importFile;
  final bool importRequestActive;
  final FocusNode libraryRefreshFocusNode;
  final FocusNode statusDetailsFocusNode;
  final FocusNode settingsGamePathFocusNode;
  final FocusNode preflightRetryFocusNode;
  final FocusNode preflightInstallMutationWaitFocusNode;
  final FocusNode preflightInstallRecoveryFocusNode;
  final FocusNode preflightManagerMutationRecoveryFocusNode;
  final ValueChanged<String> queueImportFocusAfterRemove;
  final ValueChanged<String> queueRefreshFocusAfterRemove;
  final VoidCallback queueRefreshFocus;
  final void Function(String? root, int generation) rememberPreflightRetryFocus;
  final void Function(
    String? root,
    int generation,
    bool restoreFocus,
    PreflightActionKind action,
    bool fallbackToStatus,
    String? actionToken,
  )
  rememberPreflightInstallRecoveryFocus;
  final Future<void> Function({
    required String expectedRoot,
    required bool restoreFocus,
    required PreflightActionKind expectedAction,
    required String expectedToken,
  })
  retryPreflightAndWait;

  String? _gameRoot(WidgetRef ref) =>
      gameRootFromExe(ref.read(gameExePathProvider));

  Future<void> _apply(
    BuildContext context,
    WidgetRef ref,
    String expectedRoot,
  ) async {
    if (_gameRoot(ref) != expectedRoot) return;
    final root = expectedRoot;
    final status = ref.read(statusProvider);
    final library = ref.read(libraryProvider);
    final statusForCurrentRoot = status.statusRoot == root
        ? status.status
        : null;
    final studioActiveForCurrentRoot =
        status.gameRoot == root && status.studioActive;
    if (!canApply(
          statusForCurrentRoot,
          library,
          true,
          status.busy,
          studioActiveForCurrentRoot,
          statusError: status.gameRoot == root ? status.error : null,
        ) ||
        ref.read(preflightProvider).busy ||
        ref.read(conflictsProvider).isLoading) {
      return;
    }
    try {
      await ref.read(statusProvider.notifier).apply(root);
    } finally {
      // Applying can change install/recovery evidence even when native reports
      // an error. Discard the old snapshot and re-read after status/conflicts
      // settle rather than leaving a stale setup finding authoritative.
      ref.invalidate(conflictsProvider);
      ref.read(preflightProvider.notifier).invalidateLibrary();
    }
  }

  Future<void> _refreshAll(WidgetRef ref, {String? expectedRoot}) async {
    if ((expectedRoot != null && _gameRoot(ref) != expectedRoot) ||
        ref.read(libraryProvider).busy ||
        ref.read(statusProvider).busy ||
        ref.read(preflightProvider).busy ||
        ref.read(conflictsProvider).isLoading) {
      return;
    }
    await ref.read(libraryProvider.notifier).refresh();
  }

  Future<void> _undeployAll(
    BuildContext context,
    WidgetRef ref,
    String? expectedRoot,
  ) async {
    final l10n = AppLocalizations.of(context);
    if (expectedRoot == null ||
        _gameRoot(ref) != expectedRoot ||
        _statusActionBusy(ref)) {
      return;
    }
    final root = expectedRoot;
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
    if (ok != true || _gameRoot(ref) != root || _statusActionBusy(ref)) return;
    try {
      await ref.read(statusProvider.notifier).undeployAll(root);
    } finally {
      ref.read(preflightProvider.notifier).invalidateLibrary();
    }
  }

  Future<void> _recoverDeployment(
    BuildContext context,
    WidgetRef ref,
    String expectedRoot,
    int expectedGeneration,
  ) async {
    final l10n = AppLocalizations.of(context);
    if (_gameRoot(ref) != expectedRoot ||
        !_canRecoverDeployment(ref, expectedRoot, expectedGeneration)) {
      return;
    }
    final root = expectedRoot;
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l10n.statusRecoveryRequired),
        content: Text(l10n.recoveryRequiredConfirm),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: Text(l10n.commonCancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: Text(l10n.recoveryAction),
          ),
        ],
      ),
    );
    if (ok != true ||
        _gameRoot(ref) != root ||
        !_canRecoverDeployment(ref, root, expectedGeneration)) {
      return;
    }
    try {
      await ref.read(statusProvider.notifier).undeployAll(root);
    } finally {
      ref.read(preflightProvider.notifier).invalidateLibrary();
    }
  }

  Future<void> _promptTakeOver(
    BuildContext context,
    WidgetRef ref,
    String expectedRoot,
  ) async {
    final l10n = AppLocalizations.of(context);
    if (_gameRoot(ref) != expectedRoot || !_canTakeOver(ref, expectedRoot)) {
      return;
    }
    final root = expectedRoot;
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
    if (ok != true || _gameRoot(ref) != root || !_canTakeOver(ref, root)) {
      return;
    }
    try {
      await ref.read(statusProvider.notifier).undeployAll(root);
    } finally {
      ref.read(preflightProvider.notifier).invalidateLibrary();
    }
  }

  int? _deploymentRecoveryGeneration(
    PreflightState preflight,
    String? expectedPreflightRoot,
  ) {
    if (expectedPreflightRoot == null ||
        preflight.candidateRoot != expectedPreflightRoot ||
        !preflight.authoritative ||
        preflight.busy ||
        preflight.pending ||
        preflight.report?.primarySetupFinding?.action !=
            PreflightActionKind.recoverDeployment) {
      return null;
    }
    return preflight.generation;
  }

  bool _canRecoverDeployment(
    WidgetRef ref,
    String root,
    int expectedGeneration,
  ) {
    final status = ref.read(statusProvider);
    final preflight = ref.read(preflightProvider);
    final selectedPreflightRoot = diagnosticGameRootCandidate(
      ref.read(gameExePathProvider),
    );
    return !_statusActionBusy(ref) &&
        status.statusRoot == root &&
        status.status is ManagerStatusRecoveryRequired &&
        _deploymentRecoveryGeneration(preflight, selectedPreflightRoot) ==
            expectedGeneration;
  }

  bool _canTakeOver(WidgetRef ref, String root) =>
      !_statusActionBusy(ref) &&
      statusHasStudioOwnership(ref.read(statusProvider), root);

  bool _statusActionBusy(WidgetRef ref) =>
      ref.read(libraryProvider).busy ||
      ref.read(statusProvider).busy ||
      ref.read(preflightProvider).busy ||
      ref.read(conflictsProvider).isLoading;

  void _openSettings(BuildContext context) {
    DefaultTabController.of(context).animateTo(1);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (context.mounted) settingsGamePathFocusNode.requestFocus();
    });
  }

  Future<void> _showStatusDetails(BuildContext context, WidgetRef ref) async {
    final result = await showDialog<StatusDetailsResult>(
      context: context,
      builder: (dialogContext) => Consumer(
        builder: (context, dialogRef, _) {
          final gamePath = dialogRef.watch(gameExePathProvider);
          final gameRoot = gameRootFromExe(gamePath);
          final preflightRoot = diagnosticGameRootCandidate(gamePath);
          final status = dialogRef.watch(statusProvider);
          final library = dialogRef.watch(libraryProvider);
          final conflicts = dialogRef.watch(conflictsProvider);
          final preflight = dialogRef.watch(preflightProvider);
          final statusForCurrentRoot = status.statusRoot == gameRoot
              ? status.status
              : null;
          final studioActiveForCurrentRoot =
              status.gameRoot == gameRoot && status.studioActive;
          final operationsBusy =
              library.busy ||
              status.busy ||
              preflight.busy ||
              conflicts.isLoading;
          final applyEnabled = canApply(
            statusForCurrentRoot,
            library,
            gameRoot != null,
            status.busy,
            studioActiveForCurrentRoot,
            statusError: status.gameRoot == gameRoot ? status.error : null,
          );

          return StatusDetailsDialog(
            state: status,
            currentRoot: gameRoot,
            library: library,
            operationsBusy: operationsBusy,
            showManagedFiles: dialogRef.watch(advancedDetailsProvider),
            applyEnabled:
                applyEnabled && !preflight.busy && !conflicts.isLoading,
            deploymentRecoveryGeneration: _deploymentRecoveryGeneration(
              preflight,
              preflightRoot,
            ),
          );
        },
      ),
    );
    if (!context.mounted) return;

    final action = result?.action;
    final expectedRoot = result?.rootAtClick;
    final expectedRecoveryGeneration =
        result?.deploymentRecoveryGenerationAtClick;
    switch (action) {
      case StatusDetailsAction.apply:
        if (expectedRoot != null) {
          await _apply(context, ref, expectedRoot);
        }
      case StatusDetailsAction.refresh:
        if (expectedRoot != null) {
          await _refreshAll(ref, expectedRoot: expectedRoot);
        }
      case StatusDetailsAction.recover:
        if (expectedRoot != null && expectedRecoveryGeneration != null) {
          await _recoverDeployment(
            context,
            ref,
            expectedRoot,
            expectedRecoveryGeneration,
          );
        }
      case StatusDetailsAction.takeOver:
        if (expectedRoot != null) {
          await _promptTakeOver(context, ref, expectedRoot);
        }
      case StatusDetailsAction.settings:
        _openSettings(context);
      case StatusDetailsAction.close || null:
        break;
    }

    if (!context.mounted || action == StatusDetailsAction.settings) return;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (context.mounted) statusDetailsFocusNode.requestFocus();
    });
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final media = MediaQuery.of(context);
    final gamePath = ref.watch(gameExePathProvider);
    final gameRoot = gameRootFromExe(gamePath);
    final status = ref.watch(statusProvider);
    final preflight = ref.watch(preflightProvider);
    final library = ref.watch(libraryProvider);
    final conflicts = ref.watch(conflictsProvider);
    final conflictCount = conflicts.value?.length ?? 0;
    final operationsBusy =
        library.busy || status.busy || preflight.busy || conflicts.isLoading;
    final libraryMutationsBlocked =
        importRequestActive || operationsBusy || !library.authoritative;
    final compactConflictPanel =
        media.size.height < 560 &&
        (media.size.width < 784 || media.textScaler.scale(1) > 1.35);
    final conflictPanelHeight = compactConflictPanel ? 72.0 : 240.0;

    // Apply is enabled when the enabled loadout differs from what's deployed —
    // including the first-ever deploy (nothing_deployed + >=1 enabled mod).
    final statusForCurrentRoot = status.statusRoot == gameRoot
        ? status.status
        : null;
    final studioActiveForCurrentRoot =
        status.gameRoot == gameRoot && status.studioActive;
    final applyEnabled =
        canApply(
          statusForCurrentRoot,
          library,
          gameRoot != null,
          status.busy,
          studioActiveForCurrentRoot,
          statusError: status.gameRoot == gameRoot ? status.error : null,
        ) &&
        !preflight.busy &&
        !conflicts.isLoading;

    final importAction = MenuAnchor(
      builder: (ctx, controller, _) => OutlinedButton.icon(
        key: const ValueKey('import-mod-action'),
        focusNode: importFocusNode,
        onPressed: libraryMutationsBlocked
            ? null
            : () => controller.isOpen ? controller.close() : controller.open(),
        icon: const Icon(Icons.add),
        label: Text(
          l10n.actionImport,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
      ),
      menuChildren: [
        MenuItemButton(
          key: const ValueKey('import-folder-action'),
          leadingIcon: const Icon(Icons.folder_open),
          onPressed: libraryMutationsBlocked
              ? null
              : () => unawaited(importFolder()),
          child: Text(l10n.importFolder),
        ),
        MenuItemButton(
          key: const ValueKey('import-file-action'),
          leadingIcon: const Icon(Icons.insert_drive_file_outlined),
          onPressed: libraryMutationsBlocked
              ? null
              : () => unawaited(importFile()),
          child: Text(l10n.importFile),
        ),
      ],
    );
    final applyAction = Tooltip(
      message: l10n.applyTooltip,
      child: FilledButton.icon(
        key: const ValueKey('apply-loadout-action'),
        onPressed: applyEnabled ? () => _apply(context, ref, gameRoot!) : null,
        icon: const Icon(Icons.install_desktop),
        label: Text(
          l10n.actionApply,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
      ),
    );
    final statusAction = _StatusChip(
      state: status,
      currentRoot: gameRoot,
      focusNode: statusDetailsFocusNode,
      onPressed: () => _showStatusDetails(context, ref),
    );
    final progress = status.busy || preflight.busy || conflicts.isLoading
        ? const SizedBox(
            key: ValueKey('manager-operation-progress'),
            width: 16,
            height: 16,
            child: CircularProgressIndicator(strokeWidth: 2),
          )
        : null;
    final overflowAction = PopupMenuButton<_OverflowSelection>(
      key: const ValueKey('manager-overflow-action'),
      enabled: !operationsBusy,
      onSelected: (selection) => switch (selection.action) {
        _OverflowAction.refresh => _refreshAll(
          ref,
          expectedRoot: selection.rootAtMenuBuild,
        ),
        _OverflowAction.undeployAll => _undeployAll(
          context,
          ref,
          selection.rootAtMenuBuild,
        ),
      },
      itemBuilder: (ctx) {
        final rootAtMenuBuild = gameRoot;
        return [
          PopupMenuItem(
            value: (
              action: _OverflowAction.refresh,
              rootAtMenuBuild: rootAtMenuBuild,
            ),
            child: Text(l10n.refreshAction),
          ),
          PopupMenuItem(
            value: (
              action: _OverflowAction.undeployAll,
              rootAtMenuBuild: rootAtMenuBuild,
            ),
            enabled: rootAtMenuBuild != null,
            child: Text(l10n.undeployAllAction),
          ),
        ];
      },
    );

    return Column(
      children: [
        // --- Action bar -------------------------------------------------
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
          child: LayoutBuilder(
            builder: (context, constraints) {
              final textScale = MediaQuery.textScalerOf(context).scale(1);
              final compact = constraints.maxWidth < 760 || textScale > 1.35;
              if (!compact) {
                return Row(
                  children: [
                    importAction,
                    const SizedBox(width: 8),
                    applyAction,
                    const SizedBox(width: 12),
                    statusAction,
                    if (progress != null) ...[
                      const SizedBox(width: 12),
                      progress,
                    ],
                    const Spacer(),
                    overflowAction,
                  ],
                );
              }
              return Column(
                key: const ValueKey('compact-manager-action-bar'),
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: [importAction, applyAction],
                  ),
                  const SizedBox(height: 8),
                  Row(
                    children: [
                      Expanded(
                        child: Align(
                          alignment: AlignmentDirectional.centerStart,
                          child: statusAction,
                        ),
                      ),
                      if (progress != null) ...[
                        const SizedBox(width: 8),
                        progress,
                      ],
                      const SizedBox(width: 4),
                      overflowAction,
                    ],
                  ),
                ],
              );
            },
          ),
        ),

        // Game-path hint / apply-report / errors banner.
        _InfoBanner(
          status: status,
          preflight: preflight,
          refreshAll: () => _refreshAll(ref, expectedRoot: gameRoot),
          libraryRefreshFocusNode: libraryRefreshFocusNode,
          queueRefreshFocus: queueRefreshFocus,
          openSettings: () => _openSettings(context),
          openStatusDetails: () => _showStatusDetails(context, ref),
          preflightRetryFocusNode: preflightRetryFocusNode,
          preflightInstallMutationWaitFocusNode:
              preflightInstallMutationWaitFocusNode,
          preflightInstallRecoveryFocusNode: preflightInstallRecoveryFocusNode,
          preflightManagerMutationRecoveryFocusNode:
              preflightManagerMutationRecoveryFocusNode,
          rememberPreflightRetryFocus: rememberPreflightRetryFocus,
          rememberPreflightInstallRecoveryFocus:
              rememberPreflightInstallRecoveryFocus,
          retryPreflightAndWait: retryPreflightAndWait,
        ),

        const Divider(height: 1),

        // --- List | detail ---------------------------------------------
        Expanded(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Expanded(child: ModList()),
              const VerticalDivider(width: 1),
              SizedBox(
                width: 380,
                child: DetailPanel(
                  queueImportFocusAfterRemove: queueImportFocusAfterRemove,
                  queueRefreshFocusAfterRemove: queueRefreshFocusAfterRemove,
                ),
              ),
            ],
          ),
        ),

        // --- Conflicts (collapsible) -----------------------------------
        Material(
          color: theme.colorScheme.surfaceContainerLowest,
          child: ExpansionTile(
            title: Text(
              library.authoritative
                  ? l10n.conflictsTitle(conflictCount)
                  : l10n.conflictsUnverified,
            ),
            leading: const Icon(Icons.merge_type),
            children: [
              SizedBox(
                height: conflictPanelHeight,
                child: const ConflictPanel(),
              ),
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
///  * always disabled without a game path, with unknown/errored library state,
///    or while an FFI call is in flight — including a library mutation
///    (toggle/reorder persists the loadout via
///    `mgr_set_loadout` asynchronously; applying before that settles would let
///    `mgr_apply` read a stale on-disk loadout), so [LibraryState.busy] gates
///    Apply too;
///  * enabled for [ManagerStatusChangesPending] / [ManagerStatusGameUpdated]
///    (deployed drifted from target);
///  * enabled for [ManagerStatusNothingDeployed] only when the loadout has at
///    least one enabled mod (there is something to deploy) — this is what makes
///    the first-ever deploy, and the post-studio-take-over deploy, reachable;
///  * disabled while [studioActive] — a prior apply was blocked by an active
///    studio deployment; every apply fails until it's taken over, and the
///    status may not have caught up yet (that path shows take-over, not Apply);
///  * disabled for [ManagerStatusRecoveryRequired] until the recovery chip's
///    undeploy action completes;
///  * disabled otherwise: [ManagerStatusInSync] (nothing changed),
///    [ManagerStatusStudioDeployActive] (that path shows take-over, not Apply),
///    an unknown/null status, or NothingDeployed with zero enabled mods.
bool canApply(
  ManagerStatusView? status,
  LibraryState library,
  bool gameRootSet,
  bool busy,
  bool studioActive, {
  String? statusError,
}) {
  if (!gameRootSet ||
      busy ||
      library.busy ||
      !library.authoritative ||
      library.error != null ||
      statusError != null ||
      studioActive) {
    return false;
  }
  return switch (status) {
    ManagerStatusChangesPending() => true,
    ManagerStatusGameUpdated() => true,
    ManagerStatusNothingDeployed() => library.loadout.entries.any(
      (e) => e.enabled,
    ),
    ManagerStatusRecoveryRequired() => false,
    _ => false,
  };
}

/// True when two loadouts hold the same entries in the same order (by id +
/// enabled). The status-refresh listener uses this so a pure busy/error flip on
/// [LibraryState] doesn't count as a loadout change.
bool _sameLoadoutEntries(LoadoutView a, LoadoutView b) {
  if (a.entries.length != b.entries.length) return false;
  for (var i = 0; i < a.entries.length; i++) {
    if (a.entries[i].id != b.entries[i].id ||
        a.entries[i].enabled != b.entries[i].enabled) {
      return false;
    }
  }
  return true;
}

enum _OverflowAction { refresh, undeployAll }

typedef _OverflowSelection = ({
  _OverflowAction action,
  String? rootAtMenuBuild,
});

/// Stable, keyboard-focusable deployment-status trigger.
class _StatusChip extends StatelessWidget {
  const _StatusChip({
    required this.state,
    required this.currentRoot,
    required this.focusNode,
    required this.onPressed,
  });

  final StatusState state;
  final String? currentRoot;
  final FocusNode focusNode;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final scheme = Theme.of(context).colorScheme;
    // Even before the path-change listener starts a refresh, never display
    // authority that belongs to another game installation.
    final rootMatches = state.statusRoot == currentRoot;
    final status = rootMatches ? state.status : null;

    // studioActive (from a blocked apply) also surfaces the studio chip even
    // if mgr_status hasn't been re-read as studio_deploy_active yet.
    final isStudio =
        status is ManagerStatusStudioDeployActive ||
        (state.gameRoot == currentRoot && state.studioActive);

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
      ManagerStatusRecoveryRequired() => (
        l10n.statusRecoveryRequired,
        scheme.errorContainer,
        scheme.onErrorContainer,
        Icons.warning_amber_rounded,
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
        l10n.statusUnknown,
        scheme.surfaceContainerHighest,
        scheme.onSurfaceVariant,
        Icons.help_outline,
      ),
    };

    final semanticsLabel = l10n.statusDetailsOpen(label);
    return Semantics(
      key: const ValueKey('status-details-semantics'),
      container: true,
      liveRegion: true,
      button: true,
      enabled: true,
      label: semanticsLabel,
      onTap: onPressed,
      excludeSemantics: true,
      child: Tooltip(
        message: semanticsLabel,
        child: TextButton.icon(
          key: const ValueKey('status-details-trigger'),
          focusNode: focusNode,
          onPressed: onPressed,
          style: TextButton.styleFrom(
            backgroundColor: bg,
            foregroundColor: fg,
            minimumSize: const Size(0, 40),
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
            shape: const StadiumBorder(),
          ),
          icon: Icon(icon, size: 16),
          label: Text(label, maxLines: 1, overflow: TextOverflow.ellipsis),
        ),
      ),
    );
  }
}

/// Contextual banner beneath the action bar: prompts to set the game path,
/// echoes the last apply report + warnings, or surfaces an error.
class _InfoBanner extends ConsumerWidget {
  const _InfoBanner({
    required this.status,
    required this.preflight,
    required this.refreshAll,
    required this.libraryRefreshFocusNode,
    required this.queueRefreshFocus,
    required this.openSettings,
    required this.openStatusDetails,
    required this.preflightRetryFocusNode,
    required this.preflightInstallMutationWaitFocusNode,
    required this.preflightInstallRecoveryFocusNode,
    required this.preflightManagerMutationRecoveryFocusNode,
    required this.rememberPreflightRetryFocus,
    required this.rememberPreflightInstallRecoveryFocus,
    required this.retryPreflightAndWait,
  });

  final StatusState status;
  final PreflightState preflight;
  final Future<void> Function() refreshAll;
  final FocusNode libraryRefreshFocusNode;
  final VoidCallback queueRefreshFocus;
  final VoidCallback openSettings;
  final VoidCallback openStatusDetails;
  final FocusNode preflightRetryFocusNode;
  final FocusNode preflightInstallMutationWaitFocusNode;
  final FocusNode preflightInstallRecoveryFocusNode;
  final FocusNode preflightManagerMutationRecoveryFocusNode;
  final void Function(String? root, int generation) rememberPreflightRetryFocus;
  final void Function(
    String? root,
    int generation,
    bool restoreFocus,
    PreflightActionKind action,
    bool fallbackToStatus,
    String? actionToken,
  )
  rememberPreflightInstallRecoveryFocus;
  final Future<void> Function({
    required String expectedRoot,
    required bool restoreFocus,
    required PreflightActionKind expectedAction,
    required String expectedToken,
  })
  retryPreflightAndWait;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final library = ref.watch(libraryProvider);
    final conflicts = ref.watch(conflictsProvider);
    final refreshBlocked =
        library.busy || status.busy || preflight.busy || conflicts.isLoading;

    final children = <Widget>[];

    if (preflight.candidateRoot == null) {
      children.add(
        _line(
          theme,
          Icons.info_outline,
          l10n.errorSetGamePath,
          theme.colorScheme.onSurfaceVariant,
          key: const ValueKey('preflight-no-game-path'),
          maxLines: 2,
          action: TextButton.icon(
            key: const ValueKey('preflight-settings-action'),
            onPressed: openSettings,
            icon: const Icon(Icons.settings_outlined),
            label: Text(l10n.statusDetailsOpenSettings),
          ),
        ),
      );
    }

    final finding = preflight.authoritative
        ? preflight.report?.primarySetupFinding
        : null;
    final preflightUnavailable =
        preflight.candidateRoot != null && preflight.error != null;
    if (preflightUnavailable) {
      children.add(
        _line(
          theme,
          Icons.error_outline,
          l10n.preflightUnavailable,
          theme.colorScheme.error,
          key: const ValueKey('preflight-unavailable'),
          liveRegion: true,
          maxLines: 2,
          action: _technicalDetailsAndAction(
            preflight.error!,
            const ValueKey('preflight-technical-details-action'),
            _preflightRetryAction(l10n, ref, refreshBlocked),
          ),
        ),
      );
    } else if (finding != null) {
      final isProblem = finding.state == PreflightStateKind.problem;
      final message = _preflightFindingMessage(l10n, finding);
      children.add(
        _line(
          theme,
          isProblem ? Icons.error_outline : Icons.warning_amber_rounded,
          message,
          isProblem ? theme.colorScheme.error : Colors.amber.shade800,
          key: const ValueKey('preflight-setup-finding'),
          liveRegion: true,
          maxLines: 2,
          action: _technicalDetailsAndAction(
            finding.detail,
            const ValueKey('preflight-technical-details-action'),
            _preflightAction(
              context,
              l10n,
              ref,
              preflight,
              finding,
              refreshBlocked,
            ),
          ),
        ),
      );
    }

    // Errors: a "no game path" sentinel maps to the friendly hint (already
    // shown above). Other failures stay friendly here; native detail remains
    // available in the status dialog.
    if (!preflightUnavailable &&
        finding == null &&
        status.error != null &&
        status.error != StatusNotifier.noGamePath) {
      children.add(
        _line(
          theme,
          Icons.error_outline,
          l10n.managerOperationFailed,
          theme.colorScheme.error,
          action: TextButton.icon(
            key: const ValueKey('status-error-details-action'),
            onPressed: openStatusDetails,
            icon: const Icon(Icons.fact_check_outlined),
            label: Text(l10n.preflightReviewStatus),
          ),
        ),
      );
    }
    if (library.error != null) {
      children.add(
        _line(
          theme,
          Icons.error_outline,
          library.authoritative
              ? l10n.libraryOperationFailed
              : l10n.libraryStateUnknown,
          theme.colorScheme.error,
          action: _technicalDetailsAndAction(
            library.error!,
            const ValueKey('library-technical-details-action'),
            TextButton.icon(
              key: const ValueKey('library-refresh-action'),
              focusNode: libraryRefreshFocusNode,
              onPressed: refreshBlocked
                  ? null
                  : () async {
                      if (ref.read(libraryProvider).busy ||
                          ref.read(statusProvider).busy ||
                          ref.read(preflightProvider).busy ||
                          ref.read(conflictsProvider).isLoading) {
                        return;
                      }
                      await ref.read(libraryProvider.notifier).refresh();
                      if (!context.mounted) return;
                      final refreshed = ref.read(libraryProvider);
                      if (refreshed.error != null) {
                        queueRefreshFocus();
                      }
                    },
              icon: const Icon(Icons.refresh),
              label: Text(l10n.refreshAction),
            ),
          ),
        ),
      );
    }

    final report = status.lastReport;
    if (report != null) {
      final hasWarnings = report.warnings.isNotEmpty;
      children.add(
        _line(
          theme,
          hasWarnings
              ? Icons.warning_amber_rounded
              : Icons.check_circle_outline,
          hasWarnings
              ? l10n.applyReportAppliedWithWarnings(
                  report.applied.length,
                  report.warnings.length,
                )
              : l10n.applyReportApplied(report.applied.length),
          hasWarnings
              ? Colors.amber.shade800
              : theme.colorScheme.onSurfaceVariant,
          action: hasWarnings
              ? IconButton(
                  key: const ValueKey('apply-warning-details-action'),
                  onPressed: openStatusDetails,
                  tooltip: l10n.preflightReviewStatus,
                  icon: const Icon(Icons.fact_check_outlined),
                )
              : null,
        ),
      );
    }

    if (children.isEmpty) return const SizedBox.shrink();
    return Container(
      width: double.infinity,
      color: theme.colorScheme.surfaceContainerLowest,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: children,
      ),
    );
  }

  Widget _preflightAction(
    BuildContext context,
    AppLocalizations l10n,
    WidgetRef ref,
    PreflightState preflight,
    PreflightCheckView finding,
    bool blocked,
  ) {
    return switch (finding.action) {
      PreflightActionKind.selectGameRoot => TextButton.icon(
        key: const ValueKey('preflight-settings-action'),
        onPressed: blocked ? null : openSettings,
        icon: const Icon(Icons.settings_outlined),
        label: Text(l10n.statusDetailsOpenSettings),
      ),
      PreflightActionKind.waitForInstallMutation ||
      PreflightActionKind.recoverInstall => TextButton.icon(
        key: const ValueKey('preflight-install-recovery-action'),
        focusNode: finding.action == PreflightActionKind.waitForInstallMutation
            ? preflightInstallMutationWaitFocusNode
            : preflightInstallRecoveryFocusNode,
        onPressed: blocked
            ? null
            : () => _showInstallRecoveryGuide(
                context,
                ref,
                finding,
                expectedRoot: preflight.candidateRoot,
                expectedGeneration: preflight.generation,
                expectedAction: finding.action!,
              ),
        icon: const Icon(Icons.health_and_safety_outlined),
        label: Text(l10n.preflightReviewRecovery),
      ),
      PreflightActionKind.recoverManagerMutation => TextButton.icon(
        key: const ValueKey('preflight-manager-recovery-action'),
        focusNode: preflightManagerMutationRecoveryFocusNode,
        onPressed: blocked
            ? null
            : () => _recoverManagerMutation(
                context,
                ref,
                finding,
                expectedRoot: preflight.candidateRoot,
                expectedGeneration: preflight.generation,
                expectedAction: finding.action!,
                expectedToken: finding.actionToken!,
              ),
        icon: const Icon(Icons.restore_outlined),
        label: Text(l10n.recoveryAction),
      ),
      PreflightActionKind.closeGame => _preflightFullRefreshAction(
        l10n,
        ref,
        blocked,
      ),
      PreflightActionKind.recoverDeployment ||
      PreflightActionKind.removeStudioDeployment ||
      PreflightActionKind.reviewApply ||
      PreflightActionKind.reviewReapply ||
      PreflightActionKind.inspectDeployment ||
      PreflightActionKind.runFullStatus => TextButton.icon(
        key: const ValueKey('preflight-status-action'),
        onPressed: blocked ? null : openStatusDetails,
        icon: const Icon(Icons.fact_check_outlined),
        label: Text(l10n.preflightReviewStatus),
      ),
      _ => _preflightRetryAction(l10n, ref, blocked),
    };
  }

  Future<void> _showInstallRecoveryGuide(
    BuildContext context,
    WidgetRef ref,
    PreflightCheckView finding, {
    required String? expectedRoot,
    required int expectedGeneration,
    required PreflightActionKind expectedAction,
  }) async {
    final snapshot = ref.read(preflightProvider);
    final snapshotFinding = snapshot.authoritative
        ? snapshot.report?.primarySetupFinding
        : null;
    if (expectedRoot == null ||
        snapshot.candidateRoot != expectedRoot ||
        snapshot.generation != expectedGeneration ||
        !snapshot.authoritative ||
        !identical(snapshotFinding, finding) ||
        snapshotFinding?.action != expectedAction ||
        !_preflightFindingUsesInstallRecoveryGuide(finding) ||
        ref.read(libraryProvider).busy ||
        ref.read(statusProvider).busy ||
        snapshot.busy ||
        ref.read(conflictsProvider).isLoading) {
      return;
    }
    final restoreFocus = switch (expectedAction) {
      PreflightActionKind.waitForInstallMutation =>
        preflightInstallMutationWaitFocusNode.hasFocus,
      PreflightActionKind.recoverInstall =>
        preflightInstallRecoveryFocusNode.hasFocus,
      _ => false,
    };
    final l10n = AppLocalizations.of(context);
    final retry = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        key: const ValueKey('preflight-install-recovery-dialog'),
        title: Text(l10n.installRecoveryTitle),
        content: ConstrainedBox(
          constraints: BoxConstraints(
            maxWidth: 560,
            maxHeight: MediaQuery.sizeOf(dialogContext).height * 0.45,
          ),
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(l10n.installRecoveryBody),
                const SizedBox(height: 12),
                Text(l10n.installRecoverySteps),
                if (finding.items.isNotEmpty) ...[
                  const SizedBox(height: 16),
                  Text(
                    l10n.installRecoveryEvidence,
                    style: Theme.of(dialogContext).textTheme.titleSmall,
                  ),
                  const SizedBox(height: 8),
                  for (var index = 0; index < finding.items.length; index++)
                    Padding(
                      padding: const EdgeInsets.only(bottom: 6),
                      child: SelectableText(
                        _boundedDiagnostic(finding.items[index]),
                        key: ValueKey(
                          'preflight-install-recovery-evidence-$index',
                        ),
                      ),
                    ),
                ],
              ],
            ),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialogContext, false),
            child: Text(l10n.commonCancel),
          ),
          FilledButton(
            key: const ValueKey('preflight-install-recovery-retry'),
            onPressed: () => Navigator.pop(dialogContext, true),
            child: Text(l10n.preflightRetry),
          ),
        ],
      ),
    );
    if (retry != true || !context.mounted) return;

    final current = ref.read(preflightProvider);
    final currentFinding = current.authoritative
        ? current.report?.primarySetupFinding
        : null;
    if (current.candidateRoot != expectedRoot ||
        current.generation != expectedGeneration ||
        !identical(currentFinding, finding) ||
        currentFinding?.action != expectedAction ||
        ref.read(libraryProvider).busy ||
        ref.read(statusProvider).busy ||
        current.busy ||
        ref.read(conflictsProvider).isLoading) {
      return;
    }
    rememberPreflightInstallRecoveryFocus(
      expectedRoot,
      current.generation + 1,
      restoreFocus,
      expectedAction,
      false,
      null,
    );
    ref.read(preflightProvider.notifier).retry();
  }

  Future<void> _recoverManagerMutation(
    BuildContext context,
    WidgetRef ref,
    PreflightCheckView finding, {
    required String? expectedRoot,
    required int expectedGeneration,
    required PreflightActionKind expectedAction,
    required String expectedToken,
  }) async {
    final snapshot = ref.read(preflightProvider);
    final snapshotFinding = snapshot.authoritative
        ? snapshot.report?.primarySetupFinding
        : null;
    if (expectedRoot == null ||
        diagnosticGameRootCandidate(ref.read(gameExePathProvider)) !=
            expectedRoot ||
        snapshot.candidateRoot != expectedRoot ||
        snapshot.generation != expectedGeneration ||
        !snapshot.authoritative ||
        !identical(snapshotFinding, finding) ||
        snapshotFinding?.action != expectedAction ||
        expectedAction != PreflightActionKind.recoverManagerMutation ||
        snapshotFinding?.actionToken != expectedToken ||
        ref.read(libraryProvider).busy ||
        ref.read(statusProvider).busy ||
        snapshot.busy ||
        ref.read(conflictsProvider).isLoading) {
      return;
    }

    final restoreFocus = preflightManagerMutationRecoveryFocusNode.hasFocus;
    final l10n = AppLocalizations.of(context);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        key: const ValueKey('preflight-manager-recovery-dialog'),
        scrollable: true,
        title: Text(l10n.managerRecoveryTitle),
        content: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: Text(l10n.managerRecoveryConfirm),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialogContext, false),
            child: Text(l10n.commonCancel),
          ),
          FilledButton(
            key: const ValueKey('preflight-manager-recovery-confirm'),
            onPressed: () => Navigator.pop(dialogContext, true),
            child: Text(l10n.recoveryAction),
          ),
        ],
      ),
    );
    if (confirmed != true || !context.mounted) return;

    final current = ref.read(preflightProvider);
    final currentFinding = current.authoritative
        ? current.report?.primarySetupFinding
        : null;
    if (diagnosticGameRootCandidate(ref.read(gameExePathProvider)) !=
            expectedRoot ||
        current.candidateRoot != expectedRoot ||
        current.generation != expectedGeneration ||
        !current.authoritative ||
        !identical(currentFinding, finding) ||
        currentFinding?.action != expectedAction ||
        currentFinding?.actionToken != expectedToken ||
        ref.read(libraryProvider).busy ||
        ref.read(statusProvider).busy ||
        current.busy ||
        ref.read(conflictsProvider).isLoading) {
      return;
    }

    final outcome = await ref
        .read(statusProvider.notifier)
        .recoverInstall(expectedRoot, expectedToken);
    if (!context.mounted) return;
    final recoveryError = outcome == null
        ? ref.read(statusProvider).error
        : null;
    // Recovery can settle at several valid endpoints. Reload every Manager
    // projection from Native before reporting the structured outcome.
    await ref.read(libraryProvider.notifier).refresh();
    if (!context.mounted) return;
    final selectedCandidate = diagnosticGameRootCandidate(
      ref.read(gameExePathProvider),
    );
    final selectedRoot = selectedCandidate == expectedRoot
        ? expectedRoot
        : gameRootFromExe(ref.read(gameExePathProvider));
    await ref.read(statusProvider.notifier).refresh(selectedRoot);
    if (!context.mounted) return;

    ref.invalidate(conflictsProvider);
    final latestPreflight = ref.read(preflightProvider);
    Future<void>? preflightRefresh;
    if (latestPreflight.candidateRoot != null) {
      preflightRefresh = retryPreflightAndWait(
        expectedRoot: latestPreflight.candidateRoot!,
        restoreFocus:
            latestPreflight.candidateRoot == expectedRoot && restoreFocus,
        expectedAction: expectedAction,
        expectedToken: expectedToken,
      );
    }
    try {
      await Future.wait<void>([
        ?preflightRefresh,
        ref.read(conflictsProvider.future).then((_) {}),
      ]);
    } on Object {
      // Each provider retains its own error state; both reads still settle
      // before the structured recovery outcome is presented.
    }
    if (!context.mounted ||
        diagnosticGameRootCandidate(ref.read(gameExePathProvider)) !=
            expectedRoot) {
      return;
    }
    final message = outcome == null
        ? l10n.managerRecoveryFailed
        : _managerRecoveryMessage(l10n, outcome);
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(
        SnackBar(
          content: Text(message),
          action: recoveryError == null
              ? null
              : SnackBarAction(
                  key: const ValueKey(
                    'manager-recovery-technical-details-action',
                  ),
                  label: l10n.coreTechnicalDetails,
                  onPressed: () =>
                      showTechnicalDetailsDialog(context, recoveryError),
                ),
        ),
      );
  }

  String _managerRecoveryMessage(
    AppLocalizations l10n,
    MgrInstallRecoveryOutcome outcome,
  ) => switch (outcome) {
    MgrInstallRecoveryOutcome.alreadyClean => l10n.managerRecoveryAlreadyClean,
    MgrInstallRecoveryOutcome.busy => l10n.managerRecoveryBusy,
    MgrInstallRecoveryOutcome.preMutationLockCleared =>
      l10n.managerRecoveryLockCleared,
    MgrInstallRecoveryOutcome.recoveredToPristine =>
      l10n.managerRecoveryRestoredPristine,
    MgrInstallRecoveryOutcome.completedApplyPreserved =>
      l10n.managerRecoveryApplyPreserved,
    MgrInstallRecoveryOutcome.completedUndeployConfirmed =>
      l10n.managerRecoveryUndeployConfirmed,
    MgrInstallRecoveryOutcome.compileRecoveryRequired =>
      l10n.managerRecoveryCompileRequired,
    MgrInstallRecoveryOutcome.inspectionFailed =>
      l10n.managerRecoveryInspectionFailed,
  };

  Widget _preflightRetryAction(
    AppLocalizations l10n,
    WidgetRef ref,
    bool blocked,
  ) {
    return TextButton.icon(
      key: const ValueKey('preflight-retry-action'),
      focusNode: preflightRetryFocusNode,
      onPressed: blocked
          ? null
          : () {
              if (ref.read(libraryProvider).busy ||
                  ref.read(statusProvider).busy ||
                  ref.read(preflightProvider).busy ||
                  ref.read(conflictsProvider).isLoading) {
                return;
              }
              final current = ref.read(preflightProvider);
              rememberPreflightRetryFocus(
                current.candidateRoot,
                current.generation + 1,
              );
              ref.read(preflightProvider.notifier).retry();
            },
      icon: const Icon(Icons.refresh),
      label: Text(l10n.preflightRetry),
    );
  }

  Widget _preflightFullRefreshAction(
    AppLocalizations l10n,
    WidgetRef ref,
    bool blocked,
  ) {
    return TextButton.icon(
      key: const ValueKey('preflight-retry-action'),
      focusNode: preflightRetryFocusNode,
      onPressed: blocked
          ? null
          : () async {
              if (ref.read(libraryProvider).busy ||
                  ref.read(statusProvider).busy ||
                  ref.read(preflightProvider).busy ||
                  ref.read(conflictsProvider).isLoading) {
                return;
              }
              final current = ref.read(preflightProvider);
              rememberPreflightRetryFocus(
                current.candidateRoot,
                current.generation + 1,
              );
              await refreshAll();
            },
      icon: const Icon(Icons.refresh),
      label: Text(l10n.preflightRetry),
    );
  }

  String _preflightFindingMessage(
    AppLocalizations l10n,
    PreflightCheckView finding,
  ) => switch (finding.code) {
    'game_process_running' => l10n.preflightGameRunning,
    _ => l10n.preflightAttention,
  };

  String _boundedDiagnostic(String raw) {
    final bounded = boundedDiagnosticText(raw, 512);
    return switch (bounded.value) {
      final value? => '$value${bounded.truncated ? '…' : ''}',
      null => '—',
    };
  }

  Widget _technicalDetailsAndAction(
    String detail,
    Key detailsKey,
    Widget primaryAction,
  ) => Row(
    mainAxisSize: MainAxisSize.min,
    children: [
      TechnicalDetailsIconButton(key: detailsKey, detail: detail),
      primaryAction,
    ],
  );

  Widget _line(
    ThemeData theme,
    IconData icon,
    String text,
    Color color, {
    Key? key,
    Widget? action,
    bool liveRegion = false,
    int? maxLines,
  }) {
    final line = Padding(
      key: key,
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(icon, size: 16, color: color),
          const SizedBox(width: 8),
          Expanded(
            child: Tooltip(
              message: text,
              child: Text(
                text,
                maxLines: maxLines,
                overflow: maxLines == null ? null : TextOverflow.ellipsis,
                style: theme.textTheme.bodySmall?.copyWith(color: color),
              ),
            ),
          ),
          if (action != null) ...[const SizedBox(width: 8), action],
        ],
      ),
    );
    if (!liveRegion) return line;
    return Semantics(container: true, liveRegion: true, child: line);
  }
}
