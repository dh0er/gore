# Dialog runtime internals

This page records the runtime evidence, hook-order contract, and current
limits behind `gore as compile-module` dialog topics and the generated
registration runtime. The user-facing authoring workflow is described in
[Dialog authoring](../guide/dialog-authoring.md).

## Current native runtime boundary

On BuildID `24878692`, the tested new Diego root appended to its shipped
conversation module was reached through the game's native script path. It was
visible and selectable in two runs. In the first, a legacy UE4SS registration
adapter was present but logged `sentinel-topic-missing` and never reached
`ARMED`; in the second, the proxy was absent. The same root still appeared and
selected normally. Current same-module root authoring therefore requires the
script mini-cache, not `BuildSpec.dialog_topics` or UE4SS insertion.

The same campaign also proved one new direct sub-topic, one persistent inventory
effect, one explicit knowledge rule, one persistent quest, one new voice asset,
and one representative existing-menu rebuild:

- `[GORE TEST] Neuer Diego-Unterdialog` appeared in
  `UChoiceDiegoKolonie`, selected, dispatched its new `Act` override, ended the
  conversation and returned HUD/camera control.
- A new option changed the hero's ore count from 0 to 1. The item was present
  after quicksave and restart.
- A topic guarded by
  `Rules.HideIfKnowsId("gore_diego_quest_knowledge_24878692")` disappeared
  immediately after selection and remained absent after restart. Save-query
  found the exact knowledge ID on the hero.
- A new Stonehenge quest displayed its toast and journal entry and remained in
  `Running` state after restart.
- A newly added subtitle and voice asset played from the new topic. System
  loopback matched the authored source with normalized correlation `0.763`.
- Stage A rendered an exact 20-sibling submenu and multiple slots were
  selectable. The renamed shipped topic still ran its original long `Act` and
  returned to the menu.
- A new topic field with `default ProbeMarker = 24878692` and a helper method
  were authored and used successfully in game.
- Stage B isolated `PriorityRank`: rank `-100` appeared first, rank `+100`
  appeared last, and rank-zero entries preserved authored order.
- Placement is proven on rebased current-head bundles: the default new
  sub-topic appeared immediately before Zurück and was selectable; explicit
  `--subdialog-position 1` appeared first, kept Zurück last, and was selectable.
- The earlier 4→5 menu edit remains working. By contrast, current-head Stage C
  (mini-cache SHA-256 prefix `C675BB55…`) is byte-identical to the live-tested
  saturated artifact, which failed while opening the reshaped 20-entry menu
  with an array-capacity error.

These are fixture-specific runtime observations. The saturated 20-slot reshape
is therefore unsafe, while the separate 4→5 edit and placement variants work.

## Historical low-level registration-adapter boundary

The controlled Viper fixture and version 1 of the public
`BuildSpec.dialog_topics` generator both validated this chain:

```text
new UChoice subclass in the qualified fixture
  -> prepared mini-cache with new-symbol rows
  -> gore-mod script-cache deployment
  -> resident /Script/Angelscript class
  -> ConversationTopicSet::AddTopic
  -> ClientShowChoiceUI
  -> ConversationWidget::OnShowTopicSelection
  -> visible root-menu option
```

That chain is historical runtime evidence for the low-level registration
adapter, not an isolated cross-module `--op add` recipe and not a requirement of
the current native root workflow. The current deployable source path appends a
new class to the existing conversation module and emits an edit mini-cache.

For one live conversation attempt the fail-closed observer recorded:

```text
status=ARMED       mutation=added
status=CHOICE_PASS topics=3 identity_count=1 class_count=1 exact_count=1
status=RENDER_PASS topics=3 identity_count=1 class_count=1 exact_count=1
```

For the final version-1 production-generated run, the user independently
confirmed the caption `[Gore probe] UI fixture`. The same object address and
exact class occurred once in both observed arrays, and the render callback
belonged to the same widget. No topic was selected.

Afterwards `gore mod undeploy` restored the 123,394,250-byte shipping cache to
SHA-256
`1018F1CFE6B99A650EECB33AFB96752D691D2088EAD27808971B812F04ECB4C2`.
The loader, deployment record, backup, and isolation markers were absent, all
eight pre-existing mods were restored, and 92 of 93 save files remained
byte-identical. The only difference was three ASCII digits in the already-known
`/Engine/Transient.GothicScreenshotsSave_*` object name in
`PersistentDataList.sav`; its other bytes and every slot save were unchanged.

## Current BuildID 24878692 full-module proof

The current existing-topic proof used Steam BuildID `24878692` and pristine
Shipping cache SHA-256
`7A18F954E32AF30FC24AE3A66EA35D3B5CB98560C8F5083C7846FC9CE1D77511`.
First, the complete Diego conversation module was checked out, recompiled from
source without an authored change, deployed, and exercised in game. The
conversation ran normally. This is a live round-trip proof for a source-
identical full-module recompile, not merely a cache parser or compiler-backend
comparison.

The next run changed only the authored `Caption` default of
`UChoiceDiegoExitGamestart` to the already-existing text `[Forced
Conversation]`. The caption was visible, the option was selectable, selecting
it ended the conversation, and player control returned. This proves that one
reconstructed default survived checkout, strict compilation, remap, packaging,
deployment and runtime selection.

Earlier failed runs exposed a full-module metadata bug rather than a dialog-
default or new-symbol failure. Recompilation had replaced existing Unreal event
descriptors with ordinary callable functions, so native conversation dispatch
could enter its camera/input state without reaching the choice UI. The current
edit path preserves each identity-matched existing function's shipped
`FunctionTraits` and complete Unreal-function tail, while retaining the
compiler's regenerated bytecode and matching frame metadata. Functions that do
not exist in the shipped module keep their compiler-authored metadata instead;
new dialog methods therefore need source-correct `BlueprintOverride`
declarations.

A separate same-module new-sub-topic fixture on BuildID `24878692` completed
strict standalone compilation to a 353,402-byte mini-cache. Its bundle was
353,811 bytes, and Mod Manager deployed it successfully. The new `[GORE TEST]
Neuer Diego-Unterdialog` option appeared in the native sub-menu opened by
`UChoiceDiegoKolonie`. Selecting it dispatched the new topic's compiled `Act`
override, ended the conversation, and returned HUD and camera control. This is
live compile, package, deployment, reachability, rendering, selection and
override-dispatch evidence for that same-module sub-topic fixture.

A new same-module root crossed the same boundary twice. During the first run a
legacy registration adapter was installed, but it recorded
`sentinel-topic-missing` and no `ARMED`, choice or render pass; the option was
nevertheless visible and selectable. After removing the UE4SS proxy, the same
option appeared and selected again. This isolates the shipped script system as
the source of root discovery for this fixture.

Three effect fixtures then exercised save state. An inventory option raised the
hero's ore count from 0 to 1, and the item survived quicksave/restart. A
knowledge option authored
`Rules.HideIfKnowsId("gore_diego_quest_knowledge_24878692")`; it disappeared
immediately after selection and remained absent after restart, and save-query
found that exact ID on the hero. A Stonehenge quest option displayed its quest
toast and journal entry and remained `Running` after restart. Together these
prove the exercised `Rules`/flag and effect calls, not every possible native
quest, inventory or knowledge API.

The new-voice fixture displayed its authored subtitle and played its newly
packaged voice member. A system-loopback capture had normalized correlation
`0.763` with the authored source, establishing audible delivery rather than
inferring playback from subtitle duration alone.

The structural fixture changed one existing Diego menu from four children to
five. It renamed a shipped entry, adjusted its priority, retained its long
shipped `Act`, and appended a new option. In game the old `Act` completed and
returned to the menu, and the new option was selectable. Because priority,
caption and membership changed together, that older fixture did not isolate
`PriorityRank`; the later Stage B campaign above did.

## BuildID 24539464 version-3 live observation

On 2026-08-18, on Steam BuildID `24539464`, the version-3 Viper registration
runtime crossed the live render boundary again as part of the Mod Manager
campaign. The visible choice was `[Gore probe] UI fixture`, and `UE4SS.log`
recorded the ordered `ARMED`, `CHOICE_PASS`, and `RENDER_PASS` states with
`exact_count=1` at `2026-08-18 19:54:38`. The user opened the Viper conversation
but did not select the fixture, and no save was written during this check.

This was a GORE-authored fixture, not a third-party AngelScript mod. The
packaged Manager was built from the PR #90 merge, while the app-local Core DLL
used for this script check contained the PR #91 prepared-StaticNames fix. The
third-party #269 Gothic UI Reposition mod was disabled for this observation
after an earlier crash had been isolated to its own UE4SS Lua loop calling
`FindAllOf("W_Hotbar_C")` off the game thread; that crash contained no GORE or
AngelScript frame.

Postflight restored the captured four-mod loadout byte-for-byte, removed the
temporary campaign entries and Viper payload, restored the original signed
Core DLL, and reported the user's original four-mod deployment in sync. This is
one live proof of the current version-3 GORE path on one installation. It does
not retroactively qualify the frozen `24169431` artifact below, a third-party
AngelScript package, topic selection or persistence, or a three-way script
conflict.

## Historical version-3 requalification candidates

The production generator emitted runtime version 3 for the retained Viper
candidate under
`work/probe/viper-dialog-fixture/candidate-public-v3/`. It reuses the
live-proven 3,150-byte AngelScript mini and exact Viper registration, but freezes
the then-current batch-preflight runtime from repository HEAD
`48f52a9928f0e373c3fa06967a0505e2539d185e`.

Two public builds produced the same five files. The generated Lua is 30,519
bytes with SHA-256
`8C2B1FC454BB44CBDDCBF924EF9282E3BBE7023BC2BDD6B7485120AF578EAAC2`.
The version-3 verifier requires the exact participant/class/sentinel row, one
`AddTopic` site, nullable-object handling, render-before-mutation hook order,
and participant, all-active-class, context, and all-topic-lookup preflights
before the mutation loop. It rejects object scans, delayed/game-thread work,
console/key paths, direct conversation requests, ability grant/activation,
save/quest/knowledge/property writes, removals, and array mutation. The
retained qualification recorded all 31 focused runtime tests as passing.

A copied-cache deployment produced the same 7,306-module combined-cache hash as
the earlier visual proof. Undeploy restored the exact original seven-entry
sandbox tree with full-tree SHA-256
`07724E3444617A3DF56489C9132F6DDE0CE46E6523A1C6DE092C4077F03F05A8`;
the record, backup, loader, holder, recovery, and temporary residue were absent.
This is an offline/sandbox qualification, not a live result.

The Steam hotfix installed on 2026-07-14 is retained separately as the now-
historical `work/probe/viper-dialog-fixture/candidate-hotfix-24169431/`.
Its exact generation is `BuildID 24169431`, executable SHA-256
`B52CD0453AD03987B833F7F26D09A2075109F18D653B8D4FF95271C857139E5D`, and
Shipping-cache SHA-256
`757D8624F0C7480F63CC14A1BA2D7E43F461A529064B0C0CFBF523A54639E385`.
The game was stopped and source/copy length plus SHA-256 matched immediately
after both read-only copies. No save or installed file was written.

A full byte-faithfulness comparison aligns all 163,551 functions between the
retained 1.0.3 and hotfix caches with zero semantic differences and zero
module/function alignment loss, but 155,219 functions require reference-key
normalization. Therefore merely splicing an unchanged 1.0.3 mini into the
hotfix is not qualification: its generation-specific keys can be stale even
when the resulting cache parses.

A builder produced from repository commit `01147483` was retained by exact
hash. Using the old fully composed caches as identity sources, the qualifier
remaps Viper's 35 and Asghan's 195 references onto the hotfix base twice; both
pairs are byte-identical. Viper's hotfix mini is 3,150 bytes with SHA-256
`2F68D429CCE06CA3DFFAB4F03B1B5B1FCC845E81CE32EB8E897311B7FCDA6F32`;
its runtime-v3 Lua remains the exact 30,519-byte safety-verified artifact.
Asghan's hotfix mini is 12,537 bytes with SHA-256
`CEBD9F93C9532E17FEC9969CF8CC724BAC0CDC5D48711493FDF97A6F2434B56D`;
it now builds with the production runtime-v3 adapter rather than the old
runtime-v2 artifact.

Two fresh builds per candidate reproduce their exact five-file bundles. Viper
deploys to a 7,306-module, 123,397,348-byte cache with SHA-256
`E252EDD5226B0D941E7FC78DC2F7DC53FDE479A7CCEAC5D46384858B44CAE4CA`;
Asghan deploys to a 7,306-module, 123,406,735-byte cache with SHA-256
`172F57C2CE73468458F25AC6210AB4E53738D53738D44D88087DA25E71A9909E`.
Header/module parsing, disassembly, and decompilation find both authored
modules and their required dialog/quest functions. Each undeploy restores the
exact seven-entry copied tree with SHA-256
`F46A1073F7B632EFF69268AA0E3863685D514BB01CC9D2F972844447E7717824` and
leaves no record, backup, loader, holder, recovery, temporary, or transient
build residue. The candidate-local `run-hotfix-offline-qualification.ps1`
reproduced both closures; the retained qualification records the focused
runtime suite as 31/31 green. This cleared the then-current hotfix's offline
composition prerequisite, not either live runtime boundary or the exact-
current arbitrary-source compiler gate.

The retained `24169431` candidate has its own copied
`live-qualification.ps1`, resealed to that executable, pristine cache,
hotfix-identity-remapped Viper mini/runtime, deployed-cache hash, and complete
offline closure. The pure log/parser suite accepts one exact ordered session
and rejects twelve duplicate, reordered, stale, inconsistent, or malformed
cases. AST checks keep deployment, process, marker, network, and shell commands
out of the harness. It admits only the pinned UE4SS 3.0.1 Beta #0 build
(`272ce2f8`) with the sealed loader/proxy payloads, the exact game process and
executable generation, the exact save root and sealed pre-run save tree, the
sole fixture marker, and a sealed log prefix followed by the one ordered
version-3 sequence while the menu is open. Postflight checks require the
pristine cache, restored markers, no reparse points, and byte-identical saves
after separately performed undeploy. The sole admitted save difference is a
same-length numeric change to the root `GothicScreenshotsSave_<id>` token in
`PersistentDataList.sav`; any other difference fails. The harness never
deploys, isolates mods, launches the game, clicks, saves, or restores files.
Those setup and cleanup operations stay separate and visible. One natural
Viper-menu open with no selection and no save remains the only in-game
interaction that would complete a render-only runtime-v3 requalification on
build `24169431`; it has not been run for that exact artifact. Asghan remains a
separate behavioral qualification because selecting its fixture can change
quest/save state. The offline evidence qualifies its hotfix-remapped
build/composition only, not selection, effects, persistence, or save/reload.

The central generation registry also contains Steam builds `24340829` and
`24878692`, but only for their separately recorded bounded offline authoring
evidence. No dialog-runtime candidate or qualification has been retained for
either, and none of the `24169431` evidence carries across a generation
boundary. Build `24539464` instead has the separate version-3 live observation
recorded above; it does not convert either historical candidate into a
qualified artifact.

## Native discovery versus the historical insertion adapter

The current Diego proof establishes native discovery for a root class appended
to that NPC's shipped conversation module. It does **not** establish discovery
for a class in a brand-new module, a brand-new NPC conversation, or a symbol
graph split across modules. The checked compiler and package path now covers a
single brand-new conversation module and an all-new topic graph kept inside it;
that offline evidence does not close this native-discovery gap.

The remainder of this section documents the separate historical
`BuildSpec.dialog_topics` insertion adapter. It is retained as a low-level
bundle surface and evidence record; it is not emitted as a prerequisite for the
current `gore dialog new-topic` same-module root path.

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
The bounded participant set is inspected first without resolving classes. All
authored and sentinel classes declared for participants in that exact current
conversation are then preflighted as one batch before the first mutation; an
unrelated NPC's unavailable class cannot poison the active conversation. A
missing or malformed active class records `BATCH_FAIL` and prevents every
active registration from calling `AddTopic` or reaching `ARMED` in that
attempt. Existing authored-topic lookups are likewise completed for all
locality-qualified registrations before mutation. Conversation-local
participant or exact-sentinel-topic mismatches remain registration-specific
skips. Classes are resolved at the natural callback rather than loader startup
because they may load lazily. This preflight-atomic behavior is covered by the
version-3 mock-runtime suite, and the build-`24539464` Viper observation covers
one successful live render path. It is not transactional if a native `AddTopic`
call itself fails. Such a failure stops all later mutation attempts, but the
failing call or an earlier successful call may already have mutated and neither
has a proven safe inverse; the live proof did not force that failure path.
The runtime never selects or removes a topic, scans global objects, starts a
conversation, uses a timer or console command, grants/activates an ability, or
writes a save/quest/knowledge field.

## Practical limits

This list deliberately omits the dialog edits and runtime behavior already
shown to work.

Existing-topic edits, same-module roots, same-module sub-topics and complete
same-module conversation workspaces are native script-cache payloads. Their
ordinary bundles contain no generated UE4SS component. Existing-topic edits,
same-module roots and direct sub-topics have runtime evidence without UE4SS
insertion; complete new conversations do not yet, so whether native discovery
needs a bridge remains open. `dialog_topics` is a separate historical low-level
adapter surface.

### Potentially possible, but not proven in game

- A complete conversation for an NPC with no current root topics. The offline
  product path edits an exact shipped topicless module when one exists, or adds
  one new conversation module when none exists. Both check, compile and package;
  native runtime association and discovery remain unobserved.
- A multi-level tree whose root, parents and children are all newly authored in
  that one module. Its new-to-new `Subdialog` edges compile and package, but the
  complete tree has not yet been opened and navigated in game.
- Rendering or selecting an artificial 20-choice menu reached through an
  automatic ambient opening. The fixture first entered
  `State.AmbientConversation` with `GA_Human_Conversation_Ambient` active, which
  proves automatic activation at that state/ability boundary; the forced menu
  shape then crashed and remains unqualified.
- Executing the intended cross-module dialog edge from `gore as compile`'s
  selective complete cache. The dependency graph resolves and the declared
  Add/Edit modules compose offline, while an earlier hybrid audit cache booted
  and loaded a save; the option itself has not been observed or selected.
- Game builds other than BuildID `24878692`. Older adapter observations do not
  qualify the current native source path on those builds.

### Not technically supported by the current GORE pipeline

- Packaging a normal deployable mini-cache dialog whose new symbol dependency
  comes from another script module. The selective complete-cache compile is the
  separate path for an acyclic coordinated Add/Edit graph.
- Deleting an existing module through the complete source tree. Missing base
  source requests Delete and fails closed because safe tail pruning and proof
  that retained modules no longer reference it are not available.
- Selectively composing a dependency cycle among newly added modules. An
  acyclic provider-to-consumer chain is supported; a cycle cannot be seeded
  safely and is rejected.
- Deriving one new topic from another new topic instead of deriving every new
  option directly from the conversation's private topic base.
- Making a newly authored submenu parent own an already shipped child. Existing
  parents may receive new children; newly authored parents are limited to new
  children from the same conversation module.
- Publishing the raw FullGraph backend regeneration as a playable cache. GORE
  does not expose it as the product output; an earlier manual deployment reached
  a main menu whose entries could not be activated by mouse click or Return.
- Adding a 21st child to one `Subdialog` call.
- Manually reshaping an already full 20-slot `Subdialog` call. Current-head
  Stage C (mini-cache SHA-256 prefix `C675BB55…`) is byte-identical to the
  live-tested artifact, which failed to open the reshaped menu with an
  array-capacity error; this shape is technically unsupported and unsafe, and
  `dialog check` rejects it before staging.
- Changing the base class, fields, member set, or method signatures of a shipped
  topic.

## Current compiler and preservation contract

- Dialog checkout now emits every reconstructed `__InitDefaults` record as
  class-scope `default` statements. Those statements are compiler input, so
  `Caption`, `PriorityRank`, `Rules` and topic flags can change along with
  method bodies. The compiler regenerates `__InitDefaults`; it does not restore
  the old record over authored values.
- For each identity-matched existing function, the edit pipeline preserves the
  shipped `FunctionTraits` and complete Unreal-function tail. The regenerated
  bytecode and its frame metadata remain paired and are not copied back from
  Shipping. A genuinely new function has no base record to inherit and keeps
  the compiler-authored metadata produced from its source declaration.
- Authored defaults are an all-or-nothing supersession. Every base class with
  `__InitDefaults` must still author defaults, and every shipped default target
  must remain present at least as often as in the checkout. A different value,
  call argument or additional target is allowed; a missing class or target,
  malformed/ambiguous source, or another emitter-omitted generated `__*` method
  fails closed before a mini-cache is written. Existing class ancestry,
  property layout and callable identities also remain fixed because no runtime
  ABI migration for shipped classes is proven.
- Byte-exact generated-method carry remains an all-or-nothing fallback only
  when the overlay authors no defaults. It uses strict base-keyspace remapping,
  cannot be combined with `--allow-new-symbols`, and requires exact class
  identity/layout, ordinary and UFUNCTION signatures, constructors, behavior
  declarations, module globals/imports and source identity. The generated
  record, emitter-omitted factory/spawn/accessor wrappers, every
  `Class.BehaviorFunctions` record and the local `Class.MethodTable` are then
  restored byte-for-byte. The complete mixed module and all seven tail tables
  are reparsed, function ids must remain unique, and a strict self-remap first
  proves every copied vanilla reference resolves uniquely. Removing part of an
  authored checkout cannot opt into carry; the partial overlay is rejected.
- Once every generated default is superseded by authored source,
  `compile-module --op edit --allow-new-symbols` may retain the minimal new
  class, function, name and string rows. The bounded dialog edit shape appends a
  new topic class inside the owning namespace of the same existing conversation
  module and changes an existing `Subdialog` body to reference it. Qualified
  class identity and namespace residence are part of the fail-closed check.
  Complete-default, same-module new-class/remap and cross-mini loadout oracles
  cover that shape. On BuildID `24878692`, Doctor accepted the installed
  Shipping cache and complete Binds API. Strict standalone compilation/remap
  produced a 17,085-byte Payfine same-module sub-topic mini-cache and an
  8,271-byte Charlotte same-module root-topic mini-cache. Their offline bundles
  built and passed inspection: Payfine has one component/three files and
  Charlotte has two components/five files. A current Brannok checkout plus a new
  same-module sub-topic also strictly compiled/remapped to a 104,047-byte mini;
  its 104,448-byte one-component/three-file offline bundle built and passed
  inspection. A later same-module sub-topic on the same build produced a
  353,402-byte mini-cache and a 353,811-byte bundle that Mod Manager deployed
  successfully. In game it appeared in `UChoiceDiegoKolonie`'s native sub-menu,
  was selected, ran the new `Act` override, ended the conversation and returned
  HUD and camera control. This does not authorize reparenting or changing the
  members/signatures of shipped classes.
- `dialog new-conversation` keeps conversation settings, the private root and
  every topic in one module. Exactly one shipped topicless module is an edit;
  the absence of any conversation module is an add. The scaffold starts with
  one direct choice, and further new classes may form new-to-new `Subdialog`
  edges inside that same source. Those edges use the global
  `::Subdialog(this, UChild, ...)` source form because the instance-method form
  does not bind the newly declared child in a completely new module. Source
  checking, strict standalone compilation
  and bundle inspection cover both operations and an all-new multi-level tree.
  They do not prove that the game associates a brand-new module with the NPC,
  discovers its root, or permits navigation through the resulting tree. For an
  add, the participant is validated only as a safe identifier and against
  generated-name collisions; its existence in a shipped catalog or a separate
  NPC payload is not known to this command.
- Separate add and edit mini-caches cannot depend on one another: a new module
  cannot see the conversation-private root, and the edit mini cannot resolve a
  class supplied only by the add mini. Each mini is remapped independently to
  the pristine base; neither becomes authority for the other. Full-graph V2
  instead submits the complete sealed base graph and coordinated Add/Edit
  changes to one standalone compile, so visible cross-module references can be
  resolved together. The raw regenerated cache is retained only as dependency
  evidence. Publication starts from the exact pristine cache and selectively
  remaps and composes the declared Add/Edit modules in dependency order;
  untouched modules and all pre-existing global-tail records remain byte-exact,
  while records required by new symbols may be appended. Missing base source
  requests Delete and is rejected until safe tail pruning and retained-reference
  proof exist. Cyclic dependencies among new modules likewise fail closed.
  An earlier raw output had 10,782 semantic deviations, including 81 in Diego.
  Mod Manager nevertheless installed that exact raw cache manually
  (`62A2106966A06910376ABDF956FF7DFA83F0F366A91514EB1B3D51F227800CD9`) and
  verified the installed bytes; the game reached its main menu, but neither
  mouse clicks nor Return could activate an entry. This proves complete-cache
  deployment and a concrete runtime incompatibility, which is why raw backend
  bytes are no longer publishable product output.
  A separate hybrid cache
  (`7C07974034F4D1CC8CF0CB4469FC97F9956B8F23924B9E4447927EF4F83B85EF`) kept
  every untouched module pristine and replaced only Diego plus the new probe.
  It booted and loaded a save, proving the replacement mechanism and selective
  architecture can reach gameplay. The product now automates that architecture,
  but the live observation does not prove appearance/selection of the
  cross-module option. The ordinary bundle
  composer consumes independently base-bound module mini-caches, so neither
  complete-cache experiment becomes a normal cross-module dialog mini-patch.
- A new root appended to Diego's shipped conversation module was discovered,
  rendered and selected natively with no adapter insertion, including one run
  with the UE4SS proxy absent. A same-module sub-topic instead uses authored
  `Subdialog` wiring, and the Diego fixture proves its in-game appearance,
  selection and new override dispatch. The supported source gate distinguishes
  these shapes by `bIsSubTopic` and a direct child reference from one shipped
  parent. `BuildSpec.dialog_topics` remains only the separate low-level adapter
  contract documented above.
- Source checking, strict standalone compilation, bundle packaging, deployment
  and runtime observation are separate claims. On BuildID `24878692`, the
  source-identical complete Diego recompile, Caption edit, native root, direct
  sub-topic, persistent ore/knowledge/quest effects, new voice asset and
  four-to-five-entry menu rebuild all crossed the live boundary. Earlier
  Charlotte, Payfine and Brannok fixtures stop at offline build/inspection, and
  the historical registered-root fixture proves a separate adapter render
  path. None of those older facts is needed to claim the current native root.
- Existing vanilla defaults also retain the separate offline, copy-on-write
  [`default-sites` / `patch-default` scalar path](../guide/angelscript-defaults.md).
  It re-resolves exact selectors and admits only a unique branch-free direct
  primitive/enum assignment guarded by the complete current operand. Calls,
  computed expressions, structs, handles and containers remain outside that
  narrow workflow. `tag-map-sites` / `patch-tag-map` likewise changes only an
  already-present native `GameplayTag`-to-`float32` entry. Neither scalar path
  bypasses the complete-default contract for a source edit.
- Real decompiled Payfine and Brannok `Say` calls that pass prepared `LocText`
  temporaries compile in the current strict standalone path. The Brannok product
  oracle additionally covers `Subdialog`, cached cross-module class-value
  expressions, cached mixins, and reconstructed script-class type identities.
