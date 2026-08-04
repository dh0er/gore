# Audio

The game's sounds and music live in encrypted FMOD `.bank` files at
`$GAME\G1R\Content\FMOD\Desktop\*.bank`. `gore audio` reads and replaces
samples in pure Rust — no FMOD installation or third-party tool is needed.

## Inspect a bank

```powershell
$SFX = "$GAME\G1R\Content\FMOD\Desktop\SFX.bank"

gore audio list --bank "$SFX"                  # the first 100 samples
gore audio list --bank "$SFX" --json           # machine-readable listing
gore audio list --bank "$SFX" --filter Orcdog  # only names containing Orcdog
```

Prints each sample with its name, sample rate, channel count and duration,
under a header naming the bank's codec. The sample name is the key you use in
every other command.

The listing is **bounded**. Real banks are large — `SFX.bank` holds 7,218
samples — so `list` prints at most `--max` samples (default 100).

It always says what it left out. The header names the bank total, its codec and
how many samples a `--filter` kept; a shortened listing ends with a
`… [truncated: …]` line. The JSON document carries `bank`, `codec`,
`sample_count` (the whole bank), `matched_count`, `listed_count` (the length of
`samples`), and two booleans that answer two different questions: `truncated`
says whether `--max` stopped the listing, and `complete` says whether the array
is the whole bank — a filter narrows it without truncating it.

```powershell
gore audio list --bank "$SFX" --filter Orcdog --max 500
gore audio list --bank "$SFX" --filter Orcdog --max 0 --json   # counts only
```

Do not answer a truncation notice by asking for everything at once. All 7,218
samples of `SFX.bank` are a 458,589-byte table, far past the 256 KiB the MCP
server passes on by default (`--max-output-kib` raises it), and the cut lands in
the middle of a line — so filtering what arrived answers "no such sample" for the
3,095 that never did.
Narrow with `--filter` instead, and raise `--max` only as far as you need.
`--max 0` lists nothing and reports only the counts, which is the cheap way to
ask "how many match?".

`--filter` is case-insensitive on purpose: sample names carry their own casing
(`SFX_CREA_Orcdog_Grunt_L1_05`), and a case-sensitive filter would report
"nothing found" when the truth is "wrong case". It folds case exactly the way
`gore voice list --filter` does.

Not every bank carries samples. `Master.bank` holds only the mixer and its
buses, `Master.strings.bank` only the string table, and `Music_NotDemo.bank`,
`Music_NyrasPrologue.bank`, `SFX_NotDemo.bank` and `SFX_NyrasPrologue.bank` are
506-byte placeholders. For these, `list` says the bank carries no sample data
rather than calling it damaged — they are intact, there is simply nothing in
them to extract or replace. It still says it as a failure: the command writes
`error: decoding bank: bank carries no sample data …` to stderr and exits 1,
and the `gore_audio` MCP tool flags the result as an error. A script that walks
all ten banks has to expect that; the samples themselves are in `SFX.bank`,
`Music.bank`, `VO.bank` and `CINEMATICS.bank`.

## Extract

```powershell
gore audio extract --bank "$GAME\...\SFX.bank" -o wavs               # all samples
gore audio extract --bank "$GAME\...\SFX.bank" -o wavs --sample Foo  # just one
```

`--sample` takes a single sample name, or `all` (the default).

Extraction decodes Vorbis, so a sample in another codec is skipped rather than
written. Skips are reported to stderr once per *reason*, with a count and the
first sample that hit it — a whole bank in the wrong codec is one root cause,
not 7,218 of them.

## Pick a sample the surface plays

A sample is not a sound the game triggers. The game plays FMOD *events* —
separate cooked assets under `/Game/FMOD/Events/…` — and an event draws on one
or more of the bank's samples. The two name lists are kept apart and do not
always agree: the game has an event `SFX_UI_Action_MenuButton_Hover` for which
`SFX.bank` holds no sample of that name, and the bank holds
`SFX_UI_Notify_ClickElement_01` for which there is no such event. A name that
reads like the sound you are after is a hint, not a binding; the binding is in
the cooked UI package, which names the event it plays among its imports.

Where several near-identical names exist, that decides whether you hear anything
at all. The main menu's buttons — and the pause menu's — play
`SFX_UI_Action_MenuButton_Click`. The `SFX_UI_Action_Button_Click` samples
sitting next to them in a `--filter Click` listing belong to inventory slots,
sliders, spin boxes and the settings rows. Replace one of those, click through
the main menu, and you hear the original — not because the replacement failed
but because that surface never plays it.

## Variant sets

Most sounds are one take of several. 7,191 of `SFX.bank`'s 7,218 sample names
end in a two-digit index, and 1,350 of the 2,135 groups those indices form hold
more than one member — 6,406 samples between them. `SFX_UI_Action_Button_Click`
has four takes; the largest groups have 26.

The game plays one member per trigger, so replacing a single member changes the
sound only when that member is the one picked, and the rest of the group still
plays unaltered. Count the group before you replace anything, and replace all of
it if the change has to be audible every time:

```powershell
gore audio list --bank "$SFX" --filter SFX_UI_Action_Button_Click
```

Which member a trigger picks is decided by the event's playlist inside the bank,
which `gore audio` does not read.

## Replace

Write a map of sample name → replacement WAV. Paths are resolved relative to
the map file:

```json
{
  "SampleName": "path/to/new.wav",
  "OtherSample": "other.wav"
}
```

```powershell
gore audio replace --map map.json --bank "$GAME\...\SFX.bank"
```

By default this overwrites the bank in place and backs the original up to
`*.gore-bak`. Pass `-o` to write a new bank instead and leave the game
untouched.

Replacement re-encodes your WAV as PCM16 into an appended sub-bank and repoints
the sample at it. Consequences worth knowing:

- The replacement may be **any length** — it does not have to match the
  original sample.
- The rest of the bank is not re-encoded, so the operation is fast and lossless
  for every sample you did not touch.
- The bank grows by roughly the size of the injected PCM data.

Undo an in-place replacement:

```powershell
gore audio restore --bank "$GAME\...\SFX.bank"
```

## What is proven, and by what

Check your own work by listing the bank again. A sample whose waveform now points
at the appended sub-bank is marked, and reports the injected rate, channels and
length rather than the original's:

```
#2964   44100Hz 1ch   0.35s  SFX_UI_Action_Button_Click_01  [replaced, Pcm16]
```

That marker is a real readback: it resolves the waveform to whichever sub-bank it
now names, so an absent marker after a successful `replace` means the repoint did
not land. `--json` carries the same thing as `"replaced": true`.

What it proves is that the bank names your audio where the original used to be —
nothing more. It does not prove the surface you are about to test plays that
sample (see [Pick a sample the surface plays](#pick-a-sample-the-surface-plays)),
and it does not prove anyone will hear it.

For the layer below that, the record is this: an injected bank loads in the
game's own FMOD runtime and the event plays the injected audio. That was measured
once, off-line, by rendering an event through FMOD's non-realtime writer from a
pristine bank and from an injected one and comparing the results — not by anyone
listening in game. Nothing in this toolkit ever observes the screen or the
speakers, and no test in the suite checks either.

## Share a patch without shipping game audio

A patch zip carries only the manifest and *your* replacement WAVs, never the
original game audio:

```powershell
gore audio export-patch --map map.json -o patch.zip
gore audio apply-patch  --patch patch.zip --bank "$GAME\...\SFX.bank"
```

`apply-patch` behaves like `replace`: in place with a `*.gore-bak` backup by
default, or to `-o`.

## Encryption key

Every subcommand that reads bank content accepts `--key` to override the bank
encryption key. It defaults to the Gothic 1 Remake key, so you normally never
pass it.

## Flag summary

| Flag | Commands | Meaning |
|---|---|---|
| `--bank <PATH>` | all | The `.bank` file to read or modify. |
| `--json` | `list` | One JSON document instead of the human-readable table. |
| `--filter <TEXT>` | `list` | Keep only sample names containing this substring, case-insensitive. |
| `--max <N>` | `list` | Max samples to print (default 100). The result says how many matched. `--max 0` lists nothing and reports only the counts. |
| `-o, --out <PATH>` | `extract`, `replace`, `export-patch`, `apply-patch` | Output dir (`extract`), output bank (`replace`, `apply-patch`), or output zip (`export-patch`). |
| `--sample <NAME>` | `extract` | One sample name, or `all` (default). |
| `--map <PATH>` | `replace`, `export-patch` | `{ "SampleName": "new.wav" }` JSON; WAV paths relative to it. |
| `--patch <PATH>` | `apply-patch` | Patch zip produced by `export-patch`. |
| `--key <KEY>` | `list`, `extract`, `replace`, `apply-patch` | Override the bank encryption key. |

## Related

- [Voice-over](voice.md) — spoken dialog lines are **not** in the FMOD banks;
  they are Ogg files in language ZIP archives.
- [Bundling & deploying](bundles.md) — shipping audio replacements as part of a
  mod.
