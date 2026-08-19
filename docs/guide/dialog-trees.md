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
gore dialog export -o dialog\             # every conversation as JSON
```

Everything here is offline and read-only. It needs the game installed (that is
where the script cache lives) but never launches it, never writes to it, and
never touches a save.

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
