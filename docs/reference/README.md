# GORE reference

Implementation contracts and invariants behind the commands: what a receipt
seals, why a patch is refused, what a native transaction binds, which failures
are terminal. These pages record how the tools behave and why — they are **not
instructions**, and reading them is never a prerequisite for modding.

If you want to *do* something, you want the [guide](../guide/README.md).

Reach for a page here when a command refuses something and the guide does not
explain it, when you need the exact meaning of a field in a receipt, or when you
are changing GORE itself and need to know what a boundary guarantees.

| Page | What it records |
|---|---|
| [Cooked DataAsset internals](dataassets-internals.md) | Selector seal semantics, receipts and source proofs, the no-clobber and power-loss boundary, the installed package browser, and the Mod Studio staging surface behind `gore asset`. |
| [AngelScript default-patching internals](angelscript-internals.md) | Which scalar sites are admitted and why, opcode windows, sealed evidence, fail-closed transaction semantics, and receipt fields behind `gore as default-sites` / `patch-default` / `tag-map-*`. |
| [Game updates](game-updates.md) | What a Steam patch invalidates, what keeps working, and the checklist for qualifying a new build. |
| [Dialog runtime internals](dialog-runtime.md) | The proven runtime boundary for compiled dialog topics, the requalification evidence, hook-order contract, and current limits. |
| [Mod Studio NPC and quest authoring internals](studio-authoring.md) | The offline logical-clone proof, the native archetype catalog, the revision-3 draft transaction, and the quest publication contract. |
| [Mod Studio voice authoring internals](studio-voice.md) | Installed target resolution, the all-or-nothing sealed build, and the publication and failure boundaries. |
| [Mod Studio project snapshot internals](studio-project-archive.md) | Snapshot V2 archive format, reachable closure, determinism, the import security model, wire limits, and stable failure codes. |

These pages stay in the repository. They are **not** shipped in the release zip
and are not rendered by `gore guide html` — but `gore.exe` does embed them, so
the [MCP server](../guide/mcp.md) can serve them as `gore://reference/<page>`
when an assistant needs to explain a refusal.
