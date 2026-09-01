# Reading and editing dialog trees

`gore dialog` shows what an NPC actually says and checks structural edits before
they reach the compiler. It reads the game's own script cache and reconstructs
the whole conversation: every option in the menu, what unlocks or hides it, the
lines both sides speak, the effects a choice applies, and which sub-menu it
opens.

```powershell
gore dialog list viper                    # which conversations exist
gore dialog tree om_stt_viper_302         # the whole tree
gore dialog tree brannok --lang german    # in German
gore dialog show ChoiceStt302ViperMelt    # one option in full
gore dialog text viper -o viper.json      # its lines, ready to edit and re-import
gore dialog new-topic viper --caption-key K --mod-name MyMod -o MyMod   # root-topic scaffold
gore dialog new-conversation OC_GRD_Guard30_281N --caption-key K -o GuardDialog # first conversation
gore dialog checkout viper -o work        # editable AngelScript, including defaults
gore dialog export -o dialog\             # every conversation as JSON
```

The `gore dialog` commands work offline. They read the installed cache and write
only the files you request; they do not launch the game, change the install, or
touch a save. Strict standalone compilation and packaging are also offline.
Deployment and a runtime check are later, explicit steps with different proof
boundaries.

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
puts "End." last. Equal-rank subtopics retain their authored `Subdialog` slot
order. Rank `-1` is special: it requests the game's forced-topic behavior rather
than an ordinary menu position.

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

## Editing an existing option

Wording lives in the localization cache; structure and behaviour live in the
script cache. Checkout the conversation module to change the latter:

```powershell
gore dialog checkout om_stt_viper_302 -o work
# edit work\Conversation_OM_STT_VIPER_302.as
gore dialog check work
gore dialog stage work --mod-name ViperEdit
```

`checkout` writes the compiler-ready source, an untouched copy under
`pristine\`, and a manifest bound to the exact base cache. The source includes
the reconstructed class-scope `default` statements. They are authored source,
so existing topics may change their `Caption`, `PriorityRank`, `Rules` and
flags such as `bIsSubTopic`, `bIsAmbientTopic` and `bIsFollowupTopic`, as well
as `IsVisible_Implementation`, `Act_Implementation`, spoken lines, effects,
branches, and existing `Subdialog` calls.

Changing `Caption` to a new localization key changes only the script reference.
The localized row itself is a separate asset: add or edit it with `gore loc`
and include that localization change in the bundle.

### The fail-closed default contract

An authored default block replaces the compiler-generated `__InitDefaults`
record; it is not a patch layered over it. Therefore `check` requires all of
the following before compilation:

- If any shipped class authors defaults, every shipped class in the module that
  had `__InitDefaults` must still author defaults.
- Every shipped default target must remain present at least as often as in the
  checkout. Values and arguments may change, and new defaults may be added, but
  deleting a target cannot silently reset it to an engine value.
- An emitter-omitted generated `__*` method other than `__InitDefaults` blocks
  this path because class-scope defaults cannot supersede it.
- Existing classes, parents, member layouts and callable signatures remain
  fixed. Method bodies may change; existing declarations may not disappear.

The byte-exact generated-default carry still exists as the fallback when no
existing class authors defaults. With `--allow-new-symbols`, appended classes
may author their own defaults while existing initializers and compiler wrappers
remain byte-exact. Deleting defaults from a normal dialog checkout does not opt
into that fallback. A mixture containing authored defaults for only some
existing classes, or any other partial or ambiguous source, is refused before
an output mini-cache is written.

Checkout must have the matching `Binds.Cache` for the selected game build.
Without enough native type evidence the emitter writes no authored defaults for
the affected module, rather than exposing a partial set whose recompilation
could reset hidden values.

`check` reports changed method bodies and changed default targets. It also
reports added classes, free functions and string-table entries as requiring
`--allow-new-symbols`. Those new rows do not weaken the completeness checks
above.

`check` compares against the exact cache recorded by the checkout and refuses a
different game build. It inventories source structure, but the strict
standalone compile remains the syntax and type check.

## Adding a topic safely

For a concise list containing only the remaining practical limits, separated
into possible-but-unproven behavior and technically unsupported shapes, see
[Practical limits only](dialog-authoring.md#practical-limits-only).

The bounded new-topic mini-cache shape keeps both sides of the dependency in
one existing conversation module:

1. Checkout that conversation.
2. Append the new topic class at the **end of the existing conversation
   namespace**, before its closing brace; do not interleave or reorder shipped
   declarations or move them between namespaces.
3. For a sub-menu topic, edit the existing parent's `Subdialog` body to name the
   appended class.
4. Keep the complete authored defaults and run `check`. The report should name
   the new class (and any new strings) as new symbols, not as silent losses.
5. Run `stage`; its compile command uses `--op edit --allow-new-symbols` for
   this case.

This combination passes the complete-default checker and the same-module
new-class/remap/loadout oracles. Stage A also rendered an exact 20-sibling
submenu and allowed multiple slots to be selected. Payfine, Charlotte and Brannok fixtures provide
additional strict-standalone compile and inspected-bundle coverage. On BuildID
`24878692`, Diego fixtures also crossed the native runtime boundary: a new root
appeared and was selected twice without adapter insertion; a new direct
sub-topic appeared, selected and dispatched its override; and Stage A rendered
an exact 20-sibling submenu with multiple selectable slots while preserving a
shipped long `Act` and its return to the menu.
The default insertion appeared immediately before Zurück and was selectable;
explicit position 1 appeared first while Zurück stayed last and was selectable.
The separate 4→5 edit also remains working.

`gore dialog new-topic` creates that same-module edit workspace directly. For
a **root topic**:

```powershell
gore dialog new-topic om_stt_viper_302 --caption-key STT_302_VIPER_WORK_INFO_15_01 `
  --mod-name ViperWork -o ViperWork
```

It resolves the conversation-private topic base, participant and an unused
class name; then it checks out the complete conversation and inserts the class
into its owning namespace. Run `dialog check` and `dialog stage` on the output
directory. The command does not emit an isolated `compile-module --op add`
recipe: a private topic base from another module is not visible there, and
separate add/edit mini-caches cannot depend on one another.

Without `--priority-rank`, a root scaffold chooses a normal rank immediately
before the smallest recognized `TEXT_DIALOG_END`/`TEXT_BACK` root rank. When no
such caption exists, it places the new root before the current last root-rank
group; an empty fallback uses rank 2. Automatic selection skips `-1`, because
that rank has forced-topic semantics. Pass `--priority-rank <N>` when you want
an exact rank, including an intentional `-1`.

For a real sub-menu addition, add
`--subdialog-of UExistingParentTopic`. The parent must contain exactly one
`Subdialog` call with an empty topic slot. The command adds the new class with
`bIsSubTopic`, gives it the shipped sub-topic rank `0`, and shifts existing
arguments instead of merely filling the first null. If the last child uses the
language-independent `TEXT_BACK` caption key, the default insertion point is
immediately before that Zurück/Back option so it remains last. Otherwise the
new child appends after the existing entries. At the default equal rank, that
slot order is the visible order. `--priority-rank <N>` overrides the rank
exactly; use it only when rank ordering, rather than slot ordering, is intended.

Pass `--subdialog-position <N>` to choose the 1-based position among populated
entries explicitly. Position `1` is first; position `current count + 1` is
after the current last entry, even when that is Back. Existing entries at and
after `N` shift right. The option requires `--subdialog-of`, and a zero,
out-of-range position, stale source/graph order, non-packed call, ambiguous
call, or full call fails closed without dropping a child. One call has exactly
20 child parameters, so there is no 21st slot, and the scaffold does not
automatically restructure a saturated call.
The current-head Stage C rebuild (mini-cache SHA-256 prefix `C675BB55…`) matches
the live-tested artifact byte-for-byte,
but that artifact failed to open the reshaped 20-entry menu with an
array-capacity error; treat saturated reshaping as technically unsupported and
unsafe. `dialog check` now rejects any structural change to a call that was
already full in the pristine module; it still permits defaults, methods and
`PriorityRank` edits when the full call itself is unchanged.

A new root remains an ordinary same-module script edit. In the live Diego test
it appeared and was selectable while a present legacy adapter skipped as
`sentinel-topic-missing`, then appeared again after the UE4SS proxy was removed.
Native same-module discovery supplied it; `dialog_topics` registration is not a
requirement of this workflow. `check` instead binds direct roots and sub-topics
to their source shape: a root must not set `bIsSubTopic`, while a direct child
must be referenced once from a shipped `Subdialog` body and set it true.

`BuildSpec.dialog_topics` is retained as a separate historical low-level
adapter surface for hand-authored bundles. It packages UE4SS insertion and its
telemetry; it is not emitted as the normal `dialog new-topic` root recipe.

Full-graph V2 gives one standalone compiler request the complete sealed base
graph plus all coordinated Add/Edit sources, so visible symbols in different
modules can resolve together. Its raw whole-tree regeneration is intermediate
dependency evidence, not a deployable output. GORE publishes a complete cache
by retaining the exact pristine base and selectively composing only the
source-classified Add/Edit modules in dependency order; untouched modules and
all pre-existing global-tail records remain pristine, while records required by
new symbols may be appended. A missing base source requests unsupported Delete
and is rejected until safe tail pruning and retained-reference proof exist.
Cyclic dependencies among new modules also fail closed.

That design follows two live observations. An earlier raw regeneration with
10,782 semantic deviations, including 81 in Diego, installed successfully but
reached a main menu whose entries could not be activated. A manually composed
hybrid that preserved every untouched module and replaced only Diego plus the
new probe did boot and load a save. The product now uses the latter selective
architecture. On BuildID `24878692`, its complete-cache product booted and
loaded gameplay, showed and selected the new same-module root, and let an edited
shipped automatic topic call a new provider in another module. The provider's
line played and the conversation returned control.

The normal dialog bundle path instead consumes independently base-bound module
minis. One add mini cannot provide symbols to a separate edit mini, and the
bundle composer does not turn the selective complete cache into a dialog-mini
deployment.
Keeping the new class and the rewired `Subdialog` in the same existing module is
the supported mini-cache shape. Its compile/remap and offline packaging path is
proven on Payfine and Brannok; Diego additionally proves native in-game
appearance, selection and new override dispatch.

## Starting a complete conversation

`new-conversation` covers an NPC that has no current root topic:

```powershell
gore dialog new-conversation OC_GRD_Guard30_281N --caption-key MY_GUARD_HELLO `
  --class UChoiceMyGuardHello --mod-name MyGuardDialog -o work
gore dialog check work
gore dialog stage work --mod-name MyGuardDialog
```

The command requires one exact, already-loaded per-NPC conversation-settings
module from the shipped cache. It preserves that settings class, appends the
private root and first choice under `G1R::Conversation` in the same module, and
prepares `--op edit --allow-new-symbols`. An existing rooted conversation (use
`new-topic`), a partial/ambiguous NPC name, or a missing/malformed settings
anchor fails closed.

The first choice uses `PriorityRank = 2` unless
`--priority-rank <N>` is passed. An explicit `-1` intentionally creates a forced
topic; it is never an implicit default.

That loaded-module rule comes from runtime evidence. A separate new
`Story.G1R.Conversation` module for a shipped Guard compiled, packaged and
deployed but was not discovered. The same classes inside the Guard's loaded
settings module opened automatically and ran normally. `gore dialog` therefore
does not yet give a wholly new NPC its first conversation unless another
NPC-authoring path first supplies a settings module that the game loads.

More levels are authored by appending more topic classes to that same source
module and wiring new parents to new children with `Subdialog`. Every class
needed by the tree is therefore compiled and remapped as one unit; no second
module or dependent mini-cache is involved. Use the global
`::Subdialog(this, UChoiceChild, ...)` source form for a new-to-new edge; the
20-child limit still applies to each call. Every new option derives directly
from the private topic base and stays in its namespace. A new parent may own
only new children from this conversation; mixing a shipped child below that new
parent is not supported.

Put an unconditional top-level `Say` before one of two consecutive nested menu
transitions. A synthetic three-level tree with two actionless `Subdialog` Acts
soft-locked. Adding that `Say` before the second transition made Root -> level 2
-> level 3 render, select and end cleanly; the shipped corpus likewise contains
no consecutive actionless pair. `dialog check` rejects declarations,
assignments, empty blocks and conditional calls as substitutes for this proven
separator.

The anchored first-conversation edit and the action-bearing all-new tree now
pass source checking, strict standalone compilation, script-only packaging,
deployment and runtime selection. The Guard fixture opened automatically,
spoke a shipped line, rendered two wholly new submenu choices and returned
control after selection. No UE4SS component inserted the conversation.

## Compile, package, deploy, prove

`stage` writes `spec.json` and prints a strict standalone module edit command.
It adds
`--allow-new-symbols` only when `check` found intentional new class, function or
string rows. It also names the resolved game root and refuses to stage when its
current script cache is not byte-identical to the checkout base; `compile-module`
cannot safely target an unrelated archive passed only through `--cache`:

```powershell
gore as compile-module --backend standalone --op edit `
  --module Story.G1R.Conversation.Conversation_BC_BAN_BRANNOK_863 `
  --rel-path Story/G1R/Conversation/Conversation_BC_BAN_BRANNOK_863.as `
  --source work\Conversation_BC_BAN_BRANNOK_863.as `
  --work-dir work\.gore-as-work --allow-new-symbols `
  -o work\ViperEdit.mini.Cache --game $GAME
gore mod build --spec work\spec.json -o build
```

These are distinct evidence steps:

1. `dialog check` proves the source/edit invariants offline; it does not parse
   AngelScript with the compiler.
2. `as compile-module --backend standalone` performs the strict offline compile
   and remap. It neither launches the game nor writes into the install.
3. `mod build` packages the mini-cache and any separate localization or voice
   payload. It does not deploy them. A native same-module root needs no UE4SS
   component.
4. `mod deploy` or Manager Apply changes the installation and requires the
   normal consent and recovery workflow.
5. A natural in-game conversation is the runtime test. The Diego fixtures prove
   a native same-module root, a direct new sub-topic, a representative four-to-
   five-entry menu rebuild, one persistent inventory effect, one
   `HideIfKnowsId` rule, one persistent quest and one new subtitle/voice pair.
   The Guard fixture proves an automatically opened first conversation anchored
   in a loaded settings module; the three-level Diego fixture proves an
   action-bearing all-new tree. Use a disposable or backed-up save and keep each
   observed effect separate.

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

The current runtime proof includes the same rule family with an explicit ID:
`Rules.HideIfKnowsId("gore_diego_quest_knowledge_24878692")` hid its option
immediately after selection and after restart, and save-query found that exact
knowledge ID on the hero. Stage B separately proved ordering: `PriorityRank`
`-100` appeared first, `+100` last, and rank-zero entries preserved authored
order. A new `ProbeMarker = 24878692` field and helper method were also used
successfully in game.

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
caption; they show as `(ambient)`. One fixture entered
`State.AmbientConversation` with `GA_Human_Conversation_Ambient` active without
player selection, proving automatic activation at that state/ability boundary.
The anchored Guard fixture separately opened a wholly new conversation
automatically and completed normally. An artificial 20-choice ambient fixture
crashed before its menu became usable; that combined shape remains unqualified
and does not show that ordinary ambient or forced topics are broken.

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
- [Dialog authoring](dialog-authoring.md) — editing and adding safe topic shapes
- [Scripts (AngelScript)](scripts.md) — the cache this reads
- [Finding things](find.md) — looking one class or id up
