# Mod-Studio UI Overhaul — Design

Date: 2026-07-02
Status: approved (user confirmed both open decisions: scripts tab rebuilt like textures; flat SFX categories)

Five UI changes to `apps/mod-studio`.

## 1. Scripts tab: tree of all AngelScripts (textures-style)

Today the Scripts tab shows only staged mods in a fixed 360px list; browsing vanilla
modules happens in a 200-item-capped flat picker dialog (`_ModulePicker`). The path data
needed for grouping already exists: `ScriptModuleInfo.file` is a slash-separated
game-relative path (e.g. `Gameplay/MyMod.as`, may be empty → fallback `<name>.as`).

New layout, mirroring the Textures tab:

- Left pane (Expanded flex 2): search field + folder tree of ALL vanilla modules from
  `scriptModulesProvider`, built from `file` paths. Empty query → tree; active query →
  flat hit list (matches on module name and path). Staged modules get a marker
  (filled dot / check icon) in the tree.
- Right pane (Expanded flex 3): detail for the selected module. For an unstaged vanilla
  module: name, path, and an "Edit" action that stages it (emits the pristine source via
  `scriptEmitModule`, same as today's `_editExisting`). For a staged module: the existing
  `_ModDetail` (Choose .as / Compile / status rows).
- Bottom: staged-mods panel (`ExpansionTile`, like the Audio tab's staged panel) listing
  all staged script mods with remove buttons; also the home of the "Add new" button for
  brand-new `.as` files (op add, not present in the vanilla tree).
- The `_ModulePicker` dialog and the fixed 360px staged list are removed.

The hand-rolled prefix tree in `texture_tab.dart` (`_RawNode`/`_DisplayNode`, single-child
folder compression, `_expanded` set, lazy flattened `ListView.builder`) is extracted into
a shared reusable widget (new file, e.g. `lib/app/ui/path_tree.dart`) parameterized over
leaf payload, leaf icon, staged-marker predicate, and selection callback. Textures tab is
refactored to use it; Scripts tab uses the same widget.

## 2. Items list: alphabetical by localized name

`catalog_browser.dart` currently shows items in raw class-id order (provider sort +
`groupCatalogItems` per-category sort). Change: the *displayed* list (both the
per-category list and the search-results flat list) is sorted by the localized display
name — `displayNameForItem(...)` lowercased, locale-independent `compareTo`. Sorting
happens on a copy at display time in `catalog_browser.dart`; the id-based provider and
grouping sorts stay (stable internal order). Category sidebar order is unchanged.

## 3. Dialogs tab: two-column sidebar like Items

Replace the single 560px grouped expand/collapse list with the Items layout:

- 230px speaker sidebar (`SidebarTile`): one tile per (speaker, isBark) group —
  conversations first (forum icon), then barks (campaign icon), alphabetical within each
  block; label = speaker name, count via the same `categoryWithCount` pattern.
- Middle column (remainder of the 560px browser): lines of the selected group, dense
  `ListTile`s as today (edited-dot, id title, current-language preview subtitle).
- Right pane: unchanged (`LangFieldsEditor` detail).
- Search: identical mechanics to Items — active query hides the sidebar and shows a flat
  cross-group hit list (id substring or any language text, as today).

## 4. Audio tab: bank TabBar + categorized SFX split view

- Bank selection: the `ChoiceChip` wrap becomes a `TabBar` with 4 tabs (SFX, Music,
  Cinematics, VO) driven by `kModdableBanks`.
- SFX tab content gets the Items split view: 230px category sidebar + sample list +
  existing detail pane. Other banks (175/49/2 samples) keep a flat sample list without a
  category sidebar.
- Categorization (validated against the real bank, 7218 samples): category = second `_`
  token of the sample name, case-folded, with merge map — CREA→Creatures (2868),
  OBJ/Objects→Objects (1277), MAGIC (825)→Magic, MOVE (789)→Movement, WORLD (472)→World,
  ACTION/ACTIONS (371)→Action, COMBAT (285)→Combat, PHYSICS (147)→Physics, ITEMS (53)→Items,
  UI (47)→UI, FOLEY (25)→Foley, UNDERWATER (23)→Underwater, VISION (21)→Vision,
  DIALOG (10)→Dialog, everything else (5)→Other. Flat categories (no creature
  sub-split). Category labels are localized via new l10n keys.
- Search within the SFX tab behaves like Items: active query hides the category sidebar
  and searches the whole bank. Staged-replacements bottom panel and preview/replace flow
  unchanged.

## 5. Main-tab localization + "Scripte" rename

- New l10n keys in all 12 `.arb` files: `tabDialogs`, `tabAudio`, `tabTextures`,
  `tabScripts` (DE: Dialoge / Audio / Texturen / Scripte; EN: Dialogs / Audio / Textures /
  Scripts; other languages translated accordingly). `home_page.dart` tab labels switch
  from hardcoded strings to these keys.
- The "AngelScript" tab is renamed: EN "Scripts", DE "Scripte".
- Section headers in `overrides_panel.dart` are localized: 'Textures'→`tabTextures`,
  'Audio'→`tabAudio`, 'AngelScript'→`tabScripts`, plus new keys for 'Item values' and
  'Localized text'.
- New keys for the SFX category labels (15) and any new dialog-sidebar strings.
- `flutter gen-l10n` regenerated output is committed (generated files are tracked).
- File-picker `XTypeGroup(label: 'AngelScript')` stays — it names the file type, not the tab.

## Non-goals

- No changes to deploy/export, FFI, or native crates.
- No two-level SFX categorization (rejected in review).
- No re-sorting of the item category sidebar.

## Testing

- `flutter analyze` (dart MCP hangs on this machine — CLI only) + `flutter test` in
  `apps/mod-studio`.
- New unit tests: SFX categorization mapping; localized item sort; dialog speaker
  grouping for the sidebar.
