# Plan: FMOD Audio Modding in gore (`gore audio` + mod-studio)

Ziel: Sounds/Musik aus Gothic 1 Remake **entpacken** und **wieder verpacken**, sodass
Nutzer einzelne Sounds/Tracks austauschen können. Über `gore` CLI und mod-studio GUI.

Randbedingungen:
- **Keine Third-Party-Projekte als Dependency.** (FenixProFmod/IZH318/Fmod-Bank-Tools
  sind nur GUIs um FMODs offiziellen `fsbank` — wir bauen den Ansatz selbst nach.)
- **FMOD so weit wie möglich vermeiden.** Nutzer-FMOD-Install ist akzeptabel, aber nur
  dort wo unumgänglich (kompakter Repack).

---

## 0. Ausgangslage (bereits erledigt)

- Audio = FMOD Studio 2.2.26, Bänke lose unter `…/G1R/Content/FMOD/Desktop/*.bank`.
- Bänke **verschlüsselt** (RIFF/FEV-Skelett plaintext, FSB5-Sampledaten + String-Table chiffriert).
- **Key liegt vor:** `StudioBankKey = NGpxstJ42kfNfz4z3CsS`, per UE4SS-CDO-Dump
  (`gore_fmod_key.json`, Commit `3c8001b` auf `feat/fmod-bank-key-dump`).
- Format-Kette bekannt: `.bank` (RIFF `FEV `) → 1..n eingebettete FSB5 → Samples
  (Codec pro FSB5: Vorbis / FADPCM / PCM).

---

## M0-Ergebnis (erledigt) + Repack-Entscheidung

Decrypt-Spike (`crates/gore-fmod`) erfolgreich, pure-Rust, kein FMOD. Fakten:
- **Codec = Vorbis** (NICHT FADPCM) überall, 48kHz, 1-2ch, FSB5 v1. FADPCM-Encode-Problem entfällt.
- **Genau 1 FSB5 pro Bank** mit ALLEN Samples (Music 175 / SFX 7218 / CINEMATICS 49 / VO 2).
  Master = nur Mixer, kein FSB5. Nur 2 Vorbis-Setup-CRC32 (stereo 0xc4c30a29, mono 0x355295ca).
- Cipher: `plain[i] = cipher[i].reverse_bits() ^ key[i%20]`, Key `NGpxstJ42kfNfz4z3CsS`,
  pro FSB5-Block. Symmetrisch. FEV-Metadaten plaintext.

**Repack-Entscheidung: Multi-FSB PCM-Injektion (pure-Rust, kein FMOD).** Da 1 FSB/Bank und
ein FSB nur EINEN Codec hat, bläht reines PCM-Repack die ganze Bank ×15 auf. Stattdessen:
neues kleines **PCM16-FSB5 nur für getauschte Samples** an die Bank anhängen, SNDH-Tabelle
+ RIFF/LIST-Sizes fixen, und die **Event→Sample-Referenz im BNKI** auf das neue FSB umbiegen.
→ kein FMOD, Bloat nur in Höhe der getauschten Sounds. Offenes Risiko = RE der
event→(fsb_index, subsound_index)-Referenz (Milestone 2a).

## Die FMOD-Frage, klar entschieden

| Tätigkeit | FMOD nötig? | Begründung |
|---|---|---|
| **Entpacken / anhören / extrahieren** | **Nein** | Decrypt + FSB5-Parse + Decode sind alle offen reimplementierbar (FADPCM/PCM trivial, FMOD-Vorbis via CRC32-Setup-Tabellen aus vgmstream portierbar). |
| **Repack — Pfad A: PCM-Rebuild** | **Nein** | Betroffenes FSB5 komplett als **PCM16** neu schreiben. PCM-Encode ist trivial pure-Rust. Kostet Größe (unkomprimiert). |
| **Repack — Pfad B: fsbank** | **Ja (User-Install)** | FMODs `fsbank` re-enkodiert kompakt in Vorbis/FADPCM. Kein Bundling (EULA) → User liefert FMOD Engine. |

**Kernaussage:** Das gesamte Feature ist **ohne Third-Party und ohne FMOD machbar**, wenn
wir beim Repack auf PCM gehen (Pfad A). FMOD wird nur optional für kompakte Ausgabe (Pfad B).

Warum PCM-Rebuild geht obwohl FADPCM-Encode nicht offen ist: FSB5 hat **einen Codec pro
Sub-Bank**, nicht pro Sample. Wir können einen Sample nicht in ein FADPCM-FSB einmischen —
aber wir können das **ganze betroffene FSB5 neu als PCM16 schreiben** (alle seine Original-
Samples dekodieren → als PCM zurückschreiben, plus den getauschten). Decode aller Codecs ist
offen, PCM-Encode ist trivial. Damit umgehen wir den FADPCM-Encode-Block vollständig.

Risiko/Tradeoff Pfad A: Größe. Eine Bank hat mehrere FSB5 (oft pro Event-Gruppe); wir
schreiben nur das FSB neu, das den getauschten Sample enthält → Bloat begrenzt. Wie stark,
zeigt erst Milestone 1 (Anzahl FSB5 / Gruppierung sichtbar nach Decrypt).

---

## 1. Milestone 0 — Decrypt-Spike (de-risk, ~½ Tag)

Bevor irgendwas gebaut wird, die eine offene Unbekannte schließen: **welcher Codec, wie viele
FSB5 pro Bank, wie gruppiert.** Das entscheidet Repack-Bloat und Vorbis-Aufwand.

- FMOD-Bank-Verschlüsselung reimplementieren (vgmstream `fsb_encrypted` / FMOD bank-XOR mit Key).
- Mit `NGpxstJ42kfNfz4z3CsS` `Music.bank` + `SFX.bank` entschlüsseln.
- FSB5-Header parsen: `numsamples`, `mode` (Codec), Anzahl FSB5, Sample-Namen aus String-Table.
- Output: Tabelle „Bank → FSB5-Count → Codec → Sample-Count“. **Geht ohne FMOD.**

Gate: Ergebnis bestimmt, ob Vorbis-Decode überhaupt nötig (falls alles FADPCM) und wie groß
PCM-Rebuild ausfällt.

---

## 2. Architektur (folgt bestehendem Muster)

Pattern im Repo: Domain-Codec = eigenes Crate `gore-<x>`; dünner CLI-Command in
`crates/gore/src/cmd/<x>.rs`; FFI-Bridge in `gore-ffi`; GUI in `apps/mod-studio`.
Vorbild: `gore-loc` (Codec) + `crates/gore/src/cmd/loc.rs` (CLI) + `gore-oodle` (binär-codec).

### 2.1 Neues Crate `crates/gore-fmod`
Reiner Codec, keine I/O-Policy. Module:
- `bank.rs` — RIFF/FEV-Parser: Chunks (FMT/LIST/PROJ/BNKI/…), `SNDH`-Tabelle (FSB5 Offset+Size),
  String-Table. Decrypt/Encrypt-Hülle.
- `crypto.rs` — FMOD Bank De/Encryption (Key-basiert).
- `fsb5.rs` — FSB5-Container: Sample-Header, Offsets, Codec-Mode, Namen.
- `codec/` — `pcm.rs` (decode+encode), `fadpcm.rs` (decode), `vorbis.rs` (decode via CRC32-Setup;
  Tabellen aus vgmstream portiert). **Encode nur PCM** (Pfad A).
- `repack.rs` — FSB5 neu bauen (PCM-Rebuild), `SNDH`-Offsets + RIFF-Chunk-Sizes fixen, re-encrypt.
- `fsbank.rs` *(optional, feature `fsbank`)* — FFI zu user-supplied `fsbank.dll`
  (`FSBank_Init/Build/Release`) für kompakten Vorbis/FADPCM-Repack (Pfad B).
- `error.rs` — `FmodBankError` (thiserror), wie `LcacheError`.

Public API:
```
Bank::open(bytes, key) -> Bank          // decrypt + parse
Bank::list() -> Vec<SampleInfo>          // name, codec, size, fsb_index
Bank::extract(sample, fmt) -> Vec<u8>    // -> wav/ogg
Bank::replace(sample, new_wav) -> ()     // PCM-rebuild des betroffenen FSB
Bank::save(key) -> Vec<u8>               // repack + re-encrypt
```

### 2.2 CLI `crates/gore/src/cmd/audio.rs`
Subcommand `gore audio` (Dispatch in `crates/gore/src/main.rs`, registriert in `cmd/mod.rs`):
- `gore audio list    --bank <f> [--key <k>|auto]`       → JSON/Table aller Samples
- `gore audio extract --bank <f> --sample <name|all> --out <dir>`  → wav/ogg
- `gore audio replace --bank <f> --map replace.json [--out <bank>]` → repackt; **ohne `--out`
  in-place über die Spieldatei** (Backup `*.bank.gore-bak` zuerst), wie Loc-`import`.
- `gore audio export-patch --map replace.json --out patch.zip` → Sharing-Paket (Audio + Manifest).
- `gore audio apply-patch  --patch patch.zip [--bank <f>]` → Patch auf eigene Bank anwenden (in-place+Backup).
- `gore audio restore --bank <f>` → aus `*.bank.gore-bak` zurückspielen.
- Key-Quelle: `--key`, sonst `gore_fmod_key.json` (vom Dump), sonst Fehler mit Hinweis.
- `--codec pcm|fsbank` wählt Repack-Pfad (Default `pcm` = kein FMOD).

`replace.json`: `{ "EventSampleName": "C:/path/new.wav", … }`.
Patch-Zip: `replace.json` + referenzierte Audio-Dateien (relative Pfade). Kein Spiel-Audio drin.

### 2.3 FFI `crates/gore-ffi`
Dispatch-Kommandos `audio_list` / `audio_extract` / `audio_replace` (JSON in/out, wie
`loc_*`). mod-studio ruft darüber.

### 2.4 mod-studio (`apps/mod-studio`, Flutter)
Neuer Tab „Audio“. **Wichtig: Audio ist KEIN UE4SS-Mod.** FMOD lädt `.bank` beim Start
direkt von Disk; UE4SS kann nichts injecten. Liefermodell = **Loc-Muster**, nicht der
UE4SS-Mod-Export.

UI:
- Baumliste Bank → Sample (Name, Codec, Dauer).
- Play-Button (Extract → temp wav → `just_audio`).
- „Replace…“ Filepicker pro Sample; Diff-Liste der ausstehenden Änderungen.

Zwei getrennte Aktionen (entschieden):
- **„Auf Spiel anwenden"** *(In-place + Backup, wie Loc-`import`)*: Bank repacken →
  Original nach `*.bank.gore-bak` sichern → atomar über die Spieldatei in
  `G1R/Content/FMOD/Desktop/` schreiben. Sofort live, kein Mod-Loader. „Restore"-Button
  stellt aus Backup wieder her.
- **„Patch exportieren"** *(Sharing)*: erzeugt ein kleines Audio-Patch-Paket =
  **ersetzte Audio-Dateien + `replace.json`-Manifest** (NICHT die 216MB-Bank, kein
  Versand von Original-Spiel-Audio). Empfänger wendet es mit `gore audio apply-patch`
  auf seine eigene Bank an. Audio-Äquivalent zu Loc `edits.json`.

---

## 3. Reihenfolge / Milestones

1. **M0 Decrypt-Spike** — Codec/FSB5-Map (oben). *Gate.*
2. **M1 Extract (read-only, kein FMOD)** — Crate `gore-fmod` decode-Pfad + `gore audio list/extract`.
   Verifikation: extrahierte wav gegen vgmstream-Referenz (nur Test, keine Dependency).
3. **M2 Repack Pfad A (PCM, kein FMOD)** — `replace`/`save` + re-encrypt + `gore audio replace`.
   Verifikation: repackte Bank lädt im Spiel, getauschter Sound spielt.
4. **M3 mod-studio Audio-Tab** — FFI + GUI.
5. **M4 (optional) fsbank-Pfad B** — feature-gated, für kompakte Ausgabe; nur wenn PCM-Bloat stört.

Jeder Milestone für sich nützlich; M1 liefert schon „Sounds rippen“ ohne irgendeine Abhängigkeit.

---

## 4. Offene Risiken

- **FMOD-Vorbis-Decode**: braucht CRC32-Setup-Tabellen (Port aus vgmstream). Aufwand mittel.
  Falls M0 zeigt „alles FADPCM“ → entfällt; falls Vorbis-Musik → nötig für Extract/Play.
- **PCM-Bloat (Pfad A)**: Größe der neu geschriebenen FSB. M0 zeigt Gruppierung → Abschätzung.
  Fallback ist M4 (fsbank).
- **Re-Encrypt-Schema exakt**: Bank muss bit-genau so verschlüsselt sein, dass Runtime lädt.
  In M0 verifizieren (decrypt→encrypt round-trip == original).
- **VO gestreamt?**: VO.bank winzig; evtl. Programmer-Sounds/externe Streams. Separat prüfen,
  nicht Teil v1.
