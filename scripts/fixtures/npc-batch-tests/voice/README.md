# Natuerliche Stimmen: Annaeherung und Alltag

Status: **0.1.1 aktiv**, nachdem der Kopf-/Barttest 0.1.3 bestanden wurde;
natuerliche Begruessung und Alltag sind noch nicht im Spiel geprueft.
Die [Aktivierung](deployment-after-heads.json) verwendet das bereits gepruefte,
unveraenderte Tageslichtpaket. Ausgangspunkt ist **Profil 4,
Slot 066, „NPC 02 Voice - START“**, die vorbereitete Kopie des inzwischen archivierten Slots
030. Andere Testmods werden einzeln getestet. Diese Variante
enthaelt weiterhin Bs Warnungsreparatur aus 0.1.12.

Diego bleibt das Stimmprofil von **B in der Gardistenkleidung**. **C in der
Novizenkleidung** bekommt das originale Lares-Profil. „Profil“ bezeichnet hier
die Zuordnung der Aufnahmen, nicht eine neue Aufnahme oder Sprecherimitation.

Ab Version 0.1.1 liegen die Routinewechsel bei 14:00 und 16:00 Uhr, damit
auch dieser Teil bei Tageslicht stattfindet.

## Ablauf

1. A ansprechen, **01 Natuerliche Stimmen: Aufbau mit B und C** waehlen.
   Der Aufbau setzt 12:00 Uhr. Held, A, B und C stehen danach draussen am Xardas-Turm. C steht etwas
   oberhalb von A, B unterhalb. A hat danach die Bereitschaftszeile **04**.
   Falls stattdessen „C schon vorhanden“ erscheint, den Ausgangsspielstand 066
   laden. Der Aufbau erzeugt C nur einmal; erneuter Aufbau mit unserem eigenen C
   ist erlaubt. Er ersetzt keinen fremden C aus einem aelteren Teststand.
2. **Vor der ersten Annaeherung** einen neuen Spielstand
   **„npc natuerlich - start“** anlegen. Nicht 066 ueberschreiben.
3. Mit leeren, gesenkten Haenden auf **B** zugehen. Ihn nicht ansprechen.
   Vor ihm in etwa **1–2 Metern** Abstand stehen und **10 Sekunden** beobachten.
   Falls er wegschaut, einmal vor sein Gesicht gehen. Erwarteter Trigger ist
   die Sicht auf einen befreundeten Helden innerhalb von 2,5 Metern. Eine kurze
   Begruessung sollte aus dem normalen Wahrnehmungssystem kommen.
4. Bei B stehen bleiben, ohne Dialog, Waffe, Schleichen oder Inventarmenue.
   **Hoechstens 95 Sekunden** auf eine selbstaendige Alltagszeile warten.
   Stille nach dieser Frist ist ein verwertbares negatives Ergebnis; nicht
   minutenlang weiterwarten. Begruessung und spaetere Alltagszeile getrennt notieren.
5. Zu **C** gehen und Schritte 3–4 wiederholen. Beobachten, ob die Stimme von B
   unterscheidbar ist. Nur eine sichtbar/hoerbar C zuordenbare Zeile als C-Ergebnis
   werten; auch A kann im Hintergrund die normale Alltagsfaehigkeit benutzen.
6. Optionaler Tagesablauf-Vergleich: bei A **02 13:59** waehlen und aus Bs Weg
   treten. Beim normalen naechsten Minutenwechsel laeuft B zum unteren Wartepunkt
   (der Ankunftsort des Helden beim Aufbau). **Bis 30 Sekunden** Bewegung
   beobachten; danach bis **95 Sekunden** normale Alltagsstimme beobachten.
   **03 15:59** fuehrt B auf dem normalen Minutenwechsel zurueck. Der Wechsel
   selbst verspricht keinen eigenen Spruch. C bleibt an seinem Platz.
7. Befunde in **„npc natuerlich - beobachtet“** speichern. Spiel vollstaendig
   beenden, neu starten und **„npc natuerlich - start“** laden. **Ohne erneut 01
   zu waehlen** B und C nochmals wie in 3–5 pruefen. So werden gespeicherter
   frischer C, Beziehung, Routine und Stimmzuordnung gemeinsam geprueft.

Begruessungen haben im Original einen **300-Sekunden-Wahrnehmungscooldown**.
Mehrfaches Hin-und-herlaufen ist kein sofortiger Wiederholungstest. Deshalb
wird fuer den Neustart der Stand **vor** den ersten Annaeherungen benutzt.
Uhrzeiten vorspulen ist ebenfalls kein zugesicherter Reset dieses Cooldowns.

## Rueckmeldung

| Fall | Beobachtung |
|---|---|
| B: Annaeherung / erste 10 Sekunden | Zeile oder still; wenn moeglich Wortlaut |
| B: Alltag / bis 95 Sekunden | Zeile oder still; Zeit bis zur Zeile |
| C: Annaeherung / erste 10 Sekunden | Zeile oder still; Stimme unterscheidbar? |
| C: Alltag / bis 95 Sekunden | Zeile oder still; Zeit bis zur Zeile |
| B: Tageswechsel | Bewegung ja/nein; danach Stimme ja/nein |
| Vollstaendiger Neustart | dieselben B-/C-Ergebnisse ohne erneuten Aufbau? |

Keine Taste fordert eine gesprochene Zeile an. Das Menue setzt Beziehungen,
Positionen, Routine und Zeit. Begruessung und Murmeln bleiben Originalverhalten.
Die automatischen Intervalle sind native Engine-Logik; die 95 Sekunden sind eine
Testfrist, keine bewiesene Obergrenze der Sprachauswahl. Wetter-, Essens-, Morgen-
und saemtliche zufaelligen Sprachvarianten sind damit nicht einzeln qualifiziert.
Lippensynchronisation ist ausgenommen. Korrekte Archivzuordnung beweist noch
nicht, welche konkrete Aufnahme der laufende Resolver gewaehlt hat.

## Quell- und API-Belege

Bezogen auf den unveraenderten Baum `work/npc-warning-fix/tree`:

- `AI/DailyRoutines/DailyRoutines_OG.as:8`: Human-Tagesablauf installiert
  `UAIARM_Human_NeutralVoiceReactions`.
- `AI/CharacterAI_Human.as:449` und
  `GAS/PerceptionEventMixins.as:1201`: Freund-Sicht-Ereignis; Radius 250 cm,
  Spielerfilter, Beziehung 5, 300 Sekunden Cooldown.
- `AI/AssessmentResponseSystem/AssessmentResponseModules/Human/AIARM_NeutralVoiceReactions.as:47`:
  originale Begruessungsreaktion. Tagesablauf erforderlich, Folgen/Fuehren gesperrt.
- `Story/G1R/GenericVoiclines.as:18`: Freund-Begruessung verlangt Freundschaft.
  `:1995`: gewoehnliches Murmeln; keine Bergbau-, Trage- oder Zuhoerinteraktion.
- `AI/AIAgent/Human/GEs_Human.as:33` gewaehrt `UGA_Human_Mumble`.
  `GAS/Abilities/Conversation/GA_Human_Mumbling.as:7–31`: Blockaden und Themen;
  Intervallvorgaben 30 Sekunden plus 15 Sekunden Varianz. Native Ausfuehrung
  und globale Resolverbedingungen werden nicht durch diese Vorgaben bewiesen.
- `Story/G1R/Conversation/Conversation_OC_GRD_KIRGO_251.as:573`:
  `::SetRelationshipTowards(NPC, Hero, ERelationship(5))` als Originalaufruf.
- `AI/States/AIState_DailyRoutine.as:2399`: `UAIState_TestGotoWP` verwendet
  `GotoPreferredLocation`; derselbe Weg ist in der vorherigen Laufprobe im Spiel
  bestaetigt. Keine Interaktion auf fuer andere Figuren reservierten Freepoints.
- `Story/G1R/VoiceTypes.as:4648` und `:5296`: Diego-/Lares-Zuordnungen.
  Lares verwendet dieses Profil auch in
  `AI/AIAgent/Human/Config/NC_ORG_Lares_801/ConversationCharacterSettings_NC_ORG_Lares_801.as:8`.
  `source-checks.json` prueft die zehn genauen deutschen Begruessungs-/Murmeln-
  Archivmitglieder mit SHA256, ohne Aufnahmen oder Sprachauswahl zu ersetzen.

Neue Overrides verwenden `UFUNCTION(BlueprintOverride)`. Alle vorherigen A-/B-
Klassen, Quest-Helfer, Routinen und Feldreihenfolgen bleiben vorhanden; 29 alte
Auswahlklassen sind nur unsichtbar. C wird zwingend frisch aus 030 erzeugt,
damit kein bereits initialisiertes Stimmprofil stillschweigend ueberschrieben
werden muss. Der neue Weltwert `gore_natural_voice_batch_v1_ready=2` kennzeichnet
ausschliesslich den abgeschlossenen Aufbau dieser Variante.
