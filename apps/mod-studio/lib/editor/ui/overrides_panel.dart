import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;
import '../../audio/domain/audio_replacements_notifier.dart';
import '../../l10n/app_localizations.dart';
import '../../loc/domain/loc_edits_notifier.dart';
import '../../project/dialog_topics_notifier.dart';
import '../../scripts/domain/script_mods_notifier.dart';
import '../../textures/domain/texture_replacements_notifier.dart';
import '../domain/override_entry.dart';
import '../domain/overrides_notifier.dart';

/// Unified "Changes" panel: lists every staged mod change across all
/// domains (item value overrides, localized text edits, audio replacements,
/// texture replacements, AngelScript modules, runtime dialog topics), each row
/// individually removable,
/// searchable across all sections, with per-section and global clear actions.
class OverridesPanel extends ConsumerStatefulWidget {
  const OverridesPanel({super.key});

  @override
  ConsumerState<OverridesPanel> createState() => _OverridesPanelState();
}

class _OverridesPanelState extends ConsumerState<OverridesPanel> {
  final TextEditingController _searchController = TextEditingController();
  String _query = '';

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final overridesState = ref.watch(overridesProvider);
    final overrides      = ref.read(overridesProvider.notifier);
    final locState       = ref.watch(locEditsProvider);
    final locEdits       = ref.read(locEditsProvider.notifier);
    final audioState     = ref.watch(audioReplacementsProvider);
    final audio          = ref.read(audioReplacementsProvider.notifier);
    final textureState   = ref.watch(textureReplacementsProvider);
    final textures       = ref.read(textureReplacementsProvider.notifier);
    final scriptState    = ref.watch(scriptModsProvider);
    final scripts        = ref.read(scriptModsProvider.notifier);
    final topicState     = ref.watch(dialogTopicsProvider);
    final topics         = ref.read(dialogTopicsProvider.notifier);

    final scheme = Theme.of(context).colorScheme;
    final l10n   = AppLocalizations.of(context);

    // Case-insensitive substring filter over everything a row shows plus the
    // useful raw fields behind it. Matching mirrors the row rendering below.
    final q = _query.trim().toLowerCase();
    bool matches(Iterable<String> haystack) =>
        q.isEmpty || haystack.any((s) => s.toLowerCase().contains(q));

    final overrideEntries = <OverrideEntry>[
      for (final e in overridesState.entries)
        if (matches(['${e.classId}.${e.field}', '${e.oldValue} → ${e.newValue}'])) e,
    ];
    final locPairs = <_LocEditRow>[
      for (final outer in locState.edits.entries)
        for (final inner in outer.value.entries)
          if (matches([outer.key, inner.key, inner.value]))
            _LocEditRow(locId: outer.key, set: inner.key, text: inner.value),
    ];
    final audioEntries = <AudioReplacement>[
      for (final e in audioState.entries)
        if (matches([e.bank, e.sample, p.basename(e.wavPath)])) e,
    ];
    final textureEntries = <TextureReplacement>[
      for (final e in textureState.entries)
        if (matches([e.asset, p.basename(e.imagePath)])) e,
    ];
    final scriptEntries = <ScriptMod>[
      for (final e in scriptState.entries)
        if (matches([e.moduleName, e.relPath])) e,
    ];
    final topicEntries = <DialogTopicDefinition>[
      for (final e in topicState.entries)
        if (matches([e.id, e.participantName, e.topicClass, e.sentinelClass])) e,
    ];

    final total   = overridesState.count + locState.entryCount + audioState.count + textureState.count + scriptState.count + topicState.count;
    final isEmpty = total == 0;
    final visible = overrideEntries.length + locPairs.length + audioEntries.length + textureEntries.length + scriptEntries.length + topicEntries.length;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // Slim header: search across all sections + clear-all.
        Padding(
          padding: const EdgeInsets.fromLTRB(4, 4, 4, 8),
          child: Row(
            children: [
              Expanded(
                child: TextField(
                  controller: _searchController,
                  decoration: InputDecoration(
                    labelText: l10n.searchChanges,
                    prefixIcon: const Icon(Icons.search),
                    isDense: true,
                  ),
                  onChanged: (v) => setState(() => _query = v),
                ),
              ),
              const SizedBox(width: 8),
              TextButton.icon(
                icon: const Icon(Icons.clear_all, size: 18),
                label: Text(l10n.clearAll),
                onPressed: isEmpty
                    ? null
                    : () {
                        overrides.clearAll();
                        locEdits.clearAll();
                        audio.clearAll();
                        textures.clearAll();
                        scripts.clearAll();
                        topics.clearAll();
                      },
              ),
            ],
          ),
        ),
        Expanded(
          child: isEmpty
              ? Center(
                  child: Text(
                    l10n.noPendingOverrides,
                    textAlign: TextAlign.center,
                    style: TextStyle(color: scheme.onSurfaceVariant),
                  ),
                )
              : visible == 0
                  ? Center(
                      child: Text(
                        l10n.noChangesMatch,
                        textAlign: TextAlign.center,
                        style: TextStyle(color: scheme.onSurfaceVariant),
                      ),
                    )
                  : ListView(
                      padding: const EdgeInsets.only(bottom: 12),
                      children: [
                        if (overrideEntries.isNotEmpty) ...[
                          _SectionHeader(
                            l10n.sectionItemValues,
                            clearKey: const ValueKey('clear-section-items'),
                            onClear: overrides.clearAll,
                          ),
                          for (final entry in overrideEntries)
                            _OverrideRow(entry: entry, notifier: overrides),
                        ],
                        if (locPairs.isNotEmpty) ...[
                          _SectionHeader(
                            l10n.sectionLocalizedText,
                            clearKey: const ValueKey('clear-section-loc'),
                            onClear: locEdits.clearAll,
                          ),
                          for (final row in locPairs)
                            _LocRow(row: row, notifier: locEdits),
                        ],
                        if (topicEntries.isNotEmpty) ...[
                          _SectionHeader(
                            'Runtime dialog topics',
                            clearKey: const ValueKey('clear-section-dialog-topics'),
                            onClear: topics.clearAll,
                          ),
                          for (final entry in topicEntries)
                            _DialogTopicRow(entry: entry, notifier: topics),
                        ],
                        if (audioEntries.isNotEmpty) ...[
                          _SectionHeader(
                            l10n.tabAudio,
                            clearKey: const ValueKey('clear-section-audio'),
                            onClear: audio.clearAll,
                          ),
                          for (final entry in audioEntries)
                            _AudioRow(entry: entry, notifier: audio),
                        ],
                        if (textureEntries.isNotEmpty) ...[
                          _SectionHeader(
                            l10n.tabTextures,
                            clearKey: const ValueKey('clear-section-textures'),
                            onClear: textures.clearAll,
                          ),
                          for (final entry in textureEntries)
                            _TextureRow(entry: entry, notifier: textures),
                        ],
                        if (scriptEntries.isNotEmpty) ...[
                          _SectionHeader(
                            l10n.tabScripts,
                            clearKey: const ValueKey('clear-section-scripts'),
                            onClear: scripts.clearAll,
                          ),
                          for (final entry in scriptEntries)
                            _ScriptRow(entry: entry, notifier: scripts),
                        ],
                      ],
                    ),
        ),
      ],
    );
  }
}

class _SectionHeader extends StatelessWidget {
  const _SectionHeader(this.title, {required this.onClear, this.clearKey});

  final String title;

  /// Clears ALL changes of this section's domain — intentionally independent
  /// of the current search filter (the whole group, not just visible rows).
  final VoidCallback onClear;
  final Key? clearKey;

  @override
  Widget build(BuildContext context) {
    final theme  = Theme.of(context);
    final scheme = theme.colorScheme;
    return Padding(
      padding: const EdgeInsets.fromLTRB(8, 8, 4, 0),
      child: Row(
        children: [
          Expanded(
            child: Text(
              title,
              style: theme.textTheme.labelSmall?.copyWith(
                color: scheme.onSurfaceVariant,
                fontWeight: FontWeight.w700,
                letterSpacing: 0.6,
              ),
            ),
          ),
          IconButton(
            key: clearKey,
            icon: const Icon(Icons.delete_sweep_outlined, size: 18),
            visualDensity: VisualDensity.compact,
            tooltip: AppLocalizations.of(context).clearSection,
            onPressed: onClear,
          ),
        ],
      ),
    );
  }
}

class _OverrideRow extends StatelessWidget {
  const _OverrideRow({required this.entry, required this.notifier});

  final OverrideEntry entry;
  final OverridesNotifier notifier;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 4),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '${entry.classId}.${entry.field}',
                  style: const TextStyle(fontFamily: 'Consolas', fontSize: 12),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                Text(
                  '${entry.oldValue} → ${entry.newValue}',
                  style: TextStyle(
                    fontSize: 12,
                    color: scheme.primary,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ],
            ),
          ),
          IconButton(
            icon: const Icon(Icons.remove_circle_outline, size: 18),
            tooltip: AppLocalizations.of(context).removeOverride,
            onPressed: () => notifier.removeOverride(entry.key),
          ),
        ],
      ),
    );
  }
}

class _LocEditRow {
  const _LocEditRow({required this.locId, required this.set, required this.text});

  final String locId;
  final String set;
  final String text;
}

class _LocRow extends StatelessWidget {
  const _LocRow({required this.row, required this.notifier});

  final _LocEditRow row;
  final LocEditsNotifier notifier;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 4),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '${row.locId}  ·  ${row.set}',
                  style: const TextStyle(fontFamily: 'Consolas', fontSize: 12),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                Text(
                  row.text,
                  style: TextStyle(
                    fontSize: 12,
                    color: scheme.primary,
                    fontWeight: FontWeight.w600,
                  ),
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
          IconButton(
            icon: const Icon(Icons.remove_circle_outline, size: 18),
            tooltip: AppLocalizations.of(context).removeOverride,
            onPressed: () => notifier.removeEdit(row.locId, row.set),
          ),
        ],
      ),
    );
  }
}

class _DialogTopicRow extends StatelessWidget {
  const _DialogTopicRow({required this.entry, required this.notifier});

  final DialogTopicDefinition entry;
  final DialogTopicsNotifier notifier;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 4),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '${entry.id}  ·  ${entry.participantName}',
                  style: const TextStyle(fontFamily: 'Consolas', fontSize: 12),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                Text(
                  '${entry.topicClass}  →  ${entry.sentinelClass}',
                  style: TextStyle(
                    fontSize: 12,
                    color: scheme.primary,
                    fontWeight: FontWeight.w600,
                  ),
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
          IconButton(
            key: ValueKey<String>('remove-dialog-topic-${entry.key}'),
            icon: const Icon(Icons.remove_circle_outline, size: 18),
            tooltip: AppLocalizations.of(context).removeOverride,
            onPressed: () => notifier.remove(entry.id),
          ),
        ],
      ),
    );
  }
}

class _AudioRow extends StatelessWidget {
  const _AudioRow({required this.entry, required this.notifier});

  final AudioReplacement entry;
  final AudioReplacementsNotifier notifier;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 4),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '${entry.bank}  ·  ${entry.sample}',
                  style: const TextStyle(fontFamily: 'Consolas', fontSize: 12),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                Text(
                  p.basename(entry.wavPath),
                  style: TextStyle(
                    fontSize: 12,
                    color: scheme.primary,
                    fontWeight: FontWeight.w600,
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
          IconButton(
            icon: const Icon(Icons.remove_circle_outline, size: 18),
            tooltip: AppLocalizations.of(context).removeOverride,
            onPressed: () => notifier.remove(entry.key),
          ),
        ],
      ),
    );
  }
}

class _TextureRow extends StatelessWidget {
  const _TextureRow({required this.entry, required this.notifier});

  final TextureReplacement entry;
  final TextureReplacementsNotifier notifier;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 4),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  entry.asset,
                  style: const TextStyle(fontFamily: 'Consolas', fontSize: 12),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                Text(
                  p.basename(entry.imagePath),
                  style: TextStyle(
                    fontSize: 12,
                    color: scheme.primary,
                    fontWeight: FontWeight.w600,
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
          IconButton(
            icon: const Icon(Icons.remove_circle_outline, size: 18),
            tooltip: AppLocalizations.of(context).removeOverride,
            onPressed: () => notifier.remove(entry.key),
          ),
        ],
      ),
    );
  }
}

class _ScriptRow extends StatelessWidget {
  const _ScriptRow({required this.entry, required this.notifier});

  final ScriptMod entry;
  final ScriptModsNotifier notifier;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 4),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '${entry.op == ScriptOp.add ? 'add' : 'edit'}  ·  ${entry.moduleName}',
                  style: const TextStyle(fontFamily: 'Consolas', fontSize: 12),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                Builder(builder: (_) {
                  // Mirror the tab badge / Build-Deploy gate: a mod is only "compiled" when its
                  // on-disk .as still matches the compiled hash (scriptCompileFresh), not merely
                  // when a mini exists (entry.compiled). Otherwise this panel shows green
                  // "compiled" while deploy stays blocked after a source edit.
                  final fresh = scriptCompileFresh(entry);
                  return Text(
                    fresh ? 'compiled' : 'not compiled / edited',
                    style: TextStyle(
                      fontSize: 12,
                      color: fresh ? scheme.primary : scheme.error,
                      fontWeight: FontWeight.w600,
                    ),
                  );
                }),
              ],
            ),
          ),
          IconButton(
            icon: const Icon(Icons.remove_circle_outline, size: 18),
            tooltip: AppLocalizations.of(context).removeOverride,
            onPressed: () => notifier.remove(entry.key),
          ),
        ],
      ),
    );
  }
}
