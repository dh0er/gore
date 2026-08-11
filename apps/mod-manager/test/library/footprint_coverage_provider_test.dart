import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/mgr_ffi.dart';
import 'package:gore_manager/library/domain/conflicts_provider.dart';
import 'package:gore_manager/library/domain/library_notifier.dart';
import 'package:gore_manager/library/domain/models.dart';

class _NoopCore implements GoreCoreFfiService {
  @override
  String get description => 'coverage-provider-test';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async => {'ok': true};
}

class _SeededLibraryNotifier extends LibraryNotifier {
  _SeededLibraryNotifier(LibraryState seed) : super(MgrFfi(_NoopCore())) {
    state = seed;
  }
}

ComponentView _component(String coverage) => ComponentView.fromJson({
  'type': 'triplet',
  'rel_base': 'containers/$coverage',
  'targets': ['/Game/$coverage'],
  'coverage': coverage,
});

ModEntryMetaView _mod(String id, List<ComponentView> components) =>
    ModEntryMetaView(
      id: id,
      kind: 'foreign_mixed',
      name: id,
      components: components,
    );

void main() {
  test('enabled coverage provider qualifies any non-exact component', () {
    final seed = LibraryState(
      authoritative: true,
      mods: [
        _mod('CaseID', [_component('exact'), _component('partial')]),
        _mod('caseid', [_component('opaque')]),
      ],
      loadout: const LoadoutView(
        entries: [
          LoadoutEntryView(id: 'CaseID'),
          LoadoutEntryView(id: 'caseid', enabled: false),
        ],
      ),
    );
    final container = ProviderContainer(
      overrides: [
        libraryProvider.overrideWith((ref) => _SeededLibraryNotifier(seed)),
      ],
    );
    addTearDown(container.dispose);

    expect(container.read(enabledFootprintKnowledgeIncompleteProvider), isTrue);
  });

  test('unverified library state publishes no stale coverage authority', () {
    expect(
      hasIncompleteEnabledFootprintKnowledge(
        LibraryState(
          authoritative: false,
          mods: [
            _mod('stale', [_component('opaque')]),
          ],
          loadout: const LoadoutView(entries: [LoadoutEntryView(id: 'stale')]),
        ),
      ),
      isFalse,
    );
  });

  test('enabled loadout ids are joined case-sensitively and fail closed', () {
    expect(
      hasIncompleteEnabledFootprintKnowledge(
        LibraryState(
          authoritative: true,
          mods: [
            _mod('caseid', [_component('exact')]),
          ],
          loadout: const LoadoutView(entries: [LoadoutEntryView(id: 'CaseID')]),
        ),
      ),
      isTrue,
    );
  });

  test('disabled non-exact components do not qualify conflict findings', () {
    expect(
      hasIncompleteEnabledFootprintKnowledge(
        LibraryState(
          authoritative: true,
          mods: [
            _mod('enabled', [_component('exact')]),
            _mod('disabled', [_component('opaque')]),
          ],
          loadout: const LoadoutView(
            entries: [
              LoadoutEntryView(id: 'enabled'),
              LoadoutEntryView(id: 'disabled', enabled: false),
            ],
          ),
        ),
      ),
      isFalse,
    );
  });

  test('max enabled loadout is indexed once across a large library', () {
    const exact = ComponentView(
      kind: 'triplet',
      coverage: FootprintCoverage.exact,
    );
    const opaque = ComponentView(
      kind: 'triplet',
      coverage: FootprintCoverage.opaque,
    );
    final mods = [
      for (var i = 0; i < 10000; i++) _mod('mod-$i', const [exact]),
      // Defensive duplicate: first-match parity with LibraryState.modById means
      // this later opaque value must not replace mod-0's exact metadata.
      _mod('mod-0', const [opaque]),
    ];
    final loadout = LoadoutView(
      entries: [for (var i = 0; i < 1000; i++) LoadoutEntryView(id: 'mod-$i')],
    );

    expect(
      hasIncompleteEnabledFootprintKnowledge(
        LibraryState(authoritative: true, mods: mods, loadout: loadout),
      ),
      isFalse,
    );
  });
}
