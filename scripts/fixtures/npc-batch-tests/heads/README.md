# Koepfe, Haare und Haarfarbe

Der Nutzer hat `NpcHeadPaletteTest 0.1.0` getestet: Kopfwechsel, Halsverbindung,
Haare aus/an, Flex-Haarfarbe, Wiederherstellung, Speichern/Laden, vollstaendiger
Neustart und wiederholter Kopfwechsel funktionieren. **Bart aus/an funktioniert
sichtbar nicht.** Siehe [Laufzeitbefund](runtime-result.json).

## Ursache des Bartbefunds

Der sichtbare Schnurr-, Wangen- und Kinnbart ist bereits in der Helden-
Gesichtstextur `T_NH_Head_D` eingezeichnet. H5/H6 adressierten nur das zusaetzliche
Material `MI_NH_Beard`. Im Save „npc koepfe - haare und bart aus“ sind der
Aus-Flag und fuenf Materialtreffer vorhanden; das beweist keine Rasur der
Gesichtstextur. Flex besitzt kein entsprechendes separates Bartmaterial.

Die Bezeichnung „Bart aus“ war irrefuehrend. Ab Version **0.1.1** sind H5/H6
unsichtbar; ihre Klassen und gespeicherten Felder bleiben kompatibel.
Fuer einen wirklich bartlosen Heldenkopf und separat wechselbare Baerte bleiben
eigene passende Gesichtstexturen und deren Zuordnung pro NPC offen. Ein
funktionierender Schalter dafuer ist durch diese Fixture nicht nachgewiesen.

## Verbleibende Steuerung

Nur `NpcHeadPaletteTest` aktivieren. In **Profil 4** den Spielstand
**065 „NPC 01 Koepfe - START“** laden. A ansprechen und **H0 Aufbau** waehlen.
Ab 0.1.1 setzt der Aufbau **12:00 Uhr** und stellt Held, A und C an die bekannten
Punkte vor Xardas' Turm. A erneut fuer die weiteren Optionen ansprechen.

- **H1/H2:** Flex-/Heldenkopf.
- **H3/H4:** Haare aus/an.
- **H7/H8:** Flex-Haarfarbe rot/original.
- **H9:** Heldenkopf und urspruengliche Sichtbarkeit/Farbe wiederherstellen.

Die bestandenen Kopf-Tests muessen fuer den naechsten Sprachtest nicht erneut
abgearbeitet werden. Die alten Ergebnissaves bleiben erhalten; zum erneuten
Pruefen den eigenen START-Stand verwenden.

## Umsetzung

Der poseable Kopf uebernimmt die Koerperpose, setzt nur die Gesichtsknochen
unterhalb des Kopfes auf Flex' Grundpose zurueck und nimmt C vom inkompatiblen
BFG-Interpolationspfad aus. Cs eigene Materialinstanzen steuern die Haarfarbe;
gemeinsam genutzte Materialien bleiben unveraendert. Gespeicherte Auswahlwerte
werden nach dem Neuladen oder Erneuern der Darstellung angewendet.

Die Korrektur 0.1.1 betrifft nur Mittag und das Entfernen der irrefuehrenden
Menueoptionen. Die getestete Darstellung bleibt gleich. Lip Sync bleibt ausgenommen.
