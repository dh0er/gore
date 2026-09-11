# NPC-Testpakete: restliche Faelle

Stand 2026-09-11: **Alle fuenf Pakete sind gebaut und geprueft; acht Startstaende
stehen in Profil 4 bereit.** Die Spieltests dieser neuen Faelle sind noch offen.
Der bisher getestete Stand ist mit `6848e475` gesichert; `117be93d` korrigiert
die Save-Bereinigung und `7ac45595` die benoetigte native API-Komposition.
Siehe [Build-/Paketpruefung](build-checks.json) und
[veroeffentlichte Startstaende](start-saves.json).

**Aktiv ist jetzt NpcNaturalVoiceTest 0.1.1**; Manager-Status `in_sync`.
Kopf- und andere Pakete sind deaktiviert. Die aktualisierten Kopf-/Sprachpakete
sind [geprueft](daylight-build-checks.json); aktuelle IDs und Ruecknahme stehen
im [Tageslicht-Deploymentbericht](daylight-deployment.json).
Der urspruengliche [Deploymentbericht](deployment.json) bleibt als Historie erhalten.

Die Pakete werden **einzeln** aktiviert. Zum Paketwechsel das Spiel vollstaendig
beenden und den zugehoerigen START-Stand laden. Ergebnisse anderer Pakete nicht
als Ausgangspunkt verwenden. Die Startstaende bleiben erhalten; Ergebnisse unter
den unten verlinkten Testnamen neu speichern. Spieltests macht der Benutzer.

| Reihenfolge | Paket | Startstaende in Profil 4 | Abdeckung |
|---|---|---|---|
| 1 | NpcHeadPaletteTest | 065: NPC 01 Koepfe - START | Gesicht, Haare, Haarfarbe, Wiederherstellung, Neustart; Barttextur offen |
| 2 | NpcNaturalVoiceTest | 066: NPC 02 Voice - START | Natuerliche Begruessung/Alltagsstimme, zweites Stimmprofil, Routine, Neustart |
| 3 | NpcEconomyRolesTest | 067: Handel - LP fehlt; 068: Lehrer - Erz fehlt; 069: Lehrer - bereit | Kaufen/Verkaufen/Abbrechen, beide Lernvoraussetzungen, Kosten, bereits gelernt, Persistenz |
| 4 | NpcFieldRolesTest | 070: NPC 04 Rollen - START; 072: NPC 04 Rollen - Kampf bereit | Waffenwahl-Diagnose, Folgen/Warten, Feindschaft/Gilde, Flucht, Niederlage/Tod, Wiederbelebung derselben Figur |
| 5 | NpcQuestCallbacksTest | 071: NPC 05 Quest - START | Eigenes Journal, zwei automatische Ziele, Abgabe, einmalige Belohnung, Erfolg/Abbruch, Neustart |

## 1. Koepfe

065 laden, A ansprechen, **H0 Aufbau** waehlen. Danach stehen die Testfiguren
an den bereits verwendeten Punkten vor Xardas' Turm.
Die [Kopfliste](heads/README.md) dokumentiert den Befund und die verbleibenden Optionen.
Der Nutzer hat diese Kopftests ausser Bart bestanden. Der Heldenbart ist in
die Gesichtstextur eingezeichnet; H5/H6 sind ab 0.1.1 verborgen. Ein echter
Bartwechsel ueber passende Gesichtstexturen bleibt offen. Haarfarbe wird nur an Cs Materialinstanz geaendert.

## 2. Natuerliche Stimme

Alle acht START-Saves stehen jetzt auf 12:00 Uhr; der [Tageslichtbericht](daylight-starts.json)
haelt die Aenderung mit Backups fest. Kopf- und Sprach-Aufbau setzen ebenfalls
Mittag. Beim Sprachtest liegen die optionalen Wechsel jetzt bei 14:00/16:00 Uhr.

066 laden und bei A **01 Natuerliche Stimmen: Aufbau mit B und C** waehlen.
Erst vor der Annaeherung speichern, dann B und C ohne Dialog annaehern.
Die [Stimmenliste](voice/README.md) trennt Begruessung, selbststaendiges Murmeln
und Neustart. Sie enthaelt klare Beobachtungsfristen; Stille ist ein Ergebnis.

## 3. Handel und Lehrer

1. 067 laden. Vor Testmitteln oder Handel **03 Lernen** probieren:
   0 LP/50 Erz, daher keine Veraenderung und keine gelernte Faehigkeit.
   Danach 01/02 fuer Kaufen, Verkaufen und Handelsabbruch verwenden.
2. 068 laden. **03 Lernen** probieren: 5 LP/0 Erz, ebenfalls kein Abzug/Erfolg.
3. 069 laden: 15 LP/200 Erz. Tauchen lernen kostet genau 5 LP/30 Erz.
   Erneut versuchen: trotz ausreichender Mittel kein zweiter Abzug.
   Speichern, vollstaendig neu starten und pruefen, dass Tauchen bekannt bleibt.

**04 Testmittel** ist fuer diese vorbereiteten Starts nicht erforderlich.
Weitere Inventar- und Neustartpruefungen: [Rollenliste](roles/README.md).

## 4. Begleiter, Kampf und Lebenszyklus

070 beginnt auf der bereits erprobten freien Flaeche vor dem Alten Lager;
B steht vor dem Helden. A bleibt am Tor, etwa 30 Meter entfernt. Fuer die
Steueroptionen zuerst zurueck zu A gehen. Die Diagnose 30 laeuft 60 Sekunden;
danach direkt zu B zurueck und den bekannten Waffenwechsel reproduzieren.
070 behaelt die urspruenglichen Heldenwerte. Bs Waffen und Werte werden durch
den Startstand nicht geaendert.

072 ist fuer die positiven Sieg-/Tod-Tests bestimmt: gleicher frischer
NPC-Ausgangspunkt, aber ein ausdruecklich verstaerkter Held. Die Ressourcen
stehen im abschliessenden Startstandbericht. Dieser Stand ist kein Ersatz fuer
die unveraenderte Waffenwahl-Diagnose. A soll am Tor bleiben; falls er oder eine
andere Figur eingreift, dies getrennt notieren.

Die [Rollenliste](roles/README.md) beschreibt 20–31 und die getrennten Zweige.
Zwischen Feindschaft, Gilde, Flucht und Tod jeweils den sauberen Start neu laden.
Wiederbelebung ist eine explizit gewaehlte native Tagesablauf-Regel nach Tod,
kein automatisch nachgespawnter zweiter B. Ob die Regel bei diesem menschlichen
NPC greift, ist genau der offene Test.

## 5. Quest

071 laden, bei A **Q1 Auftrag: Proviant fuer die Wache** waehlen.
Die [Questliste](quest/README.md) prueft Q1–Q7 einschliesslich Neustarts im
Zwischenstand sowie auf Erfolgs- und Abbruchpfad. Die Belohnung sind einmalig
25 Erz fuer zwei Kaese. Q3 liefert den Testvorrat genau einmal.

## Profil 4 und Rueckmeldungen

31 ueberholte Teststaende wurden mit dem korrigierten Save-Core aus Profil 4
entfernt. Die elf behaltenen Vergleichsstaende sind 023, 027, 030, 048, 052, 054,
060, 061, 062, 063 und 064. Alle urspruenglichen 42 Saves und die Profildatei sind
archiviert; andere Profile und behaltene Saves wurden per Hash geprueft.
Unmittelbar nach der Bereinigung waren es mit den acht Starts 19 Saves;
spaetere Ergebnissaves kommen hinzu. Die Vollsicherung
liegt lokal unter `work/npc-batch-tests/cleanup/archive-20260911T094051Z`.
Siehe [Bereinigungsbericht](profile-cleanup.json).

Bitte pro Paket die Beobachtungen und benannten Saves melden. Wir unterscheiden
sicht-/hoerbares Spielverhalten von gespeicherten Steuerflags. Guide und offene
Punkte werden anhand dieser Ergebnisse fortgeschrieben; ein gebautes Paket ist
noch kein bestandener Spieltest. Line-spezifischer Lip Sync bleibt ausgenommen.
