# Survival mode

Gothic 1 Remake ships a complete hunger / thirst / fatigue system that no player
can reach. This page records what it is, why it does not run, exactly what was
measured, and every value the save editor therefore hides — so that if a patch
ever switches it on, turning the editor back on is a small, informed change
rather than a fresh investigation.

Everything below was established offline from the AngelScript cache and the game
localization, then confirmed in the running game with a UE4SS probe on
2026-08-13 against Steam build changelist 171864.

## What it is

The game's own localization describes it:

| Key | English |
|---|---|
| `ui_difficulty_survivaltooltip` | Adds hunger, thirst and fatigue as additional survival mechanics. |
| `ui_profile_survival` | Survival |

There is no `ui_difficulty_survivallabel`, and no localized name or icon exists
for any of the three needs anywhere in the 43,851-key catalog or in the IoStore
texture index. The player-facing surface was never finished.

## Why it does not run

The system is built and wired, right up to the last step:

- **The abilities are granted.** `UCommonPlayerDefinition::__InitDefaults` sets
  `m_AbilitiesEffect = UGE_Player_Definition`, and that effect's
  `__InitDefaults` makes 88 `AddAbility` calls — entries 76–81 are
  `UGA_FatigueHourlyEffect`, `UGA_FatigueDebuffs`, `UGA_ThirstDebuffs`,
  `UGA_ThirstHourlyEffect`, `UGA_HungerDebuffs`, `UGA_HungerHourlyEffect`. The
  grant parameters are byte-identical to those of `UGA_Swim_Human`, which
  demonstrably runs.
- **The attribute sets are seeded.** The same function sets
  `Max{Hunger,Thirst,Fatigue}` = 1000, `FillRatio` = 12, `FillRatioPeriod` = 1,
  `MaxThresholdIndex` = 4 (Fatigue additionally
  `RecoveryRatePerHourOfSleep` = −0.125).
- **The abilities are fully configured**, including the per-threshold effect
  lists.
- **The UI row is deliberately hidden.** Of 7,308 AngelScript modules, exactly
  one mentions survival: `UI/UISettingsConfiguration.as`, which declares
  `USettingObject_Bool_SurvivalSettings_AS` whose entire generated
  `__InitDefaults` writes `false` into the inherited native `bool m_IsShown`.
- **The gate is native.** `UDifficultyManagerSubsystem::GetSurvivalModeState()`
  and `SetSurvivalModeState(bool)` are native, and no AngelScript module ever
  calls either. The ability base classes
  `UGameplayAbilityNeedHourlyEffectBase` and `UGameplayAbilityNeedDebuffsBase`
  are native-only; no `.as` file defines a body.

### What was measured

A UE4SS Lua probe forced the runtime state true **before** the hero existed, then
read the hero's attributes for a minute:

```
set survival (tick): false -> true          <- 21:08:49, still at the main menu
=== hero found (save loaded) ===            <- 21:09:03
on-load: survival=true  pc=y pawn=y state=y asc=y
  Hunger  = 900.0 / 1000.0
  Thirst  = 0.0 / 1000.0
  Fatigue = 0.0 / 1000.0
  Strength=30.0 MaxMana=35.0 Speed=4.0 Health=71.0
watch:  (unchanged after 30 s and after 60 s)
```

`Hunger` at 900/1000 is threshold stage 4, which owes −15 % Strength and 1 HP per
second. Strength stayed at 30.0 and Health stayed at 71.0. **The abilities never
activate**, even with the state true, the abilities granted and the attribute
sets present.

The most likely reason is authored, not gated: `m_ActivationSkillTag` (on the
debuff bases) and `m_HourlyAbilityTag` (the only member the hourly base adds over
the plain passive hourly base) are **never assigned by anything in the entire
cache**. If the native activation path consults either, an empty tag would keep
the system silent forever. That cannot be distinguished from a deliberate gate
without reversing `G1R-Win64-Shipping.exe`.

### The flag lives in three places, none of which helps

| Location | Note |
|---|---|
| `PersistentDataList.sav` → `m_Profiles[i].m_Survival` | The authoritative profile copy. Setting it survives a game run — the game rewrites the file and keeps the value — but it is **not** plumbed into the subsystem: `GetSurvivalModeState()` still read `false`. |
| `<slot>.sav` → `m_Profile.m_Survival` | A write-only snapshot taken at save time, demonstrably stale (it lists deleted slots). Editable, but inert. |
| `<slot>.sav` header → `FSaveDataPayload::m_SurvivalMode` | Uncompressed header field, around byte 1190. Not touched by the editor. |

**Danger:** `m_PermanentDeath` sits immediately beside `m_Survival` in
`FProfileData` (0x60 vs 0x61) and in the save header between
`m_PermaDeathGameOver` and `m_FakeSloppyCombos`. An off-by-one offset patch turns
on permadeath, which the game states cannot be reversed. Address these by name,
never by a hardcoded offset.

Survival is a per-profile setting, and in this game a new game always means a new
profile — so there is no "start a fresh run with the flag already set" path
either: the profile is created by the game with the flag false, and no UI can
change it.

## The mechanics, for when it does get switched on

Each need runs two passive abilities: an *HourlyEffect* that adds `FillRatio`
points every `FillRatioPeriod` in-game hours, and a *Debuffs* ability that maps
the fill level onto a threshold index `0..MaxThresholdIndex` and applies that
stage's effects. Stage **0 is a bonus**, stage 1 is deliberately empty (no key 1
in any map), stages 2–4 are penalties.

| Stage | Hunger | Thirst | Fatigue |
|---|---|---|---|
| 0 | +5 % Toughness | +5 % MaxMana | +5 % SpeedModifier |
| 2 | −5 % Toughness, −5 % Strength | −10 % MaxMana | −5 % MaxMana, MaxHealth, Toughness |
| 3 | −10 % both | −15 % MaxMana | −10 % MaxMana/MaxHealth, −5 % Toughness, −5 % Speed |
| 4 | −15 % both | −20 % MaxMana | −15 % MaxMana/MaxHealth, −10 % Toughness, −5 % Speed |

Stage 4 additionally applies `UGE_<Need>Debuff_Threshold_4_PassiveHealthDecrease`
— an infinite effect with period 1.0 s draining 1 Health — **per need**, so all
three at stage 4 is 3 HP/s. Thirst also drains mana periodically: 1 per 9 s at
stage 2, per 6 s at 3, per 3 s at 4.

Each stage's effects carry a per-need tag (`Debuff_Hunger` / `Debuff_Thirst` /
`Debuff_Fatigue`) which is also the ability's `m_DebuffTagToClear`; that is how
moving between stages removes the previous stage. A separate
`UGE_<Need>RefreshAttributes` re-clamps Mana and Health after the Max-percentages
change.

**Shipped data bug:** the Thirst threshold map is mis-wired. Key 3 carries both
`Threshold_2_PassiveManaDecrease` and `Threshold_3_PassiveManaDecrease` (two
stacking drains, ≈0.28 mana/s) while key 2 gets no mana drain at all. Hunger and
Fatigue are clean, so this is a copy-paste slip, not a design choice.

**Clearing a need:** Hunger and Thirst only go down through consumables
(`UGE_Item_ReduceHunger_Insta` / `UGE_Item_ReduceThirst_Insta`, SetByCaller
magnitude supplied by the item) — sleeping does nothing for them, as neither
attribute set has a sleep-recovery member. Fatigue only goes down through sleep,
at 12.5 % of `MaxFatigue` per hour, so eight hours resets 1000 to 0.

**Timing**, assuming equal bands (the fill-level → index mapping is native and
unread): 12 points per in-game hour against 1000 means the opening bonus expires
around 16.7 in-game hours, the first penalty lands near 33 h, and the health
drain near 67 h.

## What the save editor hides, and why

All sixteen values below are removed from the curated attribute view by
`_heroUnusedAttributeIds` in
`apps/save-editor/lib/features/editor/domain/hero_attributes.dart`. They remain
editable in the All-data property browser — the editor hides what cannot work,
it does not refuse access to the bytes.

| Attribute set | Value | Meaning |
|---|---|---|
| `AttributeSet_Hunger` | `Hunger` | Current hunger, 0…1000. |
| | `MaxHunger` | Cap, 1000. |
| | `FillRatio` | Points gained per tick, 12. |
| | `FillRatioPeriod` | In-game hours per tick, 1. |
| | `MaxThresholdIndex` | Number of penalty stages, 4. |
| `AttributeSet_Thirst` | `Thirst`, `MaxThirst`, `FillRatio`, `FillRatioPeriod`, `MaxThresholdIndex` | Same shape, same values. |
| `AttributeSet_Fatigue` | `Fatigue`, `MaxFatigue`, `FillRatio`, `FillRatioPeriod`, `MaxThresholdIndex` | Same shape, same values. |
| | `RecoveryRatePerHourOfSleep` | −0.125: fraction of max removed per hour slept. |

Two subtleties the code has to respect:

- `FillRatio`, `FillRatioPeriod` and `MaxThresholdIndex` exist **only** in these
  three sets, so hiding them by bare id is exact.
- `RecoveryRatePerHourOfSleep` also exists on `AttributeSet_Health` and
  `AttributeSet_Mana`, where it is real and shown under *Sleep & rest*. It is
  therefore hidden by its **set-qualified** key `Fatigue_RecoveryRatePerHourOfSleep`
  (see `heroAttributeKey` / `heroAttributeHidden`), never by id.

Not hidden, despite sitting next to this system:

- `SpeedModifier` (`AttributeSet_Movement`) — the fatigue debuff is its only
  in-play writer, but the value itself is live: a hand-set 4.0 was confirmed in
  game to survive save/load and to actually move the hero faster. It stays, under
  *Combat & movement*.
- `Toughness` and `ToughnessA/B/C` are hidden too, but for a different reason —
  encumbrance was cut. See the comment on `_heroUnusedAttributeIds`.

## If a patch ever ships it

1. Verify with a probe before changing anything: force
   `SetSurvivalModeState(true)`, set `Hunger` to 900 on a copied save, load, and
   watch Strength and Health. Stage 4 is unmistakable — that is exactly the test
   recorded above, and it is cheap to repeat.
2. If the abilities activate, delete the sixteen entries from
   `_heroUnusedAttributeIds` and restore the `survival` group: a
   `HeroAttributeGroup.survival` member, its ordered list, a `_SidebarEntry`,
   the `_entryToGroup` / `_entryLabel` / `_entryIcon` arms in both
   `hero_stats_card.dart` and `npc_attributes_panel.dart`, and a
   `heroGroupSurvival` message in all twelve `.arb` files. The group machinery
   already tolerates absent attributes — a sidebar entry only appears when its
   group has rows, so a hero without the Hunger/Thirst sets simply will not see
   it.
3. Note that `AttributeSet_Hunger` and `AttributeSet_Thirst` are absent from
   saves made before build changelist 171261 and present from that build on,
   independent of the flag. `AttributeSet_Fatigue` is present in all of them.
