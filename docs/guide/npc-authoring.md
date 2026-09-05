# Characters (NPCs)

`gore npc` reads the game's characters: which ones exist, what one of them is
made of, and which world points spawn it. This release is the read surface.
Creating, editing, cloning or removing a character, and spawning one, are a
later slice and are not in this release — see
[What is proven, and what is not](#what-is-proven-and-what-is-not).

## The class chain

A character in Gothic 1 Remake is not a data record. It is a chain of
AngelScript classes, and each link names the next one through a class-scope
`default`:

```
USpawnAIAgentDefinition_<ID>          the spawn handle
  default AIAgentConfigClass    ->  UAIAgentConfig_Human_<ID>
                                      default m_CharacterDefinition -> UCharacterDefinition_Human_<ID>
                                        default m_UniqueName = n"<ID>"
                                        its SUPER CLASS is the faction (16 guild bases)
```

The faction is not a field. A character definition **derives** from one of the
16 guild base classes, so "which guild is this NPC in" is answered by its super
class, not by reading a value out of it.

The shipped cache declares 658 character definitions, 621 agent configs and
1053 spawn definitions, plus the surrounding parts a character refers to:
1923 daily routines, 1091 conversation settings and 846 visual definitions.
The NPC catalog compiled into `gore.exe` has 1095 rows.

## Placement lives in the level scripts

Placement is not part of that chain. A character appears in the world because a
**level script** — one world section's script module — calls `SpawnAIAgent`
from a world point:

```angelscript
class UWP_EZ_START_DIEGO_SPAWN : UWorldPointScript
{
    UFUNCTION()
    void OnWorldStart()
    {
        this.SpawnAIAgent(TSubclassOf<USpawnAIAgentDefinition>(USpawnAIAgentDefinition_OC_STT_Diego::StaticClass()), nullptr);
    }
}
```

There are 1764 such calls across 3939 `UWorldPointScript` classes. 14 of the
1764 pass the definition as a bare class reference, without the
`TSubclassOf<>(…::StaticClass())` wrapper — all of them creatures in the
Ancient Fortress and the Free Mine. `gore npc sites` reads both forms, so
neither shape is invisible.

Seventeen world sections carry characters. Pick the one you mean here, then
pass its module to `gore npc sites --level`:

| World section | Level script module | Characters |
|---|---|---|
| Old Camp | `LevelScripts.Map_x2_y1_OldCamp_AI_script` | 401 |
| Exchange Zone | `LevelScripts.Map_x2_y2_ExchangeZone_AI_script` | 242 |
| Ancient Fortress | `LevelScripts.Map_x1_y1_AncientFortress_AI_script` | 229 |
| New Camp | `LevelScripts.Map_x3_y2_NewCamp_AI_script` | 159 |
| Swamp Camp | `LevelScripts.Map_x0_y1_SwampCamp_AI_script` | 140 |
| Free Mine | `LevelScripts.Map_x3_y1_FreeMine_AI_script` | 139 |
| Monastery Ruins | `LevelScripts.Map_x1_y2_MonasteryRuins_AI_script` | 131 |
| Old Mine | `LevelScripts.Old_Mine_AI_script` | 114 |
| Sleeper Temple | `LevelScripts.SleeperTemple_AI_script` | 105 |
| Tundra | `LevelScripts.Tundra_AI` | 34 |
| Orc Graveyard | `LevelScripts.OrcGraveyard_AI` | 20 |
| Sleeper Dream | `LevelScripts.Map_SleeperDream_AI_script` | 20 |
| Shipwreck | `LevelScripts.ShipWreck_AI` | 13 |
| In Extremo (Old Camp) | `LevelScripts.Map_OldCamp_IE_script` | 7 |
| Stonehenge | `LevelScripts.StoneHenge_AI` | 5 |
| Sunken Tower | `LevelScripts.SunkenTower_OldCamp_AI_script` | 3 |
| Xardas' Tower | `LevelScripts.XardasTower_AI` | 2 |

`--level` matches on the module name, so a distinctive fragment such as
`XardasTower` or `OldCamp` is enough.

## `npc list` — the ids the game ships

```
$ gore npc list diego
human    OC_STT_Diego  CharacterDefinition_Human_OC_STT_Diego
human    OC_STT_Diego_Sleeper  CharacterDefinition_Human_OC_STT_Diego_Sleeper
2 of 2 shown
```

The filter is one positional word matched against the id and the class name.
`--category <human|creature|other>` narrows further, `--max <N>` caps the
printed rows (50 by default) while the final line still reports how many
matched, and `--json` returns the same rows as one document.

`list` answers from the catalog compiled into `gore.exe`. **It needs no game
installation**, which makes it the right first step when you only want the
exact id to hand to `show`.

## `npc show` — the whole chain and its spawn sites

```
$ gore npc show OC_STT_Diego
OC_STT_Diego
  spawn definition       USpawnAIAgentDefinition_OC_STT_Diego
  ai agent config        UAIAgentConfig_Human_OC_STT_Diego
  character definition   UCharacterDefinition_Human_OC_STT_Diego
  guild base             UCharacterDefinition_Human_OldCamp_Shadow
  unique name            OC_STT_Diego
spawns at 2 site(s):
  UWP_INTRO_FALL3  in LevelScripts.Map_x2_y2_ExchangeZone_AI_script
    translation: measured, no known difference
  UWP_EZ_START_DIEGO_SPAWN  in LevelScripts.Map_x2_y2_ExchangeZone_AI_script
    translation: measured, no known difference
```

`show` takes the **exact** id, not a filter; it resolves
`USpawnAIAgentDefinition_<ID>` and walks the chain from there. A link it cannot
resolve prints as `—` rather than being dropped, so a gap in the chain never
looks like a character without a guild.

Unlike `list`, `show` reads the installed script cache. `--cache <PATH>` reads
an exact cache instead, `--game <ROOT>` picks the install; without either, the
configured game path is used and then Steam auto-detect. `--json` returns the
chain, the sites and the translation judgement as one document.

`show` deliberately emits only the modules it needs — the 29 modules in the
`LevelScripts.` namespace plus the few that declare the classes in the chain.
That is not an optimisation detail you can ignore: one shipped module,
`Map.MainMap.WorldPointManagerConfig_MainMap`, needs over five minutes on its
own to recover its class defaults, against about two seconds for an ordinary
module. A command that emitted the whole tree would not answer.

## `npc sites` — where the level scripts spawn

```
$ gore npc sites --level XardasTower
UOW_XT_DEMON_LESSER_SPAWN_WP  USpawnAIAgentDefinition_XT_XardasDemon  LevelScripts.XardasTower_AI
UXT_Skeleton_SPAWN_WP  USpawnAIAgentDefinition_Skeleton_XardasServant  LevelScripts.XardasTower_AI
2 of 2 shown
```

Each row is the world point, the spawn definition it names, and the level
script it lives in. `--level <TEXT>` keeps the sites whose module contains that
text, `--npc <ID>` keeps only the sites that spawn one character, `--max <N>`
caps the printed rows, and `--json` returns them as one document. Like `show`,
it reads the installed cache and takes `--cache` / `--game`.

## Reading the `translation:` verdict

Every site `show` prints carries one line about its level script. It reports
what is known about **recompiling that module** — because changing where a
character stands means recompiling the level script that places it. There are
exactly three states, and they must not be conflated:

| Line | What it means |
|---|---|
| `translation: measured, no known difference` | Somebody measured this module on this game version, and its recompile produced no known difference. |
| `translation: N divergent function(s), M behaviour risk(s)` | Measured, and the recompile is known to differ. `N` functions came back different; `M` of those differences are behaviour risks. |
| `translation: NOT MEASURED for this game version` | **Nobody measured this module on this game build.** This is an absence of evidence, not a clean bill of health. Do not read it as reassurance. |

The judgement is keyed to the exact cache and Binds seals, so a game update
turns measured lines into `NOT MEASURED` rather than silently carrying an old
verdict forward. `--json` returns the same three states as
`{"measured": false}` or `{"measured": true, "divergent_functions": …,
"behaviour_risks": …}`, so a caller does not have to parse the sentence.

The same risk model backs `gore as emit` and `gore as compile-module`; the
module-level detail is in [Scripts (AngelScript)](scripts.md).

## What is proven, and what is not

Read this before you build on it.

| | |
|---|---|
| **The three read commands** | Everything `list`, `show` and `sites` report is proven and produced entirely offline. `list` needs no installation at all; `show` and `sites` read a script cache and launch nothing. |
| **Recompiling a level script does not disturb its neighbours** | On the measured game build, BuildID `24878692`, the complete script tree emitted and recompiled unchanged produces a **byte-identical** cache: SHA-256 `7A18F954E32AF30FC24AE3A66EA35D3B5CB98560C8F5083C7846FC9CE1D77511`, 124,459,412 bytes, 7317 modules. On that build, recompiling a level script therefore cannot change code the author did not touch. |
| **Per-module translation** | Whether one particular level script survives its own recompile is the separate, per-module judgement the `translation:` line reports. Byte-identity of the whole tree does not answer it for a build nobody measured. |

### Not in this release

None of the following works today. They are a later slice, and nothing on this
page should be read as a hint that they already work:

- Creating a character.
- Editing an existing character — its chain, its guild, its routine, its
  conversation settings or its visuals.
- Cloning a character.
- Removing a character.
- Spawning one: adding, moving or deleting a `SpawnAIAgent` call in a level
  script.

To change something about an NPC today, use the surfaces that already have an
authoring path: [Dialog authoring](dialog-authoring.md) for what it says,
[Offline default patching](angelscript-defaults.md) for a single class default,
and [Scripts (AngelScript)](scripts.md) for the general emit/recompile/splice
route with its own risk reporting.

## Related

- [Reading and editing dialog trees](dialog-trees.md) — what a character says
- [Dialog authoring](dialog-authoring.md) — authoring that conversation
- [Scripts (AngelScript)](scripts.md) — emit, recompile and splice a module,
  and the risk report behind the `translation:` line
- [Finding things](find.md) — `gore find --domain npc` searches the same
  catalog `npc list` reads, alongside every other id namespace
- [Catalogs & data models](catalogs-and-models.md) — regenerating that catalog
