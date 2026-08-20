// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Polish (`pl`).
class AppLocalizationsPl extends AppLocalizations {
  AppLocalizationsPl([String locale = 'pl']) : super(locale);

  @override
  String get coreBlockedTitle => 'Mod Manager nie może się uruchomić';

  @override
  String get coreDllMissingMessage =>
      'Brakuje wymaganego pliku programu (gore_ffi.dll).';

  @override
  String get coreDllLoadFailedMessage =>
      'Nie udało się wczytać wymaganego pliku programu.';

  @override
  String get coreVerificationFailedMessage =>
      'Nie udało się zweryfikować wymaganego pliku programu.';

  @override
  String get coreManagerTooOldMessage =>
      'Pliki programu są nowsze niż Mod Manager. Zaktualizuj Mod Managera.';

  @override
  String get coreNativeTooOldMessage =>
      'Pliki programu są starsze niż Mod Manager. Zainstaluj Mod Managera ponownie.';

  @override
  String get coreCommandsMissingMessage =>
      'Plikom programu brakuje funkcji wymaganych przez tego Mod Managera.';

  @override
  String get coreBlockedRepairHint =>
      'Zainstaluj ponownie lub napraw Mod Managera, a potem uruchom go jeszcze raz.';

  @override
  String get coreTechnicalDetails => 'Szczegóły techniczne';

  @override
  String get coreCopyTechnicalDetails => 'Kopiuj szczegóły techniczne';

  @override
  String get coreTechnicalDetailsCopied => 'Skopiowano szczegóły techniczne';

  @override
  String get coreTechnicalDetailsCopyFailed =>
      'Nie udało się skopiować szczegółów technicznych. Spróbuj ponownie.';

  @override
  String get preflightAttention =>
      'Zanim będzie można zmienić mody, trzeba coś załatwić.';

  @override
  String get preflightGameRunning =>
      'Gothic nadal działa. Zamknij grę przed zmianą modów.';

  @override
  String get managerOperationFailed => 'Operacja nie powiodła się.';

  @override
  String get libraryOperationFailed => 'Nie udało się wczytać listy modów.';

  @override
  String get conflictsUnavailable => 'Nie udało się sprawdzić konfliktów.';

  @override
  String applyReportAppliedWithWarnings(int applied, int warnings) {
    return 'Zastosowano: $applied. Ostrzeżenia: $warnings.';
  }

  @override
  String get modDetailKind => 'Typ';

  @override
  String get modDetailVersion => 'Wersja';

  @override
  String get modDetailAuthor => 'Autor';

  @override
  String get modDetailSource => 'Źródło';

  @override
  String get modDetailImported => 'Zaimportowano';

  @override
  String get componentLocalization => 'Teksty';

  @override
  String get componentAudio => 'Dźwięk';

  @override
  String get componentAngelScript => 'Skrypty';

  @override
  String get componentTexture => 'Tekstury';

  @override
  String get componentGameFiles => 'Pliki gry';

  @override
  String get componentVoice => 'Dialogi mówione';

  @override
  String get componentKindLocalizationPatch => 'Zmiany tekstów';

  @override
  String get componentKindAudioPatch => 'Zmiany dźwięku';

  @override
  String get componentKindAngelScriptPatch => 'Zmiany skryptów';

  @override
  String get componentKindTexturePatch => 'Zmiany tekstur';

  @override
  String get componentKindLoosePak => 'Plik PAK';

  @override
  String get componentKindTriplet => 'Kontener IoStore';

  @override
  String get componentKindUe4ssLua => 'Skrypt UE4SS';

  @override
  String get componentKindRawFile => 'Plik';

  @override
  String get componentKindFilePatch => 'Zastąpiony plik gry';

  @override
  String get componentKindPakFilePatch => 'Plik gry z paczki PAK w ~mods';

  @override
  String get componentKindVoiceArchivePatch => 'Dialogi mówione';

  @override
  String get rawTargetGameText => 'Wszystkie teksty gry';

  @override
  String get rawTargetGameScripts => 'Wszystkie skrypty gry';

  @override
  String get rawTargetSoundBank => 'Bank dźwięków';

  @override
  String rawTargetSoundBankNamed(String name) {
    return 'Bank dźwięków: $name';
  }

  @override
  String get conflictKindLocalization => 'Teksty';

  @override
  String get conflictKindAudio => 'Dźwięk';

  @override
  String get conflictKindAsset => 'Dane gry';

  @override
  String get conflictKindCdo => 'Wartości obiektów';

  @override
  String get conflictKindUe4ssUnknown => 'UE4SS (niejasne)';

  @override
  String get conflictKindScriptModule => 'Skrypt gry';

  @override
  String get conflictKindVoiceArchive => 'Dialogi mówione';

  @override
  String get conflictKindRawFile => 'Plik';

  @override
  String get conflictKindLooseFile => 'Plik gry';

  @override
  String get preflightUnavailable => 'Nie udało się sprawdzić instalacji gry.';

  @override
  String get preflightRetry => 'Sprawdź ponownie';

  @override
  String get preflightReviewStatus => 'Pokaż stan';

  @override
  String get preflightReviewRecovery => 'Pokaż pomoc';

  @override
  String get installRecoveryTitle => 'Przerwana instalacja';

  @override
  String get installRecoveryBody =>
      'GORE znalazł pozostałości po instalacji lub kompilacji skryptów. To zadanie może wciąż trwać albo już się zakończyło i to zostawiło. GORE nie może tego bezpiecznie posprzątać samodzielnie.';

  @override
  String get installRecoverySteps =>
      'Jeśli zadanie wciąż trwa, poczekaj, aż się skończy — nie przerywaj go i nie usuwaj plików. Gdy masz pewność, że nic nie działa, wykonaj kroki z README.txt w poniższym folderze i sprawdź ponownie. Jeśli nie podano folderu albo nie masz pewności, zostaw wszystko i poproś o pomoc.';

  @override
  String get installRecoveryEvidence => 'Co znalazł GORE';

  @override
  String get managerRecoveryTitle => 'Napraw przerwaną zmianę';

  @override
  String get managerRecoveryConfirm =>
      'GORE znalazł przerwaną zmianę i może przywrócić grę do znanego stanu. Twoje zapisy gry nigdy nie są ruszane.';

  @override
  String get managerRecoveryAlreadyClean =>
      'Nie było już nic do naprawienia. Stan sprawdzono ponownie.';

  @override
  String get managerRecoveryBusy =>
      'Zadanie znów działa. Nic nie zmieniono — poczekaj, aż się skończy.';

  @override
  String get managerRecoveryLockCleared =>
      'Przerwane zadanie jeszcze nic nie zmieniło. Zostało posprzątane.';

  @override
  String get managerRecoveryRestoredPristine =>
      'Zmiana została cofnięta. Gra wróciła do wcześniejszego stanu.';

  @override
  String get managerRecoveryApplyPreserved =>
      'Zastosowanie już się zakończyło. Nic nie przepadło.';

  @override
  String get managerRecoveryUndeployConfirmed =>
      'Usuwanie już się zakończyło. Pozostałości posprzątano.';

  @override
  String get managerRecoveryCompileRequired =>
      'To należy do kompilacji skryptów, więc nic nie zmieniono. Otwórz pomoc dotyczącą naprawy.';

  @override
  String get managerRecoveryInspectionFailed =>
      'GORE nie mógł bezpiecznie sprawdzić przerwanego zadania. Nic nie zmieniono.';

  @override
  String get managerRecoveryFailed =>
      'Nie udało się dokończyć naprawy. Sprawdź stan, zanim spróbujesz ponownie.';

  @override
  String get statusUnknown => 'Nieznany';

  @override
  String statusDetailsTitle(String status) {
    return 'Stan: $status';
  }

  @override
  String statusDetailsOpen(String status) {
    return 'Pokaż szczegóły: $status';
  }

  @override
  String get statusDetailsNoRoot =>
      'Najpierw wybierz instalację Gothic w ustawieniach.';

  @override
  String get statusDetailsNoDeployment => 'W grze nie ma teraz żadnych modów.';

  @override
  String get statusDetailsInSyncDescription =>
      'W grze są dokładnie te mody, które tu zaznaczono.';

  @override
  String get statusDetailsDeployedLoadout => 'Mody w grze';

  @override
  String get statusDetailsChangesDescription =>
      'Twój wybór różni się od tego, co jest w grze.';

  @override
  String get statusDetailsCurrentlyDeployed => 'Teraz w grze';

  @override
  String get statusDetailsAfterApply => 'Po zastosowaniu';

  @override
  String get statusDetailsGameUpdatedDescription =>
      'Gra została zaktualizowana i nadpisała pliki modów. Zastosuj ponownie, aby je przywrócić.';

  @override
  String get statusDetailsDriftedFiles => 'Pliki, których to dotyczy';

  @override
  String get statusDetailsStudioDescription =>
      'Mod Studio ma teraz mody w tej grze. Przejmij grę, zanim Manager zastosuje twoje.';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Mod Studio: $name';
  }

  @override
  String get statusDetailsStudioNameUnknown => 'Mod Studio nie podało nazwy.';

  @override
  String get statusDetailsRecoveryDescription =>
      'Zmiana została przerwana. Napraw ją przed zmianą modów.';

  @override
  String get statusDetailsUnknownDescription =>
      'Nie udało się odczytać stanu. Najpierw odśwież.';

  @override
  String get statusDetailsUnavailable => 'Brak szczegółów.';

  @override
  String get statusDetailsEmptyLoadout => 'Brak modów.';

  @override
  String get statusDetailsLastError => 'Ostatni błąd';

  @override
  String get statusDetailsLastApply => 'Ostatnie zastosowanie';

  @override
  String get statusDetailsAppliedMods => 'Zastosowane mody';

  @override
  String get statusDetailsWarnings => 'Ostrzeżenia';

  @override
  String get statusDetailsReapply => 'Zastosuj ponownie';

  @override
  String get statusDetailsOpenSettings => 'Otwórz Ustawienia';

  @override
  String get recoveryAction => 'Napraw';

  @override
  String get recoveryRequiredConfirm =>
      'Naprawić przerwaną zmianę i usunąć częściowo zainstalowane pliki?';

  @override
  String get statusRecoveryRequired => 'Wymagana naprawa';

  @override
  String get statusDetailsOwnershipTitle => 'Pliki zarządzane przez GORE';

  @override
  String get statusDetailsOwnershipDescription =>
      'Zapisane przy zastosowaniu modów — to nie sprawdzenie, czy pliki nadal istnieją.';

  @override
  String get statusDetailsOwnershipLive => 'Zastąpione pliki gry';

  @override
  String get statusDetailsOwnershipBackups => 'Kopie oryginałów';

  @override
  String get statusDetailsOwnershipAdditive => 'Dodane pliki modów';

  @override
  String get statusDetailsOwnershipUe4ss => 'Katalogi modów UE4SS';

  @override
  String get statusDetailsOwnershipRecovery => 'Pliki naprawcze';

  @override
  String get statusDetailsOwnershipEmpty => 'Nic tu nie zapisano.';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return 'Pokazano $shown z $total ścieżek.';
  }

  @override
  String get appTitle => 'GORE Mod Manager';

  @override
  String get tabMods => 'Mody';

  @override
  String get tabSettings => 'Ustawienia';

  @override
  String get settingsGameExe => 'Instalacja Gothic';

  @override
  String get settingsGameExePick => 'Wybierz…';

  @override
  String get settingsLanguage => 'Język';

  @override
  String get libraryEmptyTitle => 'Jeszcze brak modów';

  @override
  String get libraryEmptyBody => 'Zaimportuj folder lub plik moda, aby zacząć.';

  @override
  String get detailEmptyHint => 'Wybierz mod, aby zobaczyć, co zmienia.';

  @override
  String get settingsAdvanced => 'Szczegóły zaawansowane';

  @override
  String get settingsAdvancedHint =>
      'Pokazuje stronę techniczną: zmieniane pozycje, wiarygodność sprawdzania konfliktów i pliki zarządzane przez GORE.';

  @override
  String get updatesTitle => 'Aktualizacje';

  @override
  String get checkForUpdatesAutomatically =>
      'Automatycznie sprawdzaj aktualizacje';

  @override
  String get checkForUpdatesNow => 'Sprawdź aktualizacje teraz';

  @override
  String get updatesPortableNotice =>
      'Wersja przenośna otwiera stronę pobierania w przeglądarce. Zastąp istniejące pliki nowo pobranymi.';

  @override
  String get updateCheckFailed =>
      'Nie udało się sprawdzić aktualizacji. Spróbuj później.';

  @override
  String get updateUpToDate => 'Używasz najnowszej wersji.';

  @override
  String get updateAvailableTitle => 'Dostępna aktualizacja';

  @override
  String updateAvailableMessage(String version, String current) {
    return 'Dostępna jest wersja $version. Masz $current.';
  }

  @override
  String get updateLater => 'Później';

  @override
  String get updateDownload => 'Pobierz';

  @override
  String updateOpenFailed(String url) {
    return 'Nie udało się otworzyć strony pobierania. Znajdziesz ją pod $url';
  }

  @override
  String get statusInSync => 'Aktualne';

  @override
  String get statusChangesPending => 'Niezastosowane';

  @override
  String get statusGameUpdated => 'Gra została zaktualizowana';

  @override
  String get statusStudioDeploy => 'Mod Studio aktywne';

  @override
  String get statusNothingDeployed => 'Brak modów w grze';

  @override
  String get actionImport => 'Importuj';

  @override
  String get actionApply => 'Zastosuj';

  @override
  String get actionStartGame => 'Uruchom grę';

  @override
  String get startGameTooltip =>
      'Uruchom Gothic z modami, które są teraz w grze';

  @override
  String get startGameFailed =>
      'Nie udało się uruchomić Gothic. Sprawdź instalację gry w ustawieniach.';

  @override
  String get commonCancel => 'Anuluj';

  @override
  String get commonOk => 'OK';

  @override
  String get importFolder => 'Importuj folder…';

  @override
  String get importFile => 'Importuj plik…';

  @override
  String importOutcomeCreated(String name) {
    return 'Dodano „$name”.';
  }

  @override
  String importOutcomeUpdated(String name) {
    return 'Zaktualizowano „$name”.';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '„$name” jest już na twojej liście.';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none': 'Żaden istniejący mod nie pasował.',
      'source': 'Dopasowano po tym samym źródle importu.',
      'content': 'Dopasowano po zweryfikowanej identycznej zawartości.',
      'entry_id': 'Dopasowano po identyfikatorze moda.',
      'other': 'Brak szczegółów dopasowania.',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous =>
      'To pasuje do kilku modów, które już masz. Usuń duplikaty i spróbuj ponownie.';

  @override
  String get importRefusalIdentityConflict =>
      'Źródło i zawartość pasują do dwóch różnych modów, które już masz. Uporządkuj to i spróbuj ponownie.';

  @override
  String get importFailed =>
      'Nie udało się tego zaimportować. Obsługiwane są foldery, archiwa ZIP i pojedyncze pliki modów (*_P.pak, .utoc/.ucas, .lcache, .bank, PrecompiledScript*.Cache). Najpierw rozpakuj .7z lub .rar, a potem zaimportuj folder. Mod mógł mimo to zostać dodany lub zaktualizowany — odśwież listę, zanim spróbujesz ponownie.';

  @override
  String get importPickerFailed =>
      'Nie udało się otworzyć okna wyboru pliku. Nic nie zaimportowano.';

  @override
  String get importOutcomeUnknown =>
      'Wynik jest niejasny. Odśwież, aby sprawdzić listę modów.';

  @override
  String get applyTooltip => 'Zainstaluj zaznaczone mody w grze';

  @override
  String get undeployAllAction => 'Usuń wszystko z gry';

  @override
  String get undeployAllConfirm =>
      'Usunąć z gry wszystkie mody zainstalowane przez Managera?';

  @override
  String get takeOverTitle => 'Mod Studio jest aktywne';

  @override
  String get takeOverBody =>
      'Mod Studio ma teraz mod w grze. Przejąć, aby Manager zastosował twój wybór?';

  @override
  String get takeOverAction => 'Przejmij';

  @override
  String get refreshAction => 'Odśwież';

  @override
  String conflictsTitle(int count) {
    return 'Konflikty ($count)';
  }

  @override
  String get conflictWinner => 'wygrywa';

  @override
  String get noConflicts => 'Nie znaleziono konfliktów.';

  @override
  String get conflictCoverageIncomplete =>
      'Niektórych modów nie da się w pełni sprawdzić, więc konfliktów może być więcej.';

  @override
  String get loadOrderDirection => 'Mody niżej na liście nadpisują te powyżej.';

  @override
  String get footprintCoverageScope =>
      'Wymieniono tylko znane cele konfliktów. To nie gwarancja tego, co stanie się w grze.';

  @override
  String get footprintTargetsExact => 'Zmieniane pozycje — pełna lista:';

  @override
  String get footprintTargetsPartial =>
      'Zmieniane pozycje — może być ich więcej:';

  @override
  String get footprintTargetsAdvisory =>
      'Prawdopodobnie zmieniane pozycje — wskazówki, nie dowód:';

  @override
  String get footprintTargetsOpaque =>
      'GORE nie potrafi ustalić, co to zmienia.';

  @override
  String get conflictsUnverified => 'Konflikty nieznane — najpierw odśwież.';

  @override
  String get componentsTitle => 'Co zmienia ten mod';

  @override
  String targetsMore(int count) {
    return '+$count więcej';
  }

  @override
  String get removeModDeploymentHint =>
      'To usuwa go tylko z twojej listy. Jeśli jest zainstalowany w grze, wybierz potem Zastosuj.';

  @override
  String removeModSuccess(String name) {
    return 'Usunięto „$name”.';
  }

  @override
  String removeModFailed(String name) {
    return 'Nie udało się usunąć „$name”.';
  }

  @override
  String removeModPartialFailure(String name) {
    return 'Usunięto „$name”, ale listy nie udało się w pełni odświeżyć.';
  }

  @override
  String removeModOutcomeUnknown(String name) {
    return 'Nie udało się potwierdzić, czy „$name” został usunięty.';
  }

  @override
  String get libraryStateUnknown =>
      'Lista modów jest nieaktualna. Odśwież przed zmianą lub zastosowaniem modów.';

  @override
  String get removeModAction => 'Usuń';

  @override
  String removeModConfirm(String name) {
    return 'Usunąć „$name” z twojej listy?';
  }

  @override
  String get errorSetGamePath =>
      'Najpierw wybierz instalację Gothic w ustawieniach.';

  @override
  String applyReportApplied(int count) {
    return 'Zastosowano $count modów.';
  }

  @override
  String get modDisabledHint => 'Wyłączony';

  @override
  String get kindGoremod => 'Pakiet GORE';

  @override
  String get kindTriplet => 'Mod IoStore';

  @override
  String get kindPak => 'Mod PAK';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'Podmiana całych plików';

  @override
  String get kindMixed => 'Mieszany';

  @override
  String get sevHard => 'Konflikt';

  @override
  String get sevSoft => 'Ostrzeżenie';

  @override
  String get sevInfo => 'Informacja';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'O programie';

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

  @override
  String get aboutLicense => 'Udostępniane na licencji MIT.';

  @override
  String get appearanceTitle => 'Wygląd';

  @override
  String get theme => 'Motyw';

  @override
  String get themeLight => 'Jasny';

  @override
  String get themeDark => 'Ciemny';

  @override
  String get themeSystem => 'Systemowy';

  @override
  String get uiScale => 'Wielkość interfejsu';

  @override
  String get resetZoomTooltip => 'Resetuj powiększenie (Ctrl+0)';

  @override
  String get zoomTip =>
      'Wskazówka: Ctrl + / Ctrl - zmienia powiększenie w dowolnym miejscu aplikacji.';

  @override
  String get lightMode => 'Tryb jasny';

  @override
  String get darkMode => 'Tryb ciemny';

  @override
  String get minimize => 'Minimalizuj';

  @override
  String get restore => 'Przywróć';

  @override
  String get maximize => 'Maksymalizuj';

  @override
  String get close => 'Zamknij';
}
