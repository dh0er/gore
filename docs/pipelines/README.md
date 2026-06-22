# Catalog pipeline

Catalog generation is handled by the `gore` CLI (`gore catalog`), backed by the
[`gore-catalog`](../../crates/gore-catalog) crate. The Python scripts that
previously lived here (`build_item_catalog.py`, `build_npc_catalog.py`,
`build_knowledge_catalog.py`) have been replaced by Rust in
`gore_catalog::pipeline`.

## Regenerating catalogs

```sh
DUMP="D:/SteamLibrary/steamapps/common/Gothic 1 Remake/G1R/Binaries/Win64/ue4ss/UE4SS_ObjectDump.txt"
ASSETS="apps/save-editor/assets"

gore catalog --kind item      "$DUMP" -o "$ASSETS/item_catalog.json"
gore catalog --kind npc       "$DUMP" -o "$ASSETS/npc_catalog.json"
gore catalog --kind knowledge "$DUMP" -o "$ASSETS/knowledge_catalog.json"
```

Output is byte-identical to the previous Python pipeline output.
