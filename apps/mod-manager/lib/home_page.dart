import 'package:flutter/material.dart';
import 'l10n/app_localizations.dart';
import 'settings/ui/settings_tab.dart';

/// Skeleton home: a two-tab scaffold. The Mods tab is a placeholder until the
/// library/loadout UI lands; Settings is functional (game exe + language).
class HomePage extends StatelessWidget {
  const HomePage({super.key});

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final scheme = Theme.of(context).colorScheme;

    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.appTitle),
        scrolledUnderElevation: 0,
      ),
      body: DefaultTabController(
        length: 2,
        child: Column(
          children: [
            Container(
              color: scheme.surfaceContainerLowest,
              child: Row(
                children: [
                  Expanded(
                    child: TabBar(
                      isScrollable: true,
                      // Material 3 defaults scrollable tab bars to a 52px
                      // leading inset (TabAlignment.startOffset); start flush
                      // with just a small gap instead.
                      tabAlignment: TabAlignment.start,
                      padding: const EdgeInsetsDirectional.only(start: 4),
                      tabs: [
                        Tab(
                          icon: const Icon(Icons.extension_outlined),
                          text: l10n.tabMods,
                        ),
                        Tab(
                          icon: const Icon(Icons.settings_outlined),
                          text: l10n.tabSettings,
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
            Expanded(
              child: TabBarView(
                children: [
                  // Mods: placeholder — library list/reorder/conflicts UI is
                  // a separate task.
                  Center(child: Text(l10n.tabMods)),
                  const SettingsTab(),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
