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

## Versions (pinned)
- **UE engine version: 5.4** — CONFIRMED. Source: `G1R/Binaries/Win64/ue4ss/UE4SS.log`
  prints `Found EngineVersion: 5.4` / `Using engine version: 5.4` at launch. The
  shipping exe's own ProductVersion/FileVersion fields are blank, and `strings`
  finds no `UE5`/`Release-5.x` tag (mangled, as expected) — DEAD-END for both of
  those methods; UE4SS.log is the authoritative source.
- **AngelScript core ANGELSCRIPT_VERSION: 23300 ("2.33.0 WIP")** — from the
  public Hazelight-derived mirror `WillGordon9999/UNREANGEL`
  (`Angelscript/Source/AngelscriptCode/Public/angelscript.h`:
  `#define ANGELSCRIPT_VERSION 23300`, `ANGELSCRIPT_VERSION_STRING "2.33.0 WIP"`).
  UNREANGEL's README states it targets "Unreal Engine 5.4.x", matching the game.
  The Hazelight `UnrealEngine-Angelscript` engine repo itself is gated (Epic
  org-linked, returns 404 unauthenticated), so the version is pinned via the 5.4
  mirror rather than the engine repo directly — treat as HYPOTHESIS pending a
  direct read of the exact Hazelight commit, but high-confidence (UNREANGEL is a
  near-verbatim plugin-ization of the Hazelight AngelscriptCode tree).

## Container / magic 0x9e377abe (analysis)
The cache is produced by `FAngelscriptPrecompiledData::Save` (UNREANGEL
`Private/AngelscriptManager.cpp` writes `PrecompiledScript_Shipping.Cache`;
`Private/StaticJIT/PrecompiledData.cpp` does `Writer << *this`). The top-level
`operator<<(FArchive&, FAngelscriptPrecompiledData&)` serializes, in order:
`DataGuid` (FGuid, 16 bytes), `BuildIdentifier` (int32), then `Modules` (TMap),
`TypeReferences`, ... `StaticNames`, etc.

Mapping onto the real file:
- `0x00..0x10` (16 B) = **`FGuid DataGuid`** = `d54f0ffb 10c1054b 99f11446 a43ed5dc`
  (four LE u32 words). This is a **random per-build GUID** (`FGuid::NewGuid()` in
  the ctor), i.e. a build-identity tag, **NOT a content hash**. This resolves the
  earlier OPEN "hash scheme" question: it is an identity GUID, not a checksum.
- `0x10` (u32) = `0x9e377abe`, `0x14` (u32) = `7264`. In the UNREANGEL operator
  these positions would be `BuildIdentifier` (=4 for shipping) + start of the
  `Modules` map count — which does NOT match (we see 0x9e377abe, not 4). So the
  shipped Hazelight build's header differs slightly from the UNREANGEL mirror at
  this offset: `0x9e377abe` is most plausibly a **format/version magic** the
  shipped serializer writes after the GUID, followed by a top-level entry
  `count = 7264` (≈ the 7264 type/entry count from the header).
- The literal `0x9e377abe` was **NOT found anywhere in the UNREANGEL source**
  (no code match in PrecompiledData.* or AngelscriptManager.cpp) — DEAD-END for a
  source-level constant. Pinning its exact meaning needs the precise shipped
  Hazelight commit (gated). Recorded as HYPOTHESIS: container/precompiled-data
  format magic distinct from the AngelScript bytecode version (23300).
