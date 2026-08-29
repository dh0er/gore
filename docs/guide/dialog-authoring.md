# AngelScript dialog authoring

GORE can edit a shipped conversation module, regenerate its class defaults,
retain intentional new symbols, and package the resulting mini-cache. The
bounded new-topic shape keeps the new class and any existing `Subdialog`
wiring in that same module. This page separates source editing, localization,
strict standalone compilation, packaging, deployment, and runtime evidence.
The hook-order contract and exact live observations are documented in
[Dialog runtime internals](../reference/dialog-runtime.md).

## What is proven, and what is not

Read this before you build on it. Everything below is capability; this table is
status. The evidence and the exact wording behind each line are in
[Dialog runtime internals](../reference/dialog-runtime.md) — this is a summary
of that page, not a second claim.

| | |
|---|---|
| **Shown in game** | An authored topic rendered in a real conversation on Gothic 1 Remake **1.0.3**: caption independently confirmed, `RENDER_PASS`, the same object and exact class once each in both observed arrays. That historical version-1 proof restored the 123,394,250-byte shipping cache to its recorded SHA-256, with 92 of 93 saves byte-identical. On 2026-08-18, runtime version 3 also rendered `[Gore probe] UI fixture` on BuildID 24539464 and logged `ARMED`, `CHOICE_PASS`, and `RENDER_PASS` with `exact_count=1`; no topic was selected and no save was written. |
| **Current offline compiler/package proof** | On BuildID `24878692`, `gore doctor` accepted the installed Shipping cache and complete Binds API. Strict standalone `compile-module --op edit --allow-new-symbols` produced same-module topic minis for Payfine (17,085-byte sub-topic), Charlotte (8,271-byte root topic), and Brannok (104,047-byte sub-topic). All three offline bundles built and passed inspection: Payfine and Brannok each have one component/three files; Charlotte has two components/five files. The Brannok bundle is 104,448 bytes. None was deployed and the game was not launched, so appearance and selection remain unproven. |
| **Current-proof boundary** | The version-3 probe was GORE-authored and used the PR #91-fixed app-local Core DLL. Postflight restored the captured four-mod baseline loadout byte-for-byte and removed the temporary fixture. It does not qualify a third-party AngelScript mod or a three-way script conflict. |
| **Not certified by that proof** | Selecting the topic. Authored knowledge or quest changes. **Recorded voice on an authored topic.** Save effects on the selection side. |
| **Not qualified at all** | Steam build `24340829`, the exact frozen runtime-version-3 artifact for older build `24169431`, and game/UE4SS combinations other than the separately recorded proofs. |
| **Unproven** | Automatic discovery: that a newly authored root class reaches an already constructed `ConversationTopicSet` without explicit registration. |

One consequence worth stating plainly, because it spans two guide pages:
`gore voice add` writes a valid archive member, but nothing plays a brand-new
voice path until a line exists that resolves to it — which is an authored topic,
which is the thing above that is not certified for recorded voice. `gore voice
replace`, against a line the game already speaks, is the deployment path with
evidence behind it ([voice.md](voice.md)).

## Source edit: one existing conversation module

Start with `gore dialog tree <npc>`, then checkout the conversation. The emitted
source contains every reconstructed class-scope `default` statement, including
caption, priority, rules and flags:

```powershell
gore dialog checkout oc_stt_diego -o work
# edit work\Conversation_OC_STT_DIEGO.as
gore dialog check work
```

Existing topics may change those default values and their method bodies. The
authored defaults completely supersede the compiler-generated
`__InitDefaults`; they are not a partial overlay. `check` therefore rejects a
missing default-bearing class, a removed shipped default target, another
emitter-omitted generated `__*` method, or changed existing class/member/callable
layout. The old byte-exact carry remains only for source with no authored
defaults at all. It cannot be selected by deleting part of a normal checkout.

For a new topic, add the class at the **end of the existing conversation
namespace** in this same source file, before that namespace's closing brace.
This example uses the conversation-private Diego base because the declaration
now has both the module and namespace identity where that base is visible:

```angelscript
class UChoiceMyModDiego : UTopic_Hero__OC_STT_DIEGO
{
    default Caption = LocText("MY_MOD_DIEGO_CAPTION");
    default PriorityRank = 2;

    UFUNCTION()
    bool IsVisible_Implementation()
    {
        return true;
    }

    UFUNCTION()
    void Act_Implementation()
    {
        this.EndConversation();
    }
}
```

For a genuine sub-menu topic, also author its sub-topic flag and change the
existing parent method's `Subdialog` call to reference the appended class.
That exact same-module combination passes the complete-default checker and the
new-class/remap/loadout oracles. A strict standalone compile still requires a
qualified profile compatible with the installed game build; it fails closed
before compilation when the target Binds API has moved. Do not interleave the
class with or reorder shipped declarations.

`check` reports the added class, free functions and new string literals that
need remapper rows. It still rejects unresolved types and unsafe changes to
shipped ABI. This is a bounded new-topic path, not a claim that arbitrary
existing classes, signatures or member layouts can be migrated.

## Localization is a separate payload

`default Caption = LocText("MY_MOD_DIEGO_CAPTION")` makes the script refer to a
key; it does not create the localized row. Add that row explicitly with
`gore loc import --add-missing` or the bundle/Mod Studio localization editor.
Spoken text uses the same localization cache. Recorded voice-over is a third,
separate voice-ZIP edit, and authored-topic voice playback is not certified by
the existing runtime proof.

For an untranslated smoke test, `gore dialog new-topic --caption` emits a small
`FText::FromString` helper. For a distributable mod, prefer `--caption-key` and
real localized rows. The command writes a complete same-module edit workspace,
not a separate source module.

## Strict standalone compilation

After `check`, `gore dialog stage` writes the build spec and prints the exact
compile command. A new class or string makes it include
`--allow-new-symbols`; a body/default-only edit does not need that flag. The
command includes the resolved `--game` root, and `stage` first proves that this
installation's current script cache has the checkout hash. An arbitrary
`--cache` with no matching installation is valid for inspection, but cannot be
staged into a misleading compile command:

```powershell
gore dialog stage work --mod-name MyDialogMod
gore as compile-module --backend standalone --op edit `
  --module Story.G1R.Conversation.Conversation_OC_STT_DIEGO `
  --rel-path Story/G1R/Conversation/Conversation_OC_STT_DIEGO.as `
  --source work\Conversation_OC_STT_DIEGO.as --work-dir work\.gore-as-work `
  --allow-new-symbols -o work\MyDialogMod.mini.Cache --game $GAME
```

The strict standalone backend compiles and remaps without launching the game or
writing into its install. Complete authored defaults are what make
`--op edit --allow-new-symbols` safe: no stale `__InitDefaults` carry remains to
depend on the old keyspace. A partial default set still fails closed.

That exact path has now completed three times against installed BuildID
`24878692`. The strict standalone compiler/remapper produced a 17,085-byte
Payfine same-module sub-topic mini-cache, an 8,271-byte Charlotte same-module
root-topic mini-cache, and a 104,047-byte Brannok same-module sub-topic
mini-cache. Their offline bundles also built and passed inspection. Payfine and
Brannok each have one component and three files; Charlotte has two components
and five files. The Brannok bundle is 104,448 bytes. Nothing was deployed and
the game was not started. This evidence does not show any option, exercise root
registration, or prove selection.

The Brannok product oracle also covers the harder cached-module bridge shape:
real decompiled `LocText` temporary `Say` calls, `Subdialog`, cross-module class
values, cached mixins, and script-class type identities all bind in the current
strict standalone path.

Do not turn a generated topic scaffold into an isolated
`compile-module --op add` command. The conversation root is module-private, so
a separate module cannot derive from it. Nor can separate add and edit
mini-caches depend on one another: each mini is independently remapped against
the pristine base, and one mini never becomes symbol authority for the other.

Full-graph V2 is a different compiler product. It gives one standalone compiler
request the complete sealed base graph plus all coordinated add/edit/delete
changes, so otherwise-visible cross-module references can resolve together. Its
artifact is a complete regenerated script cache and a full-graph receipt, not a
base-bound one-module mini-cache. The normal bundle composer consumes module
minis and has no deployment recipe for that complete-cache artifact. For dialog
mods, keeping the new topic and any `Subdialog` rewiring in the same existing
conversation module is therefore the practical mini-bundle path.

## Root topics and explicit registration

`gore dialog new-topic` resolves the private topic base, participant, unused
class identity and vanilla sentinel. It checks out the complete existing
conversation module, inserts the class before the owning namespace's closing
brace, and records the root
`dialog_topics` entry in its edit manifest; `dialog stage` copies that entry
into the bundle spec automatically:

```powershell
gore dialog new-topic oc_stt_diego --caption-key MY_MOD_DIEGO_CAPTION `
  --class UChoiceMyModDiego --mod-name MyDialogMod -o work
gore dialog check work
gore dialog stage work --mod-name MyDialogMod
```

For a real sub-menu addition, pass `--subdialog-of <existing-parent-topic>`.
The command replaces one empty slot in that parent's single existing
`Subdialog` call, adds `default bIsSubTopic = true`, and records no root
registration. Ambiguous parents, multiple calls, or a full call with no empty
slot fail closed.

`check` binds this intent back to the authored source and the base graph. Each
new direct topic must be exactly one of: registered once for the resolved NPC
with that conversation's checked vanilla sentinel, or referenced once by a
shipped class's `Subdialog` call. A stale class path, deleted registration,
wrong participant/sentinel, duplicate row, or orphaned sub-topic is refused
before staging.

The staged root spec has this shape:

```json
{
  "meta": { "name": "MyDialogMod", "version": "0.1.0", "author": "Me" },
  "scripts": [
    {
      "op": "edit",
      "module_name": "Story.G1R.Conversation.Conversation_OC_STT_DIEGO",
      "mini_cache": "MyDialogMod.mini.Cache"
    }
  ],
  "dialog_topics": [
    {
      "id": "diego-test",
      "participant_name": "oc_stt_diego",
      "topic_class": "/Script/Angelscript.ChoiceMyModDiego",
      "sentinel_class": "/Script/Angelscript.ChoiceDiegoExitGamestart"
    }
  ]
}
```

Compilation creates the class but does not prove that an already constructed
`ConversationTopicSet` discovers it. Automatic discovery remains unproven, so a
new root topic needs this explicit adapter registration. A sub-menu topic is
instead reached by the authored `Subdialog` wiring in its existing module. The
Payfine and Brannok now have strict compile/remap and inspected offline-bundle
evidence for this shape; their in-game appearance and selection still await
runtime proof.

State-dependent root topics can pass `new-topic --allow-hidden`, which stages
`"allow_hidden": true`. A clean
zero-match after `IsVisible_Implementation` is then logged as `HIDDEN` and does
not block a visible sibling topic. Partial identity/class matches and duplicates
still fail closed. Omit the field when the topic must be visible on every
matching conversation opening.

Do not replace bounded registration with a global object scan, delayed
injection, console command, dynamic ability grant, or direct ability
activation. The latter already entered native conversation code with invalid
internal state.

## Package and deploy

```powershell
gore mod build --spec work\spec.json -o build
gore mod inspect build\MyDialogMod
# Explicit installation change, only after inspection and consent:
gore mod deploy --bundle build\MyDialogMod --game $GAME
```

`mod build` and `mod inspect` are offline packaging/validation steps. Deployment
is a separate installation mutation handled transactionally by the bundle
engine. The builder composes generated CDO overrides and dialog registration
into one self-contained UE4SS Lua component; ambiguous multiple hand-authored
roots are rejected.

Dialog registration is a transient topic-set mutation, so its component is
marked `opaque`. The manager still reports ordinary target overlaps and an
unknown-interaction advisory when another UE4SS mod shares the loadout; that
advisory does not invent a later-wins result.

## Safe validation order

1. Run `dialog check` and resolve every default-coverage, ABI or new-symbol
   finding.
2. Compile with strict `--backend standalone`; the install should remain
   untouched by definition.
3. Parse the mini-cache, resolve its tail tables, and disassemble every new
   function. Build and inspect the bundle before deployment.
4. Capture the installation/loadout and save baseline, deploy through the
   bundle engine, and confirm the class/CDO is resident.
5. Open the NPC menu naturally on a backed-up or disposable save.
6. For a topic expected to be visible in that state, require one `registration`
   and `attempt` sequence that reaches `ARMED`, then `CHOICE_PASS`, then
   `RENDER_PASS`, with one matching object identity and exact class in both
   observed arrays. A conditional topic may instead end at `HIDDEN`; if every
   armed topic is conditional and hidden, the batch also logs `CHOICE_EMPTY`.
   That proves exact topic-set membership plus a clean UI zero-match, not visual
   delivery. The same topic must still reach both PASS stages on a later state
   where it is expected to be visible.
7. Confirm the caption visually, select nothing, exit the game, and undeploy.
8. Verify the pristine cache hash, absence of deployment residue, and compare
   saves to the pre-run snapshot.

Selecting even an `Act_Implementation` that only calls `EndConversation()` is
not a save-neutral smoke test. Native conversation code can update
`ActedTopics` or knowledge outside the authored method. Selection behavior must
therefore be tested on a disposable save with semantic before/after inspection.

## Related

- [Reading and editing dialog trees](dialog-trees.md) — what the NPC already
  says, and which option a new one should sit next to
- [Scripts (AngelScript)](scripts.md)
- [Text & dialogs](text-and-dialogs.md)
- [Dialog runtime internals](../reference/dialog-runtime.md)
