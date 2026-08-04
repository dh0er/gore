import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/providers.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:path/path.dart' as p;

final String? _liveGameRoot = Platform.environment['GORE_STORY_GAME_ROOT'];

void main() {
  test(
    'current installed generation creates and reopens an empty managed R3 project',
    () async {
      final gameRoot = Directory(_liveGameRoot!);
      final protectedFiles = <File>[
        File(
          p.join(
            gameRoot.path,
            'G1R',
            'Binaries',
            'Win64',
            'G1R-Win64-Shipping.exe',
          ),
        ),
        File(
          p.join(
            gameRoot.path,
            'G1R',
            'Script',
            'PrecompiledScript_Shipping.Cache',
          ),
        ),
        File(p.join(gameRoot.path, 'G1R', 'Script', 'Binds.Cache')),
      ];
      final before = <_LiveFileSeal>[
        for (final file in protectedFiles) await _seal(file),
      ];
      final projectRoot = Directory.systemTemp.createTempSync(
        'gore_live_empty_r3_project',
      );
      final core = NativeGoreCoreFfiService.tryCreate();
      expect(core, isNotNull, reason: 'build target/debug/gore_ffi.dll first');
      final container = ProviderContainer(
        overrides: [coreServiceProvider.overrideWithValue(core!)],
      );
      ManagedRevision3CurrentProjectLease? created;
      ManagedRevision3CurrentProjectLease? reopened;
      try {
        created =
            await container.read(managedRevision3CurrentProjectCreatorProvider)(
              ManagedRevision3ProjectCreateRequest(
                root: projectRoot,
                gameRoot: gameRoot.path,
                name: 'Live empty R3 proof ä',
                version: '0.1.0',
                author: 'GORE tests',
                authoringLocales: const <String>['de', 'en-US'],
              ),
            );
        expect(created.projectRevision, 0);
        expect(created.requiresReopen, isFalse);
        final project = (jsonDecode(created.canonicalProjectJson) as Map)
            .cast<String, Object?>();
        expect(project['project_id'], created.projectId);
        expect(project['revision'], 0);
        final index = await created.readContentIndex();
        expect(index.projectId, created.projectId);
        expect(index.projectRevision, 0);
        expect(index.projectName, 'Live empty R3 proof ä');
        expect(index.authoringLocales, const <String>['de', 'en-US']);
        expect(index.entities, isEmpty);
        expect(index.assets, isEmpty);
        final projectJson = created.canonicalProjectJson;
        final projectId = created.projectId;
        await created.close();
        created = null;

        reopened = await container.read(
          managedRevision3CurrentProjectOpenerProvider,
        )(projectRoot);
        expect(reopened.projectId, projectId);
        expect(reopened.projectRevision, 0);
        expect(reopened.canonicalProjectJson, projectJson);
        expect((await reopened.readContentIndex()).entities, isEmpty);
      } finally {
        await reopened?.close();
        await created?.close();
        container.dispose();
        if (projectRoot.existsSync()) projectRoot.deleteSync(recursive: true);
      }

      final after = <_LiveFileSeal>[
        for (final file in protectedFiles) await _seal(file),
      ];
      expect(after, before);
    },
    skip: !Platform.isWindows || _liveGameRoot == null
        ? 'requires Windows, GORE_STORY_GAME_ROOT, and the current native DLL'
        : false,
  );
}

final class _LiveFileSeal {
  const _LiveFileSeal({required this.byteLength, required this.sha256});

  final int byteLength;
  final String sha256;

  @override
  bool operator ==(Object other) =>
      other is _LiveFileSeal &&
      byteLength == other.byteLength &&
      sha256 == other.sha256;

  @override
  int get hashCode => Object.hash(byteLength, sha256);
}

Future<_LiveFileSeal> _seal(File file) async {
  final stat = await file.stat();
  final digest = await crypto.sha256.bind(file.openRead()).first;
  return _LiveFileSeal(byteLength: stat.size, sha256: digest.toString());
}
