# PrecompiledScript_Shipping.Cache — format notes

Source: Gothic 1 Remake (Steam appid 1297900), Hazelight UnrealEngine-Angelscript fork.
Build root in shipped exe strings: `D:\P4J\Gothic1Remake\G1R\Plugins\Angelscript\...`.

## Outer header (confirmed)
| Offset | Type      | Value (sample)                      | Meaning            |
|--------|-----------|-------------------------------------|--------------------|
| 0x00   | u8[16]    | d54f0ffb10c1054b99f11446a43ed5dc    | validation hash    |
| 0x10   | u32 LE    | 0x9e377abe                          | magic              |
| 0x14   | u32 LE    | 7264                                | type/entry count   |
| 0x18   | records   | —                                   | per-type records   |

## Per-type record (PARTIAL — to resolve in follow-up)
Observed at 0x18 (real `walk` output over the full 122 MB cache):

    0x00000018  len=17   AI.AIItemScoring          (17 = name + trailing NUL)
    0x0000002d  len=16   AI.AIItemScoring          (16 = same name, no NUL)
    0x00000046  len=31   UGothicAIItemActionScoringEntry
    0x00000106  len=38   UGothicAIItemActionScoringEntryManager
    0x000001cd  len=51   UAIItemScoringEntry_MeleeWeapon_BasePriorityBySkill
    ... (long run of UAIItemScoringEntry_* native class names) ...
    0x00002457  len=9    ItemClass
    0x000025a8  len=26   CalculateScoreOfItemAction
    0x00002663  len=2    AI
    0x0000266a  len=16   ScoredItemAction
    0x0000267f  len=12   RequiredTags
    0x0000272e  len=11   StaticClass
    0x0000273e  len=31   UGothicAIItemActionScoringEntry
    0x00002806  len=11   StaticClass
    0x00002816  len=38   UGothicAIItemActionScoringEntryManager

Notes from this run:
- The first record name at 0x18 appears twice: once length-prefixed with a
  trailing NUL (len 17 = "AI.AIItemScoring\0"), then again at 0x2d as len 16
  with no NUL. So both NUL-terminated and bare-length encodings co-exist in the
  stream; the scanner strips the trailing NUL either way.
- After the type/class name table, member-level identifiers appear
  (`CalculateScoreOfItemAction`, `ScoredItemAction`, `RequiredTags`,
  `StaticClass`, `ItemClass`), interleaved with `StaticClass` markers preceding
  each registered native class — consistent with reflected-class metadata.
- Namespaced AngelScript names use a `.` separator (`AI.AIItemScoring`, bare
  `AI`); native UE classes use the `U`/`F`-style prefix convention.

Open: exact record field order, what the size fields bound, where per-module
AngelScript bytecode begins. (Resolved by the container-parse follow-up plan.)

## Sibling file
`Binds.Cache` (~5.9 MB) — native binding data; likely needed for full type resolution.
