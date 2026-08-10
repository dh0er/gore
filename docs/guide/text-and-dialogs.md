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

`loc.json` is a flat map of localization id → language → value. Not every id
carries every key:

```json
{
  "itfo_apple": {
    "german": "Apfel",
    "english": "Apple"
  },
  "ch1_bringlist_entry_3": {
    "german": "Diego war sehr zufrieden als ich ihm Ian's Liste überreichte.",
    "german_new": "Diego hat die Liste bekommen.",
    "english": "Diego was very happy when I gave him Ian's list.",
    "english_newer": "Gave the list to Diego."
  }
}
```

Both records are real, trimmed here to their German and English keys, and the
difference between them is the thing on this page most likely to cost you an
afternoon. The item name has one German key. The journal entry has two, and the
game displays `german_new` — an edit to its `german` succeeds and changes
nothing on screen. Read [which language key to write](#which-language-key-to-write)
before you touch dialog or journal text.

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

## Which language key to write

One `gore loc export` of BuildID 24539464 finishes by reporting `43898 ids
across 19 languages`. The 19 counts keys in the cache header, not translations
you can choose between. Grouped by what they actually are — every id count here
comes from that same export, and counts only ids that carry text:

- **Twelve are ordinary languages with one key each.** `polish` 39,723,
  `russian` 39,563, `spanish` 39,149, `french` 39,140, `italian` 39,139,
  `japanese` 39,128, `schinese` 39,128 and `brazilian` 39,122 are complete
  enough to translate against. `korean` 16, `czech` 9, `romanian` 2 and
  `ukrainian` 2 are stubs.
- **Two are not languages.** `foreign` (1,050) is the orc language —
  `Yoch moyóch to.` — and those ids carry their German and English text as
  well, so it is an extra line rather than a translation of anything.
  `stagedirections` (1,518) is English direction notes written for the voice
  actors ("disappointed - the hero, seeking admission into the Swamp Camp, was
  in…"). Neither was tested on screen.
- **German has two keys and English has three**, and that is the one that bites.

| Key | Ids with text | What it holds |
|---|---|---|
| `german` | 9,939 | the original 1998 Gothic text — old orthography, "daß" |
| `german_new` | 33,737 | the remake's rewrite |
| `english` | 10,172 | the original, likewise |
| `english_new` | 5,562 | an intermediate pass |
| `english_newer` | 34,758 | the remake's rewrite |

2,147 ids carry both German keys, and on 1,885 of those the two readings differ
in wording rather than spelling — the same line rewritten, not reprinted.

### `german_new` wins where it exists

Where an id has both German keys, the game displays `german_new`. Where it has
only `german`, the game displays `german`.

That was observed on the shipped game, in both directions, on BuildID 24539464.
A distinguishable marker was written into each of one id's two German keys, and
the journal showed the `german_new` marker; separately, ids that carry only
`german` — menu buttons, difficulty names, NPC display names, and the dialog
subtitle `info_diego_gamestart_11_00` — all changed on screen as expected. One
build, one install, one person looking. It has not been re-checked since, and
nothing in the test suite checks it.

One cheap corroboration you can check for yourself: `G1R\Story\VoiceOver` ships
`german_new.zip` and `english_newer.zip`, and no `german.zip`. The recordings
exist for the newer generation only.

Which generation an id uses tracks what kind of text it is. Of the 33,737 ids
with `german_new`, most are voiced or conversational: `gvl_` 21,787, `text_`
9,098, `dia_` 833, `info_` 804. Ids carrying only `german` lean the other way —
`svm_` 1,833, `dia_` 830, `text_` 759, `ui_` 650 — and UI and item names are
`german`-only outright: not one of the 662 `ui_` ids, and not one id under the
item prefixes `itfo`/`itmi`/`itmw`/`itwr`/`itar`, carries `german_new` at all.
`dia_` appears in both lists, though, so "it is dialog" does not settle it.
Look the id up.

### Getting it wrong: written, and not the line you see

Writing `german` on an id that also has `german_new` is accepted everywhere and
changes nothing you can see. The cache really is rewritten; the game still shows
the `german_new` line, because at the file level nothing was wrong — you edited
a key that exists, and the game reads the other one.

Whether anything says so depends on the route:

- **`gore loc import` does not.** This is the whole of the feedback:

  ```
  Applied 1 edit(s) -> …\AlkimiaLocalization_00000000.lcache
  ```

- **`gore mod deploy` and `gore mgr apply` do.** Both check the id's other
  language slots before writing and report the edit as one the game will not
  display, naming the id and the generation that hides it. The deployment
  itself succeeded — the point of the wording is that this is an edit to
  redirect at `german_new`, not a deployment to undo.

Writing `german` on an id that has *only* `german_new` behaves differently
depending on how the edit reaches the game:

- **`gore loc import` fails loudly**, naming the id and the key. Measured
  against a copy of BuildID 24539464's cache:

  ```
  error: editing info_bau_2_daslager_15_00/german: language 'german' not found for key 'info_bau_2_daslager_15_00'
  ```

- **A bundle's `loc_edits` skips it, and says so.** Per
  `crates/gore-mod/src/lib.rs`, deploy checks that the key is one of the 19 the
  install's cache header declares. A key this install does not declare at all,
  and an id that has no slot for a declared key, are both best-effort skips
  rather than errors — deliberately, so that a mod built against a different
  game version still deploys. Deploy succeeds, the `.lcache` is rewritten, the
  `*.gore-bak` backup is taken, and the line in game is unchanged — but the
  command lists every edit it skipped, with its id and language, so a deployment
  that changed less than it was asked to no longer looks like one that did not. `gore mod build` cannot catch it either: it
  copies `loc_edits` into the bundle unvalidated, because whether a key fits an
  id is a property of the install, not of the spec.

Nothing here ever observes the screen. "Applied 1 edit(s)" means the cache now
holds your bytes under that key, and a deploy that succeeds means the file is in
place — neither is evidence that a line changed. Looking at it in game is the
only check.

### What to write

Look the id up in your export first. The export is the authority on which keys
that id carries:

| The export shows | Write |
|---|---|
| only `german` | `german` |
| only `german_new` | `german_new` |
| both | both, unless you want the two generations to read differently |

Writing both is safe in a bundle even where only one fits, since the other is
dropped. It is not safe with `gore loc import`, which errors on the key the id
lacks.

English has the same shape, and `english_newer` is both its largest key and the
one with a voice archive — but no English edit was ever checked on screen, so
which key the game reads there is untested. Verify one line in game before
building a translation on the assumption.

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

A new id gets exactly the keys you give it and no others, so
[the key choice](#which-language-key-to-write) applies here too — with nothing
pre-existing to shadow it either way. Which German key a newly added id is read
from was not tested; if you add one, check it in game before writing many.

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
