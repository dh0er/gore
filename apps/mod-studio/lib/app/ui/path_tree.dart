import 'package:flutter/material.dart';

/// A lazy folder-tree browser over slash-separated leaf paths.
///
/// Renders the paths as a collapsible folder tree: folders before leaves,
/// case-insensitive alpha order, single-child folder chains compressed into
/// one row ("A" whose only child is folder "B" → "A/B"). Only the currently
/// expanded nodes are flattened into the lazy [ListView], so it stays cheap
/// regardless of leaf count.
///
/// The compressed tree is built once per [paths] list and rebuilt only when
/// the list *identity* changes — callers should pass an identity-stable list
/// (compute it once per source load, not per build).
class PathTreeBrowser extends StatefulWidget {
  const PathTreeBrowser({
    super.key,
    required this.paths,
    required this.selectedPath,
    required this.onSelect,
    required this.leafIcon,
    this.markedPaths = const {},
  });

  /// Slash-separated leaf paths to display as a tree.
  ///
  /// Contract: paths must be leaf-only — no path may also be a directory
  /// prefix of another path's segments (e.g. `['A', 'A/b']` violates this:
  /// `A` is both a leaf and the folder containing `A/b`). On a violation the
  /// prefix path is rendered as a leaf and the subtree beneath it is silently
  /// dropped.
  ///
  /// The list should also be identity-stable across builds: the tree is
  /// rebuilt only when the list *identity* changes, so in-place mutations are
  /// not picked up — pass a new list to change the contents.
  final List<String> paths;

  /// The highlighted leaf, if any.
  final String? selectedPath;

  /// Called with the leaf path when a leaf row is tapped.
  final ValueChanged<String> onSelect;

  /// Icon shown on leaf rows (e.g. [Icons.image_outlined]).
  final IconData leafIcon;

  /// Leaves that get a trailing check icon (e.g. staged replacements).
  final Set<String> markedPaths;

  @override
  State<PathTreeBrowser> createState() => _PathTreeBrowserState();
}

class _PathTreeBrowserState extends State<PathTreeBrowser> {
  // The set of expanded folder ids, plus the compressed tree built once per
  // paths list (rebuilt only when the list identity changes).
  final Set<String> _expanded = {};
  late _DisplayNode _root;

  @override
  void initState() {
    super.initState();
    _root = _buildTree(widget.paths);
  }

  @override
  void didUpdateWidget(PathTreeBrowser oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.paths, widget.paths)) {
      _root = _buildTree(widget.paths);
    }
  }

  @override
  Widget build(BuildContext context) {
    // Flatten only the currently-expanded nodes (default collapsed → just the
    // top level), so this stays cheap regardless of the leaf count.
    final visible = <_DisplayNode>[];
    void walk(List<_DisplayNode> nodes) {
      for (final n in nodes) {
        visible.add(n);
        if (!n.isLeaf && _expanded.contains(n.id)) walk(n.children!);
      }
    }

    walk(_root.children!);
    final scheme = Theme.of(context).colorScheme;
    return ListView.builder(
      itemCount: visible.length,
      itemBuilder: (c, i) {
        final n = visible[i];
        final indent = n.depth * 14.0;
        if (n.isLeaf) {
          final isMarked = widget.markedPaths.contains(n.assetPath);
          return Padding(
            padding: EdgeInsets.only(left: indent),
            child: ListTile(
              dense: true,
              selected: n.assetPath == widget.selectedPath,
              leading: Icon(widget.leafIcon, size: 18),
              title: Text(n.label, maxLines: 1, overflow: TextOverflow.ellipsis),
              trailing: isMarked ? const Icon(Icons.check, size: 16) : null,
              onTap: () => widget.onSelect(n.assetPath!),
            ),
          );
        }
        final isOpen = _expanded.contains(n.id);
        return Padding(
          padding: EdgeInsets.only(left: indent),
          child: ListTile(
            dense: true,
            leading: Icon(
              isOpen ? Icons.expand_more : Icons.chevron_right,
              size: 18,
            ),
            title: Row(
              children: [
                Icon(
                  isOpen ? Icons.folder_open : Icons.folder,
                  size: 18,
                  color: scheme.primary,
                ),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    n.label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                const SizedBox(width: 6),
                Text(
                  '${n.leafCount}',
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: scheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
            onTap: () => setState(() {
              if (isOpen) {
                _expanded.remove(n.id);
              } else {
                _expanded.add(n.id);
              }
            }),
          ),
        );
      },
    );
  }

  /// Build the compressed display tree for [paths].
  static _DisplayNode _buildTree(List<String> paths) {
    final raw = _RawNode('');
    for (final p in paths) {
      var node = raw;
      for (final seg in p.split('/')) {
        if (seg.isEmpty) continue;
        node = node.children.putIfAbsent(seg, () => _RawNode(seg));
      }
      node.assetPath = p;
    }
    final children = raw.children.values
        .map((c) => _toDisplay(c, 0, ''))
        .toList()
      ..sort(_nodeSort);
    return _DisplayNode(label: '', depth: -1, id: '', leafCount: 0)
      ..children = children;
  }

  /// Convert a raw segment node to a display node, compressing single-child
  /// folder chains ("A" whose only child is folder "B" → "A/B") so deep paths
  /// don't cost one click per level.
  static _DisplayNode _toDisplay(_RawNode raw, int depth, String parentId) {
    var label = raw.label;
    var cur = raw;
    while (cur.assetPath == null && cur.children.length == 1) {
      final only = cur.children.values.first;
      if (only.assetPath != null) break; // single child is a leaf — keep folder
      label = '$label/${only.label}';
      cur = only;
    }
    final id = parentId.isEmpty ? label : '$parentId/$label';
    if (cur.assetPath != null) {
      return _DisplayNode(
        label: label,
        depth: depth,
        id: cur.assetPath!,
        assetPath: cur.assetPath,
        leafCount: 1,
      );
    }
    final kids = cur.children.values
        .map((c) => _toDisplay(c, depth + 1, id))
        .toList()
      ..sort(_nodeSort);
    var count = 0;
    for (final k in kids) {
      count += k.leafCount;
    }
    return _DisplayNode(label: label, depth: depth, id: id, leafCount: count)
      ..children = kids;
  }

  /// Folders before leaves, then case-insensitive alpha.
  static int _nodeSort(_DisplayNode a, _DisplayNode b) {
    if (a.isLeaf != b.isLeaf) return a.isLeaf ? 1 : -1;
    return a.label.toLowerCase().compareTo(b.label.toLowerCase());
  }
}

/// Raw prefix-tree node: one per path segment, built directly from leaf paths.
class _RawNode {
  _RawNode(this.label);
  final String label;
  final Map<String, _RawNode> children = {};
  String? assetPath; // non-null = a leaf
}

/// Display node: a compressed folder (possibly merged segments, with [children])
/// or a leaf ([assetPath] set). [id] is the stable folder path used for
/// expand/collapse tracking; [leafCount] is the leaf count beneath it.
class _DisplayNode {
  _DisplayNode({
    required this.label,
    required this.depth,
    required this.id,
    required this.leafCount,
    this.assetPath,
  });
  final String label;
  final int depth;
  final String id;
  final int leafCount;
  final String? assetPath;
  List<_DisplayNode>? children;
  bool get isLeaf => assetPath != null;
}
