import 'dart:async';

import 'package:flutter/material.dart';

import '../catalog/domain/field_schema.dart';
import '../core/mod_ffi.dart';
import '../l10n/app_localizations.dart';
import 'revision3_item_catalog.dart';
import 'revision3_item_patch_authoring.dart';

/// Programmatic navigation to one existing managed ItemPatch at an exact
/// project checkpoint.
///
/// Requests may be buffered while the matching Items surface and its current
/// native catalog proof are loading. The result is `true` only after the
/// requested vanilla class is proven to have an ItemPatch in that exact
/// project root, project ID, revision, and canonical head. A base-game catalog
/// entry without an existing patch is never treated as a match.
final class Revision3ItemsViewController {
  Object? _attachment;
  Object? _unavailableAttachment;
  String? _projectRoot;
  String? _projectId;
  int? _projectRevision;
  String? _projectHeadCanonicalJson;
  Future<bool> Function(
    String vanillaClass, {
    required String expectedProjectRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required String expectedProjectHeadCanonicalJson,
  })?
  _openVanillaClass;
  _PendingItemsNavigation? _bufferedNavigation;
  final Set<_PendingItemsNavigation> _forwardedNavigations = {};
  bool _disposed = false;

  /// Opens [vanillaClass] only when it is an existing ItemPatch at the exact
  /// supplied managed-project checkpoint.
  Future<bool> openVanillaClassAtCheckpoint(
    String vanillaClass, {
    required String projectRoot,
    required String projectId,
    required int projectRevision,
    required String projectHeadCanonicalJson,
  }) {
    final navigation = _PendingItemsNavigation(
      vanillaClass: vanillaClass,
      projectRoot: projectRoot,
      projectId: projectId,
      projectRevision: projectRevision,
      projectHeadCanonicalJson: projectHeadCanonicalJson,
    );
    if (_disposed || !navigation.isValid) {
      navigation.result.complete(false);
      return navigation.result.future;
    }
    if (_attachment != null) return _forward(navigation);
    if (_unavailableAttachment != null) {
      navigation.result.complete(false);
      return navigation.result.future;
    }
    final superseded = _bufferedNavigation;
    _bufferedNavigation = navigation;
    if (superseded != null && !superseded.result.isCompleted) {
      superseded.result.complete(false);
    }
    return navigation.result.future;
  }

  /// Permanently releases the controller and fails all outstanding requests.
  void dispose() {
    if (_disposed) return;
    _disposed = true;
    _attachment = null;
    _unavailableAttachment = null;
    _projectRoot = null;
    _projectId = null;
    _projectRevision = null;
    _projectHeadCanonicalJson = null;
    _openVanillaClass = null;
    _cancelNavigations();
  }

  Future<bool> _forward(_PendingItemsNavigation navigation) {
    final open = _openVanillaClass;
    if (open == null || !_matches(navigation)) {
      navigation.result.complete(false);
      return navigation.result.future;
    }
    _forwardedNavigations.add(navigation);
    Future<bool>.sync(
      () => open(
        navigation.vanillaClass,
        expectedProjectRoot: navigation.projectRoot,
        expectedProjectId: navigation.projectId,
        expectedProjectRevision: navigation.projectRevision,
        expectedProjectHeadCanonicalJson: navigation.projectHeadCanonicalJson,
      ),
    ).then(
      (resolved) => _completeForwarded(navigation, resolved),
      onError: (_, _) => _completeForwarded(navigation, false),
    );
    return navigation.result.future;
  }

  void _completeForwarded(_PendingItemsNavigation navigation, bool resolved) {
    _forwardedNavigations.remove(navigation);
    if (!navigation.result.isCompleted) {
      navigation.result.complete(resolved && _matches(navigation));
    }
  }

  bool _matches(_PendingItemsNavigation navigation) =>
      navigation.projectRoot == _projectRoot &&
      navigation.projectId == _projectId &&
      navigation.projectRevision == _projectRevision &&
      navigation.projectHeadCanonicalJson == _projectHeadCanonicalJson;

  bool _attach(
    Object attachment, {
    required String projectRoot,
    required String projectId,
    required int projectRevision,
    required String projectHeadCanonicalJson,
    required Future<bool> Function(
      String vanillaClass, {
      required String expectedProjectRoot,
      required String expectedProjectId,
      required int expectedProjectRevision,
      required String expectedProjectHeadCanonicalJson,
    })
    openVanillaClass,
  }) {
    final bindingValid =
        projectRoot.isNotEmpty &&
        projectId.isNotEmpty &&
        projectRevision >= 0 &&
        projectHeadCanonicalJson.isNotEmpty;
    if (_disposed ||
        !bindingValid ||
        (_attachment != null && !identical(_attachment, attachment))) {
      _rejectBufferedNavigation();
      return false;
    }
    _attachment = attachment;
    _unavailableAttachment = null;
    _projectRoot = projectRoot;
    _projectId = projectId;
    _projectRevision = projectRevision;
    _projectHeadCanonicalJson = projectHeadCanonicalJson;
    _openVanillaClass = openVanillaClass;
    _cancelMismatchedForwardedNavigations();
    final buffered = _bufferedNavigation;
    _bufferedNavigation = null;
    if (buffered != null) _forward(buffered);
    return true;
  }

  void _markUnavailable(Object attachment) {
    if (_disposed) return;
    if (_attachment != null && !identical(_attachment, attachment)) return;
    if (identical(_attachment, attachment)) {
      _attachment = null;
      _projectRoot = null;
      _projectId = null;
      _projectRevision = null;
      _projectHeadCanonicalJson = null;
      _openVanillaClass = null;
      _cancelForwardedNavigations();
    }
    _unavailableAttachment = attachment;
    _rejectBufferedNavigation();
  }

  void _detach(Object attachment) {
    if (identical(_unavailableAttachment, attachment)) {
      _unavailableAttachment = null;
    }
    if (!identical(_attachment, attachment)) return;
    _attachment = null;
    _projectRoot = null;
    _projectId = null;
    _projectRevision = null;
    _projectHeadCanonicalJson = null;
    _openVanillaClass = null;
    _cancelForwardedNavigations();
  }

  bool _isAttachedTo(Object attachment) =>
      !_disposed && identical(_attachment, attachment);

  void _cancelMismatchedForwardedNavigations() {
    final stale = _forwardedNavigations
        .where((navigation) => !_matches(navigation))
        .toList(growable: false);
    _forwardedNavigations.removeAll(stale);
    for (final navigation in stale) {
      if (!navigation.result.isCompleted) navigation.result.complete(false);
    }
  }

  void _rejectBufferedNavigation() {
    final buffered = _bufferedNavigation;
    _bufferedNavigation = null;
    if (buffered != null && !buffered.result.isCompleted) {
      buffered.result.complete(false);
    }
  }

  void _cancelForwardedNavigations() {
    final forwarded = _forwardedNavigations.toList(growable: false);
    _forwardedNavigations.clear();
    for (final navigation in forwarded) {
      if (!navigation.result.isCompleted) navigation.result.complete(false);
    }
  }

  void _cancelNavigations() {
    _rejectBufferedNavigation();
    _cancelForwardedNavigations();
  }
}

final class _PendingItemsNavigation {
  _PendingItemsNavigation({
    required this.vanillaClass,
    required this.projectRoot,
    required this.projectId,
    required this.projectRevision,
    required this.projectHeadCanonicalJson,
  });

  final String vanillaClass;
  final String projectRoot;
  final String projectId;
  final int projectRevision;
  final String projectHeadCanonicalJson;
  final Completer<bool> result = Completer<bool>();

  bool get isValid =>
      vanillaClass.isNotEmpty &&
      vanillaClass.trim() == vanillaClass &&
      projectRoot.isNotEmpty &&
      projectId.isNotEmpty &&
      projectId.trim() == projectId &&
      projectRevision >= 0 &&
      projectHeadCanonicalJson.isNotEmpty;
}

/// Familiar item browser with an exact managed-R3 authoring mode.
///
/// Without [authoring] it remains a bundled, read-only reference. With an
/// exact-current authoring service it can publish semantic ItemPatch project
/// changes, but it never gains build, deploy, game, or save authority.
class Revision3ItemsView extends StatefulWidget {
  const Revision3ItemsView({
    this.load = loadRevision3BundledItemCatalog,
    this.authoring,
    this.authoringRequiresReopen = false,
    this.onRecoverAuthoring,
    this.onDirtyChanged,
    this.onSavingChanged,
    this.mutationsEnabled = true,
    this.controller,
    super.key,
  });

  final Revision3ItemCatalogLoader load;
  final Revision3ItemPatchAuthoringService? authoring;
  final bool authoringRequiresReopen;
  final VoidCallback? onRecoverAuthoring;
  final ValueChanged<bool>? onDirtyChanged;
  final ValueChanged<bool>? onSavingChanged;
  final bool mutationsEnabled;
  final Revision3ItemsViewController? controller;

  @override
  State<Revision3ItemsView> createState() => _Revision3ItemsViewState();
}

class _Revision3ItemsViewState extends State<Revision3ItemsView> {
  Future<Revision3ItemCatalog>? _catalog;
  Future<Revision3ItemPatchCatalog>? _authoringCatalog;
  final TextEditingController _search = TextEditingController();
  String _query = '';
  Revision3ItemCategory? _category;
  String? _selectedId;
  bool _compactDetailVisible = false;
  final Map<String, Map<String, AuthoringRevision3ItemScalarValue>>
  _authoringDrafts = {};
  bool _lastReportedDirty = false;
  bool _saveInFlight = false;
  Object? _activeSaveOwner;
  int _authoringScopeEpoch = 0;
  int _itemsNavigationEpoch = 0;
  int _itemsNavigationRequestGeneration = 0;

  @override
  void initState() {
    super.initState();
    _loadCurrentMode();
    _attachController(widget.controller);
  }

  @override
  void didUpdateWidget(covariant Revision3ItemsView oldWidget) {
    super.didUpdateWidget(oldWidget);
    final oldAuthoring = oldWidget.authoring;
    final newAuthoring = widget.authoring;
    final authoringModeChanged =
        (oldAuthoring == null) != (newAuthoring == null);
    final authoringProjectScopeChanged =
        oldAuthoring != null &&
        newAuthoring != null &&
        (oldAuthoring.projectScopeIdentity !=
                newAuthoring.projectScopeIdentity ||
            oldAuthoring.projectId != newAuthoring.projectId);
    final authoringCheckpointChanged =
        oldAuthoring != null &&
        newAuthoring != null &&
        !authoringProjectScopeChanged &&
        (oldAuthoring.projectRevision != newAuthoring.projectRevision ||
            oldAuthoring.expectedHead.canonicalJson !=
                newAuthoring.expectedHead.canonicalJson);
    final bundledLoaderChanged =
        oldAuthoring == null &&
        newAuthoring == null &&
        oldWidget.load != widget.load;
    final controllerChanged = !identical(
      oldWidget.controller,
      widget.controller,
    );
    if (controllerChanged) oldWidget.controller?._detach(this);
    if (authoringModeChanged ||
        authoringProjectScopeChanged ||
        bundledLoaderChanged ||
        oldWidget.authoringRequiresReopen != widget.authoringRequiresReopen) {
      _resetAndReload();
    } else if (authoringCheckpointChanged) {
      _reloadAuthoringCheckpoint();
    } else {
      _attachController(widget.controller);
    }
    if (!identical(oldWidget.onDirtyChanged, widget.onDirtyChanged)) {
      oldWidget.onDirtyChanged?.call(false);
      _reportDirty(force: true);
    }
    if (!identical(oldWidget.onSavingChanged, widget.onSavingChanged)) {
      oldWidget.onSavingChanged?.call(false);
      widget.onSavingChanged?.call(_saveInFlight);
    }
  }

  @override
  void dispose() {
    _itemsNavigationEpoch++;
    _itemsNavigationRequestGeneration++;
    widget.controller?._detach(this);
    _search.dispose();
    _lastReportedDirty = false;
    widget.onDirtyChanged?.call(false);
    widget.onSavingChanged?.call(false);
    super.dispose();
  }

  void _resetAndReload() {
    _authoringScopeEpoch++;
    _itemsNavigationEpoch++;
    _itemsNavigationRequestGeneration++;
    widget.controller?._detach(this);
    _search.clear();
    _query = '';
    _category = null;
    _selectedId = null;
    _compactDetailVisible = false;
    _authoringDrafts.clear();
    final wasSaving = _saveInFlight;
    _activeSaveOwner = null;
    _saveInFlight = false;
    _reportDirty();
    if (wasSaving) widget.onSavingChanged?.call(false);
    _loadCurrentMode();
    _attachController(widget.controller);
  }

  void _loadCurrentMode() {
    if (widget.authoringRequiresReopen) {
      _catalog = null;
      _authoringCatalog = null;
      return;
    }
    final authoring = widget.authoring;
    if (authoring == null) {
      _authoringCatalog = null;
      _catalog = _itemCatalogObservedLoad(widget.load);
    } else {
      _catalog = null;
      _authoringCatalog = _itemCatalogObservedLoad(authoring.loadCatalog);
    }
  }

  void _reloadAuthoringCheckpoint() {
    final authoring = widget.authoring;
    if (authoring == null) return;
    _itemsNavigationEpoch++;
    _itemsNavigationRequestGeneration++;
    widget.controller?._detach(this);
    _catalog = null;
    _authoringCatalog = _itemCatalogObservedLoad(authoring.loadCatalog);
    _attachController(widget.controller);
  }

  void _attachController(Revision3ItemsViewController? controller) {
    if (controller == null) return;
    final authoring = widget.authoring;
    final catalogFuture = _authoringCatalog;
    if (widget.authoringRequiresReopen ||
        authoring == null ||
        catalogFuture == null) {
      controller._markUnavailable(this);
      return;
    }
    controller._attach(
      this,
      projectRoot: authoring.projectScopeIdentity,
      projectId: authoring.projectId,
      projectRevision: authoring.projectRevision,
      projectHeadCanonicalJson: authoring.expectedHead.canonicalJson,
      openVanillaClass:
          (
            vanillaClass, {
            required expectedProjectRoot,
            required expectedProjectId,
            required expectedProjectRevision,
            required expectedProjectHeadCanonicalJson,
          }) => _openVanillaClassAtCheckpoint(
            controller,
            catalogFuture,
            vanillaClass,
            expectedProjectRoot: expectedProjectRoot,
            expectedProjectId: expectedProjectId,
            expectedProjectRevision: expectedProjectRevision,
            expectedProjectHeadCanonicalJson: expectedProjectHeadCanonicalJson,
          ),
    );
  }

  Future<bool> _openVanillaClassAtCheckpoint(
    Revision3ItemsViewController controller,
    Future<Revision3ItemPatchCatalog> catalogFuture,
    String vanillaClass, {
    required String expectedProjectRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required String expectedProjectHeadCanonicalJson,
  }) async {
    final navigationEpoch = _itemsNavigationEpoch;
    final requestGeneration = ++_itemsNavigationRequestGeneration;
    if (!_matchesItemsCheckpoint(
      controller,
      catalogFuture,
      expectedProjectRoot: expectedProjectRoot,
      expectedProjectId: expectedProjectId,
      expectedProjectRevision: expectedProjectRevision,
      expectedProjectHeadCanonicalJson: expectedProjectHeadCanonicalJson,
    )) {
      return false;
    }
    late final Revision3ItemPatchCatalog catalog;
    try {
      catalog = await catalogFuture;
    } catch (_) {
      return false;
    }
    if (navigationEpoch != _itemsNavigationEpoch ||
        requestGeneration != _itemsNavigationRequestGeneration ||
        !_matchesItemsCheckpoint(
          controller,
          catalogFuture,
          expectedProjectRoot: expectedProjectRoot,
          expectedProjectId: expectedProjectId,
          expectedProjectRevision: expectedProjectRevision,
          expectedProjectHeadCanonicalJson: expectedProjectHeadCanonicalJson,
        ) ||
        catalog.projectId != expectedProjectId ||
        catalog.projectRevision != expectedProjectRevision ||
        catalog.head.canonicalJson != expectedProjectHeadCanonicalJson) {
      return false;
    }
    Revision3ItemPatchChoice? choice;
    for (final candidate in catalog.choices) {
      if (candidate.vanillaClass == vanillaClass) {
        choice = candidate;
        break;
      }
    }
    if (choice == null || !choice.hasPatch) return false;
    await WidgetsBinding.instance.endOfFrame;
    if (navigationEpoch != _itemsNavigationEpoch ||
        requestGeneration != _itemsNavigationRequestGeneration ||
        !_matchesItemsCheckpoint(
          controller,
          catalogFuture,
          expectedProjectRoot: expectedProjectRoot,
          expectedProjectId: expectedProjectId,
          expectedProjectRevision: expectedProjectRevision,
          expectedProjectHeadCanonicalJson: expectedProjectHeadCanonicalJson,
        ) ||
        _saveInFlight) {
      return false;
    }
    setState(() {
      _search.clear();
      _query = '';
      _category = null;
      _selectedId = choice!.vanillaClass;
      _compactDetailVisible = true;
    });
    return true;
  }

  bool _matchesItemsCheckpoint(
    Revision3ItemsViewController controller,
    Future<Revision3ItemPatchCatalog> catalogFuture, {
    required String expectedProjectRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required String expectedProjectHeadCanonicalJson,
  }) {
    final authoring = widget.authoring;
    return mounted &&
        identical(widget.controller, controller) &&
        controller._isAttachedTo(this) &&
        !widget.authoringRequiresReopen &&
        authoring != null &&
        identical(_authoringCatalog, catalogFuture) &&
        authoring.projectScopeIdentity == expectedProjectRoot &&
        authoring.projectId == expectedProjectId &&
        authoring.projectRevision == expectedProjectRevision &&
        authoring.expectedHead.canonicalJson ==
            expectedProjectHeadCanonicalJson;
  }

  void _retry() => setState(_resetAndReload);

  void _changeQuery(String value) => setState(() {
    _query = value;
    _compactDetailVisible = false;
  });

  void _clearQuery() {
    _search.clear();
    _changeQuery('');
  }

  void _selectCategory(Revision3ItemCategory? category) => setState(() {
    _category = category;
    _compactDetailVisible = false;
  });

  void _selectItem(Revision3ItemCatalogEntry item) => setState(() {
    _selectedId = item.id;
    _compactDetailVisible = true;
  });

  @override
  Widget build(BuildContext context) {
    if (widget.authoringRequiresReopen) {
      return _ItemCatalogLoadError(
        error: const Revision3ItemPatchRequiresReopenException(),
        onRecover: widget.onRecoverAuthoring,
      );
    }
    final authoring = widget.authoring;
    if (authoring != null) {
      return FutureBuilder<Revision3ItemPatchCatalog>(
        future: _authoringCatalog,
        builder: (context, snapshot) {
          if (snapshot.connectionState != ConnectionState.done) {
            return const Center(
              child: CircularProgressIndicator(
                key: Key('revision3-items-loading'),
              ),
            );
          }
          if (snapshot.hasError) {
            return _ItemCatalogLoadError(
              error: snapshot.error!,
              onRetry: _retry,
              onRecover: widget.onRecoverAuthoring,
            );
          }
          return _authoringCatalogBody(
            context,
            snapshot.requireData,
            authoring,
          );
        },
      );
    }
    return FutureBuilder<Revision3ItemCatalog>(
      future: _catalog,
      builder: (context, snapshot) {
        if (snapshot.connectionState != ConnectionState.done) {
          return const Center(
            child: CircularProgressIndicator(
              key: Key('revision3-items-loading'),
            ),
          );
        }
        if (snapshot.hasError) {
          return _ItemCatalogLoadError(error: snapshot.error!, onRetry: _retry);
        }
        return _catalogBody(context, snapshot.requireData);
      },
    );
  }

  Widget _catalogBody(BuildContext context, Revision3ItemCatalog catalog) {
    final foldedQuery = _query.trim().toLowerCase();
    final filtered = catalog.items
        .where((item) {
          if (_category != null && item.category != _category) {
            return false;
          }
          return foldedQuery.isEmpty ||
              item.id.toLowerCase().contains(foldedQuery) ||
              item.displayName.toLowerCase().contains(foldedQuery);
        })
        .toList(growable: false);
    final counts = <Revision3ItemCategory, int>{};
    for (final item in catalog.items) {
      counts.update(item.category, (count) => count + 1, ifAbsent: () => 1);
    }

    return LayoutBuilder(
      builder: (context, constraints) {
        final wide = constraints.maxWidth >= 760;
        Revision3ItemCatalogEntry? selected;
        for (final item in filtered) {
          if (item.id == _selectedId) {
            selected = item;
            break;
          }
        }
        if (wide && selected == null && filtered.isNotEmpty) {
          selected = filtered.first;
        }

        final browser = _ItemBrowser(
          items: filtered,
          totalCount: catalog.items.length,
          counts: counts,
          selectedId: selected?.id,
          category: _category,
          searchController: _search,
          onQueryChanged: _changeQuery,
          onClearQuery: _clearQuery,
          onCategoryChanged: _selectCategory,
          onSelected: _selectItem,
          enabled: true,
        );
        if (!wide) {
          if (_compactDetailVisible && selected != null) {
            return _ItemDetails(
              item: selected,
              compact: true,
              onBack: () => setState(() => _compactDetailVisible = false),
            );
          }
          return browser;
        }

        final browserWidth = (constraints.maxWidth * 0.38).clamp(330.0, 440.0);
        return Row(
          key: const Key('revision3-items-wide-layout'),
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            SizedBox(width: browserWidth, child: browser),
            const VerticalDivider(width: 1),
            Expanded(
              child: selected == null
                  ? _EmptySelection(queryActive: filtered.isEmpty)
                  : _ItemDetails(item: selected, compact: false),
            ),
          ],
        );
      },
    );
  }

  Widget _authoringCatalogBody(
    BuildContext context,
    Revision3ItemPatchCatalog catalog,
    Revision3ItemPatchAuthoringService authoring,
  ) {
    final foldedQuery = _query.trim().toLowerCase();
    final presentations = <String, Revision3ItemCatalogEntry>{};
    final filteredChoices = catalog.choices
        .where((choice) {
          final category = Revision3ItemCategory.parse(
            choice.category.wireName,
            'native item category',
          );
          if (_category != null && category != _category) return false;
          return choice.matches(foldedQuery);
        })
        .toList(growable: false);
    for (final choice in filteredChoices) {
      presentations[choice.vanillaClass] = _itemPresentation(choice);
    }
    final counts = <Revision3ItemCategory, int>{};
    for (final choice in catalog.choices) {
      final category = Revision3ItemCategory.parse(
        choice.category.wireName,
        'native item category',
      );
      counts.update(category, (count) => count + 1, ifAbsent: () => 1);
    }

    return LayoutBuilder(
      builder: (context, constraints) {
        final wide = constraints.maxWidth >= 760;
        Revision3ItemPatchChoice? selected;
        for (final choice in filteredChoices) {
          if (choice.vanillaClass == _selectedId) {
            selected = choice;
            break;
          }
        }
        if (wide && selected == null && filteredChoices.isNotEmpty) {
          selected = filteredChoices.first;
        }
        final items = filteredChoices
            .map((choice) => presentations[choice.vanillaClass]!)
            .toList(growable: false);
        final browser = _ItemBrowser(
          items: items,
          totalCount: catalog.choices.length,
          counts: counts,
          selectedId: selected?.vanillaClass,
          category: _category,
          searchController: _search,
          onQueryChanged: _changeQuery,
          onClearQuery: _clearQuery,
          onCategoryChanged: _selectCategory,
          onSelected: (item) {
            final choice = catalog.choices.firstWhere(
              (candidate) => candidate.vanillaClass == item.id,
            );
            _selectAuthoringItem(choice);
          },
          enabled: widget.mutationsEnabled && !_saveInFlight,
        );
        Widget details(Revision3ItemPatchChoice choice, bool compact) {
          final scopeEpoch = _authoringScopeEpoch;
          return _EditableItemDetails(
            key: ValueKey(
              'revision3-items-editor-${choice.stableKey}-${catalog.projectRevision}',
            ),
            choice: choice,
            item: presentations[choice.vanillaClass]!,
            authoring: authoring,
            initialDesiredOverrides: _authoringDrafts[choice.stableKey],
            onDraftChanged: (desired) {
              if (_sameAuthoringValues(desired, choice.currentOverrides)) {
                _authoringDrafts.remove(choice.stableKey);
              } else {
                _authoringDrafts[choice.stableKey] =
                    Map<String, AuthoringRevision3ItemScalarValue>.unmodifiable(
                      desired,
                    );
              }
              _reportDirty();
            },
            onPublished: () =>
                _removePublishedDraft(scopeEpoch, choice.stableKey),
            onReloadAfterFailure: _retry,
            mutationsEnabled: widget.mutationsEnabled && !_saveInFlight,
            onSaveStateChanged: _setSaveInFlight,
            compact: compact,
            onBack: compact
                ? () => setState(() => _compactDetailVisible = false)
                : null,
          );
        }

        if (!wide) {
          if (_compactDetailVisible && selected != null) {
            return details(selected, true);
          }
          return browser;
        }
        final browserWidth = (constraints.maxWidth * 0.38).clamp(330.0, 440.0);
        return Row(
          key: const Key('revision3-items-wide-layout'),
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            SizedBox(width: browserWidth, child: browser),
            const VerticalDivider(width: 1),
            Expanded(
              child: selected == null
                  ? _EmptySelection(queryActive: filteredChoices.isEmpty)
                  : details(selected, false),
            ),
          ],
        );
      },
    );
  }

  void _selectAuthoringItem(Revision3ItemPatchChoice choice) => setState(() {
    _selectedId = choice.vanillaClass;
    _compactDetailVisible = true;
  });

  void _reportDirty({bool force = false}) {
    final dirty = _authoringDrafts.isNotEmpty;
    if (!force && dirty == _lastReportedDirty) return;
    _lastReportedDirty = dirty;
    widget.onDirtyChanged?.call(dirty);
  }

  void _removePublishedDraft(int scopeEpoch, String stableKey) {
    if (!mounted || scopeEpoch != _authoringScopeEpoch) return;
    _authoringDrafts.remove(stableKey);
    _reportDirty();
  }

  void _setSaveInFlight(Object owner, bool saving) {
    if (!mounted) return;
    if (saving) {
      if (_activeSaveOwner != null && !identical(_activeSaveOwner, owner)) {
        return;
      }
      _activeSaveOwner = owner;
    } else {
      if (!identical(_activeSaveOwner, owner)) return;
      _activeSaveOwner = null;
    }
    if (_saveInFlight == saving) return;
    setState(() => _saveInFlight = saving);
    widget.onSavingChanged?.call(saving);
  }
}

/// FutureBuilder may not attach its listener until the next rebuild after a
/// Retry tap. Observe the future immediately as well, so an already-completed
/// error is rendered instead of escaping through that hand-off window.
Future<T> _itemCatalogObservedLoad<T>(Future<T> Function() load) {
  final future = Future<T>.sync(load);
  future.ignore();
  return future;
}

class _ItemCatalogLoadError extends StatelessWidget {
  const _ItemCatalogLoadError({
    required this.error,
    this.onRetry,
    this.onRecover,
  });

  final Object error;
  final VoidCallback? onRetry;
  final VoidCallback? onRecover;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final requiresRecovery = error is Revision3ItemPatchRequiresReopenException;
    final unsupported = error is Revision3ItemPatchUnsupportedSchemaException;
    final message = switch (error) {
      Revision3ItemPatchRequiresReopenException() =>
        l10n.managedItemsCatalogRequiresReopen,
      Revision3ItemPatchStaleCheckpointException() ||
      Revision3ItemPatchNoChangesException() => l10n.managedItemsCatalogStale,
      Revision3ItemPatchUnsupportedSchemaException() =>
        l10n.managedItemsCatalogUnsupported,
      _ => l10n.managedItemsCatalogLoadUnexpected,
    };
    final action = requiresRecovery
        ? onRecover
        : unsupported
        ? null
        : onRetry;
    return LayoutBuilder(
      builder: (context, constraints) {
        final minimumHeight = (constraints.maxHeight - 32).clamp(
          0.0,
          double.infinity,
        );
        return SingleChildScrollView(
          key: const Key('revision3-items-load-error-scroll'),
          padding: const EdgeInsets.all(16),
          child: ConstrainedBox(
            constraints: BoxConstraints(minHeight: minimumHeight),
            child: Center(
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 560),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.error_outline,
                      size: 36,
                      color: Theme.of(context).colorScheme.error,
                    ),
                    const SizedBox(height: 12),
                    Text(
                      l10n.managedItemsCatalogLoadTitle,
                      style: Theme.of(context).textTheme.titleMedium,
                      textAlign: TextAlign.center,
                    ),
                    const SizedBox(height: 8),
                    Text(
                      message,
                      key: const Key('revision3-items-load-error'),
                      textAlign: TextAlign.center,
                    ),
                    if (action != null) ...[
                      const SizedBox(height: 16),
                      OutlinedButton.icon(
                        key: Key(
                          requiresRecovery
                              ? 'revision3-items-recover'
                              : 'revision3-items-retry',
                        ),
                        onPressed: action,
                        icon: Icon(
                          requiresRecovery
                              ? Icons.health_and_safety_outlined
                              : Icons.refresh,
                        ),
                        label: Text(
                          requiresRecovery
                              ? l10n.managedProjectRecoveryTry
                              : l10n.managedItemsCatalogReload,
                        ),
                      ),
                    ],
                  ],
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}

class _ItemBrowser extends StatelessWidget {
  const _ItemBrowser({
    required this.items,
    required this.totalCount,
    required this.counts,
    required this.selectedId,
    required this.category,
    required this.searchController,
    required this.onQueryChanged,
    required this.onClearQuery,
    required this.onCategoryChanged,
    required this.onSelected,
    required this.enabled,
  });

  final List<Revision3ItemCatalogEntry> items;
  final int totalCount;
  final Map<Revision3ItemCategory, int> counts;
  final String? selectedId;
  final Revision3ItemCategory? category;
  final TextEditingController searchController;
  final ValueChanged<String> onQueryChanged;
  final VoidCallback onClearQuery;
  final ValueChanged<Revision3ItemCategory?> onCategoryChanged;
  final ValueChanged<Revision3ItemCatalogEntry> onSelected;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final search = Padding(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
      child: TextField(
        key: const Key('revision3-items-search'),
        controller: searchController,
        enabled: enabled,
        onChanged: onQueryChanged,
        textInputAction: TextInputAction.search,
        decoration: InputDecoration(
          labelText: l10n.searchItems,
          prefixIcon: const Icon(Icons.search),
          suffixIcon: searchController.text.isEmpty
              ? null
              : IconButton(
                  key: const Key('revision3-items-clear-search'),
                  tooltip: l10n.clearAll,
                  onPressed: enabled ? onClearQuery : null,
                  icon: const Icon(Icons.clear),
                ),
          border: const OutlineInputBorder(),
        ),
      ),
    );
    final categories = SingleChildScrollView(
      key: const Key('revision3-items-category-scroll'),
      scrollDirection: Axis.horizontal,
      padding: const EdgeInsets.fromLTRB(12, 4, 12, 8),
      child: Row(
        children: [
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 4),
            child: ChoiceChip(
              key: const Key('revision3-items-category-all'),
              selected: category == null,
              label: Text(l10n.categoryWithCount(l10n.changesAll, totalCount)),
              onSelected: enabled ? (_) => onCategoryChanged(null) : null,
            ),
          ),
          for (final itemCategory in Revision3ItemCategory.values)
            if (counts[itemCategory] case final count?)
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 4),
                child: ChoiceChip(
                  key: ValueKey(
                    'revision3-items-category-${itemCategory.name}',
                  ),
                  selected: category == itemCategory,
                  label: Text(
                    l10n.categoryWithCount(
                      _categoryLabel(l10n, itemCategory),
                      count,
                    ),
                  ),
                  onSelected: enabled
                      ? (_) => onCategoryChanged(itemCategory)
                      : null,
                ),
              ),
        ],
      ),
    );
    Widget itemTile(Revision3ItemCatalogEntry item) => ListTile(
      key: ValueKey('revision3-items-result-${item.id}'),
      selected: item.id == selectedId,
      leading: const Icon(Icons.inventory_2_outlined),
      title: Text(
        item.displayName,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      subtitle: Text(item.id, maxLines: 1, overflow: TextOverflow.ellipsis),
      trailing: const Icon(Icons.chevron_right),
      enabled: enabled,
      onTap: enabled ? () => onSelected(item) : null,
    );
    final empty = Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Text(
          l10n.noItemsMatch,
          key: const Key('revision3-items-empty'),
          textAlign: TextAlign.center,
        ),
      ),
    );

    return LayoutBuilder(
      builder: (context, constraints) {
        if (constraints.maxHeight < 360) {
          return CustomScrollView(
            key: const Key('revision3-items-browser'),
            slivers: [
              SliverToBoxAdapter(child: search),
              SliverToBoxAdapter(child: categories),
              const SliverToBoxAdapter(child: Divider(height: 1)),
              if (items.isEmpty)
                SliverFillRemaining(hasScrollBody: false, child: empty)
              else
                SliverList.builder(
                  key: const Key('revision3-items-results'),
                  itemCount: items.length,
                  itemBuilder: (context, index) => itemTile(items[index]),
                ),
            ],
          );
        }
        return Column(
          key: const Key('revision3-items-browser'),
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            search,
            categories,
            const Divider(height: 1),
            Expanded(
              child: items.isEmpty
                  ? empty
                  : ListView.builder(
                      key: const Key('revision3-items-results'),
                      itemCount: items.length,
                      itemBuilder: (context, index) => itemTile(items[index]),
                    ),
            ),
          ],
        );
      },
    );
  }
}

Revision3ItemCatalogEntry _itemPresentation(Revision3ItemPatchChoice choice) =>
    Revision3ItemCatalogEntry(
      id: choice.vanillaClass,
      displayName: choice.displayName,
      category: Revision3ItemCategory.parse(
        choice.category.wireName,
        'native item category',
      ),
      fields: choice.fields
          .map(
            (field) => FieldSchema(
              name: field.name,
              type: switch (field.scalarType) {
                AuthoringRevision3ItemScalarType.integer => FieldType.int_,
                AuthoringRevision3ItemScalarType.float_ => FieldType.float_,
                AuthoringRevision3ItemScalarType.boolean => FieldType.bool_,
              },
              minValue: field.minimumValue?.value as num?,
              maxValue: field.maximumValue?.value as num?,
              defaultValue: field.defaultValue?.value,
            ),
          )
          .toList(growable: false),
    );

class _EditableItemDetails extends StatefulWidget {
  const _EditableItemDetails({
    required this.choice,
    required this.item,
    required this.authoring,
    required this.initialDesiredOverrides,
    required this.onDraftChanged,
    required this.onPublished,
    required this.onReloadAfterFailure,
    required this.mutationsEnabled,
    required this.onSaveStateChanged,
    required this.compact,
    this.onBack,
    super.key,
  });

  final Revision3ItemPatchChoice choice;
  final Revision3ItemCatalogEntry item;
  final Revision3ItemPatchAuthoringService authoring;
  final Map<String, AuthoringRevision3ItemScalarValue>? initialDesiredOverrides;
  final ValueChanged<Map<String, AuthoringRevision3ItemScalarValue>>
  onDraftChanged;
  final VoidCallback onPublished;
  final VoidCallback onReloadAfterFailure;
  final bool mutationsEnabled;
  final void Function(Object owner, bool saving) onSaveStateChanged;
  final bool compact;
  final VoidCallback? onBack;

  @override
  State<_EditableItemDetails> createState() => _EditableItemDetailsState();
}

class _EditableItemDetailsState extends State<_EditableItemDetails> {
  final Map<String, TextEditingController> _controllers = {};
  final Map<String, String?> _errors = {};
  late Map<String, AuthoringRevision3ItemScalarValue> _desired;
  final Object _saveOwner = Object();
  bool _saving = false;
  bool _published = false;
  String? _saveError;
  bool _canReloadAfterFailure = false;

  @override
  void initState() {
    super.initState();
    _reset();
  }

  @override
  void didUpdateWidget(covariant _EditableItemDetails oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.choice, widget.choice)) _reset();
  }

  @override
  void dispose() {
    _disposeControllers();
    super.dispose();
  }

  void _disposeControllers() {
    for (final controller in _controllers.values) {
      controller.dispose();
    }
    _controllers.clear();
  }

  void _reset() {
    _disposeControllers();
    _errors.clear();
    _desired = Map<String, AuthoringRevision3ItemScalarValue>.from(
      widget.initialDesiredOverrides ?? widget.choice.currentOverrides,
    );
    for (final field in widget.choice.fields) {
      final value = _desired[field.name] ?? field.defaultValue;
      _controllers[field.name] = TextEditingController(
        text: value == null ? '' : _authoringValueText(value),
      );
    }
    _saving = false;
    _published = false;
    _saveError = null;
    _canReloadAfterFailure = false;
  }

  bool get _dirty =>
      !_sameAuthoringValues(_desired, widget.choice.currentOverrides);

  void _notifyDraftChanged() => widget.onDraftChanged(_desired);

  void _addOverride(Revision3ItemPatchFieldChoice field) {
    if (!widget.mutationsEnabled || _saving || _published) return;
    final value =
        field.defaultValue ??
        switch (field.scalarType) {
          AuthoringRevision3ItemScalarType.integer =>
            AuthoringRevision3ItemScalarValue.integer(0),
          AuthoringRevision3ItemScalarType.float_ =>
            AuthoringRevision3ItemScalarValue.float(0),
          AuthoringRevision3ItemScalarType.boolean =>
            AuthoringRevision3ItemScalarValue.boolean(false),
        };
    setState(() {
      _desired[field.name] = value;
      _controllers[field.name]!.text = _authoringValueText(value);
      _errors[field.name] = null;
      _saveError = null;
      _canReloadAfterFailure = false;
    });
    _notifyDraftChanged();
  }

  void _removeOverride(String name) {
    if (!widget.mutationsEnabled || _saving || _published) return;
    setState(() {
      _desired.remove(name);
      _errors[name] = null;
      _saveError = null;
      _canReloadAfterFailure = false;
    });
    _notifyDraftChanged();
  }

  void _clearAllOverrides() {
    if (!widget.mutationsEnabled || _saving || _published) return;
    setState(() {
      _desired.clear();
      _errors.clear();
      _saveError = null;
      _canReloadAfterFailure = false;
    });
    _notifyDraftChanged();
  }

  void _changeNumeric(Revision3ItemPatchFieldChoice field, String raw) {
    if (!widget.mutationsEnabled || _saving || _published) return;
    AuthoringRevision3ItemScalarValue? value;
    String? error;
    final trimmed = raw.trim();
    if (field.scalarType == AuthoringRevision3ItemScalarType.integer) {
      final parsed = int.tryParse(trimmed);
      if (parsed != null) {
        try {
          value = AuthoringRevision3ItemScalarValue.integer(parsed);
        } on FormatException {
          value = null;
        }
      }
    } else {
      final parsed = double.tryParse(trimmed);
      if (parsed != null && parsed.isFinite) {
        value = AuthoringRevision3ItemScalarValue.float(parsed);
      }
    }
    if (value == null) {
      error = AppLocalizations.of(context).managedItemsInvalidNumber;
    } else if (!field.accepts(value)) {
      error = AppLocalizations.of(context).managedItemsNumberOutsideNativeRange(
        _authoringBoundText(field.minimumValue!),
        _authoringBoundText(field.maximumValue!),
      );
      value = null;
    }
    setState(() {
      _errors[field.name] = error;
      if (value != null) _desired[field.name] = value;
      _saveError = null;
      _canReloadAfterFailure = false;
    });
    if (value != null) _notifyDraftChanged();
  }

  void _changeBoolean(String name, bool value) {
    if (!widget.mutationsEnabled || _saving || _published) return;
    setState(() {
      _desired[name] = AuthoringRevision3ItemScalarValue.boolean(value);
      _saveError = null;
      _canReloadAfterFailure = false;
    });
    _notifyDraftChanged();
  }

  Future<void> _save() async {
    if (!widget.mutationsEnabled ||
        _saving ||
        _published ||
        _errors.values.any((error) => error != null)) {
      return;
    }
    final submittedOverrides =
        Map<String, AuthoringRevision3ItemScalarValue>.unmodifiable(
          Map<String, AuthoringRevision3ItemScalarValue>.from(_desired),
        );
    widget.onSaveStateChanged(_saveOwner, true);
    setState(() {
      _saving = true;
      _saveError = null;
      _canReloadAfterFailure = false;
    });
    try {
      final publication = await widget.authoring.save(
        choice: widget.choice,
        desiredOverrides: submittedOverrides,
      );
      widget.onPublished();
      if (!mounted) return;
      setState(() {
        _saving = false;
        _published = true;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            AppLocalizations.of(
              context,
            ).managedItemsSaved(publication.projectRevision),
          ),
        ),
      );
    } catch (error) {
      if (!mounted) return;
      final l10n = AppLocalizations.of(context);
      final failure = switch (error) {
        Revision3ItemPatchStaleCheckpointException() => (
          l10n.managedItemsSaveStale,
          true,
        ),
        Revision3ItemPatchRequiresReopenException() => (
          l10n.managedItemsSaveRequiresReopen,
          false,
        ),
        Revision3ItemPatchNoChangesException() => (
          l10n.managedItemsSaveNoChanges,
          true,
        ),
        Revision3ItemPatchUnsupportedSchemaException() => (
          l10n.managedItemsSaveUnsupported,
          true,
        ),
        _ => (l10n.managedItemsSaveUnexpected, false),
      };
      setState(() {
        _saving = false;
        _saveError = failure.$1;
        _canReloadAfterFailure = failure.$2;
      });
    } finally {
      widget.onSaveStateChanged(_saveOwner, false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final choice = widget.choice;
    final canSave =
        !_saving &&
        widget.mutationsEnabled &&
        !_published &&
        _errors.values.every((error) => error == null) &&
        (choice.canEdit ? _dirty : choice.hasPatch && _desired.isEmpty);
    return ListView(
      key: ValueKey('revision3-items-details-${widget.item.id}'),
      padding: const EdgeInsets.fromLTRB(20, 16, 20, 32),
      children: [
        if (widget.compact)
          Align(
            alignment: Alignment.centerLeft,
            child: TextButton.icon(
              key: const Key('revision3-items-back'),
              onPressed: widget.mutationsEnabled ? widget.onBack : null,
              icon: const Icon(Icons.arrow_back),
              label: Text(l10n.tabItems),
            ),
          ),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            Chip(
              avatar: const Icon(Icons.verified_outlined, size: 18),
              label: Text(l10n.managedItemsExactSchemaBadge),
            ),
            Chip(
              avatar: const Icon(Icons.edit_outlined, size: 18),
              label: Text(l10n.managedItemsEditableBadge),
            ),
            Chip(
              avatar: const Icon(Icons.build_circle_outlined, size: 18),
              label: Text(l10n.managedItemsBuildPendingBadge),
            ),
            Chip(
              avatar: const Icon(Icons.inventory_2_outlined, size: 18),
              label: Text(_categoryLabel(l10n, widget.item.category)),
            ),
            if (choice.hasPatch)
              Chip(
                avatar: const Icon(Icons.change_circle_outlined, size: 18),
                label: Text(
                  l10n.managedItemsCurrentChanges(
                    choice.currentOverrides.length,
                  ),
                ),
              ),
          ],
        ),
        const SizedBox(height: 14),
        SelectableText(
          choice.displayName,
          key: const Key('revision3-items-detail-name'),
          style: Theme.of(context).textTheme.headlineSmall,
        ),
        const SizedBox(height: 4),
        SelectableText(
          choice.vanillaClass,
          key: const Key('revision3-items-detail-id'),
          style: Theme.of(context).textTheme.bodyMedium?.copyWith(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
            fontFamily: 'monospace',
          ),
        ),
        const SizedBox(height: 16),
        Card.filled(
          key: const Key('revision3-items-authoring-boundary'),
          child: Padding(
            padding: const EdgeInsets.all(14),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Icon(Icons.info_outline, size: 20),
                const SizedBox(width: 10),
                Expanded(child: Text(l10n.managedItemsAuthoringBoundary)),
              ],
            ),
          ),
        ),
        if (!choice.canEdit) ...[
          const SizedBox(height: 12),
          Card(
            color: Theme.of(context).colorScheme.errorContainer,
            child: Padding(
              padding: const EdgeInsets.all(14),
              child: Text(l10n.managedItemsUnsupportedSchema),
            ),
          ),
        ],
        const SizedBox(height: 20),
        LayoutBuilder(
          builder: (context, constraints) {
            final count = Text(
              l10n.managedItemsCurrentChanges(_desired.length),
              key: const Key('revision3-items-edit-count'),
              style: Theme.of(context).textTheme.titleMedium,
            );
            final clear = TextButton(
              key: const Key('revision3-items-clear-all'),
              onPressed: !widget.mutationsEnabled || _saving || _published
                  ? null
                  : _clearAllOverrides,
              child: Wrap(
                spacing: 8,
                runSpacing: 4,
                alignment: WrapAlignment.center,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: [
                  const Icon(Icons.restart_alt),
                  Text(l10n.managedItemsClearChanges),
                ],
              ),
            );
            if (_desired.isEmpty) return count;
            if (constraints.maxWidth < 520) {
              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [count, const SizedBox(height: 4), clear],
              );
            }
            return Row(
              children: [
                Expanded(child: count),
                Flexible(child: clear),
              ],
            );
          },
        ),
        const SizedBox(height: 8),
        if (choice.canEdit)
          for (final field in choice.fields)
            _EditableItemField(
              key: ValueKey(
                'revision3-items-edit-${choice.vanillaClass}-${field.name}',
              ),
              field: field,
              activeValue: _desired[field.name],
              controller: _controllers[field.name]!,
              error: _errors[field.name],
              enabled: widget.mutationsEnabled && !_saving && !_published,
              onAdd: () => _addOverride(field),
              onRemove: () => _removeOverride(field.name),
              onNumericChanged: (raw) => _changeNumeric(field, raw),
              onBooleanChanged: (value) => _changeBoolean(field.name, value),
            ),
        if (_saveError != null) ...[
          const SizedBox(height: 12),
          Card(
            color: Theme.of(context).colorScheme.errorContainer,
            child: Padding(
              padding: const EdgeInsets.all(14),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Text(
                    _saveError!,
                    key: const Key('revision3-items-save-error'),
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.onErrorContainer,
                    ),
                  ),
                  if (_canReloadAfterFailure) ...[
                    const SizedBox(height: 8),
                    Align(
                      alignment: Alignment.centerLeft,
                      child: TextButton.icon(
                        key: const Key('revision3-items-reload-after-error'),
                        onPressed: widget.onReloadAfterFailure,
                        icon: const Icon(Icons.refresh),
                        label: Text(l10n.managedItemsReloadDiscardDraft),
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
        ],
        const SizedBox(height: 18),
        FilledButton.icon(
          key: const Key('revision3-items-save'),
          onPressed: canSave ? _save : null,
          icon: _saving
              ? const SizedBox.square(
                  dimension: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : Icon(
                  _desired.isEmpty && choice.hasPatch
                      ? Icons.restore
                      : Icons.save_outlined,
                ),
          label: Text(
            _desired.isEmpty && choice.hasPatch
                ? l10n.managedItemsRevertItem
                : l10n.managedItemsSaveChanges,
          ),
        ),
        if (!canSave && !_saving && !_published) ...[
          const SizedBox(height: 8),
          Text(
            l10n.managedItemsNoUnsavedChanges,
            textAlign: TextAlign.center,
            style: Theme.of(context).textTheme.bodySmall,
          ),
        ],
      ],
    );
  }
}

class _EditableItemField extends StatelessWidget {
  const _EditableItemField({
    required this.field,
    required this.activeValue,
    required this.controller,
    required this.error,
    required this.enabled,
    required this.onAdd,
    required this.onRemove,
    required this.onNumericChanged,
    required this.onBooleanChanged,
    super.key,
  });

  final Revision3ItemPatchFieldChoice field;
  final AuthoringRevision3ItemScalarValue? activeValue;
  final TextEditingController controller;
  final String? error;
  final bool enabled;
  final VoidCallback onAdd;
  final VoidCallback onRemove;
  final ValueChanged<String> onNumericChanged;
  final ValueChanged<bool> onBooleanChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final defaultText = field.defaultValue == null
        ? l10n.managedItemsDefaultUnknown
        : l10n.managedItemsGameDefault(
            _authoringValueText(field.defaultValue!),
          );
    return Card(
      margin: const EdgeInsets.symmetric(vertical: 5),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    field.name,
                    style: Theme.of(
                      context,
                    ).textTheme.titleSmall?.copyWith(fontFamily: 'monospace'),
                  ),
                ),
                Chip(
                  label: Text(field.scalarType.wireName),
                  visualDensity: VisualDensity.compact,
                ),
              ],
            ),
            Text(
              defaultText,
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 10),
            if (activeValue == null)
              Align(
                alignment: Alignment.centerLeft,
                child: OutlinedButton.icon(
                  key: ValueKey('revision3-items-add-${field.name}'),
                  onPressed: enabled ? onAdd : null,
                  icon: const Icon(Icons.add),
                  label: Text(l10n.managedItemsChangeField),
                ),
              )
            else
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(
                    child:
                        field.scalarType ==
                            AuthoringRevision3ItemScalarType.boolean
                        ? SwitchListTile(
                            contentPadding: EdgeInsets.zero,
                            title: Text(l10n.managedItemsModValue),
                            value: activeValue!.booleanValue!,
                            onChanged: enabled ? onBooleanChanged : null,
                          )
                        : TextField(
                            key: ValueKey(
                              'revision3-items-value-${field.name}',
                            ),
                            controller: controller,
                            enabled: enabled,
                            decoration: InputDecoration(
                              labelText: l10n.managedItemsModValue,
                              errorText: error,
                              isDense: true,
                            ),
                            keyboardType:
                                field.scalarType ==
                                    AuthoringRevision3ItemScalarType.integer
                                ? const TextInputType.numberWithOptions(
                                    signed: true,
                                  )
                                : const TextInputType.numberWithOptions(
                                    signed: true,
                                    decimal: true,
                                  ),
                            onChanged: onNumericChanged,
                          ),
                  ),
                  const SizedBox(width: 8),
                  IconButton(
                    key: ValueKey('revision3-items-remove-${field.name}'),
                    tooltip: l10n.managedItemsUseGameDefault,
                    onPressed: enabled ? onRemove : null,
                    icon: const Icon(Icons.delete_outline),
                  ),
                ],
              ),
          ],
        ),
      ),
    );
  }
}

String _authoringValueText(AuthoringRevision3ItemScalarValue value) =>
    switch (value.type) {
      AuthoringRevision3ItemScalarType.integer => value.integerValue.toString(),
      AuthoringRevision3ItemScalarType.float_ => value.floatValue.toString(),
      AuthoringRevision3ItemScalarType.boolean => value.booleanValue.toString(),
    };

String _authoringBoundText(AuthoringRevision3ItemScalarValue value) =>
    switch (value.type) {
      AuthoringRevision3ItemScalarType.integer =>
        value.integerValue!.toString(),
      AuthoringRevision3ItemScalarType.float_ => value.floatValue!.toString(),
      AuthoringRevision3ItemScalarType.boolean =>
        value.booleanValue! ? 'true' : 'false',
    };

bool _sameAuthoringValues(
  Map<String, AuthoringRevision3ItemScalarValue> left,
  Map<String, AuthoringRevision3ItemScalarValue> right,
) {
  if (left.length != right.length) return false;
  for (final entry in left.entries) {
    final other = right[entry.key];
    if (other == null ||
        other.type != entry.value.type ||
        other.value != entry.value.value) {
      return false;
    }
  }
  return true;
}

class _ItemDetails extends StatelessWidget {
  const _ItemDetails({required this.item, required this.compact, this.onBack});

  final Revision3ItemCatalogEntry item;
  final bool compact;
  final VoidCallback? onBack;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final category = item.category;
    return ListView(
      key: ValueKey('revision3-items-details-${item.id}'),
      padding: const EdgeInsets.fromLTRB(20, 16, 20, 32),
      children: [
        if (compact)
          Align(
            alignment: Alignment.centerLeft,
            child: TextButton.icon(
              key: const Key('revision3-items-back'),
              onPressed: onBack,
              icon: const Icon(Icons.arrow_back),
              label: Text(l10n.tabItems),
            ),
          ),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            Chip(
              avatar: const Icon(Icons.sports_esports_outlined, size: 18),
              label: Text(l10n.managedContentScopeBaseGameLabel),
            ),
            Chip(
              avatar: const Icon(Icons.visibility_outlined, size: 18),
              label: Text(l10n.managedBaseGameBrowserInspectOnlyBadge),
            ),
            Chip(
              avatar: const Icon(Icons.inventory_2_outlined, size: 18),
              label: Text(l10n.managedItemsBundledReferenceBadge),
            ),
            Chip(label: Text(_categoryLabel(l10n, category))),
          ],
        ),
        const SizedBox(height: 14),
        SelectableText(
          item.displayName,
          key: const Key('revision3-items-detail-name'),
          style: Theme.of(context).textTheme.headlineSmall,
        ),
        const SizedBox(height: 8),
        Text(
          l10n.managedStoryWorkbenchTechnicalIdLabel,
          style: Theme.of(context).textTheme.labelMedium?.copyWith(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
        ),
        SelectableText(
          item.id,
          key: const Key('revision3-items-detail-id'),
          style: Theme.of(
            context,
          ).textTheme.bodyLarge?.copyWith(fontFamily: 'monospace'),
        ),
        const SizedBox(height: 16),
        Card.filled(
          key: const Key('revision3-items-bundled-boundary'),
          child: Padding(
            padding: const EdgeInsets.all(14),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Icon(Icons.info_outline, size: 20),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(l10n.managedItemsBundledReferenceBoundary),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 24),
        Text(
          l10n.categoryWithCount(l10n.sectionItemValues, item.fields.length),
          key: const Key('revision3-items-field-heading'),
          style: Theme.of(context).textTheme.titleMedium,
        ),
        const SizedBox(height: 8),
        if (item.fields.isEmpty)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 28),
            child: Text(
              l10n.managedItemsNoKnownFields,
              key: const Key('revision3-items-no-known-fields'),
              textAlign: TextAlign.center,
            ),
          )
        else
          for (final field in item.fields) _FieldCard(item: item, field: field),
      ],
    );
  }
}

class _FieldCard extends StatelessWidget {
  const _FieldCard({required this.item, required this.field});

  final Revision3ItemCatalogEntry item;
  final FieldSchema field;

  @override
  Widget build(BuildContext context) {
    final defaultValue = _displayDefault(field);
    return Card(
      key: ValueKey('revision3-items-field-${item.id}-${field.name}'),
      margin: const EdgeInsets.symmetric(vertical: 5),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            SelectableText(
              field.name,
              style: Theme.of(
                context,
              ).textTheme.titleSmall?.copyWith(fontFamily: 'monospace'),
            ),
            const SizedBox(height: 10),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                _FactChip(
                  icon: Icons.data_object,
                  text: _fieldType(field.type),
                ),
                if (defaultValue != null)
                  _FactChip(
                    key: ValueKey(
                      'revision3-items-field-${item.id}-${field.name}-default',
                    ),
                    icon: Icons.subdirectory_arrow_right,
                    text: '= $defaultValue',
                  ),
                if (field.minValue != null)
                  _FactChip(
                    icon: Icons.keyboard_arrow_up,
                    text: '\u2265 ${field.minValue}',
                  ),
                if (field.maxValue != null)
                  _FactChip(
                    icon: Icons.keyboard_arrow_down,
                    text: '\u2264 ${field.maxValue}',
                  ),
              ],
            ),
            if (field.type == FieldType.enum_ &&
                field.enumValues.isNotEmpty) ...[
              const SizedBox(height: 10),
              Text(
                field.enumValues.join(' \u00b7 '),
                maxLines: 3,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _FactChip extends StatelessWidget {
  const _FactChip({required this.icon, required this.text, super.key});

  final IconData icon;
  final String text;

  @override
  Widget build(BuildContext context) => Chip(
    avatar: Icon(icon, size: 16),
    label: Text(text),
    visualDensity: VisualDensity.compact,
  );
}

class _EmptySelection extends StatelessWidget {
  const _EmptySelection({required this.queryActive});

  final bool queryActive;

  @override
  Widget build(BuildContext context) => Center(
    child: Padding(
      padding: const EdgeInsets.all(24),
      child: Icon(
        queryActive ? Icons.search_off : Icons.inventory_2_outlined,
        size: 44,
        color: Theme.of(context).colorScheme.outline,
      ),
    ),
  );
}

String _categoryLabel(AppLocalizations l10n, Revision3ItemCategory category) =>
    switch (category) {
      Revision3ItemCategory.meleeWeapon => l10n.categoryMeleeWeapons,
      Revision3ItemCategory.rangedWeapon => l10n.categoryRangedWeapons,
      Revision3ItemCategory.ammunition => l10n.categoryAmmunition,
      Revision3ItemCategory.rune => l10n.categoryRunes,
      Revision3ItemCategory.scroll => l10n.categorySpellScrolls,
      Revision3ItemCategory.food => l10n.categoryFoodAndPotions,
      Revision3ItemCategory.misc => l10n.categoryMiscellaneous,
      Revision3ItemCategory.amulet => l10n.categoryAmulets,
      Revision3ItemCategory.armor => l10n.managedItemsCategoryArmor,
      Revision3ItemCategory.ring => l10n.categoryRings,
      Revision3ItemCategory.trophy => l10n.categoryAnimalTrophies,
      Revision3ItemCategory.writing => l10n.categoryWritings,
      Revision3ItemCategory.mission => l10n.categoryMissionItems,
      Revision3ItemCategory.key => l10n.categoryKeys,
      Revision3ItemCategory.special => l10n.managedItemsCategorySpecial,
    };

String _fieldType(FieldType type) => switch (type) {
  FieldType.int_ => 'int',
  FieldType.float_ => 'float',
  FieldType.bool_ => 'bool',
  FieldType.string_ => 'string',
  FieldType.enum_ => 'enum',
};

String? _displayDefault(FieldSchema field) {
  final value = field.defaultValue;
  if (value == null) return null;
  if (field.type == FieldType.string_) return '"$value"';
  if (field.type != FieldType.enum_ || value is! int) return value.toString();

  var memberIndex = -1;
  if (field.enumBackingValues.isNotEmpty) {
    memberIndex = field.enumBackingValues.indexOf(value);
  } else if (value >= 0 && value < field.enumValues.length) {
    memberIndex = value;
  }
  return memberIndex < 0
      ? value.toString()
      : '${field.enumValues[memberIndex]} ($value)';
}
