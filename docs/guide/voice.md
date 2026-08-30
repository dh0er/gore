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

## Add and replace

### Step 1 — resolve the path, do not guess it

`replace` and a bundle's `voice` entry both take the **exact, case-sensitive**
stored path, and the archive holds near-identical names under different
speakers. `german_new/OldCamp/Diego/INFO_DIEGO_GAMESTART_11_00.ogg` and
`german_new/HERO/INFO_DIEGO_GAMESTART_15_01.ogg` are both real members and both
plausible guesses for "Diego's first line" — one of them is the hero's. A
guessed path either fails loudly or edits the wrong take.

If you have a localization id, `match-line` turns it into the real member
without extracting anything:

```powershell
gore voice match-line --archive "$VO" --loc-id info_some_line
gore voice match-line --archive "$VO" --loc-id info_some_line --json
```

`--loc-id` is a trimmed ASCII id **without** the `.ogg` suffix; the command
resolves the exact `${loc_id}.ogg` basename inside the archive. This is the
lookup the Studio Voice workflow uses to bind a take to a dialog line. Without
an id, narrow with [`list --filter`](#index) and read the path off the listing.

### Step 2 — encode the recording as Ogg

`--ogg` takes an Ogg file and nothing else. Your recording tool almost certainly
wrote a WAV, so there is a conversion step, and the toolkit does not do it for
you: it ships no encoder and adds none.

Install [ffmpeg](https://ffmpeg.org/download.html) — it is a single executable on
Windows, macOS and Linux — and convert:

```powershell
ffmpeg -i line.wav -c:a libvorbis -ar 48000 -ac 1 -q:a 5 line.ogg
```

Check the result before building an archive or bundle:

```powershell
gore voice validate --ogg line.ogg
gore voice validate --ogg line.ogg --json
```

This performs the same bounded structural and timing validation used by the
archive and bundle paths. Vorbis is decoded completely to PCM; Opus packet and
timing structure is checked without decoding its audio payload. It does not
claim that a recording sounds good or has been heard in the game.

Flag by flag: `-c:a libvorbis` writes Vorbis, `-ar 48000` resamples to 48 kHz,
`-ac 1` mixes down to mono, `-q:a 5` sets the encoder's quality (libvorbis
documents the range -1 to 10; higher is better and larger). Swap `line.wav` for
`.aiff`, `.flac`, `.mp3`, `.m4a` — ffmpeg reads all of them and the rest of the
line is unchanged. `oggenc` (from vorbis-tools) and sox can do the same job, but
their flags are not written down here because they have not been checked.

Nor is that line run by any test: the toolkit has no ffmpeg dependency and is
not going to grow one. What stands behind it is that every flag was read out of
ffmpeg's own option documentation, and that the Ogg fixtures the test suite
validates were themselves produced by the same `-c:a libvorbis -ac 1` encode at
48 kHz (`crates/gore-vo/testdata/README.md`).

**What is required is the container and the codec.** `gore voice` accepts an
Ogg carrying Vorbis or Opus and refuses everything else. Skipping the conversion
is the ordinary first mistake, and the refusal names the format you handed over
and gives the line back:

```text
error: replacing voice entry: the payload is a WAV file (RIFF/WAVE), not an Ogg
stream — voice archives hold Ogg/Vorbis, the codec of every recording the game
ships (mono, 48 kHz). Convert it first: 'ffmpeg -i line.wav -c:a libvorbis
-ar 48000 -ac 1 -q:a 5 line.ogg'
```

(Wrapped here; the tool prints it on one line.) AIFF, FLAC, MP3 and MP4/M4A are
recognized the same way. Anything else says only that it is not an Ogg — and
still hands over the command.

**48 kHz mono is what the archives themselves are.** Every one of the 134,297
Ogg entries across the five archives under `G1R\Story\VoiceOver` —
`german_new`, `english_newer`, `foreign`, `polish`, `russian` — is mono 48 kHz
Vorbis, each declaring a nominal bitrate of 80 kbit/s. That is a full scan of
every entry, not a sample. Nothing here has tested whether the engine also plays
44.1 kHz or stereo, and the validator accepts both; matching what ships is
simply the choice that needs no test.

### Vorbis or Opus?

Encode Vorbis. Opus is accepted in more places than it is qualified, and the two
layers disagree:

- `gore voice add`, `replace` and `apply-manifest`, and the `voice` entries of a
  `gore mod` bundle, run one shared validator that accepts **Vorbis or Opus**.
- Mod Studio does not. Its project build refuses a selected take whose codec is
  not Vorbis and raises a `selected_take_codec_unqualified` blocker rather than
  guessing (`crates/gore-authoring/src/revision3_voice_build.rs`).

So an Opus take can go into an archive through the CLI and then block the Studio
build of the same project. Both statements you may have read are true; they are
about different layers. This is read off the code, not off a listening test —
no Opus payload has been heard in game, and the one replacement that has been
([Deployment reality check](#deployment-reality-check)) was Vorbis, because it
was lifted out of a shipped archive.

### Step 3 — build the edited archive

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
- The Ogg stream and the completed ZIP are validated before the output is
  published. Vorbis and Opus both pass here — see
  [Vorbis or Opus?](#vorbis-or-opus) above before you pick Opus.
- Unsafe paths, symlinks, encrypted entries, and resource-limit violations are
  rejected.

These commands *create an archive*. They do not install it into the game — for
that, use a [bundle](bundles.md).

### Borrowing a take instead of encoding one

For merely checking that the path works, there is a way round that needs no
encoder: take a recording out of another language's archive and use it as the
payload. The other archives under `G1R\Story\VoiceOver` hold Ogg already, so an
extracted file goes straight back in.

```powershell
gore voice extract --archive "$GAME\G1R\Story\VoiceOver\russian.zip" `
    --basename INFO_DIEGO_GAMESTART_11_03.ogg -o payload
```

For a test this beats a synthesized tone. Another language's take is a different
voice saying different words, so there is nothing to argue about when you hear
it, while a beep in the middle of a conversation is easy to mistake for
something else going wrong. It is a way to prove the path before you commit to
recording anything, not a way to ship voice work — that still needs the encoder.

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
and the archive verifies — and the intro sounds exactly as it did before. This
was found the hard way: an intro line was replaced, confirmed present in the
rebuilt archive, deployed — and the intro played exactly as before.

This page used to offer a second clue: that `german_new.zip` held one placeholder
repeated across all 44 entries in those two folders. That is no longer what the
archive contains. On the install checked here, `german_new.zip` and
`english_newer.zip` each hold 44 entries there with 44 distinct payloads, all
mono 48 kHz Vorbis like the rest of the archive — real recordings, in both. Only
the listening test still stands, and it is the one that mattered: the movie plays
its own embedded tracks whatever those folders contain.

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
first NPC line of a new game, and one of the two lines a replacement has
actually been heard on — see
[Deployment reality check](#deployment-reality-check):

```powershell
gore voice replace --archive "$VO" `
    --path "german_new/OldCamp/Diego/INFO_DIEGO_GAMESTART_11_00.ogg" `
    --ogg new.ogg -o german_replaced.zip
```

## Deployment reality check

`replace` targets an existing recording and is the established path. `add` is
archive-safe, but whether the game actually resolves a brand-new voice path at
runtime is still runtime-dependent — treat additions as experimental. Nothing
below moves that: the run described here did not exercise `add` at all.

A correct replacement is not the same as an audible one. The archive edit is
verified; whether that recording is what the engine plays at the moment you are
listening to is a separate question, and the intro above is the case where the
answer is no.

A replacement has now also been heard. On BuildID 24539464, with the game's
voice language set to German, two entries of
`G1R\Story\VoiceOver\german_new.zip` were replaced through a bundle and the
bundle deployed:

- `german_new/OldCamp/Diego/INFO_DIEGO_GAMESTART_11_00.ogg` — an Orcish line,
  taken out of `foreign.zip`.
- `german_new/OldCamp/Diego/INFO_DIEGO_GAMESTART_11_03.ogg` — the Russian take
  of the same line, taken out of `russian.zip`.

In a new game, Diego walked up and growled in Orcish instead of saying "Ich bin
Diego", and his later line came out in Russian. The line between the two,
`INFO_DIEGO_GAMESTART_11_02`, was deliberately left untouched and played as
normal German.

That untouched middle line is the part worth writing down. It shows the edit is
per-entry: nothing in the archive shifted or re-indexed around the two members
that changed, and a line sitting between them still resolved to its own
recording. Replacing every line in the scene would have sounded just as
convincing and proved none of that. The archive grew from 915,670,575 to
915,717,157 bytes on deploy, and undeploy put it back to exactly 915,670,575.

Read that for what it is. One person listened to one scene, once, on one build
and one install, with the toolkit built from commit 90940340. Two lines were
checked, in German only — no other language, no other speaker, no second
sitting. It establishes that the path works end to end on that build, on the
two entries that were touched. It does not establish that a particular
replacement of yours will be audible, and it says nothing about `add`. Nothing
in this toolkit ever listens, and no test in the suite checks any of it.

## Flag summary

| Flag | Commands | Meaning |
|---|---|---|
| `--archive <ZIP>` | all but `validate` | Input voice ZIP. Never modified. |
| `--json` | `list`, `match-line`, `validate` | One JSON document instead of human-readable output. |
| `--filter <TEXT>` | `list` | Keep only entry paths containing this substring, case-insensitive. |
| `--max <N>` | `list` | Max entries to print (default 100). The result says how many matched. `--max 0` lists nothing and reports only the counts. |
| `--directories` | `list` | Also list directory entries, which carry no audio. |
| `--loc-id <ID>` | `match-line` | Trimmed ASCII localization id, without `.ogg`. |
| `--basename <NAME>` | `extract`, `replace` | Case-insensitive basename; only when unique. |
| `--path <ARCHIVE_PATH>` | `extract`, `add`, `replace` | Exact, case-sensitive archive path. |
| `--ogg <PATH>` | `add`, `replace`, `validate` | Ogg file. Vorbis or Opus passes here; [encode Vorbis](#vorbis-or-opus). A WAV is refused with the ffmpeg line that converts it. |
| `--manifest <PATH>` | `apply-manifest` | Versioned JSON manifest; Ogg paths relative to it. |
| `-o, --out <PATH>` | all writing commands | Extraction root, or a new ZIP that must not exist. |

## Related

- [Bundling & deploying](bundles.md) — how voice edits are packaged and
  transactionally deployed into the install.
- [Mod Studio](mod-studio.md) — managing voice takes in the no-code GUI.
