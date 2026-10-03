# Save games

`gore save` exposes Save Editor operations through the native Rust core. It does not require Flutter, an Editor DLL, an Oodle DLL or a running game. Every leaf accepts `--json` for a stable `{ok,data}` / `{ok,error}` result. `gore save --help` lists the command families; use the leaf's `--help` for arguments.

## Inspect and select

```powershell
gore save list --root "C:\saves" --profile 0 --json
gore save inspect "C:\saves\G1R-001.sav" --private --json
gore save characters list "C:\saves\G1R-001.sav" --category human --json
gore save statistics "C:\saves\G1R-001.sav" --json
```

Profile IDs are internal, zero based IDs. `--other` selects detached saves. `--actor hero` selects the controlled player. NPCs can be selected by their exact saved GlobalId or an unambiguous UniqueName. Queries with pagination return a total; `--all` follows every page from `--offset`.

For quests, tutorials and glossary, `show` requires `--id`, `--entry` or `--document` and returns the matching entry independently of list pagination. Save deletion accepts a file path or `delete --root <folder> --slot G1R-001 --profile 0`; both use the same guarded recovery transaction.

## Edit or stage

```powershell
gore save attributes set G1R-001.sav --attribute Strength --current 40 --dry-run --json
gore save attributes set G1R-001.sav --attribute Strength --current 40 --draft changes.json
gore save inventory add G1R-001.sav --item ItMi_Orenugget --count 100 --draft changes.json
gore save draft show changes.json
gore save draft validate changes.json --json
gore save draft apply changes.json --json
```

Immediate domain writes and draft application use the same conflict planner as the Editor. They simulate every write before publishing, back up the pristine input once, preserve companion and placement notes, and report committed and remaining original operation indices after a partial failure. The draft retains the remaining operations and advances its input hash. A changed source blocks replay. Adds retain their individual identities; setting a declarative target again replaces its pending intent. `--dry-run` validates without changing saves, drafts, settings, backups or caches.

In MCP, pass `dry_run: true` to preview Save Editor commands without write consent, including on servers started with `--no-consent-prompts`. Previews also skip output writes, icon-cache preparation and system viewers. Live changes still require write consent.

## Domain commands

| Family | Operations |
|---|---|
| `profiles`, `profile` | List profiles, assign/import saves, detach existing or missing references |
| `rename`, `import`, `delete` | Save management with the core's profile and recovery protections |
| `attributes`, `skills` | Hero and NPC attributes, complete learned skill catalog and tiers |
| `inventory` | Items, count, add/remove/reset and whole-save slot repair; select container and slot where necessary |
| `position`, `locations` | Coordinates and facing, catalog locations, NPC spawn reset, pin, undo and resume routine |
| `time`, `difficulty` | World clock and profile difficulty, including individual custom settings |
| `quests`, `tutorials`, `story` | State changes and sparse story integers with expected-value checks |
| `glossary`, `knowledge`, `events` | Document and segment states, character knowledge and indexed memory events |
| `factions`, `locks` | Open crimes and forgiveness; catalog chest/door lock state |
| `traders` | Merchants, both saved stock maps, ore and activity/restock timing |
| `data` | Scalar and structured raw values, sets, array removal/duplication, sources and node filters |
| `backups`, `recovery` | List/rename/restore/delete backups and deleted-save recovery; `recovery repair` explicitly enables discovery repair |
| `library`, `settings` | External/hidden files and shared Editor/UI preferences, preserving unknown JSON fields |
| `catalog`, `items`, `localization`, `assets`, `screenshot` | Bundled metadata and item statistics, game texts, original images, icon caches and screenshot export |
| `overview`, `statistics`, `report` | Save-backed metrics and an offline HTML report with optional images |
| `codec`, `validate`, `about`, `licenses`, `updates` | Codec/roundtrip, product information and the Save Editor update channel |

`inventory remove --item <id-or-path>` searches every container and requires exactly one matching stack. If an item has multiple stacks, list the chosen actor's inventory with `--all` and supply `--container` and `--slot`. Previews and drafts use the same selection.

NPC pinning records the original pose/routine and refuses stale undo. Merchant forecasts expose calendar and elapsed boundaries separately; the game performs maintenance lazily. Detached saves use their own difficulty or an explicit `--resources-level`. Unknown data stays unknown instead of becoming a zero count.

## Raw access

```powershell
gore save core capabilities --json
gore save core exec --request-file request.json --json
```

The raw request format is the Editor protocol: `{"command":"…","payload":{…}}`. The capability result lists every supported endpoint and write operation. The raw bridge preserves all payload options. For typed writes, `--path-file` contains a JSON array of exact path segments and `--value-json` is a JSON scalar or object. Raw mutating requests are refused with `--dry-run`; use the corresponding domain command or `draft validate` for simulation.

## Shared settings and images

The CLI reads the same `gore/gore-save/settings.json` and `ui_settings.json` as the Editor. Legacy settings are read without modifying old files and migrated on an explicit settings write. Use `settings set --scope ui --key themeMode --value dark`, or any of the Editor's font, scale, locale, window, ID and update preference keys. Reports consume these preferences.

`--lang` selects one of the Editor's 16 interface languages; `--game-lang` independently selects one of the 10 game-text languages. Automatic selection uses `appLocale` and `gameTextLocale` from shared preferences. Changing `appLocale` selects its matching game text, like the Editor; a later `gameTextLocale` change overrides only the game text. An explicit `--lang` uses its matching game text unless `--game-lang` is also supplied. Traditional Chinese uses the simplified Chinese game catalog; Czech, Ukrainian, Hungarian, Romanian and Turkish default to English game text.

`assets prepare --game <installation>` returns an icon manifest used by `assets list/export/open/release --manifest <file>`. `assets … --kind portraits --game <installation>` reads loose glossary artwork. `report <SAVE> --out report.html --with-assets --game <installation> --open` produces a local report. Missing optional image/text sources remain visible as unavailable.

Each `assets prepare` retains its cache generation across CLI and MCP invocations until a matching `assets release`. Release the manifest once per preparation after its consumers have finished; this allows later cache cleanup or repair. These leases are separate from the Editor's live leases. Export previews require an existing output parent directory and never create it.

On Windows, `updates check/install --kind installed --target <directory>` requires `goresave.exe`, its Save Editor product metadata and the installation's `unins000.exe`. The feed is compared with that executable's version; JSON reports `installedEditorVersion`, `currentEditorVersion` and the CLI's `bundledEditorVersion` separately. Portable copies use the bundled version and open the release download page for manual replacement.

Backup deletion is permanent and requires `--yes` in scripts. Save deletion retains the native recovery transaction. Read-only listing never repairs metadata; run `recovery repair` explicitly when needed. MCP exposes the complete family as `gore_save` and retains its existing write-permission gate.
