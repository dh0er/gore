# AngelScript dialog authoring

GORE can compile a new dialog-topic class into an additive AngelScript module,
carry its new symbols in a mini-cache, compose that mini-cache into the
shipping script cache, and deploy or undeploy the result transactionally
through the bundle engine. This page covers the minimal authoring template,
the compile and packaging commands, conditional topic visibility, the
production integration paths, and the safe validation order. The runtime
evidence, hook-order contract, and current limits behind dialog-topic
insertion are documented in [Dialog runtime internals](../reference/dialog-runtime.md).

## What is proven, and what is not

Read this before you build on it. Everything below is capability; this table is
status. The evidence and the exact wording behind each line are in
[Dialog runtime internals](../reference/dialog-runtime.md) — this is a summary
of that page, not a second claim.

| | |
|---|---|
| **Shown in game** | An authored topic rendered in a real conversation on Gothic 1 Remake **1.0.3**: caption independently confirmed, `RENDER_PASS`, the same object and exact class once each in both observed arrays. That historical version-1 proof restored the 123,394,250-byte shipping cache to its recorded SHA-256, with 92 of 93 saves byte-identical. On 2026-08-18, runtime version 3 also rendered `[Gore probe] UI fixture` on BuildID 24539464 and logged `ARMED`, `CHOICE_PASS`, and `RENDER_PASS` with `exact_count=1`; no topic was selected and no save was written. |
| **Current-proof boundary** | The version-3 probe was GORE-authored and used the PR #91-fixed app-local Core DLL. Postflight restored the captured four-mod baseline loadout byte-for-byte and removed the temporary fixture. It does not qualify a third-party AngelScript mod or a three-way script conflict. |
| **Not certified by that proof** | Selecting the topic. Authored knowledge or quest changes. **Recorded voice on an authored topic.** Save effects on the selection side. |
| **Not qualified at all** | Steam build `24340829`, the exact frozen runtime-version-3 artifact for older build `24169431`, and game/UE4SS combinations other than the separately recorded proofs. |
| **Unproven** | Automatic discovery: that every class in a new module reaches an already constructed `ConversationTopicSet`. |

One consequence worth stating plainly, because it spans two guide pages:
`gore voice add` writes a valid archive member, but nothing plays a brand-new
voice path until a line exists that resolves to it — which is an authored topic,
which is the thing above that is not certified for recorded voice. `gore voice
replace`, against a line the game already speaks, is the deployment path with
evidence behind it ([voice.md](voice.md)).

## Minimal compiled topic

Derive from the existing conversation root for the target NPC. This exact
caption pattern avoids relying on the currently lossy decompilation of the
game's localization helper:

```angelscript
FText MyModCaption(const FName Text)
{
    return FText::FromString(Text.ToString());
}

class UChoiceMyModDiego : UTopic_Hero__OC_STT_DIEGO
{
    default Caption = MyModCaption(n"My test option");
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

`FText::FromString` is suitable for an unlocalized smoke test. A distributable
dialog should use a real localization ID and add it explicitly with
`gore loc import --add-missing` or through the bundle/Mod Studio localization
editor. Recorded voice-over remains a separate voice-ZIP edit.

## Compile and package

Use the high-level one-module command instead of manually replacing live game
files:

```powershell
gore as compile-module `
  --op add `
  --module MyMod.Dialog `
  --rel-path MyMod/Dialog.as `
  --source Dialog.as `
  --work-dir .gore-as-work `
  --allow-new-symbols `
  -o MyMod.Dialog.mini.Cache `
  --game $GAME
```

`--allow-new-symbols` is mandatory for a genuinely new class-bearing module.
For `--op add`, the module identity is derived from `--rel-path`
(`MyMod/Dialog.as` becomes `MyMod.Dialog`); `--module` remains a compatibility
hint and cannot silently give the new module a conflicting identity.
It retains only the new reference-table rows and remaps all existing symbols to
the selected vanilla cache. Declare that cache and the topic registration in a
bundle build spec:

```json
{
  "meta": { "name": "MyDialogMod", "version": "0.1.0", "author": "Me" },
  "scripts": [
    { "op": "add", "module_name": "MyMod.Dialog", "mini_cache": "MyMod.Dialog.mini.Cache" }
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

```powershell
gore mod build --spec spec.json -o build
gore mod deploy --bundle build/MyDialogMod --game $GAME
```

The builder composes generated CDO overrides and dialog registration into
exactly one self-contained UE4SS Lua component. A spec that would introduce
multiple hand-authored UE4SS roots is rejected rather than deployed with
ambiguous last-wins behavior. The bundle engine performs the guarded script
cache compose, backup, and restore operations.

Dialog registration mutates a transient topic set rather than a static
`Class.Field`, so its manifest marks the UE4SS component as `opaque` while
retaining any precise CDO-override targets in the same component. The mod
manager can still detect ordinary target overlaps and additionally reports an
unknown-interaction advisory when an opaque UE4SS component shares a loadout
with another UE4SS mod; that advisory has no invented later-wins winner.

## Conditional topics and integration paths

State-dependent topics can declare `"allow_hidden": true`. A clean zero-match
after the engine evaluates `IsVisible_Implementation` is then logged as
`HIDDEN` and does not block a visible sibling topic. Any partial identity/class
match or duplicate still fails closed, and only topics proven visible in the
choice array advance to the render proof. Omit the field for topics that must be
visible on every matching conversation opening; that strict behavior remains
the default. Mutually exclusive start/finish choices should normally set it on
both registrations.

Production authoring can use either integration path:

1. Add the class to an existing dialog module whose native discovery behavior
   is verified for the target NPC.
2. Declare a generated `dialog_topics` adapter with explicit participant,
   authored-class, and sentinel-class identities, managed by the bundle engine.

Mod Studio's standalone Dialogs tab stages the second form through explicit
in-memory fields and feeds it into the JSON build spec, which also remains
available for terminal and CI workflows. This standalone surface owns no
managed project/session state. Neither surface infers an NPC or sentinel from a
class name.

Do not replace this with a global object scan, delayed injection, console
command, dynamic ability grant, or direct ability activation. The latter was
already shown to enter native conversation code with an invalid internal state.

## Safe validation order

1. Compile to an output cache and confirm the install postflight is pristine.
2. Parse the mini-cache, resolve its tail tables, and disassemble every new
   function before deployment.
3. Deploy through the bundle engine and confirm the class/CDO is resident.
4. Open the NPC menu naturally on a backed-up or disposable save.
5. For a topic expected to be visible in that state, require one `registration`
   and `attempt` sequence that reaches `ARMED`, then `CHOICE_PASS`, then
   `RENDER_PASS`, with one matching object identity and exact class in both
   observed arrays. A conditional topic may instead end at `HIDDEN`; if every
   armed topic is conditional and hidden, the batch also logs `CHOICE_EMPTY`.
   That proves exact topic-set membership plus a clean UI zero-match, not visual
   delivery. The same topic must still reach both PASS stages on a later state
   where it is expected to be visible.
6. Confirm the caption visually, select nothing, exit the game, and undeploy.
7. Verify the pristine cache hash, absence of deployment residue, and compare
   saves to the pre-run snapshot.

Selecting even an `Act_Implementation` that only calls `EndConversation()` is
not a save-neutral smoke test. Native conversation code can update
`ActedTopics` or knowledge outside the authored method. Selection behavior must
therefore be tested on a disposable save with semantic before/after inspection.

## Related

- [Scripts (AngelScript)](scripts.md)
- [Text & dialogs](text-and-dialogs.md)
- [Dialog runtime internals](../reference/dialog-runtime.md)
