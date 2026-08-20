import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/l10n/app_localizations.dart';
import 'package:gore_manager/library/domain/models.dart';
import 'package:gore_manager/library/ui/mod_labels.dart';

void main() {
  test('new Manager messages resolve in all twelve shipped locales', () async {
    expect(AppLocalizations.supportedLocales, hasLength(12));

    for (final locale in AppLocalizations.supportedLocales) {
      final l10n = await AppLocalizations.delegate.load(locale);
      final messages = <String>[
        l10n.preflightAttention,
        l10n.preflightGameRunning,
        l10n.preflightUnavailable,
        l10n.managerOperationFailed,
        l10n.libraryOperationFailed,
        l10n.conflictsUnavailable,
        l10n.applyReportAppliedWithWarnings(1, 2),
        l10n.modDetailKind,
        l10n.modDetailVersion,
        l10n.modDetailAuthor,
        l10n.modDetailSource,
        l10n.modDetailImported,
        l10n.componentLocalization,
        l10n.componentAudio,
        l10n.componentAngelScript,
        l10n.componentTexture,
        l10n.componentKindLocalizationPatch,
        l10n.componentKindAudioPatch,
        l10n.componentKindAngelScriptPatch,
        l10n.componentKindTexturePatch,
        l10n.componentKindLoosePak,
        l10n.componentKindTriplet,
        l10n.componentKindUe4ssLua,
        l10n.componentKindRawFile,
        l10n.componentKindFilePatch,
        l10n.componentKindPakFilePatch,
        l10n.componentKindVoiceArchivePatch,
        l10n.conflictKindLocalization,
        l10n.conflictKindAudio,
        l10n.conflictKindAsset,
        l10n.conflictKindCdo,
        l10n.conflictKindUe4ssUnknown,
        l10n.conflictKindScriptModule,
        l10n.conflictKindVoiceArchive,
        l10n.conflictKindRawFile,
        l10n.conflictKindLooseFile,
      ];

      expect(
        messages,
        everyElement(isNotEmpty),
        reason: 'All new messages must resolve for $locale.',
      );
      expect(
        messages,
        everyElement(
          isNot(anyOf(contains('{applied}'), contains('{warnings}'))),
        ),
        reason: 'Runtime placeholders must resolve for $locale.',
      );
    }
  });

  test('German high-visibility copy remains direct and compact', () async {
    final l10n = await AppLocalizations.delegate.load(const Locale('de'));

    expect(l10n.managerOperationFailed, 'Der Vorgang ist fehlgeschlagen.');
    expect(
      l10n.applyReportAppliedWithWarnings(3, 2),
      'Angewendete Mods: 3. Warnungen: 2.',
    );
    expect(
      l10n.preflightGameRunning,
      'Gothic läuft noch. Schließe das Spiel, bevor du Mods änderst.',
    );
    expect(
      l10n.preflightUnavailable,
      'Die Spielinstallation konnte nicht geprüft werden.',
    );
    expect(l10n.componentKindLoosePak, 'PAK-Datei');
    expect(l10n.componentKindTriplet, 'IoStore-Container');
    // The plain view never shows that container word at all.
    expect(l10n.componentGameFiles, 'Spieldateien');
    expect(l10n.conflictKindUe4ssUnknown, 'UE4SS (unklar)');
    expect(l10n.conflictKindLooseFile, 'Spieldatei');
  });

  test(
    'all known component and conflict wire tags have German labels',
    () async {
      final l10n = await AppLocalizations.delegate.load(const Locale('de'));
      const componentKinds = [
        'loc_patch',
        'audio_patch',
        'angel_script_patch',
        'texture_patch',
        'loose_pak',
        'triplet',
        'ue4ss_lua',
        'raw_file',
        'file_patch',
        'pak_file_patch',
        'voice_archive_patch',
      ];
      const conflictKinds = [
        'loc',
        'audio',
        'asset',
        'cdo',
        'ue4ss_unknown',
        'script_module',
        'voice_archive',
        'raw_file',
        'loose_file',
      ];

      for (final kind in componentKinds) {
        expect(componentKindLabel(l10n, kind), isNot(kind));
      }
      for (final kind in conflictKinds) {
        expect(conflictKindLabel(l10n, kind), isNot(kind));
      }

      final chips = componentChips(l10n, [
        for (final kind in componentKinds)
          ComponentView.fromJson({'type': kind}),
      ]).map((chip) => chip.label);
      for (final kind in componentKinds) {
        expect(chips, everyElement(isNot(kind)));
      }
    },
  );

  test('all shipped ARB files keep template key parity', () {
    final files =
        Directory('lib/l10n')
            .listSync()
            .whereType<File>()
            .where((file) => file.path.endsWith('.arb'))
            .toList()
          ..sort((a, b) => a.path.compareTo(b.path));
    expect(files, hasLength(12));

    Set<String> messageKeys(File file) {
      final json = jsonDecode(file.readAsStringSync()) as Map<String, Object?>;
      return json.keys.where((key) => !key.startsWith('@')).toSet();
    }

    final template = files.singleWhere(
      (file) => file.path.endsWith('app_en.arb'),
    );
    final templateKeys = messageKeys(template);
    for (final file in files) {
      expect(
        messageKeys(file),
        templateKeys,
        reason: '${file.path} must match app_en.arb.',
      );
    }
  });
}
