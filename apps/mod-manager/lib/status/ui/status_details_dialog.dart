import 'package:flutter/material.dart';

import '../../l10n/app_localizations.dart';
import '../../library/domain/library_notifier.dart';
import '../../library/domain/models.dart';
import '../domain/status_notifier.dart';

enum StatusDetailsAction { apply, refresh, recover, takeOver, settings, close }

typedef StatusDetailsResult = ({
  StatusDetailsAction action,
  String? rootAtClick,
});

const _lazyListThreshold = 50;
const _lazyListHeight = 240.0;

/// Whether current status authority (or its fail-closed fallback evidence)
/// still says Studio owns [currentRoot]. A known non-Studio status always
/// clears the fallback at the presentation/action boundary too.
bool statusHasStudioOwnership(StatusState state, String? currentRoot) {
  if (currentRoot == null) return false;
  final status = state.statusRoot == currentRoot ? state.status : null;
  return switch (status) {
    ManagerStatusStudioDeployActive() => true,
    ManagerStatusUnknown() ||
    null => state.gameRoot == currentRoot && state.studioActive,
    _ => false,
  };
}

/// Actionable, root-bound detail view over the already parsed manager status.
///
/// This widget owns no deployment authority and performs no operation itself.
/// It returns a [StatusDetailsResult] to the Home page, including the root that
/// the user acted on. Home re-checks that root and the current providers before
/// calling the existing notifier lanes.
class StatusDetailsDialog extends StatelessWidget {
  const StatusDetailsDialog({
    required this.state,
    required this.currentRoot,
    required this.library,
    required this.operationsBusy,
    required this.applyEnabled,
    super.key,
  });

  final StatusState state;
  final String? currentRoot;
  final LibraryState library;
  final bool operationsBusy;
  final bool applyEnabled;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final scheme = Theme.of(context).colorScheme;
    final status = state.statusRoot == currentRoot ? state.status : null;
    final studioActive = statusHasStudioOwnership(state, currentRoot);
    final presentation = _presentation(l10n, scheme, status, studioActive);
    final modsById = <String, ModEntryMetaView>{
      for (final mod in library.mods) mod.id: mod,
    };

    final content = <Widget>[];
    if (currentRoot == null) {
      content.add(_paragraph(context, l10n.statusDetailsNoRoot));
    } else if (studioActive) {
      content.addAll(_studioContent(context, l10n, status));
    } else {
      content.addAll(switch (status) {
        ManagerStatusNothingDeployed() => [
          _paragraph(context, l10n.statusDetailsNoDeployment),
        ],
        ManagerStatusInSync(:final loadout) => [
          _paragraph(context, l10n.statusDetailsInSyncDescription),
          const SizedBox(height: 16),
          _loadoutSection(
            context,
            l10n,
            keyName: 'in-sync',
            title: l10n.statusDetailsDeployedLoadout,
            loadout: loadout,
            modsById: modsById,
          ),
        ],
        ManagerStatusChangesPending(:final deployed, :final target) => [
          _paragraph(context, l10n.statusDetailsChangesDescription),
          const SizedBox(height: 16),
          _loadoutSection(
            context,
            l10n,
            keyName: 'deployed',
            title: l10n.statusDetailsCurrentlyDeployed,
            loadout: deployed,
            modsById: modsById,
          ),
          const SizedBox(height: 16),
          _loadoutSection(
            context,
            l10n,
            keyName: 'target',
            title: l10n.statusDetailsAfterApply,
            loadout: target,
            modsById: modsById,
          ),
        ],
        ManagerStatusGameUpdated(:final drifted) => [
          _paragraph(context, l10n.statusDetailsGameUpdatedDescription),
          const SizedBox(height: 16),
          _stringListSection(
            context,
            keyName: 'drifted',
            title: l10n.statusDetailsDriftedFiles,
            values: drifted,
            emptyText: l10n.statusDetailsUnavailable,
            selectable: true,
          ),
        ],
        ManagerStatusRecoveryRequired() => [
          _paragraph(context, l10n.statusDetailsRecoveryDescription),
        ],
        _ => [_paragraph(context, l10n.statusDetailsUnknownDescription)],
      });
    }

    final rootBoundError = currentRoot != null && state.gameRoot == currentRoot
        ? state.error
        : null;
    final visibleError = rootBoundError == StatusNotifier.noGamePath
        ? null
        : rootBoundError;
    if (visibleError != null) {
      content.addAll([
        const SizedBox(height: 20),
        const Divider(),
        const SizedBox(height: 12),
        _sectionTitle(context, l10n.statusDetailsLastError),
        const SizedBox(height: 8),
        SelectionArea(
          child: Text(
            visibleError,
            key: const ValueKey('status-details-error'),
            style: TextStyle(color: scheme.error),
          ),
        ),
      ]);
    }

    final report = currentRoot != null && state.gameRoot == currentRoot
        ? state.lastReport
        : null;
    if (report != null) {
      content.addAll([
        const SizedBox(height: 20),
        const Divider(),
        const SizedBox(height: 12),
        _sectionTitle(context, l10n.statusDetailsLastApply),
        const SizedBox(height: 12),
        _stringListSection(
          context,
          keyName: 'applied',
          title: l10n.statusDetailsAppliedMods,
          values: report.applied,
          emptyText: l10n.applyReportApplied(0),
        ),
      ]);
      if (report.warnings.isNotEmpty) {
        content.addAll([
          const SizedBox(height: 16),
          _stringListSection(
            context,
            keyName: 'warnings',
            title: l10n.statusDetailsWarnings,
            values: report.warnings,
            emptyText: l10n.statusDetailsUnavailable,
            color: scheme.error,
          ),
        ]);
      }
    }

    return AlertDialog(
      key: const ValueKey('status-details-dialog'),
      scrollable: true,
      insetPadding: const EdgeInsets.all(24),
      title: Row(
        children: [
          Icon(presentation.icon, color: presentation.color),
          const SizedBox(width: 12),
          Expanded(
            child: Text(
              l10n.statusDetailsTitle(presentation.label),
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
      content: KeyedSubtree(
        key: const ValueKey('status-details-scroll-content'),
        child: SizedBox(
          width: 520,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: content,
          ),
        ),
      ),
      actions: _actions(context, l10n, status, studioActive, visibleError),
    );
  }

  List<Widget> _studioContent(
    BuildContext context,
    AppLocalizations l10n,
    ManagerStatusView? status,
  ) {
    final modName = status is ManagerStatusStudioDeployActive
        ? status.modName
        : '';
    return [
      _paragraph(context, l10n.statusDetailsStudioDescription),
      const SizedBox(height: 12),
      Text(
        modName.isEmpty
            ? l10n.statusDetailsStudioNameUnknown
            : l10n.statusDetailsStudioMod(modName),
        key: const ValueKey('status-details-studio-mod'),
        style: Theme.of(context).textTheme.titleSmall,
      ),
    ];
  }

  Widget _loadoutSection(
    BuildContext context,
    AppLocalizations l10n, {
    required String keyName,
    required String title,
    required LoadoutView? loadout,
    required Map<String, ModEntryMetaView> modsById,
  }) {
    final children = <Widget>[
      _sectionTitle(context, title),
      const SizedBox(height: 8),
    ];
    if (loadout == null) {
      children.add(Text(l10n.statusDetailsUnavailable));
    } else if (loadout.entries.isEmpty) {
      children.add(Text(l10n.statusDetailsEmptyLoadout));
    } else {
      final entries = loadout.entries;
      if (entries.length > _lazyListThreshold) {
        children.add(
          SizedBox(
            height: _lazyListHeight,
            child: ListView.builder(
              key: ValueKey('status-details-list-$keyName'),
              primary: false,
              itemCount: entries.length,
              itemBuilder: (context, index) => _loadoutRow(
                context,
                l10n,
                keyName: keyName,
                index: index,
                entry: entries[index],
                modsById: modsById,
              ),
            ),
          ),
        );
      } else {
        for (var index = 0; index < entries.length; index++) {
          children.add(
            _loadoutRow(
              context,
              l10n,
              keyName: keyName,
              index: index,
              entry: entries[index],
              modsById: modsById,
            ),
          );
        }
      }
    }
    return Column(
      key: ValueKey('status-details-section-$keyName'),
      crossAxisAlignment: CrossAxisAlignment.start,
      children: children,
    );
  }

  Widget _loadoutRow(
    BuildContext context,
    AppLocalizations l10n, {
    required String keyName,
    required int index,
    required LoadoutEntryView entry,
    required Map<String, ModEntryMetaView> modsById,
  }) {
    final mod = modsById[entry.id];
    final name = mod?.name.isNotEmpty == true ? mod!.name : entry.id;
    return Padding(
      key: ValueKey('status-loadout-$keyName-$index'),
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(width: 32, child: Text('${index + 1}.')),
          Expanded(child: Text(name)),
          if (!entry.enabled) ...[
            const SizedBox(width: 8),
            Text(
              l10n.modDisabledHint,
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ],
        ],
      ),
    );
  }

  Widget _stringListSection(
    BuildContext context, {
    required String keyName,
    required String title,
    required List<String> values,
    required String emptyText,
    bool selectable = false,
    Color? color,
  }) {
    final rows = values.length > _lazyListThreshold
        ? SizedBox(
            height: _lazyListHeight,
            child: ListView.builder(
              key: ValueKey('status-details-list-$keyName'),
              primary: false,
              itemCount: values.length,
              itemBuilder: (context, index) => _stringListRow(
                keyName: keyName,
                index: index,
                value: values[index],
                selectable: selectable,
                color: color,
              ),
            ),
          )
        : Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              for (var index = 0; index < values.length; index++)
                _stringListRow(
                  keyName: keyName,
                  index: index,
                  value: values[index],
                  selectable: selectable,
                  color: color,
                ),
            ],
          );
    return Column(
      key: ValueKey('status-details-section-$keyName'),
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _sectionTitle(context, title),
        const SizedBox(height: 8),
        if (values.isEmpty) Text(emptyText) else rows,
      ],
    );
  }

  Widget _stringListRow({
    required String keyName,
    required int index,
    required String value,
    required bool selectable,
    required Color? color,
  }) {
    final text = Text(value, style: TextStyle(color: color));
    return Padding(
      key: ValueKey('status-details-$keyName-$index'),
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text('-'),
          const SizedBox(width: 8),
          Expanded(child: selectable ? SelectionArea(child: text) : text),
        ],
      ),
    );
  }

  void _pop(BuildContext context, StatusDetailsAction action) {
    Navigator.pop<StatusDetailsResult>(context, (
      action: action,
      rootAtClick: currentRoot,
    ));
  }

  List<Widget> _actions(
    BuildContext context,
    AppLocalizations l10n,
    ManagerStatusView? status,
    bool studioActive,
    String? visibleError,
  ) {
    final actions = <Widget>[
      TextButton(
        key: const ValueKey('status-details-action-close'),
        onPressed: () => _pop(context, StatusDetailsAction.close),
        child: Text(l10n.commonOk),
      ),
    ];
    if (currentRoot == null) {
      actions.add(
        FilledButton.icon(
          key: const ValueKey('status-details-action-settings'),
          onPressed: () => _pop(context, StatusDetailsAction.settings),
          icon: const Icon(Icons.settings_outlined),
          label: Text(l10n.statusDetailsOpenSettings),
        ),
      );
      return actions;
    }

    final needsRefresh =
        status == null ||
        status is ManagerStatusUnknown ||
        visibleError != null ||
        !library.authoritative ||
        library.error != null;
    if (needsRefresh) {
      actions.add(
        TextButton.icon(
          key: const ValueKey('status-details-action-refresh'),
          onPressed: operationsBusy
              ? null
              : () => _pop(context, StatusDetailsAction.refresh),
          icon: const Icon(Icons.refresh),
          label: Text(l10n.refreshAction),
        ),
      );
    }

    if (studioActive) {
      actions.add(
        FilledButton.icon(
          key: const ValueKey('status-details-action-take-over'),
          onPressed: operationsBusy
              ? null
              : () => _pop(context, StatusDetailsAction.takeOver),
          icon: const Icon(Icons.lock_open_outlined),
          label: Text(l10n.takeOverAction),
        ),
      );
    } else if (status is ManagerStatusRecoveryRequired) {
      actions.add(
        FilledButton.icon(
          key: const ValueKey('status-details-action-recover'),
          onPressed: operationsBusy
              ? null
              : () => _pop(context, StatusDetailsAction.recover),
          icon: const Icon(Icons.restore),
          label: Text(l10n.recoveryAction),
        ),
      );
    } else if (status is ManagerStatusGameUpdated) {
      actions.add(
        FilledButton.icon(
          key: const ValueKey('status-details-action-reapply'),
          onPressed: applyEnabled
              ? () => _pop(context, StatusDetailsAction.apply)
              : null,
          icon: const Icon(Icons.replay),
          label: Text(l10n.statusDetailsReapply),
        ),
      );
    } else if (status is ManagerStatusChangesPending ||
        (status is ManagerStatusNothingDeployed && applyEnabled)) {
      actions.add(
        FilledButton.icon(
          key: const ValueKey('status-details-action-apply'),
          onPressed: applyEnabled
              ? () => _pop(context, StatusDetailsAction.apply)
              : null,
          icon: const Icon(Icons.play_arrow),
          label: Text(l10n.actionApply),
        ),
      );
    }
    return actions;
  }
}

({String label, IconData icon, Color color}) _presentation(
  AppLocalizations l10n,
  ColorScheme scheme,
  ManagerStatusView? status,
  bool studioActive,
) {
  if (studioActive) {
    return (
      label: l10n.statusStudioDeploy,
      icon: Icons.lock_outline,
      color: scheme.error,
    );
  }
  return switch (status) {
    ManagerStatusInSync() => (
      label: l10n.statusInSync,
      icon: Icons.check_circle_outline,
      color: scheme.primary,
    ),
    ManagerStatusChangesPending() => (
      label: l10n.statusChangesPending,
      icon: Icons.pending_outlined,
      color: scheme.tertiary,
    ),
    ManagerStatusGameUpdated() => (
      label: l10n.statusGameUpdated,
      icon: Icons.system_update_alt,
      color: scheme.error,
    ),
    ManagerStatusRecoveryRequired() => (
      label: l10n.statusRecoveryRequired,
      icon: Icons.warning_amber_rounded,
      color: scheme.error,
    ),
    ManagerStatusNothingDeployed() => (
      label: l10n.statusNothingDeployed,
      icon: Icons.circle_outlined,
      color: scheme.onSurfaceVariant,
    ),
    _ => (
      label: l10n.statusUnknown,
      icon: Icons.help_outline,
      color: scheme.onSurfaceVariant,
    ),
  };
}

Widget _paragraph(BuildContext context, String text) =>
    Text(text, style: Theme.of(context).textTheme.bodyMedium);

Widget _sectionTitle(BuildContext context, String text) =>
    Text(text, style: Theme.of(context).textTheme.titleSmall);
