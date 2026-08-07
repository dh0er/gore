# Audio

The game's sounds and music live in encrypted FMOD `.bank` files at
`$GAME\G1R\Content\FMOD\Desktop\*.bank`. `gore audio` reads and replaces
samples in pure Rust — no FMOD installation or third-party tool is needed.

Every other subcommand wants a `--bank` path. `gore audio banks` is where you
get one.

## The banks an install has

```powershell
gore audio banks          # every .bank file, one row each
gore audio banks --json   # the same answer as one JSON document
```

It takes no path. It resolves the configured install — the same `--game`
fallback every other command uses — and describes
`G1R\Content\FMOD\Desktop`. That is the same directory a bundle resolves a bare
bank name against, so a bank listed here and a bank a bundle names cannot turn
out to be two different files.

Run against BuildID 24539464 it prints ten rows:

```
FMOD banks: 10 in D:\…\Gothic 1 Remake\G1R\Content\FMOD\Desktop (4 carry samples, 7443 samples in total)
SAMPLES  CODEC     BANK (pass this whole path as --bank)
     49  Vorbis    D:\…\Desktop\CINEMATICS.bank
      —  —         D:\…\Desktop\Master.bank  [no sample data: nothing here to list, extract or replace]
      —  —         D:\…\Desktop\Master.strings.bank  [no sample data: nothing here to list, extract or replace]
    174  Vorbis    D:\…\Desktop\Music.bank
      —  —         D:\…\Desktop\Music_NotDemo.bank  [no sample data: nothing here to list, extract or replace]
      —  —         D:\…\Desktop\Music_NyrasPrologue.bank  [no sample data: nothing here to list, extract or replace]
   7218  Vorbis    D:\…\Desktop\SFX.bank
      —  —         D:\…\Desktop\SFX_NotDemo.bank  [no sample data: nothing here to list, extract or replace]
      —  —         D:\…\Desktop\SFX_NyrasPrologue.bank  [no sample data: nothing here to list, extract or replace]
      2  Vorbis    D:\…\Desktop\VO.bank
```

The third column is the whole path on purpose: it is the string the next call's
`--bank` wants, and assembling it by hand from a directory printed once at the
top is the step this command exists to remove. (The paths are shortened here to
fit the page; the command prints each one in full.)

Six of the ten rows carry no samples and are printed anyway. A listing that
showed four files while claiming to describe the directory would send you back
to searching the filesystem, which is where you started; what those six are is
under [Banks with no samples](#banks-with-no-samples).

The listing is never truncated — the directory holds ten files, so a bound
could only hide something — and it is cheap, because it decrypts each bank's
60-byte FSB5 header rather than the bank. The FSB5 cipher is position-indexed,
so a block's header decrypts on its own without the 247 MB behind it. Measured
once on one machine from an unoptimized build: 0.21 s to describe all ten banks
(about 520 MB of file), against 2.49 s for a single `audio list --max 0` of
`SFX.bank`, which decrypts it in full.

A bank that has been replaced into carries a second FSB5 sub-bank, and the row
says so. The install above was pristine, so this row is the shape the code emits
— pinned by a test that injects into a fixture bank and reads the listing back —
rather than something that run produced:

```
   7218  Vorbis    D:\…\Desktop\SFX.bank  [injected — `gore audio restore` puts the shipped bank back]
```

The marker is read out of the bank's own wrapper rather than from a record this
toolkit keeps, so it stays true across a reinstall of the tools. It does not say
*which* samples were replaced — `audio list` marks those individually.

`--json` mirrors `list --json`. The path is under `bank`, the same key `list`
uses, so it can be handed straight back as the next call's `--bank`; each entry
also carries `name`, `carries_samples`, `sample_count`, `codec`, `sub_banks` and
`injected`, under a document-level `directory`, `bank_count`,
`with_samples_count` and `sample_count`. A file that cannot be read at all is
still a row, carrying an `error` instead of a count: one damaged bank must not
cost you the other nine. Passing the wrong `--key` puts every sample-carrying
bank in exactly that state, and the error names the key as the thing to suspect.

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

### Banks with no samples

Not every bank carries samples. `Master.bank` holds only the mixer and its
buses, `Master.strings.bank` only the string table, and `Music_NotDemo.bank`,
`Music_NyrasPrologue.bank`, `SFX_NotDemo.bank` and `SFX_NyrasPrologue.bank` are
506-byte placeholders. For these, `list` says the bank carries no sample data
rather than calling it damaged — they are intact, there is simply nothing in
them to extract or replace. It still says it as a failure: the command writes
`error: decoding bank: bank carries no sample data …` to stderr and exits 1,
and the `gore_audio` MCP tool flags the result as an error. A script that walks
all ten banks has to expect that — or ask
[`gore audio banks`](#the-banks-an-install-has) instead, which is the one
command that describes these six as rows rather than as failures. The samples
themselves are in `SFX.bank`, `Music.bank`, `VO.bank` and `CINEMATICS.bank`.

## Extract

```powershell
gore audio extract --bank "$GAME\...\SFX.bank" -o wavs                      # all samples
gore audio extract --bank "$GAME\...\SFX.bank" -o wavs --sample Foo         # just one
gore audio extract --bank "$GAME\...\SFX.bank" -o wavs --filter MenuButton  # a whole set
```

`--sample` takes a single sample name, or `all` (the default). `--filter` takes
the same case-insensitive substring `list` does, which is how you pull a whole
variant set out in one call rather than one `--sample` at a time.

Extracting into a directory that already holds WAVs is fine — auditioning
candidates is what this command is for. What it will not do is replace a file
already there: the names come from the bank, so a collision means the earlier
extract's output, or something you edited. It names the file and stops.

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

One check in game bears that out and widens it. On BuildID 24340829 the four
`SFX_UI_Action_MenuButton_Click` samples were replaced with distinguishable tones
and a person listened: the tones played on the menu buttons, and also when
backing out of a submenu with Escape. The name is narrower than the behaviour, so
expect a replacement here throughout menu navigation rather than on clicks alone.
In the same run `SFX_UI_Action_Button_Hover_01` landed where its name suggests —
its tone played on hover, and nothing else the listener did played it.

Music has the same gap between name and binding, and one hole that run did not
close. The main menu's title music was replaced successfully on that build, but
`title` and `title_MASTER` in `Music.bank` were replaced in the same pass, so
which of the two the title event draws on is still unknown. Replacing one of them
alone has not been tried.

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

Nothing in the listing marks a group as one. The four
`SFX_UI_Action_MenuButton_Click` takes print at `#1912`, `#1942`, `#3507` and
`#6477` — four rows thousands apart, each looking as unrelated to the others as
to anything else on the page. The shared prefix is all that ties them together,
which is why filtering on the prefix is how you find the rest of a set. Filter on
the exact prefix you mean: `--filter` is a substring test, so the command above
does not list the menu takes at all — `SFX_UI_Action_Button_Click` is not a
substring of `SFX_UI_Action_MenuButton_Click_01`.

Which member a trigger picks is decided by the event's playlist inside the bank,
which `gore audio` does not read. For that one set it behaved randomly: on
BuildID 24340829, with a different tone in each of the four, a listener clicking
through the main menu reported them arriving in no pattern. Replacing only `_01`,
which is the obvious move, would have produced the intended sound on roughly one
click in four — easy to mistake for a tool that does not work.

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
pristine bank and from an injected one and comparing the results.

It has since also been heard. On BuildID 24340829, five replaced `SFX.bank`
samples and the main menu's title music came out of the running game and were
identified by a person listening — one build, one install, one listener, one
sitting. That single session is what the menu findings above rest on, and it is
worth reading for exactly what it is: it shows the path works end to end on that
build, not that any particular replacement of yours will be audible, and it is
not a check the toolkit can run for you. Nothing here ever observes the screen or
the speakers, and no test in the suite checks either.

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
pass it. A wrong one is visible rather than silent: every field `banks` reports
comes out of the encrypted header, so it checks the decrypted `FSB5` magic
before printing a number and names the key when that check fails.

## Flag summary

| Flag | Commands | Meaning |
|---|---|---|
| `--bank <PATH>` | all but `banks` | The `.bank` file to read or modify. `banks` is where you get the path. |
| `--game <PATH>` | `banks` | Game install root (the folder containing `G1R/`). Defaults to the configured game path, then Steam auto-detect. |
| `--json` | `list`, `banks` | One JSON document instead of the human-readable table. |
| `--filter <TEXT>` | `list`, `extract` | Keep only sample names containing this substring, case-insensitive. |
| `--max <N>` | `list` | Max samples to print (default 100). The result says how many matched. `--max 0` lists nothing and reports only the counts. |
| `-o, --out <PATH>` | `extract`, `replace`, `export-patch`, `apply-patch` | Output dir (`extract`), output bank (`replace`, `apply-patch`), or output zip (`export-patch`). |
| `--sample <NAME>` | `extract` | One sample name, or `all` (default). |
| `--map <PATH>` | `replace`, `export-patch` | `{ "SampleName": "new.wav" }` JSON; WAV paths relative to it. |
| `--patch <PATH>` | `apply-patch` | Patch zip produced by `export-patch`. |
| `--key <KEY>` | `banks`, `list`, `extract`, `replace`, `apply-patch` | Override the bank encryption key. |

## Related

- [Voice-over](voice.md) — spoken dialog lines are **not** in the FMOD banks;
  they are Ogg files in language ZIP archives.
- [Bundling & deploying](bundles.md) — shipping audio replacements as part of a
  mod.
