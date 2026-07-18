import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/l10n/app_localizations.dart';

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
  'managedTestReleaseVoiceTitle',
  'managedTestReleaseVoiceDescription',
  'managedTestReleaseVoiceAction',
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
  l10n.managedTestReleaseVoiceTitle,
  l10n.managedTestReleaseVoiceDescription,
  l10n.managedTestReleaseVoiceAction,
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
