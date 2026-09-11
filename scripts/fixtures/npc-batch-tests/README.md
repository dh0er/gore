# NPC-Testpakete: restliche Faelle

Stand 2026-09-11: **Alle fuenf Pakete sind gebaut und geprueft.** Der Kopf-/Barttest
ist bestanden; die weiteren Spieltests sind noch offen. Von den acht erstellten
Startstaenden bleiben **066–072 fuer die offenen Tests in Profil 4**; der erledigte
Kopf-Start065 und alle bisherigen Ergebnissaves sind ausserhalb des Spiels archiviert.
Der bisher getestete Stand ist mit `6848e475` gesichert; `117be93d` korrigiert
die Save-Bereinigung und `7ac45595` die benoetigte native API-Komposition.
Siehe [Build-/Paketpruefung](build-checks.json) und
[veroeffentlichte Startstaende](start-saves.json).

**Aktiv ist NpcNaturalVoiceTest 0.1.1**; Manager-Status `in_sync`.
Kopf-Test und Barttexturen sind nach dem bestandenen
[Barttest 0.1.3](heads/beard-runtime-0.1.3.json) deaktiviert. Die anderen
Testpakete bleiben deaktiviert. Die aktuelle Aktivierung steht im
[Sprach-Deploymentbericht](voice/deployment-after-heads.json).
Der fehlgeschlagene [Barttest 0.1.2](heads/beard-runtime-0.1.2.json) und die
[VT-Build-](beard-vt-build-checks.json) / [Deploymentbelege](beard-vt-deployment.json)
bleiben als Historie erhalten.
Die bisherigen [Tageslicht-](daylight-build-checks.json) und
[Deploymentberichte](daylight-deployment.json) bleiben als Historie erhalten.

Die Testpakete werden **einzeln** aktiviert; zum Kopf-Test gehoert zusaetzlich
das Textur-Begleitpaket. Zum Paketwechsel das Spiel vollstaendig
beenden und den zugehoerigen START-Stand laden. Ergebnisse anderer Pakete nicht
als Ausgangspunkt verwenden. Die benoetigten Startstaende bleiben erhalten; Ergebnisse unter
den unten verlinkten Testnamen neu speichern. Spieltests macht der Benutzer.

| Reihenfolge | Paket | Startstaende in Profil 4 | Abdeckung |
|---|---|---|---|
| 1 | NpcHeadPaletteTest | 065: NPC 01 Koepfe - START (archiviert) | Bestanden: Gesicht, Haare, Haarfarbe, Bartvarianten, Originale und Neustart |
| 2 | NpcNaturalVoiceTest | 066: NPC 02 Voice - START | Natuerliche Begruessung/Alltagsstimme, zweites Stimmprofil, Routine, Neustart |
| 3 | NpcEconomyRolesTest | 067: Handel - LP fehlt; 068: Lehrer - Erz fehlt; 069: Lehrer - bereit | Kaufen/Verkaufen/Abbrechen, beide Lernvoraussetzungen, Kosten, bereits gelernt, Persistenz |
| 4 | NpcFieldRolesTest | 070: NPC 04 Rollen - START; 072: NPC 04 Rollen - Kampf bereit | Waffenwahl-Diagnose, Folgen/Warten, Feindschaft/Gilde, Flucht, Niederlage/Tod, Wiederbelebung derselben Figur |
| 5 | NpcQuestCallbacksTest | 071: NPC 05 Quest - START | Eigenes Journal, zwei automatische Ziele, Abgabe, einmalige Belohnung, Erfolg/Abbruch, Neustart |

## 1. Koepfe

Der abgeschlossene Kopf-Test verwendete065, A und **H0 Aufbau** vor Xardas' Turm.
Dieser Startstand ist jetzt archiviert und muss nicht erneut getestet werden.
Die bestandene [Bart-Checkliste](heads/README.md) beschreibt den Nachtest:
H2/H5 fuer rasierten Heldenkopf, H6 fuer Originalbart; H1/H6 fuer Flex mit Bart,
H5 fuer Flex ohne Bart. Beide neuen Auswahlen speichern/neu starten, danach H9.
Die neue Texturzuordnung betrifft nur Cs Materialinstanzen. Der Nutzer bestaetigt
auch den vollstaendigen 0.1.3-Nachtest. Kopf-/Barttests sind damit fuer diese
Varianten abgeschlossen; jetzt folgt Paket 2.

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

Nach dem bestandenen Kopf-/Barttest wurden auf Wunsch des Benutzers weitere
**18 Saves aus Profil 4 archiviert**. Im Spiel bleiben nur die **sieben kommenden
STARTs066–072**; auch bewahrte Vergleichs-/Beweissaves und Kopf-Start065 stehen
jetzt ausschliesslich im Archiv. Die vollstaendige Sicherung dieses Durchlaufs
(25 Saves plus Profildatei und Hashmanifest) liegt dauerhaft ausserhalb des
Worktrees unter `C:\Users\Daniel\Documents\GORE\Savegame-Archiv\npc-tests-20260911T174130Z`.
Andere Profile und die sieben behaltenen Saves sind bytegleich.
Siehe [Folgearchivierung](profile-cleanup-after-heads.json).

Historie der ersten Bereinigung:

31 ueberholte Teststaende wurden mit dem korrigierten Save-Core aus Profil 4
entfernt. Die elf damals behaltenen Vergleichsstaende waren 023, 027, 030, 048, 052, 054,
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
