# Paket 5: Proviant fuer die Wache

**Profil 4, 071 „NPC 05 Quest - START“ laden.** Der Start ist auf Mittag
gestellt. **Proviantmeister A steht direkt vor dir.** Mit ihm sprechen;
die Optionen beginnen mit Q1 bis Q7.
Es ist kein neuer Aufbau erforderlich. Alle folgenden Ergebnisstaende unter
neuem Namen speichern; besonders `quest-lieferung` fuer den zweiten Zweig behalten.

Nach jeder Auswahl den Dialog schliessen lassen und einige Sekunden warten,
bevor du Journal oder Menue pruefst. Bei einem Fehler an dieser Stelle stoppen
und den Befund melden; den Fortschritt nicht erzwingen.

1. **Q1 Auftrag: Proviant fuer die Wache.** Im aktiven Journal sollen der eigene
   Titel, die zugesagten **25 Erz** und das Ziel **„Sprich die Lieferung mit dem
   Proviantmeister ab“** stehen. Kein fremder Brieftext. Q1 verschwindet, Q2
   erscheint. Als **`quest-absprache`** speichern.
2. Spiel vollstaendig beenden, neu starten und **`quest-absprache`** laden.
   Journal und Q2 muessen erhalten sein. **Q2 Abgemacht: zwei Kaese fuer 25 Erz**
   waehlen. Das erste Ziel wird automatisch erledigt; **„Gib dem Proviantmeister
   zwei Kaese“** beginnt. Der Journaleintrag **„Die Lieferung ist abgesprochen“**
   kommt hinzu.
3. **Q3 Testvorrat: zwei Kaese nehmen (einmalig).** Der Held bekommt genau
   **zwei Kaese**, kein Erz. Q3 verschwindet. Kaese- und Erzmenge notieren und
   als **`quest-lieferung`** speichern. Diesen Stand nicht ueberschreiben.
4. Spiel vollstaendig beenden, neu starten und **`quest-lieferung`** laden.
   Mengen unveraendert, zweites Ziel aktiv, Q3 weiterhin weg.
   **Q4 Hier sind die zwei Kaese** waehlen: genau **zwei Kaese weniger und
   25 Erz mehr** beim Helden. Beide Ziele und die Quest werden automatisch
   abgeschlossen; ein passender Abschlussabsatz erscheint. Q4/Q5 verschwinden,
   Q6 erscheint. Als **`quest-erfolg`** speichern.
5. **Q6 Die Lieferung ist bezahlt. Danke.** mehrmals waehlen: keine weiteren
   Gegenstaende oder Erz. Spiel vollstaendig beenden, neu starten und
   **`quest-erfolg`** laden. Abschluss, Journal und Mengen bleiben erhalten;
   Q6 nochmals probieren. Als **`quest-erfolg-neustart`** speichern.
6. Jetzt den behaltenen Zwischenstand **`quest-lieferung`** laden.
   **Q5 Ich sage die Lieferung ab (Auftrag scheitert)** waehlen. Quest und
   zweites Ziel scheitern; das erste Ziel bleibt erledigt. Ein Abbruchabsatz
   erscheint. Keine Abgabe, keine Belohnung: Mengen wie in Schritt 3.
   Q3/Q4/Q5 verschwinden, Q7 erscheint. Als **`quest-abbruch`** speichern.
7. Spiel vollstaendig beenden, neu starten und **`quest-abbruch`** laden.
   Gescheiterte Quest, Journal, Q7 und Mengen bleiben erhalten. Q7 waehlen:
   weiterhin keine Abgabe oder Belohnung. Als **`quest-abbruch-neustart`** speichern.

Bitte Abweichungen und die Ergebnisse melden. Die Slots leite ich aus den
genannten Save-Namen ab. Die technischen Erwartungen stehen im [Quest-Guide](README.md).
