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

gore voice list --archive "$VO"          # `index` is an alias
gore voice list --archive "$VO" --json   # machine-readable index
```

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

## Deployment reality check

`replace` targets an existing recording and is the established path. `add` is
archive-safe, but whether the game actually resolves a brand-new voice path at
runtime is still runtime-dependent — treat additions as experimental.

## Flag summary

| Flag | Commands | Meaning |
|---|---|---|
| `--archive <ZIP>` | all | Input voice ZIP. Never modified. |
| `--json` | `list`, `match-line` | One JSON document instead of human-readable output. |
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
