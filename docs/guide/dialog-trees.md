# Reading dialog trees

`gore dialog` shows what an NPC actually says. It reads the game's own script
cache and reconstructs the whole conversation: every option in the menu, what
unlocks or hides it, the lines both sides speak, the effects a choice applies,
and which sub-menu it opens.

```powershell
gore dialog list viper                    # which conversations exist
gore dialog tree om_stt_viper_302         # the whole tree
gore dialog tree brannok --lang german    # in German
gore dialog show ChoiceStt302ViperMelt    # one option in full
gore dialog text viper -o viper.json      # its lines, ready to edit and re-import
gore dialog new-topic viper --caption-key K --mod-name MyMod -o MyMod   # a new option
gore dialog checkout viper -o work        # its AngelScript, to change what an option does
gore dialog export -o dialog\             # every conversation as JSON
```

Everything here works offline. It needs the game installed — that is where the
script cache lives — but only ever reads it: no command on this page launches
the game, writes into the install, or touches a save. The commands that produce
something write it where you point them.

## What a tree looks like

```
OM_STT_VIPER_302
Story.G1R.Conversation.Conversation_OM_STT_VIPER_302
4 topics, 4 root option(s)

- "You're the smelter, aren't you?"
  ? asked only once
    Hero: "You're the smelter, aren't you?"
    Viper: "Now just how did you figure that out."
    UnlockDocumentSegment(n"Hero", Document_Glossary_OM_STT_VIPER, DocumentSegment_Glossary_OM_STT_VIPER_Introduction)
- "Is ore smelting tough work? "
  ? only after ChoiceStt302ViperGreet is known
  ? asked only once
  ? only once Hero heard "Down at the bottom. You can talk to Viper, our smelter. "
  ? visible when: HasAssignedDailyRoutine(DailyRoutine_OM_STT_Viper_302_Collapsed)
    Hero: "Is ore smelting tough work? "
    Viper: "No. It's the most fun I've had in my entire life. Would you like to give it a try?"
```

Lines starting with `?` are conditions, and a `?` in front of a line inside the
body means that line only plays on some branch of the option. A sub-menu is
printed as `opens a sub-menu:` with its options nested underneath.

Options appear in the order the game declares (`PriorityRank`), which is what
puts "End." last.

## Naming a conversation

`tree` and `list` take the participant identifier, part of one, or a module
name:

```powershell
gore dialog tree om_stt_viper_302     # exact
gore dialog tree brannok              # substring, when it is unambiguous
gore dialog tree Conversation_HERO    # by module
```

An ambiguous name fails with the list of candidates rather than picking one.
`gore dialog list` with no argument prints all 283 conversations; six of them
declare only character settings and no topics at all, and say so.

## Text needs the localization catalog

The cache stores localization keys, not text. Extract the catalog once and both
the captions and the spoken lines resolve:

```powershell
gore loc extract
gore dialog tree om_stt_viper_302 --lang german
```

Without it every line prints as its bare key, and the tree says why. `--lang`
takes a family (`german`, `english`) and reads the newest populated column of
that family, or an exact column name (`german_new`, `polish`) to pin one.

Use `--ids` to print class names and localization keys next to the text — that
is what you need when the next step is editing a line with
[`gore loc`](text-and-dialogs.md) or writing a new topic against an existing
one.

## Changing what an NPC says

`gore dialog text` writes exactly the lines one conversation uses as an edits
document, which is the input format `gore loc import` takes:

```powershell
gore dialog text om_stt_viper_302 --lang german -o viper.json
# edit viper.json
gore loc import --edits viper.json
```

```json
{
  "STT_302_VIPER_GREET_INFO_15_01": { "german": "Ich habe gehört, du bist der Schmelzer." },
  "TEXT_WIP_CVNQHYB_20250211_150759": { "german_new": "Du bist einer von denen, die nur Ärger machen." }
}
```

The keys come out in tree order — a caption, then its lines, then the sub-menu
it opens — so the file reads like the conversation rather than like a hash
table.

**The column per line is chosen, not assumed.** The game reads the newest
populated column of a language, so a line that has `german_new` is served from
there and an edit written to `german` would change nothing at all. The two
entries above show both cases from the same conversation. Editing the file
without moving a line to a different column keeps that right.

Lines with no text in the requested language yet come out empty under the
first column of that family, and the command says how many. Filling one in
needs `gore loc import --add-missing` if the id is absent from the cache
entirely.

This changes wording only. Which options exist, what unlocks them, and what
they do live in the script cache, not in the localization cache.

## Adding an option

`gore dialog new-topic` writes the two files a new root-level option needs,
with the identities filled in from the tree:

```powershell
gore dialog new-topic om_stt_viper_302 --caption-key STT_302_VIPER_WORK_INFO_15_01 `
  --mod-name ViperWork -o ViperWork
```

```
class     UChoiceViperWork : UTopic_Hero__OM_STT_VIPER_302
sentinel  UChoiceStt302ViperExit
```

Those two lines are the part worth automating. The class has to derive from
that conversation's own topic base, and the bundle's registration needs a
*sentinel* — an existing vanilla topic that proves the live topic set belongs to
this NPC. The command reads both out of the cache and picks the conversation's
exit option as the sentinel, since every conversation has one and it is never
conditional. It also refuses a class name the cache already declares.

The generated `Dialog.as` is the shape with runtime evidence behind it: a
caption, an always-visible option, and a body that ends the conversation.
Spoken lines, conditions and effects are yours to write — `gore dialog show`
prints how the game writes its own. The generated `spec.json` is a complete
build spec, so the next two commands are the ordinary ones:

```powershell
gore as compile-module --op add --module ViperWork.Dialog `
  --rel-path ViperWork/Dialog.as --source ViperWork\Dialog.as `
  --work-dir .gore-as-work --allow-new-symbols -o ViperWork.Dialog.mini.Cache
gore mod build --spec ViperWork\spec.json -o build
```

The command prints them with the paths filled in. Compiling drives the game's
own compiler, so it needs the game installed and takes a couple of minutes;
what that path proves and what it does not is
[Dialog authoring](dialog-authoring.md).

This adds an option to the **root** menu. Changing what the existing options do
is a different operation — [editing the module](#changing-what-an-option-does)
— and a genuinely new option inside a sub-menu is reachable by neither.

## Changing what an option does

Wording lives in the localization cache; behaviour lives in the script cache.
To change behaviour you edit the conversation's own AngelScript and recompile
that one module.

```powershell
gore dialog checkout om_stt_viper_302 -o work    # take the module out
# edit work\Conversation_OM_STT_VIPER_302.as
gore dialog check work                           # will this survive the trip back?
gore dialog stage work --mod-name ViperEdit      # write the build spec
```

`checkout` writes the exact source the compiler itself would emit for that
module, an untouched copy under `pristine\`, and a manifest binding the edit to
that exact game build.

### What you may change, and what you may not

An edited module has to come back onto the shipping cache, and two mechanisms
decide what survives. The compiler-generated defaults — caption, priority,
rules, flags — are carried back from the shipped module byte-for-byte, which
only works while every surrounding identity is unchanged. And the recompiled
module is remapped *strictly* onto the base cache's keyspace, so it can only
name things that build already has.

| | |
|---|---|
| **You may change** | what a method does: spoken lines, effects, their order and their branches; the `IsVisible` test; which existing topics a `Subdialog` offers |
| **You may not** | add, remove, rename or reorder classes or methods; change a signature or a member variable; write a `default` statement; name a type or a text id the build does not already have |

Those are not house rules, they are the conditions under which the recompile
path accepts the result. Every one of them would otherwise surface as a refusal
*after* a two-minute compile, so `check` asks the same questions offline against
the same cache:

```
this edit cannot be carried back:
  - line 216: a `default` statement. Captions, priority, rules and flags are carried back from
    the shipped module unchanged, so they cannot be edited here
  - class UChoiceStt302ViperInvented is new. An edited module has to keep exactly the classes it
    shipped with; a new topic needs its own module
  - the literal "MY_BRAND_NEW_LINE" is not in this cache's string table. An edited module can
    only use text ids the game already ships; a brand-new one needs its own module
```

A clean edit names the methods it rewrote:

```
this edit can be carried back. Rewritten:
  - UChoiceBrannok119230::void Act_Implementation()
```

`check` compares against the cache the checkout was taken from and refuses if
the game has been updated since — a new build changes every identity the edit is
checked against.

### What this reaches

Re-pointing a line at different text the game already ships, reordering what an
option says, adding or removing an effect, changing when an option is visible,
and adding an **existing** topic to a sub-menu — a `Subdialog` call lists its
options as arguments with empty slots to spare, so extending one is an ordinary
body edit.

What it does not reach: a genuinely new option inside a sub-menu. That needs a
new class, and a new class can neither go into an edited module (identity drift)
nor be named from one (strict remap). New options are root-level, through
[`new-topic`](#adding-an-option).

The nearest thing that does work is putting an existing topic into a menu it is
not offered in and rewriting what it says — new content in a sub-menu, in a
class the game already has. It is the same topic everywhere it appears, so it
changes the other menu that offers it too; `gore dialog tree` shows where that
is before you decide.

### Then compile it

`stage` writes `spec.json` and prints the two commands, with `--op edit` and without
`--allow-new-symbols`:

```powershell
gore as compile-module --op edit --module Story.G1R.Conversation.Conversation_BC_BAN_BRANNOK_863 `
  --rel-path Story/G1R/Conversation/Conversation_BC_BAN_BRANNOK_863.as `
  --source work\Conversation_BC_BAN_BRANNOK_863.as --work-dir .gore-as-work `
  -o work\ViperEdit.mini.Cache
gore mod build --spec work\spec.json -o build
```

Passing `--allow-new-symbols` here is refused by design: strict remapping is
exactly what lets the shipped captions and rules come back unchanged.

`check` reads the shipped module, not your syntax. It cannot tell you that your
AngelScript parses — only the compile step can, and that is the step that takes
the minutes.

## Conditions, in the game's own vocabulary

Every option carries two independent kinds of condition, and the tree shows
both:

| Shown as | Comes from | Means |
|---|---|---|
| `asked only once` | `Rules.HideIfKnows(self)` | disappears after being picked |
| `only after X is known` | `Rules.AllowIfCharacterHasKnowledgeOf` | X has to have been picked first — the parent/child edge of the knowledge graph |
| `hidden once X is known` | `Rules.HideIfKnows(X)` | mutually exclusive with X |
| `only once <who> heard "…"` | `Rules.RequireCharacterHasListenedTo` | a specific line has to have played |
| `only while <who> has not heard "…"` | `Rules.RequireCharacterHasNotListenedTo` | the inverse |
| `only while <who> is within N of <spot>` | `Rules.RequireCharacterCloseToWaypoint` | a place condition; check the spot with [`gore location`](catalogs-and-models.md) |
| `visible when: …` | the class's `IsVisible` script | everything the override looks at, calls and state flags alike |

The rules are declarative and complete. The `visible when:` list is not a
formula: it names what the script inspects, in the order it inspects it, and
deliberately does not reconstruct the `and`/`or` between them. Read it as "these
are the things that decide it", not as a condition you can evaluate.

## What the tree does not tell you

It reports what the cache **declares**. Whether a given option appears in a
given playthrough depends on save state, and no offline tool can answer that.

An option's body is shown as far as it is modelled. Spoken lines, sub-menus,
returning to the previous menu and ending the conversation get their own shape;
every other call — `Remember`, `StartQuest`, `SucceedQuest`,
`AddItemToInventory`, `ExchangeDailyRoutineToClass` and some eighty more —
prints with its resolved name and arguments. Nothing is dropped silently, and
the footer under each tree says how much was read:

```
read 126 step(s), 87 of them typed
```

Ambient topics are lines an NPC plays without being asked, so they have no menu
caption; they show as `(ambient)`.

## JSON, for tooling

`--json` on `list`, `tree` and `show` emits the same content as one document,
and `export` writes one file per conversation:

```powershell
gore dialog tree om_stt_viper_302 --json
gore dialog export -o dialog\
```

The JSON carries everything the human view summarizes — every rule with its
arguments, every check, every step with its guard, plus a per-conversation
`coverage` block — so a tool downstream can tell what was read from what was
not.

## Related

- [Text & dialogs](text-and-dialogs.md) — changing what a line says
- [Dialog authoring](dialog-authoring.md) — adding a new option to a conversation
- [Scripts (AngelScript)](scripts.md) — the cache this reads
- [Finding things](find.md) — looking one class or id up
