# Test fixtures

`cache_head_8k.bin` — first 8192 bytes of `PrecompiledScript_Shipping.Cache`
(Gothic 1 Remake, Steam appid 1297900). A header-only slice used for hermetic
decode tests. Not the full script payload. Regenerate with:

    head -c 8192 "<game>/G1R/Script/PrecompiledScript_Shipping.Cache" > cache_head_8k.bin
