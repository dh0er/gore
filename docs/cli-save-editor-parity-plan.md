# Vollständige Save-Editor-Parität für die GORE-CLI

Stand: 2026-10-02. Status: implementiert. `gore save` bietet 142 Befehle und
denselben Umfang über MCP. Die 64 verpflichtenden Funktionsbereiche sind im
[Paritätsmanifest](save-editor-parity.json) konkreten Befehlen zugeordnet;
Integrationstests prüfen diese Zuordnung. Die
[Benutzerdokumentation](guide/save-games.md) beschreibt die tatsächliche Syntax.
Die folgenden Befunde und Ausbauphasen dokumentieren den Ausgangszustand und
den umgesetzten Plan.

## Verbindliches Ziel

`gore` soll jede Funktion des Save-Editors über die Kommandozeile anbieten:
lesen, suchen, darstellen, bearbeiten, speichern, zurücksetzen, importieren,
sichern, wiederherstellen und konfigurieren. Auch Rohdaten, abgeleitete
Informationen, Bilder, Lokalisierung und Verwaltungsfunktionen gehören dazu.
Keine Funktion darf als „nur GUI“, „experimentell“ oder „später“ entfallen.

Parität bedeutet dieselben erreichbaren Datenzustände, Informationen,
Validierungen und Wiederherstellungsmöglichkeiten. Eine GUI-Interaktion erhält
ein bedienbares CLI-Gegenstück: Suchfeld → Suchargument, Registerwechsel →
Bereichsbefehl, Tooltip → Detailausgabe, Bild → Bildexport/Ansicht,
Speichern/Zurücksetzen → Entwurf anwenden/verwerfen. Der Save-Editor muss für
keine Save-Operation gestartet oder installiert sein.

Die CLI erhält verständliche Fachbefehle **und** einen vollständigen Zugang zum
Core-Protokoll. Ein generischer JSON-Aufruf allein erfüllt das Ziel nicht.
Unbekannte oder schreibgeschützte Daten bleiben genauso sichtbar wie im Editor;
Schreibbarkeit wird aus den tatsächlichen Core-Fähigkeiten ermittelt.

## Befund im Repository vor der Umsetzung

- [CLI-Einstieg](../crates/gore/src/main.rs): kein `save`-Befehlsbereich.
  [CLI-Abhängigkeiten](../crates/gore/Cargo.toml): bisher kein `gore-save`.
- [Save-Core](../crates/gore-save/src/lib.rs): vorhandene Rust-Bibliothek mit
  `execute_json`, 35 dispatchbaren Befehlen, Lesen/Schreiben von Saves,
  Profilverwaltung, Backups und Wiederherstellung. Der bestehende JSON-Vertrag
  ist ein direkt nutzbarer Ausgangspunkt.
- [Codec](../crates/gore-save/src/codec_backend.rs): Kraken läuft bereits in
  Rust im Prozess; Save-Bearbeitung benötigt weder Spielstart noch Oodle-DLL.
- [Editor-Orchestrierung](../apps/save-editor/lib/features/editor/domain/editor_notifier.dart):
  `saveAllPending` enthält zusätzlich wichtige Konflikt-, Reihenfolge-,
  Bündelungs- und Teilerfolgsregeln. Diese fehlen bei einem bloßen CLI-Wrapper
  um `write_save`.
- [Editor-Oberflächen](../apps/save-editor/lib/features/editor/ui): sechs
  Hauptbereiche mit weiteren Unterbereichen. Die
  [README](../apps/save-editor/README.md) nennt nur einen Teil der Funktionen.
- [Kataloge](../apps/save-editor/assets),
  [Domänenmodelle](../apps/save-editor/lib/features/editor/domain),
  [Lokalisierung](../apps/save-editor/lib/loc) und
  [Einstellungen](../apps/save-editor/lib/features/app/domain/ui_settings.dart)
  enthalten weitere Anzeige-, Ableitungs- und Verhaltensregeln.
- Bestehende CLI-Helfer `location`, `find`, `loc`, `config` und `texture` können
  Teile der Infrastruktur liefern. Ihre heutige Existenz beweist noch keine
  vollständige Save-Editor-Parität.

## Geplante öffentliche Oberfläche

Neuer Bereich: `gore save …`. Bestehende Modding-Befehle behalten ihre Bedeutung.
Die folgenden Namen bilden den geplanten Vertrag; Umbenennungen müssen die
Abdeckungsmatrix und Dokumentation gemeinsam aktualisieren.

- `<SAVE>` bezeichnet eine ausdrücklich gewählte Datei. Verwaltungsbefehle
  verwenden `--root <DIR>` und gegebenenfalls `--profile <ID>`.
- `--actor hero` oder `--actor <GLOBAL_ID>` wählt den Akteur. Anzeigenamen
  dürfen als Suche dienen; mehrdeutige Treffer müssen vor einem Schreiben
  aufgelöst werden. Knowledge-Schlüssel, UniqueName und GlobalId werden passend
  zum jeweiligen Bereich ermittelt, einschließlich der Hero-Ereignisse.
- Lesebefehle bieten Text und `--json`; JSON enthält rohe IDs, Werte, Pfade,
  Schreibbarkeit und Herkunft auch bei lokalisierten Anzeigenamen.
- Listen unterstützen die jeweiligen Editor-Filter, stabile Sortierung,
  `--offset`, `--limit` und `--all`. Eine Grenze darf keine unbemerkte Kürzung
  verursachen.
- Änderungen werden direkt angewandt oder mit `--draft <FILE>` vorgemerkt.
  `gore save draft show|validate|apply|reset` entspricht der gemeinsamen
  Speichern-/Zurücksetzen-Funktion des Editors.
- `--dry-run` simuliert die vollständige Operation mit allen Vorbedingungen
  und zeigt die geplanten Änderungen. Es verändert keine Save-, Profil-,
  Backup-, Placement-, Recovery- oder Einstellungsdatei.
- `--out <FILE>` unterstützt die vom Core angebotene Ausgabe in eine andere
  Datei. Beziehungen zu Profilen und Placement-Notizen werden dabei ausdrücklich
  ausgewiesen; der Quell-Save wird nicht versehentlich umbenannt/zugeordnet.
- Backups sind standardmäßig aktiv. Die CLI meldet Ziel, betroffene Begleitdateien,
  Backup-Pfade, Warnungen und den tatsächlich erreichten Zustand.
- Explizite Schreibbefehle führen die gewünschte Änderung aus. Dauerhaftes
  Löschen eines Backups erhält die entsprechende Bestätigung, mit `--yes` für
  Skripte. Bei nicht interaktiver Eingabe wird niemals unbegrenzt gewartet.
- `--lang` unterstützt alle Editor-Sprachen mit derselben Fallback-Reihenfolge;
  `--show-ids` steuert die Textdarstellung. JSON bleibt sprachunabhängig.
- stdout enthält Ergebnisdaten; Fortschritt und Diagnose gehen nach stderr.
  Fehlercodes unterscheiden ungültige Eingaben, nicht unterstützte Änderungen,
  Codec-/Parsefehler, Dateikonflikte und Teilerfolge.
- Das bestehende globale `--force` hebt keine Save-Validierung, Hash-Prüfung,
  Konfliktregel oder Wiederherstellungsvorbedingung auf.

## Vollständige Abdeckungsmatrix

Jede Zeile ist verpflichtend. Die Kürzel werden später in einem maschinenlesbaren
Manifest mit einzeln benannten Aktionen, Core-Zuordnung und Abnahmeszenarien
geführt. Zusammengefasste Tabellenzeilen dürfen dort keine Einzelaktion verdecken.

### Saves, Profile und Backups

| ID | Editor-Funktion einschließlich Randfällen | Geplantes CLI-Gegenstück |
| --- | --- | --- |
| S01 | Save-Verzeichnis wählen, automatisch ermitteln, neu einlesen | `settings get/set save-root`, `list --root`, `refresh` |
| S02 | Profile, aktives Profil, zugeordnete Saves und „Andere Saves“ anzeigen/auswählen | `profiles list/show`, `list --profile/--other`, Auswahl im Entwurf |
| S03 | Externe Datei öffnen; externe Einträge merken/entfernen; andere Saves dauerhaft ausblenden, ohne sie zu löschen | `inspect <SAVE>`, `library add/remove/hide/unhide/list` |
| S04 | Save-Metadaten: Slot, Anzeigename, Profil, Kapitel, Karte, Spiel-/Ladezeit, Quick-/Autosave, Größe, Hash, Format, Kompression, Status | `inspect`, `list --details` |
| S05 | Screenshot anzeigen, Diagnose-JSON anzeigen und kopieren | `screenshot export`, `inspect --json`, `inspect --copy`; Bilddatei öffnen |
| S06 | Save-Anzeigename ändern und die Profilmetadaten synchronisieren | `rename <SAVE> --name` |
| S07 | Save einem Profil zuordnen/umzuordnen; externe Datei in freien Slot importieren; Namenskollisionen vermeiden | `profile assign`, `import --root --profile` |
| S08 | Profilzuordnung entfernen; verwaiste Referenzen entfernen; physische Datei behalten | `profile detach --slot --profile --root` |
| S09 | Save löschen samt korrekter Profilaktualisierung und Recovery-Eintrag | `delete --profile --root` |
| S10 | Gelöschten Save einschließlich passendem Profilstand wiederherstellen; Recovery anzeigen/verwerfen; Neustart-/Crash-Fall | `recovery list/show/restore/dismiss` |
| S11 | Save-Backups und Backups von `PersistentDataList.sav` auflisten | `backups list --include-companions` |
| S12 | Backup-Anzeigenamen vergeben, ändern und entfernen | `backups rename --name`, `backups rename --clear-name` |
| S13 | Save-Backup oder Profil-Begleitbackup wiederherstellen; Placement-Notizen und Begleitdatei-Status beachten | `backups restore --backup --target` |
| S14 | Backup nach Bestätigung dauerhaft löschen | `backups delete --backup [--yes]` |

### Charaktere, Attribute, Skills und Positionen

| ID | Editor-Funktion einschließlich Randfällen | Geplantes CLI-Gegenstück |
| --- | --- | --- |
| C01 | Gemeinsame Charakterliste aus Akteuren, Wissen und Ereignissen; Player, NPCs, Kreaturen; Kategorien, Rollen, Tod, Händlerstatus, Suche/Filter | `characters list/show --query --kind --role`, `--actor` |
| C02 | Sämtliche Spieler-/NPC-Attribute lesen und BaseValue/CurrentValue bearbeiten; Gruppen, Schutzwerte, Level, Erfahrung, Lernpunkte, Mana, Lebenspunkte und Erklärungen | `attributes list/show/set --actor --set-class --field base/current` |
| C03 | Sämtliche Skill-Kategorien und angebotenen Stufen lesen, lernen, umlernen und verlernen; unbekannte gespeicherte Skills anzeigen | `skills list/show/set --actor --skill --tier` |
| C04 | NPC wiederbeleben einschließlich HP, Todes-/Niederlagserinnerungen, Tags und Leichenzustand | `npc revive --actor` |
| C05 | Explizite NPC-Beziehung zum Spieler anzeigen und alle Editor-Optionen setzen | `npc relationship show/set --actor` |
| C06 | Spielerposition XYZ und Rotation Pitch/Yaw/Roll lesen und setzen | `position show/set --actor hero` |
| C07 | NPC-Position/Rotation lesen und setzen; Spawnposition/-rotation als Referenz anzeigen; fehlende oder nie platzierte Akteure korrekt behandeln | `position show/set --actor <ID> --include-spawn` |
| C08 | Benannte Orte suchen, nach Gebiet/Art filtern, Position übernehmen; Richtung wahlweise übernehmen | `locations list/show`, `position set --location --apply-facing` |
| C09 | NPC auf die gespeicherte Spawnposition/-rotation zurücksetzen | `position reset-to-spawn --actor` |
| C10 | NPC am Ort halten: Routine deaktivieren, ursprünglichen Zustand erfassen; wiederholtes Umsetzen eines bereits fixierten NPCs | `position set --stay`, `position pin-status` |
| C11 | Fixierung aufheben und Routine wiederherstellen; vollständigen Move rückgängig machen; veraltete Undo-Notiz ablehnen | `position resume-routine`, `position undo` |

### Inventar und Gegenstände

| ID | Editor-Funktion einschließlich Randfällen | Geplantes CLI-Gegenstück |
| --- | --- | --- |
| I01 | Spieler-/NPC-Inventar lesen; alle Container und mehrfach vorkommenden Stacks unterscheiden; belegte/leere Slots, Actor-ID, unbekannte Items und Schreibbarkeit | `inventory list/show --actor --container --slot` |
| I02 | Anzahl eines vorhandenen Stacks ändern | `inventory set-count --actor --container --slot --count` |
| I03 | Neue Gegenstände aus dem vollständigen Katalog auswählen und hinzufügen | `items list/show`, `inventory add --actor --item --count` |
| I04 | Den konkret gewählten Stack entfernen; Hinzufügen/Entfernen vor dem Speichern zurücknehmen | `inventory remove --actor --container --slot`, `draft reset --operation` |
| I05 | Inventar auf sauberen Startzustand zurücksetzen, einschließlich Ressourcenstufe und NPC-spezifischem Verhalten | `inventory reset --actor --resources-level` |
| I06 | Beschädigte Slot-IDs erkennen und reparieren; Auswirkungen auf sämtliche betroffenen Container ausweisen | `inventory check-slots/repair-slots` |
| I07 | Spielgetreue Kategorien, Item-Details/Tooltips: Typ, Wert, Gewicht, Stackgröße, Schaden, Schutz/OnEquip, Anforderungen, Magiekreis, Spell-Level/-Mana, Verbrauchseffekte, Rezepte, Schrifttexte, Zutaten, natürliche Waffen | `items show --item --details`, `inventory show --details`, Kategorienfilter |
| I08 | Itembilder und Kategorie-/Attribut-/Schutz-/Skill-Symbole aus der Installation verwenden; ohne Bilder bedienbar bleiben | `assets prepare/list/export/open --kind`, Asset-Referenzen in Detailausgaben |

### Welt, Fortschritt, Glossar und Handel

| ID | Editor-Funktion einschließlich Randfällen | Geplantes CLI-Gegenstück |
| --- | --- | --- |
| W01 | Spielzeit als Gesamtsekunden und Tag/Uhrzeit lesen/setzen; nullbasierte Tage und Wertebereiche wie im Editor | `time show/set --seconds` oder `--day --hour --minute --second` |
| W02 | Profilschwierigkeit: Preset, Kampf/Ressourcen/Fortschritt, Flow Helper, Permadeath; Custom-Regeln, Novice-Regeln und unbekannte Presets | `difficulty show/set --profile --root` |
| W03 | Quests und Tutorials mit Gruppen, Zuständen, Namen, Suche, Zählungen und Seiten lesen; alle angebotenen Zustände bearbeiten | `quests list/show/set-state`, `tutorials list/show/set-state` |
| W04 | Story-Zustände aus gespeichertem Stand und Katalog zusammenführen; gesetzte/ungesetzte/ unbekannte IDs, Rohwert, vorhandene Editor-Darstellung, Suchen/Filtern | `story list/show` |
| W05 | Story-ID erstellen, Wert ändern und Eintrag entfernen; Expected-Werte prüfen; unbekannte Neuanlage ausdrücklich erlauben; case-insensitive Identität | `story set/unset --expect`, `--allow-unknown-create` |
| W06 | Glossar für NPCs, Kreaturen und Orte lesen; Dokumente, Kategorien, Texte und Entdeckungszustände | `glossary list/show --category --entry` |
| W07 | NPC-Entdeckung sowie Dokument-/Questzustände und einzelne Textsegmente freigeben/sperren; Unlock-/Viewed-Ereignisse korrekt berücksichtigen | `glossary set-state`, `glossary segment unlock/lock` |
| W08 | Glossarporträts und Spieltexte laden und anzeigen/exportieren | `glossary show --with-assets`, `assets export/open --kind portrait` |
| W09 | Wissen pro Spieler/NPC lesen, suchen, hinzufügen und entfernen; Charaktereintrag erstmalig anlegen; Katalogauswahl | `knowledge list/add/remove/create-character`, `catalog search --domain knowledge` |
| W10 | Ereignisse pro Spieler/NPC lesen: Tags, Zeit, Dauer, Magnitude, Beteiligte, optionale Klassen, Position und Payload; Ereignisse entfernen/duplizieren; Änderungen zurücknehmen | `events list/show/remove/duplicate`, `draft reset --operation` |
| W11 | Fraktionsverbrechen, Verbrechensarten und offene/erlassene Summen lesen; Verbrechen einer Gilde vergeben | `factions list/show/forgive --guild` |
| W12 | Alle katalogisierten Truhen/Türen samt Region, Schlüsseln, Schwierigkeit und Save-Zustand lesen; öffnen und erneut sperren einschließlich Tür-Nebenzuständen | `locks list/show/unlock/lock --lock` |
| W13 | Händlerliste/Details: aktueller Bestand, gespeicherter Restock-Bestand, Erz/Kaufkraft, Platzhalterzeilen und generierte Ereignisse | `traders list/show --index` |
| W14 | In beiden Händlerbeständen Mengen ändern, Positionen hinzufügen/entfernen und Erz bearbeiten | `traders stock set/add/remove --index --map current/default` |
| W15 | Händleraktivitätszeit lesen/setzen, „nie aktiv“ darstellen, Restock-Fenster und Voraussetzungen berechnen; Editor-Aktion zum Fälligstellen | `traders timing show/set/make-due --index` |

### Rohdaten, Anzeige, Einstellungen und Anwendungsabläufe

| ID | Editor-Funktion einschließlich Randfällen | Geplantes CLI-Gegenstück |
| --- | --- | --- |
| R01 | Alle Daten durchsuchen: Quelle, Datentyp, editierbar/read-only, Container/Structs, Query, Pfade, Trefferzahl und Seiten | `data search --source --kind --editable --query` |
| R02 | Jeden vom Editor angebotenen Werttyp lesen/bearbeiten, einschließlich nativer Struct-/Vektor-/Rotator-Werte; Roh- und Fachansichten desselben Ziels | `data show/set --path-file --value-json` |
| R03 | Elemente in typisierten Sets hinzufügen/entfernen und Arrayelemente entfernen/duplizieren | `data set-add/set-remove/array-remove/array-duplicate` |
| R04 | Vollständiges Core-Protokoll einschließlich öffentlicher Namen, privater Namens- und FString-Operationen zugänglich machen | `core exec --request-file` bzw. stdin; Fähigkeiten mit `core capabilities` |
| R05 | Codec-Status und Format-/Codec-Roundtrip prüfen; Decode-only-/nicht verifizierte Daten entsprechend sperren | `codec status`, `validate --roundtrip/--codec-roundtrip` |
| A01 | Übersicht und alle Statistikwerte mit denselben Zählregeln, unbekannten Zuständen und Datenquellen | `overview`, `statistics` |
| A02 | Spieltexte automatisch oder aus gewählter `.lcache` extrahieren; Status/Metadaten anzeigen, Texte suchen, Quelle wechseln/neu laden; alle Editor-Sprachen und Fallbacks | `localization status/prepare/find`, bestehendes `gore loc` gemeinsam nutzen |
| A03 | Bildcache vorbereiten, Quellidentität feststellen, erneuern und freigeben; Originalporträts nutzen; fehlende/stale Quelle anzeigen | `assets prepare/status/release`; Core-Assetfunktionen vollständig abdecken |
| A04 | Editor-Einstellungen lesen/ändern/zurücksetzen: Save-Verzeichnis, externe/ausgeblendete Pfade; UI-/Spieltextsprache, IDs, Theme, Schrift, Skalierung, Fenstergröße/-status, Update-Option und Quellhinweisstatus; alte Einstellungen migrieren | `settings show/get/set/reset --scope editor/ui`; sämtliche gespeicherten Felder |
| A05 | Visuelle Informationen gemeinsam betrachten, einschließlich Screenshots, Porträts, Symbole und Tooltips | `report <SAVE> --format html --out`; lokale Datei öffnen; Export benutzt dieselben Daten und Anzeigepräferenzen |
| A06 | Versions-/About-Daten, Lizenz-/Schriftinformationen, Release-/Projektlinks und Updateprüfung | `about`, `licenses`, `updates check --product save-editor`, `updates open-release` |
| A07 | Verfügbare Editor-Aktualisierung durchführen; installiertes und portables Verhalten korrekt abbilden | `updates install --product save-editor`; vorhandenen verifizierten Installer-/Updaterweg nutzen, bei portablem Build Download/Ersetzung nach dessen Vertrag |
| A08 | Änderungen je Save und Akteur vormerken, anzeigen, ersetzen und zurücknehmen; ungültige Entwürfe blockieren; Kontextwechsel/Refresh mit ungespeicherten Änderungen behandeln | `draft create/show/stage/remove/reset/validate/apply`, `refresh --draft` |
| A09 | Speichern aller Änderungen: Konflikterkennung, geordnete Teilschritte, Backup einmal, Fortschritt, Teilerfolg und unverbrauchte Änderungen | gemeinsamer Workflow hinter allen Schreibbefehlen und `draft apply` |
| A10 | Zustand nach Schreib-/Restorefehler neu lesen; Erfolg trotz nachgelagertem Anzeige-/Sidecarfehler korrekt melden; Recovery nicht verlieren | strukturierte Ergebnisse, `refresh`, `recovery`, aktualisierter Entwurf |
| A11 | Komfortaktionen wie Ordner-/Dateiauswahl, Kopieren, Details öffnen und erneutes Einlesen | Pfadargumente, `--copy`, `assets open`, `report --open`, `refresh`; keine Save-Funktion hängt von einem Dialog ab |

Für A01 sind mindestens Kapitel, Spielzeit, Weltzeit, Level, Erfahrung,
Lernpunkte, Gilde/Rang, HP/Mana, Questzustände, getötete Monster/NPCs,
besiegte NPCs, bekannte NPCs/Händler/Lehrer, offene Verbrechen, gelernte Skills,
Itemanzahl und Erz einzeln abzunehmen. Die ursprünglichen Regeln gegen
Doppelzählung und zur Behandlung unvollständiger Daten gelten auch in der CLI.

### Bindung an das vorhandene Core-Protokoll

Alle 35 derzeit dispatchbaren Befehle müssen im Manifest vorkommen. Fachbefehle
dürfen sie bündeln; kein Endpoint darf dabei unerreichbar werden.

| Bereich | Bestehende Core-Befehle |
| --- | --- |
| Bestand/Inspektion/Suche | `scan_save_dir`, `warm_save`, `inspect_save`, `search_typed_properties` |
| Codec/Validierung | `check_codec`, `validate_roundtrip`, `validate_codec_roundtrip` |
| Fortschritt | `query_progression` mit sämtlichen Bereichen: `quests`, `tutorials`, `glossary`, `knowledge`, `events`, `story` |
| Charaktere | `private.characters.list`, `private.npc.list`, `private.npc.attributes`, `private.npc.position`, `private.npc.inventory`, `private.skills.list` |
| Fraktionen/Locks | `private.factions.list`, `private.locks.list` |
| Händler | `private.traders.list`, `private.traders.detail` |
| Save-/Profilschreiben | `write_save`, `write_difficulty`, `assign_save_profile`, `remove_save_from_profile`, `delete_save` |
| Backups | `list_backups`, `restore_backup`, `delete_backup`, `rename_backup` |
| Recovery | `restore_deleted_save`, `dismiss_deleted_save_recovery` |
| Lokalisierung | `loc_status`, `loc_find`, `loc_extract` |
| Assets | `item_icons_prepare`, `item_icons_source_identity`, `item_icons_release` |

Zusätzlich werden die einzelnen Schreiboperationen von `write_save` erfasst:

- Öffentliche unterstützte Property-Pfade, insbesondere `public.m_PlayerSaveName`.
- `private.player.setPlayerName`, `private.profile.setProfileName`,
  `private.player.setAttribute`, `private.player.setTransform`.
- `private.replaceFString` und dessen Alias `private.fstring`.
- `private.typed.setValue`, `private.typed.setAdd`, `private.typed.setRemove`,
  `private.typed.arrayRemove`, `private.typed.arrayDuplicate`.
- `private.inventory.setItemCount`, `private.inventory.addItem`,
  `private.inventory.removeItem`, `private.inventory.reset`,
  `private.inventory.repairSlots`.
- `private.knowledge.addCharacter`, `private.knowledge.setEntry`.
- `private.skills.set`, `private.npc.revive`, `private.npc.setRelationship`.
- `private.glossary.setSegment`, `private.story.apply`,
  `private.factions.forgive`, `private.locks.setUnlocked`.
- `private.traders.setStock`, `private.traders.addItem`,
  `private.traders.removeItem`.

Auch Request-Optionen gehören zur Parität: Preview-/Chunkgrenzen, Such- und
Progressionfilter, Ziel-/Begleitpfade, erwartete Hashes, Profil-ID, Backup,
`outputPath`, `syncPersistentDataList`, `placementNotes` und
`clearPlacementNotes`. Der Rohzugang erhält diese Möglichkeiten vollständig;
die Fachbefehle bieten ihre jeweiligen sinnvollen Optionen ausdrücklich an.

## Gemeinsame Architektur

### 1. Rust-Core direkt anbinden

`crates/gore` erhält die Abhängigkeit `gore-save` und einen modularen Bereich
`crates/gore/src/cmd/save/`. Die CLI verwendet das `rlib` direkt und benötigt
keine Flutter-Installation und keine separate Editor-DLL.

Aus dem bestehenden Dispatcher wird eine öffentliche, typisierte Rust-API
für Requests/Responses und Operationen herausgelöst. `execute_json` und die
bisherige C-ABI bleiben kompatible Adapter. Zusätzliche Capability-Metadaten
liefern Operationen, Argumente, Werttypen und Mutationswirkung, damit Fachbefehle,
Rohzugang und Tests keine getrennten Listen pflegen.

Keine zweite Save-Parser-, Kompressions-, Backup- oder Profilimplementierung.
Der JSON-Rohzugang wird schon früh verfügbar, aber erst die vollständige
Fachoberfläche schließt die Paritätsaufgabe ab.

### 2. Editor-Workflows in Rust gemeinsam nutzen

Die Regeln aus `saveAllPending` und den Fachmodellen werden in einen
`gore-save`-Workflow mit Planungs-, Validierungs- und Ausführungsschritt
überführt. Der Editor ruft danach denselben Dienst auf. Dazu gehören:

- Doppelte/überlappende Rohpfade und Konflikte zwischen Roh- und Fachoperationen.
- Strukturelle Änderungen, verschobene Arrayindizes/Slot-IDs,
  absteigende Arraylöschungen und vorgeschriebene Reihenfolgen.
- Wechselwirkungen von Inventarreset/-reparatur, Skills/ActiveEffects,
  Wiederbelebung/Ereignissen, Glossarseiten und Hero-Erinnerungen,
  Knowledge-/Beziehungseinträgen und Händlerzeilen.
- Story-Compare-and-set und Inventarreset als exklusive Schreiboperationen.
- Placement-Drafts, Routineklasse und Undo-Notizen einschließlich mehrfacher
  Platzierung, Export, Import, Backup und Restore.
- Einmaliges Backup des Ausgangszustands; Synchronisation von
  `PersistentDataList.sav` im passenden Schritt.
- Exakte Zuordnung bereits angewandter und noch ausstehender Einzeländerungen.
  Zwei gleiche Add-Operationen bleiben zwei Operationen.

Der bestehende Editor speichert manche Kombinationen in mehreren Schritten.
Der gemeinsame Dienst muss diese Semantik korrekt darstellen: kein behaupteter
Gesamt-Rollback und kein „alles fehlgeschlagen“, wenn frühere Schritte bereits
gespeichert wurden. Resultate enthalten `committed`, `remaining`, Backups,
Warnungen und den frisch gelesenen Zustand; die CLI liefert bei Teilerfolg
einen eigenen Fehlerstatus. Eine neue Gesamttransaktion wäre eine gesonderte
Verhaltensänderung und ist keine Voraussetzung für die Parität.

### 3. Kataloge, Ableitungen und Darstellung teilen

Die vollständigen Item-/Itemstat-, NPC-, Knowledge-, Location-, Lock- und
Glossarkataloge werden für die CLI mitgeliefert. Vorhandene Kataloglogik in
`gore-catalog` wird erweitert und gemeinsam verwendet. Dart-spezifische
Ableitungen wie Inventarfilter, Charakterklassifikation, Attributgruppen,
Glossarsegmente, Statistikzählungen, Zeitumrechnung und Händler-Restockprognose
wandern in gemeinsame Rust-Modelle oder eine daraus generierte Datengrundlage.

Restockprognosen behalten die Unterscheidung zwischen Kalendergrenze,
verstrichener Zeit und Verarbeitung beim nächsten Händlerkontakt. Keine
erfundene exakte Restockzeit. Unbekannte Kategorien, Skills, Presets und fehlende
Metadaten werden sichtbar ausgewiesen.

Lokalisierung und Assetcache verwenden die vorhandenen gemeinsamen
`gore`-Benutzerverzeichnisse. Settings bleiben mit den Editor-Dateien und
Migrationen kompatibel; unbekannte Felder müssen bei CLI-Änderungen erhalten
bleiben. GUI-Darstellungsfelder sind per CLI verwaltbar und werden für den
HTML-Bericht übernommen, soweit sie dessen Darstellung betreffen.

### 4. Entwürfe und Wiederholbarkeit

Ein versionierter Entwurf enthält Ziel-Save, Akteurkontext, stabile
Operations-IDs, Fachoperationen/Rohoperationen, erwarteten Ausgangsstand und
gegebenenfalls Profil- und Placement-Bezug. Er speichert keine geheimen
prozesslokalen Zustände. Direkte Einzelbefehle durchlaufen denselben Planer.

`draft apply` prüft Save-/Profilidentität und Hashes erneut. Nach einer
Strukturänderung werden Ziele semantisch neu aufgelöst; gefährliche
Indexkombinationen werden vor dem ersten Schreiben abgewiesen.
`draft reset` verwirft die Auswahl der Änderungen, ohne einen Save zu schreiben.
Bei Teilerfolg bleiben nur unverbrauchte Operationen erhalten; ein erneuter
Aufruf darf ein bereits gespeichertes Add/Duplicate nicht wiederholen.

Beispiel des geplanten Entwurfsablaufs:

```bash
gore save inventory add G1R-001.sav --actor hero --item ItMi_Orenugget --count 100 --draft changes.json
gore save time set G1R-001.sav --day 5 --hour 12 --minute 0 --second 0 --draft changes.json
gore save draft show changes.json --json
gore save draft apply changes.json --dry-run --json
gore save draft apply changes.json --json
```

Die ersten beiden Aufrufe verändern nur den Entwurf; erst der letzte schreibt
den Save. Der gemeinsame Dienst plant die gesamte Änderung in einem Prozess
und nutzt die vorhandenen Decode-/Parse-Caches, statt für jede Einzeloperation
den großen Save erneut einzulesen.

## Besondere Verträge, die nicht verloren gehen dürfen

1. Schwierigkeit gehört zum Profil in `PersistentDataList.sav`, nicht zum
   komprimierten Slot. Eine externe Datei mit gleichem Slotnamen ist kein
   Mitglied eines lokalen Profils. Interne Profil-ID und sichtbare
   Profilnummer müssen eindeutig beschriftet sein.
2. Händlerbestand und persönliches NPC-Inventar sind getrennte Daten. Händler
   werden über ihre Zeile adressiert; `None`-Platzhalter sind nicht eindeutig
   durch Namen auswählbar. Aktueller und Default-Bestand bleiben getrennt.
3. Inventarziele enthalten Akteur, Container und Slot. Gleichartige Stacks
   dürfen nicht zusammenfallen. Reparatur kann saveweit wirken und muss diese
   Reichweite ausweisen.
4. Profil-/Delete-/Restoreoperationen behalten Hash-Vorbedingungen,
   Dateikonflikterkennung, Recovery-Manifest und Begleitbackups. Änderungen
   dürfen einen noch offenen Recovery-Stand nicht unbemerkt unbrauchbar machen.
5. Eine Placement-Notiz ist nur bei passendem aktuellen Zustand verwendbar.
   Fehlgeschlagene Notizübernahme wird neben einem bereits erfolgreichen
   Save-/Import-/Restorevorgang als Warnung gemeldet.
6. Private Schreibfähigkeit setzt vollständiges/verifiziertes Parsing und
   funktionierende Kompression voraus. Preview-Daten sind kein Schreibbeweis.
7. `scan_save_dir` ist heute nicht garantiert rein lesend: Recovery-Entdeckung
   kann einen unterbrochenen Profilvorgang abschließen. Die gemeinsame API
   trennt nebenwirkungsfreie Bestandsaufnahme und Recovery-Ausführung. CLI-
   Lesebefehle/`--dry-run` bleiben lesend; dieselbe automatische Reparatur ist
   über den Recovery-Workflow explizit erreichbar und im Editor weiter nutzbar.
8. Texte und Katalogdaten können ohne Spielinstallation verwendet werden;
   Originalbilder und neue Lokalisierungsextraktion benötigen eine passende
   Quelle. Fehlende Bilder dürfen Save-Operationen nicht blockieren.

## Umsetzung in abhängigen Arbeitspaketen

| Paket | Konkretes Ergebnis | Abhängigkeit / Abschlussnachweis |
| --- | --- | --- |
| P0 | Vollständiges Feature-Manifest aus allen sechs Editor-Tabs, Unterpanels, Menüs, Settings, Notifier-Aufrufen, Core-Befehlen und Schreiboperationen; Ausgangsszenarien aus bestehenden Editor-Tests festhalten | Jede Tabellen-ID enthält sämtliche Einzelaktionen, Quellorte und erwartete Ergebnisse; keine ungeklärte Funktion |
| P1 | Öffentliche Rust-Request-/Response-/Capability-API, CLI-Abhängigkeit und `save`-Router; Rohzugang, JSON-Fehler, Codecprüfung und nebenwirkungsfreies Inspect/Search | P0; bestehende FFI-Aufrufe bleiben kompatibel; Bibliothek und CLI bauen ohne Flutter |
| P2 | Gemeinsamer Workflow für Planung, Dry-run, Konflikte, Subwrites, Backup einmal, Placement und Teilerfolg; Editor auf diesen Dienst umstellen | P1; bestehende `editor_notifier_test.dart`-Szenarien und unabhängige Rust-Postconditions bestanden |
| P3 | Save-/Profilverwaltung, externe Library, Import, Slotwahl, Rename, Detach, Delete, Backups und Recovery | P1–P2; S01–S14 inklusive Crash-/Restart- und Dateikonfliktfällen |
| P4 | Gemeinsame Kataloge, Lokalisierungs-/Klassifikations-/Zeitmodelle, Settings-Vertrag und Assetbasis | P1; gleiche IDs, Filter, Labels, Fallbacks und unbekannte Zustände wie im Editor |
| P5 | Charakterliste, sämtliche Attribute/Skills, Beziehungen, Revive und vollständige Positions-/Routine-/Undo-Workflows | P2–P4; C01–C11 mit mehreren Akteuren und Placement-Roundtrip |
| P6 | Alle Inventarfunktionen, Container-/Stackadressierung, Slotreparatur, Reset und vollständige Itemdetails | P2/P4/P5; I01–I08, inklusive doppelter Stacks und struktureller Konflikte |
| P7 | Zeit/Schwierigkeit, Quests/Tutorials, Story, Glossar, Knowledge, Events, Fraktionen und Locks | P2–P5; W01–W12 einschließlich Hero-Identität und Story-Expected-Prüfungen |
| P8 | Händlerbestände, Erz, Details/Ereignisse und vollständiges Aktivitäts-/Restockmodell | P2/P4/P7; W13–W15, getrennte Maps und Platzhalterfälle |
| P9 | Vollständige Rohdatenbedienung, alle Statistikwerte, Bildexport/-ansicht, HTML-Bericht, sämtliche Settings, About/Lizenzen und Updateabläufe | P3–P8; R01–R05 und A01–A11 vollständig |
| P10 | CLI-Hilfe/Guide/Beispiele, Packaging und Integration in bestehende MCP-Kommandobeschreibung einschließlich Schreibklassifikation | Nach jedem Paket nachführen; final kein fehlender CLI/MCP-Spec-Eintrag und keine Aussage „Saves nur im Editor“ |
| P11 | Gesamtprüfung mit unabhängigen erwarteten Datenzuständen und Editor-/CLI-Vergleich; Windows-Releasepaket und In-Game-Qualifikation | Alle IDs vollständig bestanden; keine ausgeschlossene, vertagte oder nur per GUI ausführbare Aktion |

P0–P2 bilden den kritischen Anfang. Danach folgen die Fachbereiche in der
angegebenen Reihenfolge. Ein frühes Release einzelner Pakete darf als
Teilabdeckung erscheinen; „vollständige Parität“ gilt erst nach P11.

Voraussichtlich betroffene Orte: `crates/gore-save/src/` (API/Workflows/Modelle),
`crates/gore/src/main.rs`, `crates/gore/src/cmd/save/`, `crates/gore/Cargo.toml`,
`crates/gore-catalog/`, Editor-Domain/Notifier, passende Rust-/Flutter-Tests,
`crates/gore-mcp/src/spec/`, `docs/guide/`, README-Dateien und `build.py`.
Das ist eine gemeinsame Funktionserweiterung, kein vollständiger Neubau des
Save-Editors.

## Abnahme und dauerhafte Absicherung

### Nachweis je Funktion

Das Feature-Manifest führt pro Einzelaktion `featureId`, Editor-Quellort,
Core-Operation/Workflow, CLI-Befehl, Voraussetzungen, erwartete Datei-/Datenwirkung
und Abnahmeszenario. Neue Editor-Funktionen müssen diesen Vertrag ergänzen.
Eine CI-Prüfung vergleicht Manifest, veröffentlichte Core-Capabilities und
Clap-/MCP-Kommandos. Eine manuelle Prüfung aller sichtbaren Editor-Aktionen
ergänzt diese technische Abdeckung; eine reine Suche nach Command-Strings
beweist keine vollständige UI-Parität.

Für denselben Ausgangs-Save werden Editor und CLI gegen getrennte Kopien
geprüft. Maßgeblich sind unabhängig festgelegte Postconditions: Zielwerte,
unveränderte Nachbarakteure/-container, Profilmitgliedschaft, Backup/Recovery,
Placement-Daten und Warnungen. Unveränderte Bereiche werden semantisch und,
soweit garantiert, bytegenau verglichen. Zwei Adapter, die denselben fehlerhaften
Core aufrufen, sind allein kein ausreichender Test.

### Erforderliche Szenarien

- Lesen und Schreiben: GVAS/GSAV, alle angebotenen Werttypen, Unicode,
  fehlende optionale Daten, unbekannte Items/Akteure/Presets und defekte Dateien.
- Inventar/Skills: mehrere NPCs, mehrere Container, identische Stacks,
  Slot-ID-Schäden, Reset, natürliche Waffen, Skill-Stufen und unbekannte Skills.
- Konflikte: Rohwert plus Fachoperation, Add/Remove/Repair/Reset, Arrayindizes,
  Händlerarray, Skills/Effects, Glossar/Ereignisse, Revive und Story-Batch.
- Verwaltung: externe Saves mit lokalem Slotnamen, volles Slotangebot,
  verwaiste Profilreferenzen, konkurrierende Dateiersetzung, Restore nach
  Neustart, Crash zwischen Save-/Profiländerung und Recovery.
- Teilerfolg: gezielter Fehler nach einem erfolgreichen Subwrite, korrekte
  verbliebene Änderungen, Wiederholung ohne doppelte Add-/Duplicate-Wirkung,
  fehlgeschlagene Anzeigeaktualisierung nach erfolgreichem Schreiben.
- Position: benannter Ort mit/ohne Richtung, Spawnreset, Routine an/aus,
  zweites Umsetzen, Undo, stale Undo, Export/Import/Backup/Restore mit Notizen.
- Anzeige: sämtliche Statistikwerte, vollständige Itemdetails und
  Glossartexte, alle Editor-Sprachen, ohne Installation/Bilder/Lokalisierung,
  wechselnde Assetquelle und Caches.
- Settings/Updates: Altformatmigration, Erhalt unbekannter Felder, Änderung
  einzelner Optionen, installierter/portabler Editor und fehlgeschlagene
  Updateprüfung ohne Auswirkung auf Save-Arbeit.
- CLI-Vertrag: Pfade mit Leerzeichen und Windows-/UNC-Pfade, eindeutige
  Akteur-/Profilwahl, Text/JSON, stdin/Datei, Pagination/`--all`, stderr-
  Fortschritt, Fehler-/Teilerfolgscodes, keine Nebenwirkungen bei Dry-run.

Vorhandene Tests unter `crates/gore-save/tests/` und `apps/save-editor/test/`
liefern Regressionsevidenz; fehlende Szenarien werden gezielt ergänzt. Keine
Tests, die lediglich die neue Implementierung nacherzählen. Reale Saves werden
nur als Kopien mit dokumentierten Ausgangszuständen verwendet.

Build-/Testgrundlage: `cargo test -p gore-save`, die betroffenen
`gore-catalog`-Tests, CLI-Integrationstests mit `--features development-cli`,
Clap/MCP-Parität sowie Flutter-Analyse und relevante Editor-Tests nach der
gemeinsamen Workflow-Umstellung. Abschließend das Windows-Release über
`build.py` prüfen; andere Plattformen nur mit belegtem Funktionsumfang zusagen.

Spielwirksame Änderungen müssen zusätzlich auf einer dokumentierten
Spielversion geladen und erneut gespeichert werden: mindestens Attribute,
Skills, Inventar, Wiederbelebung/Beziehung, Position/Routine/Undo,
Quests/Story/Glossar/Knowledge/Events, Locks/Fraktionen und Händlerdaten.
Die Zustände werden nach erneutem Laden geprüft; Offline-Serialisierung allein
belegt die Wirkung im Spiel nicht.

### Fertig bedeutet

- Jede Funktion und Einzelaktion der Matrix ist über die CLI ausführbar.
- Gleiche Eingabe führt zu denselben fachlichen Ergebnissen und Grenzen wie im
  Editor, einschließlich Verwaltung, Nebeninformationen und Wiederherstellung.
- Der Save-Editor benutzt die gemeinsamen Regeln; keine divergierende Kopie
  seines Speicherplaners bleibt in der CLI bestehen.
- Keine Funktion benötigt den gestarteten Editor als Ausweichlösung.
- Die CLI ist ohne Editorinstallation nutzbar, mit Katalogen und Codec
  vollständig paketiert und mit Hilfe/Beispielen dokumentiert.
- Alle verpflichtenden Abnahmeszenarien sind bestanden. Unfertige, bewusst
  ausgelassene oder nur geplante Funktionen verhindern die Paritätsfreigabe.
