# Koepfe, Haare und Bartwechsel

`NpcHeadPaletteTest 0.1.3` korrigiert den Bartwechsel auf C:
ein bartloser Heldenkopf und ein Flex-Kopf mit kurzem Bart. H5/H6 wechseln
jetzt Gesichtstexturen in Cs eigenen Materialinstanzen. **Der Spieltest dieser
neuen Korrektur steht aus.** 0.1.2 zeigte schwarze Gesichter; beim Heldenkopf
funktionierte auch die Wiederherstellung nicht. Die zuvor getesteten Kopf-, Haar- und Farbwechsel
bleiben erhalten; der [Befund zu 0.1.0](runtime-result.json) bleibt als Historie.

## Spieltest bei Mittag

Dieses Paket zusammen mit **NpcBeardTextures013** aktivieren, alle anderen
Testpakete deaktivieren. Das Spiel **vollstaendig beenden und neu starten**.
In **Profil 4** den sauberen Spielstand
**065 "NPC 01 Koepfe - START"** laden. A ansprechen und **H0 Aufbau** waehlen.
Das setzt **12:00 Uhr** und stellt Held, A und C vor Xardas' Turm.
A erneut ansprechen, um jeweils eine Option zu waehlen. Nicht mit einem schwarzen
Ergebnissave aus 0.1.2 beginnen: bereits veraenderte Originalinstanzen sind kein
geeigneter Ausgangspunkt fuer den Wiederherstellungstest.

1. **H2 Heldenkopf**, dann **H5 Bart aus**: C soll sichtbar rasiert sein.
   Aus der Naehe vorn/seitlich ansehen, auch waehrend einer Gesichtsbewegung.
   Der Spielerheld und A/B muessen ihren urspruenglichen Kopf behalten.
   Neu speichern: **"npc bart vt - held ohne"**.
2. **H6 Bart an**: Cs urspruenglicher Heldenbart kommt zurueck.
   H5/H6 nochmals wechseln; keine zusaetzlichen Koepfe oder Abstuerze.
3. **H1 Flex**, dann **H6 Bart an**: kurzer Bart auf Cs Flex-Gesicht.
   **H5 Bart aus** stellt Flex' urspruengliches bartloses Gesicht wieder her.
   Erneut H6, neu speichern: **"npc bart vt - flex mit"**.
4. Spiel vollstaendig beenden, beide neuen Saves nacheinander laden:
   Auswahl und Gesicht bleiben erhalten. H5/H6 auch nach dem Laden pruefen.
5. **H9 Alles wiederherstellen**: urspruenglicher Heldenkopf mit Originalbart,
   Haaren und Farben, ohne Absturz. Neu speichern: **"npc bart vt - original"**.

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

0.1.2 importierte PNGs als gewoehnliche Texture2D-Ressourcen. Beide originalen
Gesichtstexturen sind jedoch gekachelte Virtual Textures. Diese Inkompatibilitaet
ist belegt und die wahrscheinliche Ursache der schwarzen Darstellung; der
konkrete GPU-Fallback wurde nicht untersucht. 0.1.3 kocht die vorhandenen Bilder
mit dem jeweiligen Original als Vorlage zu eigenen Virtual-Texture-Assets.
`texture replace --as-asset ... --fit-original` bewahrt Format und VT-Struktur,
skaliert auf 2048x2048 und vergibt neue Paketidentitaeten. Der Manager verwaltet
beide Zen-Triplets im Begleitpaket NpcBeardTextures013. `LoadObject` laedt die
neuen Assets unter `/Game/GoreMods/NpcBeardSwitch/`; Originaltexturen bleiben
unveraendert. PNG-Import wird fuer diese Gesichtsmaterialien nicht mehr benutzt.

Hero-rasiert setzt die exakt aus dem Material gelesenen Parameter
`1 - Albedo (Alpha Cutout)`, `5 - WM1 Albedo` und `6 - WM2 Albedo` auf die neue
Textur und blendet zusaetzliche Bartgeometrie aus. Die drei Normalmaps bleiben
erhalten: Mimikfalten bestehen, die unterschiedlichen Farbdetails der beiden
originalen WM-Albedos entfallen in dieser rasierten Testvariante.
Flex-mit-Bart ersetzt nur Parameter 1; dieses Material deaktiviert animierte
Albedos und Normalmaps ausdruecklich. Die Varianten testen einen gemalten kurzen
Bart; sie fuegen keine neue lange Bartgeometrie hinzu.

Die bisherige Komponenten-Factory konnte die bereits vorhandene Helden-MID
zurueckgeben und damit auch das gespeicherte Original veraendern. Die neue
Material-Factory erzeugt stets eine getrennte MID. Bei einem dynamischen
Original verwendet sie dessen statischen Parent und kopiert vor der Aenderung
seine Parameter-Overrides. Erst danach wird sie an C gebunden.
Pro Slot bleiben Originalmaterial und abgeleitete MID getrennt gespeichert.
Zurueckschalten setzt die Originalreferenz ein, wiederholtes Anwenden verwendet
dieselbe MID. Neu erzeugte Darstellungen erhalten eigene Datensaetze. Nach dem
Neuladen werden Auswahl, gekochte Texturen und Instanzen wieder aufgebaut. Neue Felder stehen
hinter dem bisherigen Controller-Praefix; bewaehrte Pose/Knochen- und BFG-
Korrekturen bleiben bestehen. Lip Sync bleibt ausgenommen.

Die [Texturbelege](beard-textures.json) enthalten Materialparameter, Quell- und
Bildhashes sowie die Bearbeitungsprompts. Die [API-Qualifikation](beard-native-api-qualification.json)
belegt die ersten nativen Aufrufe; die [MID-Ergaenzung](fresh-mid-native-api-qualification.json)
qualifiziert Factory und Override-Kopie. Die [VT-Pruefung](beard-virtual-textures.json)
belegt neue Paketidentitaeten und unveraendertes Ruecklesen. Der [Fehlversuch](beard-runtime-0.1.2.json)
bleibt dokumentiert. Offline-Build und Manager-Status
beweisen noch keine korrekte Darstellung im Spiel.
