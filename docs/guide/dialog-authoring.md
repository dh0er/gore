# AngelScript dialog authoring

GORE can edit a shipped conversation module or scaffold a complete same-module
conversation, regenerate class defaults, retain intentional new symbols, and
package the resulting mini-cache. Topic classes and their `Subdialog` wiring
stay together in that one conversation module. This page separates source
editing, localization, strict standalone compilation, packaging, deployment,
and runtime evidence.
The hook-order contract and exact live observations are documented in
[Dialog runtime internals](../reference/dialog-runtime.md).

## What is proven, and what is not

Read this before you build on it. Everything below is capability; this table is
status. The evidence and the exact wording behind each line are in
[Dialog runtime internals](../reference/dialog-runtime.md) — this is a summary
of that page, not a second claim.

| | |
|---|---|
| **Current native dialog path** | On BuildID `24878692`, against pristine Shipping cache SHA-256 `7A18F954E32AF30FC24AE3A66EA35D3B5CB98560C8F5083C7846FC9CE1D77511`, a source-identical full Diego-module recompile ran normally, and a `Caption` edit rendered and selected correctly. A new same-module root appeared and was selectable in two runs: first while a legacy UE4SS adapter was present but skipped the conversation as `sentinel-topic-missing`, then with the proxy removed. The root was therefore discovered by the shipped script path, not inserted by UE4SS. |
| **New same-module sub-topic** | `[GORE TEST] Neuer Diego-Unterdialog` appeared in `UChoiceDiegoKolonie`, was selected, ran its newly compiled `Act` override, ended the conversation, and returned HUD and camera control. Stage A also rendered an exact 20-sibling submenu and allowed multiple slots to be selected. The default placement appeared immediately before Zurück and was selectable; explicit position 1 appeared first while Zurück stayed last and was also selectable. The earlier 4→5 edit remains working. |
| **Rules and persisted effects** | A new option added one ore nugget, changing inventory from 0 to 1; the item remained after quicksave and restart. Another option used `Rules.HideIfKnowsId` with `gore_diego_quest_knowledge_24878692`: it disappeared immediately after selection and remained absent after restart, while save-query found that exact ID on the hero. A new Stonehenge quest produced its toast and journal entry and remained `Running` after restart. |
| **New field and helper** | A new topic field with `default ProbeMarker = 24878692` and a helper method were authored and used successfully in game. |
| **New voice-over** | A new topic displayed its authored subtitle and played its new voice asset. System loopback matched the authored source with normalized correlation `0.763`. This proves that fixture's localization-to-voice lookup and audible playback, not every possible audio format or event. |
| **Automatic opening** | An ambient fixture entered `State.AmbientConversation` with `GA_Human_Conversation_Ambient` active without player selection. That proves automatic conversation activation at the state/ability boundary. The later crash occurred only after the fixture forced an artificial 20-choice `Subdialog`, so it does not qualify that menu shape or disprove ordinary ambient topics. |
| **Historical low-level adapter** | `BuildSpec.dialog_topics` still describes a separate UE4SS insertion adapter. Earlier Viper runs rendered a root through `AddTopic` and recorded `ARMED -> CHOICE_PASS -> RENDER_PASS`. It is historical low-level evidence, not part of the current `gore dialog new-topic` root workflow. |

## Practical limits only

This section deliberately leaves out everything that already works. It separates
possible-but-unproven game behavior from shapes the current GORE pipeline cannot
produce.

### Potentially possible, but not proven in game

- A complete conversation for an NPC with no current root topics. The checked
  source, strict standalone compile and bundle path are supported: GORE edits an
  exact shipped topicless conversation module when one exists, or adds one new
  module when none exists. Native runtime association and discovery have not
  yet been observed for either shape.
- A multi-level tree made entirely from new topics. Its root, children and
  `Subdialog` wiring can compile and package together in one conversation
  module, but navigating such an all-new tree has not yet been observed in game.
- Rendering or selecting an artificial 20-choice menu reached through an
  automatic ambient opening. Automatic activation itself reached the ambient
  conversation state and ability, but the forced menu shape then crashed.
- Running the intended cross-module dialog change from `gore as compile`'s
  selective complete-cache product. Its dependency resolution and Add/Edit
  composition are supported offline, and the earlier hybrid audit cache booted
  and loaded a save, but appearance, selection and execution of the intended
  cross-module dialog edge have not yet been observed.
- The same behavior on game builds other than BuildID `24878692`. Historical
  adapter observations on older builds do not qualify the current native source
  workflow there.

### Not technically supported by the current GORE pipeline

- Making one deployable mini-cache dialog patch depend on a new symbol supplied
  by another script module. Each module mini is remapped against the pristine
  base independently; use the separate complete-cache compile path for an
  acyclic coordinated Add/Edit graph.
- Building a complete-cache mod that deletes an existing script module. A
  missing base source file requests Delete, which fails closed until GORE can
  prune the global tail and prove that retained modules do not reference it.
- Coordinating new modules whose dependencies form a cycle. Selective
  composition can order an acyclic provider-to-consumer chain, but cannot safely
  seed a cycle and therefore rejects it.
- Deriving one new topic class from another new topic class. Every new option
  must derive directly from the conversation's private topic base; shared
  behavior has to live in helpers rather than an inherited topic hierarchy.
- Putting an already shipped option below a newly authored submenu parent. A
  shipped parent may receive a new child, and a new parent may contain new
  same-conversation children, but the checker does not admit a mixed child list
  owned by that new parent.
- Publishing the raw whole-tree compiler regeneration as a playable cache.
  GORE deliberately withholds it and publishes only the selective product. An
  earlier manually deployed raw result had 10,782 semantic deviations,
  including 81 in Diego, and reached a main menu whose input could not activate
  any entry; byte-exact deployment was not runtime compatibility.
- Adding a 21st child to one `Subdialog` call.
- Manually reshaping an already full 20-slot `Subdialog` call. The current-head
  Stage C bundle (mini-cache SHA-256 prefix `C675BB55…`) is byte-identical to
  the live-tested artifact, which failed to open the reshaped menu with an
  array-capacity error; treat this shape as technically unsupported and unsafe.
- Changing the base class, fields, member set or method signatures of a shipped
  topic.

`gore voice add` can supply the new archive member referenced by an authored
topic. The current Diego fixture proves one such new subtitle/voice pair through
audible runtime playback; [voice.md](voice.md) covers the separate voice payload
workflow.

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

For an existing declaration, compilation keeps the shipped `FunctionTraits`
and complete Unreal-function descriptor while replacing the bytecode and its
matching frame layout with the compiler's new output. This is what preserves
the native `Act`/`IsVisible` event binding across a full-module edit. A genuinely
new function has no shipped descriptor to inherit and therefore keeps the
metadata authored by the compiler from its source declaration.

For a new topic, add the class at the **end of the existing conversation
namespace** in this same source file, before that namespace's closing brace.
This example uses the conversation-private Diego base because the declaration
now has both the module and namespace identity where that base is visible:

```angelscript
class UChoiceMyModDiego : UTopic_Hero__OC_STT_DIEGO
{
    default DebugId = 7700385383056303891;
    default Caption = LocText("MY_MOD_DIEGO_CAPTION");
    default PriorityRank = 2;

    UFUNCTION(BlueprintOverride)
    bool IsVisible() const
    {
        return true;
    }

    UFUNCTION(BlueprintOverride)
    void Act()
    {
        this.EndConversation();
    }
}
```

The spelling above is intentional. `BlueprintOverride` makes the AngelScript
frontend publish the Unreal override metadata and lower the source methods
`IsVisible` and `Act` to the compiled records
`IsVisible_Implementation` and `Act_Implementation`. A brand-new class has no
shipped function record from which that metadata could be copied, so writing
`UFUNCTION()` plus an `_Implementation` source name is rejected by `dialog
check`. `IsVisible` must remain `const`, and a new topic may not add another
overload under either hook name. Each new topic also needs one authored nonzero
signed 64-bit `DebugId`; `dialog new-topic` derives one deterministically and
avoids values already present in the conversation module. Shipped continuation
chains sometimes reuse a `DebugId`, so `dialog check` does not impose a blanket
uniqueness rule on manually authored values.

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
separate voice-ZIP edit. On BuildID `24878692`, a newly authored Diego topic
displayed its new subtitle and played its new voice asset; a system-loopback
recording matched the authored source at normalized correlation `0.763`.

For an untranslated smoke test, `gore dialog new-topic --caption` and
`new-conversation --caption` emit a small `FText::FromString` helper. For a
distributable mod, prefer `--caption-key` and real localized rows. Each command
keeps the topic and its private base in one conversation source module.

## Strict standalone compilation

After `check`, `gore dialog stage` writes the build spec and prints the exact
compile command. A shipped or topicless module uses `--op edit`; a genuinely
new conversation module uses `--op add`. A new class or string makes it include
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

The current live qualification used BuildID `24878692` and pristine Shipping
cache SHA-256
`7A18F954E32AF30FC24AE3A66EA35D3B5CB98560C8F5083C7846FC9CE1D77511`.
A source-identical strict standalone recompile of Diego's complete conversation
module ran normally in game. A second recompile changed only the
`UChoiceDiegoExitGamestart.Caption` default to `[Forced Conversation]`; the
option appeared, was selected, ended the conversation, and returned control to
the player. That is live evidence for a full existing-module recompile and one
authored `Caption` change, not for the other default fields or a newly added
topic.

A separate new same-module sub-topic on that build strictly compiled to a
353,402-byte mini-cache. Its bundle was 353,811 bytes and Mod Manager deployed
it successfully. The option `[GORE TEST] Neuer Diego-Unterdialog` appeared in
the native sub-menu opened by `UChoiceDiegoKolonie`, was selected, ran the new
topic's `Act` override, ended the conversation, and returned HUD and camera
control. This is live evidence for the complete same-module sub-topic path on
that fixture, including new-symbol remap and native override dispatch.

The same campaign crossed four further runtime boundaries. A new root topic
appeared and was selectable twice; the first run's legacy adapter logged
`sentinel-topic-missing`, and the second ran with its UE4SS proxy removed, so
native script discovery supplied the option. A new inventory effect changed ore
from 0 to 1 and remained after quicksave/restart. A
`Rules.HideIfKnowsId("gore_diego_quest_knowledge_24878692")` option disappeared
immediately and stayed absent after restart, while save-query found the exact
knowledge ID on the hero. A new Stonehenge quest produced its toast and journal
entry and remained `Running` after restart. A new subtitle/voice pair played and
matched its authored source in system loopback at correlation `0.763`.

Finally, Stage A rendered an exact 20-sibling Diego submenu. Its renamed
shipped topic still ran the original long `Act` and returned to the menu, and
multiple slots were selectable. That proves the exercised structural edit, not
the currently unsupported and unsafe Stage C saturated reshape. `dialog check`
therefore rejects every structural change to a call that was already full in
the pristine module. Defaults, methods and `PriorityRank` remain editable when
that full call itself is unchanged.

Earlier offline compiler coverage on the same BuildID produced a 17,085-byte
Payfine same-module sub-topic mini-cache, an 8,271-byte Charlotte same-module
root-topic mini-cache, and a 104,047-byte Brannok same-module sub-topic
mini-cache. Their offline bundles built and passed inspection. Payfine and
Brannok each have one component and three files; Charlotte has two components
and five files. The Brannok bundle is 104,448 bytes. Those three earlier
fixtures were not deployed or game-tested.

The Brannok product oracle also covers the harder cached-module bridge shape:
real decompiled `LocText` temporary `Say` calls, `Subdialog`, cross-module class
values, cached mixins, and script-class type identities all bind in the current
strict standalone path.

Do not turn a generated topic scaffold into an isolated
`compile-module --op add` command. The conversation root is module-private, so
a separate module cannot derive from it. Nor can separate add and edit
mini-caches depend on one another: each mini is independently remapped against
the pristine base, and one mini never becomes symbol authority for the other. A
complete new conversation does not need that unsupported arrangement because
its settings, root and every topic live in its single added module.

Full-graph V2 is a different compiler product. It gives one standalone compiler
request the complete sealed base graph plus all coordinated Add/Edit sources,
so otherwise-visible cross-module references can resolve together. The raw
whole-tree regeneration is only dependency evidence and is never the published
cache. GORE starts from the exact pristine cache, then remaps and composes only
the source-classified Add/Edit modules in dependency order. Every untouched
module and every pre-existing global-tail record remain pristine; only records
required by new symbols are appended. Missing base sources request Delete and
fail closed because safe tail pruning and retained-reference proof are not
available; cyclic dependencies among new modules also fail closed.

This selective publication rule is based on two earlier live findings. A raw
regeneration with 10,782 semantic deviations, including 81 in Diego, installed
byte-for-byte but produced a main menu that would not accept mouse click or
Return. A manually composed hybrid cache retained pristine bytes for every
untouched module, replaced only Diego plus the new cross-module probe, and
booted and loaded a save normally. The product now automates that selective
architecture, but the intended cross-module dialog option still has not been
observed or selected in game.

The normal dialog bundle composer continues to consume independently base-bound
module minis, so it cannot package the coordinated graph as a normal
cross-module dialog mini-patch. Keeping the new topic and any `Subdialog`
rewiring in one conversation module remains the practical mini-bundle path;
use `gore as compile` when an acyclic cross-module dependency is essential.

## Root topics use native same-module discovery

`gore dialog new-topic` resolves the private topic base, participant and unused
class identity. It checks out the complete existing conversation module and
inserts the class before the owning namespace's closing brace:

```powershell
gore dialog new-topic oc_stt_diego --caption-key MY_MOD_DIEGO_CAPTION `
  --class UChoiceMyModDiego --mod-name MyDialogMod -o work
gore dialog check work
gore dialog stage work --mod-name MyDialogMod
```

For a real sub-menu addition, pass `--subdialog-of <existing-parent-topic>`.
The command shifts that parent's single existing fixed-width `Subdialog` call,
adds `default bIsSubTopic = true` and the shipped sub-topic `PriorityRank = 0`,
and records no root registration. By default it inserts immediately before a
trailing child whose caption key is `TEXT_BACK`, keeping Zurück/Back last; when
there is no such proven trailing child, it appends. Use
`--subdialog-position <N>` for an explicit 1-based position among the populated
entries. Existing entries at and after `N` shift right, so no child is silently
replaced. A stale source/graph order, a hole before a populated slot, an invalid
position, ambiguous or multiple calls, and a full 20-child call all fail closed.

`check` binds this intent back to the authored source and base graph. A direct
root must not set `bIsSubTopic`; a new direct child must be referenced by one
shipped parent's `Subdialog` call and set `bIsSubTopic = true`. Stale, duplicate
or orphaned intent is refused before staging.

The BuildID `24878692` Diego root appeared and was selectable in two runs. A
legacy adapter happened to be present in the first but logged
`sentinel-topic-missing` without arming or inserting anything. The proxy was
removed for the second run and the option appeared again. The supported root
workflow is therefore the ordinary same-module script edit: compile, package and
deploy the mini-cache; UE4SS is not required.

`BuildSpec.dialog_topics` remains a separate historical, low-level adapter
surface. It can package generated CDO overrides plus a UE4SS Lua component that
calls `ConversationTopicSet::AddTopic`; earlier Viper fixtures provide bounded
render telemetry for that path. `gore dialog new-topic` and `dialog stage` do
not need that adapter for a native same-module root, and the adapter evidence
must not be presented as a prerequisite for current root authoring.

## Complete new conversations and all-new trees

Use `new-conversation` only when the participant has no existing root topic:

```powershell
gore dialog new-conversation MY_NEW_NPC --caption-key MY_NEW_NPC_HELLO `
  --class UChoiceMyNewNpcHello --mod-name MyNewNpcDialog -o work
gore dialog check work
gore dialog stage work --mod-name MyNewNpcDialog
```

The command checks that the participant is one syntactically safe AngelScript
identifier and that generated module/class names do not collide with the base
cache. It can resolve one existing conversation match and refuses ambiguous or
already rooted matches. If there is no conversation match, however, GORE cannot
prove that this identifier belongs to a shipped NPC or to a new NPC authored in
another project payload: it uses the exact supplied identifier for the new
module. Prove or copy the NPC id from the catalog/project first; spelling and
runtime binding are the modder's responsibility. Exactly one shipped topicless
module is preserved and staged as an edit; no matching module becomes an add.

The generated source contains the conversation settings, its private root and
one direct choice. To author a deeper tree, append every additional topic to
that same source module and wire each parent to its children with `Subdialog`.
This includes new-parent-to-new-child edges: they no longer need a shipped
parent. For those edges, use the global
`::Subdialog(this, UChoiceChild, ...)` source form. The instance-method form
cannot bind a newly declared child in a completely new module. The fixed width
still permits at most 20 children per individual `Subdialog` call. Every new
topic must derive directly from the private topic base and stay beside it in the
same namespace; a new parent may own only new children from this conversation,
and no topic may come from a second new module.

Source checking, strict standalone compilation and script-only bundle
inspection cover both the existing-topicless edit and brand-new-module add
shapes, including an all-new multi-level tree. That is compile/package evidence,
not runtime evidence. GORE has not yet observed whether the game associates a
brand-new module with the intended NPC, discovers its root, or navigates the
complete all-new tree. The generated bundle contains no UE4SS component, and
none is needed to compile or package it offline. Whether successful runtime
discovery needs a bridge is exactly the still-open in-game question.

## Package and deploy

```powershell
gore mod build --spec work\spec.json -o build
gore mod inspect build\MyDialogMod
# Explicit installation change, only after inspection and consent:
gore mod deploy --bundle build\MyDialogMod --game $GAME
```

`mod build` and `mod inspect` are offline packaging/validation steps. Deployment
is a separate installation mutation handled transactionally by the bundle
engine. Existing-topic edits, same-module roots, same-module sub-topics and
complete same-module conversation workspaces are ordinary script mini-cache
bundles and ship no automatic UE4SS component. Existing-topic, same-module-root
and direct-sub-topic fixtures are proven without UE4SS insertion. For complete
new conversations and all-new trees, native discovery without a bridge remains
unproven rather than guaranteed.

If a hand-authored low-level bundle deliberately includes legacy
`dialog_topics`, its transient registration component remains `opaque`. The
manager then reports an unknown-interaction advisory when another UE4SS mod
shares the loadout; that advisory does not invent a later-wins result.

## Safe validation order

1. Run `dialog check` and resolve every default-coverage, ABI or new-symbol
   finding.
2. Compile with strict `--backend standalone`; the install should remain
   untouched by definition.
3. Parse the mini-cache, resolve its tail tables, and disassemble every new
   function. Build and inspect the bundle before deployment.
4. Capture the installation/loadout and save baseline, then deploy through the
   bundle engine.
5. Open the NPC menu naturally on a backed-up or disposable save.
6. Inspect an existing-topic edit, same-module root or same-module sub-topic in
   the native menu directly. For a complete new conversation, first test the
   script-only bundle without an insertion bridge; that observation decides
   whether native discovery works. If separately
   qualifying a hand-authored legacy `dialog_topics` adapter, require its
   `ARMED -> CHOICE_PASS -> RENDER_PASS` sequence and exact identity/class
   counts, then still confirm the option visually.
7. For a render-only check, confirm the caption visually and select nothing.
   For a selection check, use a disposable save, select the exact fixture, and
   verify its expected result such as conversation end and restored player
   control. Exit the game and undeploy.
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
