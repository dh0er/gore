# gore-core pipeline

Catalog generation is handled by `gore-cli`. The Python scripts that previously
lived here (`build_item_catalog.py`, `build_npc_catalog.py`,
`build_knowledge_catalog.py`) have been replaced by Rust in
`gore_core::catalog::pipeline`.

## Regenerating catalogs

```sh
DUMP="D:/SteamLibrary/steamapps/common/Gothic 1 Remake/G1R/Binaries/Win64/ue4ss/UE4SS_ObjectDump.txt"
ASSETS="projects/gore-save/app/assets"

gore-cli catalog --kind item      "$DUMP" -o "$ASSETS/item_catalog.json"
gore-cli catalog --kind npc       "$DUMP" -o "$ASSETS/npc_catalog.json"
gore-cli catalog --kind knowledge "$DUMP" -o "$ASSETS/knowledge_catalog.json"
```

Output is byte-identical to the previous Python pipeline output.
