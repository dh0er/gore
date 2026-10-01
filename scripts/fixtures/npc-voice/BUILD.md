# Build and installation evidence — 2026-09-06

**Offline and installation checks passed.** The subsequent
[runtime campaign also passed](RESULTS.md).

Target: Gothic 1 Remake BuildID `24878692`. Manager entry
`npcvoiceproof-247d3668` is the sole enabled deployment and reported `in_sync`.
The previous `npcsessionproof-d331522e` library entry is retained but disabled.

- Eight new choices passed `dialog check` and strict standalone module compilation.
- The complete 7319-module graph passed strict standalone compilation.
- Its deployable mini contains A/B additions and Diego/XardasTower_AI edits.
- Bundle format 1: three components, ten files, 282763 bytes; exact footprints.
- Five Vorbis files fully decode; sample rates/channels match recordings.json.
- Installed A source matches the compiled cache exactly.
- All five installed audio payloads and six new text IDs match the build inputs.
- All 43898 original localization rows are unchanged.
- All 33323 original voice archive entries retain their CRC and uncompressed size;
  eight specifically referenced original recordings also retain their SHA-256.
- Pristine Shipping and voice archive backups were checked.
- The agent did not launch the game. The separate [runtime results](RESULTS.md)
  record the user's successful tests and verification of the resulting saves.

| Artifact | SHA-256 |
|---|---|
| `NpcVoiceProof.full.Cache` | `4e466b06cdc6a9f83c162c1768fdead155c8d1051398e4c50cba20c4dafb8353` |
| `NpcVoiceProof.mini.Cache` | `f1be8da336dc92413bd903464c56e5112f9072bddaca365b73d7ce4fe5ab07c9` |
| `generation-receipt.json` | `84528428a403864a0977ff242ee8c57395210918cc37e372790850b8f87f4609` |
| Bundle manifest | `57f2cc9fb81ca10f52e11632275a5dab1c8d28dd3958a7fbfd09812546c7c721` |
| Bundle tree | `7a40411b64d7afd2129afb0eeb6596983cd67230ee6e6b1152616f44f5ec7897` |
| Installed shipping | `ba47ff350233ac0f37a9d776ffe6e18e3c66378532163ba696eb268acdb4ee55` |
| Installed localization | `f3c6948bac217b69c457afeed4346bd99540ed47935d52d47db70efc268057d1` |
| Installed voice | `da44012393ca5dfaeb37ae37625c054a1e9b07d86aacf6e68344e3ff066339c9` |

Inspection and installation used the current workspace CLI. The installed
0.3.0 MCP inspector still assumes one module per script mini and rejected the
four-module bundle; the current CLI successfully validated that supported shape.
