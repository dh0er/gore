import 'dart:convert';
import 'dart:io';

import 'package:flutter/widgets.dart' show Locale;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/l10n/app_localizations.dart';
import 'package:gore_mod/l10n/app_localizations_de.dart';
import 'package:gore_mod/l10n/app_localizations_en.dart';

const _arbFiles = <String>[
  'app_de.arb',
  'app_en.arb',
  'app_es.arb',
  'app_fr.arb',
  'app_it.arb',
  'app_ja.arb',
  'app_pl.arb',
  'app_pt.arb',
  'app_pt_BR.arb',
  'app_ru.arb',
  'app_zh.arb',
  'app_zh_Hans.arb',
];

const _requiredKeys = <String>{
  'managedWorkspaceTextVoiceLabel',
  'managedWorkspaceTestReleaseLabel',
  'managedTestReleaseTitle',
  'managedTestReleaseDescription',
  'managedTestReleaseEvidenceBoundary',
  'managedTestReleaseChecksHeading',
  'managedTestReleaseReleaseHeading',
  'managedTestReleaseStatusNotChecked',
  'managedTestReleaseStatusChecking',
  'managedTestReleaseStatusChecked',
  'managedTestReleaseStatusNeedsAttention',
  'managedTestReleaseStatusBlocked',
  'managedTestReleaseStatusNotAvailable',
  'managedTestReleaseStatusAvailable',
  'managedTestReleaseEvidenceLabel',
  'managedTestReleaseStaleEvidenceDescription',
  'managedTestReleaseActionNotConnectedDescription',
  'managedTestReleaseProblemsHeading',
  'managedTestReleaseVoiceHeading',
  'managedTestReleaseProjectStructureTitle',
  'managedTestReleaseProjectStructureDescription',
  'managedTestReleaseProjectStructureAction',
  'managedTestReleaseScriptsTitle',
  'managedTestReleaseScriptsDescription',
  'managedTestReleaseScriptsAction',
  'managedProjectCompilerRetryAction',
  'managedProjectCompilerReviewAction',
  'managedProjectCompilerDialogTitle',
  'managedProjectCompilerDialogIntroduction',
  'managedProjectCompilerCloseAction',
  'managedProjectCompilerNoGame',
  'managedProjectCompilerSafetyBlocked',
  'managedProjectCompilerCompiled',
  'managedProjectCompilerEmpty',
  'managedProjectCompilerRejected',
  'managedProjectCompilerPreflightBlocked',
  'managedProjectCompilerDrifted',
  'managedProjectCompilerRequiresReopen',
  'managedProjectCompilerRecoveryRequired',
  'managedProjectCompilerFailed',
  'managedProjectCompilerFailureDetails',
  'managedProjectCompilerDiagnosticsHeading',
  'managedProjectCompilerCaptureCaptured',
  'managedProjectCompilerCaptureFallback',
  'managedProjectCompilerCaptureInvalid',
  'managedProjectCompilerCaptureUnavailable',
  'managedProjectCompilerCaptureExitUnconfirmed',
  'managedProjectCompilerCaptureDisabled',
  'managedProjectCompilerSeverityError',
  'managedProjectCompilerSeverityWarning',
  'managedProjectCompilerSeverityNote',
  'managedProjectCompilerFileLabel',
  'managedProjectCompilerLineLabel',
  'managedProjectCompilerColumnLabel',
  'managedProjectCompilerOmittedDiagnostics',
  'managedTestReleaseVoiceTitle',
  'managedTestReleaseVoiceDescription',
  'managedTestReleaseVoiceAction',
  'managedVoiceBuildReadinessTitle',
  'managedVoiceBuildReadinessRefresh',
  'managedVoiceBuildReadinessChecking',
  'managedVoiceBuildReadinessLoadError',
  'managedVoiceBuildReadinessReadyTitle',
  'managedVoiceBuildReadinessBlockedTitle',
  'managedVoiceBuildReadinessCount',
  'managedVoiceBuildReadinessBlockedBoundary',
  'managedVoiceBuildReadinessBuildReleaseGuidance',
  'managedVoiceBuildReadinessConfigureGameGuidance',
  'managedTestReleaseDataAssetsTitle',
  'managedTestReleaseDataAssetsDescription',
  'managedTestReleaseDataAssetsAction',
  'managedTestReleasePlayableBuildTitle',
  'managedTestReleasePlayableBuildDescription',
  'managedTestReleasePlayableBuildBlockedReason',
  'managedTestReleaseCreatePlayableFilesAction',
  'managedTestReleaseDeploymentTitle',
  'managedTestReleaseDeploymentDescription',
  'managedTestReleaseDeploymentBlockedReason',
  'managedTestReleaseInstallAction',
  'managedProjectCommandBarCurrentSection',
  'managedProjectCommandBarOrientationSemantics',
  'managedProjectCommandBarUndoLabel',
  'managedProjectCommandBarSearchLabel',
  'managedProjectCommandBarCreateLabel',
  'managedProjectCommandBarProblemsLabel',
  'managedProjectCommandBarHistoryLabel',
  'managedProjectCommandBarSettingsLabel',
  'managedProjectCommandBarMoreActionsTooltip',
  'managedProjectCommandBarBusyLabel',
  'managedProjectCommandBarBusyDisabledReason',
};

const _dataAssetsBoundaryMarkers = <String, List<String>>{
  'de': <String>[
    'Prüft nur den exakten aktuellen Bereich bereits vorgemerkter DataAssets, '
        'den die Projekt-Bauvorschau verifiziert hat.',
    'Neue oder strukturelle Assets, spielbare Dateien, Installation, '
        'Bereitstellung',
    'Spiel- oder Spielstandsänderungen, Laufzeitverhalten und Weltinhalte sind '
        'nicht abgedeckt.',
  ],
  'en': <String>[
    'Checks only the exact current staged DataAssets domain already verified '
        'by Project build preview.',
    'It does not cover new or structural assets, playable files, installation, '
        'deployment',
    'game or save changes, runtime behavior, or World content.',
  ],
  'es': <String>[
    'Comprueba solo el dominio exacto actual de DataAssets preparados que ya '
        'verificó la vista previa de compilación del proyecto.',
    'No cubre recursos nuevos o estructurales, archivos jugables, instalación, '
        'despliegue',
    'cambios en el juego o partidas guardadas, comportamiento en ejecución ni '
        'contenido del mundo.',
  ],
  'fr': <String>[
    'Vérifie uniquement le domaine exact et actuel des DataAssets préparés, '
        'déjà validé par l’aperçu de build du projet.',
    'Cela ne couvre pas les ressources nouvelles ou structurelles, les fichiers '
        'jouables, l’installation, le déploiement',
    'les modifications du jeu ou des fichiers de sauvegarde, le comportement '
        'à l’exécution ni le contenu du monde.',
  ],
  'it': <String>[
    'Controlla solo il dominio esatto e corrente dei DataAsset preparati, già '
        "verificato dall'anteprima build del progetto.",
    'Non copre asset nuovi o strutturali, file giocabili, installazione, '
        'distribuzione',
    'modifiche al gioco o ai file di salvataggio, comportamento in esecuzione '
        'o contenuti del mondo.',
  ],
  'ja': <String>[
    'プロジェクトのビルドプレビューで検証済みの、現在の正確なステージ済み DataAssets ドメインだけを確認します。',
    '新規または構造的なアセット、プレイ可能ファイル、インストール、デプロイ',
    'ゲームやセーブの変更、実行時の挙動、World コンテンツは対象外です。',
  ],
  'pl': <String>[
    'Sprawdza wyłącznie dokładny, bieżący obszar przygotowanych zasobów '
        'DataAssets, który został już zweryfikowany przez podgląd kompilacji '
        'projektu.',
    'Nie obejmuje nowych ani strukturalnych zasobów, plików grywalnych, '
        'instalacji, wdrażania',
    'zmian w grze lub zapisach, działania w czasie wykonywania ani zawartości '
        'świata.',
  ],
  'pt': <String>[
    'Verifica apenas o domínio DataAssets preparado, exato e atual, já validado '
        'pela pré-visualização da compilação do projeto.',
    'Não abrange recursos novos ou estruturais, ficheiros jogáveis, instalação, '
        'distribuição',
    'alterações ao jogo ou aos ficheiros guardados, comportamento em execução '
        'nem conteúdo do mundo.',
  ],
  'pt-BR': <String>[
    'Verifica somente o domínio DataAssets preparado, exato e atual, já '
        'validado pela prévia da compilação do projeto.',
    'Não abrange assets novos ou estruturais, arquivos jogáveis, instalação, '
        'implantação',
    'alterações no jogo ou em saves, comportamento em execução nem conteúdo de '
        'mundo.',
  ],
  'ru': <String>[
    'Проверяет только точный текущий домен подготовленных DataAssets, уже '
        'подтверждённый предварительным просмотром сборки проекта.',
    'Проверка не охватывает новые или структурные ресурсы, игровые файлы, '
        'установку, развёртывание',
    'изменения игры или сохранений, поведение во время выполнения и содержимое '
        'мира.',
  ],
  'zh': <String>[
    '僅檢查已由專案建置預覽驗證的精確目前暫存 DataAssets 領域。',
    '不涵蓋新增或結構性資產、可遊玩檔案、安裝、部署',
    '遊戲或存檔變更、執行階段行為或世界內容。',
  ],
  'zh-Hans': <String>[
    '仅检查已由项目构建预览验证的精确当前暂存 DataAssets 领域。',
    '不涵盖新增或结构性资产、可游玩文件、安装、部署',
    '游戏或存档更改、运行时行为或世界内容。',
  ],
};

void main() {
  test(
    'every shipped locale owns the complete Test & Release copy contract',
    () {
      for (final fileName in _arbFiles) {
        final file = File('lib/l10n/$fileName');
        expect(file.existsSync(), isTrue, reason: fileName);
        final arb = (jsonDecode(file.readAsStringSync()) as Map)
            .cast<String, Object?>();

        for (final key in _requiredKeys) {
          expect(arb, contains(key), reason: '$fileName: $key');
          expect(
            arb[key],
            isA<String>().having(
              (value) => value.trim(),
              'trimmed',
              isNotEmpty,
            ),
            reason: '$fileName: $key',
          );
        }
        expect(
          arb['managedProjectCommandBarCurrentSection'],
          contains('{section}'),
          reason: '$fileName: current section placeholder',
        );
        expect(
          arb['managedProjectCommandBarOrientationSemantics'],
          allOf(contains('{project}'), contains('{section}')),
          reason: '$fileName: orientation placeholders',
        );
        expect(
          arb['managedTestReleaseVoiceTitle'],
          arb['managedTestReleaseVoiceHeading'],
          reason: '$fileName: Voice bundle check naming',
        );
      }
    },
  );

  test(
    'generated localizations expose the complete contract for every locale',
    () {
      final localeTags = AppLocalizations.supportedLocales
          .map((locale) => locale.toLanguageTag())
          .toList(growable: false);
      expect(localeTags, unorderedEquals(_dataAssetsBoundaryMarkers.keys));
      expect(localeTags.map(_arbFileForLocaleTag), unorderedEquals(_arbFiles));

      for (final locale in AppLocalizations.supportedLocales) {
        final l10n = lookupAppLocalizations(locale);
        for (final value in _testReleaseStrings(l10n)) {
          expect(value.trim(), isNotEmpty, reason: '$locale');
        }
        expect(
          l10n.managedProjectCommandBarCurrentSection('Story'),
          contains('Story'),
          reason: '$locale: current section substitution',
        );
        expect(
          l10n.managedProjectCommandBarOrientationSemantics('My Mod', 'Story'),
          allOf(contains('My Mod'), contains('Story')),
          reason: '$locale: orientation substitutions',
        );
        final localeTag = locale.toLanguageTag();
        final markers = _dataAssetsBoundaryMarkers[localeTag];
        expect(markers, isNotNull, reason: '$locale: boundary markers');
        final arb =
            (jsonDecode(
                      File(
                        'lib/l10n/${_arbFileForLocaleTag(localeTag)}',
                      ).readAsStringSync(),
                    )
                    as Map)
                .cast<String, Object?>();
        final generatedDescription =
            l10n.managedTestReleaseDataAssetsDescription;
        expect(
          generatedDescription,
          arb['managedTestReleaseDataAssetsDescription'],
          reason: '$localeTag: generated/ARB drift',
        );
        for (final marker in markers!) {
          expect(
            generatedDescription,
            contains(marker),
            reason: '$localeTag: DataAssets boundary marker "$marker"',
          );
        }
      }

      expect(
        lookupAppLocalizations(const Locale('pt', 'BR')).localeName,
        'pt_BR',
      );
      expect(lookupAppLocalizations(const Locale('pt', 'PT')).localeName, 'pt');
      expect(
        lookupAppLocalizations(
          const Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans'),
        ).localeName,
        'zh_Hans',
      );
      expect(
        lookupAppLocalizations(
          const Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hant'),
        ).localeName,
        'zh',
      );
    },
  );

  test('compiler recovery copy covers both retained-output lanes', () {
    final recovery =
        AppLocalizationsEn().managedProjectCompilerRecoveryRequired;
    expect(recovery, contains('private compiler output'));
    expect(recovery, contains('exact restoration of the game installation'));
  });

  test('Voice row copy names and bounds only the bundle-plan check', () {
    final english = AppLocalizationsEn();
    expect(english.managedTestReleaseVoiceTitle, 'Voice bundle check');
    expect(english.managedTestReleaseVoiceHeading, 'Voice bundle check');
    expect(
      english.managedTestReleaseVoiceDescription,
      allOf(
        contains('exact current existing-member Voice bundle plan'),
        contains('text or translation coverage'),
        contains('playback'),
        contains('build output'),
        contains('deployment'),
        contains('runtime'),
      ),
    );
    expect(english.managedVoiceBuildReadinessTitle, 'Voice bundle check');
    expect(
      english.managedVoiceBuildReadinessReadyTitle,
      'Voice bundle plan checked',
    );
    expect(
      english.managedVoiceBuildReadinessBlockedTitle,
      'Voice bundle plan needs attention',
    );
    expect(
      english.managedVoiceBuildReadinessCount(1, 2),
      '1 of 2 existing Voice slots pass this bundle plan.',
    );
    expect(
      english.managedVoiceBuildReadinessBuildReleaseGuidance,
      allOf(
        contains('only the plan'),
        contains('separate action'),
        isNot(contains('Open Build & Release')),
        isNot(contains('Voice content is ready')),
      ),
    );
    expect(
      english.managedVoiceBuildReadinessConfigureGameGuidance,
      allOf(
        contains('exact Voice bundle plan is checked'),
        contains('separate offline bundle action'),
        isNot(contains('Voice content is ready')),
      ),
    );

    final german = AppLocalizationsDe();
    expect(german.managedTestReleaseVoiceTitle, 'Voice-Bundle-Prüfung');
    expect(german.managedTestReleaseVoiceHeading, 'Voice-Bundle-Prüfung');
    expect(
      german.managedTestReleaseVoiceDescription,
      allOf(
        contains('vorhandenen Archiveinträgen'),
        contains('Text- oder Übersetzungsabdeckung'),
        contains('Wiedergabe'),
        contains('Build-Ausgabe'),
        contains('Bereitstellung'),
        contains('Laufzeit'),
      ),
    );
    expect(german.managedVoiceBuildReadinessTitle, 'Voice-Bundle-Prüfung');
    expect(
      german.managedVoiceBuildReadinessReadyTitle,
      'Voice-Bundle-Plan geprüft',
    );
    expect(
      german.managedVoiceBuildReadinessBlockedTitle,
      'Voice-Bundle-Plan benötigt Aufmerksamkeit',
    );
    expect(
      german.managedVoiceBuildReadinessCount(1, 2),
      '1 von 2 vorhandenen Voice-Slots bestehen diesen Bundle-Plan.',
    );
    expect(
      german.managedVoiceBuildReadinessBuildReleaseGuidance,
      allOf(
        contains('nur der Plan geprüft'),
        contains('separate Aktion'),
        isNot(contains('Build & Release')),
        isNot(contains('Voice-Inhalt ist bereit')),
      ),
    );
    expect(
      german.managedVoiceBuildReadinessConfigureGameGuidance,
      allOf(
        contains('exakte Voice-Bundle-Plan ist geprüft'),
        contains('separate Offline-Bundle-Aktion'),
        isNot(contains('Voice-Inhalt ist bereit')),
      ),
    );
  });

  test('DataAssets row copy bounds only the exact staged preview domain', () {
    final english = AppLocalizationsEn();
    expect(
      english.managedTestReleaseDataAssetsDescription,
      allOf(
        allOf(
          contains('exact current staged DataAssets domain'),
          contains('Project build preview'),
          contains('new or structural assets'),
          contains('playable files'),
          contains('installation'),
          contains('deployment'),
        ),
        allOf(
          contains('game or save changes'),
          contains('runtime behavior'),
          contains('World content'),
          isNot(contains('visible in Problems')),
          isNot(contains('complete project-wide build evidence')),
        ),
      ),
    );

    final german = AppLocalizationsDe();
    expect(
      german.managedTestReleaseDataAssetsDescription,
      allOf(
        allOf(
          contains('exakten aktuellen Bereich'),
          contains('Projekt-Bauvorschau'),
          contains('Neue oder strukturelle Assets'),
          contains('spielbare Dateien'),
          contains('Installation'),
          contains('Bereitstellung'),
        ),
        allOf(
          contains('Spiel- oder Spielstandsänderungen'),
          contains('Laufzeitverhalten'),
          contains('Weltinhalte'),
          isNot(contains('Problemliste')),
          isNot(contains('vollständigen projektweiten Build-Nachweis')),
        ),
      ),
    );
  });
}

String _arbFileForLocaleTag(String localeTag) {
  return switch (localeTag) {
    'pt-BR' => 'app_pt_BR.arb',
    'zh-Hans' => 'app_zh_Hans.arb',
    _ => 'app_$localeTag.arb',
  };
}

List<String> _testReleaseStrings(AppLocalizations l10n) => <String>[
  l10n.managedWorkspaceTextVoiceLabel,
  l10n.managedWorkspaceTestReleaseLabel,
  l10n.managedTestReleaseTitle,
  l10n.managedTestReleaseDescription,
  l10n.managedTestReleaseEvidenceBoundary,
  l10n.managedTestReleaseChecksHeading,
  l10n.managedTestReleaseReleaseHeading,
  l10n.managedTestReleaseStatusNotChecked,
  l10n.managedTestReleaseStatusChecking,
  l10n.managedTestReleaseStatusChecked,
  l10n.managedTestReleaseStatusNeedsAttention,
  l10n.managedTestReleaseStatusBlocked,
  l10n.managedTestReleaseStatusNotAvailable,
  l10n.managedTestReleaseStatusAvailable,
  l10n.managedTestReleaseEvidenceLabel,
  l10n.managedTestReleaseStaleEvidenceDescription,
  l10n.managedTestReleaseActionNotConnectedDescription,
  l10n.managedTestReleaseProblemsHeading,
  l10n.managedTestReleaseVoiceHeading,
  l10n.managedTestReleaseProjectStructureTitle,
  l10n.managedTestReleaseProjectStructureDescription,
  l10n.managedTestReleaseProjectStructureAction,
  l10n.managedTestReleaseScriptsTitle,
  l10n.managedTestReleaseScriptsDescription,
  l10n.managedTestReleaseScriptsAction,
  l10n.managedProjectCompilerRetryAction,
  l10n.managedProjectCompilerReviewAction,
  l10n.managedProjectCompilerDialogTitle,
  l10n.managedProjectCompilerDialogIntroduction,
  l10n.managedProjectCompilerCloseAction,
  l10n.managedProjectCompilerNoGame,
  l10n.managedProjectCompilerSafetyBlocked,
  l10n.managedProjectCompilerCompiled,
  l10n.managedProjectCompilerEmpty,
  l10n.managedProjectCompilerRejected,
  l10n.managedProjectCompilerPreflightBlocked,
  l10n.managedProjectCompilerDrifted,
  l10n.managedProjectCompilerRequiresReopen,
  l10n.managedProjectCompilerRecoveryRequired,
  l10n.managedProjectCompilerFailed,
  l10n.managedProjectCompilerFailureDetails,
  l10n.managedProjectCompilerDiagnosticsHeading,
  l10n.managedProjectCompilerCaptureCaptured,
  l10n.managedProjectCompilerCaptureFallback,
  l10n.managedProjectCompilerCaptureInvalid,
  l10n.managedProjectCompilerCaptureUnavailable,
  l10n.managedProjectCompilerCaptureExitUnconfirmed,
  l10n.managedProjectCompilerCaptureDisabled,
  l10n.managedProjectCompilerSeverityError,
  l10n.managedProjectCompilerSeverityWarning,
  l10n.managedProjectCompilerSeverityNote,
  l10n.managedProjectCompilerFileLabel,
  l10n.managedProjectCompilerLineLabel,
  l10n.managedProjectCompilerColumnLabel,
  l10n.managedProjectCompilerOmittedDiagnostics,
  l10n.managedTestReleaseVoiceTitle,
  l10n.managedTestReleaseVoiceDescription,
  l10n.managedTestReleaseVoiceAction,
  l10n.managedVoiceBuildReadinessTitle,
  l10n.managedVoiceBuildReadinessRefresh,
  l10n.managedVoiceBuildReadinessChecking,
  l10n.managedVoiceBuildReadinessLoadError,
  l10n.managedVoiceBuildReadinessReadyTitle,
  l10n.managedVoiceBuildReadinessBlockedTitle,
  l10n.managedVoiceBuildReadinessCount(1, 2),
  l10n.managedVoiceBuildReadinessBlockedBoundary,
  l10n.managedVoiceBuildReadinessBuildReleaseGuidance,
  l10n.managedVoiceBuildReadinessConfigureGameGuidance,
  l10n.managedTestReleaseDataAssetsTitle,
  l10n.managedTestReleaseDataAssetsDescription,
  l10n.managedTestReleaseDataAssetsAction,
  l10n.managedTestReleasePlayableBuildTitle,
  l10n.managedTestReleasePlayableBuildDescription,
  l10n.managedTestReleasePlayableBuildBlockedReason,
  l10n.managedTestReleaseCreatePlayableFilesAction,
  l10n.managedTestReleaseDeploymentTitle,
  l10n.managedTestReleaseDeploymentDescription,
  l10n.managedTestReleaseDeploymentBlockedReason,
  l10n.managedTestReleaseInstallAction,
  l10n.managedProjectCommandBarCurrentSection('Story'),
  l10n.managedProjectCommandBarOrientationSemantics('My Mod', 'Story'),
  l10n.managedProjectCommandBarUndoLabel,
  l10n.managedProjectCommandBarSearchLabel,
  l10n.managedProjectCommandBarCreateLabel,
  l10n.managedProjectCommandBarProblemsLabel,
  l10n.managedProjectCommandBarHistoryLabel,
  l10n.managedProjectCommandBarSettingsLabel,
  l10n.managedProjectCommandBarMoreActionsTooltip,
  l10n.managedProjectCommandBarBusyLabel,
  l10n.managedProjectCommandBarBusyDisabledReason,
];
