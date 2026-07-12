# AngelScript quest authoring

GORE can compile new `UQuest` subclasses, carry their generated defaults and
new symbols in an additive mini-cache, and compose them into the game's script
cache. Automatic discovery of new quest classes is narrowly runtime-proven on
the current Gothic 1 Remake generation. Quest transitions, authored effects,
dialog selection, and persistence require separate qualification.

## Proven discovery boundary

The standalone Asghan probe added two classes in a new AngelScript module:

- `UQuest_GORE_PROBE_ASGHAN_MINI`
- `UQuest_GORE_PROBE_ASGHAN_MINI_OBJ_DONE`

After an existing Old Mine save was loaded, the native crash report's quest
table contained both runtime instances as `EQuestState::Available`:

```text
"Instance_Quest_GORE_PROBE_ASGHAN_MINI": "EQuestState::Available"
"Instance_Quest_GORE_PROBE_ASGHAN_MINI_OBJ_DONE": "EQuestState::Available"
```

The retained report is
`C:\Users\Daniel\AppData\Local\G1R\Saved\Crashes\UECC-Windows-5605EA0F42879E207C3A7F89F291944F_0000\gothic_crash_info.log`,
lines 923–924, SHA-256
`1EAC2D1D12995BFF688E258D4DA1B4653BF6C70562DA47469DC6B6388D7B10B4`.
The candidate source and offline artifacts are retained under
`work/probe/asghan-miniquest/public-v1/`.

This proves that the native quest subsystem discovered and instantiated the two
added subclasses during world/save load. It does not prove:

- availability/start/success/failure predicate ordering;
- dialog-driven `StartQuest`, `SucceedQuest`, or `FailQuest` behavior;
- journal unlocks, rewards, knowledge, or `ActedTopics` effects;
- save/reload persistence or safe behavior after uninstalling a quest mod;
- compatibility with another game executable or future hotfix.

The later crash was caused by a retired direct conversation-ability activation
bypass, after both quest instances already existed. It is not evidence that the
quest discovery path failed. That bypass must not be reused.

## Existing quest edits

Ordinary methods in an existing quest module can use the strict
`compile-module --op edit` path. Generated `__InitDefaults` records remain
carried from the selected base module and are not generally source-editable.
Any edit must preserve the exact module/class/layout and metadata constraints
documented by the AngelScript compiler workflow.

Some existing primitive defaults have a narrower offline patch path. One
current sealed site is:

```text
module: Story.G1R.Quest.Quest_BanditsCamp_BANDITSTRUST
class:  UQuest_BanditsCamp_BANDITSTRUST
owner:  UQuest
field:  bExternalStartTrigger
type:   bool
value:  true
raw:    01000000
```

`gore as default-sites` can rediscover the exact selector and
`gore as patch-default` can perform a copy-on-write compare-and-swap edit on a
cache copy. This does not generalize to parent references, localized text,
arrays, journal structures, or arbitrary generated defaults.

## New quest authoring model

The game represents a quest and its objectives as `UG1RQuest` subclasses.
Generated defaults carry parent links, `EQuestKind`, external-transition flags,
involved characters, quest giver, localization identities, journal-document
links, and `bSucceedParent`. Ordinary methods implement predicates and
transition effects. A practical authoring graph therefore needs at least:

- one root quest and typed objective children;
- explicit initial, running, success, and failure ownership;
- external versus predicate-driven transition choice per edge;
- localization and journal-segment references;
- idempotent reward/effect handlers;
- typed dialog, NPC, item, and world-condition references.

The current Asghan candidate resolves and disassembles all 23 authored
functions offline. Its discovery proof upgrades new quest **class discovery**
from hypothesis to a supported narrow mechanism, but it does not upgrade the
candidate's transition/effect behavior to production-ready.

## Safe qualification order

1. Build the new module offline and reopen the mini-cache.
2. Verify the exact new class inventory and directly inspect every generated
   `__InitDefaults` record for parent, kind, flags, text, and journal links.
3. Require all authored functions and references to resolve and disassemble.
4. Build a bundle without deploying and verify its manifest and payload hashes.
5. On a disposable save, prove discovery/read-only state first.
6. Separately test acceptance/start, then one objective transition, then parent
   completion, journal/reward behavior, and save/reload persistence.
7. Compare the disposable save semantically and verify clean undeploy before
   widening the qualified capability.

Never manufacture a conversation with a console command, ability grant, or
direct activation in order to reach a quest callsite. Dialog selection and
quest effects must be exercised through a natural conversation on a disposable
save.

## Mod Studio boundary

Mod Studio can safely provide a typed Draft quest wizard, outline/transcript/
graph views, localization, deterministic source generation, dependency checks,
and offline compilation now. It may report new-class discovery as qualified for
the exact proven game generation. Production build remains blocked for any
transition, dialog effect, reward, journal action, or persistence behavior that
has not passed its own qualification gate.
