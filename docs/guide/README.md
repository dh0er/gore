# GORE guide

Everything you need to mod Gothic 1 Remake with GORE. Start with
[Getting started](getting-started.md).

## Basics

| Page | What it covers |
|---|---|
| [Getting started](getting-started.md) | Install the CLI, point it at the game, pick the right tool, first mod |
| [CLI reference](cli-reference.md) | Every command, subcommand, and flag |
| [Finding things](find.md) | `gore find`: class names, asset paths, and what an id does in game |
| [Reading and editing dialog trees](dialog-trees.md) | `gore dialog`: inspect conversations; edit behavior/defaults; stage topics, complete conversations and all-new trees |
| [MCP server](mcp.md) | Drive the whole CLI from an AI assistant over the Model Context Protocol |

## Modding domains

| Page | What it covers |
|---|---|
| [Item & stat values](items.md) | `overrides.toml` → UE4SS Lua CDO override mod |
| [Characters](npc-authoring.md) | `gore npc`: which characters exist, the class chain one is made of, and which world points spawn it |
| [Text & dialogs](text-and-dialogs.md) | Decrypt, edit, and re-encrypt the localization `.lcache` |
| [Audio](audio.md) | Read and replace samples in the encrypted FMOD banks |
| [Voice-over](voice.md) | Index and copy-on-write edit the voice-over ZIP archives |
| [Textures](textures.md) | Replace IoStore textures via an additive Zen triplet |
| [Cooked DataAssets](dataassets.md) | Extract, patch and pack one cooked package |
| [Scripts (AngelScript)](scripts.md) | Decompile, recompile, and splice the precompiled script cache |

## AngelScript authoring

| Page | What it covers |
|---|---|
| [Dialog authoring](dialog-authoring.md) | Same-module topics and complete conversations, localization, strict compile, packaging, deployment and runtime proof boundaries |
| [Offline default patching](angelscript-defaults.md) | `default-sites` / `patch-default` / `tag-map-sites` / `patch-tag-map` |

## Shipping and combining

| Page | What it covers |
|---|---|
| [Bundling & deploying](bundles.md) | One spec → one mod that deploys and undeploys as a unit |
| [Running many mods](mod-manager.md) | `gore mgr`: library, load order, conflict evidence, preflight/recovery, Apply and Reset |

## Also

| Page | What it covers |
|---|---|
| [Mod Studio](mod-studio.md) | The no-code GUI: NPCs, quests, voice takes, project backups |
| [Catalogs & data models](catalogs-and-models.md) | Regenerating the catalogs and reflection models the tools ship with |

---

Two things deliberately live outside this guide. The contracts behind these
commands — what a receipt seals, why a patch is refused — are in
[`docs/reference/`](../reference/README.md). Building GORE itself is in
[Development](../development.md).
