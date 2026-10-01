# Paket 5: Proviant fuer die Wache

**Abgeschlossen: Alle sieben Schritte wurden am 2026-09-21 vom Benutzer
mit Version0.1.2 auf Steam-Hotfix25168047 als bestanden bestaetigt.**
Siehe [Ergebnisbericht](runtime-0.1.2.json). Die Liste bleibt zur Reproduktion
erhalten; sie muss jetzt nicht nochmals getestet werden.
Der Startstand und die Ergebnisse liegen [im externen Archiv](../profile-cleanup-after-quest-complete.json).

Fuer einen spaeteren Nachtest zuerst den archivierten Startstand wiederherstellen
und das passende Paket aktivieren. Dann **Profil4, 071 „NPC 05 Quest - START“ laden.** Der Start ist auf Mittag
gestellt. **Proviantmeister A steht direkt vor dir.**
**Q1 bis Q7 sind die Beschriftungen der Gespraechsoptionen bei A, keine
Questnummern. Es geht um eine einzige Quest: „Proviant fuer die Wache“.**
Welche Gespraechsoptionen A anbietet, aendert sich mit deinem Fortschritt.
„Q2 erscheint“ bedeutet: Beim erneuten Ansprechen von A ist die Zeile
„Q2 Abgemacht: zwei Kaese fuer 25 Erz“ im Dialogmenue waehlbar.
Du musst sie dazu noch nicht ausgewaehlt haben.
Es ist kein neuer Aufbau erforderlich. Alle folgenden Ergebnisstaende unter
neuem Namen speichern; besonders `quest-lieferung` fuer den zweiten Zweig behalten.

Nach jeder Auswahl den Dialog schliessen lassen und einige Sekunden warten,
bevor du Journal oder Menue pruefst. Bei einem Fehler an dieser Stelle stoppen
und den Befund melden; den Fortschritt nicht erzwingen.

1. A ansprechen und **Q1 Auftrag: Proviant fuer die Wache** waehlen.
   Damit nimmst du die eine Quest an. Im aktiven Journal sollen der eigene
   Titel, die zugesagten **25 Erz** und das Ziel **„Sprich die Lieferung mit dem
   Proviantmeister ab“** stehen. Kein fremder Brieftext.
   **A erneut ansprechen:** Die Gespraechsoption Q1 darf nicht mehr angeboten
   werden; stattdessen soll die Gespraechsoption Q2 waehlbar sein.
   **Q2 jetzt noch nicht waehlen.** Das Gespraech verlassen und als
   **`quest-absprache`** speichern.
2. Spiel vollstaendig beenden, neu starten und **`quest-absprache`** laden.
   Im Journal muss weiterhin dieselbe angenommene Quest stehen.
   **A erneut ansprechen:** Er muss weiterhin die noch nicht gewaehlte
   Gespraechsoption **Q2 Abgemacht: zwei Kaese fuer 25 Erz** anbieten.
   **Jetzt Q2 waehlen.** Das erste Ziel wird automatisch erledigt; **„Gib dem Proviantmeister
   zwei Kaese“** beginnt. Der Journaleintrag **„Die Lieferung ist abgesprochen“**
   kommt hinzu.
3. A erneut ansprechen und **Q3 Testvorrat: zwei Kaese nehmen (einmalig)** waehlen.
   Der Held bekommt genau **zwei Kaese**, kein Erz. Beim naechsten Ansprechen
   darf A die Gespraechsoption Q3 nicht mehr anbieten. Kaese- und Erzmenge notieren und
   als **`quest-lieferung`** speichern. Diesen Stand nicht ueberschreiben.
4. Spiel vollstaendig beenden, neu starten und **`quest-lieferung`** laden.
   Mengen unveraendert, zweites Ziel aktiv. A ansprechen: Die Gespraechsoption
   Q3 darf weiterhin nicht angeboten werden. Im Dialog
   **Q4 Hier sind die zwei Kaese** waehlen: genau **zwei Kaese weniger und
   25 Erz mehr** beim Helden. Beide Ziele und die Quest werden automatisch
   abgeschlossen; ein passender Abschlussabsatz erscheint. A erneut ansprechen:
   Die Gespraechsoptionen Q4/Q5 sind weg, Q6 ist waehlbar.
   Gespraech verlassen und als **`quest-erfolg`** speichern.
5. A wiederholt ansprechen und **Q6 Die Lieferung ist bezahlt. Danke.**
   mehrmals waehlen: keine weiteren
   Gegenstaende oder Erz. Spiel vollstaendig beenden, neu starten und
   **`quest-erfolg`** laden. Abschluss, Journal und Mengen bleiben erhalten;
   A ansprechen und die Gespraechsoption Q6 nochmals probieren.
   Als **`quest-erfolg-neustart`** speichern.
6. Jetzt den behaltenen Zwischenstand **`quest-lieferung`** laden.
   A ansprechen und **Q5 Ich sage die Lieferung ab (Auftrag scheitert)** waehlen. Quest und
   zweites Ziel scheitern; das erste Ziel bleibt erledigt. Ein Abbruchabsatz
   erscheint. Keine Abgabe, keine Belohnung: Mengen wie in Schritt 3.
   A erneut ansprechen: Die Gespraechsoptionen Q3/Q4/Q5 sind weg, Q7 ist
   waehlbar. Gespraech verlassen und als **`quest-abbruch`** speichern.
7. Spiel vollstaendig beenden, neu starten und **`quest-abbruch`** laden.
   Gescheiterte Quest, Journal und Mengen bleiben erhalten.
   A ansprechen: Die Gespraechsoption Q7 ist weiterhin waehlbar. Q7 waehlen:
   weiterhin keine Abgabe oder Belohnung. Als **`quest-abbruch-neustart`** speichern.

Bitte Abweichungen und die Ergebnisse melden. Die Slots leite ich aus den
genannten Save-Namen ab. Die technischen Erwartungen stehen im [Quest-Guide](README.md).
