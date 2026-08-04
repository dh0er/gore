# Text & dialogs (localization)

All UI text and NPC dialog lines live in the encrypted AlkimiaLocalization
`.lcache`. `gore loc` decrypts it, hands you plain JSON for every language, and
re-encrypts your edits.

There is one cache, and it is not under `Content` — it sits with the story data
at `$GAME\G1R\Story\Cache\AlkimiaLocalization_00000000.lcache`. The `_00000000`
is generated, so read the real name out of that directory rather than typing it
from memory. Easier still: every `loc` subcommand finds the file itself, the
same way the rest of the toolkit finds the install — an explicit `--lcache`
first, then the configured game path, then Steam auto-detect.

## Export, edit, import

```powershell
gore loc export -o loc.json     # auto-detects the installed .lcache
```

`loc.json` is a flat map of localization id → language → value:

```json
{
  "some_text_id": { "german": "Neuer Text", "english": "New text" },
  "another_id":   { "german": "…" }
}
```

Edit it, then write it back:

```powershell
gore loc import --edits loc.json
```

Pass `--lcache` when you mean a particular file rather than the installed one —
a copy you keep outside the game, or a second install:

```powershell
gore loc export --lcache "$GAME\G1R\Story\Cache\AlkimiaLocalization_00000000.lcache" -o loc.json
```

`import` overwrites the cache **in place**. Keep your own copy first, pass `-o`
to write the result elsewhere, or use the [bundle](bundles.md) path, which
backs the original up to `*.gore-bak` on deploy.

### Flags that matter

| Flag | Command | Meaning |
|---|---|---|
| `--lcache <PATH>` | all | The `.lcache` to read/edit — optional everywhere; without it the install is auto-detected. May also name a game dir or a Steam library to search. |
| `-o, --out <PATH>` | `export`, `import` | Output file. On `import`, defaults to overwriting the cache it read. |
| `--edits <PATH>` | `import` | The `{id:{language:value}}` edit JSON. |
| `--keep-empty` | `export` | Keep ids with no text instead of dropping them. |
| `--add-missing` | `import` | Accept ids that are not in the original cache. |
| `-y, --yes` | `extract` | Skip the confirmation prompt. |

## Adding new ids

Unknown ids are **rejected** by default — a typo in an id would otherwise
silently produce a line no one ever sees. New dialogs and quests legitimately
need new ids, so that case is an explicit opt-in:

```powershell
gore loc import --edits new-dialog.json --add-missing
```

Bundle and Mod Studio projects treat a newly authored localization id as an
explicit add operation, so you do not pass the flag there. In Mod Studio's
Dialogs tab, the add button creates a new `info_`/`dia_`/`gvl_`/`svm_` line in
the currently selected game language.

## The shared catalog

Other GORE tools (the save editor, Mod Studio) resolve ids to readable names
through a shared localization catalog. Build and inspect it with:

```powershell
gore loc extract          # auto-detects the game, writes the shared gore/loc_catalog.json
gore loc extract --lcache "$GAME"    # …or point it at a game dir / Steam library / .lcache
gore loc status           # ids, languages, source of the currently shared catalog
```

`extract` prompts before writing; `-y` skips the prompt.

## Text is not a conversation topic

Localization supplies **captions and spoken lines**. It does not by itself
create a selectable conversation topic in the dialog UI. A new topic needs a
compiled AngelScript topic class plus its guarded runtime registration — see
[AngelScript dialog authoring](dialog-authoring.md).

## Related

- [Voice-over](voice.md) — the recordings that go with the lines.
- [Bundling & deploying](bundles.md) — shipping `loc_edits` as part of a mod.
- [Mod Studio](../../apps/mod-studio/README.md) — the same edits in a GUI, with
  dialog-line browsing.
