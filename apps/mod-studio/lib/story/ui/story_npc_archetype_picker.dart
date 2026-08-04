import 'package:flutter/material.dart';

import '../../core/mod_ffi.dart';
import '../domain/story_npc_archetype_index.dart';

/// Localized copy supplied by the surrounding Story workspace.
///
/// Keeping text outside the picker lets this isolated component land without
/// modifying generated localization files. The workspace integration owns the
/// eventual AppLocalizations mapping.
final class StoryNpcArchetypePickerLabels {
  const StoryNpcArchetypePickerLabels({
    required this.title,
    required this.search,
    required this.showExperimental,
    required this.offlineQualified,
    required this.experimentalStaticLinkage,
    required this.empty,
    required this.spawnClass,
    required this.aiConfigClass,
    required this.characterDefinitionClass,
    required this.actorBlueprint,
    required this.bodyBlueprintFamily,
    required this.humanBaseFamily,
    required this.humanWomanFamily,
    required this.otherFamily,
  });

  final String title;
  final String search;
  final String showExperimental;
  final String offlineQualified;
  final String experimentalStaticLinkage;
  final String empty;
  final String spawnClass;
  final String aiConfigClass;
  final String characterDefinitionClass;
  final String actorBlueprint;
  final String bodyBlueprintFamily;
  final String humanBaseFamily;
  final String humanWomanFamily;
  final String otherFamily;
}

/// Open the qualification-aware picker and return only a curated catalog ID.
Future<String?> showStoryNpcArchetypePicker({
  required BuildContext context,
  required StoryNpcArchetypeIndex index,
  required StoryNpcArchetypePickerLabels labels,
}) => showDialog<String>(
  context: context,
  builder: (context) => AlertDialog(
    title: Text(labels.title),
    contentPadding: const EdgeInsets.fromLTRB(24, 12, 24, 24),
    content: SizedBox(
      width: 760,
      height: 620,
      child: StoryNpcArchetypePicker(
        index: index,
        labels: labels,
        onSelected: (catalogId) => Navigator.of(context).pop(catalogId),
      ),
    ),
  ),
);

/// Searchable, virtualized view over a trusted [StoryNpcArchetypeIndex].
///
/// Experimental rows remain useful for inspection but never call
/// [onSelected]. The only value crossing this UI boundary is a curated Story
/// catalog ID that the existing adapter can resolve again.
class StoryNpcArchetypePicker extends StatefulWidget {
  const StoryNpcArchetypePicker({
    required this.index,
    required this.labels,
    required this.onSelected,
    super.key,
  });

  final StoryNpcArchetypeIndex index;
  final StoryNpcArchetypePickerLabels labels;
  final ValueChanged<String> onSelected;

  @override
  State<StoryNpcArchetypePicker> createState() =>
      _StoryNpcArchetypePickerState();
}

class _StoryNpcArchetypePickerState extends State<StoryNpcArchetypePicker> {
  final TextEditingController _searchController = TextEditingController();
  String _query = '';
  bool _showExperimental = false;

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final rows = widget.index.search(
      _query,
      includeExperimental: _showExperimental,
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        TextField(
          key: const Key('story-npc-archetype-search'),
          controller: _searchController,
          autofocus: true,
          textInputAction: TextInputAction.search,
          decoration: InputDecoration(
            labelText: widget.labels.search,
            prefixIcon: const Icon(Icons.search),
            border: const OutlineInputBorder(),
            isDense: true,
          ),
          onChanged: (value) => setState(() => _query = value),
        ),
        const SizedBox(height: 8),
        SwitchListTile.adaptive(
          key: const Key('story-npc-archetype-show-experimental'),
          contentPadding: EdgeInsets.zero,
          title: Text(widget.labels.showExperimental),
          value: _showExperimental,
          onChanged: (value) => setState(() => _showExperimental = value),
        ),
        const Divider(height: 1),
        Expanded(
          child: rows.isEmpty
              ? Center(
                  child: Text(
                    widget.labels.empty,
                    key: const Key('story-npc-archetype-empty'),
                    textAlign: TextAlign.center,
                  ),
                )
              : ListView.builder(
                  key: const Key('story-npc-archetype-results'),
                  itemCount: rows.length,
                  itemBuilder: (context, index) => _ArchetypeTile(
                    row: rows[index],
                    labels: widget.labels,
                    onSelected: widget.onSelected,
                  ),
                ),
        ),
      ],
    );
  }
}

class _ArchetypeTile extends StatelessWidget {
  const _ArchetypeTile({
    required this.row,
    required this.labels,
    required this.onSelected,
  });

  final StoryNpcArchetypeRow row;
  final StoryNpcArchetypePickerLabels labels;
  final ValueChanged<String> onSelected;

  @override
  Widget build(BuildContext context) {
    final catalogId = row.curatedCatalogId;
    final enabled = row.selectable && catalogId != null;
    final badge = enabled
        ? labels.offlineQualified
        : labels.experimentalStaticLinkage;
    return Semantics(
      key: ValueKey<String>('story-npc-archetype-${row.spawnClass}'),
      button: enabled,
      enabled: enabled,
      label: '${row.label}, $badge',
      child: ListTile(
        enabled: enabled,
        isThreeLine: true,
        leading: Icon(
          enabled ? Icons.verified_outlined : Icons.science_outlined,
        ),
        title: Row(
          children: <Widget>[
            Expanded(
              child: Text(
                row.label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ),
            const SizedBox(width: 8),
            Chip(visualDensity: VisualDensity.compact, label: Text(badge)),
          ],
        ),
        subtitle: Text(
          '${labels.spawnClass}: ${row.spawnClass}\n'
          '${labels.aiConfigClass}: ${row.aiConfigClass}  •  '
          '${labels.characterDefinitionClass}: '
          '${row.characterDefinitionClass}\n'
          '${labels.actorBlueprint}: ${row.actorBlueprint}  •  '
          '${labels.bodyBlueprintFamily}: ${_familyLabel(row, labels)}',
          maxLines: 4,
          overflow: TextOverflow.ellipsis,
        ),
        onTap: enabled ? () => onSelected(catalogId) : null,
      ),
    );
  }
}

String _familyLabel(
  StoryNpcArchetypeRow row,
  StoryNpcArchetypePickerLabels labels,
) => switch (row.bodyBlueprintFamily) {
  AuthoringNpcCatalogBlueprintFamily.humanBase => labels.humanBaseFamily,
  AuthoringNpcCatalogBlueprintFamily.humanWoman => labels.humanWomanFamily,
  AuthoringNpcCatalogBlueprintFamily.other => labels.otherFamily,
};
