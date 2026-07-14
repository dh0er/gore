import 'package:flutter/material.dart';

/// Stable destinations exposed by the managed revision-3 project workspace.
enum Revision3ProjectWorkspaceSection { overview, content, dataAssets }

/// Presentation-only shell for the three managed-project work areas.
///
/// Equality of [projectIdentity] defines the project lifetime. Keep it stable
/// across revision changes; changing it resets navigation and every mounted
/// page state.
class Revision3ProjectWorkspace extends StatefulWidget {
  const Revision3ProjectWorkspace({
    required this.projectIdentity,
    required this.overviewLabel,
    required this.contentLabel,
    required this.dataAssetsLabel,
    required this.overview,
    required this.content,
    required this.dataAssets,
    super.key,
  });

  final Object projectIdentity;
  final String overviewLabel;
  final String contentLabel;
  final String dataAssetsLabel;
  final Widget overview;
  final Widget content;
  final Widget dataAssets;

  /// Selects a workspace section from any descendant context.
  static void navigate(
    BuildContext context,
    Revision3ProjectWorkspaceSection section,
  ) {
    final state = context
        .findAncestorStateOfType<_Revision3ProjectWorkspaceState>();
    if (state == null) {
      throw FlutterError(
        'Revision3ProjectWorkspace.navigate requires a descendant context.',
      );
    }
    state._select(section);
  }

  @override
  State<Revision3ProjectWorkspace> createState() =>
      _Revision3ProjectWorkspaceState();
}

class _Revision3ProjectWorkspaceState extends State<Revision3ProjectWorkspace> {
  static const _railBreakpoint = 720.0;
  static const _extendedRailBreakpoint = 1200.0;

  Revision3ProjectWorkspaceSection _selected =
      Revision3ProjectWorkspaceSection.overview;
  final Set<Revision3ProjectWorkspaceSection> _mounted = {
    Revision3ProjectWorkspaceSection.overview,
  };
  int _projectEpoch = 0;

  @override
  void didUpdateWidget(covariant Revision3ProjectWorkspace oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.projectIdentity == widget.projectIdentity) return;
    _projectEpoch++;
    _selected = Revision3ProjectWorkspaceSection.overview;
    _mounted
      ..clear()
      ..add(Revision3ProjectWorkspaceSection.overview);
  }

  void _select(Revision3ProjectWorkspaceSection section) {
    if (_selected == section && _mounted.contains(section)) return;
    setState(() {
      _selected = section;
      _mounted.add(section);
    });
  }

  @override
  Widget build(BuildContext context) => Semantics(
    key: const Key('revision3-project-workspace'),
    container: true,
    explicitChildNodes: true,
    child: LayoutBuilder(
      builder: (context, constraints) {
        final pages = _buildPages();
        if (constraints.maxWidth < _railBreakpoint) {
          return Column(
            children: [
              Expanded(child: pages),
              Semantics(
                container: true,
                explicitChildNodes: true,
                child: NavigationBar(
                  key: const Key(
                    'revision3-project-workspace-compact-navigation',
                  ),
                  selectedIndex: _selected.index,
                  onDestinationSelected: (index) =>
                      _select(Revision3ProjectWorkspaceSection.values[index]),
                  destinations: [
                    NavigationDestination(
                      key: const Key('managed-revision3-overview-tab'),
                      icon: const Icon(
                        Icons.dashboard_outlined,
                        key: Key('revision3-project-workspace-nav-overview'),
                      ),
                      selectedIcon: const Icon(Icons.dashboard),
                      label: widget.overviewLabel,
                    ),
                    NavigationDestination(
                      key: const Key('managed-revision3-library-tab'),
                      icon: const Icon(
                        Icons.account_tree_outlined,
                        key: Key('revision3-project-workspace-nav-content'),
                      ),
                      selectedIcon: const Icon(Icons.account_tree),
                      label: widget.contentLabel,
                    ),
                    NavigationDestination(
                      key: const Key('managed-revision3-dataasset-tab'),
                      icon: const Icon(
                        Icons.data_object_outlined,
                        key: Key('revision3-project-workspace-nav-data-assets'),
                      ),
                      selectedIcon: const Icon(Icons.data_object),
                      label: widget.dataAssetsLabel,
                    ),
                  ],
                ),
              ),
            ],
          );
        }
        return Row(
          children: [
            Semantics(
              container: true,
              explicitChildNodes: true,
              child: NavigationRail(
                key: const Key(
                  'revision3-project-workspace-desktop-navigation',
                ),
                selectedIndex: _selected.index,
                extended: constraints.maxWidth >= _extendedRailBreakpoint,
                onDestinationSelected: (index) =>
                    _select(Revision3ProjectWorkspaceSection.values[index]),
                destinations: [
                  NavigationRailDestination(
                    icon: const KeyedSubtree(
                      key: Key('managed-revision3-overview-tab'),
                      child: Icon(
                        Icons.dashboard_outlined,
                        key: Key(
                          'revision3-project-workspace-rail-overview-icon',
                        ),
                      ),
                    ),
                    selectedIcon: const KeyedSubtree(
                      key: Key('managed-revision3-overview-tab'),
                      child: Icon(
                        Icons.dashboard,
                        key: Key(
                          'revision3-project-workspace-rail-overview-selected-icon',
                        ),
                      ),
                    ),
                    label: Text(widget.overviewLabel),
                  ),
                  NavigationRailDestination(
                    icon: const KeyedSubtree(
                      key: Key('managed-revision3-library-tab'),
                      child: Icon(
                        Icons.account_tree_outlined,
                        key: Key(
                          'revision3-project-workspace-rail-content-icon',
                        ),
                      ),
                    ),
                    selectedIcon: const KeyedSubtree(
                      key: Key('managed-revision3-library-tab'),
                      child: Icon(
                        Icons.account_tree,
                        key: Key(
                          'revision3-project-workspace-rail-content-selected-icon',
                        ),
                      ),
                    ),
                    label: Text(widget.contentLabel),
                  ),
                  NavigationRailDestination(
                    icon: const KeyedSubtree(
                      key: Key('managed-revision3-dataasset-tab'),
                      child: Icon(
                        Icons.data_object_outlined,
                        key: Key(
                          'revision3-project-workspace-rail-data-assets-icon',
                        ),
                      ),
                    ),
                    selectedIcon: const KeyedSubtree(
                      key: Key('managed-revision3-dataasset-tab'),
                      child: Icon(
                        Icons.data_object,
                        key: Key(
                          'revision3-project-workspace-rail-data-assets-selected-icon',
                        ),
                      ),
                    ),
                    label: Text(widget.dataAssetsLabel),
                  ),
                ],
              ),
            ),
            const VerticalDivider(width: 1),
            Expanded(child: pages),
          ],
        );
      },
    ),
  );

  Widget _buildPages() => IndexedStack(
    key: const Key('revision3-project-workspace-pages'),
    index: _selected.index,
    sizing: StackFit.expand,
    children: [
      for (final section in Revision3ProjectWorkspaceSection.values)
        _mounted.contains(section)
            ? KeyedSubtree(
                key: ValueKey((_projectEpoch, section)),
                child: Semantics(
                  key: _pageKey(section),
                  container: true,
                  explicitChildNodes: true,
                  label: _label(section),
                  child: _page(section),
                ),
              )
            : const SizedBox.shrink(),
    ],
  );

  String _label(Revision3ProjectWorkspaceSection section) => switch (section) {
    Revision3ProjectWorkspaceSection.overview => widget.overviewLabel,
    Revision3ProjectWorkspaceSection.content => widget.contentLabel,
    Revision3ProjectWorkspaceSection.dataAssets => widget.dataAssetsLabel,
  };

  Widget _page(Revision3ProjectWorkspaceSection section) => switch (section) {
    Revision3ProjectWorkspaceSection.overview => widget.overview,
    Revision3ProjectWorkspaceSection.content => widget.content,
    Revision3ProjectWorkspaceSection.dataAssets => widget.dataAssets,
  };
}

Key _pageKey(Revision3ProjectWorkspaceSection section) => switch (section) {
  Revision3ProjectWorkspaceSection.overview => const Key(
    'revision3-project-workspace-page-overview',
  ),
  Revision3ProjectWorkspaceSection.content => const Key(
    'revision3-project-workspace-page-content',
  ),
  Revision3ProjectWorkspaceSection.dataAssets => const Key(
    'revision3-project-workspace-page-data-assets',
  ),
};
