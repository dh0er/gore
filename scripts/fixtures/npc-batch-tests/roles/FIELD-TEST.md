# Paket4: Begleiter, Kampf und Lebenszyklus

## Aktueller Nachtest — 0.1.3

Folgen/Warten, Niederlage/Erholung, Feindschaft, Gildenwechsel und Tod/Laden haben
funktioniert. Auch die Wiederbelebung mit0.1.1 ist jetzt vom Benutzer bestaetigt.
Nur Flucht erneut testen; [letzte Rueckmeldung und Save-Diagnose](field-runtime-0.1.2.json).
Der vorherige Versuch hat den Fluchtzustand nicht bis zur ersten Diagnosemarke
ausgefuehrt.0.1.3 erlaubt wie die funktionierenden Tagesablaeufe simulierte
Ablaufschritte; ob das den Eintritt behebt, klaert dieser Nachtest.
Die alten Ergebnisse und der nicht mehr benoetigte tote Ausgangsstand030 sind
extern archiviert. Profil4 enthaelt noch070–072.

1. **072 „NPC 04 Rollen - Kampf bereit“ frisch laden.** Bei A **26** waehlen,
   dann sofort unbewaffnet zu B gehen, bis du weniger als zehn Meter entfernt bist.
   B soll vor dir weglaufen. Der Testzustand endet nach rund60 Echtzeitsekunden;
   deshalb direkt nach der Auswahl hingehen.
2. Wenn er weggelaufen ist oder nach20–30 Sekunden weiter nichts tut, zurueck zu A.
   **27** beendet die Flucht, **31** protokolliert den Zustand. Anschliessend
   **`rolle-flucht-v4`** speichern, auch bei Stillstand.26 vorher nicht erneut
   auswaehlen: Das setzt die Diagnosewerte zurueck.27 erhaelt Eintritts- und
   Bewegungswerte und bereits vorhandene Abschluss-/Fehlercodes. Nur ein noch
   offener Abschlusscode wird auf „explizit gestoppt“ gesetzt.
   Der Save enthaelt Eintritt, Entfernung, Bewegungsversuche und tatsaechlich
   zurueckgelegte Strecke. Bei aktivem Konflikt erst dessen Ende abwarten.

31 vor27 ist ebenfalls okay;31 zeichnet den aktuellen Zustand auf und setzt
keine Eintritts- oder Bewegungswerte zurueck.

## Vollstaendige Referenz der ersten Runde

Profil4. In **070 „NPC 04 Rollen - START“** und **072 „NPC 04 Rollen - Kampf bereit“**
hat der Held jetzt **10.000/10.000 Leben und 1.000 Staerke**, B **50/50 Leben
und 0 Schutz gegen Hieb, Schlag und Stich**. 072 behaelt zusaetzlich die bisherigen
erhoehten Schutzwerte des Helden. Die Originale sind extern archiviert; siehe
[Gesundheitsanpassung](field-health-starts.json) und [Schadensanpassung](field-damage-starts.json).
Beide beginnen mittags auf der freien Flaeche vor dem Alten
Lager. B steht vor dir; **A steht am Tor etwa30 Meter hinter dir**.
Alle nummerierten Optionen waehlst du bei A. Es gibt keinen neuen Aufbau.

Vor jeder neuen Testgruppe den genannten START frisch laden. Ergebnisstaende
unter neuen Namen speichern, die STARTs behalten. **31** protokolliert Bs
Zustand und beendet den Dialog; vor einem Ergebnis-Save nach Moeglichkeit
waehlen. Ist Speichern im Konflikt gesperrt, melde das und gehe bei Bedarf auf
Abstand. Die Waffen-Diagnose zeichnet bereits selbst auf; dafuer ist31 optional.

## 1. Waffenwahl — 070

Bei A **30** waehlen. Danach sofort zu B zurueck: Die Aufzeichnung laeuft
60 Sekunden. Direkt vor B dein Schwert ziehen und die Warnung zum Kampf
eskalieren lassen, wie beim bisherigen Schwert-zu-Bogen-Fehler. Bs Waffen
behalten. Beobachten, was er waehrend Warnung und Kampf zieht.
Nach der Beobachtung B ohne Todesstoss besiegen oder auf Abstand gehen. Waffe
einstecken und das Ende des Konflikts abwarten, dann **`rolle-waffenwahl`** speichern.
Die Diagnosewerte bleiben erhalten;30 nicht erneut starten. Im Kampf oder bei
totem Helden bleibt Speichern gesperrt. Danach070 frisch laden.
Keine bestimmte Waffenwahl gilt vorab als bestanden. Dieser Test soll deren
Ursache eingrenzen. Falls andere NPCs eingreifen oder B gar nicht reagiert,
getrennt notieren; die Aufnahme nicht durch andere Rollenoptionen vorbereiten.
Diese Runde nutzt die auf Wunsch angepassten Lebens-, Staerke- und Schutzwerte;
die urspruengliche Waffenwahl bleibt als fruehere Beobachtung dokumentiert.

## 2. Folgen und Warten — 070

Bei A **20**, dann ein Stueck weggehen: B soll folgen. **31**, speichern als
**`rolle-folgen`**, laden und pruefen, dass er weiterhin folgt.
Zurueck zu A, **21**: B soll an seinem aktuellen Ort bleiben, wenn du weggehst.
**31**, **`rolle-warten`** speichern, laden und erneut weggehen. Danach bei A
**20**: B soll wieder folgen. Ergebnis kurz melden.

## 3. Trainingskampf ohne Todesziel — 072

Bei A **22**, dann zu B. B im Kampf besiegen, **keinen Todesstoss** ausfuehren.
Er soll besiegt werden und sich anschliessend erholen, statt dauerhaft zu sterben.
**31**, **`rolle-niederlage`** speichern. Eingriffe durch A oder andere NPCs melden.

## 4. Feindschaft bis zur Niederlage — 072 frisch laden

Bei A **23**, zu B gehen. Er soll feindlich reagieren. Besiegen, ohne ihn zu
toeten. Danach pruefen, ob die voruebergehende Feindschaft endet: Waffe einstecken,
Erholung abwarten und sich erneut naehern. **31**, **`rolle-feind-besiegt`** speichern.

## 5. Gilde wechseln und zurueck — 070 frisch laden

Bei A **24**, danach **31** und **`rolle-gilde-neutral`** speichern.
Anschliessend **25**, **31** und **`rolle-gilde-zurueck`** speichern.
Es muss dabei kein sichtbarer Effekt auftreten: Die tatsaechliche Gilde und
Beziehung prueft Codex anhand der Saves. Auffaellige Reaktionen trotzdem melden.

## 6. Fluchtregel — 072 frisch laden

Bei A **26**, sofort unbewaffnet auf weniger als zehn Meter an B herangehen.
Er soll im expliziten Fluchtzustand vor dem Helden fliehen; diese Option erzeugt
seit0.1.1 keine neue Feindschaft. Die Flucht ist auf rund60 Echtzeitsekunden begrenzt.
**31**, **`rolle-flucht`** speichern, soweit moeglich. Bei A **27** stellt die
vorherige Fluchtregel wieder her und beendet den expliziten Fluchtzustand,
loescht aber keine vorhandene Feindschaftserinnerung.
**31**, **`rolle-flucht-zurueck`** speichern. Danach erneut den sauberen START laden.

## 7. Tod und Wiederkehr derselben Figur — 072 frisch laden

Bei A **28**, zu B gehen und ihn im Kampf samt noetigem Todesstoss toeten.
**31**, **`rolle-tod`** speichern, laden und pruefen, dass B tot bleibt.
Bei A **29** waehlen (nur bei totem B sichtbar). Weiter vom Bereich weggehen,
mindestens eine Spielstunde verstreichen lassen und zurueckkehren. Beobachten,
ob **derselbe einzelne B** zurueckkehrt. **31**, **`rolle-wiederbelebt`** speichern;
auch einen erfolglosen Versuch nach dieser Wartezeit speichern und melden.
Die genaue Distanz-/Simulationsschwelle ist noch ungeprueft; nicht endlos warten.
Wenn B wieder lebt, bei A **21** waehlen, um die besondere Wiederkehr-Routine
wieder durch normales Warten zu ersetzen.

Quest-START071 bleibt fuer das anschliessende Paket. Technische Messwerte und
API-Grenzen stehen in der [Rollenreferenz](README.md#field-roles--lifecycle--mixed-weapons).
