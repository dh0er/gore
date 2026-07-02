import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import '../../app/domain/ui_settings.dart';
import '../../editor/domain/overrides_notifier.dart';
import '../../editor/ui/field_editor.dart';
import '../../l10n/app_localizations.dart';
import '../../loc/domain/loc_catalog_provider.dart';
import '../../loc/game_lang.dart';
import '../domain/catalog_provider.dart';
import '../domain/item_entry.dart';
import 'catalog_browser.dart';

/// Currently selected catalog item, shared by all [ItemsTab] instances so the
/// selection survives tab switches (and stays consistent between the Items
/// main tab and filtered embeddings such as the Changes tab).
final selectedItemProvider = StateProvider<CatalogItem?>((ref) => null);

/// Items main-tab layout: catalog browser on the left, field editor detail on
/// the right. Extracted from [HomePage] so the Changes tab can embed the same
/// view restricted to changed item ids via [onlyIds].
class ItemsTab extends ConsumerWidget {
  const ItemsTab({super.key, this.onlyIds});

  /// When non-null, restricts the browsable item universe to these class ids
  /// (e.g. the Changes tab passing only staged item ids). Null = full catalog.
  final Set<String>? onlyIds;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final selectedRaw = ref.watch(selectedItemProvider);
    // Re-resolve the selection against the current catalog so that loading or
    // resetting a dump re-renders the editor with the refreshed item (same id,
    // new fields/defaults) instead of the stale CatalogItem object.
    final selected = selectedRaw == null
        ? null
        : (ref
                  .watch(catalogProvider)
                  .value
                  ?.firstWhereOrNull((i) => i.id == selectedRaw.id) ??
              selectedRaw);
    final overridesState = ref.watch(overridesProvider);
    final scheme = Theme.of(context).colorScheme;
    final l10n = AppLocalizations.of(context);

    return Row(
      children: [
        // Left: catalog browser
        SizedBox(
          width: 560,
          child: CatalogBrowser(
            onlyIds: onlyIds,
            selected: selected,
            onItemSelected: (item) =>
                ref.read(selectedItemProvider.notifier).state = item,
          ),
        ),
        const VerticalDivider(width: 1),
        // Centre: field editor. Cap the editing column width and
        // centre it so the inputs don't stretch across the whole
        // window on wide displays.
        Expanded(
          child: selected == null
              ? Center(
                  child: Text(
                    l10n.selectAnItemToEdit,
                    style: TextStyle(color: scheme.onSurfaceVariant),
                  ),
                )
              : Align(
                  alignment: Alignment.topCenter,
                  child: ConstrainedBox(
                    constraints: const BoxConstraints(maxWidth: 720),
                    child: FieldEditor(
                      item: selected,
                      displayName: displayNameForItem(
                        selected,
                        ref.watch(locCatalogProvider).value ?? const {},
                        gameLangByCode(ref.watch(localeProvider)),
                      ),
                      pendingOverrides: {
                        for (final e in overridesState.entries.where(
                          (e) => e.classId == selected.id,
                        ))
                          e.field: e,
                      },
                      onOverrideChanged: (entry) => ref
                          .read(overridesProvider.notifier)
                          .setOverride(entry),
                    ),
                  ),
                ),
        ),
      ],
    );
  }
}
