# Koepfe, Haare und Bartwechsel

`NpcHeadPaletteTest 0.1.2` ergaenzt den tatsaechlichen Bartwechsel auf C:
ein bartloser Heldenkopf und ein Flex-Kopf mit kurzem Bart. H5/H6 wechseln
jetzt Gesichtstexturen in Cs eigenen Materialinstanzen. **Der Spieltest dieser
neuen Umschaltung steht aus.** Die zuvor getesteten Kopf-, Haar- und Farbwechsel
bleiben erhalten; der [Befund zu 0.1.0](runtime-result.json) bleibt als Historie.

## Spieltest bei Mittag

Nur dieses Paket aktivieren. In **Profil 4** den Spielstand
**065 "NPC 01 Koepfe - START"** laden. A ansprechen und **H0 Aufbau** waehlen.
Das setzt **12:00 Uhr** und stellt Held, A und C vor Xardas' Turm.
A erneut ansprechen, um jeweils eine Option zu waehlen.

1. **H2 Heldenkopf**, dann **H5 Bart aus**: C soll sichtbar rasiert sein.
   Aus der Naehe vorn/seitlich ansehen, auch waehrend einer Gesichtsbewegung.
   Der Spielerheld und A/B muessen ihren urspruenglichen Kopf behalten.
   Neu speichern: **"npc bart - held ohne"**.
2. **H6 Bart an**: Cs urspruenglicher Heldenbart kommt zurueck.
   H5/H6 nochmals wechseln; keine zusaetzlichen Koepfe oder Abstuerze.
3. **H1 Flex**, dann **H6 Bart an**: kurzer Bart auf Cs Flex-Gesicht.
   **H5 Bart aus** stellt Flex' urspruengliches bartloses Gesicht wieder her.
   Erneut H6, neu speichern: **"npc bart - flex mit"**.
4. Spiel vollstaendig beenden, beide neuen Saves nacheinander laden:
   Auswahl und Gesicht bleiben erhalten. H5/H6 auch nach dem Laden pruefen.
5. **H9 Alles wiederherstellen**: urspruenglicher Heldenkopf mit Originalbart,
   Haaren und Farben, ohne Absturz. Neu speichern: **"npc bart - original"**.

Nicht alle bisherigen Haar-/Kopftests muessen wiederholt werden. H3/H4 steuern
weiter die Haare, H7/H8 Flex' Haarfarbe. Die Bartauswahl bleibt bei H1/H2 bestehen;
H9 setzt sie auf das jeweilige Original zurueck. Bei einem fehlenden Wechsel
trotzdem unter dem passenden Namen speichern; die Lade- und Materialzaehler
werden mitgespeichert. START und vorhandene Ergebnissaves nicht ueberschreiben.

## Ursache und Umsetzung

Der Hero-Bart ist in `T_NH_Head_D` und beiden animierten Albedos `WM1_D`/`WM2_D`
eingezeichnet. Das fruehere Ausblenden von `MI_NH_Beard` entfernt diese Haare
nicht. Flex besitzt kein entsprechendes separates Bartmaterial. Deshalb waren
H5/H6 in 0.1.1 voruebergehend verborgen.

0.1.2 liefert zwei bearbeitete Albedo-PNGs in einem additiven, vom Manager
verwalteten Pak. Die gebundene Funktion `Rendering::ImportFileAsTexture2D`
laedt sie aus `FPaths::ProjectContentDir()/GoreMods/NpcBeardSwitch/`.
Nur Cs Komponenten bekommen neue MIDs; gemeinsame Originalmaterialien und
urspruengliche Spieltexturen werden nicht veraendert.

Hero-rasiert setzt die exakt aus dem Material gelesenen Parameter
`1 - Albedo (Alpha Cutout)`, `5 - WM1 Albedo` und `6 - WM2 Albedo` auf die neue
Textur und blendet zusaetzliche Bartgeometrie aus. Die drei Normalmaps bleiben
erhalten: Mimikfalten bestehen, die unterschiedlichen Farbdetails der beiden
originalen WM-Albedos entfallen in dieser rasierten Testvariante.
Flex-mit-Bart ersetzt nur Parameter 1; dieses Material deaktiviert animierte
Albedos und Normalmaps ausdruecklich. Die Varianten testen einen gemalten kurzen
Bart; sie fuegen keine neue lange Bartgeometrie hinzu.

Pro Slot bleiben Originalmaterial und abgeleitete MID getrennt gespeichert.
Zurueckschalten setzt die Originalreferenz ein, wiederholtes Anwenden verwendet
dieselbe MID. Neu erzeugte Darstellungen erhalten eigene Datensaetze. Nach dem
Neuladen werden Auswahl, PNGs und Instanzen wieder aufgebaut. Neue Felder stehen
hinter dem bisherigen Controller-Praefix; bewaehrte Pose/Knochen- und BFG-
Korrekturen bleiben bestehen. Lip Sync bleibt ausgenommen.

Die [Texturbelege](beard-textures.json) enthalten Materialparameter, Quell- und
Bildhashes sowie die Bearbeitungsprompts. Die [API-Qualifikation](beard-native-api-qualification.json)
belegt die eng zugelassenen nativen Aufrufe. Offline-Build und Manager-Status
beweisen noch keine korrekte Darstellung im Spiel.
