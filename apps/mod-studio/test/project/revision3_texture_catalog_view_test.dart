import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_texture_catalog.dart';
import 'package:gore_mod/project/revision3_texture_catalog_view.dart';

const _gameRoot = r'C:\Games\Gothic 1 Remake';

void main() {
  testWidgets('missing configured source offers settings without reading', (
    tester,
  ) async {
    var catalogLoads = 0;
    var settingsCalls = 0;
    await tester.pumpWidget(
      _host(
        gameRoot: null,
        loadCatalog: ({required gameRoot}) async {
          catalogLoads++;
          return _catalog('/Game/Never/T_Never');
        },
        openSettings: () => settingsCalls++,
      ),
    );
    await tester.pump();

    expect(catalogLoads, 0);
    expect(
      find.byKey(const Key('revision3-texture-catalog-setup')),
      findsOneWidget,
    );
    await tester.tap(
      find.byKey(const Key('revision3-texture-catalog-open-settings')),
    );
    expect(settingsCalls, 1);

    await tester.pumpWidget(
      _host(
        sourceSelectionIdentity: null,
        loadCatalog: ({required gameRoot}) async {
          catalogLoads++;
          return _catalog('/Game/Never/T_Never');
        },
      ),
    );
    await tester.pump();
    expect(catalogLoads, 0);
  });

  testWidgets('selection requests and displays the atomically loaded build', (
    tester,
  ) async {
    var catalogLoads = 0;
    var previewLoads = 0;
    await tester.pumpWidget(
      _host(
        loadCatalog: ({required gameRoot}) async {
          catalogLoads++;
          expect(gameRoot, _gameRoot);
          return Revision3TextureCatalogSnapshot.fromInstalledIndex(
            sourceFingerprint: _fingerprintA,
            index: const {
              '/Game/Characters/Asghan/T_Asghan_Armor': '1',
              '/Engine/World/T_Stone': '2',
            },
          );
        },
        loadPreview:
            ({
              required gameRoot,
              required expectedSourceFingerprint,
              required texture,
            }) async {
              previewLoads++;
              expect(gameRoot, _gameRoot);
              expect(expectedSourceFingerprint, _fingerprintA);
              expect(texture.packageId.decimal, '1');
              return _previewResult(
                _fingerprintA,
                format: 'PF_BC7',
                virtual: true,
                virtualLayers: 2,
                mipmapped: true,
                replaceability: Revision3TextureReplaceability.unsupported,
              );
            },
      ),
    );
    await tester.pumpAndSettle();

    expect(catalogLoads, 1);
    expect(previewLoads, 0);
    expect(find.text('2 installed textures'), findsOneWidget);
    await tester.enterText(
      find.byKey(const Key('revision3-texture-catalog-search')),
      'asghan',
    );
    await tester.pump();
    expect(find.text('T_Asghan_Armor'), findsOneWidget);
    expect(find.text('1 match / 2 total'), findsOneWidget);

    await tester.tap(find.text('T_Asghan_Armor'));
    await tester.pumpAndSettle();

    expect(previewLoads, 1);
    expect(find.text('1 × 1'), findsOneWidget);
    expect(find.text('PF_BC7'), findsOneWidget);
    expect(find.text('Virtual texture'), findsOneWidget);
    expect(find.text('2 VT layers'), findsOneWidget);
    expect(find.text('Mipmapped'), findsOneWidget);
    expect(find.text('Not replaceable'), findsOneWidget);
    expect(find.text('Installed game'), findsOneWidget);
    expect(find.text('Replace'), findsNothing);
    expect(
      find.byKey(const Key('revision3-texture-catalog-export')),
      findsNothing,
    );
  });

  testWidgets('source change discards late catalog completion', (tester) async {
    final oldCatalog = Completer<Revision3TextureCatalogSnapshot>();
    final newCatalog = Completer<Revision3TextureCatalogSnapshot>();
    var calls = 0;
    final key = GlobalKey<_ChangingSourceHostState>();
    await tester.pumpWidget(
      _ChangingSourceHost(
        key: key,
        loadCatalog: ({required gameRoot}) {
          calls++;
          return calls == 1 ? oldCatalog.future : newCatalog.future;
        },
      ),
    );
    await tester.pump();
    expect(calls, 1);
    expect(
      find.text(
        'The first exact scan can take several minutes. Only one scan runs at a time.',
      ),
      findsOneWidget,
    );

    key.currentState!.changeSource();
    await tester.pump();
    await tester.pump();
    expect(calls, 1, reason: 'catalog scans must be single-flight');
    oldCatalog.complete(_catalog('/Game/Old/T_Old'));
    await tester.pump();
    expect(calls, 2);
    newCatalog.complete(
      _catalog('/Game/New/T_New', fingerprint: _fingerprintB),
    );
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('revision3-texture-catalog-search')),
      'new',
    );
    await tester.pump();
    expect(find.text('T_New'), findsOneWidget);

    expect(find.text('T_New'), findsOneWidget);
    expect(find.text('T_Old'), findsNothing);
  });

  testWidgets('preview result with another fingerprint fails closed', (
    tester,
  ) async {
    await tester.pumpWidget(
      _host(
        loadCatalog: ({required gameRoot}) async => _catalog('/Game/UI/T_Icon'),
        loadPreview:
            ({
              required gameRoot,
              required expectedSourceFingerprint,
              required texture,
            }) async => _previewResult(_fingerprintB),
      ),
    );
    await tester.pumpAndSettle();
    await _searchAndTap(tester, 'icon', '/Game/UI/T_Icon');
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-texture-catalog-preview-error')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-texture-catalog-preview-image')),
      findsNothing,
    );
  });

  testWidgets(
    'refresh observes a new build without selection identity change',
    (tester) async {
      var catalogLoads = 0;
      var previewLoads = 0;
      await tester.pumpWidget(
        _host(
          loadCatalog: ({required gameRoot}) async {
            catalogLoads++;
            return _catalog(
              '/Game/UI/T_Icon',
              fingerprint: catalogLoads == 1 ? _fingerprintA : _fingerprintB,
            );
          },
          loadPreview:
              ({
                required gameRoot,
                required expectedSourceFingerprint,
                required texture,
              }) async {
                previewLoads++;
                return _previewResult(
                  expectedSourceFingerprint,
                  format: expectedSourceFingerprint == _fingerprintA
                      ? 'PF_OLD'
                      : 'PF_NEW',
                );
              },
        ),
      );
      await tester.pumpAndSettle();
      await _searchAndTap(tester, 'icon', '/Game/UI/T_Icon');
      await tester.pumpAndSettle();
      expect(find.text('PF_OLD'), findsOneWidget);

      await tester.tap(
        find.byKey(const Key('revision3-texture-catalog-refresh')),
      );
      await tester.pumpAndSettle();
      expect(catalogLoads, 2);
      await tester.tap(_result('/Game/UI/T_Icon'));
      await tester.pumpAndSettle();

      expect(previewLoads, 2);
      expect(find.text('PF_NEW'), findsOneWidget);
      expect(find.text('PF_OLD'), findsNothing);
    },
  );

  testWidgets('stale preview retry reloads catalog before another extract', (
    tester,
  ) async {
    var catalogLoads = 0;
    var previewLoads = 0;
    await tester.pumpWidget(
      _host(
        loadCatalog: ({required gameRoot}) async {
          catalogLoads++;
          return _catalog(
            '/Game/UI/T_Icon',
            fingerprint: catalogLoads == 1 ? _fingerprintA : _fingerprintB,
          );
        },
        loadPreview:
            ({
              required gameRoot,
              required expectedSourceFingerprint,
              required texture,
            }) async {
              previewLoads++;
              if (expectedSourceFingerprint == _fingerprintA) {
                throw const Revision3TextureSourceChangedException(
                  'GENERATION_CHANGED',
                );
              }
              return _previewResult(expectedSourceFingerprint);
            },
      ),
    );
    await tester.pumpAndSettle();
    await _searchAndTap(tester, 'icon', '/Game/UI/T_Icon');
    await tester.pumpAndSettle();

    expect(catalogLoads, 1);
    expect(previewLoads, 1);
    await tester.tap(
      find.byKey(const Key('revision3-texture-catalog-preview-retry')),
    );
    await tester.pumpAndSettle();

    expect(catalogLoads, 2);
    expect(
      previewLoads,
      1,
      reason: 'the stale fingerprint must not be retried',
    );
    await _searchAndTap(tester, 'icon', '/Game/UI/T_Icon');
    await tester.pumpAndSettle();
    expect(previewLoads, 2);
    expect(find.text('1 × 1'), findsOneWidget);
  });

  testWidgets('source change never displays a late old-build preview', (
    tester,
  ) async {
    final oldPreview = Completer<Revision3TexturePreviewResult>();
    final key = GlobalKey<_ChangingSourceHostState>();
    var previewCalls = 0;
    await tester.pumpWidget(
      _ChangingSourceHost(
        key: key,
        loadCatalog: ({required gameRoot}) async => gameRoot.endsWith('A')
            ? _catalog('/Game/Shared/T_Shared')
            : _catalog('/Game/Shared/T_Shared', fingerprint: _fingerprintB),
        loadPreview:
            ({
              required gameRoot,
              required expectedSourceFingerprint,
              required texture,
            }) {
              previewCalls++;
              if (expectedSourceFingerprint == _fingerprintA) {
                return oldPreview.future;
              }
              return Future.value(
                _previewResult(_fingerprintB, format: 'PF_NEW'),
              );
            },
      ),
    );
    await tester.pumpAndSettle();
    await _searchAndTap(tester, 'shared', '/Game/Shared/T_Shared');
    await tester.pump();
    expect(previewCalls, 1);

    key.currentState!.changeSource();
    await tester.pump();
    await tester.pumpAndSettle();
    await _searchAndTap(tester, 'shared', '/Game/Shared/T_Shared');
    await tester.pumpAndSettle();
    expect(previewCalls, 2);
    expect(find.text('PF_NEW'), findsOneWidget);

    oldPreview.complete(_previewResult(_fingerprintA, format: 'PF_OLD'));
    await tester.pumpAndSettle();
    expect(find.text('PF_NEW'), findsOneWidget);
    expect(find.text('PF_OLD'), findsNothing);
  });

  testWidgets('A to B to A shares exact in-flight work and bounded cache', (
    tester,
  ) async {
    final a = Completer<Revision3TexturePreviewResult>();
    final b = Completer<Revision3TexturePreviewResult>();
    var previewCalls = 0;
    await tester.pumpWidget(
      _host(
        loadCatalog: ({required gameRoot}) async =>
            Revision3TextureCatalogSnapshot.fromInstalledIndex(
              sourceFingerprint: _fingerprintA,
              index: const {'/Game/Test/T_A': '1', '/Game/Test/T_B': '2'},
            ),
        loadPreview:
            ({
              required gameRoot,
              required expectedSourceFingerprint,
              required texture,
            }) {
              previewCalls++;
              expect(expectedSourceFingerprint, _fingerprintA);
              return texture.assetPath.endsWith('T_A') ? a.future : b.future;
            },
      ),
    );
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('revision3-texture-catalog-search')),
      '/game/test/t_',
    );
    await tester.pump();
    await tester.tap(_result('/Game/Test/T_A'));
    await tester.pump();
    await tester.tap(_result('/Game/Test/T_B'));
    await tester.pump();
    await tester.tap(_result('/Game/Test/T_A'));
    await tester.pump();
    expect(previewCalls, 2);

    a.complete(_previewResult(_fingerprintA, format: 'PF_A'));
    await tester.pumpAndSettle();
    expect(find.text('PF_A'), findsOneWidget);
    b.complete(_previewResult(_fingerprintA, format: 'PF_B'));
    await tester.pumpAndSettle();
    expect(find.text('PF_A'), findsOneWidget);
    expect(find.text('PF_B'), findsNothing);

    await tester.tap(_result('/Game/Test/T_B'));
    await tester.pumpAndSettle();
    expect(previewCalls, 2);
    expect(find.text('PF_B'), findsOneWidget);
  });

  testWidgets('queues previews fairly with only two native loads at a time', (
    tester,
  ) async {
    final pending = <String, Completer<Revision3TexturePreviewResult>>{};
    final started = <String>[];
    await tester.pumpWidget(
      _host(
        loadCatalog: ({required gameRoot}) async =>
            Revision3TextureCatalogSnapshot.fromInstalledIndex(
              sourceFingerprint: _fingerprintA,
              index: const {
                '/Game/Queue/T_A': '1',
                '/Game/Queue/T_B': '2',
                '/Game/Queue/T_C': '3',
              },
            ),
        loadPreview:
            ({
              required gameRoot,
              required expectedSourceFingerprint,
              required texture,
            }) {
              started.add(texture.assetPath);
              final request = Completer<Revision3TexturePreviewResult>();
              pending[texture.assetPath] = request;
              return request.future;
            },
      ),
    );
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('revision3-texture-catalog-search')),
      '/game/queue/t_',
    );
    await tester.pump();

    for (final path in const [
      '/Game/Queue/T_A',
      '/Game/Queue/T_B',
      '/Game/Queue/T_C',
    ]) {
      await tester.tap(_result(path));
      await tester.pump();
    }

    expect(started, const ['/Game/Queue/T_A', '/Game/Queue/T_B']);
    expect(
      find.byKey(const Key('revision3-texture-catalog-preview-error')),
      findsNothing,
    );

    pending['/Game/Queue/T_A']!.complete(
      _previewResult(_fingerprintA, format: 'PF_A'),
    );
    await tester.pump();
    await tester.pump();
    expect(started, const [
      '/Game/Queue/T_A',
      '/Game/Queue/T_B',
      '/Game/Queue/T_C',
    ]);
    expect(
      find.byKey(const Key('revision3-texture-catalog-preview-error')),
      findsNothing,
    );

    pending['/Game/Queue/T_B']!.complete(
      _previewResult(_fingerprintA, format: 'PF_B'),
    );
    pending['/Game/Queue/T_C']!.complete(
      _previewResult(_fingerprintA, format: 'PF_C'),
    );
    await tester.pumpAndSettle();
    expect(find.text('PF_C'), findsOneWidget);
  });

  testWidgets('queued preview is discarded after selection becomes stale', (
    tester,
  ) async {
    final pending = <String, Completer<Revision3TexturePreviewResult>>{};
    final started = <String>[];
    await tester.pumpWidget(
      _host(
        loadCatalog: ({required gameRoot}) async =>
            Revision3TextureCatalogSnapshot.fromInstalledIndex(
              sourceFingerprint: _fingerprintA,
              index: const {
                '/Game/Stale/T_A': '1',
                '/Game/Stale/T_B': '2',
                '/Game/Stale/T_C': '3',
              },
            ),
        loadPreview:
            ({
              required gameRoot,
              required expectedSourceFingerprint,
              required texture,
            }) {
              started.add(texture.assetPath);
              final request = Completer<Revision3TexturePreviewResult>();
              pending[texture.assetPath] = request;
              return request.future;
            },
      ),
    );
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('revision3-texture-catalog-search')),
      '/game/stale/t_',
    );
    await tester.pump();
    for (final path in const [
      '/Game/Stale/T_A',
      '/Game/Stale/T_B',
      '/Game/Stale/T_C',
    ]) {
      await tester.tap(_result(path));
      await tester.pump();
    }
    await tester.tap(_result('/Game/Stale/T_B'));
    await tester.pump();

    pending['/Game/Stale/T_A']!.complete(_previewResult(_fingerprintA));
    await tester.pump();
    await tester.pump();
    expect(started, const ['/Game/Stale/T_A', '/Game/Stale/T_B']);

    pending['/Game/Stale/T_B']!.complete(
      _previewResult(_fingerprintA, format: 'PF_CURRENT'),
    );
    await tester.pumpAndSettle();
    expect(find.text('PF_CURRENT'), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-texture-catalog-preview-error')),
      findsNothing,
    );
  });

  testWidgets('queued preview from a replaced source never starts', (
    tester,
  ) async {
    final key = GlobalKey<_ChangingSourceHostState>();
    final pending = <String, Completer<Revision3TexturePreviewResult>>{};
    final started = <String>[];
    await tester.pumpWidget(
      _ChangingSourceHost(
        key: key,
        loadCatalog: ({required gameRoot}) async => gameRoot.endsWith('A')
            ? Revision3TextureCatalogSnapshot.fromInstalledIndex(
                sourceFingerprint: _fingerprintA,
                index: const {
                  '/Game/Old/T_A': '1',
                  '/Game/Old/T_B': '2',
                  '/Game/Old/T_C': '3',
                },
              )
            : _catalog('/Game/New/T_New', fingerprint: _fingerprintB),
        loadPreview:
            ({
              required gameRoot,
              required expectedSourceFingerprint,
              required texture,
            }) {
              started.add(texture.assetPath);
              final request = Completer<Revision3TexturePreviewResult>();
              pending[texture.assetPath] = request;
              return request.future;
            },
      ),
    );
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('revision3-texture-catalog-search')),
      '/game/old/t_',
    );
    await tester.pump();
    for (final path in const [
      '/Game/Old/T_A',
      '/Game/Old/T_B',
      '/Game/Old/T_C',
    ]) {
      await tester.tap(_result(path));
      await tester.pump();
    }
    expect(started, const ['/Game/Old/T_A', '/Game/Old/T_B']);

    key.currentState!.changeSource();
    await tester.pump();
    await tester.pumpAndSettle();
    pending['/Game/Old/T_A']!.complete(_previewResult(_fingerprintA));
    await tester.pump();
    await tester.pump();

    expect(started, const ['/Game/Old/T_A', '/Game/Old/T_B']);
    pending['/Game/Old/T_B']!.complete(_previewResult(_fingerprintA));
    await tester.pump();
    expect(tester.takeException(), isNull);
  });

  testWidgets('dispose completes queued preview waiters without native loads', (
    tester,
  ) async {
    final pending = <Completer<Revision3TexturePreviewResult>>[];
    var started = 0;
    await tester.pumpWidget(
      _host(
        loadCatalog: ({required gameRoot}) async =>
            Revision3TextureCatalogSnapshot.fromInstalledIndex(
              sourceFingerprint: _fingerprintA,
              index: const {
                '/Game/Dispose/T_A': '1',
                '/Game/Dispose/T_B': '2',
                '/Game/Dispose/T_C': '3',
              },
            ),
        loadPreview:
            ({
              required gameRoot,
              required expectedSourceFingerprint,
              required texture,
            }) {
              started++;
              final request = Completer<Revision3TexturePreviewResult>();
              pending.add(request);
              return request.future;
            },
      ),
    );
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('revision3-texture-catalog-search')),
      '/game/dispose/t_',
    );
    await tester.pump();
    for (final path in const [
      '/Game/Dispose/T_A',
      '/Game/Dispose/T_B',
      '/Game/Dispose/T_C',
    ]) {
      await tester.tap(_result(path));
      await tester.pump();
    }
    expect(started, 2);

    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pump();
    for (final request in pending) {
      request.complete(_previewResult(_fingerprintA));
    }
    await tester.pump();

    expect(started, 2);
    expect(tester.takeException(), isNull);
  });

  testWidgets('tiny previews obey a hard LRU entry limit', (tester) async {
    tester.view.devicePixelRatio = 1;
    tester.view.physicalSize = const Size(1200, 2400);
    addTearDown(tester.view.reset);
    var previewLoads = 0;
    await tester.pumpWidget(
      _host(
        loadCatalog: ({required gameRoot}) async =>
            Revision3TextureCatalogSnapshot.fromInstalledIndex(
              sourceFingerprint: _fingerprintA,
              index: {
                for (var index = 0; index < 25; index++)
                  '/Game/Cache/T_$index': '$index',
              },
            ),
        loadPreview:
            ({
              required gameRoot,
              required expectedSourceFingerprint,
              required texture,
            }) async {
              previewLoads++;
              return _previewResult(
                expectedSourceFingerprint,
                format: 'PF_${texture.displayName}',
              );
            },
      ),
    );
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('revision3-texture-catalog-search')),
      '/game/cache/t_',
    );
    await tester.pump();

    for (var index = 0; index < 25; index++) {
      await tester.tap(_result('/Game/Cache/T_$index'));
      await tester.pumpAndSettle();
    }
    expect(previewLoads, 25);

    await tester.tap(_result('/Game/Cache/T_23'));
    await tester.pumpAndSettle();
    expect(previewLoads, 25);

    await tester.tap(_result('/Game/Cache/T_0'));
    await tester.pumpAndSettle();
    expect(previewLoads, 26);
    expect(find.text('PF_T_0'), findsOneWidget);
  });

  testWidgets('equivalent callback rebuild keeps exact loaded source', (
    tester,
  ) async {
    var catalogLoads = 0;
    var previewLoads = 0;
    final key = GlobalKey<_RebuildingCallbackHostState>();
    await tester.pumpWidget(
      _RebuildingCallbackHost(
        key: key,
        loadCatalog: ({required gameRoot}) async {
          catalogLoads++;
          return _catalog('/Game/Stable/T_Stable');
        },
        loadPreview:
            ({
              required gameRoot,
              required expectedSourceFingerprint,
              required texture,
            }) async {
              previewLoads++;
              return _previewResult(expectedSourceFingerprint);
            },
      ),
    );
    await tester.pumpAndSettle();
    await _searchAndTap(tester, 'stable', '/Game/Stable/T_Stable');
    await tester.pumpAndSettle();
    expect(catalogLoads, 1);
    expect(previewLoads, 1);

    key.currentState!.rebuildWithEquivalentCallbacks();
    await tester.pumpAndSettle();
    expect(catalogLoads, 1);
    expect(previewLoads, 1);
    expect(
      find.byKey(const Key('revision3-texture-catalog-preview-image')),
      findsOneWidget,
    );
  });

  testWidgets('compact 200 percent layout opens detail and returns safely', (
    tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    tester.view.physicalSize = const Size(320, 180);
    tester.platformDispatcher.textScaleFactorTestValue = 2;
    addTearDown(tester.view.reset);
    addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);

    await tester.pumpWidget(
      _host(
        loadCatalog: ({required gameRoot}) async =>
            _catalog('/Game/Compact/T_Compact'),
      ),
    );
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    await _searchAndTap(tester, 'compact', '/Game/Compact/T_Compact');
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    expect(
      find.byKey(const Key('revision3-texture-catalog-compact-detail')),
      findsOneWidget,
    );

    await tester.tap(find.byKey(const Key('revision3-texture-catalog-back')));
    await tester.pump();
    expect(tester.takeException(), isNull);
    expect(
      find.byKey(const Key('revision3-texture-catalog-browser')),
      findsOneWidget,
    );
  });
}

Widget _host({
  String? gameRoot = _gameRoot,
  Object? sourceSelectionIdentity = 'configured-source-a',
  Revision3TextureCatalogLoader? loadCatalog,
  Revision3TexturePreviewLoader? loadPreview,
  VoidCallback? openSettings,
}) => MaterialApp(
  home: Scaffold(
    body: Revision3TextureCatalogView(
      gameRoot: gameRoot,
      sourceSelectionIdentity: sourceSelectionIdentity,
      loadCatalog:
          loadCatalog ??
          ({required gameRoot}) async => _catalog('/Game/Test/T_Default'),
      loadPreview:
          loadPreview ??
          ({
            required gameRoot,
            required expectedSourceFingerprint,
            required texture,
          }) async => _previewResult(expectedSourceFingerprint),
      openSettings: openSettings,
      copy: _copy,
    ),
  ),
);

final class _ChangingSourceHost extends StatefulWidget {
  const _ChangingSourceHost({
    required this.loadCatalog,
    this.loadPreview,
    super.key,
  });
  final Revision3TextureCatalogLoader loadCatalog;
  final Revision3TexturePreviewLoader? loadPreview;

  @override
  State<_ChangingSourceHost> createState() => _ChangingSourceHostState();
}

final class _ChangingSourceHostState extends State<_ChangingSourceHost> {
  String _root = r'C:\Games\Source A';
  String _selectionIdentity = 'source-a';

  void changeSource() => setState(() {
    _root = r'C:\Games\Source B';
    _selectionIdentity = 'source-b';
  });

  @override
  Widget build(BuildContext context) => MaterialApp(
    home: Scaffold(
      body: Revision3TextureCatalogView(
        gameRoot: _root,
        sourceSelectionIdentity: _selectionIdentity,
        loadCatalog: widget.loadCatalog,
        loadPreview:
            widget.loadPreview ??
            ({
              required gameRoot,
              required expectedSourceFingerprint,
              required texture,
            }) async => _previewResult(expectedSourceFingerprint),
        copy: _copy,
      ),
    ),
  );
}

final class _RebuildingCallbackHost extends StatefulWidget {
  const _RebuildingCallbackHost({
    required this.loadCatalog,
    required this.loadPreview,
    super.key,
  });
  final Revision3TextureCatalogLoader loadCatalog;
  final Revision3TexturePreviewLoader loadPreview;

  @override
  State<_RebuildingCallbackHost> createState() =>
      _RebuildingCallbackHostState();
}

final class _RebuildingCallbackHostState
    extends State<_RebuildingCallbackHost> {
  void rebuildWithEquivalentCallbacks() => setState(() {});

  @override
  Widget build(BuildContext context) => MaterialApp(
    home: Scaffold(
      body: Revision3TextureCatalogView(
        gameRoot: _gameRoot,
        sourceSelectionIdentity: 'stable-source',
        loadCatalog: ({required gameRoot}) =>
            widget.loadCatalog(gameRoot: gameRoot),
        loadPreview:
            ({
              required gameRoot,
              required expectedSourceFingerprint,
              required texture,
            }) => widget.loadPreview(
              gameRoot: gameRoot,
              expectedSourceFingerprint: expectedSourceFingerprint,
              texture: texture,
            ),
        copy: _copy,
      ),
    ),
  );
}

Future<void> _searchAndTap(
  WidgetTester tester,
  String query,
  String assetPath,
) async {
  await tester.enterText(
    find.byKey(const Key('revision3-texture-catalog-search')),
    query,
  );
  await tester.pump();
  await tester.tap(_result(assetPath));
}

Finder _result(String assetPath) =>
    find.byKey(ValueKey('revision3-texture-catalog-result-$assetPath'));

Revision3TextureCatalogSnapshot _catalog(
  String path, {
  Revision3TextureSourceFingerprint? fingerprint,
}) => Revision3TextureCatalogSnapshot.fromInstalledIndex(
  sourceFingerprint: fingerprint ?? _fingerprintA,
  index: {path: '1'},
);

Revision3TexturePreviewResult _previewResult(
  Revision3TextureSourceFingerprint fingerprint, {
  String format = 'PF_BC7',
  bool virtual = false,
  int? virtualLayers,
  bool mipmapped = false,
  Revision3TextureReplaceability replaceability =
      Revision3TextureReplaceability.supported,
}) => Revision3TexturePreviewResult(
  sourceFingerprint: fingerprint,
  preview: Revision3TexturePreview(
    pngBytes: _onePixelPng,
    width: 1,
    height: 1,
    pixelFormat: format,
    isVirtual: virtual,
    virtualLayers: virtualLayers ?? (virtual ? 1 : 0),
    mipmapped: mipmapped,
    replaceability: replaceability,
  ),
);

final _fingerprintA = Revision3TextureSourceFingerprint.nativeBuildId(
  'build-a|utoc:1:1',
);
final _fingerprintB = Revision3TextureSourceFingerprint.nativeBuildId(
  'build-b|utoc:2:2',
);
final Uint8List _onePixelPng = base64Decode(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
);

String _catalogCount(int count) =>
    '$count installed texture${count == 1 ? '' : 's'}';
String _searchCount(int matches, int total) => '$matches match / $total total';
String _virtualLayerCount(int count) =>
    '$count VT layer${count == 1 ? '' : 's'}';

const _copy = Revision3TextureCatalogViewCopy(
  setupTitle: 'Choose a game installation',
  setupDescription: 'Texture discovery needs one exact configured source.',
  setupActionLabel: 'Open settings',
  loadingLabel: 'Reading installed texture index…',
  loadingDescription:
      'The first exact scan can take several minutes. Only one scan runs at a time.',
  catalogCount: _catalogCount,
  searchCount: _searchCount,
  emptyTitle: 'No installed textures found',
  emptyDescription: 'The selected source returned an empty texture index.',
  errorTitle: 'Texture index could not be read',
  errorDescription: 'Nothing was changed. Check the source and retry.',
  retryLabel: 'Try again',
  refreshTooltip: 'Read a fresh index',
  searchLabel: 'Search textures',
  searchHint: 'Texture name or Unreal path',
  clearSearchTooltip: 'Clear search',
  selectPrompt: 'Select an installed texture to inspect its original.',
  previewLoadingLabel: 'Extracting original preview…',
  previewErrorTitle: 'Preview could not be read',
  previewErrorDescription: 'Nothing was changed. Retry the exact texture.',
  previewRetryLabel: 'Retry preview',
  backToCatalogLabel: 'Back to textures',
  inspectionOnlyNotice:
      'Installed-game preview only. This grants no edit, build, deployment, runtime, game, or save authority.',
  installedSourceBadge: 'Installed game',
  regularTextureBadge: 'Regular texture',
  virtualTextureBadge: 'Virtual texture',
  virtualLayerCount: _virtualLayerCount,
  mipmappedBadge: 'Mipmapped',
  singleMipBadge: 'Single mip',
  replaceableBadge: 'Replaceable',
  notReplaceableBadge: 'Not replaceable',
  unknownReplaceabilityBadge: 'Replaceability unknown',
  unknownFormatLabel: 'Format unknown',
);
