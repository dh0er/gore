# Game updates

Steam patches Gothic 1 Remake without warning. Some of GORE keeps working
untouched; some of it refuses until the new build has been qualified. This page
says which is which, what a maintainer has to check, and why the line falls
where it does.

## Why anything breaks at all

Most commands read the game's files and describe what they find. Those cannot go
stale — a texture is a texture on any build.

A smaller set *mutates* the game from offline evidence: it decides where to write
by reasoning about native class layout, field ownership and default-value sites
it cannot re-derive at runtime. Being wrong there means writing a plausible value
into the wrong field. So that evidence is sealed: pinned by SHA-256 to the exact
build it was audited against, and the seal fails closed. A single changed byte
empties the whole table and every dependent lookup answers "unknown" rather than
guessing.

That is a deliberate trade. The cost is that a routine patch disables real
capability until somebody re-qualifies. The benefit is that it has never
silently written to the wrong field.

## What a patch moves

Three identities travel together and all three are sealed:

| file | what it is |
|---|---|
| `G1R\Binaries\Win64\G1R-Win64-Shipping.exe` | the build |
| `G1R\Script\PrecompiledScript_Shipping.Cache` | the AngelScript module cache, carrying a GUID in its first 16 bytes |
| `G1R\Script\Binds.Cache` | the native binding table: class → `/Script/` path, and field → type |

Deployable AngelScript mini-caches are exact-generation artifacts too. GORE
stamps the target cache GUID during remap and, before any installation mutation,
requires the mini GUID to equal the effective base cache and resolves its
bytecode and retained table dependencies against that base. A game update that
changes the cache therefore invalidates old minis even when their module names
still exist; rebuild or remap them against the new pristine cache.

A fourth input is sealed but **not shipped by Steam**: the `.usmap` reflection
dump under `G1R\Binaries\Win64\ue4ss\`. UE4SS generates it on your machine. Its
filename carries the engine build (`G1R-5.4.3-<build>-<hash>.usmap`), so a stale
dump is visible by name — but nothing forces it to be regenerated, and an old
dump describing a previous executable will pass its own hash check while
describing the wrong game. This is the one input where every seal can be green
and the answer still wrong. Re-dump it before trusting anything downstream.

## What worked before and after qualification

The following table is the retained fail-closed snapshot measured immediately
after the 2026-07-31 update (build `24340829`), before its generation row was
qualified and admitted:

| | |
|---|---|
| **Unaffected** | `texture`, `loc`, `audio`, `voice`, `asset`, `mod build` overrides, `as decompile` / `emit` / `walk` / `info`. None of these consults sealed evidence. |
| **Degraded** | `as default-sites`, `as patch-default`. They fall back to scalar-only: sites the script cache can type on its own stay editable, anything needing native ancestry does not. |
| **Refused** | `as tag-map-sites`, `as patch-tag-map`, and Mod Studio's story, NPC, quest and item authoring. |

After qualification, the central registry contains exactly four reviewed Steam
rows: Steam 1.0.3 Hotfix 1, build `24169431`, build `24340829`, and build
`24878692` from the 2026-08-27/28 update. Each row is a closed exact-generation
admission, not a range or a promise about nearby builds.

The `24340829` row retains its bounded offline qualification: no class was
reparented, no property owner moved, the curated modules reproduced
byte-for-byte, and the audited Item field matrix stayed unchanged.

The `24878692` row seals the new executable, pristine Shipping cache and
`Binds.Cache` together. Its fresh 172709 USMAP,
`G1R-5.4.3-172709-272ce2f8.usmap`, is required: the stale 171261 dump is not
evidence for this row because it omits
`/Script/G1R.RagdollConfig.m_FollowBone`. The bounded comparison establishes a
new compatibility generation with these exact facts:

- The USMAP still has 6,602 class rows, the direct-parent graph and native
  ancestry profile are unchanged, no class was reparented, and no existing
  property's declaring owner moved. The Binds/USMAP bridge remains 6,582
  classes with zero unresolved default fields.
- The GameplayTag-to-float32 profile is unchanged: eight declarations and
  1,432 cache operands.
- The ordered Binds API changed in exactly four classes:
  `UGameplayAbilityAttackLoop` added three methods;
  `UGameplayAbilityMagicBase` and `UGameplayAbilityCastSpell` each changed one
  method's constness; `UGameplayAbilityCastSpell` also removed
  `NeedUseManaToCast_Scriptable`; and `URagdollConfig` added the `FName`
  property `m_FollowBone`. Method counts moved 16,416 → 16,418 and class-property
  counts 14,966 → 14,967.
- Scalar default windows moved 26,399 → 26,389, fully explained by content:
  -5 Golem Stone Bridge `URagdollConfig`, -2 its `UWaterConfig`, -2 the hostile
  HumanOrc Training relationship modifier, and -5 with the removed
  `AIState_Warning_Human_Taunts` module; +1 each for the KalomDemon relationship
  modifier, ArenaDetection `RegionTrait`, ArenaFallingThroughLavaPit
  `RegionTrait`, and LordDemon Pyrolaser `ForceKnockback`. The arithmetic is
  -14 + 4 = -10, and every remaining field resolves.
- All six curated story modules reproduce their published source seals
  byte-for-byte. The cache still uses the `0x9e377abe` build identifier and its
  established wire format; no opcode-table change was observed.

The ordered API delta requires a new standalone-compiler profile. It is not by
itself evidence of AngelScript language or compiler-core semantic drift; the
frozen 27-case and full-tree embedded-versus-standalone comparisons remain the
separate compiler gate.

Mod Studio's Story/NPC/Quest authoring recognizes each exact registered triple
only within its existing project-only contract. Item authoring is separately
admitted by the row selected from the exact executable seal and its audited Item
field matrix; it does not use Shipping or Binds as Item evidence. Neither row
grants dialog-runtime, production-build, deployment, live-game, or DataAsset
qualification; every other consumer keeps its own independent gate.

Nothing produced a wrong answer in the historical fail-closed state, and the
admitted row does not widen its recorded proof. Anything that cannot prove the
evidence required by its own contract still refuses to act on it.

## The checklist

Run this when the game updates. Steps 1–4 are cheap and answer whether anything
is wrong at all; 5–10 are the independent generation, compiler, and live-use
qualification gates.

**1. Confirm what moved.** `gore story-catalog` refuses with all three identities
printed — byte length and SHA-256 for the executable, the shipping cache and the
binds cache. That refusal is the cheapest inventory there is; keep its output.

**2. Re-dump the USMAP.** Launch the game once with UE4SS, then compare the new
dump against the sealed one. If the filename's build number changed, the reflection
layout changed with it. Do not skip this because the old file still hashes to the
sealed value — it will, and it will be describing the previous build.

**3. Diff the two USMAPs before deciding anything.** Both dumps can sit side by
side, so this is a file comparison, not an investigation. What matters, in order:

- **`(class, direct parent)` edges that changed.** A reparented class is the
  difference between an additive patch and a re-audit.
- **Properties whose declaring owner moved between a class and its base.** This
  is the specific hazard the seals exist for: it makes ancestry resolve a field
  to the wrong owner, and every derived digest would be recomputed over that
  wrong graph and pass.
- **Classes added and removed.** Additive on its own is harmless.
- **Wire-shape changes** on properties that are still there (`Object` →
  `WeakObject`, `Int` → `Float`), and **case-only renames** — resolution is
  case-sensitive, so `VCA_MASTER` → `VCA_Master` silently resolves to nothing.

The 2026-07-31 update: 12 classes added, 4 removed, **0 reparented, 0 properties
moved**. That is what made it a transcription job rather than an audit.

The 2026-08-27/28 update kept all 6,602 class rows and the exact direct-parent
graph, with **0 reparented and 0 existing properties moved**. The fresh 172709
dump nevertheless matters: unlike the stale 171261 dump, it carries the new
`RagdollConfig.m_FollowBone` property that the matching Binds file exposes.

**4. Cross-check the dump against the build, without launching anything.** The new
`Binds.Cache` should name the classes and properties the new USMAP added and
none of those it removed. If the two disagree, one of them is from the wrong
build.

**5. Run `gore as qualify --game "$GAME"`.** It derives every value a generation
row needs, in-crate against the crate's own parsers, and prints the row plus a
qualification record naming the test behind each number.

It derives; it does not admit. The row it prints is a draft — putting it in the
binary is still a person's decision, which is the point.

It refuses rather than guessing, and each refusal is one of the traps above:

| refusal | what it caught |
|---|---|
| no dump can be tied to this executable | the USMAP problem — a dump is only accepted when the executable actually spells the class names it declares |
| two dumps fit equally well | a tie is not a coin flip |
| a count fell | a digest cannot tell you a parser dropped rows; a count can |
| a curated module stopped reproducing | content moved, not just identity — this is no longer a transcription job |
| the sealed ancestry does not qualify against the dump this run sealed | the seal would have been self-consistent and wrong |

Two things it will not do, deliberately. It never re-implements a parser to go
faster: a copy that is subtly wrong produces a seal that is perfectly
self-consistent and describes nothing, and every seal that digests *parsed
output* rather than raw bytes has that property. And it never writes into the
generation table.

**6. Read what it refused, if it refused.** A fallen count or a diverged module is
the finding, not an obstacle. Both mean the update changed something the previous
audit had checked, and the answer is to look at what changed rather than to pass
`--usmap` until the command stops complaining.

**7. Add the row and re-run the suite.** One struct literal, one qualification
file. The tests make the row re-derive its own published ids from its own
components, so a mistyped digest fails there rather than in the field.

**8. Requalify every standalone-compiler product target.** A central generation
row and a compiler profile answer different questions. The former admits
generation-bound authoring evidence; the latter captures the ordered
AngelScript API and frontend/compiler support. Run the frozen 27-case embedded
and standalone corpus for both required product targets (`24539464` and
`24878692`) against the exact same final signed sidecar. Promote and typed-reload
each profile independently. A Binds/API-only update normally requires new
profile data, not a speculative compiler-core change.

**9. Run and seal one full-tree differential per compiler profile.** Freeze the
complete source tree before the comparison. The verifier must produce both
outputs itself from one in-memory snapshot: exactly one guarded game-only run
with captured native diagnostics and exact restoration, followed by exactly one
strict standalone run with no installation mutation or fallback. Receipt v2
binds both execution records, its profile SHA-256, the common final sidecar and
protocol, that profile's Shipping and Binds seals, the source aggregate, and
both retained candidates. A copied or caller-selected reference cache is not
authority. The release verifier also refuses `GORE_AS_DIAGNOSTICS_HOOK`, so an
ambient development helper cannot stand in for the SHA-256-verified diagnostics
hook. Accept only exact WholeCache structural equality, zero semantic
diffs, and zero alignment loss; benign normalized byte differences stay
visible. The internal package recorder then requires exactly both product
targets and exactly one receipt per profile. Missing and additional targets or
receipts fail Record, Verify, and Stage. This is a package-completeness gate,
not a whole-binary runtime allowlist: installed builds still qualify by bounded
PE/cache structure and the ordered Binds API.

**10. Confirm every live claim separately on the running game.** A seal proves
the evidence is the evidence, never that the game agrees with it. Patch one
default you can see and look before claiming that live behavior. When the
admitted scope stops at bounded offline authoring, record that boundary instead
of implying this step happened: neither build `24340829` nor build `24878692`
gains dialog-runtime qualification from its central generation row.

## What is deliberately not automated

Re-sealing can be automated for everything whose meaning is checkable from files:
the binds digests, the identity triple, the curated record seals. It is not
automated for the native ancestry profile while the USMAP is a user-generated
artifact, because that is exactly where a green seal can describe the wrong build.

Per-record admission — invalidating only the binds records a patch actually
touched, instead of the whole file — was costed and rejected. On the 2026-07-31
update 69 of 69 pinned rows survived byte-identical and the class bridge moved by
3 rows out of 11,196, yet it would have recovered nothing: every consumer is
additionally gated on the script-cache GUID, which lives in a different file and
changes on every patch. Keying the gate on the parsed-map digest instead of the
file digest fails for the same reason — both maps changed too.

## Related

- [AngelScript internals](angelscript-internals.md) — what the sealed evidence
  admits and what the fail-closed transaction guarantees.
- [Offline AngelScript default patching](../guide/angelscript-defaults.md) — the
  commands this page is about.
