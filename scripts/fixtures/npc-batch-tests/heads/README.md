# Koepfe, Haare, Bart und Haarfarbe

Nur `NpcHeadPaletteTest` aktivieren. In **Profil 4** den Spielstand
**065 „NPC 01 Koepfe - START“** laden. Er stammt aus dem Stand vor Cs Erstellung.
A ansprechen und **H0 Aufbau** waehlen. Held, A und C stehen danach an den
bekannten Punkten vor Xardas' Turm. A erneut fuer die weiteren Optionen ansprechen.
Den START-Stand behalten und Ergebnisse unter den unten genannten Namen speichern.

## Testreihenfolge

1. **H0**, danach **H1**: Flex-Kopf, Halsverbindung und normale Koerperbewegung
   pruefen.
2. **H3** blendet Haare aus; Augenbrauen, Augen, Gesicht und Koerper bleiben
   sichtbar. **H4** stellt die Haare wieder her. Einmal wiederholen und von
   vorne, seitlich und hinten ansehen.
3. **H7** faerbt Flex' Haare rot; **H8** stellt die urspruengliche Farbe wieder
   her. Noch einmal H7 waehlen und als **`kopf-farbe`** speichern. Spiel ganz
   beenden, neu starten und diesen Stand laden. Farbe und Halsverbindung muessen
   erhalten sein; weiterhin nur ein Kopf. H8 muss die Farbe wiederherstellen.
4. **H2** waehlt den Heldenkopf. **H5** entfernt dessen separate Bart-Geometrie,
   **H6** stellt sie wieder her. Aufgemalte Stoppeln koennen bleiben.
   **H3/H4** auch bei den Haaren des Heldenkopfs pruefen.
5. Beim Heldenkopf Haare und Bart ausschalten (**H3, H5**). Als **`kopf-teile`**
   speichern, Spiel ganz beenden und neu laden. Beide bleiben ausgeschaltet.
   **H9** stellt Heldenkopf, Haare und Bart ohne Absturz wieder her.
   Als **`kopf-standard`** speichern.
6. **H1 zweimal** waehlen: weiterhin genau ein korrekt verbundener Flex-Kopf.
   **H9** muss wieder zum Heldenkopf zurueckkehren.

Bitte sichtbare Ergebnisse und die benannten Saves melden. Sichtbare Kappen
oder Naehte nach dem Ausblenden der Haare ebenfalls melden. Flex' Stoppeln sind
Teil der Gesichtstextur; H5/H6 testen die separate Bart-Geometrie des Helden.
Die Kleidung bleibt gleich. Lip Sync ist ausgenommen.

## Umsetzung und Grenzen

Der bereits getestete poseable Kopf uebernimmt die Koerperpose, setzt nur die
Gesichtsknochen unterhalb des Kopfes auf Flex' Grundpose zurueck und nimmt C vom
inkompatiblen BFG-Interpolationspfad aus. Die Kopfabschnitte des Originalkoerpers
bleiben ausschliesslich im Flex-Modus verborgen.

Die Haarfarbe verwendet Cs eigene dynamische Materialinstanzen und die am
Originalmaterial geprueften Parameter `Root Color` und `Tip Color`. Andere NPCs
und gemeinsam genutzte Materialien werden dadurch nicht umgefaerbt. Gespeicherte
Auswahlwerte werden nach dem Neuladen oder Erneuern der Darstellung angewendet.

Build und API-Pruefung sind bestanden. Ob Materialabschnitte und Farben im
Spiel korrekt erscheinen, bleibt Gegenstand dieses Tests. Die Fixture prueft
vorhandene Spielgeometrie; sie erstellt keine beliebigen neuen Meshes.
