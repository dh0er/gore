import 'package:flutter/material.dart';

/// Stable, author-facing destinations of the managed revision-3 workspace.
///
/// The order is the canonical primary-navigation order and must not depend on
/// project content or currently available authoring capabilities.
enum Revision3ProjectWorkspaceSection {
  home,
  content,
  story,
  textVoice,
  testRelease,
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

class _Revision3ProjectWorkspaceState extends State<Revision3ProjectWorkspace>
    with SingleTickerProviderStateMixin {
  Revision3ProjectWorkspaceSection _selected =
      Revision3ProjectWorkspaceSection.home;
  final Set<Revision3ProjectWorkspaceSection> _mounted = {
    Revision3ProjectWorkspaceSection.home,
  };
  final Map<Revision3ProjectWorkspaceSection, String?> _secondaryRoutes = {
    Revision3ProjectWorkspaceSection.home: null,
  };
  late final TabController _tabController;
  final Map<Revision3ProjectWorkspaceSection, GlobalKey> _tabVisibilityKeys = {
    for (final section in Revision3ProjectWorkspaceSection.values)
      section: GlobalKey(debugLabel: 'workspace-tab-${_sectionKey(section)}'),
  };
  int _projectEpoch = 0;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(
      length: widget.destinations.length,
      initialIndex: _selected.index,
      vsync: this,
    );
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

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
    _tabController.index = Revision3ProjectWorkspaceSection.home.index;
    _scheduleTabVisible(Revision3ProjectWorkspaceSection.home);
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
      _scheduleTabVisible(location.section);
      return;
    }
    setState(() {
      _selected = location.section;
      _secondaryRoutes[location.section] = location.secondary;
      _mounted.add(location.section);
    });
    if (_tabController.index != location.section.index) {
      _tabController.index = location.section.index;
    }
    _scheduleTabVisible(location.section);
  }

  void _scheduleTabVisible(Revision3ProjectWorkspaceSection section) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || _selected != section) return;
      final tabContext = _tabVisibilityKeys[section]?.currentContext;
      if (tabContext == null) return;
      Scrollable.ensureVisible(
        tabContext,
        alignment: 0.5,
        duration: const Duration(milliseconds: 160),
        curve: Curves.easeOutCubic,
      );
    });
  }

  @override
  Widget build(BuildContext context) => Semantics(
    key: const Key('revision3-project-workspace'),
    container: true,
    explicitChildNodes: true,
    child: Column(
      children: [
        _buildTabBar(),
        Expanded(child: _buildPageArea(_buildPages())),
      ],
    ),
  );

  Widget _buildTabBar() {
    final theme = Theme.of(context);
    final labelStyle =
        theme.tabBarTheme.labelStyle ?? theme.textTheme.titleSmall;
    final fontSize = labelStyle?.fontSize ?? 14.0;
    final scaledLineHeight =
        MediaQuery.textScalerOf(context).scale(fontSize) *
        (labelStyle?.height ?? 1.2);
    final contentHeight = scaledLineHeight > 24.0 ? scaledLineHeight : 24.0;
    final tabHeight = contentHeight + 24.0 < 48.0 ? 48.0 : contentHeight + 24.0;

    return LayoutBuilder(
      builder: (context, constraints) {
        if (_tabsNeedOverflowSelector(
          context,
          constraints.maxWidth,
          fontSize,
        )) {
          return _buildCompactSectionSelector(theme);
        }
        return _buildDirectTabs(theme, tabHeight);
      },
    );
  }

  bool _tabsNeedOverflowSelector(
    BuildContext context,
    double availableWidth,
    double fontSize,
  ) {
    if (!availableWidth.isFinite) return false;
    final scaledFontSize = MediaQuery.textScalerOf(context).scale(fontSize);
    final effectiveScale = scaledFontSize > fontSize
        ? scaledFontSize / fontSize
        : 1.0;
    return availableWidth / effectiveScale < 600;
  }

  Widget _buildCompactSectionSelector(ThemeData theme) {
    final selectedDestination = widget.destinations[_selected.index];
    final localizations = MaterialLocalizations.of(context);
    return Material(
      color: theme.colorScheme.surfaceContainerLow,
      child: SafeArea(
        bottom: false,
        child: Padding(
          padding: const EdgeInsets.all(4),
          child: PopupMenuButton<Revision3ProjectWorkspaceSection>(
            key: const Key('revision3-project-workspace-section-selector'),
            initialValue: _selected,
            tooltip: localizations.showMenuTooltip,
            position: PopupMenuPosition.under,
            onSelected: _selectSection,
            itemBuilder: (context) => [
              for (final destination in widget.destinations)
                PopupMenuItem(
                  key: _sectionOptionKey(destination.section),
                  value: destination.section,
                  child: Row(
                    children: [
                      Icon(
                        destination.section == _selected
                            ? destination.selectedIcon
                            : destination.icon,
                      ),
                      const SizedBox(width: 12),
                      Expanded(child: Text(destination.label, maxLines: 2)),
                    ],
                  ),
                ),
            ],
            child: Semantics(
              key: const Key(
                'revision3-project-workspace-section-selector-semantics',
              ),
              button: true,
              label: selectedDestination.label,
              hint: localizations.showMenuTooltip,
              child: ExcludeSemantics(
                child: ConstrainedBox(
                  constraints: const BoxConstraints(minHeight: 48),
                  child: Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 12),
                    child: Row(
                      children: [
                        Icon(selectedDestination.selectedIcon),
                        const SizedBox(width: 12),
                        Expanded(
                          child: Text(
                            selectedDestination.label,
                            maxLines: 2,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                        const SizedBox(width: 8),
                        const Icon(Icons.arrow_drop_down),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildDirectTabs(ThemeData theme, double tabHeight) => Material(
    color: theme.colorScheme.surfaceContainerLow,
    child: SafeArea(
      bottom: false,
      child: TabBar(
        key: const Key('revision3-project-workspace-tabbar'),
        controller: _tabController,
        isScrollable: true,
        tabAlignment: TabAlignment.start,
        padding: const EdgeInsetsDirectional.only(start: 4),
        labelPadding: EdgeInsets.zero,
        onTap: (index) => _selectSection(widget.destinations[index].section),
        tabs: [
          for (final destination in widget.destinations)
            Tab(
              key: _tabKey(destination.section),
              height: tabHeight,
              child: KeyedSubtree(
                key: _tabVisibilityKeys[destination.section],
                child: Tooltip(
                  message: destination.label,
                  child: SizedBox(
                    height: tabHeight,
                    child: Padding(
                      padding: const EdgeInsets.symmetric(horizontal: 16),
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Icon(
                            destination.section == _selected
                                ? destination.selectedIcon
                                : destination.icon,
                          ),
                          const SizedBox(width: 8),
                          Text(destination.label, softWrap: false),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
            ),
        ],
      ),
    ),
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

Key _tabKey(Revision3ProjectWorkspaceSection section) =>
    Key('revision3-project-workspace-tab-${_sectionKey(section)}');

Key _pageKey(Revision3ProjectWorkspaceSection section) =>
    Key('revision3-project-workspace-page-${_sectionKey(section)}');

Key _sectionOptionKey(Revision3ProjectWorkspaceSection section) =>
    Key('revision3-project-workspace-section-option-${_sectionKey(section)}');

String _sectionKey(Revision3ProjectWorkspaceSection section) =>
    switch (section) {
      Revision3ProjectWorkspaceSection.home => 'home',
      Revision3ProjectWorkspaceSection.content => 'content',
      Revision3ProjectWorkspaceSection.story => 'story',
      Revision3ProjectWorkspaceSection.textVoice => 'text-voice',
      Revision3ProjectWorkspaceSection.testRelease => 'test-release',
    };
