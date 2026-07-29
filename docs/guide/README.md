# GORE guide

Everything you need to mod Gothic 1 Remake with GORE. Start with
[Getting started](getting-started.md).

## Basics

| Page | What it covers |
|---|---|
| [Getting started](getting-started.md) | Install the CLI, point it at the game, pick the right tool, first mod |
| [CLI reference](cli-reference.md) | Every command, subcommand, and flag |
| [Catalogs & data models](catalogs-and-models.md) | Regenerating catalogs, reflection models, and real in-game defaults |
| [Building](building.md) | Toolchain, `build.py`, repo layout, crates, versioning |

## Modding domains

| Page | What it covers |
|---|---|
| [Item & stat values](items.md) | `overrides.toml` → UE4SS Lua CDO override mod |
| [Text & dialogs](text-and-dialogs.md) | Decrypt, edit, and re-encrypt the localization `.lcache` |
| [Audio](audio.md) | Read and replace samples in the encrypted FMOD banks |
| [Voice-over](voice.md) | Index and copy-on-write edit the voice-over ZIP archives |
| [Textures](textures.md) | Replace IoStore textures via an additive Zen triplet |
| [Cooked DataAssets](dataassets.md) | Narrow, receipt-bound fixed-leaf editing of cooked packages |
| [Scripts (AngelScript)](scripts.md) | Decompile, recompile, and splice the precompiled script cache |

## Shipping and combining

| Page | What it covers |
|---|---|
| [Bundling & deploying](bundles.md) | One spec → one mod that deploys and undeploys as a unit |
| [Running many mods](mod-manager.md) | `gore mgr`: library, load order, conflicts, composed apply |

## AngelScript authoring

| Page | What it covers |
|---|---|
| [Dialog authoring](dialog-authoring.md) | Compiled topic template, runtime evidence, safe test order |
| [Offline default patching](angelscript-defaults.md) | `default-sites` / `patch-default` / `tag-map-sites` / `patch-tag-map` |
| [NPC authoring](studio-npc.md) | Linked class chains for a logical NPC identity |
| [Quest authoring](studio-quest.md) | Revision-3 quest drafts and their generator contract |

## Mod Studio workflows

| Page | What it covers |
|---|---|
| [Voice authoring](studio-voice.md) | Managed revision-3 Voice takes and the offline bundle |
| [Project export](studio-project-export.md) | Snapshot V2 backups of a managed project |
| [Project import](studio-project-import.md) | Inspecting and restoring a Snapshot V2 backup |

Engineering notes, product vision, and reverse-engineering plans live in
[`docs/internal/`](../internal).
