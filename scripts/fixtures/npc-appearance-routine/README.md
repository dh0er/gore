# Aussehen und Tagesablauf

## Bestätigt: Lesen, Trinken und Tageswechsel

`NpcActivitiesProof` 0.1.3 hat den Spieltest nicht bestanden: B blieb stehen und
führte keine Tätigkeit aus. Im Cache fehlten die Ereignisbindungen der neuen
AI-Klasse. `NpcActivitiesEventFix` 0.1.4 korrigiert nur diese zwei Deklarationen
und hat am 2026-09-07 auch den vollständigen Spieltest bestanden: Lesen nach
dem Aufbau, Hinweg und Trinken ab12:00, Fortsetzung nach vollständigem Neustart
ohne erneuten Aufbau, Rückweg und Lesen ab18:00 sowie Lesen am nächsten Morgen.
Build-/Installationsnachweise stehen in
[BUILD.md](BUILD.md), Diagnose und Ergebnisse in [RESULTS.md](RESULTS.md).

### Bestandenen Aktivitätentest reproduzieren

1. **Slot39 „npc aktivitaet - morgen“ laden**. Bei A **„09 Aufbau (Lesen und Trinken)“**
   wählen. B soll am äußeren Startpunkt lesen. Bis zu30 Sekunden beobachten.
   Falls B schon hier untätig bleibt, einen neuen Save anlegen und den Test
   beenden; die weiteren Schritte sind dann nicht nötig.
2. Bei A **„10 Uhr auf 11:59 - zum Trinken“** wählen. Gespräch verlassen, B den
   Weg freihalten. Ab12:00 soll er zum zweiten Punkt laufen und dort trinken.
   Nach Ankunft bis zu30 Sekunden beobachten. Als **„NPC Aktivitaet Fix - Mittag“**
   neu speichern; alte Saves behalten.
3. Spiel vollständig beenden, neu starten und diesen Save laden. **Keinen Aufbau
   wählen.** B soll am Mittagsplatz wieder trinken.
4. Bei A **„11 Uhr auf 17:59 - zurueck zum Lesen“** wählen. Ab18:00 soll B zum
   ersten Punkt zurücklaufen und wieder lesen. Als **„NPC Aktivitaet Fix - Abend“**
   neu speichern.
5. Zur Morgenprobe **„12 Uhr auf 08:00 - naechster Morgen“**
   wählen. B soll am ersten Punkt bleiben und lesen. Als
   **„NPC Aktivitaet Fix - Morgen“** speichern.

Ergebnis je Tätigkeit/Wechsel nennen. Bei einem Fehler reicht der erste
fehlgeschlagene Schritt mit einem neuen Save; die Slotnummern werden aus den
Save-Namen gelesen. Sichtbare Bewegung und sichtbare Tätigkeit getrennt melden.

Die direkten Aktionen `Action_Conversation_ReadBook` und
`Action_Conversation_Drink` sind für menschliche NPCs mit `bPossibleAnywhere`
registriert. Die Routine ruft sie nach `GotoPreferredLocation` auf und wiederholt
sie mit jeweils zwei Sekunden Pause. Der Nutzer beobachtete kurze Aktionen von
etwa drei Sekunden: Buch beziehungsweise Flasche herausnehmen, benutzen,
wegstecken und erneut beginnen. Das entspricht der Schleife im Testscript;
die übergebenen20 Sekunden sind eine Höchstdauer pro Aufruf, keine erzwungene
Animationslänge. Eine durchgehende längere Leseanimation ist damit nicht geprüft.
Ein Zeitwechsel beendet den alten Zustand über den vorhandenen
Graceful-Exit-Mechanismus. Die Uhrzeitoptionen versetzen B nicht ausdrücklich;
nur09 setzt die Ausgangsposition und ersetzt seine gespeicherte Routine.

Dies prüft freie Tagesaktivitäten. Wachplätze, Möbel, Schlafen und Arbeiten mit
Interaktionsobjekten sind damit noch nicht getestet. B/C behalten die bewiesene
Kleidung; ein neuer Kopf ist in diesem Paket nicht enthalten.

## Bestätigter Vorgänger: Gehen

Die Kleidung von B/C, Laden, C ohne Duplikate und eine bisherige Voice-Option
haben bestanden. Die fehlgeschlagenen Bewegungen und Saves031–034 stehen in
[RESULTS.md](RESULTS.md). **Mit `NpcRoutineWalkingProof` 0.1.2 ist B beim
natürlichen Wechsel auf12:00 sichtbar zum erwarteten Punkt gelaufen.**
Der zugehörige Save ist **Slot36 „npc lauf - mittag“**. Position, Routine und
weiterhin genau ein C wurden im Save überprüft.

### Bestandenen Lauftest mit0.1.2 reproduzieren

1. Spiel starten und **Slot34 „aufbau erneut“ laden**.
2. Bei A **„09 Aufbau (Lauftest)“** wählen. B soll am äußeren Startpunkt stehen.
3. Bei A **„10 Uhr auf 11:59 - Wechsel beobachten“** wählen. Gespräch verlassen,
   B im Blick behalten und seinen Weg freihalten. Beim natürlichen Wechsel
   auf12:00 soll er zum Platz laufen, auf den der Held beim Aufbau versetzt wurde.
   Bis zu einer Minute warten. Sichtbares Gehen und plötzliches Versetzen unterscheiden.
4. Einen neuen Save **„NPC Lauf - Mittag“** anlegen. Ergebnis und neue Slotnummer
   nennen; die bisherigen Saves behalten. Falls B stillsteht, genügt genau diese
   Beobachtung. Die übrigen Uhrzeitoptionen sind für diesen ersten Nachtest unnötig.

Der damalige0.1.2-Test erwartete Gehen und anschließendes Stehen. Wach- und
Trinkanimationen waren nicht Teil dieses Durchlaufs. Ein direkter Zeitsprung kann die native
Zeitraffersimulation auslösen und ist deshalb kein Beleg für sichtbares Laufen.
Der Rückweg am Abend/Morgen und ein Neustart aus Slot36 wurden in diesem
Nachtest nicht separat geprüft; der bestätigte Lauf betrifft den Mittagswechsel.

### Änderung im Vorgänger0.1.2

Die bisherigen Wach-/Trink-Zustände suchen passende Interaktionsplätze. Die
verwendeten Navigationspunkte bieten ihnen keine Aktionen im vorgesehenen Radius.
B verwendete in0.1.2 den vorhandenen Spielzustand `UAIState_TestGotoWP`, der direkt
`GotoPreferredLocation` aufruft. Die Route läuft zwischen zwei im Spiel draußen
bestätigten Punkten: morgens393, mittags `FP_XT_WAIT_OUTSIDE`, abends393.
Zeitversatz bleibt null, Radius100 cm, Routine-Teleportmodus `Never`.

Nur der Aufbau versetzt B ausdrücklich und ersetzt seine gespeicherte Routine.
Die Uhrzeitoptionen ändern ausschließlich die Spielzeit. A/C bleiben an392/391;
die bestehenden Quest-, Kleidungs- und Voice-Inhalte bleiben im Gesamtpaket.

## Warum der Heldenkopf erscheint

B/C wählen ausdrücklich `MO_Player`, `Person=Hero`, `Head=Head_01`. Die geprüften
Kleidungsparameter dieses Modells funktionieren. Bestehende vollständige NPC-Looks
lassen sich über deren visuelle Definition übernehmen, wie bereits bei A.

Die Spielskripte enthalten auch Kopf-, Haar-, Bart- und Farbparameter für NPCs.
Viele ausgelieferte NPCs verwenden aber fertig erzeugte Gesamtmodelle. Unabhängige
NPC-Gesichter zusammen mit frei gewählter Kleidung sind deshalb noch nicht
nachgewiesen. Vorhandene GTO-Pakete sind untersucht beziehungsweise in
[RESULTS.md](RESULTS.md) abgegrenzt; `MO_Characters` wurde im Basiscontainer nicht gefunden.

[controls.as](controls.as) enthält As Steuerung, [npc-b-c.as](npc-b-c.as) B/C.
Offline-/Installationsnachweise stehen in [BUILD.md](BUILD.md). Das Spiel wird
weiterhin ausschließlich vom Nutzer getestet.
