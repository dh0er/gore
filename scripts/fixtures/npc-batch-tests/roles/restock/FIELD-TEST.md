# Haendlernachschub testen

**Profil 4 → `NPC 06 Nachschub - START` (Slot 067).**
Es ist 12:00 Uhr. A steht am bisherigen Testplatz bei Xardas' Turm.
Kein Aufbau noetig. Sprich A an; getestet wird **sein Handelsbestand**.

| Schritt | Was du machen sollst | Erwartung | Danach neu speichern als |
|---|---|---|---|
| 1 | **01 Handel: Bestand ansehen**. Noch nichts kaufen oder verkaufen. | A: **3 Kaese, 10 Pfeile, 100 Erz**. | `nachschub-vorher` |
| 2 | **02 Nachschub** waehlen, A erneut ansprechen und **01** oeffnen. | A: **5 Kaese, 15 Pfeile, 120 Erz**. 02 ist weg; stattdessen gibt es 03. | `nachschub-geliefert` |
| 3 | **03 dieselbe Lieferung erneut melden**, dann wieder **01**. | Weiterhin **5 Kaese, 15 Pfeile, 120 Erz**. Dieselbe Lieferung darf nicht noch einmal dazukommen. | `nachschub-wiederholt` |
| 4 | Spiel vollstaendig beenden und neu starten. **`nachschub-wiederholt` laden**, dann **01**. Erst Bestand pruefen, danach **1 Kaese kaufen**. | Vor Kauf **5/15/120**, danach **4 Kaese, 15 Pfeile**; As Erz steigt um den Kaufpreis. Erzmenge merken. | `nachschub-gehandelt` |
| 5 | Wieder vollstaendig beenden und neu starten. **`nachschub-gehandelt` laden**, dann **01**. | A hat weiterhin **4 Kaese, 15 Pfeile und exakt die gemerkte Erzmenge**. Du behaeltst den gekauften Kaese. 02 bleibt verschwunden. | `nachschub-neustart` |

Zum Speichern Handel und Gespraech beenden. Nach dem ersten Start immer den
jeweils genannten Ergebnisstand laden, nicht erneut den START-Stand.
Wenn ein Bestand abweicht: diesen Zustand speichern, die Zahlen melden und
dort stoppen. Ich lese die Save-Namen selbst aus; Slotnummern musst du nicht notieren.
