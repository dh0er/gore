import 'package:flutter/material.dart';

import 'revision3_project_workspace.dart';

/// Stable views hosted below the managed Settings / Expert destination.
enum Revision3SettingsExpertView {
  settings,
  dataAssetLab;

  String? get secondaryRoute => switch (this) {
    Revision3SettingsExpertView.settings => null,
    Revision3SettingsExpertView.dataAssetLab => 'data-asset-lab',
  };
}

/// Presentation-only Settings / Expert destination for managed projects.
///
/// Labels and both tool surfaces are injected by the owning shell. This widget
/// owns only route projection and lazy presentation lifetime; it owns no
/// settings data or dialog and grants no project-mutation or expert authority.
class Revision3SettingsExpertPage extends StatefulWidget {
  Revision3SettingsExpertPage({
    required this.location,
    required this.settingsLabel,
    required this.dataAssetLabLabel,
    required this.settings,
    required this.dataAssetLab,
    super.key,
  }) : assert(
         location.section == Revision3ProjectWorkspaceSection.settingsExpert,
         'Revision3SettingsExpertPage requires the Settings / Expert section '
         'location.',
       );

  final Revision3ProjectWorkspaceLocation location;
  final String settingsLabel;
  final String dataAssetLabLabel;
  final Widget settings;
  final Widget dataAssetLab;

  @override
  State<Revision3SettingsExpertPage> createState() =>
      _Revision3SettingsExpertPageState();
}

class _Revision3SettingsExpertPageState
    extends State<Revision3SettingsExpertPage> {
  final Set<Revision3SettingsExpertView> _mounted = {};

  Revision3SettingsExpertView get _selected =>
      widget.location.secondary ==
          Revision3SettingsExpertView.dataAssetLab.secondaryRoute
      ? Revision3SettingsExpertView.dataAssetLab
      : Revision3SettingsExpertView.settings;

  @override
  void initState() {
    super.initState();
    _mounted.add(_selected);
  }

  @override
  void didUpdateWidget(covariant Revision3SettingsExpertPage oldWidget) {
    super.didUpdateWidget(oldWidget);
    _mounted.add(_selected);
  }

  void _select(Revision3SettingsExpertView view) {
    Revision3ProjectWorkspace.navigate(
      context,
      Revision3ProjectWorkspaceLocation(
        Revision3ProjectWorkspaceSection.settingsExpert,
        secondary: view.secondaryRoute,
      ),
    );
  }

  @override
  Widget build(BuildContext context) => Semantics(
    key: const Key('revision3-settings-expert-page'),
    container: true,
    explicitChildNodes: true,
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Material(
          color: Theme.of(context).colorScheme.surfaceContainerLow,
          child: SingleChildScrollView(
            key: const Key('revision3-settings-expert-page-navigation-scroll'),
            scrollDirection: Axis.horizontal,
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 8),
            child: SegmentedButton<Revision3SettingsExpertView>(
              key: const Key('revision3-settings-expert-page-navigation'),
              showSelectedIcon: false,
              segments: [
                ButtonSegment(
                  value: Revision3SettingsExpertView.settings,
                  icon: const Icon(Icons.settings_outlined),
                  label: Text(
                    widget.settingsLabel,
                    key: const Key(
                      'revision3-settings-expert-page-nav-settings',
                    ),
                  ),
                ),
                ButtonSegment(
                  value: Revision3SettingsExpertView.dataAssetLab,
                  icon: const Icon(Icons.data_object_outlined),
                  label: Text(
                    widget.dataAssetLabLabel,
                    key: const Key(
                      'revision3-settings-expert-page-nav-data-asset-lab',
                    ),
                  ),
                ),
              ],
              selected: {_selected},
              onSelectionChanged: (selection) => _select(selection.single),
            ),
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: IndexedStack(
            key: const Key('revision3-settings-expert-page-pages'),
            index: _selected.index,
            sizing: StackFit.expand,
            children: [
              _mounted.contains(Revision3SettingsExpertView.settings)
                  ? KeyedSubtree(
                      key: const Key('revision3-settings-expert-page-settings'),
                      child: widget.settings,
                    )
                  : const SizedBox.shrink(),
              _mounted.contains(Revision3SettingsExpertView.dataAssetLab)
                  ? KeyedSubtree(
                      key: const Key(
                        'revision3-settings-expert-page-data-asset-lab',
                      ),
                      child: widget.dataAssetLab,
                    )
                  : const SizedBox.shrink(),
            ],
          ),
        ),
      ],
    ),
  );
}
