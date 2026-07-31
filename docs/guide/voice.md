# Voice-over archives

Localized dialog recordings are Ogg files inside language ZIP archives under
`$GAME\G1R\Story\VoiceOver` — for example `german_new.zip`. `gore voice`
indexes them, extracts single recordings, and produces edited archives without
ever modifying the input.

Spoken lines are **not** in the FMOD banks. Sounds and music are covered in
[Audio](audio.md).

## Index

```powershell
$VO = "$GAME\G1R\Story\VoiceOver\german_new.zip"

gore voice list --archive "$VO"                 # `index` is an alias
gore voice list --archive "$VO" --json          # machine-readable index
gore voice list --archive "$VO" --filter DIA_   # only paths containing DIA_
```

The listing is **bounded**. Real archives are large — `german_new.zip` holds
33,323 entries — so `list` prints at most `--max` entries (default 100) and
leaves out the directory records, which carry no audio.

It always says what it left out. The header names the archive total, how many
entries a `--filter` kept, and how many directory records it dropped; a
shortened listing ends with a `… [truncated: …]` line. The JSON document carries
`entry_count` (the whole archive), `directory_count` (directory records among
the matches), `matched_count`, `listed_count` (the length of `entries`), and two
booleans that answer two different questions: `truncated` says whether `--max`
stopped the listing, and `complete` says whether the array is the whole archive
— a filter or a dropped directory record narrows it without truncating it.

```powershell
gore voice list --archive "$VO" --filter DIA_ --max 500
gore voice list --archive "$VO" --directories   # include directory records
gore voice list --archive "$VO" --filter DIA_ --max 0 --json   # counts only
```

Do not answer a truncation notice by asking for everything at once. All 33,323
entries of `german_new.zip` are an ~11 MB JSON document, far past what an MCP
client accepts in one result, and the cut lands inside the array — narrow with
`--filter` instead, and raise `--max` only as far as you need. `--max 0` lists
nothing and reports only the counts, which is the cheap way to ask "how many
match?".

`--filter` is case-insensitive on purpose: real archives hold `LINE_ONE.OGG`
next to `line.ogg`, and a case-sensitive filter would report "nothing found"
when the truth is "wrong case". It folds case exactly the way `--basename`
does, so `--filter MÜLLER` finds `DIA_Müller_01.ogg` in a German archive.

## Selecting one entry

Real archives contain **duplicate basenames**, so there are two selectors:

- `--basename <NAME>` — case-insensitive, accepted only when it matches exactly
  one entry. Convenient, and it fails loudly when it is ambiguous.
- `--path <ARCHIVE_PATH>` — the complete, case-sensitive archive path. Always
  unambiguous.

```powershell
gore voice extract --archive "$VO" --basename DIA_X.ogg -o extracted
gore voice extract --archive "$VO" --path "NPC/Quest/DIA_X.ogg" -o extracted
```

`-o` is an extraction root; the archive path is preserved below it. Extract
never overwrites an existing file.

## Resolving a localization id

When you know a localization id and want to know whether a recording for it
exists — without extracting anything:

```powershell
gore voice match-line --archive "$VO" --loc-id info_some_line
gore voice match-line --archive "$VO" --loc-id info_some_line --json
```

`--loc-id` is a trimmed ASCII id **without** the `.ogg` suffix; the command
resolves the exact `${loc_id}.ogg` basename inside the archive. This is the
lookup the Studio Voice workflow uses to bind a take to a dialog line.

## Add and replace

Both commands read the input archive, build a **new** archive, and publish it
only after full validation:

```powershell
gore voice replace --archive "$VO" --path "NPC/Quest/DIA_X.ogg" `
                   --ogg new.ogg -o german_replaced.zip

gore voice add --archive "$VO" --path "GoreMods/MyMod/DIA_NEW.ogg" `
               --ogg new.ogg -o german_added.zip
```

- The input is never modified.
- `-o` must be a path that does **not** already exist.
- The Ogg stream (Vorbis or Opus) and the completed ZIP are validated before
  the output is published.
- Unsafe paths, symlinks, encrypted entries, and resource-limit violations are
  rejected.

These commands *create an archive*. They do not install it into the game — for
that, use a [bundle](bundles.md).

## Multi-file patches: the manifest

For a distributable patch touching several recordings, use the versioned
manifest format. A format-1 manifest carries an ordered, non-empty `edits`
array:

```json
{
  "format": 1,
  "edits": [
    {
      "op": "replace",
      "path": "NPC/Quest/DIA_X.ogg",
      "ogg": "files/DIA_X.ogg"
    },
    {
      "op": "add",
      "path": "GoreMods/MyMod/DIA_NEW.ogg",
      "ogg": "files/DIA_NEW.ogg"
    }
  ]
}
```

```powershell
gore voice apply-manifest --archive "$VO" --manifest voice-patch.json `
                          -o german_patched.zip
# `gore voice apply` is a shorter alias.
```

Manifest rules, all enforced:

- `path` values are **complete archive paths**. Replacements match them exactly
  and case-sensitively; basename selectors are intentionally unavailable in
  manifests.
- Each `ogg` value is a portable, `/`-separated path relative to the manifest
  file. Absolute paths, empty/`.`/`..` components, backslashes, symlinks,
  Windows reparse points, and any path escaping the manifest directory are
  rejected.
- Unknown format versions and unknown operations are rejected.
- Case-insensitive duplicate targets are rejected.
- Every Ogg is read and validated **before** anything is applied; then the whole
  ordered batch runs in one verified archive pass.
- Replacements keep their original slots; additions are appended in manifest
  order.
- Any error publishes no output at all.

## The intro movie brings its own audio

`G1R\Content\Movies\G1R_Intro.bk2` is a pre-rendered Bink video — 3840×2160,
9,047 frames at 60 fps, 150.8 s — and it carries **four embedded audio tracks**
of its own (ids 0–3, all 48 kHz). The intro you hear is baked into that file.

So the per-line recordings under `Cutscenes/Intro/` and
`Cutscenes/IntroCutscene/` are a dead end for anything audible. `gore voice
replace` does its job on them — the new Ogg lands in the archive, validated,
and the archive verifies — and the intro sounds exactly as it did before. A
second clue that they are not the playback path: in `german_new.zip` all 44
Oggs in those two folders are
**byte-identical**, one 8.07 s 22.05 kHz placeholder repeated (`list` shows it
as one repeated CRC32). Other language archives do hold real recordings there —
`english_newer.zip` has 44 files with 44 different payloads — but the movie
still plays its own tracks.

This was found the hard way: an intro line was replaced, confirmed present in
the rebuilt archive, deployed — and the intro played exactly as before.

The subtitles are the opposite story. The one subtitle file that ships,
`G1R_Intro_en.srt`, holds localization ids rather than text:

```
5
00:00:22,706 --> 00:00:23,190
AlkimiaLocalization:TEXT_WIP_DKPDOUC_20260303_012643
```

Every language's intro subtitles therefore come out of the loc catalog, and a
`loc_edits` change to one of those ids shows up on the very next playback. See
[Text & dialogs](text-and-dialogs.md).

For an audible proof early in a new game, replace a real in-engine conversation
line instead. `german_new/OldCamp/Diego/INFO_DIEGO_GAMESTART_11_00.ogg` is the
first NPC line of a new game:

```powershell
gore voice replace --archive "$VO" `
    --path "german_new/OldCamp/Diego/INFO_DIEGO_GAMESTART_11_00.ogg" `
    --ogg new.ogg -o german_replaced.zip
```

## Deployment reality check

`replace` targets an existing recording and is the established path. `add` is
archive-safe, but whether the game actually resolves a brand-new voice path at
runtime is still runtime-dependent — treat additions as experimental.

A correct replacement is not the same as an audible one. The archive edit is
verified; whether that recording is what the engine plays at the moment you are
listening to is a separate question, and the intro above is the case where the
answer is no.

## Flag summary

| Flag | Commands | Meaning |
|---|---|---|
| `--archive <ZIP>` | all | Input voice ZIP. Never modified. |
| `--json` | `list`, `match-line` | One JSON document instead of human-readable output. |
| `--filter <TEXT>` | `list` | Keep only entry paths containing this substring, case-insensitive. |
| `--max <N>` | `list` | Max entries to print (default 100). The result says how many matched. `--max 0` lists nothing and reports only the counts. |
| `--directories` | `list` | Also list directory entries, which carry no audio. |
| `--loc-id <ID>` | `match-line` | Trimmed ASCII localization id, without `.ogg`. |
| `--basename <NAME>` | `extract`, `replace` | Case-insensitive basename; only when unique. |
| `--path <ARCHIVE_PATH>` | `extract`, `add`, `replace` | Exact, case-sensitive archive path. |
| `--ogg <PATH>` | `add`, `replace` | Ogg/Vorbis or Ogg/Opus file. |
| `--manifest <PATH>` | `apply-manifest` | Versioned JSON manifest; Ogg paths relative to it. |
| `-o, --out <PATH>` | all writing commands | Extraction root, or a new ZIP that must not exist. |

## Related

- [Bundling & deploying](bundles.md) — how voice edits are packaged and
  transactionally deployed into the install.
- [Mod Studio](mod-studio.md) — managing voice takes in the no-code GUI.
