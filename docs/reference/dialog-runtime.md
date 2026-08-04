# Dialog runtime internals

This page records the runtime evidence, hook-order contract, and current
limits behind `gore as compile-module` dialog topics and the generated
registration runtime. The user-facing authoring workflow is described in
[Dialog authoring](../guide/dialog-authoring.md).

## Proven runtime boundary

The controlled Viper fixture and version 1 of the public
`BuildSpec.dialog_topics` generator both validated this chain:

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

The central generation registry now also contains Steam build `24340829`, but
only for its separately recorded bounded offline authoring evidence. No
dialog-runtime candidate or qualification has been retained for `24340829`,
and none of the `24169431` runtime evidence carries forward across that
generation boundary. Offline composition and any separately authorized live
runtime requalification therefore remain open for the current build.

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
version-3 mock-runtime suite; it is not transactional if a native `AddTopic`
call itself fails. Such a failure stops all later mutation attempts, but the
failing call or an earlier successful call may already have mutated and neither
has a proven safe inverse. Version 3 is not yet a live-game proof.
The runtime never selects or removes a topic, scans global objects, starts a
conversation, uses a timer or console command, grants/activates an ability, or
writes a save/quest/knowledge field.

## Current limits

- Automatic discovery for a new module remains unproven.
- The controlled visual proof currently covers Gothic 1 Remake 1.0.3 with
  UE4SS 3.0.1. Both the reviewed v0.4 fixture and version 1 of the exact adapter
  emitted by the parameterized production generator completed the clean live
  visual proof. Runtime version 3 has a frozen offline candidate for the older
  build `24169431`: its deterministic builds, batch class preflight, observer
  behavior, forbidden-operation boundary, and exact sandbox closure were
  qualified offline, but its exact artifact never completed the same clean
  visual requalification. Build `24340829` has no dialog-runtime qualification
  at all. Other game, UE4SS, and runtime combinations remain to be qualified.
- Topic selection, authored knowledge/quest changes, recorded voice, and
  selection-side save effects are not certified by the insertion proof.
- The exact native ordering of knowledge rules, `IsVisible_Implementation`,
  participant checks, and UI relevance is not recovered.
- `emit-all` does not yet emit generated `__InitDefaults` methods as editable
  source. `compile-module --op edit` carries an existing `__InitDefaults`
  record only through the strict, base-keyspace remap path and only when the
  complete class identity/layout, ordinary method signatures and UFUNCTION
  metadata, constructors, behavior declarations, module globals/imports, and
  source identity remain exact. The vanilla generated record, emitter-omitted
  factory/spawn/accessor wrappers, every `Class.BehaviorFunctions` record, and
  the full local `Class.MethodTable` are then restored byte-for-byte and
  reparsed as a postcondition. The base header, complete module region, all
  seven tail tables, and EOF must parse exactly; the mixed result's serialized
  function IDs must stay unique both locally and against every untouched base
  module. A strict self-remap first proves that every copied vanilla reference
  resolves uniquely. `--allow-new-symbols`, an authored CDO `default`
  statement (ordinary switch `default:` labels are allowed), another generated
  `__*` shape, malformed/ambiguous identities, or
  any regenerated metadata/layout drift fails closed without writing a mini;
  base/source failures are rejected before rebuilding the source tree or
  launching the game compiler. Newly authored modules remain supported through
  `--op add`, including explicit `default` statements.
- Existing vanilla defaults now also have a separate offline, copy-on-write
  [`default-sites` / `patch-default` scalar path](../guide/angelscript-defaults.md).
  It re-resolves exact module/class/declaring-owner/field selectors, proves the
  target-to-owner ancestry, requires the complete current operand as a raw
  compare-and-swap guard, and changes no save or live runtime state. This does
  not make `__InitDefaults` source-editable: only a unique branch-free direct
  primitive/enum assignment in a one-terminal-`RET` initializer is admitted.
  Calls, computed expressions, structs, object handles, and containers remain
  unsupported by that scalar workflow. The separately sealed `tag-map-sites` /
  `patch-tag-map` workflow can patch only an already-present native
  `GameplayTag`-to-`float32` entry; it cannot add keys or maps, resize bytecode,
  or make generated defaults source-editable.
- Decompiled `Say` calls can omit the prepared `FText` argument. Use only a
  signature verified against `Binds.Cache` or a known compiling source template.
