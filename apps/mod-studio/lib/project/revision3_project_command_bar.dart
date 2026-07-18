import 'dart:async';

import 'package:flutter/material.dart';

typedef Revision3ProjectCommandCallback = FutureOr<void> Function();

enum Revision3ProjectCommandKind { undo, search, create, problems, settings }

/// One command exposed by [Revision3ProjectCommandBar].
///
/// A disabled command must carry its author-facing reason. This keeps command
/// gates truthful in both tooltips and accessibility output.
@immutable
final class Revision3ProjectCommand {
  const Revision3ProjectCommand.enabled(
    Revision3ProjectCommandCallback callback,
  ) : onInvoke = callback,
      disabledReason = null;

  const Revision3ProjectCommand.disabled(String disabledReason)
    : assert(disabledReason.length > 0),
      onInvoke = null,
      disabledReason = disabledReason;

  final Revision3ProjectCommandCallback? onInvoke;
  final String? disabledReason;

  bool get isEnabled => onInvoke != null;
}

/// A project-wide operation already owned by the host.
///
/// Supplying this state disables every command in the bar. [command] is
/// optional because the host operation can originate elsewhere in the app.
@immutable
final class Revision3ProjectCommandBarBusyState {
  const Revision3ProjectCommandBarBusyState({
    required this.label,
    required this.disabledReason,
    this.command,
  }) : assert(label.length > 0),
       assert(disabledReason.length > 0);

  final String label;
  final String disabledReason;
  final Revision3ProjectCommandKind? command;
}

/// Author-facing copy for [Revision3ProjectCommandBar].
@immutable
final class Revision3ProjectCommandBarCopy {
  const Revision3ProjectCommandBarCopy({
    this.currentSectionTemplate = 'Current section: {section}',
    this.orientationSemanticsTemplate =
        'Project {project}. Current section: {section}.',
    this.undoLabel = 'Undo',
    this.searchLabel = 'Search',
    this.createLabel = 'Create',
    this.problemsLabel = 'Problems',
    this.settingsLabel = 'Settings',
    this.moreActionsTooltip = 'More project actions',
    this.busyLabel = 'Finishing the current project action…',
    this.busyDisabledReason = 'Wait for the current project action to finish.',
  });

  static const german = Revision3ProjectCommandBarCopy(
    currentSectionTemplate: 'Aktueller Bereich: {section}',
    orientationSemanticsTemplate:
        'Projekt {project}. Aktueller Bereich: {section}.',
    undoLabel: 'R\u00fcckg\u00e4ngig',
    searchLabel: 'Suchen',
    createLabel: 'Erstellen',
    problemsLabel: 'Probleme',
    settingsLabel: 'Einstellungen',
    moreActionsTooltip: 'Weitere Projektaktionen',
    busyLabel: 'Die aktuelle Projektaktion wird abgeschlossen…',
    busyDisabledReason:
        'Warte, bis die aktuelle Projektaktion abgeschlossen ist.',
  );

  final String currentSectionTemplate;
  final String orientationSemanticsTemplate;
  final String undoLabel;
  final String searchLabel;
  final String createLabel;
  final String problemsLabel;
  final String settingsLabel;
  final String moreActionsTooltip;
  final String busyLabel;
  final String busyDisabledReason;

  String currentSection(String section) =>
      currentSectionTemplate.replaceAll('{section}', section);

  String orientationSemantics({
    required String project,
    required String section,
  }) => orientationSemanticsTemplate
      .replaceAll('{project}', project)
      .replaceAll('{section}', section);
}

/// Persistent project orientation and common project commands.
///
/// The bar serializes its own callbacks. Hosts that serialize actions across
/// the whole project workspace can additionally supply [busy].
class Revision3ProjectCommandBar extends StatefulWidget {
  const Revision3ProjectCommandBar({
    super.key,
    required this.projectDisplayName,
    required this.currentSectionLabel,
    this.undoCommand,
    required this.searchCommand,
    required this.createCommand,
    required this.problemsCommand,
    this.settingsCommand,
    this.busy,
    this.copy = const Revision3ProjectCommandBarCopy(),
  }) : assert(projectDisplayName.length > 0),
       assert(currentSectionLabel.length > 0);

  static const rootKey = Key('revision3-project-command-bar');
  static const orientationKey = Key(
    'revision3-project-command-bar-orientation',
  );
  static const projectNameKey = Key(
    'revision3-project-command-bar-project-name',
  );
  static const sectionKey = Key('revision3-project-command-bar-section');
  static const undoKey = Key('revision3-project-command-bar-undo');
  static const searchKey = Key('revision3-project-command-bar-search');
  static const createKey = Key('revision3-project-command-bar-create');
  static const problemsKey = Key('revision3-project-command-bar-problems');
  static const settingsKey = Key('managed-open-settings');
  static const moreKey = Key('revision3-project-command-bar-more');
  static const compactCreateKey = Key(
    'revision3-project-command-bar-compact-create',
  );
  static const compactUndoKey = Key(
    'revision3-project-command-bar-compact-undo',
  );
  static const compactProblemsKey = Key(
    'revision3-project-command-bar-compact-problems',
  );
  static const compactSettingsKey = Key(
    'revision3-project-command-bar-compact-settings',
  );
  static const busyStatusKey = Key('revision3-project-command-bar-busy-status');

  final String projectDisplayName;
  final String currentSectionLabel;
  final Revision3ProjectCommand? undoCommand;
  final Revision3ProjectCommand searchCommand;
  final Revision3ProjectCommand createCommand;
  final Revision3ProjectCommand problemsCommand;
  final Revision3ProjectCommand? settingsCommand;
  final Revision3ProjectCommandBarBusyState? busy;
  final Revision3ProjectCommandBarCopy copy;

  @override
  State<Revision3ProjectCommandBar> createState() =>
      _Revision3ProjectCommandBarState();
}

class _Revision3ProjectCommandBarState
    extends State<Revision3ProjectCommandBar> {
  Revision3ProjectCommandBarBusyState? _internalBusy;

  Revision3ProjectCommandBarBusyState? get _busy =>
      widget.busy ?? _internalBusy;

  Revision3ProjectCommand _commandFor(Revision3ProjectCommandKind kind) =>
      switch (kind) {
        Revision3ProjectCommandKind.undo => widget.undoCommand!,
        Revision3ProjectCommandKind.search => widget.searchCommand,
        Revision3ProjectCommandKind.create => widget.createCommand,
        Revision3ProjectCommandKind.problems => widget.problemsCommand,
        Revision3ProjectCommandKind.settings => widget.settingsCommand!,
      };

  String _labelFor(Revision3ProjectCommandKind kind) => switch (kind) {
    Revision3ProjectCommandKind.undo => widget.copy.undoLabel,
    Revision3ProjectCommandKind.search => widget.copy.searchLabel,
    Revision3ProjectCommandKind.create => widget.copy.createLabel,
    Revision3ProjectCommandKind.problems => widget.copy.problemsLabel,
    Revision3ProjectCommandKind.settings => widget.copy.settingsLabel,
  };

  IconData _iconFor(Revision3ProjectCommandKind kind) => switch (kind) {
    Revision3ProjectCommandKind.undo => Icons.undo,
    Revision3ProjectCommandKind.search => Icons.search,
    Revision3ProjectCommandKind.create => Icons.add,
    Revision3ProjectCommandKind.problems => Icons.warning_amber_outlined,
    Revision3ProjectCommandKind.settings => Icons.settings_outlined,
  };

  Key _wideKeyFor(Revision3ProjectCommandKind kind) => switch (kind) {
    Revision3ProjectCommandKind.undo => Revision3ProjectCommandBar.undoKey,
    Revision3ProjectCommandKind.search => Revision3ProjectCommandBar.searchKey,
    Revision3ProjectCommandKind.create => Revision3ProjectCommandBar.createKey,
    Revision3ProjectCommandKind.problems =>
      Revision3ProjectCommandBar.problemsKey,
    Revision3ProjectCommandKind.settings =>
      Revision3ProjectCommandBar.settingsKey,
  };

  String? _disabledReasonFor(Revision3ProjectCommandKind kind) =>
      _busy?.disabledReason ?? _commandFor(kind).disabledReason;

  bool _isEnabled(Revision3ProjectCommandKind kind) =>
      _busy == null && _commandFor(kind).isEnabled;

  Future<void> _invoke(Revision3ProjectCommandKind kind) async {
    if (_busy != null) return;
    final command = _commandFor(kind);
    final invoke = command.onInvoke;
    if (invoke == null) return;

    setState(() {
      _internalBusy = Revision3ProjectCommandBarBusyState(
        command: kind,
        label: widget.copy.busyLabel,
        disabledReason: widget.copy.busyDisabledReason,
      );
    });
    try {
      await invoke();
    } finally {
      if (mounted) {
        setState(() => _internalBusy = null);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Material(
      key: Revision3ProjectCommandBar.rootKey,
      color: theme.colorScheme.surfaceContainerLow,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(16),
        side: BorderSide(color: theme.colorScheme.outlineVariant),
      ),
      clipBehavior: Clip.antiAlias,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
        child: LayoutBuilder(
          builder: (context, constraints) {
            final textScale = MediaQuery.textScalerOf(context).scale(1);
            final useWideLayout =
                constraints.maxWidth >= 720 && textScale < 1.6;
            final content = useWideLayout
                ? _buildWide(context)
                : _buildCompact(context);
            final busy = _busy;
            return Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              mainAxisSize: MainAxisSize.min,
              children: [
                content,
                if (busy != null) ...[
                  if (useWideLayout) const SizedBox(height: 10),
                  _BusyStatus(busy: busy, showVisual: useWideLayout),
                ],
              ],
            );
          },
        ),
      ),
    );
  }

  Widget _buildWide(BuildContext context) => Row(
    crossAxisAlignment: CrossAxisAlignment.center,
    children: [
      Expanded(child: _buildOrientation(context, compact: false)),
      const SizedBox(width: 20),
      Wrap(
        spacing: 8,
        runSpacing: 8,
        alignment: WrapAlignment.end,
        children: [
          if (widget.undoCommand != null)
            _buildWideCommand(Revision3ProjectCommandKind.undo),
          _buildWideCommand(Revision3ProjectCommandKind.search),
          _buildWideCommand(Revision3ProjectCommandKind.create),
          _buildWideCommand(Revision3ProjectCommandKind.problems),
          if (widget.settingsCommand != null) _buildWideSettingsCommand(),
        ],
      ),
    ],
  );

  Widget _buildCompact(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    mainAxisSize: MainAxisSize.min,
    children: [
      _buildOrientation(context, compact: true),
      const SizedBox(height: 8),
      Wrap(
        spacing: 8,
        runSpacing: 8,
        alignment: WrapAlignment.end,
        crossAxisAlignment: WrapCrossAlignment.center,
        children: [
          _buildWideCommand(Revision3ProjectCommandKind.search),
          _buildCompactOverflow(),
        ],
      ),
    ],
  );

  Widget _buildOrientation(BuildContext context, {required bool compact}) {
    final theme = Theme.of(context);
    final projectName = Text(
      widget.projectDisplayName,
      key: Revision3ProjectCommandBar.projectNameKey,
      maxLines: compact ? 2 : 1,
      overflow: TextOverflow.ellipsis,
      style: compact
          ? theme.textTheme.titleLarge?.copyWith(fontWeight: FontWeight.w700)
          : theme.textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
    );
    final section = Text(
      widget.copy.currentSection(widget.currentSectionLabel),
      key: Revision3ProjectCommandBar.sectionKey,
      maxLines: compact ? 2 : 1,
      overflow: TextOverflow.ellipsis,
      style: theme.textTheme.bodyMedium?.copyWith(
        color: theme.colorScheme.onSurfaceVariant,
      ),
    );
    return Semantics(
      key: Revision3ProjectCommandBar.orientationKey,
      container: true,
      header: true,
      label: widget.copy.orientationSemantics(
        project: widget.projectDisplayName,
        section: widget.currentSectionLabel,
      ),
      child: ExcludeSemantics(
        child: compact
            ? Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [projectName, const SizedBox(height: 2), section],
              )
            : Row(
                children: [
                  Flexible(child: projectName),
                  const SizedBox(width: 12),
                  Flexible(child: section),
                ],
              ),
      ),
    );
  }

  Widget _buildWideCommand(Revision3ProjectCommandKind kind) {
    final enabled = _isEnabled(kind);
    final disabledReason = _disabledReasonFor(kind);
    final label = _labelFor(kind);
    final icon = _busy?.command == kind ? Icons.hourglass_top : _iconFor(kind);
    final VoidCallback? onPressed = enabled ? () => _invoke(kind) : null;
    final button = kind == Revision3ProjectCommandKind.create
        ? FilledButton.icon(
            key: _wideKeyFor(kind),
            onPressed: onPressed,
            icon: Icon(icon),
            label: Text(label),
          )
        : OutlinedButton.icon(
            key: _wideKeyFor(kind),
            onPressed: onPressed,
            icon: Icon(icon),
            label: Text(label),
          );
    return Semantics(
      button: true,
      enabled: enabled,
      focusable: enabled,
      label: label,
      hint: disabledReason,
      onTap: enabled ? () => _invoke(kind) : null,
      child: ExcludeSemantics(
        child: Tooltip(message: disabledReason ?? label, child: button),
      ),
    );
  }

  Widget _buildCompactOverflow() =>
      PopupMenuButton<Revision3ProjectCommandKind>(
        key: Revision3ProjectCommandBar.moreKey,
        tooltip: widget.copy.moreActionsTooltip,
        icon: const Icon(Icons.more_horiz),
        onSelected: _invoke,
        itemBuilder: (context) => [
          if (widget.undoCommand != null)
            _buildCompactMenuItem(Revision3ProjectCommandKind.undo),
          _buildCompactMenuItem(Revision3ProjectCommandKind.create),
          _buildCompactMenuItem(Revision3ProjectCommandKind.problems),
          if (widget.settingsCommand != null)
            _buildCompactMenuItem(Revision3ProjectCommandKind.settings),
        ],
      );

  Widget _buildWideSettingsCommand() {
    const kind = Revision3ProjectCommandKind.settings;
    final enabled = _isEnabled(kind);
    final disabledReason = _disabledReasonFor(kind);
    final label = _labelFor(kind);
    final icon = _busy?.command == kind ? Icons.hourglass_top : _iconFor(kind);
    return Semantics(
      button: true,
      enabled: enabled,
      focusable: enabled,
      label: label,
      hint: disabledReason,
      onTap: enabled ? () => _invoke(kind) : null,
      child: ExcludeSemantics(
        child: Tooltip(
          message: disabledReason ?? label,
          child: IconButton.outlined(
            key: Revision3ProjectCommandBar.settingsKey,
            onPressed: enabled ? () => _invoke(kind) : null,
            icon: Icon(icon),
          ),
        ),
      ),
    );
  }

  PopupMenuItem<Revision3ProjectCommandKind> _buildCompactMenuItem(
    Revision3ProjectCommandKind kind,
  ) {
    final enabled = _isEnabled(kind);
    final disabledReason = _disabledReasonFor(kind);
    final label = _labelFor(kind);
    final key = switch (kind) {
      Revision3ProjectCommandKind.undo =>
        Revision3ProjectCommandBar.compactUndoKey,
      Revision3ProjectCommandKind.create =>
        Revision3ProjectCommandBar.compactCreateKey,
      Revision3ProjectCommandKind.problems =>
        Revision3ProjectCommandBar.compactProblemsKey,
      Revision3ProjectCommandKind.settings =>
        Revision3ProjectCommandBar.compactSettingsKey,
      Revision3ProjectCommandKind.search => throw StateError(
        'Search is never rendered in the compact overflow.',
      ),
    };
    final content = Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(_busy?.command == kind ? Icons.hourglass_top : _iconFor(kind)),
        const SizedBox(width: 12),
        Flexible(child: Text(label)),
      ],
    );
    return PopupMenuItem<Revision3ProjectCommandKind>(
      key: key,
      value: kind,
      enabled: enabled,
      child: Semantics(
        button: true,
        enabled: enabled,
        label: label,
        hint: disabledReason,
        child: Tooltip(message: disabledReason ?? label, child: content),
      ),
    );
  }
}

class _BusyStatus extends StatelessWidget {
  const _BusyStatus({required this.busy, required this.showVisual});

  final Revision3ProjectCommandBarBusyState busy;
  final bool showVisual;

  @override
  Widget build(BuildContext context) => Semantics(
    key: Revision3ProjectCommandBar.busyStatusKey,
    container: true,
    liveRegion: true,
    label: busy.label,
    child: ExcludeSemantics(
      child: showVisual
          ? Row(
              children: [
                const Icon(Icons.hourglass_top, size: 18),
                const SizedBox(width: 8),
                Expanded(child: Text(busy.label)),
              ],
            )
          : const SizedBox.shrink(),
    ),
  );
}
