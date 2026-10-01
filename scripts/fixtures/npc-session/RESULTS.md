# NPC session results — 2026-09-06

**Passed on Gothic 1 Remake BuildID `24878692`.** Checkout health, authored NPC
save/load across sessions, and an invented identity's quest, knowledge and
relationship are qualified for this fixture.

## Evidence sources

The user performed the game steps and reported: “alles erledigt. nicht ganz in
deiner reihenfolge, aber alles sauber. alles funktioniert perfekt.” This is the
confirmation of the requested campaign, including full process restarts; its
order differed from the suggested sequence. Six supplied screenshots show the
active/completed journal and named saves. Save names below come from
`inspect_save` public metadata, not filename ordering.

The read-only verifier independently checks serialized state and save integrity.
Its raw `runtimeRestartProven` fields remain `false`: bytes alone cannot establish
a process restart. Runtime qualification combines those checks with the user's
observations. The agent did not launch or operate the game.

## Save results

| Save | Player save name | Phase | Exact quest state |
|---|---|---|---|
| `G1R-023.sav` | quest gestartet | running | `EQuestState::Running` |
| `G1R-026.sav` | ich bin wieder da - noch nicht angeklickt | running | `EQuestState::Running` |
| `G1R-025.sav` | Quest beendet | completed | `EQuestState::Succeeded` |
| `G1R-027.sav` | gut dass wir freunde geblieben sind angeklickt | completed | `EQuestState::Succeeded` |

All four reports passed with zero errors. The reload comparisons are
`023 → 026` (active) and `025 → 027` (completed), despite their creation order.

- A: `GORE_TEST_A-WP_WarningFlock_XT_01`; B: `GORE_TEST_B-WP_WarningFlock_XT_02`.
  Exact identities, inventories and attributes match across both reload pairs.
- Quest class: `/Script/Angelscript.Quest_GORE_NPC_SESSION`.
- A owns `gore_npc_session_started` in all four saves and
  `gore_npc_session_completed` only in the two completed saves. Neither B, Hero
  nor Diego owns either fixture marker.
- A retains personal relationship `Friend` and exactly one
  `/Script/Angelscript.ActivePersonalRelationshipModifier_Story`, targeting
  `Hero` with `ERelationship::Friend`. Effects are written by selected choices;
  startup does not recreate them.
- Checkout save `G1R-002.sav` and its reload `G1R-028.sav` (`gespeichert`, profile
  3) both contain Health and MaxHealth **base/current 1234** for
  `OC_STT_Diego-WP_EZ_START_DIEGO_SPAWN`.
- All inspected saves were unchanged by the reads; each reload has distinct bytes.

## Reproducibility

The [fixture instructions](README.md), [quest](quest.as), [choices](choices.as)
and [read-only verifier](../../npc_session_proof.py) describe the campaign.
Raw reports and user saves remain local; fingerprints identify the inspected
files without including save contents in the repository.

| Save | SHA-256 |
|---|---|
| `G1R-002.sav` | `2fc4a9104d839443b8c58c350f7afa6061e3c1a17ac64542229e907e9977e32c` |
| `G1R-023.sav` | `b9f78d4ca0671f85ce48cbea47571031f0751113e96b782daf7a624571c7ce2c` |
| `G1R-025.sav` | `858a986793f3fc4e4f1673e01aa4ac485d1f9738df853fbdb08a64942b8f6b3d` |
| `G1R-026.sav` | `fca7ea12e05c6915d789a258e4611f0dc177499f731498ed9dc8e11d8b355ade` |
| `G1R-027.sav` | `9ec1df3bd9c08555b2b8ab8a17ff1278c67e9cc35b9a1d1f0f31055e7194e703` |
| `G1R-028.sav` | `6b0b4a8325fa878e2d9195eedfe5d18e013c4db832a91a5e1e6b586853b72ee5` |

| Local report | SHA-256 |
|---|---|
| `023-proof.json` | `e89cac2b50f3cde2fbb7846adc5114d011bc9bb627edc4dfc28918ff4c3009a6` |
| `025-proof.json` | `a726607372a7858bc7575b623282ab2e0dcc8d6051a8ac5543cd76bfa0b25040` |
| `026-proof.json` | `ebb539ffa776f30488114fd2ea92c64a9f18c754c9f255980cdaaa9e31bb9086` |
| `027-proof.json` | `78d4adfee88611e88de2a25ac4019b97e862693cef7a350d7ea882d7f37b46ad` |
| `checkout-reload-proof.json` | `1e41771d38f4733b6782464a7a7171eae46542eaab8c2875cd5686f37ce5058f` |

| Artifact | SHA-256 |
|---|---|
| `NpcSessionProof.full.Cache` | `bcd89b2aa7637324c1d5b18bcd44a8159b5aafef58cbb2f48cb432a487767993` |
| `NpcSessionProof.mini.Cache` | `ecdb39f28a65af216e9990a641ee8bc3a975bf9fb3636f5bb04d95d4bcf92ff8` |
| `generation-receipt.json` | `c8398f2d3de61fc86d76a83b5d27c45c798e14f96b4827f8f3b3e41daa10d832` |
| Installed Shipping cache | `87e3b4602b29c281289db6dec086f7ce03864d3e7888712dc33fe6e10abcde75` |
| Pristine Shipping backup | `7a18f954e32af30fc24ae3a66ea35d3b5cb98560c8f5083c7846fc9ce1d77511` |
| Save reader DLL | `50475b8970d3c081a5edfff11c0d593773f682d94dea03720eb67d72f7ba035d` |

The full graph passed strict standalone compilation. The bundle contains four
planned modules: additions for A/B and edits for Diego/XardasTower_AI. Manager
entry `npcsessionproof-d331522e` was applied and `in_sync`; installed A source
matched the compiled source, A/B spawned once, and existing spawns were retained.

Offline checks already passed: 91 dialog unit tests, 130 symbol-remap integration
tests, 17 quest-generator tests and 13 save-verifier tests (**251 total**).

## Screenshot record and scope

| Attachment filename | Observation | SHA-256 |
|---|---|---|
| `codex-clipboard-23be741f-2436-402b-9292-cd7b85e245c8.png` | Active quest title and instruction | `0f247b668b8085f224c890c2fb4fb16757cf05b04df22f11c8f163dfd7e66b3c` |
| `codex-clipboard-131d8dc5-a9a3-464f-9093-d3091a8a4ffc.png` | Completed quest detail | `aa2ac6af852d2eaf89bd9bae970317e341c2834073c63b5cf4107e1f7d670e79` |
| `codex-clipboard-232b5e42-d6c2-46cd-bdbf-2f58335d07cb.png` | Completed list, green check and `<GORE_TEST_A>` giver | `0ba84d0712ec4cf3c12cfb64ca2a5e86460497f2163dbcd877f71a50da9ad458` |
| `codex-clipboard-2e859a7c-5f29-4d5d-93ec-820bbe17ecef.png` | Active list and `<GORE_TEST_A>` giver | `6602f5b7f1e1329f09f40f0ecbc477cc35bcbaae15043db213429db153439c18` |
| `codex-clipboard-b1ac2d76-5018-4bbc-a094-ec4491e1319e.png` | Four named campaign saves | `b653441b53ff3744408294d2e4f6afe5a40b70693e25e00dfca13163e42acd5e` |
| `codex-clipboard-e13063b4-de91-4426-808c-463f45a748e0.png` | Profile 3 manual save gespeichert | `2e50da995811adb1c205176804db13964e8b2f6cba31c0e625b91fbf5af79170` |

The journal title is **Erinnere dich an mich**. This fixture retains the giver's
`<GORE_TEST_A>` identifier and authors no dedicated questlog document. The
completed detail screenshot also contains unrelated vanilla letter text below
the objective; these results qualify quest state and persistence, not finished
questlog presentation. No new claim is made about modular appearance matching
its template, daily-routine movement, identifiable template voice or other builds.
