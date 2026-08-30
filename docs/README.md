# GORE documentation

| | |
|---|---|
| **[Guide](guide/README.md)** | How to mod Gothic 1 Remake with GORE. Start here. |
| **[Reference](reference/README.md)** | The contracts and invariants behind those commands — why a command refuses something, what a receipt seals. Not instructions. |
| **[Development](development.md)** | Building GORE itself: toolchain, `build.py`, repo layout, crates, releases. |

New here? Start with [Getting started](guide/getting-started.md).

Only the guide ships in the CLI release zip, as Markdown plus a browsable
`docs\guide.html`. `gore.exe` embeds the guide *and* the reference so the
[MCP server](guide/mcp.md) can serve both to an AI assistant.

## Guide at a glance

- [Getting started](guide/getting-started.md) · [CLI reference](guide/cli-reference.md) · [MCP server](guide/mcp.md) · [Finding things](guide/find.md)
- Domains: [items](guide/items.md) · [text & dialogs](guide/text-and-dialogs.md) · [audio](guide/audio.md) · [voice](guide/voice.md) · [textures](guide/textures.md) · [DataAssets](guide/dataassets.md) · [scripts](guide/scripts.md)
- AngelScript authoring: [dialogs](guide/dialog-authoring.md) · [defaults](guide/angelscript-defaults.md)
- Shipping: [bundles](guide/bundles.md) · [many mods](guide/mod-manager.md)
- Also: [Mod Studio](guide/mod-studio.md) · [catalogs & data models](guide/catalogs-and-models.md)

## Apps

- [Save Editor](../apps/save-editor/README.md)
- [Mod Studio](../apps/mod-studio/README.md)
- [Mod Manager](../apps/mod-manager/README.md)
- [gore-lua helper library](../lua/README.md)
