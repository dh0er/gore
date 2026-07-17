import 'package:flutter/material.dart';

/// Stable, author-facing destinations of the managed revision-3 workspace.
///
/// The order is the canonical primary-navigation order and must not depend on
/// project content or currently available authoring capabilities.
enum Revision3ProjectWorkspaceSection {
  home,
  content,
  story,
  world,
  localizationVoice,
  validateTest,
  buildRelease,
  history,
  settingsExpert,
}

/// One exact workspace route.
///
/// [secondary] belongs to the selected primary section. The shell remembers
/// the last secondary route independently for every section while the same
/// project remains open.
@immutable
final class Revision3ProjectWorkspaceLocation {
  const Revision3ProjectWorkspaceLocation(this.section, {this.secondary});

  final Revision3ProjectWorkspaceSection section;
  final String? secondary;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Revision3ProjectWorkspaceLocation &&
          other.section == section &&
          other.secondary == secondary;

  @override
  int get hashCode => Object.hash(section, secondary);
}

typedef Revision3ProjectWorkspacePageBuilder =
    Widget Function(
      BuildContext context,
      Revision3ProjectWorkspaceLocation location,
    );

/// Builds persistent workspace chrome for the currently selected location.
///
/// Unlike destination pages, this builder is not mounted lazily and remains
/// above the page area while primary and secondary locations change.
typedef Revision3ProjectWorkspaceChromeBuilder =
    Widget Function(
      BuildContext context,
      Revision3ProjectWorkspaceLocation location,
    );

/// Immutable description of one primary workspace destination.
@immutable
final class Revision3ProjectWorkspaceDestination {
  const Revision3ProjectWorkspaceDestination({
    required this.section,
    required this.label,
    required this.icon,
    required this.selectedIcon,
    required this.pageBuilder,
  });

  final Revision3ProjectWorkspaceSection section;
  final String label;
  final IconData icon;
  final IconData selectedIcon;
  final Revision3ProjectWorkspacePageBuilder pageBuilder;
}

/// Responsive, presentation-only shell for a managed revision-3 project.
///
/// Equality of [projectIdentity] defines the project lifetime. Keep it stable
/// across revision changes: changing it resets navigation and every mounted
/// page, while rebuilding the same project preserves both.
class Revision3ProjectWorkspace extends StatefulWidget {
  Revision3ProjectWorkspace({
    required this.projectIdentity,
    required List<Revision3ProjectWorkspaceDestination> destinations,
    this.chromeBuilder,
    super.key,
  }) : destinations = _validatedDestinations(destinations);

  final Object projectIdentity;
  final List<Revision3ProjectWorkspaceDestination> destinations;
  final Revision3ProjectWorkspaceChromeBuilder? chromeBuilder;

  static const chromeScrollKey = Key(
    'revision3-project-workspace-chrome-scroll',
  );

  /// Selects an exact workspace location from any descendant context.
  static void navigate(
    BuildContext context,
    Revision3ProjectWorkspaceLocation location,
  ) {
    final state = context
        .findAncestorStateOfType<_Revision3ProjectWorkspaceState>();
    if (state == null) {
      throw FlutterError(
        'Revision3ProjectWorkspace.navigate requires a descendant context.',
      );
    }
    state._selectLocation(location);
  }

  @override
  State<Revision3ProjectWorkspace> createState() =>
      _Revision3ProjectWorkspaceState();
}

class _Revision3ProjectWorkspaceState extends State<Revision3ProjectWorkspace> {
  static const _railBreakpoint = 720.0;
  static const _extendedRailBreakpoint = 1200.0;
  static const _railDestinationExtent = 72.0;
  static const _railVerticalPadding = 24.0;

  Revision3ProjectWorkspaceSection _selected =
      Revision3ProjectWorkspaceSection.home;
  final Set<Revision3ProjectWorkspaceSection> _mounted = {
    Revision3ProjectWorkspaceSection.home,
  };
  final Map<Revision3ProjectWorkspaceSection, String?> _secondaryRoutes = {
    Revision3ProjectWorkspaceSection.home: null,
  };
  int _projectEpoch = 0;

  @override
  void didUpdateWidget(covariant Revision3ProjectWorkspace oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.projectIdentity == widget.projectIdentity) return;
    _projectEpoch++;
    _selected = Revision3ProjectWorkspaceSection.home;
    _mounted
      ..clear()
      ..add(Revision3ProjectWorkspaceSection.home);
    _secondaryRoutes
      ..clear()
      ..[Revision3ProjectWorkspaceSection.home] = null;
  }

  Revision3ProjectWorkspaceLocation _locationFor(
    Revision3ProjectWorkspaceSection section,
  ) => Revision3ProjectWorkspaceLocation(
    section,
    secondary: _secondaryRoutes[section],
  );

  void _selectSection(Revision3ProjectWorkspaceSection section) =>
      _selectLocation(_locationFor(section));

  void _selectLocation(Revision3ProjectWorkspaceLocation location) {
    if (_selected == location.section &&
        _mounted.contains(location.section) &&
        _secondaryRoutes[location.section] == location.secondary) {
      return;
    }
    setState(() {
      _selected = location.section;
      _secondaryRoutes[location.section] = location.secondary;
      _mounted.add(location.section);
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
          return _buildNarrowWorkspace(pages);
        }
        return _buildRailWorkspace(
          pages,
          extended: constraints.maxWidth >= _extendedRailBreakpoint,
        );
      },
    ),
  );

  Widget _buildNarrowWorkspace(Widget pages) {
    final selected = widget.destinations[_selected.index];
    return Column(
      children: [
        Material(
          key: const Key('revision3-project-workspace-narrow-navigation'),
          color: Theme.of(context).colorScheme.surfaceContainerLow,
          child: SafeArea(
            bottom: false,
            child: Padding(
              padding: const EdgeInsetsDirectional.fromSTEB(16, 8, 8, 8),
              child: Row(
                children: [
                  Icon(selected.selectedIcon),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Semantics(
                      header: true,
                      child: Text(
                        selected.label,
                        key: const Key(
                          'revision3-project-workspace-narrow-current-label',
                        ),
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                    ),
                  ),
                  PopupMenuButton<Revision3ProjectWorkspaceSection>(
                    key: const Key('revision3-project-workspace-narrow-menu'),
                    initialValue: _selected,
                    onSelected: _selectSection,
                    itemBuilder: (context) => [
                      for (final destination in widget.destinations)
                        PopupMenuItem(
                          key: _navigationKey(destination.section),
                          value: destination.section,
                          child: Row(
                            children: [
                              Icon(
                                destination.section == _selected
                                    ? destination.selectedIcon
                                    : destination.icon,
                              ),
                              const SizedBox(width: 12),
                              Expanded(child: Text(destination.label)),
                            ],
                          ),
                        ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ),
        Expanded(child: _buildPageArea(pages)),
      ],
    );
  }

  Widget _buildRailWorkspace(Widget pages, {required bool extended}) => Row(
    children: [
      Semantics(
        key: const Key('revision3-project-workspace-desktop-navigation'),
        container: true,
        explicitChildNodes: true,
        child: LayoutBuilder(
          builder: (context, constraints) {
            final minimumHeight =
                (widget.destinations.length * _railDestinationExtent) +
                _railVerticalPadding;
            final viewportHeight = constraints.maxHeight.isFinite
                ? constraints.maxHeight
                : minimumHeight;
            final railHeight = viewportHeight < minimumHeight
                ? minimumHeight
                : viewportHeight;
            return SingleChildScrollView(
              key: const Key(
                'revision3-project-workspace-desktop-navigation-scroll',
              ),
              child: SizedBox(
                height: railHeight,
                child: NavigationRail(
                  key: const Key('revision3-project-workspace-rail'),
                  selectedIndex: _selected.index,
                  extended: extended,
                  onDestinationSelected: (index) =>
                      _selectSection(widget.destinations[index].section),
                  destinations: [
                    for (final destination in widget.destinations)
                      NavigationRailDestination(
                        icon: _RailDestinationIcon(
                          key: _navigationKey(destination.section),
                          icon: destination.icon,
                          label: destination.label,
                          selected: false,
                        ),
                        selectedIcon: _RailDestinationIcon(
                          key: _navigationKey(destination.section),
                          icon: destination.selectedIcon,
                          label: destination.label,
                          selected: true,
                        ),
                        label: Text(destination.label),
                      ),
                  ],
                ),
              ),
            );
          },
        ),
      ),
      const VerticalDivider(width: 1),
      Expanded(child: _buildPageArea(pages)),
    ],
  );

  Widget _buildPageArea(Widget pages) {
    final chromeBuilder = widget.chromeBuilder;
    if (chromeBuilder == null) return pages;
    return LayoutBuilder(
      builder: (context, constraints) {
        final chrome = KeyedSubtree(
          key: ValueKey(('chrome', _projectEpoch)),
          child: Semantics(
            key: const Key('revision3-project-workspace-chrome'),
            container: true,
            child: Builder(
              builder: (context) =>
                  chromeBuilder(context, _locationFor(_selected)),
            ),
          ),
        );
        if (!constraints.maxHeight.isFinite) {
          return Column(
            mainAxisSize: MainAxisSize.min,
            children: [chrome, pages],
          );
        }

        const minimumPageHeight = 96.0;
        final maximumChromeHeight = constraints.maxHeight > minimumPageHeight
            ? constraints.maxHeight - minimumPageHeight
            : constraints.maxHeight * 0.45;
        return Column(
          children: [
            KeyedSubtree(
              key: ValueKey(('chrome-scroll', _projectEpoch)),
              child: ConstrainedBox(
                constraints: BoxConstraints(maxHeight: maximumChromeHeight),
                child: SingleChildScrollView(
                  key: Revision3ProjectWorkspace.chromeScrollKey,
                  primary: false,
                  child: chrome,
                ),
              ),
            ),
            Expanded(child: pages),
          ],
        );
      },
    );
  }

  Widget _buildPages() => IndexedStack(
    key: const Key('revision3-project-workspace-pages'),
    index: _selected.index,
    sizing: StackFit.expand,
    children: [
      for (final destination in widget.destinations)
        _mounted.contains(destination.section)
            ? KeyedSubtree(
                key: ValueKey((_projectEpoch, destination.section)),
                child: Semantics(
                  key: _pageKey(destination.section),
                  container: true,
                  explicitChildNodes: true,
                  label: destination.label,
                  child: Builder(
                    builder: (context) => destination.pageBuilder(
                      context,
                      _locationFor(destination.section),
                    ),
                  ),
                ),
              )
            : const SizedBox.shrink(),
    ],
  );
}

class _RailDestinationIcon extends StatelessWidget {
  const _RailDestinationIcon({
    required this.icon,
    required this.label,
    required this.selected,
    super.key,
  });

  final IconData icon;
  final String label;
  final bool selected;

  @override
  Widget build(BuildContext context) => Tooltip(
    message: label,
    child: Semantics(
      button: true,
      selected: selected,
      label: label,
      child: Icon(icon),
    ),
  );
}

List<Revision3ProjectWorkspaceDestination> _validatedDestinations(
  List<Revision3ProjectWorkspaceDestination> destinations,
) {
  final sections = Revision3ProjectWorkspaceSection.values;
  if (destinations.length != sections.length) {
    throw ArgumentError.value(
      destinations,
      'destinations',
      'must contain exactly one destination for every canonical section',
    );
  }
  for (var index = 0; index < sections.length; index++) {
    if (destinations[index].section != sections[index]) {
      throw ArgumentError.value(
        destinations,
        'destinations',
        'must follow canonical section order without duplicates',
      );
    }
  }
  return List<Revision3ProjectWorkspaceDestination>.unmodifiable(destinations);
}

Key _navigationKey(Revision3ProjectWorkspaceSection section) =>
    Key('revision3-project-workspace-nav-${_sectionKey(section)}');

Key _pageKey(Revision3ProjectWorkspaceSection section) =>
    Key('revision3-project-workspace-page-${_sectionKey(section)}');

String _sectionKey(Revision3ProjectWorkspaceSection section) =>
    switch (section) {
      Revision3ProjectWorkspaceSection.home => 'home',
      Revision3ProjectWorkspaceSection.content => 'content',
      Revision3ProjectWorkspaceSection.story => 'story',
      Revision3ProjectWorkspaceSection.world => 'world',
      Revision3ProjectWorkspaceSection.localizationVoice =>
        'localization-voice',
      Revision3ProjectWorkspaceSection.validateTest => 'validate-test',
      Revision3ProjectWorkspaceSection.buildRelease => 'build-release',
      Revision3ProjectWorkspaceSection.history => 'history',
      Revision3ProjectWorkspaceSection.settingsExpert => 'settings-expert',
    };
