# AngelScript dialog authoring

GORE can compile a new dialog-topic class into an additive AngelScript module,
carry its new symbols in a mini-cache, compose that mini-cache into the shipping
cache, and deploy/undeploy the result transactionally. The complete visual path
has been validated in Gothic 1 Remake 1.0.3; automatic discovery and topic
selection are separate concerns described below.

## Proven runtime boundary

The controlled Viper fixture and the public `BuildSpec.dialog_topics` generator
both validated this chain:

```text
new UChoice subclass
  -> gore as compile-module --allow-new-symbols
  -> additive mini-cache
  -> gore-mod script-cache deployment
  -> resident /Script/Angelscript class
  -> ConversationTopicSet::AddTopic
  -> ClientShowChoiceUI
  -> ConversationWidget::OnShowTopicSelection
  -> visible root-menu option
```

For one live conversation attempt the fail-closed observer recorded:

```text
status=ARMED       mutation=added
status=CHOICE_PASS topics=3 identity_count=1 class_count=1 exact_count=1
status=RENDER_PASS topics=3 identity_count=1 class_count=1 exact_count=1
```

For the final production-generated run, the user independently confirmed the
caption `[Gore probe] UI fixture`. The same object address and exact class
occurred once in both observed arrays, and the render callback belonged to the
same widget. No topic was selected.

Afterwards `gore mod undeploy` restored the 123,394,250-byte shipping cache to
SHA-256
`1018F1CFE6B99A650EECB33AFB96752D691D2088EAD27808971B812F04ECB4C2`.
The loader, deployment record, backup, and isolation markers were absent, all
eight pre-existing mods were restored, and 92 of 93 save files remained
byte-identical. The only difference was three ASCII digits in the already-known
`/Engine/Transient.GothicScreenshotsSave_*` object name in
`PersistentDataList.sav`; its other bytes and every slot save were unchanged.

## Minimal compiled topic

Derive from the existing conversation root for the target NPC. This exact
caption pattern avoids relying on the currently lossy decompilation of the
game's localization helper:

```angelscript
FText MyModCaption(const FName Text)
{
    return FText::FromString(Text.ToString());
}

class UChoiceMyModViper : UTopic_Hero__OM_STT_VIPER_302
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
      "id": "viper-test",
      "participant_name": "om_stt_viper_302",
      "topic_class": "/Script/Angelscript.ChoiceMyModViper",
      "sentinel_class": "/Script/Angelscript.ChoiceStt302ViperExit"
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

## Discovery versus insertion

The runtime proof establishes that the added class is valid and renderable. It
does **not** establish that every class in a new module is automatically added
to an already constructed NPC `ConversationTopicSet`.

The reviewed fixture rules are now parameterized by `BuildSpec.dialog_topics`;
there are no Viper or Asghan constants in the generated runtime. During each
natural conversation pre-hook, every registration resolves its authored and
sentinel classes before any topic mutation, then requires the exact participant
identifier, a matching exact-class sentinel, and the expected conversation
ability/group/topic-set/widget relationship. It reuses an existing authored
topic when found and otherwise calls only `ConversationTopicSet::AddTopic`.
Later hooks require the same object identity and exact class exactly once in
both the choice and render arrays and record the
`ARMED -> CHOICE_PASS -> RENDER_PASS` proof.

Any missing class/hook, unreadable or malformed array, participant-name failure,
identity/class split, duplicate, or changed conversation object fails closed.
All declared authored and sentinel classes are preflighted as one batch before
the first mutation; a class failure prevents every registration from mutating.
Conversation-local context mismatches skip the current attempt. Classes are
resolved at the natural callback rather than loader startup because they may
load lazily.
The runtime never selects or removes a topic, scans global objects, starts a
conversation, uses a timer or console command, grants/activates an ability, or
writes a save/quest/knowledge field.

Production authoring can use either integration path:

1. Add the class to an existing dialog module whose native discovery behavior
   is verified for the target NPC.
2. Declare a generated `dialog_topics` adapter with explicit participant,
   authored-class, and sentinel-class identities, managed by the bundle engine.

Mod Studio's Dialogs tab stages the second form through explicit fields and
persists it in `.goremod` projects; the JSON build spec remains available for
terminal and CI workflows. Neither surface infers an NPC or sentinel from a
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
5. Require one `registration` and `attempt` sequence that reaches `ARMED`, then
   `CHOICE_PASS`, then `RENDER_PASS`, with one matching object identity and exact
   class in both observed arrays.
6. Confirm the caption visually, select nothing, exit the game, and undeploy.
7. Verify the pristine cache hash, absence of deployment residue, and compare
   saves to the pre-run snapshot.

Selecting even an `Act_Implementation` that only calls `EndConversation()` is
not a save-neutral smoke test. Native conversation code can update
`ActedTopics` or knowledge outside the authored method. Selection behavior must
therefore be tested on a disposable save with semantic before/after inspection.

## Current limits

- Automatic discovery for a new module remains unproven.
- The controlled visual proof currently covers Gothic 1 Remake 1.0.3 with
  UE4SS 3.0.1. Both the reviewed v0.4 fixture and the exact adapter emitted by
  the parameterized production generator completed the clean live visual proof.
  Other runtime combinations remain to be qualified.
- Topic selection, authored knowledge/quest changes, recorded voice, and
  selection-side save effects are not certified by the insertion proof.
- The exact native ordering of knowledge rules, `IsVisible_Implementation`,
  participant checks, and UI relevance is not recovered.
- `emit-all` does not yet emit generated `__InitDefaults` methods as editable
  source; verify important CDO defaults directly in the compiled candidate.
- Decompiled `Say` calls can omit the prepared `FText` argument. Use only a
  signature verified against `Binds.Cache` or a known compiling source template.
