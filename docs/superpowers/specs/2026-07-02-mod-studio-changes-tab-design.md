# Mod-Studio Changes-Tab Redesign — Design

Date: 2026-07-02
Status: approved (session auto-approve)

The Änderungen/Changes tab becomes a two-column view instead of the flat list:

- Left: 230px sidebar (`SidebarTile`) with six entries + live counts:
  **Alle** (`changesAll`, new l10n key, all 12 arbs), **Items** (`tabItems`,
  count = `overridesProvider.count`), **Dialoge** (`tabDialogs`, count =
  `locEditsProvider` edited ids), **Audio** (`tabAudio`, staged replacements),
  **Texturen** (`tabTextures`, staged replacements), **Scripte** (`tabScripts`,
  staged mods).
- Right content per selection:
  - **Alle**: the existing `OverridesPanel` unchanged (own header + clear-all).
  - **Items**: the Items main-tab layout (catalog browser + field editor)
    filtered to changed item ids.
  - **Dialoge**: `DialogeTab` filtered to edited loc ids.
  - **Audio**: `AudioTab` showing only staged samples per bank.
  - **Texturen**: `TextureTab` tree/list over staged asset paths only.
  - **Scripte**: `ScriptTab` tree/flat list over staged relPaths only.

## Reuse mechanism (no duplication)

Each main-tab widget gains an optional filter constructor param, default null/off
(main tabs unchanged):

- `CatalogBrowser({Set<String>? onlyIds})` — items filtered to ids; categories
  with zero remaining items hidden. The Items main-tab Row currently inlined in
  `home_page.dart` is extracted into `lib/catalog/ui/items_tab.dart`
  (`ItemsTab({Set<String>? onlyIds})`) and used by both home_page and ChangesTab.
- `DialogeTab({Set<String>? onlyIds})` — dialog rows built from the loc catalog
  restricted to those ids (groups/counts follow).
- `AudioTab({bool onlyStaged = false})` — sample lists restricted to staged
  replacements of the current bank; bank TabBar stays.
- `TextureTab({bool onlyStaged = false})` — paths list = staged replacement keys.
- `ScriptTab({bool onlyStaged = false})` — vanilla tree/flat list restricted to
  staged relPaths; staged-adds (no vanilla leaf) remain visible via the staged
  bottom panel.

Filtered views are live: un-staging the last entry of a domain empties that view
(each tab already renders a sensible empty state or gets a small hint).
Filter sets are derived by ChangesTab from the same providers the OverridesPanel
already watches.

New `lib/editor/ui/changes_tab.dart` hosts sidebar + content switch
(`home_page.dart` swaps `OverridesPanel` for `ChangesTab`; keep-alive wrapper
stays).

## Non-goals

- No changes to deploy/export or domain notifiers.
- No redesign of the "Alle" list itself.
