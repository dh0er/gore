import 'dart:convert';
import 'dart:io';

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
      expect(AppLocalizations.supportedLocales, hasLength(_arbFiles.length));

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
      }
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
