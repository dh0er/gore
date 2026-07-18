import 'package:flutter/material.dart';

import 'revision3_project_workspace.dart';

/// Stable views hosted below the canonical managed-project Content section.
enum Revision3ContentWorkspaceView {
  library,
  items,
  dataAssets;

  String? get secondaryRoute => switch (this) {
    Revision3ContentWorkspaceView.library => null,
    Revision3ContentWorkspaceView.items => 'items',
    Revision3ContentWorkspaceView.dataAssets => 'data-assets',
  };
}

/// Presentation-only secondary navigation for project-owned content tools.
///
/// The canonical [Revision3ProjectWorkspace] owns the selected route. This
/// widget only renders that route and asks the parent workspace to change it,
/// which means dashboard deep links and the user's last Content sub-view stay
/// in sync. Views are mounted lazily so opening the Content library does not
/// also start DataAsset discovery.
class Revision3ContentWorkspace extends StatefulWidget {
  Revision3ContentWorkspace({
    required this.projectIdentity,
    required this.location,
    required this.libraryLabel,
    required this.itemsLabel,
    required this.dataAssetsLabel,
    required this.library,
    required this.items,
    required this.dataAssets,
    super.key,
  }) : assert(
         location.section == Revision3ProjectWorkspaceSection.content,
         'Revision3ContentWorkspace requires the Content section location.',
       );

  /// Stable project identity without the changing project revision.
  ///
  /// Lazily mounted pages retain their UI state while this stays equal and
  /// restart when another project is adopted in the same workspace state.
  final Object projectIdentity;
  final Revision3ProjectWorkspaceLocation location;
  final String libraryLabel;
  final String itemsLabel;
  final String dataAssetsLabel;
  final Widget library;
  final Widget items;
  final Widget dataAssets;

  @override
  State<Revision3ContentWorkspace> createState() =>
      _Revision3ContentWorkspaceState();
}

class _Revision3ContentWorkspaceState extends State<Revision3ContentWorkspace> {
  final Set<Revision3ContentWorkspaceView> _mounted = {};

  Revision3ContentWorkspaceView get _selected =>
      switch (widget.location.secondary) {
        'items' => Revision3ContentWorkspaceView.items,
        'data-assets' => Revision3ContentWorkspaceView.dataAssets,
        _ => Revision3ContentWorkspaceView.library,
      };

  @override
  void initState() {
    super.initState();
    _mounted.add(_selected);
  }

  @override
  void didUpdateWidget(covariant Revision3ContentWorkspace oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.projectIdentity != widget.projectIdentity) {
      _mounted.clear();
    }
    _mounted.add(_selected);
  }

  void _select(Revision3ContentWorkspaceView view) {
    Revision3ProjectWorkspace.navigate(
      context,
      Revision3ProjectWorkspaceLocation(
        Revision3ProjectWorkspaceSection.content,
        secondary: view.secondaryRoute,
      ),
    );
  }

  @override
  Widget build(BuildContext context) => Column(
    key: const Key('revision3-content-workspace'),
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      Material(
        color: Theme.of(context).colorScheme.surfaceContainerLow,
        child: SingleChildScrollView(
          key: const Key('revision3-content-workspace-navigation-scroll'),
          scrollDirection: Axis.horizontal,
          padding: const EdgeInsets.fromLTRB(16, 10, 16, 10),
          child: SegmentedButton<Revision3ContentWorkspaceView>(
            key: const Key('revision3-content-workspace-navigation'),
            showSelectedIcon: false,
            segments: [
              ButtonSegment(
                value: Revision3ContentWorkspaceView.library,
                icon: const Icon(Icons.account_tree_outlined),
                label: Text(
                  widget.libraryLabel,
                  key: const Key('revision3-content-workspace-nav-library'),
                ),
              ),
              ButtonSegment(
                value: Revision3ContentWorkspaceView.items,
                icon: const Icon(Icons.inventory_2_outlined),
                label: Text(
                  widget.itemsLabel,
                  key: const Key('revision3-content-workspace-nav-items'),
                ),
              ),
              ButtonSegment(
                value: Revision3ContentWorkspaceView.dataAssets,
                icon: const Icon(Icons.data_object_outlined),
                label: Text(
                  widget.dataAssetsLabel,
                  key: const Key('revision3-content-workspace-nav-data-assets'),
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
          key: const Key('revision3-content-workspace-pages'),
          index: _selected.index,
          sizing: StackFit.expand,
          children: [
            _mounted.contains(Revision3ContentWorkspaceView.library)
                ? KeyedSubtree(
                    key: const Key('revision3-content-workspace-page-library'),
                    child: KeyedSubtree(
                      key: ValueKey((widget.projectIdentity, 'library')),
                      child: widget.library,
                    ),
                  )
                : const SizedBox.shrink(),
            _mounted.contains(Revision3ContentWorkspaceView.items)
                ? KeyedSubtree(
                    key: const Key('revision3-content-workspace-page-items'),
                    child: KeyedSubtree(
                      key: ValueKey((widget.projectIdentity, 'items')),
                      child: widget.items,
                    ),
                  )
                : const SizedBox.shrink(),
            _mounted.contains(Revision3ContentWorkspaceView.dataAssets)
                ? KeyedSubtree(
                    key: const Key(
                      'revision3-content-workspace-page-data-assets',
                    ),
                    child: KeyedSubtree(
                      key: ValueKey((widget.projectIdentity, 'data-assets')),
                      child: widget.dataAssets,
                    ),
                  )
                : const SizedBox.shrink(),
          ],
        ),
      ),
    ],
  );
}
