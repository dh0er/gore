import 'package:flutter/material.dart';

/// Honest source scopes exposed by the unified managed-project content host.
enum Revision3ScopedContentScope { thisMod, baseGame, installed }

/// Presentation-only source host for the managed-project Content Library.
///
/// Equality of [projectIdentity] defines the hosted project lifetime. Rebuilds
/// for the same project retain the selected scope and every visited page. A
/// different identity returns to [Revision3ScopedContentScope.thisMod] and
/// discards pages mounted for the previous project.
class Revision3ScopedContentBrowser extends StatefulWidget {
  const Revision3ScopedContentBrowser({
    required this.projectIdentity,
    required this.thisModLabel,
    required this.baseGameLabel,
    required this.installedLabel,
    required this.thisMod,
    required this.baseGame,
    required this.installed,
    super.key,
  });

  final Object projectIdentity;
  final String thisModLabel;
  final String baseGameLabel;
  final String installedLabel;
  final Widget thisMod;
  final Widget baseGame;
  final Widget installed;

  @override
  State<Revision3ScopedContentBrowser> createState() =>
      _Revision3ScopedContentBrowserState();
}

class _Revision3ScopedContentBrowserState
    extends State<Revision3ScopedContentBrowser> {
  Revision3ScopedContentScope _selected = Revision3ScopedContentScope.thisMod;
  final Set<Revision3ScopedContentScope> _mounted = {
    Revision3ScopedContentScope.thisMod,
  };
  int _projectEpoch = 0;

  @override
  void didUpdateWidget(covariant Revision3ScopedContentBrowser oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.projectIdentity == widget.projectIdentity) return;
    _projectEpoch++;
    _selected = Revision3ScopedContentScope.thisMod;
    _mounted
      ..clear()
      ..add(Revision3ScopedContentScope.thisMod);
  }

  void _select(Revision3ScopedContentScope scope) {
    if (scope == _selected) return;
    setState(() {
      _selected = scope;
      _mounted.add(scope);
    });
  }

  @override
  Widget build(BuildContext context) => Semantics(
    key: const Key('revision3-scoped-content-browser'),
    container: true,
    explicitChildNodes: true,
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Material(
          color: Theme.of(context).colorScheme.surfaceContainerLow,
          child: SingleChildScrollView(
            key: const Key(
              'revision3-scoped-content-browser-navigation-scroll',
            ),
            scrollDirection: Axis.horizontal,
            padding: const EdgeInsets.fromLTRB(16, 10, 16, 10),
            child: SegmentedButton<Revision3ScopedContentScope>(
              key: const Key('revision3-scoped-content-browser-navigation'),
              showSelectedIcon: false,
              segments: [
                ButtonSegment(
                  value: Revision3ScopedContentScope.thisMod,
                  label: Text(
                    widget.thisModLabel,
                    key: const Key(
                      'revision3-scoped-content-browser-nav-this-mod',
                    ),
                  ),
                ),
                ButtonSegment(
                  value: Revision3ScopedContentScope.baseGame,
                  label: Text(
                    widget.baseGameLabel,
                    key: const Key(
                      'revision3-scoped-content-browser-nav-base-game',
                    ),
                  ),
                ),
                ButtonSegment(
                  value: Revision3ScopedContentScope.installed,
                  label: Text(
                    widget.installedLabel,
                    key: const Key(
                      'revision3-scoped-content-browser-nav-installed',
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
            key: const Key('revision3-scoped-content-browser-pages'),
            index: _selected.index,
            sizing: StackFit.expand,
            children: [
              _buildPage(Revision3ScopedContentScope.thisMod, widget.thisMod),
              _buildPage(Revision3ScopedContentScope.baseGame, widget.baseGame),
              _buildPage(
                Revision3ScopedContentScope.installed,
                widget.installed,
              ),
            ],
          ),
        ),
      ],
    ),
  );

  Widget _buildPage(Revision3ScopedContentScope scope, Widget child) =>
      _mounted.contains(scope)
      ? KeyedSubtree(
          key: ValueKey((_projectEpoch, scope)),
          child: Semantics(
            key: Key(
              'revision3-scoped-content-browser-page-${_scopeKey(scope)}',
            ),
            container: true,
            explicitChildNodes: true,
            child: child,
          ),
        )
      : const SizedBox.shrink();
}

String _scopeKey(Revision3ScopedContentScope scope) => switch (scope) {
  Revision3ScopedContentScope.thisMod => 'this-mod',
  Revision3ScopedContentScope.baseGame => 'base-game',
  Revision3ScopedContentScope.installed => 'installed',
};
